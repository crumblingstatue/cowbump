use {
    super::{
        egui_ui::EguiState,
        get_tex_for_entry,
        open::{builtin, external},
        resources::Resources,
        thumbnail_loader::ThumbnailLoader,
        State, ThumbnailCache,
    },
    crate::{
        collection::{Collection, Entries},
        dlog, entry,
        filter_reqs::Requirements,
        preferences::Preferences,
    },
    anyhow::Context as _,
    egui_sfml::{
        egui::Context,
        sfml::{
            graphics::{
                Color, Rect, RectangleShape, RenderStates, RenderTarget, RenderWindow, Shape,
                Sprite, Text, Transformable,
            },
            system::Vector2f,
            window::{mouse, Event, Key},
        },
    },
};

pub struct ThumbnailsView {
    pub thumbs_per_row: u8,
    pub thumb_size: u32,
    pub y_offset: f32,
    pub sort_by: SortBy,
    pub uids: Vec<entry::Id>,
    pub highlight: Option<u32>,
}

pub enum SortBy {
    Path,
    Id,
}

fn thumbs_per_row_and_size(window_width: u32, preferences: &Preferences) -> (u8, u32) {
    let thumbnails_per_row = preferences.thumbs_per_row;
    let thumbnail_size = window_width / u32::from(thumbnails_per_row);
    (thumbnails_per_row, thumbnail_size)
}

