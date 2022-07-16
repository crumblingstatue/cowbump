use egui_sfml::sfml::graphics::RenderWindow;

use crate::{
    entry,
    gui::{Activity, State},
};

/// Open functionality when enter is pressed in thumbnails view
pub(in crate::gui) fn on_enter_open(state: &mut State, window: &RenderWindow) {
    if state.selected_uids.is_empty() {
        open_list(state, state.thumbs_view.uids.clone(), 0, window);
    } else {
        open_list(state, state.selected_uids.clone(), 0, window);
    }
}

/// Opens the built-in viewer with a list of images and a starting index in that list
pub(in crate::gui) fn open_list(
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
