use {
    super::{Action, EguiState, FileOp, PromptAction, icons},
    crate::{
        application::Application,
        collection::{self, SortBy, SortOrder},
        gui::{Activity, SelectionBuf, State, viewer},
    },
    anyhow::anyhow,
    constcat::concat,
    egui_sf2g::{
        egui::{self, Button, Color32, Context, Label, RichText, TopBottomPanel},
        sf2g::graphics::{RenderTarget, RenderWindow},
    },
};

pub(in crate::gui) struct TopBar {
    visible: bool,
    // TODO: Ui state. maybe should be somewhere else? Dunno.
    sel_rename: Option<usize>,
    sel_focus: bool,
}

impl Default for TopBar {
    fn default() -> Self {
        Self {
            visible: true,
            sel_rename: Default::default(),
            sel_focus: Default::default(),
        }
    }
}
impl TopBar {
    pub(in crate::gui) fn toggle(&mut self) {
        self.visible ^= true;
    }
}

pub(super) fn do_frame(
    state: &mut State,
    egui_state: &mut EguiState,
    egui_ctx: &Context,
    app: &mut Application,
    win: &RenderWindow,
) -> anyhow::Result<()> {
    if !egui_state.top_bar.visible {
        return Ok(());
    }
    let n_selected = state.sel.current_mut().map_or(0, |buf| buf.len());
    let mut result = Ok(());
    TopBottomPanel::top("top_panel").show(egui_ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            match state.activity {
                Activity::Thumbnails => {
                    file_menu(ui, app, state, egui_state, &mut result, win.size().x);
                    actions_menu(ui, app, state, egui_state, n_selected);
                    collection_menu(ui, egui_state);
                }
                Activity::Viewer => viewer::menu_ui(ui, state, win),
            }
            help_menu(ui, app, egui_state);
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
                            && let Some(buf) = state.sel.current_mut()
                        {
                            buf.clear();
                        }
                    }
                    let mut i = 0;
                    state.sel.bufs.retain_mut(|sel| {
                        let mut retain = true;
                        if egui_state.top_bar.sel_rename == Some(i) {
                            let re = ui.text_edit_singleline(&mut sel.name);
                            if egui_state.top_bar.sel_focus {
                                re.request_focus();
                                egui_state.top_bar.sel_focus = false;
                            }
                            if re.lost_focus()
                                && ui.input(|inp| {
                                    inp.key_pressed(egui::Key::Enter)
                                        || inp.key_pressed(egui::Key::Escape)
                                })
                            {
                                egui_state.top_bar.sel_rename = None;
                            }
                            i += 1;
                            return true;
                        }
                        let mut re = ui.selectable_label(i == state.sel.current, &sel.name);
                        re = re.on_hover_text(format!("Selection buffer {}", i + 1));
                        re.context_menu(|ui| {
                            if ui.button("Remove").clicked() {
                                retain = false;
                            }
                            if ui.button("Rename").clicked() {
                                egui_state.top_bar.sel_rename = Some(i);
                                egui_state.top_bar.sel_focus = true;
                            }
                        });
                        if re.clicked() {
                            state.sel.current = i;
                        }
                        if re.double_clicked() {
                            egui_state.top_bar.sel_rename = Some(i);
                            egui_state.top_bar.sel_focus = true;
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
            let log = crate::gui::debug_log::LOG.lock();
            if !log.is_empty()
                && ui
                    .button(
                        RichText::new(format!("{} {}", icons::WARN, log.len()))
                            .strong()
                            .color(Color32::YELLOW),
                    )
                    .on_hover_text("Debug output")
                    .clicked()
            {
                egui_state.debug_window.toggle();
            }
        });
        if egui_state.loading_changes_notify {
            ui.horizontal(|ui| {
                ui.label("Scanning folder changes...");
                ui.spinner();
            });
        }
    });
    result
}

