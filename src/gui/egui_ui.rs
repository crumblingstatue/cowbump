mod entries_window;
mod sequences;
mod tag_list;
mod top_bar;

use crate::{
    db::local::LocalDb,
    entry,
    filter_spec::FilterSpec,
    gui::{search_goto_cursor, State},
    tag,
};
use egui::{Align2, Color32, CtxRef, TextEdit, Window};
use retain_mut::RetainMut;

use self::{
    entries_window::EntriesWindow,
    sequences::{SequenceWindow, SequencesWindow},
    tag_list::TagWindow,
};

#[derive(Default)]
pub(crate) struct EguiState {
    entries_windows: Vec<EntriesWindow>,
    pub sequences_window: SequencesWindow,
    sequence_windows: Vec<SequenceWindow>,
    pub tag_window: TagWindow,
    pub(crate) action: Option<Action>,
    pub top_bar: bool,
    info_messages: Vec<InfoMessage>,
    prompts: Vec<Prompt>,
    // We just closed window with esc, ignore the esc press outside of egui
    pub just_closed_window_with_esc: bool,
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
    top_bar::do_frame(state, egui_ctx, db);
    do_search_edit(state, egui_ctx, db);
    do_filter_edit(state, egui_ctx, db);
    tag_list::do_frame(state, db, egui_ctx);
    sequences::do_sequences_window(state, db, egui_ctx);
    sequences::do_sequence_windows(state, db, egui_ctx);
    entries_window::do_frame(state, db, egui_ctx);
    do_info_messages(state, egui_ctx);
    do_prompts(state, egui_ctx, db);
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
