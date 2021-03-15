mod debug;
mod thumbnail_loader;

use crate::db::{Db, Uid};
use crate::FilterSpec;
use egui::{Color32, Label, TextureId};
use failure::Error;
use retain_mut::RetainMut;

use self::thumbnail_loader::ThumbnailLoader;
use arboard::Clipboard;
use sfml::graphics::{
    Color, Font, RectangleShape, RenderStates, RenderTarget, RenderWindow, Shape, Sprite, Text,
    Texture, Transformable,
};
use sfml::window::{mouse, Event, Key, Style, VideoMode};
use sfml::SfBox;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

pub fn run(db: &mut Db) -> Result<(), Error> {
    let mut window = RenderWindow::new(
        VideoMode::desktop_mode(),
        "Cowbump",
        Style::NONE,
        &Default::default(),
    );
    window.set_vertical_sync_enabled(true);
    let mut state = State::new(window.size().x);
    let mut on_screen_uids: Vec<Uid> = Vec::new();
    let mut selected_uids: BTreeSet<Uid> = Default::default();
    let mut load_anim_rotation = 0.0;
    let mut egui_ctx = egui::CtxRef::default();
    let tex = egui_sfml::create_texture(&mut egui_ctx, &window);
    while window.is_open() {
        let scroll_speed = 8.0;
        if Key::DOWN.is_pressed() {
            state.y_offset += scroll_speed;
        } else if Key::UP.is_pressed() {
            state.y_offset -= scroll_speed;
            if state.y_offset < 0.0 {
                state.y_offset = 0.0;
            }
        }
        let mut raw_input = egui_sfml::make_raw_input(&window);

        while let Some(event) = window.poll_event() {
            egui_sfml::handle_event(&mut raw_input, &event);
            match event {
                Event::Closed => window.close(),
                Event::KeyPressed { code, .. } => {
                    match code {
                        Key::ESCAPE => selected_uids.clear(),
                        Key::HOME => {
                            if !egui_ctx.wants_keyboard_input() {
                                state.y_offset = 0.0;
                            }
                        }
                        Key::END => {
                            if !egui_ctx.wants_keyboard_input() {
                                // Align the bottom edge of the view with the bottom edge of the last row.
                                // To do align the camera with a bottom edge, we need to subtract the screen
                                // height from it.
                                let bottom_align = |y: f32| y - window.size().y as f32;
                                let n_pics = db.filter(&state.filter).count();
                                let rows = n_pics as u32 / state.thumbnails_per_row;
                                let bottom = (rows + 1) * state.thumbnail_size;
                                state.y_offset = bottom_align(bottom as f32);
                            }
                        }
                        Key::F12 => debug::toggle(),
                        _ => {}
                    }
                }
                _ => {}
            }
            handle_event_viewer(
                event,
                &mut state,
                &mut on_screen_uids,
                db,
                &mut selected_uids,
                &window,
                egui_ctx.wants_pointer_input(),
                egui_ctx.wants_keyboard_input(),
            );
        }
        egui_ctx.begin_frame(raw_input);
        if state.search_edit {
            egui::Window::new("Search").show(&egui_ctx, |ui| {
                let re = ui.text_edit_singleline(&mut state.search_string);
                ui.memory().request_kb_focus(re.id);
                if re.lost_kb_focus() {
                    state.search_edit = false;
                }
                if re.changed() {
                    state.search_cursor = 0;
                    search_goto_cursor(&mut state, db);
                }
            });
        }
        if state.filter_edit {
            egui::Window::new("Filter").show(&egui_ctx, |ui| {
                let ed = ui.text_edit_singleline(&mut state.filter.substring_match);
                ui.memory().request_kb_focus(ed.id);
                if ed.lost_kb_focus() {
                    state.filter_edit = false;
                }
            });
        }
        if state.tag_window {
            let add_tag = &mut state.add_tag;
            let tags = &mut db.tags;
            let entries = &mut db.entries;
            egui::Window::new("Tag editor")
                .open(&mut state.tag_window)
                .show(&egui_ctx, move |ui| {
                    ui.vertical(|ui| {
                        let mut i = 0;
                        tags.retain(|tag| {
                            ui.label(tag.names[0].clone());
                            let keep = if ui.button("x").clicked() {
                                for en in entries.iter_mut() {
                                    en.tags.retain(|&uid| uid != i);
                                }
                                false
                            } else {
                                true
                            };
                            i += 1;
                            keep
                        });
                        if ui.button("Add tag").clicked() {
                            *add_tag = Some(AddTag::default());
                        }
                    });
                });
        }
        if let Some(add_tag) = &mut state.add_tag {
            let mut rem = false;
            egui::Window::new("Add new tag").show(&egui_ctx, |ui| {
                let re = ui.text_edit_singleline(&mut add_tag.name);
                ui.memory().request_kb_focus(re.id);
                if re.ctx.input().key_down(egui::Key::Enter) {
                    rem = true;
                    db.add_new_tag(crate::tag::Tag {
                        names: vec![add_tag.name.clone()],
                        implies: vec![],
                    });
                }
            });
            if rem {
                state.add_tag = None;
            }
        }
        image_windows_ui(&mut state, db, &egui_ctx);
        recalc_on_screen_items(&mut on_screen_uids, db, &state, window.size().y);
        window.clear(Color::BLACK);
        state.draw_thumbnails(
            &mut window,
            db,
            &on_screen_uids,
            &selected_uids,
            load_anim_rotation,
        );
        if let Some(id) = state.highlight {
            let mut search_highlight = RectangleShape::with_size(
                (state.thumbnail_size as f32, state.thumbnail_size as f32).into(),
            );
            search_highlight.set_fill_color(Color::TRANSPARENT);
            search_highlight.set_outline_color(Color::RED);
            search_highlight.set_outline_thickness(-2.0);
            let y_of_item = id as u32 / state.thumbnails_per_row;
            let pixel_y = y_of_item as f32 * state.thumbnail_size as f32;
            let highlight_offset = pixel_y - state.y_offset;
            let x_of_item = id as u32 % state.thumbnails_per_row;
            search_highlight.set_position((
                x_of_item as f32 * state.thumbnail_size as f32,
                highlight_offset,
            ));
            window.draw(&search_highlight);
        }
        let (_output, shapes) = egui_ctx.end_frame();
        let mut tex_src = TexSrc {
            state: &mut state,
            db,
        };
        egui_sfml::draw(&mut window, &egui_ctx, &tex, shapes, &mut tex_src);
        debug::draw(&mut window, &state.font);
        window.display();
        load_anim_rotation += 2.0;
    }
    Ok(())
}

