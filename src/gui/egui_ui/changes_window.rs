use std::path::PathBuf;

use egui_sfml::{
    egui::{Color32, Context, ImageButton, Label, RichText, ScrollArea, TextureId, Window},
    sfml::graphics::{RenderTarget, RenderWindow},
};
use fnv::FnvHashMap;

use crate::{db::FolderChanges, entry, gui::entries_view::EntriesView};

use super::EguiState;

struct AddedInfo {
    id: entry::Id,
    checked: bool,
}

#[derive(Default)]
pub struct ChangesWindow {
    pub open: bool,
    changes: FolderChanges,
    added: FnvHashMap<PathBuf, AddedInfo>,
    applied: bool,
}

pub(super) fn do_frame(
    state: &mut crate::gui::State,
    egui_state: &mut EguiState,
    egui_ctx: &Context,
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
                        ui.set_width(800.);
                        ui.set_height(720.);
                        ui.heading("Added");
                        ScrollArea::vertical()
                            .id_source("scroll_add")
                            .show(ui, |ui| {
                                for add in &changes.add {
                                    match win.added.get_mut(add) {
                                        Some(info) => {
                                            ui.horizontal(|ui| {
                                                let img_button = ImageButton::new(
                                                    TextureId::User(info.id.0),
                                                    (128.0, 128.0),
                                                );
                                                let re = ui.add(img_button);
                                                if info.checked {
                                                    ui.painter().rect_stroke(
                                                        re.rect,
                                                        1.0,
                                                        (2.0, Color32::GREEN),
                                                    );
                                                }
                                                if re.clicked() {
                                                    state.highlight_and_seek_to_entry(
                                                        info.id,
                                                        rw.size().y,
                                                    );
                                                }
                                                ui.checkbox(
                                                    &mut info.checked,
                                                    add.to_string_lossy().as_ref(),
                                                );
                                            });
                                        }
                                        None => {
                                            let label = Label::new(
                                                RichText::new(add.to_string_lossy().as_ref())
                                                    .color(Color32::GREEN),
                                            );
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
                                    let label = Label::new(
                                        RichText::new(rem.to_string_lossy().as_ref())
                                            .color(Color32::RED),
                                    );
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
                            win.added
                                .insert(path.to_owned(), AddedInfo { id, checked: false });
                        });
                        state.entries_view = EntriesView::from_collection(
                            app.active_collection().unwrap().1,
                            &state.filter,
                        );
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
