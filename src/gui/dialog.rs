use crate::db::{Db, Uid};
use crate::gui::thumbnail_loader::ThumbnailLoader;
use crate::tag::Tag;
use ropey::Rope;
use sfml::graphics::{
    Color, Font, RectangleShape, RenderStates, RenderTarget, RenderWindow, Shape, Sprite, Text,
    Texture, Transformable,
};
use sfml::system::{Vector2f, Vector2u};
use sfml::window::{Event, Key};

/// A stack of dialogues
#[derive(Default)]
pub struct Stack {
    dialogs: Vec<Box<dyn Dialog>>,
}

struct Button {
    x: f32,
    y: f32,
    w: u32,
    h: u32,
    text: String,
}

struct TagButton {
    button: Button,
    uid: Uid,
}

impl Button {
    fn draw(&self, x: f32, y: f32, font: &Font, window: &mut RenderWindow) {
        let mut rect = RectangleShape::with_size(Vector2f::new(self.w as f32, self.h as f32));
        rect.set_position((x + self.x, y + self.y));
        rect.set_fill_color(Color::rgb(200, 200, 200));
        window.draw_rectangle_shape(&rect, &RenderStates::DEFAULT);
        let mut text = Text::new(&self.text, font, self.h);
        text.set_position((x + self.x, y + self.y));
        text.set_fill_color(Color::rgb(50, 50, 50));
        window.draw_text(&text, &RenderStates::DEFAULT);
    }
}

impl Stack {
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        window: &mut RenderWindow,
        font: &Font,
        db: &Db,
        thumbnail_cache: &super::ThumbnailCache,
        thumb_size: u32,
        error_texture: &Texture,
        loading_texture: &Texture,
        thumbnail_loader: &mut ThumbnailLoader,
        load_anim_rotation: f32,
    ) {
        let Vector2u { x: ww, y: wh } = window.size();
        let wcx = ww / 2;
        let wcy = wh / 2;
        for dialog in &self.dialogs {
            let Vector2f { x: dw, y: dh } = dialog.size();
            let dcx = dw / 2.;
            let dcy = dh / 2.;
            dialog.draw(
                window,
                wcx as f32 - dcx,
                wcy as f32 - dcy,
                font,
                db,
                thumbnail_cache,
                thumb_size,
                error_texture,
                loading_texture,
                thumbnail_loader,
                load_anim_rotation,
            );
        }
    }
    pub fn push(&mut self, dialog: Box<dyn Dialog>) {
        self.dialogs.push(dialog);
    }
    pub fn pop(&mut self) -> Option<Box<dyn Dialog>> {
        self.dialogs.pop()
    }
    pub fn handle_event(&mut self, event: Event, window: &RenderWindow, db: &mut Db) -> bool {
        let Vector2u { x: ww, y: wh } = window.size();
        let wcx = ww / 2;
        let wcy = wh / 2;
        let mut pop = false;
        let mut push = None;
        let result = match self.dialogs.last_mut() {
            Some(last) => {
                let Vector2f { x: dw, y: dh } = last.size();
                let dcx = dw as u32 / 2;
                let dcy = dh as u32 / 2;
                let msg = last.handle_event(wcx - dcx, wcy - dcy, event, db);
                match msg {
                    Msg::PopMe => pop = true,
                    Msg::PushNew(dial) => push = Some(dial),
                    Msg::Nothing => {}
                }
                true
            }
            None => false,
        };
        if pop {
            self.pop();
        }
        if let Some(dial) = push {
            self.push(dial);
        }
        result
    }
}

