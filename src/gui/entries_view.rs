use sfml::{
    graphics::{
        Color, Rect, RectangleShape, RenderStates, RenderTarget, RenderWindow, Shape, Sprite, Text,
        Transformable,
    },
    system::Vector2f,
    window::Key,
};

use crate::{db::local::LocalDb, entry, filter_spec::FilterSpec};

use super::{
    get_tex_for_entry, thumbnail_loader::ThumbnailLoader, Resources, State, ThumbnailCache,
};

pub struct EntriesView {
    uids: Vec<entry::Id>,
}

impl EntriesView {
    pub fn from_db(db: &LocalDb) -> Self {
        let uids: Vec<entry::Id> = db.entries.keys().cloned().collect();
        let mut this = Self { uids };
        this.sort(db);
        this
    }
    pub fn sort(&mut self, db: &LocalDb) {
        self.uids.sort_by_key(|uid| &db.entries[uid].path);
    }
    pub fn filter<'a>(
        &'a self,
        db: &'a LocalDb,
        spec: &'a FilterSpec,
    ) -> impl Iterator<Item = entry::Id> + 'a {
        self.uids
            .iter()
            .filter_map(|uid| crate::entry::filter_map(*uid, &db.entries[uid], spec))
    }
    /// Delete `uid` from the list.
    pub fn delete(&mut self, uid: entry::Id) {
        self.uids.retain(|&rhs| uid != rhs);
    }
}

pub(super) fn draw_thumbnails(
    state: &mut State,
    res: &Resources,
    window: &mut RenderWindow,
    db: &LocalDb,
    uids: &[entry::Id],
    selected_uids: &[entry::Id],
    load_anim_rotation: f32,
    pointer_active: bool,
) {
    let mouse_pos = window.mouse_position();
    let thumb_size = state.thumbnail_size;
    state
        .thumbnail_loader
        .write_to_cache(&mut state.thumbnail_cache);
    let mut sprite = Sprite::new();
    for (i, &uid) in uids.iter().enumerate() {
        let column = (i as u32) % state.thumbnails_per_row as u32;
        let row = (i as u32) / state.thumbnails_per_row as u32;
        let x = (column * thumb_size) as f32;
        let y = (row * thumb_size) as f32 - (state.y_offset % thumb_size as f32);
        let image_rect = Rect::new(x, y, thumb_size as f32, thumb_size as f32);
        let mouse_over = image_rect.contains(Vector2f::new(mouse_pos.x as f32, mouse_pos.y as f32));
        if selected_uids.contains(&uid) {
            sprite.set_color(Color::GREEN);
        } else {
            sprite.set_color(Color::WHITE);
        }
        draw_thumbnail(
            &state.thumbnail_cache,
            db,
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
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_thumbnail<'a: 'b, 'b>(
    thumbnail_cache: &'a ThumbnailCache,
    db: &LocalDb,
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
    let (has_img, texture) = get_tex_for_entry(
        thumbnail_cache,
        id,
        &res.error_texture,
        db,
        thumbnail_loader,
        thumb_size,
        &res.loading_texture,
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
    if Key::LALT.is_pressed() {
        show_filename = true;
        let mut rect = RectangleShape::new();
        rect.set_fill_color(Color::rgba(0, 0, 0, 128));
        rect.set_size((380., 24.));
        rect.set_position(fname_pos);
        window.draw(&rect);
    }
    if show_filename {
        if let Some(file_name) = db.entries[&id].path.file_name().map(|e| e.to_str()) {
            let mut text = Text::new(file_name.unwrap(), &res.font, 12);
            text.set_position(fname_pos);
            window.draw_text(&text, &RenderStates::DEFAULT);
        }
    }
}
