use {
    super::EguiState,
    crate::{
        application::Application,
        db::FolderChanges,
        entry,
        gui::{native_dialog, thumbnails_view::ThumbnailsView},
    },
    egui_sfml::{
        egui::{
            self, load::SizedTexture, Color32, Context, ImageButton, Label, RichText, ScrollArea,
            TextureId, Window,
        },
        sfml::graphics::{RenderTarget, RenderWindow},
    },
    fnv::FnvHashMap,
    std::path::PathBuf,
};

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

enum Action {
    RemFile { idx: usize },
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
                            .id_source("scroll_add")
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
                                                if info.checked {
                                                    ui.painter().rect_stroke(
                                                        re.rect,
                                                        1.0,
                                                        (2.0, Color32::GREEN),
                                                    );
                                                }
                                                if re.clicked() {
                                                    state.thumbs_view.highlight_and_seek_to_entry(
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
                        state.thumbs_view = ThumbnailsView::from_collection(
                            rw.size().x,
                            Application::active_collection(&mut app.active_collection)
                                .unwrap()
                                .1,
                            &state.filter,
                            &app.database.preferences,
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
    if let Some(action) = action {
        match action {
            Action::RemFile { idx } => {
                let path = &changes.add[idx];
                if let Err(e) = std::fs::remove_file(path) {
                    native_dialog::error("Failed to remove file", e);
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
    pub(crate) fn open(&mut self, changes: FolderChanges) {
        self.open = true;
        self.changes = changes;
    }
}
