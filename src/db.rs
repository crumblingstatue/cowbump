pub mod global;
pub mod local;
mod serialization;

/// Unique identifier for entries/tags.
///
/// Use 64 bit so we can just keep indefinitely assigning new Uids without worry of running out.
pub type Uid = u64;
