use {crate::application::Application, egui_sfml::egui};

#[derive(Default)]
pub struct CollectionsDbWindow {
    pub open: bool,
    pub path_assign_id: Option<crate::collection::Id>,
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
                        egui_state.collections_db_window.path_assign_id = Some(*id);
                        egui_state.file_dialog.select_directory();
                    }
                });
            }
        });
    if let Some(coll_id) = &egui_state.collections_db_window.path_assign_id
        && let Some(path) = egui_state.file_dialog.take_selected()
    {
        *app.database.collections.get_mut(coll_id).unwrap() = path;
        egui_state.collections_db_window.path_assign_id = None;
    }
}
