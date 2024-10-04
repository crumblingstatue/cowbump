pub mod debug_log;
mod egui_ui;
mod open;
mod resources;
mod thumbnail_loader;
mod thumbnails_view;
mod util;
mod viewer;

use {
    self::{
        egui_ui::{Action, EguiState},
        resources::Resources,
        thumbnail_loader::ThumbnailLoader,
        thumbnails_view::{
            handle_event, search_next, search_prev, select_all, SortBy, ThumbnailsView,
        },
        viewer::ViewerState,
    },
    crate::{
        application::Application,
        collection::{self, Entries},
        db::EntryMap,
        entry,
        filter_reqs::Requirements,
        preferences::Preferences,
    },
    anyhow::Context as _,
    arboard::Clipboard,
    egui_sfml::{
        sfml::{
            graphics::{
                Color, Rect, RectangleShape, RenderTarget, RenderWindow, Shape, Text, Texture,
                Transformable, View,
            },
            window::{Event, Key, Style, VideoMode},
            SfBox,
        },
        SfEgui,
    },
    rand::seq::SliceRandom,
    thumbnails_view::EventFlags,
};

pub fn run(app: &mut Application) -> anyhow::Result<()> {
    let (video, style) = if app.database.preferences.start_fullscreen {
        (VideoMode::desktop_mode(), Style::NONE)
    } else {
        (VideoMode::new(1280, 720, 32), Style::RESIZE)
    };
    let mut window = RenderWindow::new(video, "Cowbump", style, &Default::default());
    window.set_vertical_sync_enabled(true);
    window.set_position((0, 0).into());
    let res = Resources::load()?;
    let mut state = State::new(window.size().x, &app.database.preferences)?;
    let mut load_anim_rotation = 0.0;
    let mut sf_egui = SfEgui::new(&window);
    egui_ui::set_up_style(sf_egui.context(), &app.database.preferences.style);
    let mut egui_state = EguiState::new();

    if app.database.preferences.open_last_coll_at_start && !app.database.recent.is_empty() {
        match app.load_last() {
            Ok(changes) => {
                if !changes.empty() {
                    egui_state.changes_window.open(changes);
                }
                let coll = app
                    .active_collection
                    .as_ref()
                    .context("Can't get active collection")?;
                state.thumbs_view = ThumbnailsView::from_collection(
                    window.size().x,
                    &coll.1,
                    &state.filter,
                    &app.database.preferences,
                );
                let root_path = &app.database.collections[&coll.0];
                std::env::set_current_dir(root_path)?;
            }
            Err(e) => {
                egui_state
                    .modal
                    .err(format!("Error loading most recent collection: {e:?}"));
            }
        }
    }

    while window.is_open() {
        if !sf_egui.context().wants_keyboard_input() {
            let scroll_speed = app.database.preferences.arrow_key_scroll_speed;
            if Key::Down.is_pressed() {
                state.thumbs_view.y_offset += scroll_speed;
                if app.active_collection.is_some() {
                    state.thumbs_view.clamp_bottom(&window);
                }
            } else if Key::Up.is_pressed() {
                state.thumbs_view.y_offset -= scroll_speed;
                if state.thumbs_view.y_offset < 0.0 {
                    state.thumbs_view.y_offset = 0.0;
                }
            }
        }
        let mut ev_flags = EventFlags::default();
        while let Some(event) = window.poll_event() {
            sf_egui.add_event(&event);
            match event {
                Event::Closed => window.close(),
                Event::KeyPressed {
                    code, ctrl, shift, ..
                } => match code {
                    Key::F1 => egui_state.top_bar.toggle(),
                    Key::F11 => util::take_and_save_screenshot(&window, &mut egui_state),
                    Key::F12 if !ctrl && !shift => egui_state.debug_window.toggle(),
                    Key::Q if ctrl => window.close(),
                    _ => {}
                },
                Event::Resized { width, height } => {
                    window.set_view(&View::from_rect(Rect::new(
                        0.,
                        0.,
                        width as f32,
                        height as f32,
                    )));
                    state.thumbs_view.resize(width, &app.database.preferences);
                }
                _ => {}
            }
            match state.activity {
                Activity::Thumbnails => {
                    if let Some((_id, coll)) = &mut app.active_collection {
                        handle_event(
                            event,
                            &mut state,
                            &mut egui_state,
                            coll,
                            &window,
                            sf_egui.context(),
                            &mut app.database.preferences,
                            &mut ev_flags,
                        );
                    }
                }
                Activity::Viewer => {
                    if !sf_egui.context().wants_pointer_input() {
                        viewer::handle_event(&mut state, &event, &window);
                    }
                }
            }
        }
        egui_state.begin_frame();
        let mut result = Ok(());
        sf_egui.run(&mut window, |rw, ctx| {
            result = egui_ui::do_ui(&mut state, &mut egui_state, ctx, app, &res, rw);
        })?;
        if let Err(e) = result {
            // Note: These are not egui errors. Egui is doing fine when these errors
            // happen, so we can use the egui modal dialog to display them.
            egui_state.modal.err(format!("{e:?}"));
        }
        if let Some(action) = &egui_state.action {
            match action {
                Action::Quit => window.close(),
                Action::QuitNoSave => {
                    app.no_save = true;
                    window.close();
                }
                Action::SelectNone => state.sel.clear_current(),
                Action::FindNext => {
                    if let Some((_, coll)) = &mut app.active_collection {
                        search_next(&mut state, coll, window.size().y);
                    }
                }
                Action::FindPrev => {
                    if let Some((_, coll)) = &mut app.active_collection {
                        search_prev(&mut state, coll, window.size().y);
                    }
                }
                Action::SelectAll => {
                    if let Some((_, coll)) = &mut app.active_collection {
                        select_all(&mut state, coll);
                    }
                }
                Action::SortByPath => {
                    state.thumbs_view.sort_by = SortBy::Path;
                    if let Some((_, coll)) = &mut app.active_collection {
                        state
                            .thumbs_view
                            .update_from_collection(coll, &state.filter);
                    }
                }
                Action::SortById => {
                    state.thumbs_view.sort_by = SortBy::Id;
                    if let Some((_, coll)) = &mut app.active_collection {
                        state
                            .thumbs_view
                            .update_from_collection(coll, &state.filter);
                    }
                }
                Action::Shuffle => {
                    state.thumbs_view.uids.shuffle(&mut rand::thread_rng());
                }
                Action::OpenEntriesWindow => {
                    let id_vec = state
                        .sel
                        .current_as_nonempty_id_vec()
                        .context("Selection buffer inaccessible")?;
                    egui_state.add_entries_window(id_vec.clone());
                }
            }
        }
        // Do some post-egui event handling here.
        //
        // This needs to happen after the egui pass(es), because it needs to react to what
        // happened in the egui ui, like if a window was just closed.
        if ev_flags.esc_pressed
            && !sf_egui.context().wants_keyboard_input()
            && !sf_egui.context().wants_pointer_input()
            && !egui_state.just_closed_window_with_esc
        {
            state.sel.clear_current();
        }
        window.clear(Color::BLACK);
        match &mut app.active_collection {
            Some((_, coll)) => match state.activity {
                Activity::Thumbnails => {
                    thumbnails_view::draw_thumbnails(
                        &mut state,
                        &res,
                        &mut window,
                        &coll.entries,
                        load_anim_rotation,
                        !sf_egui.context().wants_pointer_input(),
                    );
                }
                Activity::Viewer => {
                    viewer::update(&mut state, &window);
                    viewer::draw(&mut state, &mut window, coll, &res);
                }
            },
            None => {
                let msg = "Welcome to cowbump!\n\
                \n\
                To start, load a folder with File->Load folder\n\
                You can also pick from the recently used list, if you had opened something before\n\
                \n\
                If you don't see the top menu, you can toggle it with F1";
                let mut text = Text::new(msg, &res.font, 24);
                text.set_position((16., 64.));
                window.draw(&text);
            }
        }
        if let Some(index) = state.thumbs_view.highlight {
            let mut search_highlight = RectangleShape::with_size(
                (
                    state.thumbs_view.thumb_size as f32,
                    state.thumbs_view.thumb_size as f32,
                )
                    .into(),
            );
            search_highlight.set_fill_color(Color::TRANSPARENT);
            search_highlight.set_outline_color(Color::RED);
            search_highlight.set_outline_thickness(-4.0);
            let (x, y) = state.thumbs_view.item_position(index);
            search_highlight.set_position((x as f32, y as f32 - state.thumbs_view.y_offset));
            window.draw(&search_highlight);
        }
        if let Some(tex) = egui_state.load_folder_window.texture.as_ref() {
            let mut rs = RectangleShape::from_rect(Rect::new(800., 64., 512., 512.));
            rs.set_texture(tex, true);
            rs.set_outline_color(Color::YELLOW);
            rs.set_outline_thickness(4.0);
            window.draw(&rs);
        }
        let mut tex_src = egui_ui::TexSrc::new(&mut state, &res, app);
        sf_egui.draw(&mut window, Some(&mut tex_src));
        window.display();
        load_anim_rotation += 2.0;
    }
    if !app.no_save {
        app.save_active_collection()?;
        app.database.save()?;
    }
    Ok(())
}

