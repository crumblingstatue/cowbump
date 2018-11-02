mod dialog;
mod thumbnail_loader;

use db::{Db, Uid};
use failure::Error;
use FilterSpec;

use self::thumbnail_loader::ThumbnailLoader;
use sfml::graphics::{
    Color, Font, RenderTarget, RenderWindow, Sprite, Text, Texture, Transformable,
};
use sfml::system::SfBox;
use sfml::window::{mouse, Event, Key, Style, VideoMode};
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
    recalc_on_screen_items(&mut on_screen_uids, db, &state, window.size().y);
    while window.is_open() {
        let scroll_speed = 8.0;
        if Key::Down.is_pressed() {
            state.y_offset += scroll_speed;
            recalc_on_screen_items(&mut on_screen_uids, db, &state, window.size().y);
        } else if Key::Up.is_pressed() {
            state.y_offset -= scroll_speed;
            if state.y_offset < 0.0 {
                state.y_offset = 0.0;
            }
            recalc_on_screen_items(&mut on_screen_uids, db, &state, window.size().y);
        }

        while let Some(event) = window.poll_event() {
            if let Event::Closed = event {
                window.close();
            }
            if !state.dialog_stack.handle_event(event, &window, db) {
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
        window.clear(Color::BLACK);
        state.draw_thumbnails(&mut window, db, &on_screen_uids, &selected_uids);
        state.dialog_stack.draw(
            &mut window,
            &state.font,
            db,
            &state.thumbnail_cache,
            state.thumbnail_size,
            &state.error_texture,
            &state.loading_texture,
            &mut state.thumbnail_loader,
        );
        window.display();
    }
    Ok(())
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
            let thumb_x = x as u32 / state.thumbnail_size;
            let rel_offset = state.y_offset as u32 % state.thumbnail_size;
            let thumb_y = (y as u32 + rel_offset) / state.thumbnail_size;
            let thumb_index = thumb_y * state.thumbnails_per_row + thumb_x;
            let uid: Uid = on_screen_uids[thumb_index as usize];
            if button == mouse::Button::Left {
                if Key::LShift.is_pressed() {
                    if selected_uids.contains(&uid) {
                        selected_uids.remove(&uid);
                    } else {
                        selected_uids.insert(uid);
                    }
                } else {
                    open_with_external(&[&db.entries[uid as usize].path]);
                }
            } else if button == mouse::Button::Right {
                state
                    .dialog_stack
                    .push(Box::new(dialog::Meta::new(uid, db)));
            }
        }
        Event::KeyPressed { code, .. } => {
            if code == Key::PageDown {
                state.y_offset += window.size().y as f32;
                recalc_on_screen_items(on_screen_uids, db, state, window.size().y);
            } else if code == Key::PageUp {
                state.y_offset -= window.size().y as f32;
                if state.y_offset < 0.0 {
                    state.y_offset = 0.0;
                }
                recalc_on_screen_items(on_screen_uids, db, state, window.size().y);
            } else if code == Key::Return {
                let mut paths: Vec<&Path> = Vec::new();
                for &uid in selected_uids.iter() {
                    paths.push(&db.entries[uid as usize].path);
                }
                open_with_external(&paths);
            }
        }
        _ => {}
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
    dialog_stack: dialog::Stack,
}

impl State {
    fn new(window_width: u32) -> Self {
        let thumbnails_per_row = 5;
        Self {
            thumbnails_per_row,
            y_offset: 0.0,
            thumbnail_size: window_width / thumbnails_per_row,
            filter: FilterSpec { has_tags: vec![] },
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
            dialog_stack: Default::default(),
        }
    }
    fn draw_thumbnails(
        &mut self,
        window: &mut RenderWindow,
        db: &Db,
        uids: &[Uid],
        selected_uids: &BTreeSet<Uid>,
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
            );
        }
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(too_many_arguments))]
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
) {
    let texture = match thumbnail_cache.get(&uid) {
        Some(opt_texture) => match *opt_texture {
            Some(ref tex) => tex,
            None => {
                if let Some(ext) = db.entries[uid as usize]
                    .path
                    .extension()
                    .map(|e| e.to_str())
                {
                    let mut text = Text::new(ext.unwrap(), font, 20);
                    text.set_position((x, y + 64.0));
                    window.draw(&text);
                }
                error_texture
            }
        },
        None => {
            let entry = &db.entries[uid as usize];
            thumbnail_loader.request(&entry.path, thumb_size, uid);
            loading_texture
        }
    };
    sprite.set_texture(texture, true);
    sprite.set_position((x, y));
    window.draw(sprite);
}

fn open_with_external(paths: &[&Path]) {
    use std::process::Command;
    struct Cmd {
        command: Command,
        exts: Vec<String>,
        have_args: bool,
    }
    let mut general_cmd = Cmd {
        command: Command::new("feh"),
        exts: vec![],
        have_args: false,
    };
    let mut commands = vec![
        Cmd {
            command: {
                let mut c = Command::new("mpv");
                c.arg("--ab-loop-a=0");
                c
            },
            exts: vec!["gif".into(), "webm".into()],
            have_args: false,
        },
        Cmd {
            command: {
                let mut c = Command::new("swfopen");
                c.arg("chromium");
                c
            },
            exts: vec!["swf".into()],
            have_args: false,
        },
    ];
    for path in paths {
        let mut cmd = &mut general_cmd;
        if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
            for c in &mut commands {
                if c.exts.iter().any(|e| e == ext) {
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
