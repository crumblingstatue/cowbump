use sfml::graphics::{Color, Font, RectangleShape, RenderTarget, RenderWindow, Shape, Sprite, Text,
                     Texture, Transformable};
use sfml::system::{Vector2f, Vector2u};
use sfml::window::Event;
use gui::thumbnail_loader::ThumbnailLoader;
use db::{Db, Uid};

/// A stack of dialogues
#[derive(Default)]
pub struct Stack {
    dialogs: Vec<Box<Dialog>>,
}

impl Stack {
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
            );
        }
    }
    pub fn push(&mut self, dialog: Box<Dialog>) {
        self.dialogs.push(dialog);
    }
    pub fn pop(&mut self) -> Option<Box<Dialog>> {
        self.dialogs.pop()
    }
    pub fn handle_event(&mut self, event: Event) -> bool {
        let mut pop = false;
        let result = match self.dialogs.last_mut() {
            Some(last) => {
                if !last.handle_event(event) {
                    pop = true;
                }
                true
            }
            None => false,
        };
        if pop {
            self.pop();
        }
        result
    }
}

pub trait Dialog {
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
    );
    fn size(&self) -> Vector2f;
    fn handle_event(&mut self, event: Event) -> bool;
}

/// Meta dialog about an entry
pub struct Meta {
    uid: Uid,
}

impl Meta {
    pub fn new(uid: Uid) -> Self {
        Self { uid }
    }
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
    ) {
        let mut rect = RectangleShape::new();
        rect.set_position((x, y));
        rect.set_size(self.size());
        rect.set_fill_color(&Color::rgb(90, 90, 90));
        let path = &db.entries[self.uid as usize].path;
        let mut text = Text::new(&path.display().to_string(), font, 10);
        text.set_position((x, y));
        window.draw(&rect);
        window.draw(&text);
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
        );
    }
    fn size(&self) -> Vector2f {
        Vector2f::new(512., 384.)
    }
    fn handle_event(&mut self, event: Event) -> bool {
        if let Event::MouseButtonPressed { .. } = event {
            false
        } else {
            true
        }
    }
}
