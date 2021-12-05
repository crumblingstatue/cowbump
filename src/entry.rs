use crate::{
    collection::{Sequences, Tags},
    db::{TagSet, Uid},
    entry,
    filter_spec::FilterSpec,
    gui::debug_log::dlog,
    tag,
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
    pub fn spec_satisfied(
        &self,
        id: Id,
        spec: &FilterSpec,
        tags: &Tags,
        sequences: &Sequences,
    ) -> bool {
        if !self
            .path
            .to_string_lossy()
            .to_lowercase()
            .contains(&spec.filename_substring)
        {
            return false;
        }
        for required_tag in &spec.has_tags {
            if !self.satisfies_required_tag(*required_tag, tags) {
                return false;
            }
        }
        for required_no_tag in &spec.doesnt_have_tags {
            if self.satisfies_required_tag(*required_no_tag, tags) {
                return false;
            }
        }
        if spec.doesnt_have_any_tags && !self.tags.is_empty() {
            return false;
        }
        let part_of_seq = sequences.values().any(|seq| seq.contains_entry(id));
        if (spec.part_of_seq && !part_of_seq) || (spec.not_part_of_seq && part_of_seq) {
            return false;
        }
        true
    }
    fn satisfies_required_tag(&self, required_tag_id: tag::Id, tags: &Tags) -> bool {
        self.tags
            .iter()
            .any(|tag_id| tag_satisfies_required_tag(*tag_id, required_tag_id, tags, &mut 0))
    }
}

fn tag_satisfies_required_tag(
    tag_id: tag::Id,
    required_tag_id: tag::Id,
    tags: &Tags,
    depth: &mut u32,
) -> bool {
    *depth += 1;
    if *depth == 10 {
        dlog!(
            "Tag satisfies depth limit exceeded. Aborting [tag: {}, required: {}]",
            tags[&tag_id].names[0],
            tags[&required_tag_id].names[0]
        );
        return false;
    }
    if tag_id == required_tag_id {
        return true;
    }
    // Check if any implied tags satisfy required
    let tag = &tags[&tag_id];
    tag.implies.iter().any(|implied_tag_id| {
        tag_satisfies_required_tag(*implied_tag_id, required_tag_id, tags, depth)
    })
}

pub fn filter_map(
    uid: entry::Id,
    entry: &Entry,
    spec: &FilterSpec,
    tags: &Tags,
    sequences: &Sequences,
) -> Option<entry::Id> {
    if entry.spec_satisfied(uid, spec, tags, sequences) {
        Some(uid)
    } else {
        None
    }
}
