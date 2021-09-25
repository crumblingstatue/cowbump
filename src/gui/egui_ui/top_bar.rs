use egui::{Button, CtxRef, TopBottomPanel};
use rfd::{FileDialog, MessageDialog};

use crate::{
    application::Application,
    gui::{entries_view::EntriesView, State},
};

use super::{info_message, prompt, Action, PromptAction};

pub(super) fn do_frame(state: &mut State, egui_ctx: &CtxRef, app: &mut Application) {
    if state.egui_state.top_bar {
        TopBottomPanel::top("top_panel").show(egui_ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu(ui, "File", |ui| {
                    if ui.button("Load folder").clicked() {
                        if let Some(dir_path) = FileDialog::new().pick_folder() {
                            match app.load_folder(dir_path) {
                                Ok(()) => {
                                    state.entries_view =
                                        EntriesView::from_db(app.local_db.as_ref().unwrap());
                                }
                                Err(e) => {
                                    MessageDialog::new()
                                        .set_title("Error")
                                        .set_description(&e.to_string())
                                        .show();
                                }
                            }
                        }
                    }
                    let butt = Button::new("Close folder").enabled(app.local_db.is_some());
                    if ui.add(butt).clicked() {
                        app.local_db = None;
                    }
                    ui.separator();
                    if let Some(db) = &app.local_db {
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
                        state.egui_state.tag_window.toggle();
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
