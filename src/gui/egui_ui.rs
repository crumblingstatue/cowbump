mod batch_rename_window;
mod changes_window;
mod coll_prefs_window;
mod collections_window;
mod debug_window;
mod entries_window;
mod filter_popup;
mod find_popup;
mod icons;
mod load_folder_window;
mod preferences_window;
mod query_popup;
mod sequences;
mod tag_autocomplete;
mod tag_list;
mod top_bar;
mod ui_ext;

use {
    self::{
        batch_rename_window::BatchRenameWindow,
        changes_window::ChangesWindow,
        coll_prefs_window::CollPrefsWindow,
        collections_window::CollectionsDbWindow,
        debug_window::DebugWindow,
        entries_window::EntriesWindow,
        load_folder_window::LoadFolderWindow,
        preferences_window::PreferencesWindow,
        query_popup::QueryPopup,
        sequences::{SequenceWindow, SequencesWindow},
        tag_list::TagWindow,
    },
    super::{get_tex_for_entry, resources::Resources},
    crate::{application::Application, collection::Collection, entry, gui::State, tag},
    anyhow::Context as _,
    egui_file_dialog::FileDialog,
    egui_sfml::{
        egui::{self, Context, FontFamily, FontId, TextStyle, Window},
        sfml::{
            self,
            graphics::{RenderTarget, RenderWindow, Texture},
        },
    },
};

pub(crate) struct EguiState {
    entries_windows: Vec<EntriesWindow>,
    pub sequences_window: SequencesWindow,
    sequence_windows: Vec<SequenceWindow>,
    pub preferences_window: PreferencesWindow,
    pub tag_window: TagWindow,
    pub(crate) action: Option<Action>,
    pub top_bar: bool,
    pub load_folder_window: LoadFolderWindow,
    pub(crate) changes_window: ChangesWindow,
    // We just closed window with esc, ignore the esc press outside of egui
    pub just_closed_window_with_esc: bool,
    pub debug_window: DebugWindow,
    pub find_popup: QueryPopup,
    pub filter_popup: QueryPopup,
    /// Uid counter for egui entities like windows
    egui_uid_counter: u64,
    pub(crate) coll_prefs_window: CollPrefsWindow,
    pub(crate) batch_rename_window: BatchRenameWindow,
    pub(crate) collections_db_window: CollectionsDbWindow,
    pub(crate) file_dialog: FileDialog,
    /// If `Some`, save this screenshot to the selected path of the file dialog
    pub(crate) file_op: Option<FileOp>,
    pub(crate) modal: ModalDialog,
}

pub(crate) enum FileOp {
    OpenDirectory,
    SaveScreenshot(sfml::graphics::Image),
    CreateBackup,
    RestoreBackup,
}

impl EguiState {
    pub(crate) fn new() -> Self {
        Self {
            entries_windows: Default::default(),
            sequences_window: Default::default(),
            sequence_windows: Default::default(),
            preferences_window: Default::default(),
            tag_window: Default::default(),
            action: Default::default(),
            top_bar: true,
            load_folder_window: Default::default(),
            changes_window: Default::default(),
            just_closed_window_with_esc: Default::default(),
            debug_window: Default::default(),
            find_popup: Default::default(),
            filter_popup: Default::default(),
            egui_uid_counter: 0,
            coll_prefs_window: Default::default(),
            batch_rename_window: Default::default(),
            collections_db_window: Default::default(),
            file_dialog: FileDialog::new()
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::default()),
            file_op: None,
            modal: ModalDialog::default(),
        }
    }
}

#[derive(Clone)]
pub enum PromptAction {
    QuitNoSave,
    DeleteTags(Vec<tag::Id>),
}

#[derive(Default)]
pub struct ModalDialog {
    payload: Option<ModalPayload>,
}

enum ModalPayload {
    Err(String),
    Success(String),
    About,
    Prompt {
        title: String,
        message: String,
        action: PromptAction,
    },
}

