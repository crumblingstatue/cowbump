use std::path::PathBuf;

use egui::{Button, Color32, Label, ScrollArea, Window};
use fnv::FnvHashMap;
use sfml::graphics::{RenderTarget, RenderWindow};

use crate::{db::FolderChanges, entry, gui::entries_view::EntriesView};

use super::EguiState;

#[derive(Default)]
pub struct ChangesWindow {
    pub open: bool,
    changes: FolderChanges,
    resolved: FnvHashMap<PathBuf, entry::Id>,
    applied: bool,
}

pub(super) fn do_frame(
    state: &mut crate::gui::State,
    egui_state: &mut EguiState,
    egui_ctx: &egui::CtxRef,
    app: &mut crate::application::Application,
    rw: &RenderWindow,
) {
    let win = &mut egui_state.changes_window;
    if !win.open {
        return;
    }
    let changes = &win.changes;
    let mut close = false;
    Window::new("Changes to collection")
        .open(&mut win.open)
        .show(egui_ctx, |ui| {
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
                                    match win.resolved.get(add) {
                                        Some(id) => {
                                            let button =
                                                Button::new(add.to_string_lossy().as_ref());
                                            if ui.add(button).clicked() {
                                                state.highlight_and_seek_to_entry(
                                                    *id,
                                                    rw.size().y,
                                                    &app.active_collection.as_ref().unwrap().1,
                                                );
                                            }
                                        }
                                        None => {
                                            let label = Label::new(add.to_string_lossy().as_ref())
                                                .text_color(Color32::GREEN);
                                            ui.add(label);
                                        }
                                    }
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
                if !win.applied {
                    if ui.button("Apply").clicked() {
                        app.apply_changes_to_active_collection(changes, |path, id| {
                            win.resolved.insert(path.to_owned(), id);
                        });
                        state.entries_view =
                            EntriesView::from_collection(app.active_collection().unwrap().1);
                        win.applied = true;
                    }
                    if ui.button("Ignore").clicked() {
                        close = true;
                    }
                } else if ui.button("Close").clicked() {
                    close = true;
                }
            });
        });
    if close {
        win.open = false;
    }
}
impl ChangesWindow {
    pub(crate) fn open(&mut self, changes: FolderChanges) {
        self.open = true;
        self.changes = changes;
    }
}
