use egui_sfml::sfml::{
    graphics::{
        Color, Rect, RectangleShape, RenderStates, RenderTarget, RenderWindow, Shape, Sprite, Text,
        Transformable,
    },
    system::Vector2f,
    window::Key,
};

use crate::{collection::Collection, entry, filter_reqs::Requirements};

use super::{
    get_tex_for_entry, thumbnail_loader::ThumbnailLoader, Resources, State, ThumbnailCache,
};

pub struct EntriesView {
    pub y_offset: f32,
    pub sort_by: SortBy,
    uids: Vec<entry::Id>,
}

impl Default for EntriesView {
    fn default() -> Self {
        Self {
            y_offset: Default::default(),
            sort_by: SortBy::Path,
            uids: Default::default(),
        }
    }
}

pub enum SortBy {
    Path,
    Id,
}

impl EntriesView {
    pub fn from_collection(coll: &Collection, reqs: &Requirements) -> Self {
        let mut this = Self {
            uids: Vec::new(),
            y_offset: 0.0,
            sort_by: SortBy::Path,
        };
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
        self.uids.iter().cloned()
    }
    pub fn entry_position(&self, id: entry::Id) -> Option<usize> {
        self.iter().position(|id2| id2 == id)
    }
    pub fn get(&self, index: usize) -> Option<entry::Id> {
        self.uids.get(index).copied()
    }
}

fn thumbs_skip_take(state: &State, window_height: u32) -> (usize, usize) {
    let thumb_size = state.thumbnail_size;
    let mut thumbnails_per_column = (window_height / thumb_size) as u8;
    // Compensate for truncating division
    if window_height % thumb_size != 0 {
        thumbnails_per_column += 1;
    }
    // Since we can scroll, we can have another partially drawn frame per screen
    thumbnails_per_column += 1;
    let thumbnails_per_screen = (state.thumbnails_per_row * thumbnails_per_column) as usize;
    let row_offset = state.entries_view.y_offset as u32 / thumb_size;
    let skip = row_offset * state.thumbnails_per_row as u32;
    (skip as usize, thumbnails_per_screen)
}

pub(super) fn draw_thumbnails(
    state: &mut State,
    res: &Resources,
    window: &mut RenderWindow,
    coll: &Collection,
    load_anim_rotation: f32,
    pointer_active: bool,
) {
    let mouse_pos = window.mouse_position();
    let thumb_size = state.thumbnail_size;
    state
        .thumbnail_loader
        .write_to_cache(&mut state.thumbnail_cache);
    let mut sprite = Sprite::new();
    let (skip, take) = thumbs_skip_take(state, window.size().y);
    for (rel_idx, (abs_idx, uid)) in state
        .entries_view
        .iter()
        .enumerate()
        .skip(skip)
        .take(take)
        .enumerate()
    {
        let column = (rel_idx as u32) % state.thumbnails_per_row as u32;
        let row = (rel_idx as u32) / state.thumbnails_per_row as u32;
        let x = (column * thumb_size) as f32;
        let y = (row * thumb_size) as f32 - (state.entries_view.y_offset % thumb_size as f32);
        let image_rect = Rect::new(x, y, thumb_size as f32, thumb_size as f32);
        let mouse_over = image_rect.contains(Vector2f::new(mouse_pos.x as f32, mouse_pos.y as f32));
        if state.selected_uids.contains(&uid) {
            sprite.set_color(Color::GREEN);
        } else {
            sprite.set_color(Color::WHITE);
        }
        draw_thumbnail(
            &state.thumbnail_cache,
            coll,
            window,
            x,
            y,
            uid,
            thumb_size,
            &mut sprite,
            res,
            &mut state.thumbnail_loader,
            load_anim_rotation,
        );
        if mouse_over && pointer_active {
            let mut rs = RectangleShape::from_rect(image_rect);
            rs.set_fill_color(Color::rgba(225, 225, 200, 48));
            rs.set_outline_color(Color::rgb(200, 200, 0));
            rs.set_outline_thickness(-2.0);
            window.draw(&rs);
        }
        if let Some(idx) = state.select_begin && idx == abs_idx {
            let mut s = Sprite::with_texture(&res.sel_begin_texture);
            s.set_position((x, y));
            window.draw(&s);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_thumbnail<'a: 'b, 'b>(
    thumbnail_cache: &'a ThumbnailCache,
    coll: &Collection,
    window: &mut RenderWindow,
    x: f32,
    y: f32,
    id: entry::Id,
    thumb_size: u32,
    sprite: &mut Sprite<'b>,
    res: &'a Resources,
    thumbnail_loader: &mut ThumbnailLoader,
    load_anim_rotation: f32,
) {
    let (has_img, texture) =
        get_tex_for_entry(thumbnail_cache, id, coll, thumbnail_loader, thumb_size, res);
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
        if let Some(path_string) = coll.entries[&id].path.to_str() {
            let mut text = Text::new(path_string, &res.font, 12);
            text.set_position(fname_pos);
            window.draw_text(&text, &RenderStates::DEFAULT);
        }
    }
}
