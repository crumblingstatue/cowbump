mod changes_window;
mod debug_window;
mod entries_window;
mod filter_popup;
mod find_popup;
mod load_folder_window;
mod preferences_window;
mod query_popup;
mod sequences;
mod tag_autocomplete;
mod tag_list;
mod tag_specific_apps_window;
mod top_bar;

use crate::{application::Application, collection::Collection, entry, gui::State, tag};
use egui_sfml::{
    egui::{Context, FontFamily, FontId, TextStyle, Window},
    sfml::graphics::{RenderTarget, RenderWindow, Texture},
};

use self::{
    changes_window::ChangesWindow,
    debug_window::DebugWindow,
    entries_window::EntriesWindow,
    load_folder_window::LoadFolderWindow,
    preferences_window::PreferencesWindow,
    query_popup::QueryPopup,
    sequences::{SequenceWindow, SequencesWindow},
    tag_list::TagWindow,
    tag_specific_apps_window::TagSpecificAppsWindow,
};

use super::{get_tex_for_entry, resources::Resources};

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
    info_messages: Vec<InfoMessage>,
    prompts: Vec<Prompt>,
    // We just closed window with esc, ignore the esc press outside of egui
    pub just_closed_window_with_esc: bool,
    pub debug_window: DebugWindow,
    pub find_popup: QueryPopup,
    pub filter_popup: QueryPopup,
    /// Uid counter for egui entities like windows
    egui_uid_counter: u64,
    pub(crate) tag_specific_apps_window: TagSpecificAppsWindow,
}

impl Default for EguiState {
    fn default() -> Self {
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
            info_messages: Default::default(),
            prompts: Default::default(),
            just_closed_window_with_esc: Default::default(),
            debug_window: Default::default(),
            find_popup: Default::default(),
            filter_popup: Default::default(),
            egui_uid_counter: 0,
            tag_specific_apps_window: Default::default(),
        }
    }
}

struct Prompt {
    msg: InfoMessage,
    action: PromptAction,
}

enum PromptAction {
    QuitNoSave,
    DeleteTags(Vec<tag::Id>),
}

fn ok_prompt(ctx: &Context, title: &str, msg: &str) -> bool {
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

fn ok_cancel_prompt(ctx: &Context, title: &str, msg: &str) -> Option<OkCancel> {
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
    FindNext,
    FindPrev,
    SelectAll,
    SelectNone,
    SortByPath,
    SortById,
    OpenEntriesWindow,
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

pub(super) fn do_ui(
    state: &mut State,
    egui_state: &mut EguiState,
    egui_ctx: &Context,
    app: &mut Application,
    res: &Resources,
    win: &RenderWindow,
) -> anyhow::Result<()> {
    top_bar::do_frame(state, egui_state, egui_ctx, app, win)?;
    preferences_window::do_frame(state, egui_state, app, egui_ctx, win);
    load_folder_window::do_frame(state, egui_state, egui_ctx, res, app, win.size().x);
    changes_window::do_frame(state, egui_state, egui_ctx, app, win);
    debug_window::do_frame(egui_state, egui_ctx);
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
        tag_specific_apps_window::do_frame(
            egui_state,
            coll,
            egui_ctx,
            &mut app.database.preferences,
        );
        entries_window::do_frame(
            state,
            egui_state,
            coll,
            egui_ctx,
            win,
            &mut app.database,
            res,
        );
        do_info_messages(egui_state, egui_ctx);
        do_prompts(egui_state, egui_ctx, app);
    }
    Ok(())
}

fn do_info_messages(egui_state: &mut EguiState, egui_ctx: &Context) {
    egui_state
        .info_messages
        .retain_mut(|msg| !ok_prompt(egui_ctx, &msg.title, &msg.message));
}

fn do_prompts(egui_state: &mut EguiState, egui_ctx: &Context, app: &mut Application) {
    egui_state.prompts.retain(|prompt| {
        match ok_cancel_prompt(egui_ctx, &prompt.msg.title, &prompt.msg.message) {
            Some(OkCancel::Ok) => match prompt.action {
                PromptAction::QuitNoSave => {
                    egui_state.action = Some(Action::QuitNoSave);
                    false
                }
                PromptAction::DeleteTags(ref uids) => {
                    app.active_collection.as_mut().unwrap().1.remove_tags(uids);
                    false
                }
            },
            Some(OkCancel::Cancel) => false,
            None => true,
        }
    });
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
        app: &'db mut Application,
    ) -> Self {
        TexSrc {
            state,
            res,
            coll: app.active_collection.as_ref().map(|(_id, col)| col),
        }
    }
}

impl<'state, 'res, 'db> egui_sfml::UserTexSource for TexSrc<'state, 'res, 'db> {
    fn get_texture(&mut self, id: u64) -> (f32, f32, &Texture) {
        let tex = match self.coll {
            Some(coll) => {
                get_tex_for_entry(
                    &self.state.thumbnail_cache,
                    entry::Id(id),
                    coll,
                    &mut self.state.thumbnail_loader,
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
