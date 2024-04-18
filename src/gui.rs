pub mod debug_log;
mod egui_ui;
pub mod native_dialog;
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
        collection::{self, Collection},
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
    let mut state = State::new(window.size().x, &app.database.preferences);
    let mut egui_state = EguiState::default();
    let mut load_anim_rotation = 0.0;
    let mut sf_egui = SfEgui::new(&window);
    egui_ui::set_up_style(sf_egui.context(), &app.database.preferences.style);

    if app.database.preferences.open_last_coll_at_start && !app.database.recent.is_empty() {
        match app.load_last() {
            Ok(changes) => {
                if !changes.empty() {
                    egui_state.changes_window.open(changes);
                }
                let coll = app.active_collection.as_ref().unwrap();
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
                native_dialog::error("Error loading most recent collection", e);
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

        while let Some(event) = window.poll_event() {
            sf_egui.add_event(&event);
            match event {
                Event::Closed => match app.save_active_collection() {
                    Ok(()) => window.close(),
                    Err(e) => native_dialog::error("Failed to save collection", e),
                },
                Event::KeyPressed {
                    code, ctrl, shift, ..
                } => match code {
                    Key::F1 => egui_state.top_bar ^= true,
                    Key::F11 => util::take_and_save_screenshot(&window),
                    Key::F12 if !ctrl && !shift => egui_state.debug_window.toggle(),
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
                        );
                    }
                }
                Activity::Viewer => {
                    if !sf_egui.context().wants_pointer_input() {
                        viewer::handle_event(&mut state, &event, &mut window)
                    }
                }
            }
        }
        egui_state.begin_frame();
        sf_egui.begin_frame();
        let result = egui_ui::do_ui(
            &mut state,
            &mut egui_state,
            sf_egui.context(),
            app,
            &res,
            &window,
        );
        sf_egui.end_frame(&mut window)?;
        if let Err(e) = result {
            native_dialog::error("Error", e);
        }
        let mut coll = app.active_collection.as_mut().map(|(_id, coll)| coll);
        if let Some(action) = &egui_state.action {
            match action {
                Action::Quit => window.close(),
                Action::QuitNoSave => {
                    app.no_save = true;
                    window.close();
                }
                Action::SelectNone => state.sel.current_mut().clear(),
                Action::FindNext => {
                    search_next(&mut state, coll.as_mut().unwrap(), window.size().y)
                }
                Action::FindPrev => {
                    search_prev(&mut state, coll.as_mut().unwrap(), window.size().y)
                }
                Action::SelectAll => select_all(&mut state, coll.as_mut().unwrap()),
                Action::SortByPath => {
                    state.thumbs_view.sort_by = SortBy::Path;
                    state
                        .thumbs_view
                        .update_from_collection(coll.as_ref().unwrap(), &state.filter)
                }
                Action::SortById => {
                    state.thumbs_view.sort_by = SortBy::Id;
                    state
                        .thumbs_view
                        .update_from_collection(coll.as_ref().unwrap(), &state.filter)
                }
                Action::Shuffle => {
                    state.thumbs_view.uids.shuffle(&mut rand::thread_rng());
                }
                Action::OpenEntriesWindow => {
                    egui_state.add_entries_window(state.sel.current_mut().clone())
                }
            }
        }
        window.clear(Color::BLACK);
        match &mut coll {
            Some(db) => match state.activity {
                Activity::Thumbnails => {
                    thumbnails_view::draw_thumbnails(
                        &mut state,
                        &res,
                        &mut window,
                        db,
                        load_anim_rotation,
                        !sf_egui.context().wants_pointer_input(),
                    );
                }
                Activity::Viewer => {
                    viewer::update(&mut state, &window);
                    viewer::draw(&mut state, &mut window, db, &res);
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

pub type SelectionBuf = Vec<entry::Id>;

pub struct SelectionBufs {
    current: usize,
    bufs: Vec<SelectionBuf>,
}

impl SelectionBufs {
    pub fn new() -> Self {
        Self {
            current: 0,
            bufs: vec![Vec::new()],
        }
    }
    pub fn current_mut(&mut self) -> &mut SelectionBuf {
        &mut self.bufs[self.current]
    }
    pub fn for_each_mut(&mut self, mut f: impl FnMut(&mut SelectionBuf)) {
        for buf in &mut self.bufs {
            f(buf);
        }
    }
    pub fn add_buf(&mut self) {
        self.bufs.push(SelectionBuf::new())
    }
}

#[derive(PartialEq, Eq)]
enum Activity {
    Thumbnails,
    Viewer,
}

fn set_active_collection(
    entries_view: &mut ThumbnailsView,
    app: &mut Application,
    id: collection::Id,
    reqs: &Requirements,
    window_width: u32,
) -> anyhow::Result<()> {
    app.save_active_collection()?;
    *entries_view = ThumbnailsView::from_collection(
        window_width,
        Application::active_collection(&mut app.active_collection)
            .as_ref()
            .unwrap()
            .1,
        reqs,
        &app.database.preferences,
    );
    let root = &app.database.collections[&id];
    std::env::set_current_dir(root).context("failed to set directory")
}

fn get_tex_for_entry<'t>(
    thumbnail_cache: &'t ThumbnailCache,
    id: entry::Id,
    coll: &Collection,
    thumbnail_loader: &mut ThumbnailLoader,
    thumb_size: u32,
    res: &'t Resources,
) -> (bool, &'t Texture) {
    let (has_img, texture) = match thumbnail_cache.get(&id) {
        Some(opt_texture) => match *opt_texture {
            Some(ref tex) => (true, tex as &Texture),
            None => (false, &*res.error_texture),
        },
        None => {
            let Some(entry) = &coll.entries.get(&id) else {
                return (false, &*res.error_texture);
            };
            thumbnail_loader.request(&entry.path, thumb_size, id);
            (false, &*res.loading_texture)
        }
    };
    (has_img, texture)
}

impl State {
    fn new(window_width: u32, prefs: &Preferences) -> Self {
        let mut egui_state = EguiState::default();
        egui_state.top_bar = true;
        Self {
            filter: Requirements::default(),
            thumbnail_cache: Default::default(),
            thumbnail_loader: Default::default(),
            search_cursor: 0,
            search_success: false,
            clipboard_ctx: Clipboard::new().unwrap(),
            thumbs_view: ThumbnailsView::new(window_width, prefs),
            find_reqs: Requirements::default(),
            sel: SelectionBufs::new(),
            select_a: None,
            activity: Activity::Thumbnails,
            viewer_state: ViewerState::default(),
        }
    }
    fn wipe_search(&mut self) {
        self.search_cursor = 0;
        self.search_success = false;
        self.thumbs_view.highlight = None;
    }
}
