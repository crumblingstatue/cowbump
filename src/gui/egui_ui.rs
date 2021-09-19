use crate::{
    db::{Db, Uid},
    gui::{common_tags, search_goto_cursor, State},
    FilterSpec,
};
use egui::{
    Align2, Button, Color32, Grid, Key, Label, Rgba, ScrollArea, TextEdit, TextureId,
    TopBottomPanel,
};
use retain_mut::RetainMut;
use std::{mem, path::Path, process::Command};

#[derive(Default)]
pub(crate) struct EguiState {
    delete_confirm_windows: Vec<DeleteConfirmWindow>,
    custom_command_windows: Vec<CustomCommandWindow>,
    image_prop_windows: Vec<ImagePropWindow>,
    tag_add_question_windows: Vec<TagAddQuestionWindow>,
    tag_window_filter_string: String,
    pub(crate) action: Option<Action>,
    pub top_bar: bool,
}

pub(crate) enum Action {
    Quit,
    SearchNext,
    SearchPrev,
    SelectAll,
    SelectNone,
    SortImages,
}

impl EguiState {
    pub fn begin_frame(&mut self) {
        self.action = None;
    }
}

struct TagAddQuestionWindow {
    tag_text: String,
    image_uids: Vec<u32>,
}

/// Image properties window
struct ImagePropWindow {
    image_uids: Vec<Uid>,
    add_tag_buffer: String,
    rename_buffer: String,
    adding_tag: bool,
    renaming: bool,
}

impl ImagePropWindow {
    fn new(image_uids: Vec<Uid>) -> Self {
        Self {
            image_uids,
            add_tag_buffer: String::default(),
            rename_buffer: String::default(),
            adding_tag: false,
            renaming: false,
        }
    }
}

struct DeleteConfirmWindow {
    uids: Vec<Uid>,
}

