use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::{
    collection::{self, Collection},
    db::{Db, FolderChanges},
    serialization,
};

pub struct Application {
    pub database: Db,
    pub active_collection: Option<(collection::Id, Collection)>,
    pub no_save: bool,
}

impl Application {
    pub fn new() -> anyhow::Result<Self> {
        let global_db = Db::load().context("Failed to load global database")?;
        Ok(Self {
            database: global_db,
            active_collection: None,
            no_save: false,
        })
    }
    pub fn add_collection(&mut self, collection: Collection, root: PathBuf) -> collection::Id {
        let id = self.database.insert_collection(root);
        self.active_collection = Some((id, collection));
        self.database.recent.use_(id);
        id
    }
    pub(crate) fn load_last(&mut self) -> anyhow::Result<FolderChanges> {
        if let Some(&id) = self.database.recent.most_recent() {
            self.load_collection(id)
        } else {
            Ok(FolderChanges::default())
        }
    }
    pub(crate) fn load_collection(&mut self, id: collection::Id) -> anyhow::Result<FolderChanges> {
        let path = &self.database.collections[&id];
        let coll_dir = collections_dir_name(&self.database.data_dir);
        let coll: Collection = serialization::read_from_file(collection_filename(&coll_dir, id))?;
        let changes = coll.scan_changes(path)?;
        self.active_collection = Some((id, coll));
        self.database.recent.use_(id);
        Ok(changes)
    }
    pub(crate) fn active_collection(&mut self) -> Option<(collection::Id, &mut Collection)> {
        self.active_collection.as_mut().map(|c| (c.0, &mut c.1))
    }

    pub(crate) fn apply_changes_to_active_collection(&mut self, changes: &FolderChanges) {
        if let Some((_id, coll)) = self.active_collection.as_mut() {
            coll.apply_changes(changes, &mut self.database.uid_counter)
        }
    }
}

pub(crate) fn switch_collection(
    data_dir: &Path,
    active_collection: &mut Option<(collection::Id, Collection)>,
    coll: Option<(collection::Id, Collection)>,
) -> anyhow::Result<()> {
    if let Some((id, coll)) = active_collection {
        save_collection(data_dir, *id, coll)?;
    }
    *active_collection = coll;
    Ok(())
}

fn save_collection(
    data_dir: &Path,
    id: collection::Id,
    collection: &Collection,
) -> anyhow::Result<()> {
    let dir_name = collections_dir_name(data_dir);
    std::fs::create_dir_all(&dir_name)?;
    serialization::write_to_file(collection, collection_filename(&dir_name, id))
}

fn collections_dir_name(data_dir: &Path) -> PathBuf {
    data_dir.join("collections")
}

fn collection_filename(collections_dir: &Path, id: collection::Id) -> PathBuf {
    collections_dir.join(format!("{}.db", id.0))
}