fn image_windows_ui(state: &mut State, db: &mut Db, egui_ctx: &egui::CtxRef) {
    state.image_prop_windows.retain_mut(|propwin| {
        let mut open = true;
        let n_images = propwin.image_uids.len();
        let title = {
            if propwin.image_uids.len() == 1 {
                db.entries[propwin.image_uids[0] as usize]
                    .path
                    .display()
                    .to_string()
            } else {
                format!("{} images", n_images)
            }
        };
        egui::Window::new(title)
            .open(&mut open)
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        for &id in &propwin.image_uids {
                            ui.image(
                                TextureId::User(id as u64),
                                (512.0 / n_images as f32, 512.0 / n_images as f32),
                            );
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        for tagid in common_tags(&propwin.image_uids, db) {
                            ui.group(|ui| {
                                ui.add(
                                    Label::new(db.tags[tagid as usize].names[0].clone())
                                        .wrap(false)
                                        .background_color(Color32::from_rgb(50, 40, 45)),
                                );
                                if ui.button("x").clicked() {
                                    // TODO: This only works for 1 item windows
                                    db.entries[propwin.image_uids[0] as usize]
                                        .tags
                                        .retain(|&t| t != tagid);
                                }
                            });
                        }
                        let plus_re = ui.button("+");
                        let popup_id = ui.make_persistent_id("popid");
                        if plus_re.clicked() {
                            ui.memory().toggle_popup(popup_id);
                        }
                        egui::popup::popup_below_widget(ui, popup_id, &plus_re, |ui| {
                            ui.set_min_width(100.0);
                            let mut tag_add = None;
                            for (i, tag) in db.tags.iter().enumerate() {
                                let name = tag.names[0].clone();
                                if ui.button(name).clicked() {
                                    tag_add = Some((propwin.image_uids[0], i as u32));
                                }
                            }
                            if let Some((image_id, tag_id)) = tag_add {
                                db.add_tag_for(image_id, tag_id);
                            }
                        });
                    })
                });
            });
        open
    });
}

