use egui_sfml::egui;

use crate::{collection::Collection, preferences::Preferences};

use super::EguiState;

#[derive(Default)]
pub struct TagSpecificAppsWindow {
    pub open: bool,
    add_buf: AddBuf,
}

#[derive(Default)]
struct AddBuf {
    tag: String,
    app: String,
}

pub(super) fn do_frame(
    egui_state: &mut EguiState,
    coll: &mut Collection,
    egui_ctx: &egui::Context,
    prefs: &mut Preferences,
) {
    let win = &mut egui_state.tag_specific_apps_window;
    egui::Window::new("Tag specific applications")
        .open(&mut win.open)
        .show(egui_ctx, |ui| {
            ui.label("new");
            ui.label("tag");
            ui.text_edit_singleline(&mut win.add_buf.tag);
            ui.label("app");
            ui.text_edit_singleline(&mut win.add_buf.app);
            if ui.button("Add new").clicked() {
                if let Some(tag) = coll.resolve_tag(&win.add_buf.tag) {
                    if let Some(app) = prefs.resolve_app(&win.add_buf.app) {
                        coll.tag_specific_apps.insert(tag, app);
                    }
                }
            }
            ui.separator();
            for (tag_id, app_id) in coll.tag_specific_apps.iter() {
                ui.horizontal(|ui| {
                    ui.label(&coll.tags[tag_id].names[0]);
                    ui.label(&prefs.applications[app_id].name);
                });
            }
        });
}
