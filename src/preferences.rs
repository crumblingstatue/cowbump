use {
    crate::{
        collection::{SortBy, SortOrder},
        db::Uid,
    },
    egui_colors::{Colorix, tokens::ThemeColor},
    egui_sf2g::egui::emath::Numeric,
    fnv::FnvHashMap,
    serde_derive::{Deserialize, Serialize},
    std::{borrow::Cow, ops::RangeInclusive, path::PathBuf},
};

type AppMap = FnvHashMap<AppId, App>;

pub trait AppMapExt {
    fn name_of(&self, id: &AppId) -> Cow<str>;
}

impl AppMapExt for AppMap {
    fn name_of(&'_ self, id: &AppId) -> Cow<'_, str> {
        self.get(id)
            .map_or(format!("<dangling appid: {id:?}>").into(), |app| {
                Cow::Borrowed(&app.name)
            })
    }
}

#[derive(Serialize, Deserialize)]
pub struct Preferences {
    pub open_last_coll_at_start: bool,
    pub applications: AppMap,
    pub associations: FnvHashMap<String, Option<AppId>>,
    #[serde(default = "ScrollWheelMultiplier::default")]
    pub scroll_wheel_multiplier: f32,
    #[serde(default = "UpDownArrowScrollSpeed::default")]
    pub arrow_key_scroll_speed: f32,
    #[serde(default)]
    pub style: Style,
    #[serde(default = "built_in_viewer_default")]
    pub use_built_in_viewer: bool,
    #[serde(default)]
    pub start_fullscreen: bool,
    #[serde(default = "thumbs_per_row_default")]
    pub thumbs_per_row: u8,
    #[serde(default)]
    pub color_theme: Option<ColorTheme>,
    #[serde(default = "default_sort_pref")]
    pub sort_pref: SortPreference,
}

fn default_sort_pref() -> SortPreference {
    SortPreference {
        by: SortBy::Path,
        order: SortOrder::Asc,
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct SortPreference {
    pub by: SortBy,
    pub order: SortOrder,
}

#[derive(Serialize, Deserialize)]
pub struct ColorTheme {
    colors: [MyThemeColor; 12],
    #[serde(default)]
    pub light_dark_preference: Option<LightDarkPref>,
}

#[derive(Serialize, Deserialize)]
pub enum LightDarkPref {
    Light,
    Dark,
}

impl ColorTheme {
    fn from_colorix(theme: &[ThemeColor; 12], light_dark: Option<LightDarkPref>) -> Self {
        Self {
            colors: theme.map(|preset| preset.rgb()),
            light_dark_preference: light_dark,
        }
    }
    pub(crate) fn to_colorix(&self) -> [ThemeColor; 12] {
        self.colors.map(ThemeColor::Custom)
    }
}

pub type MyThemeColor = [u8; 3];

const fn built_in_viewer_default() -> bool {
    true
}

const fn thumbs_per_row_default() -> u8 {
    5
}

impl Preferences {
    pub fn resolve_app(&self, name: &str) -> Option<AppId> {
        self.applications
            .iter()
            .find(|(_k, v)| v.name == name)
            .map(|(k, _v)| *k)
    }
    pub fn set_color_theme_from_colorix(
        &mut self,
        colorix: &Colorix,
        light_dark: Option<LightDarkPref>,
    ) {
        self.color_theme = Some(ColorTheme::from_colorix(colorix.theme(), light_dark));
    }
}

#[derive(Serialize, Deserialize)]
pub struct Style {
    pub heading_size: f32,
    pub button_size: f32,
    pub body_size: f32,
    pub monospace_size: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            heading_size: 20.0,
            body_size: 16.0,
            button_size: 16.0,
            monospace_size: 14.0,
        }
    }
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

pub trait ValuePref {
    type Type: Numeric;
    const DEFAULT: Self::Type;
    const RANGE: RangeInclusive<Self::Type>;
    const NAME: &'static str;
    fn default() -> Self::Type {
        Self::DEFAULT
    }
}

pub enum ScrollWheelMultiplier {}
impl ValuePref for ScrollWheelMultiplier {
    type Type = f32;
    const DEFAULT: f32 = 64.0;
    const RANGE: RangeInclusive<f32> = 2.0..=512.0;
    const NAME: &'static str = "Mouse wheel scrolling multiplier";
}

pub enum UpDownArrowScrollSpeed {}
impl ValuePref for UpDownArrowScrollSpeed {
    type Type = f32;
    const DEFAULT: f32 = 8.0;
    const RANGE: RangeInclusive<f32> = 1.0..=64.0;
    const NAME: &'static str = "Up/Down arrow key scroll speed";
}

pub enum ThumbnailsPerRow {}
impl ValuePref for ThumbnailsPerRow {
    type Type = u8;
    const DEFAULT: u8 = thumbs_per_row_default();

    const RANGE: RangeInclusive<Self::Type> = 1..=20;

    const NAME: &'static str = "Thumbnails per row";
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            open_last_coll_at_start: true,
            applications: Default::default(),
            associations: Default::default(),
            scroll_wheel_multiplier: ScrollWheelMultiplier::DEFAULT,
            arrow_key_scroll_speed: UpDownArrowScrollSpeed::DEFAULT,
            style: Default::default(),
            use_built_in_viewer: true,
            start_fullscreen: false,
            thumbs_per_row: thumbs_per_row_default(),
            color_theme: None,
            sort_pref: default_sort_pref(),
        }
    }
}
