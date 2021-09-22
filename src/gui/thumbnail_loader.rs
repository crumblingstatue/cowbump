use crate::{
    db::{Uid, UidMap, UidMapEntry},
    gui::ThumbnailCache,
};
use image::{self, imageops::FilterType, ImageBuffer, ImageResult, Rgba};
use sfml::graphics::Texture;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

type RgbaBuf = ImageBuffer<Rgba<u8>, Vec<u8>>;
type ImageSlot = Option<ImageResult<RgbaBuf>>;

/// Loads images on a separate thread, one at a time.
#[derive(Default)]
pub struct ThumbnailLoader {
    image_slots: Arc<Mutex<UidMap<ImageSlot>>>,
}

impl ThumbnailLoader {
    pub fn request(&mut self, name: &Path, size: u32, uid: Uid) {
        let mut slots = self.image_slots.lock().unwrap();
        if let UidMapEntry::Vacant(e) = slots.entry(uid) {
            e.insert(None);
            let slots_clone = Arc::clone(&self.image_slots);
            let name = name.to_owned();
            ::std::thread::spawn(move || {
                let data = match std::fs::read(name) {
                    Ok(data) => data,
                    Err(e) => {
                        slots_clone
                            .lock()
                            .unwrap()
                            .insert(uid, Some(Err(image::ImageError::IoError(e))));
                        return;
                    }
                };
                let image_result = image::load_from_memory(&data);
                let result =
                    image_result.map(|i| i.resize(size, size, FilterType::Triangle).to_rgba8());
                slots_clone.lock().unwrap().insert(uid, Some(result));
            });
        }
    }
    pub fn write_to_cache(&mut self, cache: &mut ThumbnailCache) {
        let mut slots = self.image_slots.lock().unwrap();
        slots.retain(|&uid, slot| {
            if let Some(result) = slot.take() {
                match result {
                    Ok(buf) => {
                        let (w, h) = buf.dimensions();
                        let mut tex = Texture::new().unwrap();
                        if !tex.create(w, h) {
                            panic!("Failed to create texture");
                        }
                        unsafe {
                            tex.update_from_pixels(&buf.into_raw(), w, h, 0, 0);
                        }
                        cache.insert(uid, Some(tex));
                    }
                    Err(_) => {
                        cache.insert(uid, None);
                    }
                }
                false
            } else {
                true
            }
        });
    }
    pub fn busy_with(&self) -> Vec<Uid> {
        self.image_slots.lock().unwrap().keys().cloned().collect()
    }
}
