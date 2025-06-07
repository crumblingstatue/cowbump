use {
    super::{Activity, State, resources::Resources, thumbnail_loader::imagebuf_to_sf_tex},
    crate::{collection::Collection, dlog, entry},
    egui_sf2g::{
        egui,
        sf2g::{
            cpp::FBox,
            graphics::{
                RenderStates, RenderTarget, RenderWindow, Sprite, Text, Texture, Transformable,
            },
            window::{Event, Key, mouse},
        },
    },
    std::{collections::VecDeque, time::Instant},
};

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
    match state.viewer_state.image_cache.get(id) {
        Some(result) => match result {
            Ok(tex) => {
                let mut spr = Sprite::with_texture(tex);
                spr.move_((
                    state.viewer_state.image_offset.0 as f32,
                    state.viewer_state.image_offset.1 as f32,
                ));
                spr.set_scale((state.viewer_state.scale, state.viewer_state.scale));
                window.draw_sprite(&spr, &RenderStates::DEFAULT);
            }
            Err(e) => {
                let mut text = Text::new(e.to_string(), &res.font, 20);
                text.tf.position = [200., 200.];
                text.draw(window, &RenderStates::DEFAULT);
            }
        },
        None => {
            let data = match std::fs::read(&entry.path) {
                Ok(data) => data,
                Err(e) => {
                    dlog!("Error loading image: {e}");
                    return;
                }
            };
            match image::load_from_memory(&data) {
                Ok(img) => {
                    let tex = imagebuf_to_sf_tex(img.to_rgba8());
                    state.viewer_state.image_cache.insert((id, Ok(tex)));
                }
                Err(e) => {
                    state
                        .viewer_state
                        .image_cache
                        .insert((id, Err(anyhow::anyhow!(e))));
                }
            }
            state.viewer_state.zoom_to_fit(window);
        }
    }
}

pub(super) fn handle_event(state: &mut State, event: &Event, window: &RenderWindow) {
    match *event {
        Event::KeyPressed { code, shift, .. } => match code {
            Key::Left => state.viewer_state.prev(window),
            Key::Right => state.viewer_state.next(window),
            Key::Escape => state.activity = Activity::Thumbnails,
            Key::Equal if shift => state.viewer_state.zoom_in(),
            Key::Equal => state.viewer_state.original_scale(),
            Key::Hyphen => state.viewer_state.zoom_out(),
            Key::Delete => state.viewer_state.remove_from_view_list(),
            Key::R => state.viewer_state.zoom_to_fit(window),
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

type ImageResult = Result<FBox<Texture>, anyhow::Error>;
type CacheKvPair = (entry::Id, ImageResult);

struct ImageCache {
    img_results: VecDeque<CacheKvPair>,
    capacity: usize,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self {
            img_results: Default::default(),
            capacity: 100,
        }
    }
}

impl ImageCache {
    fn get(&self, id: entry::Id) -> Option<&ImageResult> {
        self.img_results
            .iter()
            .find_map(|kvpair| (kvpair.0 == id).then_some(&kvpair.1))
    }
    fn insert(&mut self, kvpair: CacheKvPair) {
        self.img_results.push_back(kvpair);
        if self.img_results.len() > self.capacity {
            self.img_results.pop_front();
        }
    }
}

#[derive(Default)]
pub struct ViewerState {
    pub index: usize,
    image_cache: ImageCache,
    scale: f32,
    image_offset: (i32, i32),
    grab_origin: Option<(i32, i32)>,
    pub image_list: Vec<entry::Id>,
    pub slideshow_timer_ms: u32,
    pub last_slideshow_instant: Option<Instant>,
}

impl ViewerState {
    pub(in crate::gui) fn shown_entry(&self) -> Option<entry::Id> {
        self.image_list.get(self.index).copied()
    }
    pub(in crate::gui) fn zoom_to_fit(&mut self, window: &RenderWindow) {
        self.scale = 1.0;
        self.image_offset = (0, 0);
        let id = self.image_list[self.index];
        if let Some(Ok(img)) = self.image_cache.get(id) {
            let img_size = img.size();
            let win_size = window.size();
            let x_ratio = win_size.x as f32 / img_size.x as f32;
            let y_ratio = win_size.y as f32 / img_size.y as f32;
            let min_ratio = f32::min(x_ratio, y_ratio);
            if min_ratio < 1.0 {
                self.scale = min_ratio;
            }
        }
    }
    pub(in crate::gui) fn remove_from_view_list(&mut self) {
        self.image_list.remove(self.index);
        self.index = self.index.saturating_sub(1);
    }

    pub(in crate::gui) fn zoom_out(&mut self) {
        self.scale -= 0.1;
    }

    pub(in crate::gui) fn original_scale(&mut self) {
        self.scale = 1.0;
    }

    pub(in crate::gui) fn zoom_in(&mut self) {
        self.scale += 0.1;
    }

    pub(in crate::gui) fn next(&mut self, window: &RenderWindow) {
        if self.index == self.image_list.len() - 1 {
            self.index = 0;
        } else {
            self.index += 1;
        }
        self.zoom_to_fit(window);
    }

    pub(in crate::gui) fn prev(&mut self, window: &RenderWindow) {
        if self.index == 0 {
            self.index = self.image_list.len() - 1;
        } else {
            self.index -= 1;
        }
        self.zoom_to_fit(window);
    }
}

pub fn menu_ui(ui: &mut egui::Ui, state: &mut State, win: &RenderWindow) {
    if ui.button("Back (Esc)").clicked() {
        state.activity = Activity::Thumbnails;
    }
    ui.menu_button("Viewer", |ui| {
        if ui.button("Previous (<-)").clicked() {
            state.viewer_state.prev(win);
        }
        if ui.button("Next (->)").clicked() {
            state.viewer_state.next(win);
        }
        if ui.button("Zoom out (-)").clicked() {
            state.viewer_state.zoom_out();
        }
        if ui.button("Original zoom (=)").clicked() {
            state.viewer_state.original_scale();
        }
        if ui.button("Zoom in (+)").clicked() {
            state.viewer_state.zoom_in();
        }
        if ui.button("Zoom to fit (R)").clicked() {
            state.viewer_state.zoom_to_fit(win);
        }
        ui.separator();
        if ui.button("Remove from view list (Del)").clicked() {
            state.viewer_state.remove_from_view_list();
        }
        ui.label("Slideshow timer");
        ui.add(egui::DragValue::new(
            &mut state.viewer_state.slideshow_timer_ms,
        ));
    });
}

pub(crate) fn update(state: &mut State, win: &RenderWindow) {
    let timer = state.viewer_state.slideshow_timer_ms;
    if timer != 0 {
        let last = state
            .viewer_state
            .last_slideshow_instant
            .get_or_insert(Instant::now());
        if last.elapsed().as_millis() >= u128::from(state.viewer_state.slideshow_timer_ms) {
            state.viewer_state.next(win);
            state.viewer_state.last_slideshow_instant = Some(Instant::now());
        }
    }
}
