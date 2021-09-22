pub mod global;
pub mod local;
mod serialization;
use fnv::{FnvHashMap, FnvHashSet};

use crate::{entry, tag};

/// Unique identifier for entries/tags.
///
/// Use 64 bit so we can just keep indefinitely assigning new Uids without worry of running out.
pub type Uid = u64;
pub type EntrySet = FnvHashSet<entry::Id>;
pub type EntryMap<V> = FnvHashMap<entry::Id, V>;
pub type TagSet = FnvHashSet<tag::Id>;
