use anyhow::Context;
use egui::{Button, CtxRef, TopBottomPanel};
use rfd::{FileDialog, MessageDialog};

use crate::{
    application::Application,
    collection,
    gui::{entries_view::EntriesView, State},
};

use super::{info_message, load_folder_window, prompt, Action, PromptAction};

pub(super) fn do_frame(
    state: &mut State,
    egui_ctx: &CtxRef,
    app: &mut Application,
) -> anyhow::Result<()> {
    let mut result = Ok(());
    if state.egui_state.top_bar {
        TopBottomPanel::top("top_panel").show(egui_ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if app.database.recent.len() > 0 {
                    egui::menu::menu(ui, "Recent", |ui| {
                        enum Action {
                            Open(collection::Id),
                            Remove(collection::Id),
                            None,
                        }
                        let mut action = Action::None;
                        for &id in app.database.recent.iter() {
                            ui.horizontal(|ui| {
                                if ui
                                    .button(&format!(
                                        "🗁 {}",
                                        &app.database.collections[&id].root_path.display()
                                    ))
                                    .clicked()
                                {
                                    action = Action::Open(id);
                                }
                                if ui.button("🗑").clicked() {
                                    action = Action::Remove(id);
                                }
                            });
                        }
                        match action {
                            Action::Open(id) => match app.load_collection(id) {
                                Ok(()) => {
                                    state.entries_view = EntriesView::from_collection(
                                        &app.database.collections[&app.active_collection.unwrap()],
                                    );
                                    result = std::env::set_current_dir(
                                        &app.database.collections[&id].root_path,
                                    )
                                    .context("failed to set directory");
                                }
                                Err(e) => {
                                    MessageDialog::new()
                                        .set_title("Error")
                                        .set_description(&e.to_string())
                                        .show();
                                }
                            },
                            Action::Remove(id) => app.database.recent.remove(id),
                            Action::None => {}
                        }
                    });
                }
                egui::menu::menu(ui, "File", |ui| {
                    if ui.button("🗁 Load folder").clicked() {
                        if let Some(dir_path) = FileDialog::new().pick_folder() {
                            load_folder_window::open(
                                &mut state.egui_state.load_folder_window,
                                dir_path,
                            );
                        }
                    }
                    let butt =
                        Button::new("🗀 Close folder").enabled(app.active_collection.is_some());
                    if ui.add(butt).clicked() {
                        app.active_collection = None;
                    }
                    ui.separator();
                    if ui.button("⛃⬉ Create backup").clicked() {
                        match app.database.save_backup() {
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
                    if ui.button("⛃⬊ Restore backup").clicked() {
                        prompt(
                            &mut state.egui_state.prompts,
                            "Restore Backup",
                            "Warning: This will overwrite the current contents of the database.",
                            PromptAction::RestoreBackup,
                        )
                    }
                    ui.separator();
                    if ui.button("☰ Preferences").clicked() {
                        state.egui_state.preferences_window.toggle();
                    }
                    ui.separator();
                    if ui.button("🗙 Quit without saving").clicked() {
                        prompt(
                            &mut state.egui_state.prompts,
                            "Quit without saving",
                            "Warning: All changes made this session will be lost.",
                            PromptAction::QuitNoSave,
                        )
                    }
                    ui.separator();
                    if ui.button("⎆ Quit").clicked() {
                        state.egui_state.action = Some(Action::Quit);
                    }
                });
                egui::menu::menu(ui, "Actions", |ui| {
                    let active_coll = app.active_collection.is_some();
                    ui.separator();
                    if ui
                        .add(Button::new("🔍 Filter (F)").enabled(active_coll))
                        .clicked()
                    {
                        state.filter_edit ^= true;
                    }
                    ui.separator();
                    if ui
                        .add(Button::new("🔍 Search (/)").enabled(active_coll))
                        .clicked()
                    {
                        state.search_edit ^= true;
                    }
                    if ui
                        .add(Button::new("⮫ Next result (N)").enabled(active_coll))
                        .clicked()
                    {
                        state.egui_state.action = Some(Action::SearchNext);
                    }
                    if ui
                        .add(Button::new("⮪ Previous result (P)").enabled(active_coll))
                        .clicked()
                    {
                        state.egui_state.action = Some(Action::SearchPrev);
                    }
                    ui.separator();
                    if ui
                        .add(Button::new("☑ Select All (ctrl+A)").enabled(active_coll))
                        .clicked()
                    {
                        state.egui_state.action = Some(Action::SelectAll);
                    }
                    if ui
                        .add(Button::new("☐ Select None (Esc)").enabled(active_coll))
                        .clicked()
                    {
                        state.egui_state.action = Some(Action::SelectNone);
                    }
                    ui.separator();
                    if ui
                        .add(Button::new("♻ Sort by filename (S)").enabled(active_coll))
                        .clicked()
                    {
                        state.egui_state.action = Some(Action::SortEntries);
                    }
                });
                egui::menu::menu(ui, "Windows", |ui| {
                    ui.separator();
                    if ui.button("＃ Tag list (T)").clicked() {
                        state.egui_state.tag_window.toggle();
                    }
                    if ui.button("⬌ Sequences (Q)").clicked() {
                        state.egui_state.sequences_window.on ^= true;
                    }
                });
                ui.separator();
                ui.label("(F1 to toggle)");
            });
        });
    }
    result
}
