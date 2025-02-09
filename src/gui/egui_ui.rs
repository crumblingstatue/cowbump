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
mod modal;
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
    crate::{
        application::Application,
        collection::{Collection, TagsExt},
        entry,
        gui::State,
        preferences::{LightDarkPref, Preferences},
    },
    anyhow::Context as _,
    egui_colors::Colorix,
    egui_file_dialog::FileDialog,
    egui_sfml::{
        egui::{self, Context, FontFamily, FontId, TextStyle, ThemePreference},
        sfml::{
            cpp::FBox,
            graphics::{Image, RenderTarget, RenderWindow, Texture},
        },
    },
    modal::{ModalDialog, PromptAction},
    top_bar::TopBar,
};

pub(crate) struct EguiState {
    pub(super) top_bar: TopBar,
    entries_windows: Vec<EntriesWindow>,
    pub sequences_window: SequencesWindow,
    sequence_windows: Vec<SequenceWindow>,
    pub preferences_window: PreferencesWindow,
    pub tag_window: TagWindow,
    pub(crate) action: Option<Action>,
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
    pub(crate) colorix: Option<Colorix>,
    /// Whether to tell the user we're loading folder changes
    pub loading_changes_notify: bool,
}

pub(crate) enum FileOp {
    OpenDirectory,
    SaveScreenshot(FBox<Image>),
    CreateBackup,
    RestoreBackup,
}

impl EguiState {
    pub(crate) fn new(prefs: &Preferences, egui_ctx: &Context) -> Self {
        Self {
            entries_windows: Default::default(),
            sequences_window: Default::default(),
            sequence_windows: Default::default(),
            preferences_window: Default::default(),
            tag_window: Default::default(),
            action: Default::default(),
            top_bar: Default::default(),
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
            colorix: prefs.color_theme.as_ref().map(|theme| {
                if let Some(pref) = &theme.light_dark_preference {
                    match pref {
                        LightDarkPref::Light => egui_ctx.set_theme(ThemePreference::Light),
                        LightDarkPref::Dark => egui_ctx.set_theme(ThemePreference::Dark),
                    }
                }
                Colorix::global(egui_ctx, theme.to_colorix())
            }),
            loading_changes_notify: false,
        }
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
    SortByNTags,
}

impl EguiState {
    pub fn begin_frame(&mut self) {
        self.just_closed_window_with_esc = false;
        self.action = None;
    }
}

/// Do the egui ui update thingy.
///
/// # Panics
///
/// During prompt action handling, there is a [`PromptAction::PanicTest`], which will cause a panic.
pub(super) fn do_ui(
    state: &mut State,
    egui_state: &mut EguiState,
    egui_ctx: &Context,
    app: &mut Application,
    res: &Resources,
    win: &RenderWindow,
) -> anyhow::Result<()> {
    // Do the modal handling first, so it can steal Esc/Enter inputs
    if let Some(action) = egui_state
        .modal
        .show_payload(egui_ctx, &mut state.clipboard_ctx)
    {
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
            PromptAction::MergeTag { merge, into } => {
                let Some((_, coll)) = &mut app.active_collection else {
                    anyhow::bail!("No active collection");
                };
                coll.merge_tags(merge, into)?;
                let into_name = coll.tags.first_name_of(&into);
                egui_state
                    .modal
                    .success(format!("Successful merge into {into_name}"));
            }
            PromptAction::PanicTest => panic!("User inflicted panic"),
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
        && let Some(path) = egui_state.file_dialog.take_picked()
    {
        match op {
            FileOp::OpenDirectory => {
                if let Some(id) = app.database.find_collection_by_path(&path) {
                    app.load_collection(id)?;
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
                    egui_state
                        .modal
                        .err(format!("Failed to restore backup: {e}"));
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
                    &coll.entries,
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
