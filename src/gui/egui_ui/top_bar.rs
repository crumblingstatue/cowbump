use egui::{Button, Color32, CtxRef, Label, TopBottomPanel};
use rfd::{FileDialog, MessageButtons, MessageDialog};
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
                ui.menu_button("File", |ui| {
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
                        }}
                    if ui.add_enabled(app.active_collection.is_some(), Button::new("🗀 Close folder")).clicked() {
                        if let Err(e) = application::switch_collection(
                            &app.database.data_dir,
                            &mut app.active_collection,
                            None,
                        ) {
                            result = Err(e);
                        }
                    }
                    ui.add_enabled_ui(app.database.recent.len() > 0, |ui| {
                        ui.menu_button("🕓 Recent", |ui| {
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
                    });
                    ui.separator();
                    if ui.button("⛃⬉ Create backup").clicked() {
                        if let Some(path) = FileDialog::new()
                            .set_file_name("cowbump_backup.zip")
                            .save_file()
                        {
                            let result: anyhow::Result<()> = try
                            {
                                app.save_active_collection()?;
                                app.database.save_backups(&path)?;
                            };
                            match result {
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
                    }
                    if ui.button("⛃⬊ Restore backup").clicked() {
                        let continue_ = MessageDialog::new()
                        .set_buttons(MessageButtons::OkCancel).
                        set_title("Restore backup").
                        set_description("This will replace all your current data with the backup. Continue?").show();
                        if continue_ {
                            if let Some(path) = FileDialog::new().pick_file() {
                                app.active_collection = None;
                                if let Err(e) = app.database.restore_backups_from(&path) {
                                    native_dialog::error("Failed to restore backup", e);
                                } else {
                                    MessageDialog::new().set_title("Backup restored!").show();
                                }
                            }
                        }
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
                ui.menu_button("Actions", |ui| {
                    let active_coll = app.active_collection.is_some();
                    if ui
                        .add_enabled(active_coll, Button::new("🔍 Filter (F)"))
                        .clicked()
                    {
                        egui_state.filter_popup.on ^= true;
                    }
                    ui.separator();
                    if ui
                        .add_enabled(active_coll, Button::new("🔍 Search (/)"))
                        .clicked()
                    {
                        egui_state.search_edit ^= true;
                    }
                    if ui
                        .add_enabled(active_coll, Button::new("⮫ Next result (N)"))
                        .clicked()
                    {
                        egui_state.action = Some(Action::SearchNext);
                    }
                    if ui
                        .add_enabled(active_coll, Button::new("⮪ Previous result (P)"))
                        .clicked()
                    {
                        egui_state.action = Some(Action::SearchPrev);
                    }
                    ui.separator();
                    if ui
                        .add_enabled(active_coll, Button::new("☑ Select All (ctrl+A)"))
                        .clicked()
                    {
                        egui_state.action = Some(Action::SelectAll);
                    }
                    if ui
                        .add_enabled(active_coll, Button::new("☐ Select None (Esc)"))
                        .clicked()
                    {
                        egui_state.action = Some(Action::SelectNone);
                    }
                    ui.separator();
                    if ui
                        .add_enabled(n_selected > 0,
                            Button::new("Ｓ Open entries window for selected entries (F2)")
                        )
                        .clicked()
                    {
                        egui_state.action = Some(Action::OpenEntriesWindow);
                    }
                    ui.separator();
                    if ui
                        .add_enabled(active_coll, Button::new("♻ Sort by filename (S)"))
                        .clicked()
                    {
                        egui_state.action = Some(Action::SortEntries);
                    }
                });
                ui.menu_button("Windows", |ui| {
                    if ui.button("＃ Tag list (T)").clicked() {
                        egui_state.tag_window.toggle();
                    }
                    if ui.button("⬌ Sequences (Q)").clicked() {
                        egui_state.sequences_window.on ^= true;
                    }
                });
                ui.menu_button("Help", |ui| {
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
