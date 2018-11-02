use bincode;
use entry::Entry;
use failure::Error;
use std::fs::File;
use std::path::Path;
use tag::Tag;
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
    pub fn add_from_folder(&mut self, path: &Path) -> Result<(), Error> {
        let wd = WalkDir::new(path).sort_by(|a, b| a.file_name().cmp(b.file_name()));

        for dir_entry in wd {
            let dir_entry = dir_entry?;
            if dir_entry.file_type().is_dir() {
                continue;
            }
            let dir_entry_path = dir_entry.path();
            let already_have: bool = self
                .entries
                .iter()
                .any(|db_en| db_en.path == dir_entry_path);
            if !already_have {
                self.entries.push(Entry::new(dir_entry_path.to_owned()));
            }
        }
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
    pub fn filter<'a>(&'a self, spec: &'a ::FilterSpec) -> impl Iterator<Item = Uid> + 'a {
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
}

const DB_FILENAME: &str = "cowbump.db";
