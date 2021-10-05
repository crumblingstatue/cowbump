use std::ops::Range;

use egui::{popup_below_widget, Align2, Color32, CtxRef, Key, TextEdit};

use crate::{collection::Collection, filter_spec::FilterSpec, gui::State};

use super::EguiState;

#[derive(Default)]
pub struct FilterPopup {
    pub on: bool,
    pub string: String,
    /// Autocomplete selection
    ac_select: usize,
}

fn str_range(parent: &str, sub: &str) -> Range<usize> {
    let beg = sub.as_ptr() as usize - parent.as_ptr() as usize;
    let end = beg + sub.len();
    beg..end
}

/// Returns whether filter state changed
pub(super) fn do_frame(
    state: &mut State,
    egui_state: &mut EguiState,
    egui_ctx: &CtxRef,
    coll: &mut Collection,
) -> bool {
    let popup = &mut egui_state.filter_popup;
    let mut filter_changed = false;
    if popup.on {
        egui::Window::new("Filter")
            .anchor(Align2::LEFT_TOP, [32.0, 32.0])
            .title_bar(false)
            .auto_sized()
            .show(egui_ctx, |ui| {
                let mut err = None;
                ui.horizontal(|ui| {
                    ui.label("filter");
                    let count = coll.filter(&state.filter).count();
                    let mut te = TextEdit::singleline(&mut popup.string).lock_focus(true);
                    if count == 0 {
                        te = te.text_color(Color32::RED);
                    }
                    let re = ui.add(te);
                    let popup_id = ui.make_persistent_id("tag_completion");
                    let last = popup.string.split_ascii_whitespace().last().unwrap_or("");
                    let input = egui_ctx.input();
                    if input.key_pressed(Key::ArrowDown) {
                        popup.ac_select += 1;
                    }
                    if input.key_pressed(Key::ArrowUp) && popup.ac_select > 0 {
                        popup.ac_select -= 1;
                    }
                    if !popup.string.is_empty() {
                        let filt = coll.tags.iter().filter(|(_id, tag)| {
                            let name = &tag.names[0];
                            name.contains(last) && name != last
                        });
                        let len = filt.clone().count();
                        if len > 0 {
                            if popup.ac_select >= len {
                                popup.ac_select = len - 1;
                            }
                            let mut complete = None;
                            popup_below_widget(ui, popup_id, &re, |ui| {
                                for (i, (id, tag)) in filt.enumerate() {
                                    if ui
                                        .selectable_label(popup.ac_select == i, &tag.names[0])
                                        .clicked()
                                    {
                                        complete = Some(id);
                                    }
                                    if popup.ac_select == i
                                        && (input.key_pressed(Key::Tab)
                                            || input.key_pressed(Key::Enter))
                                    {
                                        complete = Some(id);
                                    }
                                }
                            });
                            if let Some(id) = complete {
                                let range = str_range(&popup.string, last);
                                popup.string.replace_range(range, &coll.tags[id].names[0]);
                                state.wipe_search();
                                filter_changed = true;
                            }
                            if !popup.string.is_empty() {
                                ui.memory().open_popup(popup_id);
                            } else {
                                ui.memory().close_popup();
                            }
                        }
                    }
                    ui.label(&format!("{} results", count));
                    popup.string.make_ascii_lowercase();
                    match FilterSpec::parse_and_resolve(&popup.string, coll) {
                        Ok(spec) => state.filter = spec,
                        Err(e) => {
                            err = Some(format!("Error: {}", e));
                        }
                    };
                    if input.key_pressed(egui::Key::Enter) {
                        popup.on = false;
                    }
                    if re.changed() {
                        state.wipe_search();
                        filter_changed = true;
                    }
                    ui.memory().request_focus(re.id);
                });
                if let Some(e) = err {
                    ui.label(e);
                }
            });
    }
    filter_changed
}
