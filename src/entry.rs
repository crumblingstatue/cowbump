use crate::{
    collection::Tags,
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
    pub fn spec_satisfied(&self, spec: &FilterSpec, tags: &Tags) -> bool {
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
                // Let's say we searched for 'pachyderm'.
                // This entry doesn't contain 'pachyderm', but it contains 'elephant',
                // which implies 'pachyderm'.
                // Therefore we need to check for each tag that the required tag implies
                // and check if we contain any of them.
                let contains_any_implied = self.tags.iter().any(|my_tag| {
                    let tag = &tags[my_tag];
                    tag.implies.contains(required_tag)
                });
                if !contains_any_implied {
                    return false;
                }
            }
        }
        for required_no_tag in &spec.doesnt_have_tags {
            if self.tags.contains(required_no_tag) {
                return false;
            }
            // The same implies-relation must be done for excluded tags.
            // The idea is that this entry must not contain any tags
            // that implies this excluded-tag.
            let contains_any_implied = self.tags.iter().any(|my_tag| {
                let tag = &tags[my_tag];
                tag.implies.contains(required_no_tag)
            });
            if contains_any_implied {
                return false;
            }
        }
        if spec.doesnt_have_any_tags && !self.tags.is_empty() {
            return false;
        }
        true
    }
}

pub fn filter_map(
    uid: entry::Id,
    entry: &Entry,
    spec: &FilterSpec,
    tags: &Tags,
) -> Option<entry::Id> {
    if entry.spec_satisfied(spec, tags) {
        Some(uid)
    } else {
        None
    }
}