impl ThumbnailsView {
    pub fn new(window_width: u32, preferences: &Preferences) -> Self {
        let (thumbs_per_row, thumb_size) = thumbs_per_row_and_size(window_width, preferences);
        Self {
            y_offset: Default::default(),
            sort_by: SortBy::Path,
            uids: Default::default(),
            thumb_size,
            thumbs_per_row,
            highlight: None,
        }
    }
    pub fn resize(&mut self, window_width: u32, preferences: &Preferences) {
        (self.thumbs_per_row, self.thumb_size) = thumbs_per_row_and_size(window_width, preferences);
    }
    pub fn from_collection(
        window_width: u32,
        coll: &Collection,
        reqs: &Requirements,
        preferences: &Preferences,
    ) -> Self {
        let mut this = Self::new(window_width, preferences);
        this.update_from_collection(coll, reqs);
        this
    }
    pub fn update_from_collection(&mut self, coll: &Collection, reqs: &Requirements) {
        self.uids = coll.filter(reqs).collect();
        self.sort(coll);
    }
    fn sort(&mut self, coll: &Collection) {
        match self.sort_by {
            SortBy::Id => self.uids.sort_by_key(|uid| uid.0),
            SortBy::Path => self.uids.sort_by_key(|uid| &coll.entries[uid].path),
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = entry::Id> + '_ {
        self.uids.iter().copied()
    }
    pub fn entry_position(&self, id: entry::Id) -> Option<usize> {
        self.iter().position(|id2| id2 == id)
    }
    pub fn get(&self, index: usize) -> Option<entry::Id> {
        self.uids.get(index).copied()
    }
    fn skip_take(&self, window_height: u32) -> (usize, usize) {
        let thumb_size = self.thumb_size;
        let mut thumbnails_per_column = (window_height / thumb_size) as u8;
        // Compensate for truncating division
        if window_height % thumb_size != 0 {
            thumbnails_per_column += 1;
        }
        // Since we can scroll, we can have another partially drawn frame per screen
        thumbnails_per_column += 1;
        let thumbnails_per_screen =
            usize::from(self.thumbs_per_row) * usize::from(thumbnails_per_column);
        let row_offset = self.y_offset as u32 / thumb_size;
        let skip = row_offset * u32::from(self.thumbs_per_row);
        (skip as usize, thumbnails_per_screen)
    }
    fn find_bottom(&self, window: &RenderWindow) -> f32 {
        let n_pics = self.iter().count();
        let mut rows = n_pics as u32 / u32::from(self.thumbs_per_row);
        if n_pics as u32 % u32::from(self.thumbs_per_row) != 0 {
            rows += 1;
        }
        let bottom = rows * self.thumb_size;
        let mut b = bottom as f32 - window.size().y as f32;
        if b < 0. {
            b = 0.;
        }
        b
    }
    fn clamp_top(&mut self) {
        if self.y_offset < 0.0 {
            self.y_offset = 0.0;
        }
    }
    pub fn clamp_bottom(&mut self, window: &RenderWindow) {
        let bottom = self.find_bottom(window);
        if self.y_offset > bottom {
            self.y_offset = bottom;
        }
    }
    fn go_to_bottom(&mut self, window: &RenderWindow) {
        self.y_offset = self.find_bottom(window);
    }
    /// Returns the absolute thumb index at (x,y) on the screen
    ///
    /// This is absolute, so the top left image on the screen could have a different index
    /// based on the scroll y offset
    fn abs_thumb_index_at_xy(&self, x: i32, y: i32) -> usize {
        let thumb_x = x as u32 / self.thumb_size;
        let thumb_y = (y as u32 + self.y_offset as u32) / self.thumb_size;
        let thumb_index = thumb_y * u32::from(self.thumbs_per_row) + thumb_x;
        thumb_index as usize
    }
    fn entry_at_xy(&self, x: i32, y: i32) -> Option<entry::Id> {
        let thumb_index = self.abs_thumb_index_at_xy(x, y);
        self.get(thumb_index)
    }
    /// Calculate absolute pixel position of an item at `index`
    pub fn item_position(&self, index: u32) -> (u32, u32) {
        let thumbs_per_row: u32 = self.thumbs_per_row.into();
        let row = index / thumbs_per_row;
        let pixel_y = row * self.thumb_size;
        let col = index % thumbs_per_row;
        let pixel_x = col * self.thumb_size;
        (pixel_x, pixel_y)
    }
    pub fn seek_to_contain_index(&mut self, index: usize, height: u32) {
        let (_x, y) = self.item_position(index as u32);
        let view_y = &mut self.y_offset;
        let thumb_size = self.thumb_size;
        if y < (*view_y as u32) {
            let diff = (*view_y as u32) - y;
            *view_y -= diff as f32;
        }
        if y + thumb_size > (*view_y as u32 + height) {
            let diff = (y + thumb_size) - (*view_y as u32 + height);
            *view_y += diff as f32;
        }
    }
    pub fn highlight_and_seek_to_entry(&mut self, id: entry::Id, height: u32) -> bool {
        match self.entry_position(id) {
            Some(idx) => {
                self.highlight = Some(idx as u32);
                self.seek_to_contain_index(idx, height);
                true
            }
            None => false,
        }
    }
}

pub(super) fn draw_thumbnails(
    state: &mut State,
    res: &Resources,
    window: &mut RenderWindow,
    entries: &Entries,
    load_anim_rotation: f32,
    pointer_active: bool,
) {
    let mouse_pos = window.mouse_position();
    let thumb_size = state.thumbs_view.thumb_size;
    state
        .thumbnail_loader
        .write_to_cache(&mut state.thumbnail_cache);
    let mut sprite = Sprite::new();
    let (skip, take) = state.thumbs_view.skip_take(window.size().y);
    for (rel_idx, (abs_idx, uid)) in state
        .thumbs_view
        .iter()
        .enumerate()
        .skip(skip)
        .take(take)
        .enumerate()
    {
        let column = (rel_idx as u32) % u32::from(state.thumbs_view.thumbs_per_row);
        let row = (rel_idx as u32) / u32::from(state.thumbs_view.thumbs_per_row);
        let x = (column * thumb_size) as f32;
        let y = (row * thumb_size) as f32 - (state.thumbs_view.y_offset % thumb_size as f32);
        let image_rect = Rect::new(x, y, thumb_size as f32, thumb_size as f32);
        let mouse_over = image_rect.contains(Vector2f::new(mouse_pos.x as f32, mouse_pos.y as f32));
        if state.sel.current_contains(&uid) {
            sprite.set_color(Color::GREEN);
        } else {
            sprite.set_color(Color::WHITE);
        }
        draw_thumbnail(
            &state.thumbnail_cache,
            entries,
            window,
            x,
            y,
            uid,
            thumb_size,
            &mut sprite,
            res,
            &state.thumbnail_loader,
            load_anim_rotation,
        );
        if mouse_over && pointer_active {
            let mut rs = RectangleShape::from_rect(image_rect);
            rs.set_fill_color(Color::rgba(225, 225, 200, 48));
            rs.set_outline_color(Color::rgb(200, 200, 0));
            rs.set_outline_thickness(-2.0);
            window.draw(&rs);
        }
        if let Some(idx) = state.select_a
            && idx == abs_idx
        {
            let mut s = Sprite::with_texture(&res.sel_begin_texture);
            s.set_position((x, y));
            window.draw(&s);
        }
    }
}

#[expect(clippy::too_many_arguments)]
fn draw_thumbnail<'a: 'b, 'b>(
    thumbnail_cache: &'a ThumbnailCache,
    entries: &Entries,
    window: &mut RenderWindow,
    x: f32,
    y: f32,
    id: entry::Id,
    thumb_size: u32,
    sprite: &mut Sprite<'b>,
    res: &'a Resources,
    thumbnail_loader: &ThumbnailLoader,
    load_anim_rotation: f32,
) {
    let (has_img, texture) = get_tex_for_entry(
        thumbnail_cache,
        id,
        entries,
        thumbnail_loader,
        thumb_size,
        res,
    );
    sprite.set_texture(texture, true);
    sprite.set_position((x, y));
    if thumbnail_loader.busy_with().contains(&id) {
        sprite.set_origin((27.0, 6.0));
        sprite.move_((48.0, 48.0));
        sprite.set_rotation(load_anim_rotation);
    } else {
        sprite.set_rotation(0.0);
        sprite.set_origin((0.0, 0.0));
    }
    window.draw_sprite(sprite, &RenderStates::DEFAULT);
    let mut show_filename = !has_img;
    let fname_pos = (x, y + 64.0);
    if Key::LAlt.is_pressed() {
        show_filename = true;
        let mut rect = RectangleShape::new();
        rect.set_fill_color(Color::rgba(0, 0, 0, 128));
        rect.set_size((380., 24.));
        rect.set_position(fname_pos);
        window.draw(&rect);
    }
    if show_filename {
        if let Some(path_string) = entries[&id].path.to_str() {
            let mut text = Text::new(path_string, &res.font, 12);
            text.set_position(fname_pos);
            window.draw_text(&text, &RenderStates::DEFAULT);
        }
    }
}