fn common_tags(ids: &[u32], db: &Db) -> BTreeSet<Uid> {
    let mut set = BTreeSet::new();
    for &id in ids {
        for &tagid in &db.entries[id as usize].tags {
            set.insert(tagid);
        }
    }
    set
}

fn get_uid_xy(x: i32, y: i32, state: &State, on_screen_uids: &[Uid]) -> Option<Uid> {
    let thumb_x = x as u32 / state.thumbnail_size;
    let rel_offset = state.y_offset as u32 % state.thumbnail_size;
    let thumb_y = (y as u32 + rel_offset) / state.thumbnail_size;
    let thumb_index = thumb_y * state.thumbnails_per_row + thumb_x;
    on_screen_uids.get(thumb_index as usize).copied()
}

fn handle_event_viewer(
    event: Event,
    state: &mut State,
    on_screen_uids: &mut Vec<Uid>,
    db: &mut Db,
    selected_uids: &mut BTreeSet<Uid>,
    window: &RenderWindow,
    egui_mouse: bool,
    egui_kb: bool,
) {
    match event {
        Event::MouseButtonPressed { button, x, y } => {
            if egui_mouse {
                return;
            }
            let uid = match get_uid_xy(x, y, state, on_screen_uids) {
                Some(uid) => uid,
                None => return,
            };
            if button == mouse::Button::LEFT {
                if Key::LSHIFT.is_pressed() {
                    if selected_uids.contains(&uid) {
                        selected_uids.remove(&uid);
                    } else {
                        selected_uids.insert(uid);
                    }
                } else {
                    open_with_external(&[&db.entries[uid as usize].path]);
                }
            } else if button == mouse::Button::RIGHT {
                let vec = if selected_uids.contains(&uid) {
                    selected_uids.iter().cloned().collect()
                } else {
                    vec![uid]
                };
                state.image_prop_windows.push(ImagePropWindow::new(vec));
            }
        }
        Event::KeyPressed { code, .. } => {
            if egui_kb {
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
                for &uid in selected_uids.iter() {
                    paths.push(&db.entries[uid as usize].path);
                }
                if paths.is_empty() && state.filter.active() {
                    for uid in db.filter(&state.filter) {
                        paths.push(&db.entries[uid as usize].path);
                    }
                }
                open_with_external(&paths);
            } else if code == Key::SLASH {
                state.search_edit = true;
            } else if code == Key::N {
                state.search_cursor += 1;
                search_goto_cursor(state, db);
                // Keep the last entry highlighted even if search fails
                if !state.search_success {
                    state.search_cursor -= 1;
                }
            } else if code == Key::P {
                if state.search_cursor > 0 {
                    state.search_cursor -= 1;
                }
                search_goto_cursor(state, db);
            } else if code == Key::F {
                state.filter_edit = true;
            } else if code == Key::C {
                use arboard::ImageData;
                let mp = window.mouse_position();
                let uid = match get_uid_xy(mp.x, mp.y, state, on_screen_uids) {
                    Some(uid) => uid,
                    None => return,
                };
                let imgpath = &db.entries[uid as usize].path;
                let buf = std::fs::read(imgpath).unwrap();
                let img = match image::load_from_memory(&buf) {
                    Ok(img) => img,
                    Err(e) => {
                        eprintln!("(clipboard) Image open error: {}", e);
                        return;
                    }
                };
                let rgba = img.to_rgba8();
                let img_data = ImageData {
                    width: rgba.width() as usize,
                    height: rgba.height() as usize,
                    bytes: rgba.into_raw().into(),
                };
                if let Err(e) = state.clipboard_ctx.set_image(img_data) {
                    eprintln!("Error setting clipboard: {}", e);
                }
            } else if code == Key::T {
                state.tag_window = !state.tag_window;
            }
        }
        _ => {}
    }
}

