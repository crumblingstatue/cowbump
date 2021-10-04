use egui::{Button, Color32, CtxRef, Label, TopBottomPanel};
use rfd::{FileDialog, MessageDialog};
use sfml::graphics::RenderWindow;

use crate::{
    application::{self, Application},
    collection,
    gui::{native_dialog, State},
};

use super::{info_message, load_folder_window, prompt, Action, EguiState, PromptAction};

pub(super) fn do_frame(
    state: &mut State,
    egui_state: &mut EguiState,
    egui_ctx: &CtxRef,
    app: &mut Application,
    win: &RenderWindow,
) -> anyhow::Result<()> {
    let n_selected = state.selected_uids.len();
    let mut result = Ok(());
    if egui_state.top_bar {
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
                                        &app.database.collections[&id].display()
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
                                Ok(changes) => {
                                    if !changes.empty() {
                                        egui_state.changes_window.open(changes);
                                    }
                                    result = crate::gui::set_active_collection(
                                        &mut state.entries_view,
                                        app,
                                        id,
                                    );
                                }
                                Err(e) => {
                                    native_dialog::error("Error loading recent collection", e);
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
                            if let Some(id) = app.database.find_collection_by_path(&dir_path) {
                                let changes = match app.load_collection(id) {
                                    Ok(changes) => changes,
                                    Err(e) => {
                                        result = Err(e);
                                        return;
                                    }
                                };
                                if !changes.empty() {
                                    egui_state.changes_window.open(changes);
                                }
                                crate::gui::set_active_collection(&mut state.entries_view, app, id)
                                    .unwrap();
                            } else {
                                load_folder_window::open(
                                    &mut egui_state.load_folder_window,
                                    dir_path,
                                );
                            }
                        }
                    }
                    let butt =
                        Button::new("🗀 Close folder").enabled(app.active_collection.is_some());
                    if ui.add(butt).clicked() {
                        if let Err(e) = application::switch_collection(
                            &app.database.data_dir,
                            &mut app.active_collection,
                            None,
                        ) {
                            result = Err(e);
                        }
                    }
                    ui.separator();
                    if ui.button("⛃⬉ Create backup").clicked() {
                        match app.database.save_backup() {
                            Ok(_) => {
                                info_message(
                                    &mut egui_state.info_messages,
                                    "Success",
                                    "Backup successfully created.",
                                );
                            }
                            Err(e) => {
                                info_message(
                                    &mut egui_state.info_messages,
                                    "Error",
                                    &e.to_string(),
                                );
                            }
                        }
                    }
                    if ui.button("⛃⬊ Restore backup").clicked() {
                        prompt(
                            &mut egui_state.prompts,
                            "Restore Backup",
                            "Warning: This will overwrite the current contents of the database.",
                            PromptAction::RestoreBackup,
                        )
                    }
                    ui.separator();
                    if ui.button("☰ Preferences").clicked() {
                        egui_state.preferences_window.toggle();
                    }
                    ui.separator();
                    if ui.button("🗙 Quit without saving").clicked() {
                        prompt(
                            &mut egui_state.prompts,
                            "Quit without saving",
                            "Warning: All changes made this session will be lost.",
                            PromptAction::QuitNoSave,
                        )
                    }
                    ui.separator();
                    if ui.button("⎆ Quit").clicked() {
                        egui_state.action = Some(Action::Quit);
                    }
                });
                egui::menu::menu(ui, "Actions", |ui| {
                    let active_coll = app.active_collection.is_some();
                    if ui
                        .add(Button::new("🔍 Filter (F)").enabled(active_coll))
                        .clicked()
                    {
                        egui_state.filter_edit ^= true;
                    }
                    ui.separator();
                    if ui
                        .add(Button::new("🔍 Search (/)").enabled(active_coll))
                        .clicked()
                    {
                        egui_state.search_edit ^= true;
                    }
                    if ui
                        .add(Button::new("⮫ Next result (N)").enabled(active_coll))
                        .clicked()
                    {
                        egui_state.action = Some(Action::SearchNext);
                    }
                    if ui
                        .add(Button::new("⮪ Previous result (P)").enabled(active_coll))
                        .clicked()
                    {
                        egui_state.action = Some(Action::SearchPrev);
                    }
                    ui.separator();
                    if ui
                        .add(Button::new("☑ Select All (ctrl+A)").enabled(active_coll))
                        .clicked()
                    {
                        egui_state.action = Some(Action::SelectAll);
                    }
                    if ui
                        .add(Button::new("☐ Select None (Esc)").enabled(active_coll))
                        .clicked()
                    {
                        egui_state.action = Some(Action::SelectNone);
                    }
                    ui.separator();
                    if ui
                        .add(
                            Button::new("Ｓ Open entries window for selected entries (F2)")
                                .enabled(n_selected > 0),
                        )
                        .clicked()
                    {
                        egui_state.action = Some(Action::OpenEntriesWindow);
                    }
                    ui.separator();
                    if ui
                        .add(Button::new("♻ Sort by filename (S)").enabled(active_coll))
                        .clicked()
                    {
                        egui_state.action = Some(Action::SortEntries);
                    }
                });
                egui::menu::menu(ui, "Windows", |ui| {
                    if ui.button("＃ Tag list (T)").clicked() {
                        egui_state.tag_window.toggle();
                    }
                    if ui.button("⬌ Sequences (Q)").clicked() {
                        egui_state.sequences_window.on ^= true;
                    }
                });
                egui::menu::menu(ui, "Help", |ui| {
                    if ui.button("About").clicked() {
                        MessageDialog::new()
                            .set_description(&format!("Cowbump version {}", crate::VERSION))
                            .show();
                    }
                    ui.separator();
                    ui.vertical_centered(|ui| {
                        ui.label("= Debug =");
                    });
                    if ui.button("Save screenshot (F11)").clicked() {
                        crate::gui::util::take_and_save_screenshot(win);
                    }
                    if ui.button("Open data dir").clicked() {
                        open::that_in_background(&app.database.data_dir);
                    }
                    if ui.button("Debug window").clicked() {
                        egui_state.debug_window.toggle();
                    }
                });
                if n_selected > 0 {
                    ui.separator();
                    ui.add(
                        Label::new(format!("{} entries selected", n_selected))
                            .text_color(Color32::GREEN),
                    );
                    ui.add(Label::new("(Esc to deselect)").text_color(Color32::YELLOW));
                }
                ui.separator();
                ui.label("(F1 to toggle this panel)");
            });
        });
    }
    result
}