pub(in crate::gui) fn handle_event(
    event: Event,
    state: &mut State,
    egui_state: &mut EguiState,
    coll: &Collection,
    window: &RenderWindow,
    egui_ctx: &Context,
    preferences: &mut Preferences,
) {
    match event {
        Event::MouseButtonPressed { button, x, y } => {
            if egui_ctx.wants_pointer_input() {
                return;
            }
            let Some(uid) = state.thumbs_view.entry_at_xy(x, y) else {
                return;
            };
            if button == mouse::Button::Left {
                if Key::LShift.is_pressed() {
                    let Some(sel_buf) = state.sel.current_mut() else {
                        egui_state.modal.err("Selection buffer is not accessible");
                        return;
                    };
                    if sel_buf.contains(&uid) {
                        sel_buf.buf.retain(|&rhs| rhs != uid);
                    } else {
                        sel_buf.buf.push(uid);
                    }
                } else if Key::LControl.is_pressed() {
                    let curr_thumb_idx = state.thumbs_view.abs_thumb_index_at_xy(x, y);
                    match state.select_a {
                        Some(a) => {
                            let Some(sel_buf) = state.sel.current_mut() else {
                                egui_state.modal.err("Selection buffer is not accessible");
                                return;
                            };
                            let (min, max) = (a.min(curr_thumb_idx), a.max(curr_thumb_idx));
                            for id in state.thumbs_view.iter().skip(min).take((max + 1) - min) {
                                sel_buf.buf.push(id);
                            }
                            state.select_a = None;
                        }
                        None => state.select_a = Some(curr_thumb_idx),
                    }
                } else if preferences.use_built_in_viewer {
                    if let Some(id) = state.thumbs_view.entry_at_xy(x, y) {
                        builtin::open_single_with_others(
                            id,
                            coll,
                            state,
                            window,
                            state.thumbs_view.abs_thumb_index_at_xy(x, y),
                        );
                    }
                } else {
                    external::open_single_with_others(coll, uid, preferences);
                }
            } else if button == mouse::Button::Right {
                egui_state.add_entries_window(
                    state
                        .sel
                        .current_as_nonempty_id_vec()
                        .map_or_else(|| vec![uid], Vec::to_owned),
                );
            }
        }
        Event::KeyPressed { code, ctrl, .. } => {
            if egui_ctx.wants_keyboard_input()
                || egui_state.file_dialog.state() == egui_file_dialog::DialogState::Open
            {
                return;
            }
            if code == Key::PageDown {
                state.thumbs_view.y_offset += window.size().y as f32;
                state.thumbs_view.clamp_bottom(window);
            } else if code == Key::PageUp {
                state.thumbs_view.y_offset -= window.size().y as f32;
                state.thumbs_view.clamp_top();
            } else if code == Key::Enter {
                if preferences.use_built_in_viewer {
                    builtin::on_enter_open(state, window);
                } else {
                    if let Err(e) = external::on_enter_open(state, coll, preferences) {
                        egui_state.modal.err(format!("Failed to open file(s): {e}"));
                    }
                }
            } else if code == Key::A && ctrl {
                select_all(state, coll);
            } else if code == Key::Slash {
                egui_state.find_popup.on = true;
            } else if code == Key::N {
                search_next(state, coll, window.size().y);
            } else if code == Key::P {
                search_prev(state, coll, window.size().y);
            } else if code == Key::F {
                egui_state.filter_popup.on = true;
            } else if code == Key::C {
                let mp = window.mouse_position();
                let Some(uid) = state.thumbs_view.entry_at_xy(mp.x, mp.y) else {
                    return;
                };
                if let Err(e) = copy_image_to_clipboard(state, coll, uid) {
                    egui_state.modal.err(format!("Clipboard copy failed: {e}"));
                }
            } else if code == Key::T {
                egui_state.tag_window.toggle();
            } else if code == Key::Q {
                egui_state.sequences_window.on ^= true;
            } else if code == Key::S {
                state
                    .thumbs_view
                    .update_from_collection(coll, &state.filter);
            } else if code == Key::Home {
                if !egui_ctx.wants_keyboard_input() {
                    state.thumbs_view.y_offset = 0.0;
                }
            } else if code == Key::End && !egui_ctx.wants_keyboard_input() {
                // Align the bottom edge of the view with the bottom edge of the last row.
                // To do align the camera with a bottom edge, we need to subtract the screen
                // height from it.
                state.thumbs_view.go_to_bottom(window);
            } else if code == Key::F2
                && let Some(id_vec) = state.sel.current_as_nonempty_id_vec()
            {
                egui_state.add_entries_window(id_vec.clone());
            } else if code == Key::Escape
                && !egui_ctx.wants_keyboard_input()
                && !egui_ctx.wants_pointer_input()
                && !egui_state.just_closed_window_with_esc
            {
                state.sel.clear_current();
            }
        }
        Event::MouseWheelScrolled { delta, .. } => {
            state.thumbs_view.y_offset -= delta * preferences.scroll_wheel_multiplier;
            if delta > 0.0 {
                state.thumbs_view.clamp_top();
            } else {
                state.thumbs_view.clamp_bottom(window);
            }
        }
        _ => {}
    }
}

