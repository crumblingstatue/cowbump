//! Misc entry/collection utilities

use crate::{collection::Collection, db::TagSet, entry};

pub fn common_tags(ids: &[entry::Id], coll: &Collection) -> TagSet {
    let mut set = TagSet::default();
    for &id in ids {
        for &tagid in &coll.entries[&id].tags {
            set.insert(tagid);
        }
    }
    set
}
