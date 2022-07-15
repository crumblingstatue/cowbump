pub mod debug_log;
mod egui_ui;
mod entries_view;
pub mod native_dialog;
mod open;
mod thumbnail_loader;
pub mod thumbnails_view;
mod util;
mod viewer;

use self::{
    egui_ui::Action,
    entries_view::{EntriesView, SortBy},
    thumbnail_loader::ThumbnailLoader,
    thumbnails_view::{clamp_bottom, handle_event, search_next, search_prev, select_all},
    viewer::ViewerState,
};
use crate::{
    application::Application,
    collection::{self, Collection},
    db::{EntryMap, TagSet},
    entry,
    filter_reqs::Requirements,
    gui::egui_ui::EguiState,
};
use anyhow::Context as _;
use arboard::Clipboard;
use egui_sfml::{
    sfml::{
        graphics::{
            Color, Font, IntRect, Rect, RectangleShape, RenderTarget, RenderWindow, Shape, Text,
            Texture, Transformable,
        },
        window::{Event, Key, Style, VideoMode},
        SfBox,
    },
    SfEgui,
};

pub fn run(app: &mut Application) -> anyhow::Result<()> {
    let mut window = RenderWindow::new(
        VideoMode::desktop_mode(),
        "Cowbump",
        Style::NONE,
        &Default::default(),
    );
    window.set_vertical_sync_enabled(true);
    window.set_position((0, 0).into());
    let res = Resources::load()?;
    let mut state = State::new(window.size().x);
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
                state.entries_view = EntriesView::from_collection(&coll.1, &state.filter);
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
                state.entries_view.y_offset += scroll_speed;
                if app.active_collection.is_some() {
                    clamp_bottom(&window, &mut state);
                }
            } else if Key::Up.is_pressed() {
                state.entries_view.y_offset -= scroll_speed;
                if state.entries_view.y_offset < 0.0 {
                    state.entries_view.y_offset = 0.0;
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
                Activity::Viewer => viewer::handle_event(&mut state, &event, &mut window),
            }
        }
        egui_state.begin_frame();
        let mut result = Ok(());
        sf_egui.do_frame(|ctx| {
            result = egui_ui::do_ui(&mut state, &mut egui_state, ctx, app, &res, &window);
        });
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
                Action::SelectNone => state.selected_uids.clear(),
                Action::FindNext => {
                    search_next(&mut state, coll.as_mut().unwrap(), window.size().y)
                }
                Action::FindPrev => {
                    search_prev(&mut state, coll.as_mut().unwrap(), window.size().y)
                }
                Action::SelectAll => select_all(&mut state, coll.as_mut().unwrap()),
                Action::SortByPath => {
                    state.entries_view.sort_by = SortBy::Path;
                    state
                        .entries_view
                        .update_from_collection(coll.as_ref().unwrap(), &state.filter)
                }
                Action::SortById => {
                    state.entries_view.sort_by = SortBy::Id;
                    state
                        .entries_view
                        .update_from_collection(coll.as_ref().unwrap(), &state.filter)
                }
                Action::OpenEntriesWindow => {
                    egui_state.add_entries_window(state.selected_uids.clone())
                }
            }
        }
        window.clear(Color::BLACK);
        match &mut coll {
            Some(db) => match state.activity {
                Activity::Thumbnails => {
                    entries_view::draw_thumbnails(
                        &mut state,
                        &res,
                        &mut window,
                        db,
                        load_anim_rotation,
                        !sf_egui.context().wants_pointer_input(),
                    );
                }
                Activity::Viewer => {
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
        if let Some(index) = state.highlight {
            let mut search_highlight = RectangleShape::with_size(
                (state.thumbnail_size as f32, state.thumbnail_size as f32).into(),
            );
            search_highlight.set_fill_color(Color::TRANSPARENT);
            search_highlight.set_outline_color(Color::RED);
            search_highlight.set_outline_thickness(-4.0);
            let (x, y) = state.item_position(index);
            search_highlight.set_position((x as f32, y as f32 - state.entries_view.y_offset));
            window.draw(&search_highlight);
        }
        if let Some(tex) = egui_state.load_folder_window.texture.as_ref() {
            let mut rs = RectangleShape::from_rect(Rect::new(800., 64., 512., 512.));
            rs.set_texture(tex, true);
            rs.set_outline_color(Color::YELLOW);
            rs.set_outline_thickness(4.0);
            window.draw(&rs);
        }
        let mut tex_src = TexSrc {
            state: &mut state,
            res: &res,
            coll: app.active_collection.as_ref().map(|(_id, col)| col),
        };
        sf_egui.draw(&mut window, Some(&mut tex_src));
        window.display();
        load_anim_rotation += 2.0;
    }
    if !app.no_save {
        app.database.save()?;
    }
    Ok(())
}

fn common_tags(ids: &[entry::Id], coll: &Collection) -> TagSet {
    let mut set = TagSet::default();
    for &id in ids {
        for &tagid in &coll.entries[&id].tags {
            set.insert(tagid);
        }
    }
    set
}

type ThumbnailCache = EntryMap<Option<SfBox<Texture>>>;

struct Resources {
    loading_texture: SfBox<Texture>,
    error_texture: SfBox<Texture>,
    sel_begin_texture: SfBox<Texture>,
    font: SfBox<Font>,
}

impl Resources {
    pub fn load() -> anyhow::Result<Self> {
        let mut loading_texture = Texture::new().context("texture create error")?;
        let mut error_texture = Texture::new().context("texture create error")?;
        let mut sel_begin_texture = Texture::new().context("texture create error")?;
        let font = unsafe {
            Font::from_memory(include_bytes!("../Vera.ttf")).context("failed to load font")?
        };
        loading_texture.load_from_memory(include_bytes!("../loading.png"), IntRect::default())?;
        error_texture.load_from_memory(include_bytes!("../error.png"), IntRect::default())?;
        sel_begin_texture
            .load_from_memory(include_bytes!("../select_begin.png"), IntRect::default())?;
        Ok(Self {
            loading_texture,
            error_texture,
            sel_begin_texture,
            font,
        })
    }
}

struct State {
    thumbnails_per_row: u8,
    thumbnail_size: u32,
    filter: Requirements,
    thumbnail_cache: ThumbnailCache,
    thumbnail_loader: ThumbnailLoader,
    find_reqs: Requirements,
    /// The same search can be used to seek multiple entries
    search_cursor: usize,
    search_success: bool,
    highlight: Option<u32>,
    clipboard_ctx: Clipboard,
    entries_view: EntriesView,
    selected_uids: Vec<entry::Id>,
    /// For batch select, this marks the beginning
    select_begin: Option<usize>,
    activity: Activity,
    viewer_state: ViewerState,
}

#[derive(PartialEq, Eq)]
enum Activity {
    Thumbnails,
    Viewer,
}

fn set_active_collection(
    entries_view: &mut EntriesView,
    app: &mut Application,
    id: collection::Id,
    reqs: &Requirements,
) -> anyhow::Result<()> {
    app.save_active_collection()?;
    *entries_view = EntriesView::from_collection(app.active_collection().as_ref().unwrap().1, reqs);
    let root = &app.database.collections[&id];
    std::env::set_current_dir(root).context("failed to set directory")
}

struct TexSrc<'state, 'res, 'db> {
    state: &'state mut State,
    res: &'res Resources,
    coll: Option<&'db Collection>,
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
                    self.state.thumbnail_size,
                    self.res,
                )
                .1
            }
            None => &*self.res.error_texture,
        };
        (tex.size().x as f32, tex.size().y as f32, tex)
    }
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
            let entry = &coll.entries[&id];
            thumbnail_loader.request(&entry.path, thumb_size, id);
            (false, &*res.loading_texture)
        }
    };
    (has_img, texture)
}

