use std::path::Path;

use anyhow::Context;

use crate::db::{global::GlobalDb, local::LocalDb};

pub struct Application {
    pub global_db: GlobalDb,
    pub local_db: Option<LocalDb>,
    pub no_save: bool,
}

impl Application {
    pub fn new() -> anyhow::Result<Self> {
        let global_db = GlobalDb::load().context("Failed to load global database")?;
        Ok(Self {
            global_db,
            local_db: None,
            no_save: false,
        })
    }
    pub fn load_folder(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        std::env::set_current_dir(path.as_ref())?;
        let mut db = if !path.as_ref().exists() {
            LocalDb::default()
        } else {
            LocalDb::load()?
        };
        db.update_from_folder(path.as_ref()).with_context(|| {
            format!(
                "Failed to update database from folder '{}'",
                path.as_ref().display()
            )
        })?;
        self.local_db = Some(db);
        Ok(())
    }
}
