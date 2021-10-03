mod egui_ui;
mod entries_view;
pub mod native_dialog;
mod thumbnail_loader;
mod util;

use self::{egui_ui::Action, entries_view::EntriesView, thumbnail_loader::ThumbnailLoader};
use crate::{
    application::Application,
    collection::{self, Collection},
    db::{EntryMap, TagSet},
    entry,
    filter_spec::FilterSpec,
    gui::egui_ui::EguiState,
    preferences::{AppId, Preferences},
};
use anyhow::{bail, Context};
use arboard::Clipboard;
use egui::{CtxRef, FontDefinitions, FontFamily, TextStyle};
use egui_sfml::SfEgui;
use sfml::{
    graphics::{
        Color, Font, IntRect, Rect, RectangleShape, RenderTarget, RenderWindow, Shape, Text,
        Texture, Transformable,
    },
    window::{mouse, Event, Key, Style, VideoMode},
    SfBox,
};
use std::{cell::RefCell, collections::BTreeMap, path::Path, process::Command};

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
    let mut on_screen_uids: Vec<entry::Id> = Vec::new();
    let mut load_anim_rotation = 0.0;
    let mut sf_egui = SfEgui::new(&window);
    let egui_ctx = sf_egui.context();
    let font_defs = FontDefinitions {
        family_and_size: BTreeMap::from([
            (TextStyle::Small, (FontFamily::Proportional, 12.0)),
            (TextStyle::Body, (FontFamily::Proportional, 18.0)),
            (TextStyle::Button, (FontFamily::Proportional, 18.0)),
            (TextStyle::Heading, (FontFamily::Proportional, 20.0)),
            (TextStyle::Monospace, (FontFamily::Monospace, 13.0)),
        ]),
        ..Default::default()
    };

    if app.database.preferences.open_last_coll_at_start && app.database.recent.len() > 0 {
        match app.load_last() {
            Ok(changes) => {
                if !changes.empty() {
                    egui_state.changes_window.open(changes);
                }
                let coll = app.active_collection.as_ref().unwrap();
                state.entries_view = EntriesView::from_collection(&coll.1);
                let root_path = &app.database.collections[&coll.0];
                std::env::set_current_dir(root_path)?;
            }
            Err(e) => {
                native_dialog::error("Error loading most recent collection", e);
            }
        }
    }

    egui_ctx.set_fonts(font_defs);
    while window.is_open() {
        if !sf_egui.context().wants_keyboard_input() {
            let scroll_speed = app.database.preferences.arrow_key_scroll_speed;
            if Key::Down.is_pressed() {
                state.entries_view.y_offset += scroll_speed;
                if let Some((_id, coll)) = &app.active_collection {
                    clamp_bottom(&window, &mut state, coll);
                }
            } else if Key::Up.is_pressed() {
                state.entries_view.y_offset -= scroll_speed;
                if state.entries_view.y_offset < 0.0 {
                    state.entries_view.y_offset = 0.0;
                }
            }
        }
        let mut esc_pressed = false;

        while let Some(event) = window.poll_event() {
            sf_egui.add_event(&event);
            match event {
                Event::Closed => match app.save_active_collection() {
                    Ok(()) => window.close(),
                    Err(e) => native_dialog::error("Failed to save collection", e),
                },
                Event::KeyPressed { code, .. } => {
                    match code {
                        Key::Escape => esc_pressed = true,
                        Key::Home => {
                            if !sf_egui.context().wants_keyboard_input() {
                                state.entries_view.y_offset = 0.0;
                            }
                        }
                        Key::End => {
                            if let Some((_id, coll)) = &mut app.active_collection {
                                if !sf_egui.context().wants_keyboard_input() {
                                    // Align the bottom edge of the view with the bottom edge of the last row.
                                    // To do align the camera with a bottom edge, we need to subtract the screen
                                    // height from it.
                                    go_to_bottom(&window, &mut state, coll);
                                }
                            }
                        }
                        Key::F1 => egui_state.top_bar ^= true,
                        Key::F2 => {
                            if !state.selected_uids.is_empty() {
                                egui_state.add_entries_window(state.selected_uids.clone())
                            }
                        }
                        Key::F11 => util::take_and_save_screenshot(&window),
                        Key::F12 => egui_state.debug_window.toggle(),
                        _ => {}
                    }
                }
                _ => {}
            }
            if let Some((_id, coll)) = &mut app.active_collection {
                handle_event_viewer(
                    event,
                    &mut state,
                    &mut egui_state,
                    &mut on_screen_uids,
                    coll,
                    &window,
                    sf_egui.context(),
                    &mut app.database.preferences,
                );
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
        if esc_pressed
            && !sf_egui.context().wants_keyboard_input()
            && !sf_egui.context().wants_pointer_input()
            && !egui_state.just_closed_window_with_esc
        {
            state.selected_uids.clear()
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
                Action::SearchNext => {
                    search_next(&mut state, coll.as_mut().unwrap(), window.size().y)
                }
                Action::SearchPrev => {
                    search_prev(&mut state, coll.as_mut().unwrap(), window.size().y)
                }
                Action::SelectAll => select_all(&mut state, coll.as_mut().unwrap()),
                Action::SortEntries => state.entries_view.sort(coll.as_mut().unwrap()),
                Action::OpenEntriesWindow => {
                    egui_state.add_entries_window(state.selected_uids.clone())
                }
            }
        }
        if let Some(coll) = &mut coll {
            recalc_on_screen_items(
                &mut on_screen_uids,
                coll,
                &state.entries_view,
                &state,
                window.size().y,
            );
        }
        window.clear(Color::BLACK);
        match &mut coll {
            Some(db) => {
                entries_view::draw_thumbnails(
                    &mut state,
                    &res,
                    &mut window,
                    db,
                    &on_screen_uids,
                    load_anim_rotation,
                    !sf_egui.context().wants_pointer_input(),
                );
            }
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

fn go_to_bottom(window: &RenderWindow, state: &mut State, coll: &Collection) {
    state.entries_view.y_offset = find_bottom(state, coll, window);
}

fn clamp_bottom(window: &RenderWindow, state: &mut State, coll: &Collection) {
    let bottom = find_bottom(state, coll, window);
    if state.entries_view.y_offset > bottom {
        state.entries_view.y_offset = bottom;
    }
}

fn find_bottom(state: &State, coll: &Collection, window: &RenderWindow) -> f32 {
    let n_pics = state.entries_view.filter(coll, &state.filter).count();
    let mut rows = n_pics as u32 / state.thumbnails_per_row as u32;
    if n_pics as u32 % state.thumbnails_per_row as u32 != 0 {
        rows += 1;
    }
    let bottom = rows * state.thumbnail_size;
    let mut b = bottom as f32 - window.size().y as f32;
    if b < 0. {
        b = 0.;
    }
    b
}

fn common_tags(ids: &[entry::Id], db: &Collection) -> TagSet {
    let mut set = TagSet::default();
    for &id in ids {
        for &tagid in &db.entries[&id].tags {
            set.insert(tagid);
        }
    }
    set
}

fn entry_at_xy(
    x: i32,
    y: i32,
    state: &State,
    on_screen_entries: &[entry::Id],
) -> Option<entry::Id> {
    let thumb_x = x as u32 / state.thumbnail_size;
    let rel_offset = state.entries_view.y_offset as u32 % state.thumbnail_size;
    let thumb_y = (y as u32 + rel_offset) / state.thumbnail_size;
    let thumb_index = thumb_y * state.thumbnails_per_row as u32 + thumb_x;
    on_screen_entries.get(thumb_index as usize).copied()
}

fn handle_event_viewer(
    event: Event,
    state: &mut State,
    egui_state: &mut EguiState,
    on_screen_entries: &mut Vec<entry::Id>,
    coll: &mut Collection,
    window: &RenderWindow,
    ctx: &CtxRef,
    preferences: &mut Preferences,
) {
    match event {
        Event::MouseButtonPressed { button, x, y } => {
            if ctx.wants_pointer_input() {
                return;
            }
            let uid = match entry_at_xy(x, y, state, on_screen_entries) {
                Some(uid) => uid,
                None => return,
            };
            if button == mouse::Button::Left {
                if Key::LShift.is_pressed() {
                    if state.selected_uids.contains(&uid) {
                        state.selected_uids.retain(|&rhs| rhs != uid);
                    } else {
                        state.selected_uids.push(uid);
                    }
                } else if let Err(e) = open_with_external(&[&coll.entries[&uid].path], preferences)
                {
                    native_dialog::error("Failed to open file", e);
                }
            } else if button == mouse::Button::Right {
                let vec = if state.selected_uids.contains(&uid) {
                    state.selected_uids.clone()
                } else {
                    vec![uid]
                };
                egui_state.add_entries_window(vec);
            }
        }
        Event::KeyPressed { code, ctrl, .. } => {
            if ctx.wants_keyboard_input() {
                return;
            }
            if code == Key::PageDown {
                state.entries_view.y_offset += window.size().y as f32;
                clamp_bottom(window, state, coll);
            } else if code == Key::PageUp {
                state.entries_view.y_offset -= window.size().y as f32;
                clamp_top(state);
            } else if code == Key::Enter {
                let mut paths: Vec<&Path> = Vec::new();
                for &uid in state.selected_uids.iter() {
                    paths.push(&coll.entries[&uid].path);
                }
                if paths.is_empty() && state.filter.active() {
                    for uid in coll.filter(&state.filter) {
                        paths.push(&coll.entries[&uid].path);
                    }
                }
                paths.sort();
                if let Err(e) = open_with_external(&paths, preferences) {
                    native_dialog::error("Failed to open file", e);
                }
            } else if code == Key::A && ctrl {
                select_all(state, coll);
            } else if code == Key::Slash {
                state.search_edit = true;
            } else if code == Key::N {
                search_next(state, coll, window.size().y);
            } else if code == Key::P {
                search_prev(state, coll, window.size().y);
            } else if code == Key::F {
                state.filter_edit = true;
            } else if code == Key::C {
                let mp = window.mouse_position();
                let uid = match entry_at_xy(mp.x, mp.y, state, on_screen_entries) {
                    Some(uid) => uid,
                    None => return,
                };
                if let Err(e) = copy_image_to_clipboard(state, &coll, uid) {
                    native_dialog::error("Clipboard copy failed", e);
                }
            } else if code == Key::T {
                egui_state.tag_window.toggle();
            } else if code == Key::Q {
                egui_state.sequences_window.on ^= true;
            } else if code == Key::S {
                state.entries_view.sort(coll);
            }
        }
        Event::MouseWheelScrolled { delta, .. } => {
            state.entries_view.y_offset -= delta * preferences.scroll_wheel_multiplier;
            if delta > 0.0 {
                clamp_top(state);
            } else {
                clamp_bottom(window, state, coll);
            }
        }
        _ => {}
    }
}

fn clamp_top(state: &mut State) {
    if state.entries_view.y_offset < 0.0 {
        state.entries_view.y_offset = 0.0;
    }
}

fn open_with_external(paths: &[&Path], preferences: &mut Preferences) -> anyhow::Result<()> {
    let built_tasks = build_tasks(paths, preferences)?;
    for task in built_tasks.tasks {
        let app = &preferences.applications[&task.app];
        let mut cmd = Command::new(&app.path);
        cmd.args(task.args);
        cmd.spawn()?;
    }
    if built_tasks.remainder.len() >= 5 {
        let msg = "\
        You are trying to open too many unassociated files. This is unsupported.\n\
        See File->Preferences->Associations for app associations.";
        bail!(msg);
    }
    for path in built_tasks.remainder {
        open::that_in_background(path);
    }
    Ok(())
}

struct BuiltTasks<'p> {
    tasks: Vec<Task<'p>>,
    remainder: Vec<&'p Path>,
}

fn build_tasks<'a, 'p>(
    paths: &[&'p Path],
    preferences: &'a mut Preferences,
) -> anyhow::Result<BuiltTasks<'p>> {
    let mut tasks: Vec<Task> = Vec::new();
    let mut remainder = Vec::new();
    for path in paths {
        let ext = path
            .extension()
            .map(|ext| ext.to_str().unwrap())
            .unwrap_or("");
        match preferences.associations.get(ext) {
            Some(Some(app_id)) => {
                if let Some(task) = tasks.iter_mut().find(|task| task.app == *app_id) {
                    task.args.push(path);
                } else {
                    tasks.push(Task {
                        app: *app_id,
                        args: vec![path],
                    });
                }
            }
            _ => {
                // Make sure extension preference exists, so the user doesn't
                // have to add it manually to the list.
                preferences.associations.insert(ext.to_owned(), None);
                remainder.push(*path);
            }
        }
    }
    Ok(BuiltTasks { tasks, remainder })
}

#[derive(Debug)]
struct Task<'p> {
    app: AppId,
    args: Vec<&'p Path>,
}

fn copy_image_to_clipboard(
    state: &mut State,
    db: &&mut Collection,
    uid: entry::Id,
) -> anyhow::Result<()> {
    use arboard::ImageData;
    let imgpath = &db.entries[&uid].path;
    let buf = std::fs::read(imgpath).unwrap();
    let img = image::load_from_memory(&buf).context("Failed to load image from memory")?;
    let rgba = img.to_rgba8();
    let img_data = ImageData {
        width: rgba.width() as usize,
        height: rgba.height() as usize,
        bytes: rgba.into_raw().into(),
    };
    state
        .clipboard_ctx
        .set_image(img_data)
        .context("Failed to copy to clipboard")
}

fn select_all(state: &mut State, coll: &Collection) {
    state.selected_uids.clear();
    for uid in coll.filter(&state.filter) {
        state.selected_uids.push(uid);
    }
}

fn search_prev(state: &mut State, coll: &mut Collection, view_height: u32) {
    if state.search_cursor > 0 {
        state.search_cursor -= 1;
    }
    search_goto_cursor(state, coll, view_height);
}

fn search_next(state: &mut State, coll: &mut Collection, view_height: u32) {
    state.search_cursor += 1;
    search_goto_cursor(state, coll, view_height);
    if !state.search_success {
        state.search_cursor -= 1;
    }
}

fn find_nth(state: &State, coll: &Collection, nth: usize) -> Option<usize> {
    state
        .entries_view
        .filter(coll, &state.filter)
        .enumerate()
        .filter(|(_, uid)| {
            let en = &coll.entries[uid];
            en.spec_satisfied(&state.search_spec)
        })
        .map(|(i, _)| i)
        .nth(nth)
}

fn search_goto_cursor(state: &mut State, db: &Collection, view_height: u32) {
    if let Some(index) = find_nth(state, db, state.search_cursor) {
        state.highlight = Some(index as u32);
        state.search_success = true;
        state.seek_view_to_contain_index(index, view_height);
    } else {
        state.search_success = false;
    }
}

fn recalc_on_screen_items(
    uids: &mut Vec<entry::Id>,
    db: &Collection,
    entries_view: &EntriesView,
    state: &State,
    window_height: u32,
) {
    uids.clear();
    let thumb_size = state.thumbnail_size;
    let mut thumbnails_per_column = (window_height / thumb_size) as u8;
    // Compensate for truncating division
    if window_height % thumb_size != 0 {
        thumbnails_per_column += 1;
    }
    // Since we can scroll, we can have another partially drawn frame per screen
    thumbnails_per_column += 1;
    let thumbnails_per_screen = (state.thumbnails_per_row * thumbnails_per_column) as usize;
    let row_offset = state.entries_view.y_offset as u32 / thumb_size;
    let skip = row_offset * state.thumbnails_per_row as u32;
    uids.extend(
        entries_view
            .filter(db, &state.filter)
            .skip(skip as usize)
            .take(thumbnails_per_screen),
    );
}

type ThumbnailCache = EntryMap<Option<SfBox<Texture>>>;

struct Resources {
    loading_texture: SfBox<Texture>,
    error_texture: SfBox<Texture>,
    font: SfBox<Font>,
}

impl Resources {
    pub fn load() -> anyhow::Result<Self> {
        let mut loading_texture = Texture::new().context("failed to load loading texture")?;
        let mut error_texture = Texture::new().context("failed to load error texture")?;
        let font =
            Font::from_memory(include_bytes!("../Vera.ttf")).context("failed to load font")?;
        loading_texture.load_from_memory(include_bytes!("../loading.png"), IntRect::default())?;
        error_texture.load_from_memory(include_bytes!("../error.png"), IntRect::default())?;
        Ok(Self {
            loading_texture,
            error_texture,
            font,
        })
    }
}

struct State {
    thumbnails_per_row: u8,
    thumbnail_size: u32,
    filter: FilterSpec,
    thumbnail_cache: ThumbnailCache,
    thumbnail_loader: ThumbnailLoader,
    search_edit: bool,
    search_string: String,
    search_spec: FilterSpec,
    /// The same search can be used to seek multiple entries
    search_cursor: usize,
    search_success: bool,
    highlight: Option<u32>,
    filter_edit: bool,
    filter_string: String,
    clipboard_ctx: Clipboard,
    entries_view: EntriesView,
    debug_log: RefCell<Vec<String>>,
    selected_uids: Vec<entry::Id>,
}

fn set_active_collection(
    entries_view: &mut EntriesView,
    app: &mut Application,
    id: collection::Id,
) -> anyhow::Result<()> {
    app.save_active_collection()?;
    *entries_view = EntriesView::from_collection(app.active_collection().as_ref().unwrap().1);
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
        let (_has, tex) = get_tex_for_entry(
            &self.state.thumbnail_cache,
            entry::Id(id),
            &self.res.error_texture,
            self.coll,
            &mut self.state.thumbnail_loader,
            self.state.thumbnail_size,
            &self.res.loading_texture,
        );
        (tex.size().x as f32, tex.size().y as f32, tex)
    }
}

