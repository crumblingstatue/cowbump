use crate::{
    db::{Db, Uid},
    gui::{common_tags, search_goto_cursor, AddTag, State},
};
use egui::{Align2, Button, Color32, Label, Rgba, TextEdit, TextureId};
use retain_mut::RetainMut;
use std::{path::Path, process::Command};

#[derive(Default)]
pub(super) struct EguiState {
    image_rename_windows: Vec<ImageRenameWindow>,
    delete_confirm_windows: Vec<DeleteConfirmWindow>,
    custom_command_windows: Vec<CustomCommandWindow>,
    image_prop_windows: Vec<ImagePropWindow>,
}

/// Image properties window
struct ImagePropWindow {
    image_uids: Vec<Uid>,
}

impl ImagePropWindow {
    fn new(image_uids: Vec<Uid>) -> Self {
        Self { image_uids }
    }
}

struct ImageRenameWindow {
    uid: Uid,
    name_buffer: String,
}

struct DeleteConfirmWindow {
    uids: Vec<Uid>,
}

pub(super) fn do_ui(state: &mut State, egui_ctx: &egui::CtxRef, db: &mut Db) {
    if state.search_edit {
        egui::Window::new("Search")
            .anchor(Align2::LEFT_TOP, [32.0, 32.0])
            .title_bar(false)
            .auto_sized()
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("search");
                    let mut te = TextEdit::singleline(&mut state.search_string);
                    if !state.search_success {
                        te = te.text_color(Color32::RED);
                    }
                    let re = ui.add(te);
                    if re.ctx.input().key_pressed(egui::Key::Enter) || re.lost_focus() {
                        state.search_edit = false;
                    }
                    if re.changed() || re.ctx.input().key_pressed(egui::Key::Enter) {
                        state.search_cursor = 0;
                        search_goto_cursor(state, db);
                    }
                    ui.memory().request_focus(re.id);
                });
            });
    }
    if state.filter_edit {
        egui::Window::new("Filter")
            .anchor(Align2::LEFT_TOP, [32.0, 32.0])
            .title_bar(false)
            .auto_sized()
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("filter");
                    let no_entries = db.filter(&state.filter).next().is_none();
                    let mut te = TextEdit::singleline(&mut state.filter.substring_match);
                    if no_entries {
                        te = te.text_color(Color32::RED);
                    }
                    let re = ui.add(te);
                    state.filter.substring_match.make_ascii_lowercase();
                    if re.ctx.input().key_pressed(egui::Key::Enter) || re.lost_focus() {
                        state.filter_edit = false;
                    }
                    if re.changed() {
                        state.wipe_search();
                        state.y_offset = 0.0;
                    }
                    ui.memory().request_focus(re.id);
                });
            });
    }
    if state.tag_window {
        let add_tag = &mut state.add_tag;
        let tags = &mut db.tags;
        let entries = &mut db.entries;
        egui::Window::new("Tag editor")
            .open(&mut state.tag_window)
            .show(egui_ctx, move |ui| {
                ui.vertical(|ui| {
                    tags.retain(|tag_uid, tag| {
                        let mut keep = false;
                        ui.horizontal(|ui| {
                            ui.label(tag.names[0].clone());
                            keep = if ui.button("x").clicked() {
                                for (_uid, en) in entries.iter_mut() {
                                    en.tags.retain(|&uid| uid != *tag_uid);
                                }
                                false
                            } else {
                                true
                            };
                        });
                        keep
                    });
                    if ui.button("Add tag").clicked() {
                        *add_tag = Some(AddTag::default());
                    }
                });
            });
    }
    let mut close = false;
    if let Some(add_tag) = &mut state.add_tag {
        let mut rem = false;
        egui::Window::new("Add new tag").show(egui_ctx, |ui| {
            let re = ui.text_edit_singleline(&mut add_tag.name);
            if re.ctx.input().key_down(egui::Key::Enter) {
                rem = true;
                db.add_new_tag(crate::tag::Tag {
                    names: vec![add_tag.name.clone()],
                    implies: vec![],
                });
            }
            if re.lost_focus() {
                close = true;
            }
            ui.memory().request_focus(re.id);
        });
        if rem {
            state.add_tag = None;
        }
    }
    if close {
        state.add_tag = None;
    }
    image_windows_ui(state, db, egui_ctx);
    image_rename_windows_ui(state, db, egui_ctx);
    delete_confirm_windows_ui(state, db, egui_ctx);
    custom_command_windows_ui(state, db, egui_ctx);
}

