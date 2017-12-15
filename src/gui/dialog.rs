use sfml::graphics::{Color, Font, RectangleShape, RenderTarget, RenderWindow, Shape, Text,
                     Transformable};
use sfml::system::{Vector2f, Vector2u};
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
            );
        }
    }
    pub fn push(&mut self, dialog: Box<Dialog>) {
        self.dialogs.push(dialog);
    }
    pub fn pop(&mut self) -> Option<Box<Dialog>> {
        self.dialogs.pop()
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
    );
    fn size(&self) -> Vector2f;
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
    ) {
        let mut rect = RectangleShape::new();
        rect.set_position((x, y));
        rect.set_size(self.size());
        rect.set_fill_color(&Color::rgb(90, 90, 90));
        let path = &db.entries[self.uid as usize].path;
        let mut text = Text::new(&path.display().to_string(), font, 12);
        text.set_position((x, y));
        window.draw(&rect);
        window.draw(&text);
    }
    fn size(&self) -> Vector2f {
        Vector2f::new(512., 384.)
    }
}
