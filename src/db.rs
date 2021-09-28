pub mod global;
use std::path::{Path, PathBuf};

use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    collection::{self, Collection},
    entry,
    preferences::Preferences,
    recently_used_list::RecentlyUsedList,
    serialization, tag,
};

/// Unique identifier for entries/tags.
///
/// Use 64 bit so we can just keep indefinitely assigning new Uids without worry of running out.
pub type Uid = u64;
pub type EntrySet = FnvHashSet<entry::Id>;
pub type EntryMap<V> = FnvHashMap<entry::Id, V>;
pub type TagSet = FnvHashSet<tag::Id>;
pub type CollMap<V> = FnvHashMap<collection::Id, V>;

use anyhow::Context;
use directories::ProjectDirs;
use serde_derive::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct Db {
    pub uid_counter: UidCounter,
    pub collections: CollMap<Collection>,
    pub preferences: Preferences,
    /// History of last opened collections
    pub recent: RecentlyUsedList<collection::Id>,
    #[serde(skip)]
    data_dir: PathBuf,
}

#[derive(Default, Serialize, Deserialize)]
pub struct UidCounter(Uid);

impl UidCounter {
    pub fn next(&mut self) -> Uid {
        let uid = self.0;
        self.0 += 1;
        uid
    }
}

impl Db {
    pub fn load() -> anyhow::Result<Self> {
        let dirs = ProjectDirs::from("", "crumblingstatue", "cowbump")
            .context("Failed to retrieve project dirs")?;
        let data_dir = dirs.data_dir();
        if !data_dir.exists() {
            std::fs::create_dir_all(data_dir)?;
        }
        let path = data_dir.join(FILENAME);
        let mut db = if path.exists() {
            serialization::read_from_file(path)?
        } else {
            Self::default()
        };
        db.data_dir = data_dir.to_owned();
        Ok(db)
    }
    pub fn insert_collection(&mut self, collection: Collection) -> collection::Id {
        let key = collection::Id(self.uid_counter.next());
        self.collections.insert(key, collection);
        key
    }
    pub fn save(&self) -> anyhow::Result<()> {
        serialization::write_to_file(self, self.data_dir.join(FILENAME))
    }
    pub fn save_backup(&self) -> anyhow::Result<()> {
        serialization::write_to_file(self, self.data_dir.join(BACKUP_FILENAME))
    }
    pub fn load_backup(&mut self) -> anyhow::Result<()> {
        let new = serialization::read_from_file(self.data_dir.join(BACKUP_FILENAME))?;
        *self = new;
        Ok(())
    }

    pub(crate) fn find_collection_by_path(&self, path: &Path) -> Option<collection::Id> {
        self.collections
            .iter()
            .find(|(_k, v)| v.root_path == path)
            .map(|(k, _v)| *k)
    }

    /*pub(crate) fn update_collection(&mut self, id: collection::Id) -> anyhow::Result<()> {
        self.collections
            .get_mut(&id)
            .unwrap()
            .update_from_paths(&mut self.uid_counter)
    }*/
}

const FILENAME: &str = "cowbump.db";
const BACKUP_FILENAME: &str = "cowbump.db.bak";
