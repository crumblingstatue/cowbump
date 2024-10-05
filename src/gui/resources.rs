use {
    anyhow::Context,
    egui_sfml::sfml::{
        graphics::{Font, IntRect, Texture},
        SfBox,
    },
};

macro_rules! res {
    ($path:literal) => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)
    };
}

pub struct Resources {
    pub loading_texture: SfBox<Texture>,
    pub error_texture: SfBox<Texture>,
    pub sel_begin_texture: SfBox<Texture>,
    pub font: SfBox<Font>,
}

impl Resources {
    pub fn load() -> anyhow::Result<Self> {
        let mut loading_texture = Texture::new().context("texture create error")?;
        let mut error_texture = Texture::new().context("texture create error")?;
        let mut sel_begin_texture = Texture::new().context("texture create error")?;
        let font = Font::from_memory_static(include_bytes!(res!("Vera.ttf")))
            .context("failed to load font")?;
        loading_texture
            .load_from_memory(include_bytes!(res!("loading.png")), IntRect::default())?;
        error_texture.load_from_memory(include_bytes!(res!("error.png")), IntRect::default())?;
        sel_begin_texture
            .load_from_memory(include_bytes!(res!("select_begin.png")), IntRect::default())?;
        Ok(Self {
            loading_texture,
            error_texture,
            sel_begin_texture,
            font,
        })
    }
}
