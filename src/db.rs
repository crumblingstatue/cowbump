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
