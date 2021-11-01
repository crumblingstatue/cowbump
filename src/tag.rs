use crate::db::{TagSet, Uid};
use serde_derive::{Deserialize, Serialize};

/// An identifiable quality that entries can be tagged by.
#[derive(Serialize, Deserialize, Debug)]
pub struct Tag {
    /// Names that map to this tag.
    ///
    /// For example, a tag that stands for `happy` could be mapped to by
    /// `happy`, `merry`, or `cheerful`.
    pub names: Vec<String>,
    /// Tags that this tag implies.
    ///
    /// For example, `elephant` might imply `pachyderm` and `animal`.
    pub implies: TagSet,
}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Id(pub Uid);
