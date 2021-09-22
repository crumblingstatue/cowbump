use crate::{
    db::{local::Db, Uid},
    gui::{common_tags, open_with_external, search_goto_cursor, State},
    FilterSpec,
};
use egui::{
    Align2, Button, Color32, CtxRef, Grid, ImageButton, Key, Label, Rgba, ScrollArea, TextEdit,
    TextureId, TopBottomPanel, Window,
};
use retain_mut::RetainMut;

use std::{
    collections::HashSet,
    io::Read,
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
};

#[derive(Default)]
pub(crate) struct EguiState {
    image_prop_windows: Vec<ImagePropWindow>,
    pub sequences_window: SequencesWindow,
    sequence_windows: Vec<SequenceWindow>,
    tag_window: TagWindow,
    pub(crate) action: Option<Action>,
    pub top_bar: bool,
    info_messages: Vec<InfoMessage>,
    prompts: Vec<Prompt>,
}

struct SequenceWindow {
    uid: Uid,
}

impl SequenceWindow {
    fn new(uid: Uid) -> Self {
        Self { uid }
    }
}

#[derive(Default)]
pub struct SequencesWindow {
    pub on: bool,
    add_new: bool,
    add_new_buffer: String,
}

#[derive(Default)]
struct TagWindow {
    on: bool,
    filter_string: String,
    selected_uids: HashSet<Uid>,
}

struct Prompt {
    msg: InfoMessage,
    action: PromptAction,
}

enum PromptAction {
    RestoreBackup,
    QuitNoSave,
    DeleteTags(Vec<Uid>),
}

fn ok_prompt(ctx: &CtxRef, title: &str, msg: &str) -> bool {
    let mut clicked = false;
    Window::new(title)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(msg);
                if ui.button("Ok").clicked() {
                    clicked = true;
                }
            })
        });
    clicked
}

enum OkCancel {
    Ok,
    Cancel,
}

fn ok_cancel_prompt(ctx: &CtxRef, title: &str, msg: &str) -> Option<OkCancel> {
    let mut clicked = None;
    Window::new(title)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(msg);
                ui.horizontal(|ui| {
                    if ui.button("Ok").clicked() {
                        clicked = Some(OkCancel::Ok);
                    }
                    if ui.button("Cancel").clicked() {
                        clicked = Some(OkCancel::Cancel);
                    }
                })
            })
        });
    clicked
}

pub(crate) enum Action {
    Quit,
    QuitNoSave,
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
    pub fn toggle_tag_window(&mut self) {
        self.tag_window.on ^= true;
    }
}

fn info_message(
    info_messages: &mut Vec<InfoMessage>,
    title: impl Into<String>,
    message: impl Into<String>,
) {
    info_messages.push(InfoMessage {
        title: title.into(),
        message: message.into(),
    })
}

struct InfoMessage {
    title: String,
    message: String,
}

/// Image properties window
#[derive(Default)]
struct ImagePropWindow {
    image_uids: Vec<Uid>,
    add_tag_buffer: String,
    rename_buffer: String,
    adding_tag: bool,
    renaming: bool,
    delete_confirm: bool,
    custom_command_prompt: bool,
    add_to_seq_prompt: bool,
    cmd_buffer: String,
    args_buffer: String,
    err_str: String,
    new_tags: Vec<String>,
    children: Vec<ChildWrapper>,
}

struct ChildWrapper {
    child: Child,
    exit_status: Option<ExitStatus>,
    stdout: String,
    stderr: String,
    name: String,
}

impl ChildWrapper {
    fn new(child: Child, name: String) -> Self {
        Self {
            child,
            exit_status: None,
            stdout: String::new(),
            stderr: String::new(),
            name,
        }
    }
}

impl ImagePropWindow {
    fn new(image_uids: Vec<Uid>) -> Self {
        Self {
            image_uids,
            ..Default::default()
        }
    }
}

