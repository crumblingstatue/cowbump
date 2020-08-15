use ropey::Rope;
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
    pub fn cursor(&self) -> usize {
        self.cursor
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
}