fn get_filename_from_path(path: &Path) -> String {
    path.components()
        .last()
        .unwrap()
        .as_os_str()
        .to_string_lossy()
        .to_string()
}

fn image_windows_ui(state: &mut State, db: &mut Db, egui_ctx: &egui::CtxRef) {
    let egui_state = &mut state.egui_state;
    egui_state.image_prop_windows.retain_mut(|propwin| {
        let mut open = true;
        let n_images = propwin.image_uids.len();
        let title = {
            if propwin.image_uids.len() == 1 {
                db.entries[&propwin.image_uids[0]]
                    .path
                    .display()
                    .to_string()
            } else {
                format!("{} images", n_images)
            }
        };
        egui::Window::new(title)
            .open(&mut open)
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.set_max_width(512.0);
                        for &id in &propwin.image_uids {
                            ui.image(
                                TextureId::User(id as u64),
                                (512.0 / n_images as f32, 512.0 / n_images as f32),
                            );
                        }
                    });
                    ui.vertical(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            for tagid in common_tags(&propwin.image_uids, db) {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        let tag_name = match db.tags.get(&tagid) {
                                            Some(tag) => &tag.names[0],
                                            None => "<unknown tag>",
                                        };
                                        ui.add(
                                            Label::new(tag_name)
                                                .wrap(false)
                                                .background_color(Color32::from_rgb(50, 40, 45)),
                                        );
                                        if ui.button("x").clicked() {
                                            // TODO: This only works for 1 item windows
                                            db.entries
                                                .get_mut(&propwin.image_uids[0])
                                                .unwrap()
                                                .tags
                                                .retain(|&t| t != tagid);
                                        }
                                    })
                                });
                            }
                        });
                        let plus_re = ui.button("Add tag");
                        let popup_id = ui.make_persistent_id("popid");
                        if plus_re.clicked() {
                            ui.memory().toggle_popup(popup_id);
                        }
                        egui::popup::popup_below_widget(ui, popup_id, &plus_re, |ui| {
                            ui.set_min_width(100.0);
                            let mut tag_add = None;
                            for (&uid, tag) in db.tags.iter() {
                                if !db.has_tag(propwin.image_uids[0], uid) {
                                    let name = tag.names[0].clone();
                                    if ui.button(name).clicked() {
                                        tag_add = Some((propwin.image_uids[0], uid));
                                    }
                                }
                            }
                            if let Some((image_id, tag_id)) = tag_add {
                                db.add_tag_for(image_id, tag_id);
                            }
                        });
                        if propwin.image_uids.len() == 1
                            && ui.add(Button::new("Rename").wrap(false)).clicked()
                        {
                            let uid = propwin.image_uids[0];
                            let win = ImageRenameWindow {
                                uid,
                                name_buffer: get_filename_from_path(&db.entries[&uid].path),
                            };
                            egui_state.image_rename_windows.push(win);
                        }
                        if ui
                            .add(Button::new("Delete from disk").wrap(false))
                            .clicked()
                        {
                            egui_state.delete_confirm_windows.push(DeleteConfirmWindow {
                                uids: propwin.image_uids.clone(),
                            });
                        }
                        if ui
                            .add(Button::new("Run custom command").wrap(false))
                            .clicked()
                        {
                            let win = CustomCommandWindow {
                                uids: propwin.image_uids.clone(),
                                cmd_buffer: String::new(),
                                args_buffer: String::new(),
                                err_str: String::new(),
                                just_opened: true,
                            };
                            egui_state.custom_command_windows.push(win);
                        }
                    });
                });
            });
        open
    });
}