pub(super) fn do_ui(state: &mut State, egui_ctx: &egui::CtxRef, db: &mut Db) {
    if state.egui_state.top_bar {
        TopBottomPanel::top("top_panel").show(egui_ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu(ui, "File", |ui| {
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        state.egui_state.action = Some(Action::Quit);
                    }
                });
                egui::menu::menu(ui, "Actions", |ui| {
                    ui.separator();
                    if ui.button("Filter (F)").clicked() {
                        state.filter_edit ^= true;
                    }
                    if ui.button("Search (/)").clicked() {
                        state.search_edit ^= true;
                    }
                    if ui.button("Next result (N)").clicked() {
                        state.egui_state.action = Some(Action::SearchNext);
                    }
                    if ui.button("Previous result (P)").clicked() {
                        state.egui_state.action = Some(Action::SearchPrev);
                    }
                    if ui.button("Select All (ctrl+A)").clicked() {
                        state.egui_state.action = Some(Action::SelectAll);
                    }
                    if ui.button("Select None (Esc)").clicked() {
                        state.egui_state.action = Some(Action::SelectNone);
                    }
                    if ui.button("Sort images by filename (S)").clicked() {
                        state.egui_state.action = Some(Action::SortImages);
                    }
                });
                egui::menu::menu(ui, "Windows", |ui| {
                    ui.separator();
                    if ui.button("Tag list (T)").clicked() {
                        state.tag_window ^= true;
                    }
                });
                ui.separator();
                ui.label("(F1 to toggle)");
            });
        });
    }
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
                    match FilterSpec::parse_and_resolve(&state.search_string, db) {
                        Ok(spec) => state.search_spec = spec,
                        Err(e) => {
                            ui.label(&format!("Error: {}", e));
                        }
                    }
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
                let mut err = None;
                ui.horizontal(|ui| {
                    ui.label("filter");
                    let count = db.filter(&state.filter).count();
                    let mut te = TextEdit::singleline(&mut state.filter_string);
                    if count == 0 {
                        te = te.text_color(Color32::RED);
                    }
                    let re = ui.add(te);
                    ui.label(&format!("{} results", count));
                    state.filter_string.make_ascii_lowercase();
                    match FilterSpec::parse_and_resolve(&state.filter_string, db) {
                        Ok(spec) => state.filter = spec,
                        Err(e) => {
                            err = Some(format!("Error: {}", e));
                        }
                    };
                    if re.ctx.input().key_pressed(egui::Key::Enter) || re.lost_focus() {
                        state.filter_edit = false;
                    }
                    if re.changed() {
                        state.wipe_search();
                        state.y_offset = 0.0;
                    }
                    ui.memory().request_focus(re.id);
                });
                if let Some(e) = err {
                    ui.label(e);
                }
            });
    }
    if state.tag_window {
        let tags = &mut db.tags;
        let mut close = false;
        let close_ref = &mut close;
        let tag_filter_string_ref = &mut state.egui_state.tag_window_filter_string;
        let filter_string_ref = &mut state.filter_string;
        let filter_spec_ref = &mut state.filter;
        egui::Window::new("Tag list")
            .open(&mut state.tag_window)
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
                let scroll = ScrollArea::auto_sized();
                scroll.show(ui, |ui| {
                    Grid::new("tag_window_grid")
                        .spacing((16.0, 8.0))
                        .striped(true)
                        .show(ui, |ui| {
                            let mut uids: Vec<Uid> = tags.keys().cloned().collect();
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
                                let button = Button::new(name).fill(if has_this_tag {
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
                                ui.end_row();
                                if clicked_any {
                                    *filter_string_ref = filter_spec_ref.to_spec_string(tags);
                                }
                            }
                        });
                });

                if egui_ctx.input().key_pressed(Key::Escape) {
                    *close_ref = true;
                }
            });
        if close {
            state.just_closed_window_with_esc = true;
            state.tag_window = false;
        }
    }
    image_windows_ui(state, db, egui_ctx);
    delete_confirm_windows_ui(state, db, egui_ctx);
    custom_command_windows_ui(state, db, egui_ctx);
    tag_add_question_windows_ui(state, db, egui_ctx);
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
    state.egui_state.image_prop_windows.retain_mut(|propwin| {
        let mut open = true;
        let n_images = propwin.image_uids.len();
        let title = {
            if propwin.image_uids.len() == 1 {
                get_filename_from_path(&db.entries[&propwin.image_uids[0]].path)
            } else {
                format!("{} images", n_images)
            }
        };
        let esc_pressed = egui_ctx.input().key_pressed(Key::Escape);
        let mut close = esc_pressed;
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
                        let plus_re = ui.button("Add tags");
                        if plus_re.clicked() {
                            propwin.adding_tag ^= true;
                        }
                        if propwin.adding_tag {
                            let re = ui.text_edit_singleline(&mut propwin.add_tag_buffer);
                            re.request_focus();
                            if esc_pressed {
                                propwin.adding_tag = false;
                                propwin.add_tag_buffer.clear();
                                close = false;
                            }
                            if re.ctx.input().key_pressed(Key::Enter) {
                                let add_tag_buffer: &str = &propwin.add_tag_buffer;
                                let image_uids: &[Uid] = &propwin.image_uids;
                                let tags = add_tag_buffer.split_whitespace();
                                for tag in tags {
                                    match db.resolve_tag(tag) {
                                        Some(tag_uid) => {
                                            db.add_tag_for_multi(image_uids, tag_uid);
                                        }
                                        None => state.egui_state.tag_add_question_windows.push(
                                            TagAddQuestionWindow {
                                                tag_text: tag.to_owned(),
                                                image_uids: image_uids.to_owned(),
                                            },
                                        ),
                                    }
                                }
                                propwin.add_tag_buffer.clear();
                                propwin.adding_tag = false;
                            }
                        }

                        if ui
                            .add(
                                Button::new("Rename")
                                    .wrap(false)
                                    .enabled(propwin.image_uids.len() == 1),
                            )
                            .clicked()
                        {
                            propwin.renaming ^= true;
                        }
                        if propwin.renaming {
                            let re = ui.text_edit_singleline(&mut propwin.rename_buffer);
                            if re.ctx.input().key_pressed(egui::Key::Enter) {
                                db.rename(propwin.image_uids[0], &propwin.rename_buffer);
                                propwin.renaming = false;
                            }
                            if re.lost_focus() {
                                propwin.renaming = false;
                                close = false;
                            }
                            ui.memory().request_focus(re.id);
                        }
                        if ui
                            .add(Button::new("Delete from disk").wrap(false))
                            .clicked()
                        {
                            state
                                .egui_state
                                .delete_confirm_windows
                                .push(DeleteConfirmWindow {
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
                            state.egui_state.custom_command_windows.push(win);
                        }
                    });
                });
            });
        if close {
            state.just_closed_window_with_esc = true;
            open = false;
        }
        open
    });
}

fn tag_add_question_windows_ui(state: &mut State, db: &mut Db, egui_ctx: &egui::CtxRef) {
    state.egui_state.tag_add_question_windows.retain_mut(|win| {
        let mut open = true;
        egui::Window::new("Tag add").show(egui_ctx, |ui| {
            ui.label(&format!(
                "The tag '{}' doesn't exist. Create it?",
                win.tag_text
            ));
            if ui.button("Yes").clicked() {
                let tag_uid = db.add_new_tag_from_text(mem::take(&mut win.tag_text));
                db.add_tag_for_multi(&win.image_uids, tag_uid);
                open = false;
            }
            if ui.button("No").clicked() {
                open = false;
            }
        });
        open
    })
}

struct CustomCommandWindow {
    uids: Vec<u32>,
    cmd_buffer: String,
    args_buffer: String,
    err_str: String,
    just_opened: bool,
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