pub trait Dialog {
    #[allow(clippy::too_many_arguments)]
    fn draw(
        &self,
        window: &mut RenderWindow,
        x: f32,
        y: f32,
        font: &Font,
        db: &Db,
        thumbnail_cache: &super::ThumbnailCache,
        thumb_size: u32,
        error_texture: &Texture,
        loading_texture: &Texture,
        thumbnail_loader: &mut ThumbnailLoader,
        load_anim_rotation: f32,
    );
    fn size(&self) -> Vector2f;
    fn handle_event(&mut self, x: u32, y: u32, event: Event, db: &mut Db) -> Msg;
    fn draw_bg(&self, x: f32, y: f32, window: &mut RenderWindow) {
        let mut rect = RectangleShape::new();
        rect.set_position((x, y));
        rect.set_size(self.size());
        rect.set_fill_color(Color::rgb(90, 90, 90));
        rect.set_outline_color(Color::WHITE);
        rect.set_outline_thickness(1.0);
        window.draw_rectangle_shape(&rect, &RenderStates::DEFAULT);
    }
}

/// Meta dialog about an entry
pub struct Meta {
    uid: Uid,
    close_button: Button,
    add_tag_button: Button,
    tag_buttons: Vec<TagButton>,
    renaming: bool,
    rename_cursor: usize,
    rename_rope: Rope,
}

impl Meta {
    pub fn new(uid: Uid, db: &Db) -> Self {
        Self {
            uid,
            close_button: Button {
                text: "X".into(),
                x: 512.0 - 16.0,
                y: 0.0,
                w: 16,
                h: 16,
            },
            add_tag_button: Button {
                text: "+tag".into(),
                x: 512.0 - 64.0,
                y: 96.0,
                w: 64,
                h: 16,
            },
            tag_buttons: tag_buttons_from_uid(uid, db),
            renaming: false,
            rename_cursor: 0,
            rename_rope: Rope::new(),
        }
    }
}

fn tag_buttons_from_uid(uid: Uid, db: &Db) -> Vec<TagButton> {
    let en = &db.entries[uid as usize];
    let mut buttons = Vec::new();
    for (i, &tag_uid) in en.tags.iter().enumerate() {
        buttons.push(TagButton {
            uid: tag_uid,
            button: Button {
                text: db.tags[tag_uid as usize].names[0].clone(),
                x: (i * 100) as f32,
                y: 300.,
                w: 96,
                h: 16,
            },
        })
    }
    buttons
}

pub enum Msg {
    Nothing,
    PopMe,
    PushNew(Box<dyn Dialog>),
}

