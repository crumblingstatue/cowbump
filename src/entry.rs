use db::Uid;
use std::path::PathBuf;

/// Path to an image, along with associated tags
#[derive(Serialize, Deserialize)]
pub struct Entry {
    /// Absolute path of the image
    pub path: PathBuf,
    pub tags: Vec<Uid>,
}

impl Entry {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            tags: Default::default(),
        }
    }
}
