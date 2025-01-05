use {
    crate::{db::EntryMap, dlog, entry, gui::ThumbnailCache},
    egui_sfml::sfml::{cpp::FBox, graphics::Texture},
    image::{ImageBuffer, ImageResult, Rgba, imageops::FilterType},
    parking_lot::Mutex,
    std::{collections::hash_map, path::Path, process::Command, sync::Arc},
};

type RgbaBuf = ImageBuffer<Rgba<u8>, Vec<u8>>;
type ImageSlot = Option<ImageResult<RgbaBuf>>;

/// Loads images on a separate thread, one at a time.
#[derive(Default)]
pub struct ThumbnailLoader {
    image_slots: Arc<Mutex<EntryMap<ImageSlot>>>,
}

impl ThumbnailLoader {
    pub fn request(&self, name: &Path, size: u32, uid: entry::Id) {
        let mut slots = self.image_slots.lock();
        if let hash_map::Entry::Vacant(e) = slots.entry(uid) {
            e.insert(None);
            let slots_clone = Arc::clone(&self.image_slots);
            let name = name.to_owned();
            ::std::thread::spawn(move || {
                let data = match std::fs::read(&name) {
                    Ok(data) => data,
                    Err(e) => {
                        slots_clone
                            .lock()
                            .insert(uid, Some(Err(image::ImageError::IoError(e))));
                        return;
                    }
                };
                let mut image_result = image::load_from_memory(&data);
                if image_result.is_err() {
                    let result = Command::new("ffmpeg")
                        .args(["-y", "-i"])
                        .arg(&name)
                        .args(["-frames:v", "1", "-f", "image2pipe", "/dev/stdout"])
                        .output();
                    match result {
                        Ok(out) => {
                            image_result = image::load_from_memory(&out.stdout);
                        }
                        Err(e) => {
                            dlog!("Failed to generate thumbnail with ffmpeg: {e}");
                        }
                    }
                }
                let result =
                    image_result.map(|i| i.resize(size, size, FilterType::Triangle).to_rgba8());
                slots_clone.lock().insert(uid, Some(result));
            });
        }
    }
    pub fn write_to_cache(&self, cache: &mut ThumbnailCache) {
        let mut slots = self.image_slots.lock();
        slots.retain(|&uid, slot| {
            if let Some(result) = slot.take() {
                match result {
                    Ok(buf) => {
                        let tex = imagebuf_to_sf_tex(buf);
                        cache.insert(uid, Some(tex));
                    }
                    Err(e) => {
                        dlog!("Error loading thumbnail: {e}");
                        cache.insert(uid, None);
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
