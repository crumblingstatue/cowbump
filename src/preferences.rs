use std::path::PathBuf;

use fnv::FnvHashMap;
use serde_derive::{Deserialize, Serialize};

use crate::db::Uid;

#[derive(Serialize, Deserialize)]
pub struct Preferences {
    pub open_last_coll_at_start: bool,
    pub applications: FnvHashMap<AppId, App>,
    pub associations: FnvHashMap<String, Option<AppId>>,
    #[serde(default = "scroll_wheel_default")]
    pub scroll_wheel_multiplier: f32,
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

pub const fn scroll_wheel_default() -> f32 {
    64.0
}
pub const SCROLL_WHEEL_MIN: f32 = 2.0;
pub const SCROLL_WHEEL_MAX: f32 = 512.0;

impl Default for Preferences {
    fn default() -> Self {
        Self {
            open_last_coll_at_start: true,
            applications: Default::default(),
            associations: Default::default(),
            scroll_wheel_multiplier: scroll_wheel_default(),
        }
    }
}