impl Dialog for Meta {
    fn draw(
        &self,
        window: &mut RenderWindow,
        x: f32,
        y: f32,
        font: &Font,
        db: &Db,
        thumbnail_cache: &super::ThumbnailCache,
        thumb_size: u32,
        error_texture: &Texture,
        loading_texture: &Texture,
        thumbnail_loader: &mut ThumbnailLoader,
        load_anim_rotation: f32,
    ) {
        self.draw_bg(x, y, window);
        let en = &db.entries[self.uid as usize];
        let path = &en.path;
        let mut text = Text::new(&path.display().to_string(), font, 10);
        text.set_position((x, y));
        window.draw_text(&text, &RenderStates::DEFAULT);
        let mut sprite = Sprite::new();
        super::draw_thumbnail(
            thumbnail_cache,
            db,
            window,
            x,
            y + 12.0,
            self.uid,
            thumb_size,
            &mut sprite,
            font,
            error_texture,
            loading_texture,
            thumbnail_loader,
            load_anim_rotation,
        );
        self.close_button.draw(x, y, font, window);
        self.add_tag_button.draw(x, y, font, window);
        for b in &self.tag_buttons {
            b.button.draw(x, y, font, window);
        }
        if self.renaming {
            text.move_((0.0, 100.0));
            text.set_fill_color(Color::RED);
            let rename_string: String = self.rename_rope.clone().into();
            text.set_string(&rename_string);
            text.set_outline_color(Color::WHITE);
            text.set_outline_thickness(1.0);
            text.set_character_size(16);
            window.draw(&text);
            let mut cursor_shape = RectangleShape::with_size((4., 24.).into());
            cursor_shape.set_fill_color(Color::BLUE);
            let text_bounds = text.global_bounds();
            let offset = calc_cursor_offset(&rename_string, font, self.rename_cursor);
            cursor_shape.set_position((text_bounds.left + offset, text_bounds.top));
            window.draw(&cursor_shape);
        }
    }
    fn size(&self) -> Vector2f {
        Vector2f::new(512., 384.)
    }
    fn handle_event(&mut self, x: u32, y: u32, event: Event, db: &mut Db) -> Msg {
        // kek, hack again
        self.tag_buttons = tag_buttons_from_uid(self.uid, db);
        match event {
            Event::MouseButtonPressed { x: mx, y: my, .. } => {
                if mouse_overlaps_button(x, y, mx, my, &self.close_button) {
                    Msg::PopMe
                } else if mouse_overlaps_button(x, y, mx, my, &self.add_tag_button) {
                    Msg::PushNew(Box::new(AddTagPicker::new(self.uid)))
                } else {
                    Msg::Nothing
                }
            }
            Event::KeyPressed { code, .. } => match code {
                Key::F2 => {
                    let entry = &db.entries[self.uid as usize];
                    let filename = entry.path.file_name().unwrap();
                    self.rename_rope = Rope::from(filename.to_string_lossy());
                    self.renaming = true;
                    self.rename_cursor = String::from(self.rename_rope.clone())
                        .rfind('.')
                        .unwrap_or_else(|| String::from(self.rename_rope.clone()).chars().count());
                    Msg::Nothing
                }
                Key::Return => {
                    if self.renaming {
                        let path = &mut db.entries[self.uid as usize].path;
                        let new_path = path
                            .parent()
                            .unwrap()
                            .join(&String::from(self.rename_rope.clone()));
                        std::fs::rename(&path, &new_path).unwrap();
                        *path = new_path;
                        self.renaming = false;
                    }
                    Msg::Nothing
                }
                Key::Escape => {
                    if self.renaming {
                        self.renaming = false;
                        self.rename_rope = Rope::new();
                        Msg::Nothing
                    } else {
                        Msg::PopMe
                    }
                }
                Key::Right => {
                    if self.renaming
                        && self.rename_cursor
                            < String::from(self.rename_rope.clone()).chars().count()
                    {
                        self.rename_cursor += 1;
                    }
                    Msg::Nothing
                }
                Key::Left => {
                    if self.renaming && self.rename_cursor > 0 {
                        self.rename_cursor -= 1;
                    }
                    Msg::Nothing
                }
                Key::BackSpace => {
                    if self.renaming && self.rename_cursor > 0 {
                        self.rename_rope
                            .remove(self.rename_cursor - 1..self.rename_cursor);
                        self.rename_cursor -= 1;
                    }
                    Msg::Nothing
                }
                Key::Home => {
                    self.rename_cursor = 0;
                    Msg::Nothing
                }
                Key::End => {
                    self.rename_cursor = self.rename_rope.len_chars();
                    Msg::Nothing
                }
                Key::Delete => {
                    if self.rename_rope.len_chars() > 0 {
                        self.rename_rope
                            .remove(self.rename_cursor..self.rename_cursor + 1);
                    }
                    Msg::Nothing
                }
                _ => Msg::Nothing,
            },
            Event::TextEntered { unicode } => {
                if self.renaming && !unicode.is_ascii_control() {
                    self.rename_rope.insert_char(self.rename_cursor, unicode);
                    self.rename_cursor += 1;
                }
                Msg::Nothing
            }
            _ => Msg::Nothing,
        }
    }
}

fn calc_cursor_offset(rename_string: &str, font: &Font, rename_cursor: usize) -> f32 {
    let mut offset = 0.0;
    for ch in rename_string.chars().take(rename_cursor) {
        let glyph = font.glyph(ch as u32, 16, false, 1.0);
        offset += glyph.advance;
    }
    offset
}

fn mouse_overlaps_button(
    dialog_x: u32,
    dialog_y: u32,
    mouse_x: i32,
    mouse_y: i32,
    button: &Button,
) -> bool {
    let button_x = dialog_x + button.x as u32;
    let button_y = dialog_y + button.y as u32;
    let mouse_x = mouse_x as u32;
    let mouse_y = mouse_y as u32;
    mouse_x > button_x
        && mouse_y > button_y
        && mouse_x < button_x + button.w
        && mouse_y < button_y + button.h
}

