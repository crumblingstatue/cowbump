use db::{Uid, UID_NONE};
use std::sync::{Arc, Mutex};
use image::{self, FilterType, ImageBuffer, ImageResult, Rgba};
use std::fs::File;
use std::io::prelude::*;
use sfml::graphics::{Texture, TextureBox};
use std::collections::HashMap;
use std::path::Path;

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
                let mut f = File::open(name).unwrap();
                // Try to load file as efficiently as possible, using a single compact allocation.
                // We trust that `len` returned by metadata is correct.
                let len = f.metadata().unwrap().len() as usize;
                let mut buf = Vec::with_capacity(len as usize);
                unsafe {
                    // Set length for `read_exact` to fill.
                    buf.set_len(len);
                    // This should fill all the uninitialized buffer.
                    f.read_exact(&mut buf).unwrap();
                }
                // Because loading images is memory intensive, and we might load multiple images
                // in parallel, we eagerly drop some stuff in order to free up memory
                // as soon as possible.
                drop(f);
                let image_result = image::load_from_memory(&buf);
                drop(buf);
                let result =
                    image_result.map(|i| i.resize(size, size, FilterType::Triangle).to_rgba());
                *image_slot.lock().unwrap() = Some(result);
            });
        }
    }
    pub fn write_to_cache(&mut self, cache: &mut HashMap<Uid, Option<TextureBox>>) {
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
}
