use {
    crate::{
        db::{EntryMap, EntrySet, FolderChanges, Uid, UidCounter},
        dlog,
        entry::{self, Entry},
        filter_reqs::Requirements,
        folder_scan::walkdir,
        preferences,
        sequence::{self, Sequence},
        tag::{self, Tag},
    },
    anyhow::{Context, bail},
    fnv::FnvHashMap,
    serde_derive::{Deserialize, Serialize},
    std::{
        borrow::Cow,
        ffi::OsStr,
        path::{Path, PathBuf},
        sync::mpsc::Receiver,
    },
    thiserror::Error,
};

pub type Entries = EntryMap<Entry>;
pub type Tags = FnvHashMap<tag::Id, Tag>;
pub type Sequences = FnvHashMap<sequence::Id, Sequence>;
pub type TagSpecificApps = FnvHashMap<tag::Id, preferences::AppId>;

pub trait TagsExt {
    fn first_name_of(&self, id: &tag::Id) -> Cow<str>;
}

impl TagsExt for Tags {
    fn first_name_of(&self, id: &tag::Id) -> Cow<str> {
        match self.get(id) {
            Some(tag) => tag.first_name().into(),
            None => format!("<dangling:{id:?}>").into(),
        }
    }
}

/// A collection of entries.
///
/// Each collection has a root that all the entries stem from.
#[derive(Serialize, Deserialize)]
pub struct Collection {
    /// List of entries
    pub entries: Entries,
    /// List of tags
    pub tags: Tags,
    pub sequences: Sequences,
    #[serde(default)]
    pub tag_specific_apps: TagSpecificApps,
    /// Extensions that are ignored when updating from folder contents
    #[serde(default)]
    pub ignored_extensions: Vec<String>,
}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Id(pub Uid);

impl Collection {
    pub fn make_new(uid_counter: &mut UidCounter, paths: &[impl AsRef<Path>]) -> Self {
        let mut coll = Collection {
            entries: Entries::default(),
            tags: Tags::default(),
            sequences: Sequences::default(),
            tag_specific_apps: TagSpecificApps::default(),
            ignored_extensions: Vec::new(),
        };
        coll.update_from_paths(uid_counter, paths);
        coll
    }
    pub fn update_from_paths(&mut self, uid_counter: &mut UidCounter, paths: &[impl AsRef<Path>]) {
        // Indices in the entries vector that correspond to valid entries that exist
        let mut valid_uids = EntrySet::default();

        for path in paths {
            let path = path.as_ref();
            let mut already_have = false;
            for (&uid, en) in &self.entries {
                if en.path == path {
                    already_have = true;
                    valid_uids.insert(uid);
                    break;
                }
            }
            let should_add = !already_have;
            if should_add {
                let uid = entry::Id(uid_counter.next());
                valid_uids.insert(uid);
                self.entries.insert(uid, Entry::new(path.to_owned()));
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
    }
    pub fn add_tag_for(&mut self, entry: entry::Id, tag: tag::Id) -> Result<(), AddTagError> {
        match self.entries.get_mut(&entry) {
            Some(en) => {
                en.tags.insert(tag);
                Ok(())
            }
            None => Err(AddTagError),
        }
    }
    pub fn add_tag_for_multi(
        &mut self,
        entries: &[entry::Id],
        tag: tag::Id,
    ) -> Result<(), AddTagError> {
        for img in entries {
            self.add_tag_for(*img, tag)?;
        }
        Ok(())
    }
    fn add_new_tag(&mut self, tag: Tag, uid_counter: &mut UidCounter) -> tag::Id {
        let uid = tag::Id(uid_counter.next());
        self.tags.insert(uid, tag);
        uid
    }
    /// Returns `None` if said tag already exists
    pub(crate) fn add_new_tag_from_text(
        &mut self,
        mut tag_text: String,
        uid_counter: &mut UidCounter,
    ) -> Option<tag::Id> {
        // Ensure we can only insert lowercase tags
        tag_text.make_ascii_lowercase();
        if self.has_text_as_tag_name(&tag_text) {
            return None;
        }
        Some(self.add_new_tag(
            Tag {
                names: vec![tag_text],
                implies: Default::default(),
            },
            uid_counter,
        ))
    }
    pub fn filter<'a>(&'a self, reqs: &'a Requirements) -> impl Iterator<Item = entry::Id> + 'a {
        self.entries.iter().filter_map(move |(&uid, en)| {
            entry::filter_map(uid, en, reqs, &self.tags, &self.sequences)
        })
    }
    pub fn rename(&mut self, uid: entry::Id, new: &str) -> anyhow::Result<()> {
        let en = self.entries.get_mut(&uid).context("Couldn't get entry")?;
        pathbuf_rename_filename(&mut en.path, new)?;
        Ok(())
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
    /// Adds the specified entries to the specified sequence
    ///
    /// # Panics
    ///
    /// Panics if `seq` r `entries` refer to dangling ids.
    pub(crate) fn add_entries_to_sequence(&mut self, seq: sequence::Id, entries: &[entry::Id]) {
        // Do a default filename based sorting before adding
        let mut sorted = entries.to_owned();
        sorted.sort_by_key(|id| self.entries.get(id).map(|en| &en.path));
        self.sequences.get_mut(&seq).unwrap().entries.extend(sorted);
    }

    pub(crate) fn find_related_sequences(&self, ids: &[entry::Id]) -> Vec<sequence::Id> {
        self.related_seqs_of(ids).collect()
    }

    fn related_seqs_of<'a>(
        &'a self,
        ids: &'a [entry::Id],
    ) -> impl Iterator<Item = sequence::Id> + 'a {
        self.sequences.iter().filter_map(|(k, v)| {
            if slice_contains_any_of(&v.entries, ids) {
                Some(*k)
            } else {
                None
            }
        })
    }

    pub(crate) fn get_first_related_sequence_of(&self, id: entry::Id) -> Option<&Sequence> {
        self.related_seqs_of(&[id])
            .next()
            .and_then(|id| self.sequences.get(&id))
    }

    pub(crate) fn scan_changes(&self, root: PathBuf) -> Receiver<anyhow::Result<FolderChanges>> {
        let paths = self
            .entries
            .values()
            .map(|en| en.path.to_path_buf())
            .collect::<Vec<_>>();
        let ign_ext = self.ignored_extensions.clone();
        let (send, recv) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let changes = scan_changes(&root, paths, &ign_ext);
            if let Err(e) = send.send(changes) {
                dlog!("Failed to send folder changes: {e}");
            }
        });
        recv
    }

    pub(crate) fn apply_changes(
        &mut self,
        changes: &FolderChanges,
        uid_counter: &mut UidCounter,
        mut callback: impl FnMut(&Path, entry::Id),
    ) {
        for path in &changes.add {
            let id = self.add_new_entry(path.clone(), uid_counter);
            callback(path, id);
        }
        self.entries
            .retain(|_k, en| !changes.remove.contains(&en.path));
    }

    fn add_new_entry(&mut self, path: PathBuf, uid_counter: &mut UidCounter) -> entry::Id {
        let uid = entry::Id(uid_counter.next());
        self.entries.insert(uid, Entry::new(path));
        uid
    }
    /// Check if we have the specific text as a tag name in the tag database
    fn has_text_as_tag_name(&self, tag_text: &str) -> bool {
        self.tags
            .values()
            .any(|tag| tag.names.iter().any(|text| text == tag_text))
    }
    /// Merge `merge` into `into`, replacing all references to it with `into`.
    ///
    /// 1. Replace all references of `merge` with `into`
    /// 2. Merge all the names of `merge` into `into`
    /// 3. Finally, remove `merge`
    pub(crate) fn merge_tags(&mut self, merge: tag::Id, into: tag::Id) -> anyhow::Result<()> {
        self.replace_tag_refs(merge, into);
        // Merge names
        {
            let [Some(merge), Some(into)] = self.tags.get_many_mut([&merge, &into]) else {
                bail!("Couldn't get tags for merge operation");
            };
            into.names.append(&mut merge.names);
        }
        self.tags.remove(&merge);
        Ok(())
    }
    fn replace_tag_refs(&mut self, replace: tag::Id, with: tag::Id) {
        // Entries
        for en in self.entries.values_mut() {
            en.replace_tag(replace, with);
        }
        // Tag-implies
        for tag in self.tags.values_mut() {
            tag.replace_imply(replace, with);
        }
    }
}

