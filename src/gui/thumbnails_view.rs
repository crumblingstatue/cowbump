use anyhow::Context as _;
use egui_sfml::{
    egui::Context,
    sfml::{
        graphics::{RenderTarget, RenderWindow},
        window::{mouse, Event, Key},
    },
};

use crate::{collection::Collection, entry, preferences::Preferences};

use super::{
    egui_ui::EguiState,
    native_dialog,
    open::{builtin, external},
    State,
};

fn go_to_bottom(window: &RenderWindow, state: &mut State) {
    state.entries_view.y_offset = find_bottom(state, window);
}

pub(in crate::gui) fn clamp_bottom(window: &RenderWindow, state: &mut State) {
    let bottom = find_bottom(state, window);
    if state.entries_view.y_offset > bottom {
        state.entries_view.y_offset = bottom;
    }
}

fn find_bottom(state: &State, window: &RenderWindow) -> f32 {
    let n_pics = state.entries_view.iter().count();
    let mut rows = n_pics as u32 / state.thumbnails_per_row as u32;
    if n_pics as u32 % state.thumbnails_per_row as u32 != 0 {
        rows += 1;
    }
    let bottom = rows * state.thumbnail_size;
    let mut b = bottom as f32 - window.size().y as f32;
    if b < 0. {
        b = 0.;
    }
    b
}

fn entry_at_xy(x: i32, y: i32, state: &State) -> Option<entry::Id> {
    let thumb_index = abs_thumb_index_at_xy(x, y, state);
    state.entries_view.get(thumb_index)
}

/// Returns the absolute thumb index at (x,y) on the screen
///
/// This is absolute, so the top left image on the screen could have a different index
/// based on the scroll y offset
fn abs_thumb_index_at_xy(x: i32, y: i32, state: &State) -> usize {
    let thumb_x = x as u32 / state.thumbnail_size;
    let thumb_y = (y as u32 + state.entries_view.y_offset as u32) / state.thumbnail_size;
    let thumb_index = thumb_y * state.thumbnails_per_row as u32 + thumb_x;
    thumb_index as usize
}

pub(in crate::gui) fn handle_event(
    event: Event,
    state: &mut State,
    egui_state: &mut EguiState,
    coll: &mut Collection,
    window: &RenderWindow,
    egui_ctx: &Context,
    preferences: &mut Preferences,
) {
    match event {
        Event::MouseButtonPressed { button, x, y } => {
            if egui_ctx.wants_pointer_input() {
                return;
            }
            let uid = match entry_at_xy(x, y, state) {
                Some(uid) => uid,
                None => return,
            };
            if button == mouse::Button::Left {
                if Key::LShift.is_pressed() {
                    if state.selected_uids.contains(&uid) {
                        state.selected_uids.retain(|&rhs| rhs != uid);
                    } else {
                        state.selected_uids.push(uid);
                    }
                } else if Key::LControl.is_pressed() {
                    let thumb_idx = abs_thumb_index_at_xy(x, y, state);
                    match state.select_begin {
                        Some(begin) => {
                            for id in state
                                .entries_view
                                .iter()
                                .skip(begin)
                                .take((thumb_idx + 1) - begin)
                            {
                                state.selected_uids.push(id);
                            }
                            state.select_begin = None;
                        }
                        None => state.select_begin = Some(thumb_idx),
                    }
                } else if preferences.use_built_in_viewer {
                    builtin::open(
                        state,
                        state.entries_view.uids.clone(),
                        abs_thumb_index_at_xy(x, y, state),
                        window,
                    );
                } else {
                    external::handle_open(coll, uid, preferences);
                }
            } else if button == mouse::Button::Right {
                let vec = if state.selected_uids.contains(&uid) {
                    state.selected_uids.clone()
                } else {
                    vec![uid]
                };
                egui_state.add_entries_window(vec);
            }
        }
        Event::KeyPressed { code, ctrl, .. } => {
            if egui_ctx.wants_keyboard_input() {
                return;
            }
            if code == Key::PageDown {
                state.entries_view.y_offset += window.size().y as f32;
                clamp_bottom(window, state);
            } else if code == Key::PageUp {
                state.entries_view.y_offset -= window.size().y as f32;
                clamp_top(state);
            } else if code == Key::Enter {
                if preferences.use_built_in_viewer {
                    builtin::on_enter_open(state, window);
                } else {
                    external::on_enter_open(state, coll, preferences);
                }
            } else if code == Key::A && ctrl {
                select_all(state, coll);
            } else if code == Key::Slash {
                egui_state.find_popup.on = true;
            } else if code == Key::N {
                search_next(state, coll, window.size().y);
            } else if code == Key::P {
                search_prev(state, coll, window.size().y);
            } else if code == Key::F {
                egui_state.filter_popup.on = true;
            } else if code == Key::C {
                let mp = window.mouse_position();
                let uid = match entry_at_xy(mp.x, mp.y, state) {
                    Some(uid) => uid,
                    None => return,
                };
                if let Err(e) = copy_image_to_clipboard(state, coll, uid) {
                    native_dialog::error("Clipboard copy failed", e);
                }
            } else if code == Key::T {
                egui_state.tag_window.toggle();
            } else if code == Key::Q {
                egui_state.sequences_window.on ^= true;
            } else if code == Key::S {
                state
                    .entries_view
                    .update_from_collection(coll, &state.filter);
            } else if code == Key::Home {
                if !egui_ctx.wants_keyboard_input() {
                    state.entries_view.y_offset = 0.0;
                }
            } else if code == Key::End && !egui_ctx.wants_keyboard_input() {
                // Align the bottom edge of the view with the bottom edge of the last row.
                // To do align the camera with a bottom edge, we need to subtract the screen
                // height from it.
                go_to_bottom(window, state);
            } else if code == Key::F2 && !state.selected_uids.is_empty() {
                egui_state.add_entries_window(state.selected_uids.clone())
            } else if code == Key::Escape
                && !egui_ctx.wants_keyboard_input()
                && !egui_ctx.wants_pointer_input()
                && !egui_state.just_closed_window_with_esc
            {
                state.selected_uids.clear()
            }
        }
        Event::MouseWheelScrolled { delta, .. } => {
            state.entries_view.y_offset -= delta * preferences.scroll_wheel_multiplier;
            if delta > 0.0 {
                clamp_top(state);
            } else {
                clamp_bottom(window, state);
            }
        }
        _ => {}
    }
}