pub(super) fn do_ui(state: &mut State, egui_ctx: &egui::CtxRef, db: &mut Db) {
    do_top_bar(state, egui_ctx, db);
    do_search_edit(state, egui_ctx, db);
    do_filter_edit(state, egui_ctx, db);
    do_tag_window(state, db, egui_ctx);
    do_sequences_window(state, db, egui_ctx);
    do_sequence_windows(state, db, egui_ctx);
    do_image_windows(state, db, egui_ctx);
    do_info_messages(state, egui_ctx);
    do_prompts(state, egui_ctx, db);
}

fn do_sequence_windows(state: &mut State, db: &mut Db, egui_ctx: &CtxRef) {
    state.egui_state.sequence_windows.retain_mut(|win| {
        let mut open = true;
        let seq = db.sequences.get_mut(&win.uid).unwrap();
        let name = &seq.name;
        enum Action {
            SwapLeft,
            SwapRight,
            Remove,
            Open,
        }
        let mut action = Action::SwapLeft;
        let mut subject = None;
        Window::new(&format!("Sequence: {}", name))
            .hscroll(true)
            .min_width(3. * 256.)
            .open(&mut open)
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    let seq_images_len = seq.images.len();
                    for (i, &img_uid) in seq.images.iter().enumerate() {
                        ui.vertical(|ui| {
                            let img_butt = ImageButton::new(TextureId::User(img_uid), (256., 256.));
                            if ui.add(img_butt).clicked() {
                                action = Action::Open;
                                subject = Some(img_uid);
                            }
                            ui.horizontal(|ui| {
                                if i > 0 && ui.button("<").clicked() {
                                    action = Action::SwapLeft;
                                    subject = Some(img_uid);
                                }
                                if ui.button("-").clicked() {
                                    action = Action::Remove;
                                    subject = Some(img_uid);
                                }
                                if i < seq_images_len - 1 && ui.button(">").clicked() {
                                    action = Action::SwapRight;
                                    subject = Some(img_uid);
                                }
                            });
                        });
                    }
                });
            });
        if let Some(uid) = subject {
            match action {
                Action::SwapLeft => {
                    seq.swap_image_left(uid);
                }
                Action::SwapRight => {
                    seq.swap_image_right(uid);
                }
                Action::Remove => {
                    seq.remove_image(uid);
                }
                Action::Open => {
                    let mut paths = Vec::new();
                    for img_uid in seq.iage_uids_wrapped_from(uid) {
                        paths.push(db.entries[&img_uid].path.as_ref());
                    }
                    open_with_external(&paths);
                }
            }
        }
        open
    });
}

fn do_sequences_window(state: &mut State, db: &mut Db, egui_ctx: &CtxRef) {
    let seq_win = &mut state.egui_state.sequences_window;
    let enter_pressed = egui_ctx.input().key_pressed(Key::Enter);
    if seq_win.on {
        Window::new("Sequences").show(egui_ctx, |ui| {
            if ui.button("+").clicked() {
                seq_win.add_new ^= true;
            }
            if seq_win.add_new {
                ui.text_edit_singleline(&mut seq_win.add_new_buffer);
                if enter_pressed {
                    db.add_new_sequence(&seq_win.add_new_buffer);
                }
            }
            ui.separator();
            db.sequences.retain(|&uid, seq| {
                if ui.button(&seq.name).clicked() {
                    state
                        .egui_state
                        .sequence_windows
                        .push(SequenceWindow::new(uid));
                }
                true
            });
        });
    }
}

fn do_info_messages(state: &mut State, egui_ctx: &CtxRef) {
    state
        .egui_state
        .info_messages
        .retain_mut(|msg| !ok_prompt(egui_ctx, &msg.title, &msg.message));
}

