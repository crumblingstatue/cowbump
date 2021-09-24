use crate::{
    entry::{self, Entry},
    filter_spec::FilterSpec,
    sequence::{self, Sequence},
    tag::{self, Tag},
};
use fnv::FnvHashMap;
use serde_derive::{Deserialize, Serialize};
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

use super::{serialization, EntryMap, EntrySet, Uid};

pub type Entries = EntryMap<Entry>;
pub type Tags = FnvHashMap<tag::Id, Tag>;
pub type Sequences = FnvHashMap<sequence::Id, Sequence>;

/// The database of all entries.
///
/// This is the data that is saved to disk between runs.
///
/// Note that this is not where any kind of sorting happens.
/// That happens at the view level. This just maps Uids to entries.
/// Nothing else.
#[derive(Default, Serialize, Deserialize)]
pub struct LocalDb {
    /// List of entries
    pub entries: Entries,
    /// List of tags
    pub tags: Tags,
    pub sequences: Sequences,
    uid_counter: Uid,
}

impl LocalDb {
    pub fn update_from_folder(&mut self, folder: &Path) -> anyhow::Result<()> {
        let wd = WalkDir::new(folder).sort_by(|a, b| a.file_name().cmp(b.file_name()));
        // Indices in the entries vector that correspond to valid entries that exist
        let mut valid_uids = EntrySet::default();

        for dir_entry in wd {
            let dir_entry = dir_entry?;
            if dir_entry.file_type().is_dir() {
                continue;
            }
            let dir_entry_path = dir_entry.into_path();
            let dir_entry_path = match dir_entry_path.strip_prefix(folder) {
                Ok(stripped) => stripped,
                Err(e) => {
                    eprintln!("Failed to add entry {:?}: {}", dir_entry_path, e);
                    continue;
                }
            };
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
            if file_name == DB_FILENAME || file_name == DB_BACKUP_FILENAME {
                should_add = false;
            }
            if should_add {
                eprintln!("Adding {}", dir_entry_path.display());
                let uid = entry::Id(self.new_uid());
                valid_uids.insert(uid);
                self.entries
                    .insert(uid, Entry::new(dir_entry_path.to_owned()));
            }
        }
        // Remove indices that don't correspond to valid entries
        self.entries.retain(|uid, en| {
            let keep = valid_uids.contains(uid);
            if !keep {
                eprintln!("Removing {}", en.path.display());
            }
            keep
        });
        Ok(())
    }
    pub fn add_tag_for(&mut self, entry: entry::Id, tag: tag::Id) {
        let tags = &mut self.entries.get_mut(&entry).unwrap().tags;
        tags.insert(tag);
    }
    pub fn add_tag_for_multi(&mut self, entries: &[entry::Id], tag: tag::Id) {
        for img in entries {
            self.add_tag_for(*img, tag);
        }
    }
    pub fn add_new_tag(&mut self, tag: Tag) -> tag::Id {
        let uid = tag::Id(self.new_uid());
        self.tags.insert(uid, tag);
        uid
    }
    pub(crate) fn add_new_tag_from_text(&mut self, tag_text: String) -> tag::Id {
        self.add_new_tag(Tag {
            names: vec![tag_text],
            implies: Default::default(),
        })
    }
    pub fn filter<'a>(&'a self, spec: &'a FilterSpec) -> impl Iterator<Item = entry::Id> + 'a {
        self.entries
            .iter()
            .filter_map(move |(&uid, en)| crate::entry::filter_map(uid, en, spec))
    }
    pub fn save_to_fs(&self) -> anyhow::Result<()> {
        let mut f = File::create(DB_FILENAME)?;
        serialization::write_local(self, &mut f)?;
        Ok(())
    }
    pub fn save_backup(&self) -> anyhow::Result<()> {
        let f = File::create(DB_BACKUP_FILENAME)?;
        serialization::write_local(self, f)?;
        Ok(())
    }
    pub fn load_from_fs() -> anyhow::Result<Self> {
        let f = File::open(DB_FILENAME)?;
        serialization::read_local(f)
    }
    pub fn load_backup(&mut self) -> anyhow::Result<()> {
        let f = File::open(DB_BACKUP_FILENAME)?;
        let new = serialization::read_local(f)?;
        *self = new;
        Ok(())
    }
    pub fn new_uid(&mut self) -> Uid {
        let uid = self.uid_counter;
        self.uid_counter += 1;
        uid
    }
    pub fn rename(&mut self, uid: entry::Id, new: &str) {
        let en = self.entries.get_mut(&uid).unwrap();
        pathbuf_rename_filename(&mut en.path, new);
    }

    pub(crate) fn resolve_tag(&self, word: &str) -> Option<tag::Id> {
        for (k, v) in &self.tags {
            if v.names.iter().any(|name| name == word) {
                return Some(*k);
            }
        }
        None
    }

    pub fn remove_tags(&mut self, tags_to_del: &[tag::Id]) {
        self.tags.retain(|uid, _| {
            if tags_to_del.contains(uid) {
                cleanse_tag_from_entries(&mut self.entries, *uid);
                false
            } else {
                true
            }
        });
    }

    pub(crate) fn add_new_sequence(&mut self, name: &str) -> sequence::Id {
        let uid = sequence::Id(self.new_uid());
        self.sequences.insert(uid, Sequence::new_with_name(name));
        uid
    }

    pub(crate) fn add_entries_to_sequence(&mut self, seq: sequence::Id, entries: &[entry::Id]) {
        // Do a default filename based sorting before adding
        let mut sorted = entries.to_owned();
        sorted.sort_by_key(|id| &self.entries[id].path);
        self.sequences.get_mut(&seq).unwrap().entries.extend(sorted);
    }

    pub(crate) fn find_related_sequences(&self, ids: &[entry::Id]) -> Vec<sequence::Id> {
        self.sequences
            .iter()
            .filter_map(|(k, v)| {
                if slice_contains_any_of(&v.entries, ids) {
                    Some(*k)
                } else {
                    None
                }
            })
            .collect()
    }
}

fn slice_contains_any_of<T: PartialEq>(haystack: &[T], needles: &[T]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn cleanse_tag_from_entries(entries: &mut Entries, tag_to_cleanse: tag::Id) {
    for en in entries.values_mut() {
        en.tags.retain(|&tag| tag != tag_to_cleanse)
    }
}

/// Rename the last component (filename) of a PathBuf, and rename it on the filesystem too.
fn pathbuf_rename_filename(buf: &mut PathBuf, new_name: &str) {
    let mut new_buf = buf.clone();
    new_buf.pop();
    new_buf.push(new_name);
    if let Err(e) = std::fs::rename(&buf, &new_buf) {
        eprintln!("Rename error: {}", e);
    }
    *buf = new_buf;
}

const DB_FILENAME: &str = "cowbump.db";
const DB_BACKUP_FILENAME: &str = "cowbump.db.bak";
