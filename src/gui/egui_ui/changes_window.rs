use egui::{Color32, Label, ScrollArea, Window};

use crate::{db::FolderChanges, gui::entries_view::EntriesView};

use super::EguiState;

#[derive(Default)]
pub struct ChangesWindow {
    open: bool,
    changes: FolderChanges,
}

pub(super) fn do_frame(
    state: &mut crate::gui::State,
    egui_state: &mut EguiState,
    egui_ctx: &egui::CtxRef,
    app: &mut crate::application::Application,
) {
    let win = &mut egui_state.changes_window;
    if !win.open {
        return;
    }
    let changes = &win.changes;
    Window::new("Changes to collection").show(egui_ctx, |ui| {
        ui.horizontal(|ui| {
            if !changes.add.is_empty() {
                ui.vertical(|ui| {
                    ui.set_width(300.);
                    ui.set_height(600.);
                    ui.heading("Added");
                    ScrollArea::vertical()
                        .id_source("scroll_add")
                        .show(ui, |ui| {
                            for add in &changes.add {
                                let label = Label::new(add.to_string_lossy().as_ref())
                                    .text_color(Color32::GREEN);
                                ui.add(label);
                            }
                        });
                });
            }
            if !changes.remove.is_empty() {
                ui.vertical(|ui| {
                    ui.set_height(600.);
                    ui.set_width(300.);
                    ui.heading("Removed");
                    ScrollArea::vertical()
                        .id_source("scroll_rm")
                        .show(ui, |ui| {
                            for rem in &changes.remove {
                                let label = Label::new(rem.to_string_lossy().as_ref())
                                    .text_color(Color32::RED);
                                ui.add(label);
                            }
                        });
                });
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Apply").clicked() {
                app.apply_changes_to_active_collection(changes);
                state.entries_view =
                    EntriesView::from_collection(app.active_collection().unwrap().1);
                win.open = false;
            }
            if ui.button("Ignore").clicked() {
                win.open = false;
            }
        });
    });
}
impl ChangesWindow {
    pub(crate) fn open(&mut self, changes: FolderChanges) {
        self.open = true;
        self.changes = changes;
    }
}
