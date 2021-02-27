mod debug;
mod thumbnail_loader;

use crate::db::{Db, Uid};
use crate::FilterSpec;
use egui::{Event as EguiEv, Modifiers, PointerButton, Pos2, RawInput, TextureId};
use failure::Error;

use self::thumbnail_loader::ThumbnailLoader;
use arboard::Clipboard;
use sfml::graphics::{
    Color, Font, PrimitiveType, RectangleShape, RenderStates, RenderTarget, RenderWindow, Shape,
    Sprite, Text, Texture, Transformable, Vertex, VertexArray,
};
use sfml::window::{mouse, Event, Key, Style, VideoMode};
use sfml::SfBox;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

fn egui_tex_to_rgba_vec(tex: &egui::Texture) -> Vec<u8> {
    let srgba = tex.srgba_pixels();
    let mut vec = Vec::new();
    for c in srgba {
        vec.extend_from_slice(&c.to_array());
    }
    vec
}

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
    // Texture isn't valid until first call to begin_frame, so we just render a dummy frame
    egui_ctx.begin_frame(RawInput::default());
    let _ = egui_ctx.end_frame();
    let egui_tex = egui_ctx.texture();
    let mut tex = Texture::new(egui_tex.width as u32, egui_tex.height as u32).unwrap();
    let tex_pixels = egui_tex_to_rgba_vec(&egui_tex);
    unsafe {
        tex.update_from_pixels(
            &tex_pixels,
            egui_tex.width as u32,
            egui_tex.height as u32,
            0,
            0,
        );
    }
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
        let mut raw_input = RawInput {
            screen_rect: Some(egui::Rect {
                min: Pos2::new(0., 0.),
                max: Pos2::new(window.size().x as f32, window.size().y as f32),
            }),
            ..Default::default()
        };

        while let Some(event) = window.poll_event() {
            match event {
                Event::Closed => window.close(),
                Event::KeyPressed { code, .. } => {
                    if let Some(key) = sf_kp_to_egui_kp(code) {
                        raw_input.events.push(egui::Event::Key {
                            key,
                            modifiers: egui::Modifiers::default(),
                            pressed: true,
                        });
                    }
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
                Event::MouseMoved { x, y } => {
                    raw_input
                        .events
                        .push(EguiEv::PointerMoved(Pos2::new(x as f32, y as f32)));
                }
                Event::MouseButtonPressed { x, y, button } => {
                    raw_input.events.push(EguiEv::PointerButton {
                        pos: Pos2::new(x as f32, y as f32),
                        button: sf_button_to_egui(button),
                        pressed: true,
                        modifiers: Modifiers::default(),
                    });
                }
                Event::MouseButtonReleased { x, y, button } => {
                    raw_input.events.push(EguiEv::PointerButton {
                        pos: Pos2::new(x as f32, y as f32),
                        button: sf_button_to_egui(button),
                        pressed: false,
                        modifiers: Modifiers::default(),
                    });
                }
                Event::TextEntered { unicode } => {
                    if !unicode.is_control() {
                        raw_input.events.push(EguiEv::Text(unicode.to_string()));
                    }
                }
                _ => {}
            }
            if !(egui_ctx.wants_pointer_input() || egui_ctx.wants_keyboard_input()) {
                handle_event_viewer(
                    event,
                    &mut state,
                    &mut on_screen_uids,
                    db,
                    &mut selected_uids,
                    &window,
                );
            }
        }
        egui_ctx.begin_frame(raw_input);
        if state.search_edit {
            egui::Window::new("Search").show(&egui_ctx, |ui| {
                let prev = state.search_string.clone();
                let re = ui.text_edit_singleline(&mut state.search_string);
                ui.memory().request_kb_focus(re.id);
                if re.lost_kb_focus() {
                    state.search_edit = false;
                }
                // Text was changed. TODO: Figure out if there is better way to see
                if prev != state.search_string {
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
        state.image_prop_windows.retain(|ids| {
            let mut open = true;
            let title = {
                if ids.len() == 1 {
                    db.entries[ids[0] as usize].path.display().to_string()
                } else {
                    format!("{} images", ids.len())
                }
            };
            egui::Window::new(title)
                .open(&mut open)
                .show(&egui_ctx, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for &id in ids {
                            ui.image(
                                TextureId::User(id as u64),
                                (512.0 / ids.len() as f32, 512.0 / ids.len() as f32),
                            );
                        }
                    });
                });
            open
        });
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
        for egui::ClippedMesh(_rect, mesh) in egui_ctx.tessellate(shapes) {
            let mut arr = VertexArray::new(PrimitiveType::TRIANGLES, mesh.indices.len());
            let (tw, th, tex) = match mesh.texture_id {
                TextureId::Egui => (egui_tex.width as f32, egui_tex.height as f32, &*tex),
                TextureId::User(id) => {
                    let (_has, tex) = get_tex_for_uid(
                        &state.thumbnail_cache,
                        id as u32,
                        &state.error_texture,
                        db,
                        &mut state.thumbnail_loader,
                        state.thumbnail_size,
                        &state.loading_texture,
                    );
                    (tex.size().x as f32, tex.size().y as f32, tex)
                }
            };
            for idx in mesh.indices {
                let v = mesh.vertices[idx as usize];
                let sf_v = Vertex::new(
                    (v.pos.x, v.pos.y).into(),
                    Color::rgba(v.color.r(), v.color.g(), v.color.b(), v.color.a()),
                    (v.uv.x * tw, v.uv.y * th).into(),
                );
                arr.append(&sf_v);
            }
            let mut rs = RenderStates::default();
            rs.set_texture(Some(&tex));
            window.draw_with_renderstates(&arr, &rs);
        }
        debug::draw(&mut window, &state.font);
        window.display();
        load_anim_rotation += 2.0;
    }
    Ok(())
}

