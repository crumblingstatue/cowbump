pub mod global;
pub mod local;
mod serialization;
use fnv::{FnvHashMap, FnvHashSet};
use std::{collections::hash_map::Entry, hash::Hash};

/// Unique identifier for entries/tags.
///
/// Use 64 bit so we can just keep indefinitely assigning new Uids without worry of running out.
pub type Uid = u64;
pub type UidSet = FnvHashSet<Uid>;
pub type UidMap<T> = FnvHashMap<Uid, T>;
pub type UidMapEntry<'a, T> = Entry<'a, Uid, T>;

pub trait SetExt<T> {
    fn toggle(&mut self, item: T);
    fn toggle_by(&mut self, uid: T, on: bool);
}

impl<T: Hash + Eq> SetExt<T> for FnvHashSet<T> {
    fn toggle(&mut self, item: T) {
        let on = !self.contains(&item);
        self.toggle_by(item, on);
    }
    fn toggle_by(&mut self, item: T, on: bool) {
        if on {
            self.insert(item);
        } else {
            self.remove(&item);
        }
    }
}