fn find_nth(state: &State, db: &Db, nth: usize) -> Option<Uid> {
    let string = state.search_string.to_lowercase();
    db.entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            entry
                .path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_lowercase()
                .contains(&string)
        })
        .map(|(i, _)| i as Uid)
        .nth(nth)
}

fn search_goto_cursor(state: &mut State, db: &Db) {
    if let Some(uid) = find_nth(state, db, state.search_cursor) {
        state.highlight = Some(uid);
        state.search_success = true;
        let y_of_item = uid / state.thumbnails_per_row;
        let y: f32 = (y_of_item * state.thumbnail_size) as f32;
        state.y_offset = y;
    } else {
        state.search_success = false;
    }
}

fn recalc_on_screen_items(uids: &mut Vec<Uid>, db: &Db, state: &State, window_height: u32) {
    uids.clear();
    let thumb_size = state.thumbnail_size;
    let mut thumbnails_per_column = window_height / thumb_size;
    // Compensate for truncating division
    if window_height % thumb_size != 0 {
        thumbnails_per_column += 1;
    }
    // Since we can scroll, we can have another partially drawn frame per screen
    thumbnails_per_column += 1;
    let thumbnails_per_screen = (state.thumbnails_per_row * thumbnails_per_column) as usize;
    let row_offset = state.y_offset as u32 / thumb_size;
    let skip = row_offset * state.thumbnails_per_row;
    uids.extend(
        db.filter(&state.filter)
            .skip(skip as usize)
            .take(thumbnails_per_screen),
    );
}

type ThumbnailCache = HashMap<Uid, Option<SfBox<Texture>>>;

struct State {
    thumbnails_per_row: u32,
    y_offset: f32,
    thumbnail_size: u32,
    filter: FilterSpec,
    loading_texture: SfBox<Texture>,
    error_texture: SfBox<Texture>,
    thumbnail_cache: ThumbnailCache,
    thumbnail_loader: ThumbnailLoader,
    font: SfBox<Font>,
    search_edit: bool,
    search_string: String,
    /// The same search can be used to seek multiple entries
    search_cursor: usize,
    search_success: bool,
    highlight: Option<Uid>,
    filter_edit: bool,
    clipboard_ctx: Clipboard,
    image_prop_windows: Vec<ImagePropWindow>,
    tag_window: bool,
    add_tag: Option<AddTag>,
}

struct TexSrc<'state, 'db> {
    state: &'state mut State,
    db: &'db Db,
}

impl<'state, 'db> egui_sfml::UserTexSource for TexSrc<'state, 'db> {
    fn get_texture(&mut self, id: u64) -> (f32, f32, &Texture) {
        let (_has, tex) = get_tex_for_uid(
            &self.state.thumbnail_cache,
            id as u32,
            &self.state.error_texture,
            self.db,
            &mut self.state.thumbnail_loader,
            self.state.thumbnail_size,
            &self.state.loading_texture,
        );
        (tex.size().x as f32, tex.size().y as f32, tex)
    }
}

/// Add tag state when adding a new tag
#[derive(Default)]
struct AddTag {
    name: String,
}

/// Image properties window
struct ImagePropWindow {
    image_uids: Vec<Uid>,
}

impl ImagePropWindow {
    fn new(image_uids: Vec<Uid>) -> Self {
        Self { image_uids }
    }
}