fn sf_kp_to_egui_kp(code: Key) -> Option<egui::Key> {
    use egui::Key as EKey;
    Some(match code {
        Key::DOWN => EKey::ArrowDown,
        Key::LEFT => EKey::ArrowLeft,
        Key::RIGHT => EKey::ArrowRight,
        Key::UP => EKey::ArrowUp,
        Key::ESCAPE => EKey::Escape,
        Key::TAB => EKey::Tab,
        Key::BACKSPACE => EKey::Backspace,
        Key::ENTER => EKey::Enter,
        Key::SPACE => EKey::Space,
        Key::INSERT => EKey::Insert,
        Key::DELETE => EKey::Delete,
        Key::HOME => EKey::Home,
        Key::END => EKey::End,
        Key::PAGEUP => EKey::PageUp,
        Key::PAGEDOWN => EKey::PageDown,
        Key::NUM0 => EKey::Num0,
        Key::NUM1 => EKey::Num1,
        Key::NUM2 => EKey::Num2,
        Key::NUM3 => EKey::Num3,
        Key::NUM4 => EKey::Num4,
        Key::NUM5 => EKey::Num5,
        Key::NUM6 => EKey::Num6,
        Key::NUM7 => EKey::Num7,
        Key::NUM8 => EKey::Num8,
        Key::NUM9 => EKey::Num9,
        Key::A => EKey::A,
        Key::B => EKey::B,
        Key::C => EKey::C,
        Key::D => EKey::D,
        Key::E => EKey::E,
        Key::F => EKey::F,
        Key::G => EKey::G,
        Key::H => EKey::H,
        Key::I => EKey::I,
        Key::J => EKey::J,
        Key::K => EKey::K,
        Key::L => EKey::L,
        Key::M => EKey::M,
        Key::N => EKey::N,
        Key::O => EKey::O,
        Key::P => EKey::P,
        Key::Q => EKey::Q,
        Key::R => EKey::R,
        Key::S => EKey::S,
        Key::T => EKey::T,
        Key::U => EKey::U,
        Key::V => EKey::V,
        Key::W => EKey::W,
        Key::X => EKey::X,
        Key::Y => EKey::Y,
        Key::Z => EKey::Z,
        _ => return None,
    })
}

fn sf_button_to_egui(button: mouse::Button) -> PointerButton {
    match button {
        mouse::Button::LEFT => PointerButton::Primary,
        mouse::Button::RIGHT => PointerButton::Secondary,
        mouse::Button::MIDDLE => PointerButton::Middle,
        _ => panic!("Unhandled pointer button: {:?}", button),
    }
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
) {
    match event {
        Event::MouseButtonPressed { button, x, y } => {
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
                state.image_prop_windows.push(vec);
            }
        }
        Event::KeyPressed { code, .. } => {
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
    image_prop_windows: Vec<Vec<Uid>>,
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
