use egui::{CtxRef, Window};

use super::EguiState;

#[derive(Default)]
pub struct PreferencesWindow {
    pub on: bool,
}

impl PreferencesWindow {
    pub fn toggle(&mut self) {
        self.on ^= true;
    }
}

pub(crate) fn do_frame(
    egui_state: &mut EguiState,
    app: &mut crate::application::Application,
    egui_ctx: &CtxRef,
) {
    Window::new("Preferences")
        .open(&mut egui_state.preferences_window.on)
        .show(egui_ctx, |ui| {
            ui.checkbox(
                &mut app.database.preferences.open_last_coll_at_start,
                "Open last collection at startup",
            );
        });
}