type ThumbnailCache = EntryMap<Option<SfBox<Texture>>>;

struct State {
    filter: Requirements,
    thumbnail_cache: ThumbnailCache,
    thumbnail_loader: ThumbnailLoader,
    find_reqs: Requirements,
    /// The same search can be used to seek multiple entries
    search_cursor: usize,
    search_success: bool,
    clipboard_ctx: Clipboard,
    thumbs_view: ThumbnailsView,
    sel: SelectionBufs,
    /// For batch select, this marks the "a" point
    select_a: Option<usize>,
    activity: Activity,
    viewer_state: ViewerState,
}
pub struct SelectionBuf {
    pub buf: Vec<entry::Id>,
    pub name: String,
}

impl SelectionBuf {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            buf: Vec::new(),
            name: name.into(),
        }
    }
    pub fn clear(&mut self) {
        self.buf.clear();
    }
    pub fn extend(&mut self, iter: impl IntoIterator<Item = entry::Id>) {
        self.buf.extend(iter);
    }
    pub fn as_vec(&self) -> &Vec<entry::Id> {
        &self.buf
    }
    pub fn remove(&mut self, idx: usize) {
        self.buf.remove(idx);
    }
    pub fn len(&self) -> usize {
        self.buf.len()
    }
    pub fn contains(&self, id: &entry::Id) -> bool {
        self.buf.contains(id)
    }
}

