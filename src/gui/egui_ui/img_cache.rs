use {
    crate::entry,
    sf2g::{cpp::FBox, graphics::Texture},
    std::collections::VecDeque,
};

type ImageResult = Result<FBox<Texture>, anyhow::Error>;
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
}
