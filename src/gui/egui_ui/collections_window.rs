use {super::EguiModalExt, crate::application::Application, egui_sfml::egui};

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
            app.database.collections.retain(|id, path| {
                let mut retain = true;
                ui.horizontal(|ui| {
                    ui.label(id.0.to_string());
                    if ui.button(path.display().to_string()).clicked() {
                        egui_state.collections_db_window.path_assign_id = Some(*id);
                        egui_state.file_dialog.select_directory();
                    }
                    if ui.button("Remove").clicked() {
                        retain = false;
                    }
                });
                retain
            });
        });
    if let Some(assign_id) = &egui_state.collections_db_window.path_assign_id
        && let Some(path) = egui_state.file_dialog.take_selected()
    {
        if let Some(coll_path) = app.database.collections.get_mut(assign_id) {
            *coll_path = path;
        } else {
            egui_state
                .modal
                .err("Failed to assign path (no such collection)");
        }
        egui_state.collections_db_window.path_assign_id = None;
    }
}