fn copy_image_to_clipboard(
    state: &mut State,
    coll: &Collection,
    uid: entry::Id,
) -> anyhow::Result<()> {
    use arboard::ImageData;
    let imgpath = &coll.entries[&uid].path;
    let buf = std::fs::read(imgpath)?;
    let img = image::load_from_memory(&buf).context("Failed to load image from memory")?;
    let rgba = img.to_rgba8();
    let img_data = ImageData {
        width: rgba.width() as usize,
        height: rgba.height() as usize,
        bytes: rgba.into_raw().into(),
    };
    state
        .clipboard_ctx
        .set_image(img_data)
        .context("Failed to copy to clipboard")
}

pub(in crate::gui) fn select_all(state: &mut State, coll: &Collection) {
    let Some(buf) = state.sel.current_mut() else {
        dlog!("Current selection buffer inacessible");
        return;
    };
    buf.clear();
    for uid in coll.filter(&state.filter) {
        buf.buf.push(uid);
    }
}

pub(in crate::gui) fn search_prev(state: &mut State, coll: &Collection, view_height: u32) {
    if state.search_cursor > 0 {
        state.search_cursor -= 1;
    }
    search_goto_cursor(state, coll, view_height);
}

pub(in crate::gui) fn search_next(state: &mut State, coll: &Collection, view_height: u32) {
    state.search_cursor += 1;
    search_goto_cursor(state, coll, view_height);
    if !state.search_success {
        state.search_cursor -= 1;
    }
}

fn find_nth(state: &State, coll: &Collection, nth: usize) -> Option<usize> {
    state
        .thumbs_view
        .iter()
        .enumerate()
        .filter(|(_, uid)| {
            let en = &coll.entries[uid];
            en.all_reqs_satisfied(*uid, &state.find_reqs, &coll.tags, &coll.sequences)
        })
        .map(|(i, _)| i)
        .nth(nth)
}

pub(in crate::gui) fn search_goto_cursor(state: &mut State, coll: &Collection, view_height: u32) {
    if let Some(index) = find_nth(state, coll, state.search_cursor) {
        state.thumbs_view.highlight = Some(index as u32);
        state.search_success = true;
        state.thumbs_view.seek_to_contain_index(index, view_height);
    } else {
        state.search_success = false;
    }
}
