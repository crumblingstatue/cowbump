use std::path::PathBuf;

use fnv::FnvHashMap;
use serde_derive::{Deserialize, Serialize};

use crate::db::Uid;

#[derive(Serialize, Deserialize)]
pub struct Preferences {
    pub open_last_coll_at_start: bool,
    pub applications: FnvHashMap<AppId, App>,
    pub associations: FnvHashMap<String, Option<AppId>>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct App {
    pub name: String,
    pub path: PathBuf,
    /// A custom-parsed arguments string with `{}` placeholding for the entry list
    pub args_string: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct AppId(pub Uid);

impl Default for Preferences {
    fn default() -> Self {
        Self {
            open_last_coll_at_start: true,
            applications: Default::default(),
            associations: Default::default(),
        }
    }
}
