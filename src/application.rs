use anyhow::Context;

use crate::{
    collection::{self, Collection},
    db::Db,
};

pub struct Application {
    pub database: Db,
    pub active_collection: Option<collection::Id>,
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
    pub fn add_collection(&mut self, collection: Collection) -> collection::Id {
        let id = self.database.insert_collection(collection);
        self.active_collection = Some(id);
        self.database.recent.use_(id);
        id
    }
    pub(crate) fn load_last(&mut self) -> anyhow::Result<()> {
        if let Some(&id) = self.database.recent.most_recent() {
            self.load_collection(id)?;
        }
        Ok(())
    }
    pub(crate) fn load_collection(&mut self, id: collection::Id) -> anyhow::Result<()> {
        //self.database.update_collection(id)?;
        self.active_collection = Some(id);
        self.database.recent.use_(id);
        Ok(())
    }
}