fn clamp_top(state: &mut State) {
    if state.entries_view.y_offset < 0.0 {
        state.entries_view.y_offset = 0.0;
    }
}

fn copy_image_to_clipboard(
    state: &mut State,
    coll: &mut Collection,
    uid: entry::Id,
) -> anyhow::Result<()> {
    use arboard::ImageData;
    let imgpath = &coll.entries[&uid].path;
    let buf = std::fs::read(imgpath).unwrap();
    let img = image::load_from_memory(&buf).context("Failed to load image from memory")?;
    let rgba = img.to_rgba8();
    let img_data = ImageData {
        width: rgba.width() as usize,
        height: rgba.height() as usize,
        bytes: rgba.into_raw().into(),
    };
    state
        .clipboard_ctx
        .set_image(img_data)
        .context("Failed to copy to clipboard")
}

pub(in crate::gui) fn select_all(state: &mut State, coll: &Collection) {
    state.selected_uids.clear();
    for uid in coll.filter(&state.filter) {
        state.selected_uids.push(uid);
    }
}

pub(in crate::gui) fn search_prev(state: &mut State, coll: &mut Collection, view_height: u32) {
    if state.search_cursor > 0 {
        state.search_cursor -= 1;
    }
    search_goto_cursor(state, coll, view_height);
}

pub(in crate::gui) fn search_next(state: &mut State, coll: &mut Collection, view_height: u32) {
    state.search_cursor += 1;
    search_goto_cursor(state, coll, view_height);
    if !state.search_success {
        state.search_cursor -= 1;
    }
}

fn find_nth(state: &State, coll: &Collection, nth: usize) -> Option<usize> {
    state
        .entries_view
        .iter()
        .enumerate()
        .filter(|(_, uid)| {
            let en = &coll.entries[uid];
            en.all_reqs_satisfied(*uid, &state.find_reqs, &coll.tags, &coll.sequences)
        })
        .map(|(i, _)| i)
        .nth(nth)
}

pub(in crate::gui) fn search_goto_cursor(state: &mut State, coll: &Collection, view_height: u32) {
    if let Some(index) = find_nth(state, coll, state.search_cursor) {
        state.highlight = Some(index as u32);
        state.search_success = true;
        state.seek_view_to_contain_index(index, view_height);
    } else {
        state.search_success = false;
    }
}
