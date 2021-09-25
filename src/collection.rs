use crate::{
    db::{EntryMap, EntrySet, Uid, UidCounter},
    entry::{self, Entry},
    filter_spec::FilterSpec,
    sequence::{self, Sequence},
    tag::{self, Tag},
};
use fnv::FnvHashMap;
use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;
use walkdir::WalkDir;

pub type Entries = EntryMap<Entry>;
pub type Tags = FnvHashMap<tag::Id, Tag>;
pub type Sequences = FnvHashMap<sequence::Id, Sequence>;

/// A collection of entries.
///
/// Each collection has a root that all the entries stem from.
#[derive(Serialize, Deserialize)]
pub struct Collection {
    /// The root that all entries stem from
    pub root_path: PathBuf,
    /// List of entries
    pub entries: Entries,
    /// List of tags
    pub tags: Tags,
    pub sequences: Sequences,
}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
pub struct Id(pub Uid);

impl Collection {
    pub fn from_root(root_path: PathBuf, uid_counter: &mut UidCounter) -> anyhow::Result<Self> {
        let mut coll = Collection {
            root_path,
            entries: Entries::default(),
            tags: Tags::default(),
            sequences: Sequences::default(),
        };
        coll.update_from_fs(uid_counter)?;
        Ok(coll)
    }
    pub fn update_from_fs(&mut self, uid_counter: &mut UidCounter) -> anyhow::Result<()> {
        let wd = WalkDir::new(&self.root_path).sort_by(|a, b| a.file_name().cmp(b.file_name()));
        // Indices in the entries vector that correspond to valid entries that exist
        let mut valid_uids = EntrySet::default();

        for dir_entry in wd {
            let dir_entry = dir_entry?;
            if dir_entry.file_type().is_dir() {
                continue;
            }
            let dir_entry_path = dir_entry.into_path();
            let dir_entry_path = match dir_entry_path.strip_prefix(&self.root_path) {
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
            let should_add = !already_have;
            let _file_name = dir_entry_path.file_name().unwrap();
            if should_add {
                eprintln!("Adding {}", dir_entry_path.display());
                let uid = entry::Id(uid_counter.next());
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
    pub fn add_new_tag(&mut self, tag: Tag, uid_counter: &mut UidCounter) -> tag::Id {
        let uid = tag::Id(uid_counter.next());
        self.tags.insert(uid, tag);
        uid
    }
    pub(crate) fn add_new_tag_from_text(
        &mut self,
        tag_text: String,
        uid_counter: &mut UidCounter,
    ) -> tag::Id {
        self.add_new_tag(
            Tag {
                names: vec![tag_text],
                implies: Default::default(),
            },
            uid_counter,
        )
    }
    pub fn filter<'a>(&'a self, spec: &'a FilterSpec) -> impl Iterator<Item = entry::Id> + 'a {
        self.entries
            .iter()
            .filter_map(move |(&uid, en)| crate::entry::filter_map(uid, en, spec))
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

    pub(crate) fn add_new_sequence(
        &mut self,
        name: &str,
        uid_counter: &mut UidCounter,
    ) -> sequence::Id {
        let uid = sequence::Id(uid_counter.next());
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
