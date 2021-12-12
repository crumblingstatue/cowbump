use std::mem;

use egui::{Button, Color32, CtxRef, Grid, Key, ScrollArea, TextEdit};

use crate::{
    collection::Collection,
    db::{TagSet, UidCounter},
    gui::{
        debug_log::dlog,
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
    prop_active: Option<tag::Id>,
    new_name: String,
    new_name_add: bool,
    new_imply: String,
    new_imply_add: bool,
    new_tag_buf: String,
    new_tag_add: bool,
}

impl TagWindow {
    pub fn toggle(&mut self) {
        self.on ^= true;
    }
}

pub(super) fn do_frame(
    state: &mut State,
    egui_state: &mut EguiState,
    coll: &mut Collection,
    egui_ctx: &CtxRef,
    uid_counter: &mut UidCounter,
) {
    if !egui_state.tag_window.on {
        return;
    }
    let mut close = false;
    let close_ref = &mut close;
    let tag_filter_string_ref = &mut egui_state.tag_window.filter_string;
    let filter_string_ref = &mut egui_state.filter_popup.string;
    let filter_spec_ref = &mut state.filter;
    let selected_uids = &mut egui_state.tag_window.selected_uids;
    let active_ref = &mut egui_state.tag_window.prop_active;
    let new_name_ref = &mut egui_state.tag_window.new_name;
    let new_name_add_ref = &mut egui_state.tag_window.new_name_add;
    let new_imply_ref = &mut egui_state.tag_window.new_imply;
    let new_imply_add_ref = &mut egui_state.tag_window.new_imply_add;
    let new_tag_buf_ref = &mut egui_state.tag_window.new_tag_buf;
    let new_tag_add_ref = &mut egui_state.tag_window.new_tag_add;
    // Clear selected uids that have already been deleted
    selected_uids.retain(|uid| coll.tags.contains_key(uid));
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
                if ui.button("Add new tag").clicked() {
                    *new_tag_add_ref ^= true;
                }
            });
            if *new_tag_add_ref
                && ui
                    .add(TextEdit::singleline(new_tag_buf_ref).hint_text("New tag"))
                    .lost_focus()
                && ui.input().key_pressed(Key::Enter)
            {
                coll.add_new_tag_from_text(mem::take(new_tag_buf_ref), uid_counter);
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_min_height(600.);
                    ui.set_width(400.0);
                    let scroll = ScrollArea::vertical()
                        .max_height(600.0)
                        .auto_shrink([false, false]);
                    scroll.show(ui, |ui| {
                        Grid::new("tag_window_grid")
                            .spacing((8.0, 8.0))
                            .striped(true)
                            .num_columns(4)
                            .show(ui, |ui| {
                                let tags = &mut coll.tags;
                                let mut uids: Vec<tag::Id> = tags.keys().cloned().collect();
                                uids.sort_by_key(|uid| &tags[uid].names[0]);
                                for tag_uid in &uids {
                                    let tag = &tags[tag_uid];
                                    let name = &tag.names[0];
                                    if !name.contains(&tag_filter_string_ref[..]) {
                                        continue;
                                    }
                                    let mut button = Button::new(name);
                                    let mut checked = selected_uids.contains(tag_uid);
                                    if active_ref == &Some(*tag_uid) {
                                        button = button.fill(Color32::from_rgb(95, 69, 8));
                                    } else if checked {
                                        button = button.fill(Color32::from_rgb(189, 145, 85));
                                    }
                                    if ui.add(button).clicked() {
                                        *active_ref = Some(*tag_uid);
                                    }
                                    let has_this_tag = filter_spec_ref.has_tags.contains(tag_uid);
                                    let doesnt_have_this_tag =
                                        filter_spec_ref.doesnt_have_tags.contains(tag_uid);
                                    let button = Button::new("✔").fill(if has_this_tag {
                                        Color32::from_rgb(43, 109, 57)
                                    } else {
                                        Color32::from_rgb(45, 45, 45)
                                    });
                                    let mut clicked_any = false;
                                    if ui.add(button).clicked() {
                                        filter_spec_ref.toggle_has(*tag_uid);
                                        filter_spec_ref.set_doesnt_have(*tag_uid, false);
                                        clicked_any = true;
                                    }
                                    let neg_button = Button::new("！")
                                        .text_color(Color32::RED)
                                        .fill(if doesnt_have_this_tag {
                                            Color32::from_rgb(109, 47, 43)
                                        } else {
                                            Color32::from_rgb(45, 45, 45)
                                        });
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
                                    PromptAction::DeleteTags(
                                        selected_uids.iter().cloned().collect(),
                                    ),
                                )
                            }
                            if ui.button("Clear selection").clicked() {
                                selected_uids.clear();
                            }
                        });
                    }
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_min_width(400.);
                    match active_ref {
                        None => {
                            ui.heading("Click a tag to edit properties");
                        }
                        Some(id) => {
                            if !coll.tags.contains_key(id) {
                                // Prevent crashing if we just deleted this tag
                                return;
                            }
                            ui.heading(format!("Tag {} (#{})", coll.tags[id].names[0], id.0));
                            ui.separator();
                            ui.label("Names");
                            ui.add_space(4.0);
                            let tag = coll.tags.get_mut(id).unwrap();
                            tag.names.retain_mut(|name| {
                                ui.label(name);
                                true
                            });
                            ui.horizontal(|ui| {
                                if ui.button("+").clicked() {
                                    *new_name_add_ref = true;
                                }
                                if *new_name_add_ref
                                    && ui
                                        .add(
                                            TextEdit::singleline(new_name_ref)
                                                .hint_text("New alias"),
                                        )
                                        .lost_focus()
                                    && ui.input().key_pressed(Key::Enter)
                                {
                                    tag.names.push(mem::take(new_name_ref));
                                }
                            });
                            ui.add_space(12.0);
                            ui.label("Implies");
                            ui.add_space(4.0);
                            let mut remove = None;
                            for imply_id in &coll.tags[id].implies {
                                ui.horizontal(|ui| {
                                    ui.label(&coll.tags[imply_id].names[0]);
                                    if ui.button("🗑").clicked() {
                                        remove = Some(*imply_id);
                                    }
                                });
                            }
                            if let Some(imply_id) = remove {
                                coll.tags.get_mut(id).unwrap().implies.remove(&imply_id);
                            }
                            ui.horizontal(|ui| {
                                if ui.button("+").clicked() {
                                    *new_imply_add_ref = true;
                                }
                                if *new_imply_add_ref
                                    && ui
                                        .add(
                                            TextEdit::singleline(new_imply_ref)
                                                .hint_text("New implication"),
                                        )
                                        .lost_focus()
                                    && ui.input().key_pressed(Key::Enter)
                                {
                                    if let Some(resolved_id) = coll.resolve_tag(new_imply_ref) {
                                        let tag = coll.tags.get_mut(id).unwrap();
                                        tag.implies.insert(resolved_id);
                                        dlog!("Success?");
                                        dlog!("{:?}", tag);
                                    }
                                }
                            });
                        }
                    }
                });
            });

            if egui_ctx.input().key_pressed(Key::Escape) {
                *close_ref = true;
            }
        });
    if close {
        egui_state.just_closed_window_with_esc = true;
        egui_state.tag_window.on = false;
    }
}
