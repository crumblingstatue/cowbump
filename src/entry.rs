use crate::db::Uid;
use serde_derive::{Deserialize, Serialize};
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
    pub fn spec_satisfied(&self, spec: &crate::FilterSpec) -> bool {
        if !self
            .path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_lowercase()
            .contains(&spec.filename_substring)
        {
            return false;
        }
        for required_tag in &spec.has_tags {
            if !self.tags.contains(required_tag) {
                return false;
            }
        }
        for required_no_tag in &spec.doesnt_have_tags {
            if self.tags.contains(required_no_tag) {
                return false;
            }
        }
        if spec.doesnt_have_any_tags && !self.tags.is_empty() {
            return false;
        }
        true
    }
}

pub fn image_filter_map(uid: Uid, entry: &Entry, spec: &crate::FilterSpec) -> Option<Uid> {
    if entry.spec_satisfied(spec) {
        Some(uid)
    } else {
        None
    }
}