impl State {
    fn new(window_width: u32) -> Self {
        let thumbnails_per_row = 5;
        let thumbnail_size = window_width / thumbnails_per_row as u32;
        let mut egui_state = EguiState::default();
        egui_state.top_bar = true;
        Self {
            thumbnails_per_row,
            thumbnail_size,
            filter: Requirements::default(),
            thumbnail_cache: Default::default(),
            thumbnail_loader: Default::default(),
            search_cursor: 0,
            search_success: false,
            highlight: None,
            clipboard_ctx: Clipboard::new().unwrap(),
            entries_view: EntriesView::default(),
            find_reqs: Requirements::default(),
            selected_uids: Default::default(),
            select_begin: None,
            activity: Activity::Thumbnails,
            viewer_state: ViewerState::default(),
        }
    }
    fn wipe_search(&mut self) {
        self.search_cursor = 0;
        self.search_success = false;
        self.highlight = None;
    }
    fn seek_view_to_contain_index(&mut self, index: usize, height: u32) {
        let (_x, y) = self.item_position(index as u32);
        let view_y = &mut self.entries_view.y_offset;
        let thumb_size = self.thumbnail_size as u32;
        if y < (*view_y as u32) {
            let diff = (*view_y as u32) - y;
            *view_y -= diff as f32;
        }
        if y + thumb_size > (*view_y as u32 + height) {
            let diff = (y + thumb_size) - (*view_y as u32 + height);
            *view_y += diff as f32;
        }
    }
    /// Calculate absolute pixel position of an item at `index`
    fn item_position(&self, index: u32) -> (u32, u32) {
        let thumbs_per_row: u32 = self.thumbnails_per_row.into();
        let row = index / thumbs_per_row;
        let pixel_y = row * self.thumbnail_size;
        let col = index % thumbs_per_row;
        let pixel_x = col * self.thumbnail_size;
        (pixel_x, pixel_y)
    }
    fn highlight_and_seek_to_entry(&mut self, id: entry::Id, height: u32) -> bool {
        match self.entries_view.entry_position(id) {
            Some(idx) => {
                self.highlight = Some(idx as u32);
                self.seek_view_to_contain_index(idx, height);
                true
            }
            None => false,
        }
    }
}