pub fn scan_changes(
    root: &Path,
    coll_paths: Vec<PathBuf>,
    ignored_extensions: &[String],
) -> anyhow::Result<FolderChanges> {
    let wd = walkdir(root);
    let mut add = Vec::new();
    let mut remove = Vec::new();
    // Scan for additions (paths we don't have)
    for dir_entry in wd {
        let dir_entry = dir_entry?;
        if dir_entry.file_type().is_dir() {
            continue;
        }
        let ignored_ext = dir_entry.path().extension().is_some_and(|ext| {
            ignored_extensions
                .iter()
                .any(|ign_ext| ext == AsRef::<OsStr>::as_ref(ign_ext))
        });
        if ignored_ext {
            continue;
        }
        let dir_entry_path = dir_entry.into_path();
        let dir_entry_path = match dir_entry_path.strip_prefix(root) {
            Ok(stripped) => stripped,
            Err(e) => {
                eprintln!("Failed to add entry {dir_entry_path:?}: {e}");
                continue;
            }
        };
        if !coll_paths.iter().any(|p| p == dir_entry_path) {
            add.push(dir_entry_path.to_owned());
        }
    }
    // Scan for removes (paths we have but fs doesn't have)
    for path in coll_paths {
        if !root.join(&path).exists() {
            remove.push(path);
        }
    }
    Ok(FolderChanges { add, remove })
}

#[derive(Debug, Error)]
#[error("Failed to add tag")]
pub struct AddTagError;

fn slice_contains_any_of<T: PartialEq>(haystack: &[T], needles: &[T]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn cleanse_tag_from_entries(entries: &mut Entries, tag_to_cleanse: tag::Id) {
    for en in entries.values_mut() {
        en.tags.retain(|&tag| tag != tag_to_cleanse);
    }
}

/// Rename the last component (filename) of a `PathBuf`, and rename it on the filesystem too.
fn pathbuf_rename_filename(buf: &mut PathBuf, new_name: &str) -> anyhow::Result<()> {
    let mut new_buf = buf.clone();
    new_buf.pop();
    new_buf.push(new_name);
    if new_buf.exists() {
        bail!("Destination file already exists");
    }
    std::fs::rename(&*buf, &new_buf)?;
    *buf = new_buf;
    Ok(())
}
