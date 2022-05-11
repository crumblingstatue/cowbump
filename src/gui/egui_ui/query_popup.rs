use egui_sfml::egui::{Align2, Window};

use super::tag_autocomplete::AcState;

#[derive(Default)]
pub struct QueryPopup {
    pub on: bool,
    pub string: String,
    pub err_string: String,
    pub ac_state: AcState,
}

/// Create an egui window in the corner for queries (filter/search)
pub fn query_window(title: &str) -> Window {
    Window::new(title)
        .anchor(Align2::LEFT_TOP, [32.0, 32.0])
        .title_bar(false)
        .auto_sized()
}
