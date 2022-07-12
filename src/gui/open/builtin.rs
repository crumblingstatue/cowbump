use egui_sfml::sfml::graphics::RenderWindow;

use crate::{
    entry,
    gui::{Activity, State},
};

/// Enter-press open with the built-in viewer
pub(in crate::gui) fn enter_open_builtin(state: &mut State, window: &RenderWindow) {
    if state.selected_uids.is_empty() {
        open_built_in_viewer(state, state.entries_view.uids.clone(), 0, window);
    } else {
        open_built_in_viewer(state, state.selected_uids.clone(), 0, window);
    }
}

/// Opens the built-in viewer with a list of images and a starting index in that list
pub(in crate::gui) fn open_built_in_viewer(
    state: &mut State,
    image_list: Vec<entry::Id>,
    starting_index: usize,
    window: &RenderWindow,
) {
    state.activity = Activity::Viewer;
    state.viewer_state.image_list = image_list;
    state.viewer_state.index = starting_index;
    state.viewer_state.reset_view(window);
}