impl ModalDialog {
    pub fn err(&mut self, body: impl std::fmt::Display) {
        self.payload = Some(ModalPayload::Err(body.to_string()));
    }
    pub fn about(&mut self) {
        self.payload = Some(ModalPayload::About);
    }
    pub fn success(&mut self, msg: impl std::fmt::Display) {
        self.payload = Some(ModalPayload::Success(msg.to_string()));
    }
    pub fn prompt(
        &mut self,
        title: impl Into<String>,
        message: impl Into<String>,
        action: PromptAction,
    ) {
        self.payload = Some(ModalPayload::Prompt {
            title: title.into(),
            message: message.into(),
            action,
        });
    }
    pub fn show_payload(&mut self, ctx: &Context) -> Option<PromptAction> {
        let mut action = None;
        if let Some(payload) = &self.payload {
            let (key_enter, key_esc) = ctx.input_mut(|inp| {
                (
                    inp.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                    inp.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                )
            });
            let mut close = false;
            show_modal_ui(ctx, |ui| match payload {
                ModalPayload::Err(s) => {
                    let rect = ui.ctx().screen_rect();
                    ui.set_width(rect.width() - 128.0);
                    ui.vertical_centered(|ui| {
                        ui.heading("Error");
                        egui::ScrollArea::vertical()
                            .max_height(rect.height() - 128.0)
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut s.as_str())
                                        .code_editor()
                                        .desired_width(ui.available_width()),
                                );
                            });
                        if ui.button("Ok").clicked() || key_enter || key_esc {
                            close = true;
                        }
                    });
                }
                ModalPayload::Success(s) => {
                    ui.vertical_centered(|ui| {
                        ui.label(s);
                        ui.add_space(16.0);
                        if ui.button("Close").clicked() || key_enter || key_esc {
                            close = true;
                        }
                    });
                }
                ModalPayload::About => {
                    ui.vertical_centered(|ui| {
                        ui.label(["Cowbump version ", crate::VERSION].concat());
                        ui.add_space(16.0);
                        if ui.button("Close").clicked() || key_enter || key_esc {
                            close = true;
                        }
                    });
                }
                ModalPayload::Prompt {
                    title,
                    message,
                    action: prompt_action,
                } => {
                    ui.with_layout(
                        egui::Layout::top_down_justified(egui::Align::Center),
                        |ui| {
                            ui.heading(title);
                            ui.label(message);
                            if ui.button([icons::CHECK, " Ok"].concat()).clicked() || key_enter {
                                action = Some(prompt_action.clone());
                                close = true;
                            }
                            if ui.button(icons::CANCEL_TEXT).clicked() || key_esc {
                                close = true;
                            }
                        },
                    );
                }
            });
            if close {
                self.payload = None;
            }
        }
        action
    }
}

fn show_modal_ui(ctx: &Context, ui_fn: impl FnOnce(&mut egui::Ui)) {
    let re = egui::Area::new(egui::Id::new("modal_area"))
        .fixed_pos(egui::Pos2::ZERO)
        .show(ctx, |ui| {
            let screen_rect = ui.ctx().input(|inp| inp.screen_rect);
            ui.allocate_response(screen_rect.size(), egui::Sense::click());
            ui.painter().rect_filled(
                screen_rect,
                egui::Rounding::ZERO,
                egui::Color32::from_rgba_premultiplied(0, 0, 0, 200),
            );
        });
    ctx.move_to_top(re.response.layer_id);
    let re = Window::new("egui_modal_popup")
        .title_bar(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui_fn(ui);
        });
    if let Some(re) = re {
        // This helps steal keyboard focus from underlying ui and app
        re.response.request_focus();
        ctx.move_to_top(re.response.layer_id);
    }
}

pub(crate) enum Action {
    Quit,
    QuitNoSave,
    FindNext,
    FindPrev,
    SelectAll,
    SelectNone,
    SortByPath,
    SortById,
    OpenEntriesWindow,
    Shuffle,
}

impl EguiState {
    pub fn begin_frame(&mut self) {
        self.just_closed_window_with_esc = false;
        self.action = None;
    }
}

