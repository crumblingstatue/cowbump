use crate::db::{Db, Uid};
use crate::gui::{common_tags, search_goto_cursor, AddTag, State};
use egui::{Color32, Label, TextureId};
use retain_mut::RetainMut;
use std::path::Path;

#[derive(Default)]
pub(super) struct EguiState {
    image_rename_windows: Vec<ImageRenameWindow>,
    delete_confirm_windows: Vec<DeleteConfirmWindow>,
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
        egui::Window::new("Search").show(egui_ctx, |ui| {
            let re = ui.text_edit_singleline(&mut state.search_string);
            ui.memory().request_kb_focus(re.id);
            if re.lost_kb_focus() {
                state.search_edit = false;
            }
            if re.changed() {
                state.search_cursor = 0;
                search_goto_cursor(state, db);
            }
        });
    }
    if state.filter_edit {
        egui::Window::new("Filter").show(egui_ctx, |ui| {
            let ed = ui.text_edit_singleline(&mut state.filter.substring_match);
            ui.memory().request_kb_focus(ed.id);
            if ed.lost_kb_focus() {
                state.filter_edit = false;
            }
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
                    let mut i = 0;
                    tags.retain(|_uid, tag| {
                        ui.label(tag.names[0].clone());
                        let keep = if ui.button("x").clicked() {
                            for (_uid, en) in entries.iter_mut() {
                                en.tags.retain(|&uid| uid != i);
                            }
                            false
                        } else {
                            true
                        };
                        i += 1;
                        keep
                    });
                    if ui.button("Add tag").clicked() {
                        *add_tag = Some(AddTag::default());
                    }
                });
            });
    }
    if let Some(add_tag) = &mut state.add_tag {
        let mut rem = false;
        egui::Window::new("Add new tag").show(egui_ctx, |ui| {
            let re = ui.text_edit_singleline(&mut add_tag.name);
            ui.memory().request_kb_focus(re.id);
            if re.ctx.input().key_down(egui::Key::Enter) {
                rem = true;
                db.add_new_tag(crate::tag::Tag {
                    names: vec![add_tag.name.clone()],
                    implies: vec![],
                });
            }
        });
        if rem {
            state.add_tag = None;
        }
    }
    image_windows_ui(state, db, egui_ctx);
    image_rename_windows_ui(state, db, egui_ctx);
    delete_confirm_windows_ui(state, db, egui_ctx);
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
    let image_prop_windows = &mut state.image_prop_windows;
    let egui_state = &mut state.egui_state;
    image_prop_windows.retain_mut(|propwin| {
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
                        for &id in &propwin.image_uids {
                            ui.image(
                                TextureId::User(id as u64),
                                (512.0 / n_images as f32, 512.0 / n_images as f32),
                            );
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        for tagid in common_tags(&propwin.image_uids, db) {
                            ui.group(|ui| {
                                ui.add(
                                    Label::new(db.tags[&tagid].names[0].clone())
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
                            });
                        }
                        let plus_re = ui.button("+");
                        let popup_id = ui.make_persistent_id("popid");
                        if plus_re.clicked() {
                            ui.memory().toggle_popup(popup_id);
                        }
                        egui::popup::popup_below_widget(ui, popup_id, &plus_re, |ui| {
                            ui.set_min_width(100.0);
                            let mut tag_add = None;
                            for (i, (_uid, tag)) in db.tags.iter().enumerate() {
                                let name = tag.names[0].clone();
                                if ui.button(name).clicked() {
                                    tag_add = Some((propwin.image_uids[0], i as u32));
                                }
                            }
                            if let Some((image_id, tag_id)) = tag_add {
                                db.add_tag_for(image_id, tag_id);
                            }
                        });
                        if propwin.image_uids.len() == 1 && ui.button("Rename").clicked() {
                            let uid = propwin.image_uids[0];
                            let win = ImageRenameWindow {
                                uid,
                                name_buffer: get_filename_from_path(&db.entries[&uid].path),
                            };
                            egui_state.image_rename_windows.push(win);
                        }
                        if ui.button("Delete from disk").clicked() {
                            egui_state.delete_confirm_windows.push(DeleteConfirmWindow {
                                uids: propwin.image_uids.clone(),
                            });
                        }
                    })
                });
            });
        open
    });
}

fn image_rename_windows_ui(state: &mut State, db: &mut Db, egui_ctx: &egui::CtxRef) {
    state.egui_state.image_rename_windows.retain_mut(|win| {
        let mut open = true;
        egui::Window::new("Rename").show(egui_ctx, |ui| {
            let re = ui.text_edit_singleline(&mut win.name_buffer);
            ui.memory().request_kb_focus(re.id);
            if re.ctx.input().key_pressed(egui::Key::Enter) {
                db.rename(win.uid, &win.name_buffer);
                open = false;
            }
            if re.lost_kb_focus() {
                open = false;
            }
        });
        open
    });
}

fn delete_confirm_windows_ui(state: &mut State, db: &mut Db, egui_ctx: &egui::CtxRef) {
    let mut retain = true;
    state.egui_state.delete_confirm_windows.retain(|win| {
        egui::Window::new("Delete confirm request").show(egui_ctx, |ui| {
            if ui.button("Yes").clicked() {
                remove_images(&win.uids, db);
                retain = false;
            }
            if ui.button("No").clicked() {
                retain = false;
            }
        });
        retain
    });
}

fn remove_images(image_uids: &[Uid], db: &mut Db) {
    for &uid in image_uids {
        let path = &db.entries[&uid].path;
        if let Err(e) = std::fs::remove_file(path) {
            eprintln!("Remove error: {}", e);
        }
        db.entries.remove(&uid);
    }
}
