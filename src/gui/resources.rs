use {
    anyhow::Context,
    egui_sf2g::sf2g::{
        cpp::FBox,
        graphics::{Font, Texture},
    },
};

macro_rules! res {
    ($path:literal) => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)
    };
}

pub struct Resources {
    pub loading_texture: FBox<Texture>,
    pub error_texture: FBox<Texture>,
    pub sel_begin_texture: FBox<Texture>,
    pub movie_texture: FBox<Texture>,
    pub font: FBox<Font>,
}

impl Resources {
    pub fn load() -> anyhow::Result<Self> {
        let mut loading_texture = Texture::new().context("texture create error")?;
        let mut error_texture = Texture::new().context("texture create error")?;
        let mut sel_begin_texture = Texture::new().context("texture create error")?;
        let mut movie_texture = Texture::new().context("texture create error")?;
        let font = Font::from_memory_static(include_bytes!(res!("Vera.ttf")))
            .context("failed to load font")?;
        loading_texture.load_from_memory(include_bytes!(res!("loading.png")))?;
        error_texture.load_from_memory(include_bytes!(res!("error.png")))?;
        sel_begin_texture.load_from_memory(include_bytes!(res!("select_begin.png")))?;
        movie_texture.load_from_memory(include_bytes!(res!("movie.png")))?;
        Ok(Self {
            loading_texture,
            error_texture,
            sel_begin_texture,
            movie_texture,
            font,
        })
    }
}
