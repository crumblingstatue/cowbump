use egui::{Button, Color32, CtxRef, Grid, Key, ScrollArea, TextEdit};

use crate::{
    collection::Collection,
    db::TagSet,
    gui::{
        egui_ui::{prompt, PromptAction},
        State,
    },
    tag,
};

use super::EguiState;

#[derive(Default)]
pub struct TagWindow {
    on: bool,
    filter_string: String,
    selected_uids: TagSet,
}

impl TagWindow {
    pub fn toggle(&mut self) {
        self.on ^= true;
    }
}

pub(super) fn do_frame(
    state: &mut State,
    egui_state: &mut EguiState,
    db: &mut Collection,
    egui_ctx: &CtxRef,
) {
    if egui_state.tag_window.on {
        let tags = &mut db.tags;
        let mut close = false;
        let close_ref = &mut close;
        let tag_filter_string_ref = &mut egui_state.tag_window.filter_string;
        let filter_string_ref = &mut egui_state.filter_string;
        let filter_spec_ref = &mut state.filter;
        let selected_uids = &mut egui_state.tag_window.selected_uids;
        // Clear selected uids that have already been deleted
        selected_uids.retain(|uid| tags.contains_key(uid));
        let prompts = &mut egui_state.prompts;
        egui::Window::new("Tag list")
            .open(&mut egui_state.tag_window.on)
            .show(egui_ctx, move |ui| {
                ui.horizontal(|ui| {
                    let te = TextEdit::singleline(tag_filter_string_ref).hint_text("Filter");
                    ui.add(te);
                    if ui.button("Clear filter").clicked() {
                        tag_filter_string_ref.clear();
                    }
                    if ui.button("Clear tags").clicked() {
                        filter_spec_ref.clear();
                    }
                });
                ui.separator();
                let scroll = ScrollArea::vertical().max_height(600.0);
                scroll.show(ui, |ui| {
                    Grid::new("tag_window_grid")
                        .spacing((16.0, 8.0))
                        .striped(true)
                        .show(ui, |ui| {
                            let mut uids: Vec<tag::Id> = tags.keys().cloned().collect();
                            uids.sort_by_key(|uid| &tags[uid].names[0]);
                            for tag_uid in &uids {
                                let tag = &tags[tag_uid];
                                let name = &tag.names[0];
                                if !name.contains(&tag_filter_string_ref[..]) {
                                    continue;
                                }
                                let has_this_tag = filter_spec_ref.has_tags.contains(tag_uid);
                                let doesnt_have_this_tag =
                                    filter_spec_ref.doesnt_have_tags.contains(tag_uid);
                                let mut checked = selected_uids.contains(tag_uid);
                                let mut button = Button::new(name).fill(if has_this_tag {
                                    Color32::from_rgb(43, 109, 57)
                                } else {
                                    Color32::from_rgb(45, 45, 45)
                                });
                                if checked {
                                    button = button
                                        .fill(Color32::from_rgb(246, 244, 41))
                                        .text_color(Color32::BLACK);
                                }
                                let mut clicked_any = false;
                                if ui.add(button).clicked() {
                                    filter_spec_ref.toggle_has(*tag_uid);
                                    filter_spec_ref.set_doesnt_have(*tag_uid, false);
                                    clicked_any = true;
                                }
                                let neg_button = Button::new("!").text_color(Color32::RED).fill(
                                    if doesnt_have_this_tag {
                                        Color32::from_rgb(109, 47, 43)
                                    } else {
                                        Color32::from_rgb(45, 45, 45)
                                    },
                                );
                                if ui.add(neg_button).clicked() {
                                    filter_spec_ref.toggle_doesnt_have(*tag_uid);
                                    filter_spec_ref.set_has(*tag_uid, false);
                                    clicked_any = true;
                                }
                                ui.checkbox(&mut checked, "");
                                if checked {
                                    selected_uids.insert(*tag_uid);
                                } else {
                                    selected_uids.remove(tag_uid);
                                }
                                ui.end_row();
                                if clicked_any {
                                    *filter_string_ref = filter_spec_ref.to_spec_string(tags);
                                }
                            }
                        });
                });
                if !selected_uids.is_empty() {
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            let n = selected_uids.len();
                            let fstring;
                            let msg = format!(
                                "Delete the selected {}tag{}?",
                                if n == 1 {
                                    ""
                                } else {
                                    fstring = format!("{} ", n);
                                    &fstring
                                },
                                if n == 1 { "" } else { "s" }
                            );
                            prompt(
                                prompts,
                                "Tag deletion",
                                msg,
                                PromptAction::DeleteTags(selected_uids.iter().cloned().collect()),
                            )
                        }
                        if ui.button("Clear selection").clicked() {
                            selected_uids.clear();
                        }
                    });
                }

                if egui_ctx.input().key_pressed(Key::Escape) {
                    *close_ref = true;
                }
            });
        if close {
            egui_state.just_closed_window_with_esc = true;
            egui_state.tag_window.on = false;
        }
    }
}