fn do_prompts(state: &mut State, egui_ctx: &CtxRef, db: &mut Db) {
    state.egui_state.prompts.retain(|prompt| {
        match ok_cancel_prompt(egui_ctx, &prompt.msg.title, &prompt.msg.message) {
            Some(OkCancel::Ok) => match prompt.action {
                PromptAction::RestoreBackup => {
                    match db.load_backup() {
                        Ok(_) => {
                            info_message(
                                &mut state.egui_state.info_messages,
                                "Success",
                                "Successfully restored backup.",
                            );
                        }
                        Err(e) => {
                            info_message(
                                &mut state.egui_state.info_messages,
                                "Error",
                                &e.to_string(),
                            );
                        }
                    }
                    false
                }
                PromptAction::QuitNoSave => {
                    state.egui_state.action = Some(Action::QuitNoSave);
                    false
                }
                PromptAction::DeleteTags(ref uids) => {
                    db.remove_tags(uids);
                    false
                }
            },
            Some(OkCancel::Cancel) => false,
            None => true,
        }
    });
}

fn do_tag_window(state: &mut State, db: &mut Db, egui_ctx: &CtxRef) {
    if state.egui_state.tag_window.on {
        let tags = &mut db.tags;
        let mut close = false;
        let close_ref = &mut close;
        let tag_filter_string_ref = &mut state.egui_state.tag_window.filter_string;
        let filter_string_ref = &mut state.filter_string;
        let filter_spec_ref = &mut state.filter;
        let selected_uids = &mut state.egui_state.tag_window.selected_uids;
        // Clear selected uids that have already been deleted
        selected_uids.retain(|uid| tags.contains_key(uid));
        let prompts = &mut state.egui_state.prompts;
        egui::Window::new("Tag list")
            .open(&mut state.egui_state.tag_window.on)
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
            state.just_closed_window_with_esc = true;
            state.egui_state.tag_window.on = false;
        }
    }
}

fn do_filter_edit(state: &mut State, egui_ctx: &CtxRef, db: &mut Db) {
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
}

fn do_search_edit(state: &mut State, egui_ctx: &CtxRef, db: &mut Db) {
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
}

fn do_top_bar(state: &mut State, egui_ctx: &CtxRef, db: &mut Db) {
    if state.egui_state.top_bar {
        TopBottomPanel::top("top_panel").show(egui_ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu(ui, "File", |ui| {
                    ui.separator();
                    if ui.button("Create database backup").clicked() {
                        match db.save_backup() {
                            Ok(_) => {
                                info_message(
                                    &mut state.egui_state.info_messages,
                                    "Success",
                                    "Backup successfully created.",
                                );
                            }
                            Err(e) => {
                                info_message(
                                    &mut state.egui_state.info_messages,
                                    "Error",
                                    &e.to_string(),
                                );
                            }
                        }
                    }
                    if ui.button("Restore database backup").clicked() {
                        prompt(
                            &mut state.egui_state.prompts,
                            "Restore Backup",
                            "Warning: This will overwrite the current contents of the database.",
                            PromptAction::RestoreBackup,
                        )
                    }
                    ui.separator();
                    if ui.button("Quit without saving").clicked() {
                        prompt(
                            &mut state.egui_state.prompts,
                            "Quit without saving",
                            "Warning: All changes made this session will be lost.",
                            PromptAction::QuitNoSave,
                        )
                    }
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
                    ui.separator();
                    if ui.button("Search (/)").clicked() {
                        state.search_edit ^= true;
                    }
                    if ui.button("Next result (N)").clicked() {
                        state.egui_state.action = Some(Action::SearchNext);
                    }
                    if ui.button("Previous result (P)").clicked() {
                        state.egui_state.action = Some(Action::SearchPrev);
                    }
                    ui.separator();
                    if ui.button("Select All (ctrl+A)").clicked() {
                        state.egui_state.action = Some(Action::SelectAll);
                    }
                    if ui.button("Select None (Esc)").clicked() {
                        state.egui_state.action = Some(Action::SelectNone);
                    }
                    ui.separator();
                    if ui.button("Sort images by filename (S)").clicked() {
                        state.egui_state.action = Some(Action::SortImages);
                    }
                });
                egui::menu::menu(ui, "Windows", |ui| {
                    ui.separator();
                    if ui.button("Tag list (T)").clicked() {
                        state.egui_state.tag_window.on ^= true;
                    }
                    if ui.button("Sequences (Q)").clicked() {
                        state.egui_state.sequences_window.on ^= true;
                    }
                });
                ui.separator();
                ui.label("(F1 to toggle)");
            });
        });
    }
}

