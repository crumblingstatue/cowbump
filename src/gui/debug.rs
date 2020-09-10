use lazy_static::lazy_static;
use sfml::graphics::{Color, Font, RenderTarget, RenderWindow, Text, Transformable};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

lazy_static! {
    static ref INFOS: Mutex<Vec<String>> = Mutex::new(Vec::new());
}

static ENABLED: AtomicBool = AtomicBool::new(false);

/// Add an info bit to draw
#[allow(dead_code)]
pub fn info(msg: String) {
    if ENABLED.load(Ordering::Acquire) {
        INFOS.lock().unwrap().push(msg);
    }
}

/// Draw all info bits, then clear
pub fn draw(rw: &mut RenderWindow, font: &Font) {
    if ENABLED.load(Ordering::Acquire) {
        let mut infos = INFOS.lock().unwrap();
        let mut text = Text::new("", font, 16);
        text.set_fill_color(Color::RED);
        let mut y = 0.0;
        for info in infos.iter() {
            text.set_string(info);
            text.set_position((0.0, y));
            rw.draw(&text);
            y += 17.0;
        }
        infos.clear();
    }
}

pub fn toggle() {
    let current = ENABLED.load(Ordering::Acquire);
    ENABLED.store(!current, Ordering::Release);
}
