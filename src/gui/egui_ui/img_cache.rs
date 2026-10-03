use {
    crate::{
        entry::{self, Entry},
        gui::thumbnail_loader::imagebuf_to_sf_tex,
    },
    sf2g::{cpp::FBox, graphics::Texture},
    std::{collections::VecDeque, path::Path},
};

pub type ImageResult = Result<FBox<Texture>, anyhow::Error>;
type CacheKvPair = (entry::Id, ImageResult);

pub struct ImageCache {
    img_results: VecDeque<CacheKvPair>,
    capacity: usize,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self {
            img_results: Default::default(),
            capacity: 100,
        }
    }
}

impl ImageCache {
    pub fn get(&self, id: entry::Id) -> Option<&ImageResult> {
        self.img_results
            .iter()
            .find_map(|kvpair| (kvpair.0 == id).then_some(&kvpair.1))
    }
    pub fn insert(&mut self, kvpair: CacheKvPair) {
        self.img_results.push_back(kvpair);
        if self.img_results.len() > self.capacity {
            self.img_results.pop_front();
        }
    }
    pub fn load(&mut self, id: entry::Id, path: &Path) {
        let data = match std::fs::read(path) {
            Ok(data) => data,
            Err(e) => {
                crate::dlog!("Error loading image: {e}");
                return;
            }
        };
        match image::load_from_memory(&data) {
            Ok(img) => {
                let tex = imagebuf_to_sf_tex(img.to_rgba8());
                self.insert((id, Ok(tex)));
            }
            Err(e) => {
                self.insert((id, Err(anyhow::anyhow!(e))));
            }
        }
    }
    pub fn fetch(&mut self, id: entry::Id, entry: &Entry) -> (&ImageResult, bool) {
        match self.get(id) {
            Some(result) => (result, false),
            None => {
                self.load(id, &entry.path);
                #[expect(
                    clippy::missing_panics_doc,
                    reason = "Doesn't panic, we load before the unwrap"
                )]
                (self.get(id).unwrap(), true)
            }
        }
    }
}
