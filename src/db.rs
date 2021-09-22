pub mod global;
pub mod local;
mod serialization;
use fnv::{FnvHashMap, FnvHashSet};
use std::collections::hash_map::Entry;

/// Unique identifier for entries/tags.
///
/// Use 64 bit so we can just keep indefinitely assigning new Uids without worry of running out.
pub type Uid = u64;
pub type UidSet = FnvHashSet<Uid>;
pub type UidMap<T> = FnvHashMap<Uid, T>;
pub type UidMapEntry<'a, T> = Entry<'a, Uid, T>;

pub trait UidSetExt {
    fn toggle(&mut self, uid: Uid);
    fn toggle_by(&mut self, uid: Uid, on: bool);
}

impl UidSetExt for UidSet {
    fn toggle(&mut self, uid: Uid) {
        self.toggle_by(uid, !self.contains(&uid))
    }
    fn toggle_by(&mut self, uid: Uid, on: bool) {
        if on {
            self.insert(uid);
        } else {
            self.remove(&uid);
        }
    }
}