fn get_tex_for_entry<'t>(
    thumbnail_cache: &'t ThumbnailCache,
    id: entry::Id,
    error_texture: &'t Texture,
    db: Option<&Collection>,
    thumbnail_loader: &mut ThumbnailLoader,
    thumb_size: u32,
    loading_texture: &'t Texture,
) -> (bool, &'t Texture) {
    let (has_img, texture) = match thumbnail_cache.get(&id) {
        Some(opt_texture) => match *opt_texture {
            Some(ref tex) => (true, tex as &Texture),
            None => (false, error_texture),
        },
        None => match db {
            Some(db) => {
                let entry = &db.entries[&id];
                thumbnail_loader.request(&entry.path, thumb_size, id);
                (false, loading_texture)
            }
            None => (false, error_texture),
        },
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
            filter: FilterSpec::default(),
            thumbnail_cache: Default::default(),
            thumbnail_loader: Default::default(),
            search_edit: false,
            search_string: String::new(),
            search_cursor: 0,
            search_success: false,
            highlight: None,
            filter_edit: false,
            filter_string: String::new(),
            clipboard_ctx: Clipboard::new().unwrap(),
            entries_view: EntriesView::default(),
            search_spec: FilterSpec::default(),
            debug_log: Default::default(),
            selected_uids: Default::default(),
        }
    }
    fn wipe_search(&mut self) {
        self.search_cursor = 0;
        self.search_edit = false;
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
    fn highlight_and_seek_to_entry(
        &mut self,
        id: entry::Id,
        height: u32,
        coll: &Collection,
    ) -> bool {
        match self.entries_view.entry_position(id, coll, &self.filter) {
            Some(idx) => {
                self.highlight = Some(idx as u32);
                self.seek_view_to_contain_index(idx, height);
                true
            }
            None => false,
        }
    }
}