fn help_menu(ui: &mut egui::Ui, app: &Application, egui_state: &mut EguiState) {
    ui.menu_button("Help", |ui| {
        if ui.button(concat!(icons::QUESTION, "About")).clicked() {
            egui_state.modal.about();
        }
        if ui.button(concat!(icons::QUESTION, "Keybinds")).clicked() {
            egui_state.modal.keybinds();
        }
        ui.separator();
        ui.vertical_centered(|ui| {
            ui.label("= Debug =");
        });
        if ui
            .button(concat!(icons::FOLDER, " Open data dir"))
            .clicked()
            && let Err(e) = open::that(&app.database.data_dir)
        {
            egui_state
                .modal
                .err(format!("Error opening database dir: {e:?}"));
        }
        if ui.button(concat!(icons::TERM, " Debug window")).clicked() {
            egui_state.debug_window.toggle();
        }
        ui.separator();
        ui.menu_button("Simulate popup", |ui| {
            if ui.button("Error popup").clicked() {
                egui_state
                    .modal
                    .err(format!("{:?}", anyhow!("Simulated error just happened!")));
            }
            if ui.button("Success popup").clicked() {
                egui_state.modal.success("Something successfully happened.");
            }
            if ui.button("Panic (crash Cowbump)").clicked() {
                egui_state.modal.prompt(
                    "Panic?",
                    "Are you sure you want to crash cowbump?\nUnsaved data will be lost!",
                    PromptAction::PanicTest,
                );
            }
        });
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
        if ui.button(concat!(icons::FOLDER, " Load folder")).clicked() {
            egui_state.file_op = Some(FileOp::OpenDirectory);
            egui_state.file_dialog.pick_directory();
        }
        if ui.button("↺ Reload folder").clicked() {
            match app.reload_active_collection() {
                Ok(changes) => changes,
                Err(e) => {
                    *result = Err(e);
                    return;
                }
            };
        }
        if ui
            .add_enabled(
                app.active_collection.is_some(),
                Button::new("🗀 Close folder"),
            )
            .clicked()
            && let Err(e) = app.switch_collection(None)
        {
            *result = Err(e);
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
                        Ok(()) => {
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
            .button(concat!(icons::CABINET, " Collections database..."))
            .clicked()
        {
            egui_state.collections_db_window.open = true;
        }
        ui.separator();
        if ui.button("⛃⬉ Create backup").clicked() {
            egui_state.file_dialog.save_file();
            egui_state.file_op = Some(FileOp::CreateBackup);
        }
        if ui.button("⛃⬊ Restore backup").clicked() {
            egui_state.file_dialog.pick_file();
            egui_state.file_op = Some(FileOp::RestoreBackup);
        }
        ui.separator();
        if ui
            .button(concat!(icons::HAMBURGER, " Preferences"))
            .clicked()
        {
            egui_state.preferences_window.toggle();
        }
        ui.separator();
        if ui
            .button(concat!(icons::CANCEL, " Quit without saving"))
            .clicked()
        {
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
            .add(Button::new(concat!(icons::TAG, " Tag list")).shortcut_text("T"))
            .clicked()
        {
            egui_state.tag_window.toggle();
        }
        if ui
            .add(Button::new("⬌ Sequences").shortcut_text("Q"))
            .clicked()
        {
            egui_state.sequences_window.on ^= true;
        }
        if ui.button(concat!(icons::QUESTION, " Changes")).clicked() {
            egui_state.changes_window.open ^= true;
        }
        if ui
            .button(concat!(icons::HAMBURGER, " Preferences"))
            .clicked()
        {
            egui_state.coll_prefs_window.open ^= true;
        }
    });
}

fn actions_menu(
    ui: &mut egui::Ui,
    app: &Application,
    state: &mut State,
    egui_state: &mut EguiState,
    n_selected: usize,
) {
    ui.menu_button("Actions", |ui| {
        let active_coll = app.active_collection.is_some();
        if ui
            .add_enabled(active_coll, Button::new("🔍 Filter").shortcut_text("F"))
            .clicked()
        {
            egui_state.filter_popup.on ^= true;
        }
        ui.separator();
        if ui
            .add_enabled(active_coll, Button::new("🔍 Find").shortcut_text("/"))
            .clicked()
        {
            egui_state.find_popup.on ^= true;
        }
        if ui
            .add_enabled(active_coll, Button::new("⮫ Next result").shortcut_text("N"))
            .clicked()
        {
            egui_state.action = Some(Action::FindNext);
        }
        if ui
            .add_enabled(
                active_coll,
                Button::new("⮪ Previous result").shortcut_text("P"),
            )
            .clicked()
        {
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
            egui_state.action = Some(Action::SelectAll);
        }
        if ui
            .add_enabled(
                active_coll,
                Button::new("☐ Select None").shortcut_text("Esc"),
            )
            .clicked()
        {
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
            egui_state.action = Some(Action::OpenEntriesWindow);
        }
        ui.separator();
        ui.menu_button(concat!(icons::SORT, " Sort"), |ui| {
            if !active_coll {
                ui.disable();
            }
            if ui
                .add(Button::new(concat!(icons::SORT, " Sort")).shortcut_text("S"))
                .clicked()
            {
                egui_state.action = Some(Action::Sort);
            }
            if ui
                .add(Button::new(concat!(icons::QUESTION, " Shuffle")).shortcut_text("R"))
                .clicked()
            {
                egui_state.action = Some(Action::Shuffle);
            }
            ui.separator();
            ui.selectable_value(&mut state.thumbs_view.sort_by, SortBy::Path, "By filename");
            ui.selectable_value(&mut state.thumbs_view.sort_by, SortBy::Id, "By id");
            ui.selectable_value(
                &mut state.thumbs_view.sort_by,
                SortBy::NTags,
                "By number of tags",
            );
            ui.separator();
            ui.selectable_value(
                &mut state.thumbs_view.sort_order,
                SortOrder::Asc,
                "Ascending",
            );
            ui.selectable_value(
                &mut state.thumbs_view.sort_order,
                SortOrder::Desc,
                "Descending",
            );
        });
    });
}
