use {
    super::{icons, Action, EguiState, FileOp, PromptAction},
    crate::{
        application::Application,
        collection,
        gui::{viewer, Activity, SelectionBuf, State},
    },
    egui_sfml::{
        egui::{self, Button, Color32, Context, Label, RichText, TopBottomPanel},
        sfml::graphics::{RenderTarget, RenderWindow},
    },
};

pub(super) fn do_frame(
    state: &mut State,
    egui_state: &mut EguiState,
    egui_ctx: &Context,
    app: &mut Application,
    win: &RenderWindow,
) -> anyhow::Result<()> {
    if !egui_state.top_bar {
        return Ok(());
    }
    let n_selected = state.sel.current_mut().len();
    let mut result = Ok(());
    TopBottomPanel::top("top_panel").show(egui_ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            match state.activity {
                Activity::Thumbnails => {
                    file_menu(ui, app, state, egui_state, &mut result, win.size().x);
                    actions_menu(ui, app, egui_state, n_selected);
                    collection_menu(ui, egui_state);
                }
                Activity::Viewer => viewer::menu_ui(ui, state, win),
            }
            help_menu(ui, win, app, egui_state);
            ui.separator();
            match state.activity {
                Activity::Thumbnails => {
                    if n_selected > 0 {
                        ui.separator();
                        ui.add(Label::new(
                            RichText::new(format!("{n_selected} entries selected"))
                                .color(Color32::GREEN),
                        ));
                        if ui
                            .add(Button::new(
                                RichText::new("(Click here (or Esc) to deselect)")
                                    .color(Color32::YELLOW),
                            ))
                            .clicked()
                        {
                            state.sel.current_mut().clear();
                        }
                    }
                    let mut i = 0;
                    state.sel.bufs.retain_mut(|sel| {
                        let mut retain = true;
                        if i == state.sel.current && state.sel.rename {
                            ui.text_edit_singleline(&mut sel.name);
                            if ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                                state.sel.rename = false;
                            }
                            return true;
                        }
                        let mut re = ui.selectable_label(i == state.sel.current, &sel.name);
                        re = re.on_hover_text(format!("Selection buffer {}", i + 1));
                        re.context_menu(|ui| {
                            if ui.button("Remove").clicked() {
                                retain = false;
                                ui.close_menu();
                            }
                            if ui.button("Rename").clicked() {
                                state.sel.rename = true;
                                ui.close_menu();
                            }
                        });
                        if re.clicked() {
                            state.sel.current = i;
                        }
                        i += 1;
                        retain
                    });
                    // Ensure invariants
                    if state.sel.current >= state.sel.bufs.len() {
                        state.sel.current = state.sel.bufs.len().saturating_sub(1);
                    }
                    if state.sel.bufs.is_empty() {
                        state.sel.current = 0;
                        state.sel.bufs.push(SelectionBuf::new("Sel 1"));
                    }
                    if ui
                        .button(icons::ADD)
                        .on_hover_text("Add selection buffer")
                        .clicked()
                    {
                        state
                            .sel
                            .add_buf(format!("Sel {}", state.sel.bufs.len() + 1));
                    }
                }
                Activity::Viewer => {
                    ui.separator();
                    ui.label(format!(
                        "{}/{}",
                        state.viewer_state.index + 1,
                        state.viewer_state.image_list.len()
                    ));
                }
            }
            ui.separator();
            ui.label("(F1 to toggle this panel)");
            if !crate::gui::debug_log::LOG.with(|log| log.borrow().is_empty())
                && ui
                    .button(RichText::new(icons::WARN).strong().color(Color32::YELLOW))
                    .on_hover_text("Debug output")
                    .clicked()
            {
                egui_state.debug_window.toggle();
            }
        });
    });
    result
}

