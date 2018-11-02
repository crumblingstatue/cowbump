use crate::db::Uid;

/// An identifiable quality that entries can be tagged by.
#[derive(Serialize, Deserialize)]
pub struct Tag {
    /// Names that map to this tag.
    ///
    /// For example, a tag that stands for `happy` could be mapped to by
    /// `happy`, `merry`, or `cheerful`.
    pub names: Vec<String>,
    /// Tags that this tag implies.
    ///
    /// For example, `elephant` might imply `pachyderm` and `animal`.
    pub implies: Vec<Uid>,
}