fn prompt(
    prompts: &mut Vec<Prompt>,
    title: impl Into<String>,
    message: impl Into<String>,
    action: PromptAction,
) {
    prompts.push(Prompt {
        msg: InfoMessage {
            message: message.into(),
            title: title.into(),
        },
        action,
    })
}

fn get_filename_from_path(path: &Path) -> String {
    path.components()
        .last()
        .unwrap()
        .as_os_str()
        .to_string_lossy()
        .to_string()
}

fn do_image_windows(state: &mut State, db: &mut Db, egui_ctx: &egui::CtxRef) {
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
                        let n_visible_images = n_images.min(64);
                        for &id in propwin.image_uids.iter().take(n_visible_images) {
                            ui.image(
                                TextureId::User(id as u64),
                                (
                                    512.0 / n_visible_images as f32,
                                    512.0 / n_visible_images as f32,
                                ),
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
                                        None => {
                                            propwin.new_tags.push(tag.to_owned());
                                        }
                                    }
                                }
                                propwin.add_tag_buffer.clear();
                                propwin.adding_tag = false;
                            }
                        }

                        if !propwin.new_tags.is_empty() {
                            ui.label(
                                "You added the following tags to the image,\
                                 but they aren't present in the database: ",
                            );
                        }
                        propwin.new_tags.retain_mut(|tag| {
                            let mut retain = true;
                            ui.horizontal(|ui| {
                                ui.label(&tag[..]);
                                if ui.button("Add").clicked() {
                                    let tag_uid = db.add_new_tag_from_text(tag.to_owned());
                                    db.add_tag_for_multi(&propwin.image_uids, tag_uid);
                                    retain = false;
                                }
                                if ui.button("Cancel").clicked() {
                                    retain = false;
                                }
                            });
                            retain
                        });

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
                        if !propwin.delete_confirm {
                            if ui
                                .add(Button::new("Delete from disk").wrap(false))
                                .clicked()
                            {
                                propwin.delete_confirm ^= true;
                            }
                        } else {
                            let del_uids = &mut propwin.image_uids;
                            let del_len = del_uids.len();
                            let label_string = if del_len == 1 {
                                format!(
                                    "About to delete {}",
                                    db.entries[&del_uids[0]].path.display()
                                )
                            } else {
                                format!("About to delete {} images", del_len)
                            };
                            ui.label(&label_string);
                            ui.horizontal(|ui| {
                                if ui.add(Button::new("Confirm").fill(Color32::RED)).clicked() {
                                    remove_images(&mut state.entries_view, del_uids, db);
                                    propwin.delete_confirm = false;
                                    close = true;
                                }
                                if esc_pressed || ui.add(Button::new("Cancel")).clicked() {
                                    propwin.delete_confirm = false;
                                    close = false;
                                }
                            });
                        }
                        if ui.button("Add to sequence").clicked() {
                            propwin.add_to_seq_prompt ^= true;
                        }
                        if propwin.add_to_seq_prompt {
                            let mut add = None;
                            for (&uid, seq) in &db.sequences {
                                if ui.button(&seq.name).clicked() {
                                    add = Some(uid);
                                    break;
                                }
                            }
                            if let Some(uid) = add {
                                db.add_images_to_sequence(uid, &propwin.image_uids);
                            }
                        }
                        if ui
                            .add(Button::new("Run custom command").wrap(false))
                            .clicked()
                        {
                            propwin.custom_command_prompt ^= true;
                        }
                        if propwin.custom_command_prompt {
                            if esc_pressed {
                                propwin.custom_command_prompt = false;
                                close = false;
                            }
                            ui.label("Command");
                            let re = ui.text_edit_singleline(&mut propwin.cmd_buffer);
                            ui.label("Args (use {} for image path, or leave empty)");
                            ui.text_edit_singleline(&mut propwin.args_buffer);
                            if re.ctx.input().key_pressed(egui::Key::Enter) {
                                let mut cmd = Command::new(&propwin.cmd_buffer);
                                cmd.stderr(Stdio::piped());
                                cmd.stdin(Stdio::piped());
                                cmd.stdout(Stdio::piped());
                                for uid in &propwin.image_uids {
                                    let en = &db.entries[uid];
                                    for arg in propwin.args_buffer.split_whitespace() {
                                        if arg == "{}" {
                                            cmd.arg(&en.path);
                                        } else {
                                            cmd.arg(arg);
                                        }
                                    }
                                    if propwin.args_buffer.is_empty() {
                                        cmd.arg(&en.path);
                                    }
                                }
                                match cmd.spawn() {
                                    Ok(child) => {
                                        propwin.err_str.clear();
                                        propwin.custom_command_prompt = false;
                                        propwin.children.push(ChildWrapper::new(
                                            child,
                                            propwin.cmd_buffer.clone(),
                                        ));
                                    }
                                    Err(e) => propwin.err_str = e.to_string(),
                                }
                            }
                            if !propwin.err_str.is_empty() {
                                ui.add(
                                    Label::new(format!("Error: {}", propwin.err_str))
                                        .text_color(Rgba::RED),
                                );
                            }
                        }
                        propwin.children.retain_mut(|c_wrap| {
                            ui.separator();
                            ui.heading(&c_wrap.name);
                            let mut retain = true;
                            if let Some(status) = c_wrap.exit_status {
                                ui.label("stdout:");
                                ui.code(&c_wrap.stdout);
                                ui.label("stderr:");
                                ui.code(&c_wrap.stderr);
                                let exit_code_msg = match status.code() {
                                    Some(code) => code.to_string(),
                                    None => "<terminated>".to_string(),
                                };
                                ui.label(&format!(
                                    "Exit code: {} ({})",
                                    exit_code_msg,
                                    status.success()
                                ));
                                return !ui.button("x").clicked();
                            }
                            let mut clicked = false;
                            ui.horizontal(|ui| {
                                clicked = ui.button("x").clicked();
                                ui.label(&format!("[running] ({})", c_wrap.child.id()));
                            });
                            if clicked {
                                let _ = c_wrap.child.kill();
                                return false;
                            }
                            match c_wrap.child.try_wait() {
                                Ok(opt_status) => {
                                    c_wrap.exit_status = opt_status;
                                    if let Some(status) = opt_status {
                                        if !status.success() {
                                            if let Some(stdout) = &mut c_wrap.child.stdout {
                                                let mut buf = String::new();
                                                let _ = stdout.read_to_string(&mut buf);
                                                c_wrap.stdout = buf;
                                            }
                                            if let Some(stderr) = &mut c_wrap.child.stderr {
                                                let mut buf = String::new();
                                                let _ = stderr.read_to_string(&mut buf);
                                                c_wrap.stderr = buf;
                                            }
                                        } else {
                                            retain = false;
                                        }
                                    }
                                }
                                Err(e) => {
                                    propwin.err_str = e.to_string();
                                }
                            }
                            retain
                        })
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

fn remove_images(view: &mut super::EntriesView, image_uids: &mut Vec<Uid>, db: &mut Db) {
    for uid in image_uids.drain(..) {
        let path = &db.entries[&uid].path;
        if let Err(e) = std::fs::remove_file(path) {
            eprintln!("Remove error: {}", e);
        }
        view.delete(uid);
        db.entries.remove(&uid);
    }
}
impl EguiState {
    pub(crate) fn add_image_prop_window(&mut self, vec: Vec<Uid>) {
        self.image_prop_windows.push(ImagePropWindow::new(vec));
    }
}
