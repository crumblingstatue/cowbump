use {
    super::egui_ui::{EguiState, FileOp},
    anyhow::Context,
    egui_sfml::sfml::{
        graphics::{Image, RenderTarget, RenderWindow, Texture},
        system::Vector2u,
    },
};

pub fn take_screenshot(win: &RenderWindow) -> anyhow::Result<Image> {
    let mut tex = Texture::new().context("Failed to create texture")?;
    let Vector2u { x: w, y: h } = win.size();
    if tex.create(w, h).is_err() {
        panic!();
    }
    unsafe {
        tex.update_from_render_window(win, 0, 0);
    }
    tex.copy_to_image()
        .context("Failed to copy texture to image")
}

pub fn take_and_save_screenshot(win: &RenderWindow, egui_state: &mut EguiState) {
    let result: anyhow::Result<()> = try {
        let ss = take_screenshot(win)?;
        egui_state.file_dialog.save_file();
        egui_state.file_op = Some(FileOp::SaveScreenshot(ss));
    };
    if let Err(e) = result {
        egui_state
            .modal
            .err(format!("Failed to take screenshot: {e:?}"));
    }
}
