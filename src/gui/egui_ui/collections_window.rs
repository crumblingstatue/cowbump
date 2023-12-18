use egui_sfml::egui;

use crate::application::Application;

#[derive(Default)]
pub struct CollectionsDbWindow {
    pub open: bool,
}

pub(crate) fn do_frame(
    app: &mut Application,
    egui_state: &mut super::EguiState,
    egui_ctx: &egui::Context,
) {
    egui::Window::new("Collections database editor")
        .open(&mut egui_state.collections_db_window.open)
        .show(egui_ctx, |ui| {
            for (id, path) in &mut app.database.collections {
                ui.horizontal(|ui| {
                    ui.label(id.0.to_string());
                    if ui.button(path.display().to_string()).clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            *path = folder;
                        }
                    }
                });
            }
        });
}