pub(super) fn do_ui(
    state: &mut State,
    egui_state: &mut EguiState,
    egui_ctx: &Context,
    app: &mut Application,
    res: &Resources,
    win: &RenderWindow,
) -> anyhow::Result<()> {
    // Do the modal handling first, so it can steal Esc/Enter inputs
    if let Some(action) = egui_state.modal.show_payload(egui_ctx) {
        match action {
            PromptAction::QuitNoSave => {
                egui_state.action = Some(Action::QuitNoSave);
            }
            PromptAction::DeleteTags(ref uids) => {
                let Some((_, coll)) = &mut app.active_collection else {
                    anyhow::bail!("No active collection");
                };
                coll.remove_tags(uids);
            }
        }
    }
    top_bar::do_frame(state, egui_state, egui_ctx, app, win)?;
    preferences_window::do_frame(state, egui_state, app, egui_ctx, win);
    load_folder_window::do_frame(state, egui_state, egui_ctx, res, app, win.size().x);
    changes_window::do_frame(state, egui_state, egui_ctx, app, win);
    debug_window::do_frame(egui_state, egui_ctx);
    collections_window::do_frame(app, egui_state, egui_ctx);
    if let Some((_id, coll)) = app.active_collection.as_mut() {
        find_popup::do_frame(state, egui_state, egui_ctx, coll, win);
        if filter_popup::do_frame(state, egui_state, egui_ctx, coll) {
            state
                .thumbs_view
                .update_from_collection(coll, &state.filter);
            state.thumbs_view.clamp_bottom(win);
        }
        tag_list::do_frame(
            state,
            egui_state,
            coll,
            egui_ctx,
            &mut app.database.uid_counter,
        );
        sequences::do_sequences_window(
            state,
            egui_state,
            coll,
            &mut app.database.uid_counter,
            egui_ctx,
            &mut app.database.preferences,
            win,
        );
        sequences::do_sequence_windows(
            state,
            egui_state,
            coll,
            egui_ctx,
            &mut app.database.preferences,
            win,
        );
        coll_prefs_window::do_frame(egui_state, coll, egui_ctx, &app.database.preferences);
        entries_window::do_frame(
            state,
            egui_state,
            coll,
            egui_ctx,
            win,
            &mut app.database,
            res,
        );
        batch_rename_window::do_frame(state, egui_state, coll, egui_ctx, win);
    }
    if let Some(op) = &egui_state.file_op
        && let Some(path) = egui_state.file_dialog.take_selected()
    {
        match op {
            FileOp::OpenDirectory => {
                if let Some(id) = app.database.find_collection_by_path(&path) {
                    let changes = app.load_collection(id)?;
                    if !changes.empty() {
                        egui_state.changes_window.open(changes);
                    }
                    let result = crate::gui::set_active_collection(
                        &mut state.thumbs_view,
                        app,
                        id,
                        &state.filter,
                        win.size().x,
                    );
                    if let Err(e) = result {
                        egui_state
                            .modal
                            .err(format!("Failed to set active collection: {e:?}"));
                    }
                } else {
                    load_folder_window::open(&mut egui_state.load_folder_window, path);
                }
            }
            FileOp::SaveScreenshot(ss) => {
                let path_str = path.to_str().context("Failed to convert path to str")?;
                ss.save_to_file(path_str).context("Failed to save image")?;
                egui_state
                    .modal
                    .success(format!("Saved screenshot to {path_str}"));
            }
            FileOp::CreateBackup => {
                let result: anyhow::Result<()> = try {
                    app.save_active_collection()?;
                    app.database.save_backups(&path)?;
                };
                match result {
                    Ok(_) => {
                        egui_state.modal.success("Backup successfully created.");
                    }
                    Err(e) => {
                        egui_state.modal.err(format!("Error creating backup: {e}"));
                    }
                }
            }
            FileOp::RestoreBackup => {
                app.active_collection = None;
                if let Err(e) = app.database.restore_backups_from(&path) {
                    crate::gui::native_dialog::error_blocking("Failed to restore backup", e);
                } else {
                    egui_state.modal.success("Backup restored");
                }
            }
        }
        egui_state.file_op = None;
    }
    egui_state.file_dialog.update(egui_ctx);
    Ok(())
}

impl EguiState {
    pub(crate) fn add_entries_window(&mut self, vec: Vec<entry::Id>) {
        self.entries_windows
            .push(EntriesWindow::new(vec, self.egui_uid_counter));
        self.egui_uid_counter += 1;
    }
}

pub fn set_up_style(ctx: &Context, pref_style: &crate::preferences::Style) {
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(pref_style.heading_size, FontFamily::Proportional),
        ),
        (
            TextStyle::Button,
            FontId::new(pref_style.button_size, FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(pref_style.body_size, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(pref_style.monospace_size, FontFamily::Monospace),
        ),
    ]
    .into();
    ctx.set_style(style);
}

pub(super) struct TexSrc<'state, 'res, 'db> {
    state: &'state mut State,
    res: &'res Resources,
    coll: Option<&'db Collection>,
}

impl<'state, 'res, 'db> TexSrc<'state, 'res, 'db> {
    pub(super) fn new(
        state: &'state mut State,
        res: &'res Resources,
        app: &'db Application,
    ) -> Self {
        TexSrc {
            state,
            res,
            coll: app.active_collection.as_ref().map(|(_id, col)| col),
        }
    }
}

impl egui_sfml::UserTexSource for TexSrc<'_, '_, '_> {
    fn get_texture(&mut self, id: u64) -> (f32, f32, &Texture) {
        let tex = match self.coll {
            Some(coll) => {
                get_tex_for_entry(
                    &self.state.thumbnail_cache,
                    entry::Id(id),
                    coll,
                    &self.state.thumbnail_loader,
                    self.state.thumbs_view.thumb_size,
                    self.res,
                )
                .1
            }
            None => &*self.res.error_texture,
        };
        (tex.size().x as f32, tex.size().y as f32, tex)
    }
}
