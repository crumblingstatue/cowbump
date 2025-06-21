use {
    super::EguiState,
    crate::{
        application::Application, db::FolderChanges, entry, gui::thumbnails_view::ThumbnailsView,
    },
    egui_sf2g::{
        egui::{
            self, Color32, Context, ImageButton, Label, RichText, ScrollArea, TextureId, Window,
            load::SizedTexture,
        },
        sf2g::graphics::{RenderTarget, RenderWindow},
    },
    fnv::FnvHashMap,
    std::path::PathBuf,
};

struct AddedInfo {
    id: entry::Id,
}

#[derive(Default)]
pub struct ChangesWindow {
    pub open: bool,
    changes: FolderChanges,
    added: FnvHashMap<PathBuf, AddedInfo>,
    applied: bool,
}

enum Action {
    RemFile { idx: usize },
}

pub(super) fn do_frame(
    state: &mut crate::gui::State,
    egui_state: &mut EguiState,
    egui_ctx: &Context,
    app: &mut Application,
    rw: &RenderWindow,
) {
    let win = &mut egui_state.changes_window;
    if !win.open {
        return;
    }
    let changes = &mut win.changes;
    let mut close = false;
    let mut action = None;
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
                            .id_salt("scroll_add")
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                for (idx, add) in changes.add.iter().enumerate() {
                                    match win.added.get_mut(add) {
                                        Some(info) => {
                                            ui.horizontal(|ui| {
                                                let img_button =
                                                    ImageButton::new(SizedTexture::new(
                                                        TextureId::User(info.id.0),
                                                        (128.0, 128.0),
                                                    ));
                                                let re = ui.add(img_button);
                                                if re.clicked() {
                                                    state.thumbs_view.highlight_and_seek_to_entry(
                                                        info.id,
                                                        rw.size().y,
                                                    );
                                                }
                                            });
                                        }
                                        None => {
                                            ui.scope(|ui| {
                                                let vis = &mut ui.style_mut().visuals.widgets;
                                                vis.inactive.fg_stroke.color =
                                                    Color32::from_rgb(26, 138, 11);
                                                vis.hovered.fg_stroke.color =
                                                    Color32::from_rgb(167, 255, 155);
                                                let label =
                                                    Label::new(add.to_string_lossy().as_ref())
                                                        .sense(egui::Sense::click());
                                                ui.add(label).context_menu(|ui| {
                                                    if ui.button("Delete file").clicked() {
                                                        action = Some(Action::RemFile { idx });
                                                        ui.close_menu();
                                                    }
                                                })
                                            });
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
                            .id_salt("scroll_rm")
                            .auto_shrink(false)
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
                            win.added.insert(path.to_owned(), AddedInfo { id });
                        });
                        if let Some((_, active_coll)) = &mut app.active_collection {
                            state.thumbs_view = ThumbnailsView::from_collection(
                                rw.size().x,
                                active_coll,
                                &state.filter,
                                &app.database.preferences,
                            );
                            win.applied = true;
                        } else {
                            egui_state.modal.err("No active collection");
                        }
                    }
                    if ui.button("Ignore").clicked() {
                        close = true;
                    }
                } else if ui.button("Close").clicked() {
                    close = true;
                }
            });
        });
    if let Some(action) = action {
        match action {
            Action::RemFile { idx } => {
                let Some(path) = changes.add.get(idx) else {
                    egui_state
                        .modal
                        .err(format!("Dangling change index ({idx})"));
                    return;
                };
                if let Err(e) = std::fs::remove_file(path) {
                    egui_state.modal.err(format!("Failed to remove file: {e}"));
                }
                changes.add.remove(idx);
            }
        }
    }
    if close {
        win.open = false;
    }
}
impl ChangesWindow {
    /// Opens a freshly initialized changes window containing the provided changes
    pub(crate) fn open_fresh(&mut self, changes: FolderChanges) {
        *self = Default::default();
        self.open = true;
        self.changes = changes;
    }
}
