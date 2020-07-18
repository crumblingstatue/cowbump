use crate::{
    db::{Uid, UID_NONE},
    gui::ThumbnailCache,
};
use image::{self, imageops::FilterType, ImageBuffer, ImageResult, Rgba};
use sfml::graphics::Texture;
use std::path::Path;
use std::sync::{Arc, Mutex};

type RgbaBuf = ImageBuffer<Rgba<u8>, Vec<u8>>;

/// Loads images on a separate thread, one at a time.
pub struct ThumbnailLoader {
    busy_with: Uid,
    image_slot: Arc<Mutex<Option<ImageResult<RgbaBuf>>>>,
}

impl Default for ThumbnailLoader {
    fn default() -> Self {
        Self {
            busy_with: UID_NONE,
            image_slot: Default::default(),
        }
    }
}

impl ThumbnailLoader {
    pub fn request(&mut self, name: &Path, size: u32, uid: Uid) {
        if self.busy_with == UID_NONE {
            self.busy_with = uid;
            let image_slot = Arc::clone(&self.image_slot);
            let name = name.to_owned();
            ::std::thread::spawn(move || {
                let data = match std::fs::read(name) {
                    Ok(data) => data,
                    Err(e) => {
                        *image_slot.lock().unwrap() = Some(Err(image::ImageError::IoError(e)));
                        return;
                    }
                };
                let image_result = image::load_from_memory(&data);
                let result =
                    image_result.map(|i| i.resize(size, size, FilterType::Triangle).to_rgba());
                *image_slot.lock().unwrap() = Some(result);
            });
        }
    }
    pub fn write_to_cache(&mut self, cache: &mut ThumbnailCache) {
        if let Some(result) = self.image_slot.lock().unwrap().take() {
            match result {
                Ok(buf) => {
                    let (w, h) = buf.dimensions();
                    let mut tex = Texture::new(w, h).unwrap();
                    unsafe {
                        tex.update_from_pixels(&buf.into_raw(), w, h, 0, 0);
                    }
                    cache.insert(self.busy_with, Some(tex));
                }
                Err(_) => {
                    cache.insert(self.busy_with, None);
                }
            }
            self.busy_with = UID_NONE;
        }
    }
    pub fn busy_with(&self) -> Uid {
        self.busy_with
    }
}
