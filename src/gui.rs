mod egui_ui;
mod entries_view;
mod thumbnail_loader;

use crate::{
    application::Application,
    collection::{self, Collection},
    db::{EntryMap, TagSet, Uid},
    entry,
    filter_spec::FilterSpec,
    gui::egui_ui::EguiState,
};
use std::collections::BTreeMap;

use self::{egui_ui::Action, entries_view::EntriesView, thumbnail_loader::ThumbnailLoader};
use anyhow::Context;
use arboard::Clipboard;
use egui::{CtxRef, FontDefinitions, FontFamily, TextStyle};
use egui_sfml::SfEgui;
use rfd::{MessageDialog, MessageLevel};
use sfml::{
    graphics::{
        Color, Font, IntRect, Rect, RectangleShape, RenderTarget, RenderWindow, Shape, Sprite,
        Text, Texture, Transformable,
    },
    window::{mouse, Event, Key, Style, VideoMode},
    SfBox,
};
use std::path::Path;

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
    let mut on_screen_uids: Vec<entry::Id> = Vec::new();
    let mut selected_uids: Vec<entry::Id> = Default::default();
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
        app.load_last()?;
        let coll = &app.database.collections[&app.active_collection.unwrap()];
        state.entries_view = EntriesView::from_collection(coll);
        std::env::set_current_dir(&coll.root_path)?;
    }

    egui_ctx.set_fonts(font_defs);
    while window.is_open() {
        if !sf_egui.context().wants_keyboard_input() {
            let scroll_speed = 8.0;
            if Key::DOWN.is_pressed() {
                state.y_offset += scroll_speed;
            } else if Key::UP.is_pressed() {
                state.y_offset -= scroll_speed;
                if state.y_offset < 0.0 {
                    state.y_offset = 0.0;
                }
            }
        }
        let mut esc_pressed = false;

        while let Some(event) = window.poll_event() {
            sf_egui.add_event(&event);
            match event {
                Event::Closed => window.close(),
                Event::KeyPressed { code, .. } => {
                    match code {
                        Key::ESCAPE => esc_pressed = true,
                        Key::HOME => {
                            if !sf_egui.context().wants_keyboard_input() {
                                state.y_offset = 0.0;
                            }
                        }
                        Key::END => {
                            if let Some(coll) = app
                                .active_collection
                                .map(|idx| &app.database.collections[&idx])
                            {
                                if !sf_egui.context().wants_keyboard_input() {
                                    // Align the bottom edge of the view with the bottom edge of the last row.
                                    // To do align the camera with a bottom edge, we need to subtract the screen
                                    // height from it.
                                    let bottom_align = |y: f32| y - window.size().y as f32;
                                    let n_pics =
                                        state.entries_view.filter(coll, &state.filter).count();
                                    let rows = n_pics as u32 / state.thumbnails_per_row as u32;
                                    let bottom = (rows + 1) * state.thumbnail_size;
                                    state.y_offset = bottom_align(bottom as f32);
                                }
                            }
                        }
                        Key::F1 => state.egui_state.top_bar ^= true,
                        _ => {}
                    }
                }
                _ => {}
            }
            if let Some(coll) = app
                .active_collection
                .map(|id| app.database.collections.get_mut(&id).unwrap())
            {
                handle_event_viewer(
                    event,
                    &mut state,
                    &mut on_screen_uids,
                    coll,
                    &mut selected_uids,
                    &window,
                    sf_egui.context(),
                );
            }
        }
        state.begin_frame();
        let mut result = Ok(());
        sf_egui.do_frame(|ctx| {
            result = egui_ui::do_ui(&mut state, ctx, app, &res);
        });
        result?;
        if esc_pressed
            && !sf_egui.context().wants_keyboard_input()
            && !sf_egui.context().wants_pointer_input()
            && !state.egui_state.just_closed_window_with_esc
        {
            selected_uids.clear()
        }
        let mut coll = app
            .active_collection
            .map(|id| app.database.collections.get_mut(&id).unwrap());
        if let Some(action) = &state.egui_state.action {
            match action {
                Action::Quit => window.close(),
                Action::QuitNoSave => {
                    app.no_save = true;
                    window.close();
                }
                Action::SelectNone => selected_uids.clear(),
                Action::SearchNext => search_next(&mut state, coll.as_mut().unwrap()),
                Action::SearchPrev => search_prev(&mut state, coll.as_mut().unwrap()),
                Action::SelectAll => select_all(&mut selected_uids, &state, coll.as_mut().unwrap()),
                Action::SortEntries => state.entries_view.sort(coll.as_mut().unwrap()),
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
                    &selected_uids,
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
        if let Some(id) = state.highlight {
            let mut search_highlight = RectangleShape::with_size(
                (state.thumbnail_size as f32, state.thumbnail_size as f32).into(),
            );
            search_highlight.set_fill_color(Color::TRANSPARENT);
            search_highlight.set_outline_color(Color::RED);
            search_highlight.set_outline_thickness(-2.0);
            let y_of_item = id as f32 / state.thumbnails_per_row as f32;
            let pixel_y = y_of_item as f32 * state.thumbnail_size as f32;
            let highlight_offset = pixel_y - state.y_offset;
            let x_of_item = id as f32 % state.thumbnails_per_row as f32;
            search_highlight.set_position((
                x_of_item as f32 * state.thumbnail_size as f32,
                highlight_offset,
            ));
            window.draw(&search_highlight);
        }
        if let Some(tex) = state.egui_state.load_folder_window.texture.as_ref() {
            let mut rs = RectangleShape::from_rect(Rect::new(800., 64., 512., 512.));
            rs.set_texture(tex, true);
            rs.set_outline_color(Color::YELLOW);
            rs.set_outline_thickness(4.0);
            window.draw(&rs);
        }
        let mut tex_src = TexSrc {
            state: &mut state,
            res: &res,
            coll: app
                .active_collection
                .map(|id| &app.database.collections[&id]),
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
    let rel_offset = state.y_offset as u32 % state.thumbnail_size;
    let thumb_y = (y as u32 + rel_offset) / state.thumbnail_size;
    let thumb_index = thumb_y * state.thumbnails_per_row as u32 + thumb_x;
    on_screen_entries.get(thumb_index as usize).copied()
}

fn handle_event_viewer(
    event: Event,
    state: &mut State,
    on_screen_entries: &mut Vec<entry::Id>,
    db: &mut Collection,
    selected_entries: &mut Vec<entry::Id>,
    window: &RenderWindow,
    ctx: &CtxRef,
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
            if button == mouse::Button::LEFT {
                if Key::LSHIFT.is_pressed() {
                    if selected_entries.contains(&uid) {
                        selected_entries.retain(|&rhs| rhs != uid);
                    } else {
                        selected_entries.push(uid);
                    }
                } else {
                    open_with_external(&[&db.entries[&uid].path]);
                }
            } else if button == mouse::Button::RIGHT {
                let vec = if selected_entries.contains(&uid) {
                    selected_entries.clone()
                } else {
                    vec![uid]
                };
                state.egui_state.add_entries_window(vec);
            }
        }
        Event::KeyPressed { code, ctrl, .. } => {
            if ctx.wants_keyboard_input() {
                return;
            }
            if code == Key::PAGEDOWN {
                state.y_offset += window.size().y as f32;
            } else if code == Key::PAGEUP {
                state.y_offset -= window.size().y as f32;
                if state.y_offset < 0.0 {
                    state.y_offset = 0.0;
                }
            } else if code == Key::ENTER {
                let mut paths: Vec<&Path> = Vec::new();
                for &uid in selected_entries.iter() {
                    paths.push(&db.entries[&uid].path);
                }
                if paths.is_empty() && state.filter.active() {
                    for uid in db.filter(&state.filter) {
                        paths.push(&db.entries[&uid].path);
                    }
                }
                paths.sort();
                open_with_external(&paths);
            } else if code == Key::A && ctrl {
                select_all(selected_entries, state, db);
            } else if code == Key::SLASH {
                state.search_edit = true;
            } else if code == Key::N {
                search_next(state, db);
            } else if code == Key::P {
                search_prev(state, db);
            } else if code == Key::F {
                state.filter_edit = true;
            } else if code == Key::C {
                let mp = window.mouse_position();
                let uid = match entry_at_xy(mp.x, mp.y, state, on_screen_entries) {
                    Some(uid) => uid,
                    None => return,
                };
                if let Err(e) = copy_image_to_clipboard(state, &db, uid) {
                    MessageDialog::new()
                        .set_title("Error")
                        .set_level(MessageLevel::Error)
                        .set_description(&e.to_string())
                        .show();
                }
            } else if code == Key::T {
                state.egui_state.tag_window.toggle();
            } else if code == Key::Q {
                state.egui_state.sequences_window.on ^= true;
            } else if code == Key::S {
                state.entries_view.sort(db);
            }
        }
        _ => {}
    }
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

fn select_all(selected_uids: &mut Vec<entry::Id>, state: &State, db: &Collection) {
    selected_uids.clear();
    for uid in db.filter(&state.filter) {
        selected_uids.push(uid);
    }
}

fn search_prev(state: &mut State, db: &mut Collection) {
    if state.search_cursor > 0 {
        state.search_cursor -= 1;
    }
    search_goto_cursor(state, db);
}

fn search_next(state: &mut State, db: &mut Collection) {
    state.search_cursor += 1;
    search_goto_cursor(state, db);
    if !state.search_success {
        state.search_cursor -= 1;
    }
}

fn find_nth(state: &State, db: &Collection, nth: usize) -> Option<Uid> {
    state
        .entries_view
        .filter(db, &state.filter)
        .enumerate()
        .filter(|(_, uid)| {
            let en = &db.entries[uid];
            en.spec_satisfied(&state.search_spec)
        })
        .map(|(i, _)| i as Uid)
        .nth(nth)
}

fn search_goto_cursor(state: &mut State, db: &Collection) {
    if let Some(uid) = find_nth(state, db, state.search_cursor) {
        state.highlight = Some(uid);
        state.search_success = true;
        let y_of_item = uid as f32 / state.thumbnails_per_row as f32;
        let y: f32 = (y_of_item * state.thumbnail_size as f32) as f32;
        state.y_offset = y;
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
    let row_offset = state.y_offset as u32 / thumb_size;
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
    y_offset: f32,
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
    highlight: Option<Uid>,
    filter_edit: bool,
    filter_string: String,
    clipboard_ctx: Clipboard,
    egui_state: egui_ui::EguiState,
    entries_view: EntriesView,
}

fn set_active_collection(
    entries_view: &mut EntriesView,
    app: &mut Application,
    id: collection::Id,
) -> anyhow::Result<()> {
    *entries_view =
        EntriesView::from_collection(&app.database.collections[&app.active_collection.unwrap()]);
    std::env::set_current_dir(&app.database.collections[&id].root_path)
        .context("failed to set directory")
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
            y_offset: 0.0,
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
            egui_state,
            entries_view: EntriesView::default(),
            search_spec: FilterSpec::default(),
        }
    }
    fn wipe_search(&mut self) {
        self.search_cursor = 0;
        self.search_edit = false;
        self.search_success = false;
        self.highlight = None;
    }
    fn begin_frame(&mut self) {
        self.egui_state.begin_frame();
    }
}

fn open_with_external(paths: &[&Path]) {
    use std::process::Command;
    struct Cmd {
        command: Command,
        have_args: bool,
        exts: &'static [&'static str],
    }
    let mut general_cmd = Cmd {
        command: {
            let mut c = Command::new("feh");
            c.arg("--auto-rotate");
            c
        },
        exts: &[],
        have_args: false,
    };
    let mut commands = vec![
        Cmd {
            command: {
                let mut c = Command::new("mpv");
                c.arg("--ab-loop-a=0");
                c
            },
            exts: &["gif", "webm", "mov", "mp4", "m4v", "wmv", "avi"],
            have_args: false,
        },
        Cmd {
            command: Command::new("ruffle"),
            exts: &["swf"],
            have_args: false,
        },
    ];
    for path in paths {
        let mut cmd = &mut general_cmd;
        if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
            let lower = &ext.to_lowercase();
            for c in &mut commands {
                if c.exts.iter().any(|&e| e == lower) {
                    cmd = c;
                }
            }
        }
        cmd.command.arg(path);
        cmd.have_args = true;
    }
    if general_cmd.have_args {
        general_cmd.command.spawn().unwrap();
    }

    for mut c in commands {
        if c.have_args {
            c.command.spawn().unwrap();
        }
    }
}