impl State {
    fn new(window_width: u32) -> Self {
        let thumbnails_per_row = 5;
        let thumbnail_size = window_width / thumbnails_per_row;
        Self {
            thumbnails_per_row,
            y_offset: 0.0,
            thumbnail_size,
            filter: FilterSpec {
                has_tags: vec![],
                substring_match: String::new(),
            },
            loading_texture: Texture::from_memory(
                include_bytes!("../../loading.png"),
                &Default::default(),
            )
            .unwrap(),
            error_texture: Texture::from_memory(
                include_bytes!("../../error.png"),
                &Default::default(),
            )
            .unwrap(),
            thumbnail_cache: Default::default(),
            thumbnail_loader: Default::default(),
            font: Font::from_memory(include_bytes!("../../Vera.ttf")).unwrap(),
            search_edit: false,
            search_string: String::new(),
            search_cursor: 0,
            search_success: false,
            highlight: None,
            filter_edit: false,
            clipboard_ctx: Clipboard::new().unwrap(),
            image_prop_windows: Vec::new(),
            tag_window: false,
            add_tag: None,
        }
    }
    fn draw_thumbnails(
        &mut self,
        window: &mut RenderWindow,
        db: &Db,
        uids: &[Uid],
        selected_uids: &BTreeSet<Uid>,
        load_anim_rotation: f32,
    ) {
        let thumb_size = self.thumbnail_size;
        self.thumbnail_loader
            .write_to_cache(&mut self.thumbnail_cache);
        let mut sprite = Sprite::new();
        for (i, &uid) in uids.iter().enumerate() {
            let column = (i as u32) % self.thumbnails_per_row;
            let row = (i as u32) / self.thumbnails_per_row;
            let x = (column * thumb_size) as f32;
            let y = (row * thumb_size) as f32 - (self.y_offset % thumb_size as f32);
            if selected_uids.contains(&uid) {
                sprite.set_color(Color::GREEN);
            } else {
                sprite.set_color(Color::WHITE);
            }
            draw_thumbnail(
                &self.thumbnail_cache,
                db,
                window,
                x,
                y,
                uid,
                thumb_size,
                &mut sprite,
                &self.font,
                &self.error_texture,
                &self.loading_texture,
                &mut self.thumbnail_loader,
                load_anim_rotation,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_thumbnail<'a: 'b, 'b>(
    thumbnail_cache: &'a ThumbnailCache,
    db: &Db,
    window: &mut RenderWindow,
    x: f32,
    y: f32,
    uid: Uid,
    thumb_size: u32,
    sprite: &mut Sprite<'b>,
    font: &Font,
    error_texture: &'a Texture,
    loading_texture: &'a Texture,
    thumbnail_loader: &mut ThumbnailLoader,
    load_anim_rotation: f32,
) {
    let (has_img, texture) = get_tex_for_uid(
        thumbnail_cache,
        uid,
        error_texture,
        db,
        thumbnail_loader,
        thumb_size,
        loading_texture,
    );
    sprite.set_texture(texture, true);
    sprite.set_position((x, y));
    if thumbnail_loader.busy_with() == uid {
        sprite.set_origin((27.0, 6.0));
        sprite.move_((48.0, 48.0));
        sprite.set_rotation(load_anim_rotation);
    } else {
        sprite.set_rotation(0.0);
        sprite.set_origin((0.0, 0.0));
    }
    window.draw_sprite(sprite, &RenderStates::DEFAULT);
    if !has_img {
        if let Some(file_name) = db.entries[uid as usize]
            .path
            .file_name()
            .map(|e| e.to_str())
        {
            let mut text = Text::new(file_name.unwrap(), font, 12);
            text.set_position((x, y + 64.0));
            window.draw_text(&text, &RenderStates::DEFAULT);
        }
    }
}

fn get_tex_for_uid<'t>(
    thumbnail_cache: &'t HashMap<u32, Option<SfBox<Texture>>>,
    uid: u32,
    error_texture: &'t Texture,
    db: &Db,
    thumbnail_loader: &mut ThumbnailLoader,
    thumb_size: u32,
    loading_texture: &'t Texture,
) -> (bool, &'t Texture) {
    let (has_img, texture) = match thumbnail_cache.get(&uid) {
        Some(opt_texture) => match *opt_texture {
            Some(ref tex) => (true, tex as &Texture),
            None => (false, error_texture),
        },
        None => {
            let entry = &db.entries[uid as usize];
            thumbnail_loader.request(&entry.path, thumb_size, uid);
            (false, loading_texture)
        }
    };
    (has_img, texture)
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
