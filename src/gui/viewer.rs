use egui_sfml::sfml::{
    graphics::{RenderTarget, RenderWindow, Sprite, Texture, Transformable},
    window::{mouse, Event, Key},
    SfBox,
};

use crate::{collection::Collection, db::EntryMap};

use super::{thumbnail_loader::imagebuf_to_sf_tex, Activity, State};

pub(super) fn draw(state: &mut State, window: &mut RenderWindow, coll: &Collection) {
    let id = state.entries_view.uids[state.viewer_state.index];
    let entry = &coll.entries[&id];
    match state.viewer_state.image_cache.entry(id) {
        std::collections::hash_map::Entry::Occupied(en) => {
            let tex = en.get();
            let mut spr = Sprite::with_texture(tex);
            spr.move_((
                state.viewer_state.image_offset.0 as f32,
                state.viewer_state.image_offset.1 as f32,
            ));
            spr.set_scale((state.viewer_state.scale, state.viewer_state.scale));
            window.draw(&spr);
        }
        std::collections::hash_map::Entry::Vacant(en) => {
            let data = std::fs::read(&entry.path).unwrap();
            let img = image::load_from_memory(&data).unwrap();
            let tex = imagebuf_to_sf_tex(img.to_rgba8());
            en.insert(tex);
        }
    }
}

pub(super) fn handle_event(state: &mut State, event: &Event) {
    match *event {
        Event::KeyPressed { code, shift, .. } => match code {
            Key::Left => {
                if state.viewer_state.index == 0 {
                    state.viewer_state.index = state.entries_view.uids.len() - 1;
                } else {
                    state.viewer_state.index -= 1;
                }
                state.viewer_state.reset_view();
            }
            Key::Right => {
                if state.viewer_state.index == state.entries_view.uids.len() - 1 {
                    state.viewer_state.index = 0;
                } else {
                    state.viewer_state.index += 1;
                }
                state.viewer_state.reset_view();
            }
            Key::Escape => state.activity = Activity::Thumbnails,
            Key::Equal if shift => state.viewer_state.scale += 0.1,
            Key::Equal => state.viewer_state.scale = 1.0,
            Key::Hyphen => state.viewer_state.scale -= 0.1,
            _ => {}
        },
        Event::MouseButtonPressed {
            button: mouse::Button::Left,
            x,
            y,
        } => {
            let off = state.viewer_state.image_offset;
            state.viewer_state.grab_origin = Some((off.0 + x, off.1 + y));
        }
        Event::MouseButtonReleased {
            button: mouse::Button::Left,
            ..
        } => {
            state.viewer_state.grab_origin = None;
        }
        Event::MouseMoved { x, y } => {
            if let Some(origin) = state.viewer_state.grab_origin {
                state.viewer_state.image_offset.0 = origin.0 - x;
                state.viewer_state.image_offset.1 = origin.1 - y;
            }
        }
        _ => {}
    }
}

#[derive(Default)]
pub struct ViewerState {
    pub index: usize,
    image_cache: EntryMap<SfBox<Texture>>,
    scale: f32,
    image_offset: (i32, i32),
    grab_origin: Option<(i32, i32)>,
}

impl ViewerState {
    pub fn reset_view(&mut self) {
        self.scale = 1.0;
        self.image_offset = (0, 0);
    }
}