struct CustomCommandWindow {
    uids: Vec<u32>,
    cmd_buffer: String,
    args_buffer: String,
    err_str: String,
    just_opened: bool,
}

fn image_rename_windows_ui(state: &mut State, db: &mut Db, egui_ctx: &egui::CtxRef) {
    state.egui_state.image_rename_windows.retain_mut(|win| {
        let mut open = true;
        egui::Window::new("Rename").show(egui_ctx, |ui| {
            let re = ui.text_edit_singleline(&mut win.name_buffer);
            if re.ctx.input().key_pressed(egui::Key::Enter) {
                db.rename(win.uid, &win.name_buffer);
                open = false;
            }
            if re.lost_focus() {
                open = false;
            }
            ui.memory().request_focus(re.id);
        });
        open
    });
}

fn custom_command_windows_ui(state: &mut State, db: &mut Db, egui_ctx: &egui::CtxRef) {
    state.egui_state.custom_command_windows.retain_mut(|win| {
        let mut open = true;
        egui::Window::new("Custom Command").show(egui_ctx, |ui| {
            ui.label("Command");
            let re = ui.text_edit_singleline(&mut win.cmd_buffer);
            if win.just_opened {
                re.request_focus();
            }
            ui.label("Args (use {} for image path, or leave empty)");
            ui.text_edit_singleline(&mut win.args_buffer);
            if re.ctx.input().key_pressed(egui::Key::Enter) {
                let mut cmd = Command::new(&win.cmd_buffer);
                for uid in &win.uids {
                    let en = &db.entries[uid];
                    for arg in win.args_buffer.split_whitespace() {
                        if arg == "{}" {
                            cmd.arg(&en.path);
                        } else {
                            cmd.arg(arg);
                        }
                    }
                    if win.args_buffer.is_empty() {
                        cmd.arg(&en.path);
                    }
                }
                match cmd.spawn() {
                    Ok(_) => open = false,
                    Err(e) => win.err_str = e.to_string(),
                }
            }
            if !win.err_str.is_empty() {
                ui.add(Label::new(format!("Error: {}", win.err_str)).text_color(Rgba::RED));
            }
        });
        win.just_opened = false;
        open
    });
}

fn delete_confirm_windows_ui(state: &mut State, db: &mut Db, egui_ctx: &egui::CtxRef) {
    let mut retain = true;
    let entries_view = &mut state.entries_view;
    let egui_state = &mut state.egui_state;
    let mut i = 0;
    let mut removes = Vec::new();
    egui_state.delete_confirm_windows.retain(|win| {
        egui::Window::new("Delete confirm request").show(egui_ctx, |ui| {
            if ui.button("Yes").clicked() {
                remove_images(entries_view, &win.uids, db);
                removes.push(i);
                retain = false;
            }
            if ui.button("No").clicked() {
                retain = false;
            }
        });
        i += 1;
        retain
    });
    for i in removes {
        state.egui_state.image_prop_windows.remove(i);
    }
}

fn remove_images(view: &mut super::EntriesView, image_uids: &[Uid], db: &mut Db) {
    for &uid in image_uids {
        let path = &db.entries[&uid].path;
        if let Err(e) = std::fs::remove_file(path) {
            eprintln!("Remove error: {}", e);
        }
        view.delete(uid);
        db.entries.remove(&uid);
    }
}
impl EguiState {
    pub(crate) fn add_image_prop_window(&mut self, vec: Vec<u32>) {
        self.image_prop_windows.push(ImagePropWindow::new(vec));
    }
}