struct AddTagPicker {
    for_uid: Uid,
    close_button: Button,
    new_tag_button: Button,
    tag_buttons: Vec<TagButton>,
}

impl AddTagPicker {
    fn new(for_uid: Uid) -> Self {
        Self {
            for_uid,
            close_button: Button {
                text: "X".into(),
                x: 300.0 - 16.0,
                y: 0.0,
                w: 16,
                h: 16,
            },
            new_tag_button: Button {
                text: "New".into(),
                x: 0.0,
                y: 0.0,
                w: 48,
                h: 16,
            },
            tag_buttons: Vec::new(),
        }
    }
}

impl Dialog for AddTagPicker {
    fn draw(
        &self,
        window: &mut RenderWindow,
        x: f32,
        y: f32,
        font: &Font,
        _db: &Db,
        _thumbnail_cache: &super::ThumbnailCache,
        _thumb_size: u32,
        _error_texture: &Texture,
        _loading_texture: &Texture,
        _thumbnail_loader: &mut ThumbnailLoader,
        _load_anim_rotation: f32,
    ) {
        self.draw_bg(x, y, window);
        self.new_tag_button.draw(x, y, font, window);
        self.close_button.draw(x, y, font, window);
        for tb in &self.tag_buttons {
            tb.button.draw(x, y, font, window);
        }
    }
    fn size(&self) -> Vector2f {
        Vector2f::new(300., 500.)
    }
    fn handle_event(&mut self, x: u32, y: u32, event: Event, db: &mut Db) -> Msg {
        self.tag_buttons.clear();
        // lel hacky update of buttons on any event
        for (i, t) in db.tags.iter().enumerate() {
            let b = Button {
                text: t.names[0].clone(),
                x: 0.0,
                y: 18.0 + i as f32 * 18.0,
                w: 100,
                h: 16,
            };
            self.tag_buttons.push(TagButton {
                button: b,
                uid: i as Uid,
            });
        }
        if let Event::MouseButtonPressed { x: mx, y: my, .. } = event {
            if mouse_overlaps_button(x, y, mx, my, &self.close_button) {
                return Msg::PopMe;
            } else if mouse_overlaps_button(x, y, mx, my, &self.new_tag_button) {
                return Msg::PushNew(Box::new(LineEdit::new()));
            }
            for b in &self.tag_buttons {
                if mouse_overlaps_button(x, y, mx, my, &b.button) {
                    db.add_tag_for(self.for_uid, b.uid);
                    return Msg::PopMe;
                }
            }
        }
        Msg::Nothing
    }
}

struct LineEdit {
    text: String,
}

impl LineEdit {
    pub fn new() -> Self {
        Self {
            text: String::new(),
        }
    }
}

impl Dialog for LineEdit {
    fn draw(
        &self,
        window: &mut RenderWindow,
        x: f32,
        y: f32,
        font: &Font,
        _db: &Db,
        _thumbnail_cache: &super::ThumbnailCache,
        _thumb_size: u32,
        _error_texture: &Texture,
        _loading_texture: &Texture,
        _thumbnail_loader: &mut ThumbnailLoader,
        _load_anim_rotation: f32,
    ) {
        self.draw_bg(x, y, window);
        let mut text = Text::new(&self.text, font, 24);
        text.set_position((x, y));
        window.draw_text(&text, &RenderStates::DEFAULT);
    }
    fn size(&self) -> Vector2f {
        Vector2f::new(400., 32.)
    }
    fn handle_event(&mut self, _x: u32, _y: u32, event: Event, db: &mut Db) -> Msg {
        match event {
            Event::KeyPressed { code, .. } => match code {
                Key::Return => {
                    db.add_new_tag(Tag {
                        names: vec![self.text.clone()],
                        implies: Vec::new(),
                    });
                    return Msg::PopMe;
                }
                Key::BackSpace => {
                    self.text.pop();
                }
                _ => {}
            },
            Event::TextEntered { unicode } => {
                if !unicode.is_control() {
                    self.text.push(unicode);
                }
            }
            _ => {}
        }
        Msg::Nothing
    }
}
