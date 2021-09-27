use crate::{
    db::{TagSet, Uid},
    entry,
    filter_spec::FilterSpec,
};
use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

/// Path to an item we're interested in organizing, along with associated tags
#[derive(Serialize, Deserialize)]
pub struct Entry {
    /// Image path relative to collection root. Assumed to be unique within the collection.
    pub path: PathBuf,
    pub tags: TagSet,
}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Id(pub Uid);

impl Entry {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            tags: Default::default(),
        }
    }
    pub fn spec_satisfied(&self, spec: &FilterSpec) -> bool {
        if !self
            .path
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

pub fn filter_map(uid: entry::Id, entry: &Entry, spec: &FilterSpec) -> Option<entry::Id> {
    if entry.spec_satisfied(spec) {
        Some(uid)
    } else {
        None
    }
}
