use egui_sfml::sfml::{
    graphics::{Image, RenderTarget, RenderWindow, Texture},
    system::Vector2u,
};
use rfd::FileDialog;

use crate::gui::native_dialog;
use anyhow::Context;

pub fn take_screenshot(win: &RenderWindow) -> anyhow::Result<Image> {
    let mut tex = Texture::new().context("Failed to create texture")?;
    let Vector2u { x: w, y: h } = win.size();
    if !tex.create(w, h) {
        panic!();
    }
    unsafe {
        tex.update_from_render_window(win, 0, 0);
    }
    tex.copy_to_image()
        .context("Failed to copy texture to image")
}

pub fn take_and_save_screenshot(win: &RenderWindow) {
    let result: anyhow::Result<()> = try {
        let ss = take_screenshot(win)?;
        if let Some(path) = FileDialog::new()
            .add_filter("Images", &["png", "jpg", "bmp", "tga"])
            .set_file_name("cowbump-screenshot.png")
            .save_file()
        {
            let path_str = path.to_str().context("Failed to convert path to str")?;
            ss.save_to_file(path_str)
                .then_some(())
                .context("Failed to save image")?;
        }
    };
    if let Err(e) = result {
        native_dialog::error("Failed to take screenshot", e);
    }
}
