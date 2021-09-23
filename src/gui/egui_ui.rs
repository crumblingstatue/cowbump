mod entries_window;

use crate::{
    db::{local::LocalDb, TagSet},
    entry,
    filter_spec::FilterSpec,
    gui::{open_with_external, search_goto_cursor, State},
    sequence, tag,
};
use egui::{
    Align2, Button, Color32, CtxRef, Grid, ImageButton, Key, ScrollArea, TextEdit, TextureId,
    TopBottomPanel, Window,
};
use retain_mut::RetainMut;

use self::entries_window::EntriesWindow;

#[derive(Default)]
pub(crate) struct EguiState {
    entries_windows: Vec<EntriesWindow>,
    pub sequences_window: SequencesWindow,
    sequence_windows: Vec<SequenceWindow>,
    tag_window: TagWindow,
    pub(crate) action: Option<Action>,
    pub top_bar: bool,
    info_messages: Vec<InfoMessage>,
    prompts: Vec<Prompt>,
    // We just closed window with esc, ignore the esc press outside of egui
    pub just_closed_window_with_esc: bool,
}

struct SequenceWindow {
    uid: sequence::Id,
}

impl SequenceWindow {
    fn new(uid: sequence::Id) -> Self {
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
    selected_uids: TagSet,
}

struct Prompt {
    msg: InfoMessage,
    action: PromptAction,
}

enum PromptAction {
    RestoreBackup,
    QuitNoSave,
    DeleteTags(Vec<tag::Id>),
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
    SortEntries,
}

impl EguiState {
    pub fn begin_frame(&mut self) {
        self.just_closed_window_with_esc = false;
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

pub(super) fn do_ui(state: &mut State, egui_ctx: &egui::CtxRef, db: &mut LocalDb) {
    do_top_bar(state, egui_ctx, db);
    do_search_edit(state, egui_ctx, db);
    do_filter_edit(state, egui_ctx, db);
    do_tag_window(state, db, egui_ctx);
    do_sequences_window(state, db, egui_ctx);
    do_sequence_windows(state, db, egui_ctx);
    entries_window::do_frame(state, db, egui_ctx);
    do_info_messages(state, egui_ctx);
    do_prompts(state, egui_ctx, db);
}

fn do_sequence_windows(state: &mut State, db: &mut LocalDb, egui_ctx: &CtxRef) {
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
                    let seq_entries_len = seq.entries.len();
                    for (i, &img_uid) in seq.entries.iter().enumerate() {
                        ui.vertical(|ui| {
                            let img_butt =
                                ImageButton::new(TextureId::User(img_uid.0), (256., 256.));
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
                                if i < seq_entries_len - 1 && ui.button(">").clicked() {
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
                    seq.swap_entry_left(uid);
                }
                Action::SwapRight => {
                    seq.swap_entry_right(uid);
                }
                Action::Remove => {
                    seq.remove_entry(uid);
                }
                Action::Open => {
                    let mut paths = Vec::new();
                    for img_uid in seq.entry_uids_wrapped_from(uid) {
                        paths.push(db.entries[&img_uid].path.as_ref());
                    }
                    open_with_external(&paths);
                }
            }
        }
        open
    });
}

fn do_sequences_window(state: &mut State, db: &mut LocalDb, egui_ctx: &CtxRef) {
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

fn do_prompts(state: &mut State, egui_ctx: &CtxRef, db: &mut LocalDb) {
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

fn do_tag_window(state: &mut State, db: &mut LocalDb, egui_ctx: &CtxRef) {
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
            state.egui_state.just_closed_window_with_esc = true;
            state.egui_state.tag_window.on = false;
        }
    }
}

fn do_filter_edit(state: &mut State, egui_ctx: &CtxRef, db: &mut LocalDb) {
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

fn do_search_edit(state: &mut State, egui_ctx: &CtxRef, db: &mut LocalDb) {
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

fn do_top_bar(state: &mut State, egui_ctx: &CtxRef, db: &mut LocalDb) {
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
                    if ui.button("Sort entries by filename (S)").clicked() {
                        state.egui_state.action = Some(Action::SortEntries);
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

impl EguiState {
    pub(crate) fn add_entries_window(&mut self, vec: Vec<entry::Id>) {
        self.entries_windows.push(EntriesWindow::new(vec));
    }
}
