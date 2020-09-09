use ropey::Rope;
use sfml::graphics::{Font, RectangleShape, RenderTarget, RenderWindow, Text, Transformable};
use sfml::window::Key;
use std::borrow::Cow;

#[derive(Default)]
pub struct TextEdit {
    rope: Rope,
    cursor: usize,
}

impl TextEdit {
    pub fn string(&self) -> Cow<str> {
        self.rope.clone().into()
    }
    pub fn set_string(&mut self, string: Cow<str>) {
        self.rope = Rope::from(string);
    }
    pub fn clear(&mut self) {
        self.rope = Rope::new();
        self.cursor = 0;
    }
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.rope.remove(self.cursor - 1..self.cursor);
            self.cursor -= 1;
        }
    }
    pub fn delete(&mut self) {
        if self.rope.len_chars() > 0 {
            self.rope.remove(self.cursor..self.cursor + 1);
        }
    }
    pub fn type_(&mut self, ch: char) {
        if !ch.is_ascii_control() {
            self.rope.insert_char(self.cursor, ch);
            self.cursor += 1;
        }
    }
    pub fn left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }
    pub fn right(&mut self) {
        if self.cursor < self.rope.len_chars() {
            self.cursor += 1;
        }
    }
    pub fn home(&mut self) {
        self.cursor = 0;
    }
    pub fn end(&mut self) {
        self.cursor = self.rope.len_chars();
    }
    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor = pos;
    }
    pub fn handle_sfml_key(&mut self, key: Key) {
        match key {
            Key::Right => {
                self.right();
            }
            Key::Left => {
                self.left();
            }
            Key::BackSpace => {
                self.backspace();
            }
            Key::Home => {
                self.home();
            }
            Key::End => {
                self.end();
            }
            Key::Delete => {
                self.delete();
            }
            _ => {}
        }
    }
    pub fn draw_sfml(
        &self,
        window: &mut RenderWindow,
        font: &Font,
        text: &mut Text,
        cursor: &mut RectangleShape,
    ) {
        cursor.set_size((2.0, text.character_size() as f32));
        let rename_string = self.string();
        text.set_string(&*rename_string);
        let offset = calc_cursor_offset(&rename_string, font, self.cursor, text.character_size());
        let text_bounds = text.global_bounds();
        cursor.set_position((text_bounds.left + offset, text_bounds.top));
        window.draw(text);
        window.draw(cursor);
    }
    /// Returns char position of the desired char, if it's found
    pub fn rfind(&self, c: char) -> Option<usize> {
        let s: Cow<str> = self.rope.clone().into();
        s.rfind(c).map(|pos| self.rope.byte_to_char(pos))
    }
}

fn calc_cursor_offset(
    rename_string: &str,
    font: &Font,
    rename_cursor: usize,
    character_size: u32,
) -> f32 {
    let mut offset = 0.0;
    for ch in rename_string.chars().take(rename_cursor) {
        let glyph = font.glyph(ch as u32, character_size, false, 1.0);
        offset += glyph.advance;
    }
    offset
}
