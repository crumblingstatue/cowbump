use crate::entry::Entry;
use crate::tag::Tag;
use failure::Error;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::path::Path;
use walkdir::WalkDir;

/// The database of all entries.
///
/// This is the data that is saved to disk between runs.
#[derive(Default, Serialize, Deserialize)]
pub struct Db {
    /// List of entries
    pub entries: Vec<Entry>,
    /// List of tags
    pub tags: Vec<Tag>,
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
        let mut valid_indices = HashSet::new();

        for dir_entry in wd {
            let dir_entry = dir_entry?;
            if dir_entry.file_type().is_dir() {
                continue;
            }
            let dir_entry_path = dir_entry.into_path();
            let mut already_have = false;
            for (i, en) in self.entries.iter().enumerate() {
                if en.path == dir_entry_path {
                    already_have = true;
                    valid_indices.insert(i);
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
                valid_indices.insert(self.entries.len());
                self.entries.push(Entry::new(dir_entry_path));
            }
        }
        // Remove indices that don't correspond to valid images
        let mut i = 0;
        self.entries.retain(|en| {
            let keep = valid_indices.contains(&i);
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
        let tags = &mut self.entries[entry as usize].tags;
        if !tags.contains(&tag) {
            tags.push(tag);
            false
        } else {
            true
        }
    }
    pub fn add_new_tag(&mut self, tag: Tag) {
        self.tags.push(tag);
    }
    pub fn filter<'a>(&'a self, spec: &'a crate::FilterSpec) -> impl Iterator<Item = Uid> + 'a {
        self.entries.iter().enumerate().filter_map(move |en| {
            for required_tag in &spec.has_tags {
                if !en.1.tags.contains(required_tag) {
                    return None;
                }
            }
            Some(en.0 as Uid)
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
    /// Sort the entries alphabetically
    pub fn sort_entries(&mut self) {
        self.entries.sort_by(|en1, en2| en1.path.cmp(&en2.path));
    }
}

const DB_FILENAME: &str = "cowbump.db";
