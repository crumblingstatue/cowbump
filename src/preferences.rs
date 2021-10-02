use std::{ops::RangeInclusive, path::PathBuf};

use fnv::FnvHashMap;
use serde_derive::{Deserialize, Serialize};

use crate::db::Uid;

#[derive(Serialize, Deserialize)]
pub struct Preferences {
    pub open_last_coll_at_start: bool,
    pub applications: FnvHashMap<AppId, App>,
    pub associations: FnvHashMap<String, Option<AppId>>,
    #[serde(default = "ScrollWheelMultiplier::default")]
    pub scroll_wheel_multiplier: f32,
    #[serde(default = "UpDownArrowScrollSpeed::default")]
    pub arrow_key_scroll_speed: f32,
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

pub trait FloatPref {
    const DEFAULT: f32;
    const RANGE: RangeInclusive<f32>;
    const NAME: &'static str;
    fn default() -> f32 {
        Self::DEFAULT
    }
}

pub enum ScrollWheelMultiplier {}
impl FloatPref for ScrollWheelMultiplier {
    const DEFAULT: f32 = 64.0;
    const RANGE: RangeInclusive<f32> = 2.0..=512.0;
    const NAME: &'static str = "Mouse wheel scrolling multiplier";
}

pub enum UpDownArrowScrollSpeed {}
impl FloatPref for UpDownArrowScrollSpeed {
    const DEFAULT: f32 = 8.0;
    const RANGE: RangeInclusive<f32> = 1.0..=64.0;
    const NAME: &'static str = "Up/Down arrow key scroll speed";
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            open_last_coll_at_start: true,
            applications: Default::default(),
            associations: Default::default(),
            scroll_wheel_multiplier: ScrollWheelMultiplier::DEFAULT,
            arrow_key_scroll_speed: UpDownArrowScrollSpeed::DEFAULT,
        }
    }
}