pub struct SelectionBufs {
    current: usize,
    bufs: Vec<SelectionBuf>,
}

impl SelectionBufs {
    pub fn new() -> Self {
        Self {
            current: 0,
            bufs: vec![SelectionBuf::new("Sel 1")],
        }
    }
    pub fn current(&self) -> Option<&SelectionBuf> {
        self.bufs.get(self.current)
    }
    pub fn current_mut(&mut self) -> Option<&mut SelectionBuf> {
        self.bufs.get_mut(self.current)
    }
    pub fn for_each_mut(&mut self, mut f: impl FnMut(&mut SelectionBuf)) {
        for buf in &mut self.bufs {
            f(buf);
        }
    }
    pub fn add_buf(&mut self, name: impl Into<String>) {
        self.bufs.push(SelectionBuf::new(name));
    }
    fn n_selected(&self) -> usize {
        self.current().map_or(0, SelectionBuf::len)
    }
    fn none_selected(&self) -> bool {
        self.n_selected() == 0
    }
    /// Returns the currently active selected-ids vec, if any, if not empty
    fn current_as_nonempty_id_vec(&self) -> Option<&Vec<entry::Id>> {
        self.current()
            .filter(|buf| !buf.buf.is_empty())
            .map(SelectionBuf::as_vec)
    }
    fn selected_ids_iter(&self) -> impl Iterator<Item = &entry::Id> {
        match self.current() {
            Some(buf) => buf.buf.iter(),
            None => [].iter(),
        }
    }
    fn current_contains(&self, id: &entry::Id) -> bool {
        self.current().map_or(false, |buf| buf.contains(id))
    }

    fn clear_current(&mut self) {
        if let Some(current) = self.current_mut() {
            current.clear();
        }
    }
}

#[derive(PartialEq, Eq)]
enum Activity {
    Thumbnails,
    Viewer,
}

fn set_active_collection(
    entries_view: &mut ThumbnailsView,
    app: &Application,
    id: collection::Id,
    reqs: &Requirements,
    window_width: u32,
) -> anyhow::Result<()> {
    app.save_active_collection()?;
    let active_coll = &app
        .active_collection
        .as_ref()
        .context("No active collection")?
        .1;
    *entries_view =
        ThumbnailsView::from_collection(window_width, active_coll, reqs, &app.database.preferences);
    let root = app
        .database
        .collections
        .get(&id)
        .context("dangling collection id")?;
    std::env::set_current_dir(root).context("failed to set directory")
}

fn get_tex_for_entry<'t>(
    thumbnail_cache: &'t ThumbnailCache,
    id: entry::Id,
    entries: &Entries,
    thumbnail_loader: &ThumbnailLoader,
    thumb_size: u32,
    res: &'t Resources,
) -> (bool, &'t Texture) {
    let (has_img, texture) = match thumbnail_cache.get(&id) {
        Some(opt_texture) => match *opt_texture {
            Some(ref tex) => (true, &**tex),
            None => (false, &*res.error_texture),
        },
        None => {
            let Some(entry) = entries.get(&id) else {
                return (false, &*res.error_texture);
            };
            thumbnail_loader.request(&entry.path, thumb_size, id);
            (false, &*res.loading_texture)
        }
    };
    (has_img, texture)
}

impl State {
    fn new(window_width: u32, prefs: &Preferences) -> anyhow::Result<Self> {
        Ok(Self {
            filter: Requirements::default(),
            thumbnail_cache: Default::default(),
            thumbnail_loader: Default::default(),
            search_cursor: 0,
            search_success: false,
            clipboard_ctx: Clipboard::new()?,
            thumbs_view: ThumbnailsView::new(window_width, prefs),
            find_reqs: Requirements::default(),
            sel: SelectionBufs::new(),
            select_a: None,
            activity: Activity::Thumbnails,
            viewer_state: ViewerState::default(),
        })
    }
    fn wipe_search(&mut self) {
        self.search_cursor = 0;
        self.search_success = false;
        self.thumbs_view.highlight = None;
    }
}