fn help_menu(ui: &mut egui::Ui, win: &RenderWindow, app: &Application, egui_state: &mut EguiState) {
    ui.menu_button("Help", |ui| {
        if ui.button([icons::QUESTION, "About"].concat()).clicked() {
            ui.close_menu();
            egui_state.modal.about();
        }
        ui.separator();
        ui.vertical_centered(|ui| {
            ui.label("= Debug =");
        });
        if ui
            .add(Button::new([icons::CAMERA, " Save screenshot"].concat()).shortcut_text("F11"))
            .clicked()
        {
            ui.close_menu();
            crate::gui::util::take_and_save_screenshot(win, egui_state);
        }
        if ui
            .button([icons::FOLDER, " Open data dir"].concat())
            .clicked()
        {
            ui.close_menu();
            if let Err(e) = open::that(&app.database.data_dir) {
                egui_state
                    .modal
                    .err(format!("Error opening database dir: {e:?}"));
            }
        }
        if ui.button([icons::TERM, " Debug window"].concat()).clicked() {
            ui.close_menu();
            egui_state.debug_window.toggle();
        }
    });
}

fn file_menu(
    ui: &mut egui::Ui,
    app: &mut Application,
    state: &mut State,
    egui_state: &mut EguiState,
    result: &mut anyhow::Result<()>,
    window_width: u32,
) {
    ui.menu_button("File", |ui| {
        if ui
            .button([icons::FOLDER, " Load folder"].concat())
            .clicked()
        {
            ui.close_menu();
            egui_state.file_op = Some(FileOp::OpenDirectory);
            egui_state.file_dialog.select_directory();
        }
        if ui.button("↺ Reload folder").clicked() {
            ui.close_menu();
            let changes = match app.reload_active_collection() {
                Ok(changes) => changes,
                Err(e) => {
                    *result = Err(e);
                    return;
                }
            };
            if !changes.empty() {
                egui_state.changes_window.open(changes);
            }
        }
        if ui
            .add_enabled(
                app.active_collection.is_some(),
                Button::new("🗀 Close folder"),
            )
            .clicked()
        {
            if let Err(e) = app.switch_collection(None) {
                *result = Err(e);
            }
            ui.close_menu();
        }
        ui.add_enabled_ui(!app.database.recent.is_empty(), |ui| {
            ui.menu_button("🕓 Recent", |ui| {
                enum Action {
                    Open(collection::Id),
                    Remove(collection::Id),
                    None,
                }
                let mut action = Action::None;
                for &id in app.database.recent.iter() {
                    ui.horizontal(|ui| {
                        match app.database.collections.get(&id) {
                            Some(coll) => {
                                if ui.button(format!("🗁 {}", &coll.display())).clicked() {
                                    action = Action::Open(id);
                                    ui.close_menu();
                                }
                            }
                            None => {
                                ui.label(format!("Dangling collection with id {id:?}"));
                            }
                        }
                        if ui.button(icons::REMOVE).clicked() {
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
                            *result = crate::gui::set_active_collection(
                                &mut state.thumbs_view,
                                app,
                                id,
                                &state.filter,
                                window_width,
                            );
                        }
                        Err(e) => {
                            egui_state
                                .modal
                                .err(format!("Error loading recent collection: {e:?}"));
                        }
                    },
                    Action::Remove(id) => app.database.recent.remove(id),
                    Action::None => {}
                }
            });
        });
        if ui
            .button([icons::CABINET, " Collections database..."].concat())
            .clicked()
        {
            egui_state.collections_db_window.open = true;
            ui.close_menu();
        }
        ui.separator();
        if ui.button("⛃⬉ Create backup").clicked() {
            ui.close_menu();
            egui_state.file_dialog.save_file();
            egui_state.file_op = Some(FileOp::CreateBackup);
        }
        if ui.button("⛃⬊ Restore backup").clicked() {
            ui.close_menu();
            egui_state.file_dialog.select_file();
            egui_state.file_op = Some(FileOp::RestoreBackup);
        }
        ui.separator();
        if ui
            .button([icons::HAMBURGER, " Preferences"].concat())
            .clicked()
        {
            ui.close_menu();
            egui_state.preferences_window.toggle();
        }
        ui.separator();
        if ui
            .button([icons::CANCEL, " Quit without saving"].concat())
            .clicked()
        {
            ui.close_menu();
            egui_state.modal.prompt(
                "Quit without saving",
                "Warning: All changes made this session will be lost.",
                PromptAction::QuitNoSave,
            );
        }
        ui.separator();
        if ui
            .add(Button::new("⎆ Quit").shortcut_text("Ctrl+Q"))
            .clicked()
        {
            egui_state.action = Some(Action::Quit);
        }
    });
}

fn collection_menu(ui: &mut egui::Ui, egui_state: &mut EguiState) {
    ui.menu_button("Collection", |ui| {
        if ui
            .add(Button::new([icons::TAG, " Tag list"].concat()).shortcut_text("T"))
            .clicked()
        {
            ui.close_menu();
            egui_state.tag_window.toggle();
        }
        if ui
            .add(Button::new("⬌ Sequences").shortcut_text("Q"))
            .clicked()
        {
            ui.close_menu();
            egui_state.sequences_window.on ^= true;
        }
        if ui.button([icons::QUESTION, " Changes"].concat()).clicked() {
            ui.close_menu();
            egui_state.changes_window.open ^= true;
        }
        if ui
            .button([icons::HAMBURGER, " Preferences"].concat())
            .clicked()
        {
            ui.close_menu();
            egui_state.coll_prefs_window.open ^= true;
        }
    });
}

fn actions_menu(
    ui: &mut egui::Ui,
    app: &Application,
    egui_state: &mut EguiState,
    n_selected: usize,
) {
    ui.menu_button("Actions", |ui| {
        let active_coll = app.active_collection.is_some();
        if ui
            .add_enabled(active_coll, Button::new("🔍 Filter").shortcut_text("F"))
            .clicked()
        {
            ui.close_menu();
            egui_state.filter_popup.on ^= true;
        }
        ui.separator();
        if ui
            .add_enabled(active_coll, Button::new("🔍 Find").shortcut_text("/"))
            .clicked()
        {
            ui.close_menu();
            egui_state.find_popup.on ^= true;
        }
        if ui
            .add_enabled(active_coll, Button::new("⮫ Next result").shortcut_text("N"))
            .clicked()
        {
            ui.close_menu();
            egui_state.action = Some(Action::FindNext);
        }
        if ui
            .add_enabled(
                active_coll,
                Button::new("⮪ Previous result").shortcut_text("P"),
            )
            .clicked()
        {
            ui.close_menu();
            egui_state.action = Some(Action::FindPrev);
        }
        ui.separator();
        if ui
            .add_enabled(
                active_coll,
                Button::new("☑ Select All").shortcut_text("ctrl+A"),
            )
            .clicked()
        {
            ui.close_menu();
            egui_state.action = Some(Action::SelectAll);
        }
        if ui
            .add_enabled(
                active_coll,
                Button::new("☐ Select None").shortcut_text("Esc"),
            )
            .clicked()
        {
            ui.close_menu();
            egui_state.action = Some(Action::SelectNone);
        }
        ui.separator();
        if ui
            .add_enabled(
                n_selected > 0,
                Button::new("Ｓ Open entries window for selected entries").shortcut_text("F2"),
            )
            .clicked()
        {
            ui.close_menu();
            egui_state.action = Some(Action::OpenEntriesWindow);
        }
        ui.separator();
        if ui
            .add_enabled(
                active_coll,
                Button::new("♻ Sort by filename").shortcut_text("S"),
            )
            .clicked()
        {
            ui.close_menu();
            egui_state.action = Some(Action::SortByPath);
        }
        if ui
            .add_enabled(active_coll, Button::new("♻ Sort by id"))
            .clicked()
        {
            ui.close_menu();
            egui_state.action = Some(Action::SortById);
        }
        if ui
            .add_enabled(
                active_coll,
                Button::new([icons::QUESTION, "Shuffle"].concat()),
            )
            .clicked()
        {
            ui.close_menu();
            egui_state.action = Some(Action::Shuffle);
        }
    });
}
