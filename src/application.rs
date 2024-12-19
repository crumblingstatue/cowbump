use {
    crate::{
        collection::{self, Collection},
        db::{Db, FolderChanges},
        entry, serialization,
    },
    anyhow::{Context, bail},
    std::path::{Path, PathBuf},
};

type ActiveCollection = Option<(collection::Id, Collection)>;

pub struct Application {
    pub database: Db,
    pub active_collection: ActiveCollection,
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
                .with_context(|| format!("Error loading collection {id:?}"))
        } else {
            Ok(FolderChanges::default())
        }
    }
    pub(crate) fn reload_active_collection(&mut self) -> anyhow::Result<FolderChanges> {
        if let Some((id, _)) = self.active_collection {
            self.load_collection(id)
        } else {
            bail!("No active collection")
        }
    }
    pub(crate) fn load_collection(&mut self, id: collection::Id) -> anyhow::Result<FolderChanges> {
        self.save_active_collection()?;
        let path = &self
            .database
            .collections
            .get(&id)
            .context("No collection with such id")?;
        let coll_dir = collections_dir_name(&self.database.data_dir);
        let filename = collection_filename(&coll_dir, id);
        let coll: Collection = serialization::read_from_file(&filename)
            .with_context(move || format!("Deserialization error for: {}", filename.display()))?;
        let changes = coll.scan_changes(path)?;
        self.active_collection = Some((id, coll));
        self.database.recent.use_(id);
        Ok(changes)
    }
    pub(crate) fn apply_changes_to_active_collection(
        &mut self,
        changes: &FolderChanges,
        callback: impl FnMut(&Path, entry::Id),
    ) {
        if let Some((_id, coll)) = self.active_collection.as_mut() {
            coll.apply_changes(changes, &mut self.database.uid_counter, callback);
        }
    }
    pub fn save_active_collection(&self) -> anyhow::Result<()> {
        match self.active_collection.as_ref() {
            Some((id, coll)) => self.save_collection(*id, coll),
            None => Ok(()),
        }
    }
    pub(crate) fn switch_collection(
        &mut self,
        coll: Option<(collection::Id, Collection)>,
    ) -> anyhow::Result<()> {
        if let Some((id, coll)) = &self.active_collection {
            self.save_collection(*id, coll)?;
        }
        self.active_collection = coll;
        Ok(())
    }
    fn save_collection(&self, id: collection::Id, collection: &Collection) -> anyhow::Result<()> {
        let dir_name = collections_dir_name(&self.database.data_dir);
        std::fs::create_dir_all(&dir_name)?;
        serialization::write_to_file(collection, collection_filename(&dir_name, id))
    }
}

fn collections_dir_name(data_dir: &Path) -> PathBuf {
    data_dir.join("collections")
}

fn collection_filename(collections_dir: &Path, id: collection::Id) -> PathBuf {
    collections_dir.join(format!("{}.db", id.0))
}
