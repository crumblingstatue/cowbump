use crate::entry::Entry;
use crate::tag::Tag;
use failure::Error;
use serde_derive::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::path::Path;
use walkdir::WalkDir;

/// The database of all entries.
///
/// This is the data that is saved to disk between runs.
#[derive(Default, Serialize, Deserialize)]
pub struct Db {
    /// List of entries
    pub entries: BTreeMap<Uid, Entry>,
    /// List of tags
    pub tags: BTreeMap<Uid, Tag>,
    uid_counter: Uid,
}

/// Unique identifier for entries/tags.
///
/// 32-bit is chosen because it is more compact than 64 bits,
/// but still large enough that we will not run out of new ids in practice.
pub type Uid = u32;

/// Special Uid value that represents "None".
///
/// It uses the max value of u32, which means it is not expected to have
/// as much unique items as that.
pub const UID_NONE: Uid = Uid::max_value();

impl Db {
    pub fn update_from_folder(&mut self, path: &Path) -> Result<(), Error> {
        let wd = WalkDir::new(path).sort_by(|a, b| a.file_name().cmp(b.file_name()));
        // Indices in the entries vector that correspond to valid images that exist
        let mut valid_uids = HashSet::new();

        for dir_entry in wd {
            let dir_entry = dir_entry?;
            if dir_entry.file_type().is_dir() {
                continue;
            }
            let dir_entry_path = dir_entry.into_path();
            let mut already_have = false;
            for (&uid, en) in &self.entries {
                if en.path == dir_entry_path {
                    already_have = true;
                    valid_uids.insert(uid);
                    break;
                }
            }
            let mut should_add = !already_have;
            let file_name = dir_entry_path.file_name().unwrap();
            if file_name == DB_FILENAME {
                should_add = false;
            }
            if should_add {
                eprintln!("Adding {}", dir_entry_path.display());
                let uid = self.new_uid();
                valid_uids.insert(uid);
                self.entries.insert(uid, Entry::new(dir_entry_path));
            }
        }
        // Remove indices that don't correspond to valid images
        let mut i = 0;
        self.entries.retain(|_uid, en| {
            let keep = valid_uids.contains(&i);
            if !keep {
                eprintln!("Removing {}", en.path.display());
            }
            i += 1;
            keep
        });
        Ok(())
    }
    /// Add a tag for an entry.
    ///
    /// Returns whether the entry already had the tag, so it didn't need to be added.
    pub fn add_tag_for(&mut self, entry: Uid, tag: Uid) -> bool {
        let tags = &mut self.entries.get_mut(&entry).unwrap().tags;
        if !tags.contains(&tag) {
            tags.push(tag);
            false
        } else {
            true
        }
    }
    pub fn add_new_tag(&mut self, tag: Tag) {
        self.tags.insert(self.tags.len() as Uid, tag);
    }
    pub fn filter<'a>(&'a self, spec: &'a crate::FilterSpec) -> impl Iterator<Item = Uid> + 'a {
        self.entries
            .iter()
            .enumerate()
            .filter_map(move |(_uid, en)| {
                if !en
                    .1
                    .path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&spec.substring_match)
                {
                    return None;
                }
                for required_tag in &spec.has_tags {
                    if !en.1.tags.contains(required_tag) {
                        return None;
                    }
                }
                Some(*en.0)
            })
    }
    pub fn save_to_fs(&self) -> Result<(), Error> {
        let mut f = File::create(DB_FILENAME)?;
        bincode::serialize_into(&mut f, self)?;
        Ok(())
    }
    pub fn load_from_fs() -> Result<Self, Error> {
        let mut f = File::open(DB_FILENAME)?;
        Ok(bincode::deserialize_from(&mut f)?)
    }
    pub fn new_uid(&mut self) -> Uid {
        let uid = self.uid_counter;
        self.uid_counter += 1;
        uid
    }
}

const DB_FILENAME: &str = "cowbump.db";
