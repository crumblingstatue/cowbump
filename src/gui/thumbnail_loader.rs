use {
    super::Thumbnail,
    crate::{db::EntryMap, dlog, entry, gui::ThumbnailCache},
    egui_sf2g::sf2g::{cpp::FBox, graphics::Texture},
    image::{ImageBuffer, ImageResult, Rgba, imageops::FilterType},
    parking_lot::Mutex,
    std::{
        collections::hash_map,
        path::Path,
        process::Command,
        sync::{
            Arc,
            atomic::{self, AtomicBool},
        },
    },
};

type RgbaBuf = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Default)]
struct ImageSlot {
    result: Option<ImageResult<RgbaBuf>>,
    ffmpeg: bool,
}

/// Loads images on a separate thread, one at a time.
#[derive(Default)]
pub struct ThumbnailLoader {
    image_slots: Arc<Mutex<EntryMap<ImageSlot>>>,
    no_ffmpeg: Arc<AtomicBool>,
}

impl ThumbnailLoader {
    pub fn request(&self, name: &Path, size: u32, uid: entry::Id) {
        let mut slots = self.image_slots.lock();
        if let hash_map::Entry::Vacant(e) = slots.entry(uid) {
            e.insert(ImageSlot::default());
            let slots_clone = Arc::clone(&self.image_slots);
            let name = name.to_owned();
            let no_ffmpeg = self.no_ffmpeg.clone();
            ::std::thread::spawn(move || {
                let data = match std::fs::read(&name) {
                    Ok(data) => data,
                    Err(e) => {
                        slots_clone.lock().insert(
                            uid,
                            ImageSlot {
                                result: Some(Err(image::ImageError::IoError(e))),
                                ffmpeg: false,
                            },
                        );
                        return;
                    }
                };
                let mut image_result = image::load_from_memory(&data);
                let mut ffmpeg_was_used = false;
                if let Err(err) = &image_result
                    && !no_ffmpeg.load(atomic::Ordering::Relaxed)
                {
                    let result = Command::new("ffmpeg")
                        .args(["-y", "-i"])
                        .arg(&name)
                        .args(["-frames:v", "1", "-f", "image2pipe", "pipe:1"])
                        .output();
                    match result {
                        Ok(out) => {
                            dlog!("Error loading {name:?}: {err}. Loading with ffmpeg");
                            image_result = image::load_from_memory(&out.stdout);
                            ffmpeg_was_used = true;
                        }
                        Err(e) => {
                            dlog!("Failed to generate thumbnail with ffmpeg: {e}");
                            no_ffmpeg.store(true, atomic::Ordering::Relaxed);
                        }
                    }
                }
                let result =
                    image_result.map(|i| i.resize(size, size, FilterType::Triangle).to_rgba8());
                slots_clone.lock().insert(
                    uid,
                    ImageSlot {
                        result: Some(result),
                        ffmpeg: ffmpeg_was_used,
                    },
                );
            });
        }
    }
    pub fn write_to_cache(&self, cache: &mut ThumbnailCache) {
        let mut slots = self.image_slots.lock();
        slots.retain(|&uid, slot| {
            if let Some(result) = slot.result.take() {
                match result {
                    Ok(buf) => {
                        let tex = imagebuf_to_sf_tex(buf);
                        cache.insert(
                            uid,
                            Thumbnail {
                                texture: Some(tex),
                                ffmpeg_loaded: slot.ffmpeg,
                            },
                        );
                    }
                    Err(e) => {
                        dlog!("Error loading thumbnail: {e}");
                        cache.insert(
                            uid,
                            Thumbnail {
                                texture: None,
                                ffmpeg_loaded: slot.ffmpeg,
                            },
                        );
                    }
                }
                false
            } else {
                true
            }
        });
    }
    pub fn busy_with(&self) -> Vec<entry::Id> {
        self.image_slots.lock().keys().copied().collect()
    }
}

/// Convert an `image` crate image to SFML `Texture`
///
/// # Panics
///
/// If the texture can't be created, it will panic. Shouldn't happen normally.
pub fn imagebuf_to_sf_tex(buf: ImageBuffer<Rgba<u8>, Vec<u8>>) -> FBox<Texture> {
    let (w, h) = buf.dimensions();
    let mut tex = Texture::new().unwrap();
    if tex.create(w, h).is_err() {
        panic!("Failed to create texture");
    }
    tex.update_from_pixels(&buf.into_raw(), w, h, 0, 0);
    tex
}
