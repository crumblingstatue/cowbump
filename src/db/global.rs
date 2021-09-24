use anyhow::Context;
use directories::ProjectDirs;
use serde_derive::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct GlobalDb {}

impl GlobalDb {
    pub fn load() -> anyhow::Result<Self> {
        let dirs = ProjectDirs::from("", "crumblingstatue", "cowbump")
            .context("Failed to retrieve project dirs")?;
        let data_dir = dirs.data_dir();
        if !data_dir.exists() {
            std::fs::create_dir_all(data_dir)?;
        }
        let path = data_dir.join(FILENAME);
        if path.exists() {
            crate::db::serialization::read_from_file(path)
        } else {
            Ok(Self::default())
        }
    }
}

const FILENAME: &str = "cowbump_global.db";
