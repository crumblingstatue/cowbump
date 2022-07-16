use egui_sfml::sfml::{
    graphics::{RenderTarget, RenderWindow, Sprite, Text, Texture, Transformable},
    window::{mouse, Event, Key},
    SfBox,
};

use crate::{collection::Collection, db::EntryMap, entry};

use super::{resources::Resources, thumbnail_loader::imagebuf_to_sf_tex, Activity, State};

pub(super) fn draw(
    state: &mut State,
    window: &mut RenderWindow,
    coll: &Collection,
    res: &Resources,
) {
    if state.viewer_state.image_list.is_empty() {
        state.activity = Activity::Thumbnails;
        return;
    }
    let id = state.viewer_state.image_list[state.viewer_state.index];
    let entry = &coll.entries[&id];
    match state.viewer_state.image_cache.entry(id) {
        std::collections::hash_map::Entry::Occupied(en) => match en.get() {
            Ok(tex) => {
                let mut spr = Sprite::with_texture(tex);
                spr.move_((
                    state.viewer_state.image_offset.0 as f32,
                    state.viewer_state.image_offset.1 as f32,
                ));
                spr.set_scale((state.viewer_state.scale, state.viewer_state.scale));
                window.draw(&spr);
            }
            Err(e) => {
                let mut text = Text::new(&e.to_string(), &res.font, 20);
                text.set_position((200., 200.));
                window.draw(&text);
            }
        },
        std::collections::hash_map::Entry::Vacant(en) => {
            let data = std::fs::read(&entry.path).unwrap();
            match image::load_from_memory(&data) {
                Ok(img) => {
                    let tex = imagebuf_to_sf_tex(img.to_rgba8());
                    en.insert(Ok(tex));
                }
                Err(e) => {
                    en.insert(Err(anyhow::anyhow!(e)));
                }
            }
            state.viewer_state.reset_view(window);
        }
    }
}

pub(super) fn handle_event(state: &mut State, event: &Event, window: &mut RenderWindow) {
    match *event {
        Event::KeyPressed { code, shift, .. } => match code {
            Key::Left => state.viewer_state.prev(window),
            Key::Right => state.viewer_state.next(window),
            Key::Escape => state.activity = Activity::Thumbnails,
            Key::Equal if shift => state.viewer_state.zoom_in(),
            Key::Equal => state.viewer_state.original_scale(),
            Key::Hyphen => state.viewer_state.zoom_out(),
            Key::Delete => state.viewer_state.remove_from_view_list(),
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
    image_cache: EntryMap<Result<SfBox<Texture>, anyhow::Error>>,
    scale: f32,
    image_offset: (i32, i32),
    grab_origin: Option<(i32, i32)>,
    pub image_list: Vec<entry::Id>,
}

impl ViewerState {
    pub fn reset_view(&mut self, window: &RenderWindow) {
        self.scale = 1.0;
        self.image_offset = (0, 0);
        let id = self.image_list[self.index];
        if let Some(Ok(img)) = self.image_cache.get(&id) {
            let img_size = img.size();
            let win_size = window.size();
            if img_size.y > win_size.y {
                self.scale = win_size.y as f32 / img_size.y as f32;
            }
        }
    }
    pub(in crate::gui) fn remove_from_view_list(&mut self) {
        self.image_list.remove(self.index);
        self.index = self.index.saturating_sub(1);
    }

    pub(in crate::gui) fn zoom_out(&mut self) {
        self.scale -= 0.1
    }

    pub(in crate::gui) fn original_scale(&mut self) {
        self.scale = 1.0
    }

    pub(in crate::gui) fn zoom_in(&mut self) {
        self.scale += 0.1
    }

    pub(in crate::gui) fn next(&mut self, window: &RenderWindow) {
        if self.index == self.image_list.len() - 1 {
            self.index = 0;
        } else {
            self.index += 1;
        }
        self.reset_view(window);
    }

    pub(in crate::gui) fn prev(&mut self, window: &RenderWindow) {
        if self.index == 0 {
            self.index = self.image_list.len() - 1;
        } else {
            self.index -= 1;
        }
        self.reset_view(window);
    }
}
