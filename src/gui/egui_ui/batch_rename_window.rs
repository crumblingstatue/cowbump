use {
    crate::{
        entry,
        gui::{native_dialog, State},
    },
    egui_sfml::{
        egui::{self, PointerButton, TextureId},
        sfml::graphics::RenderWindow,
    },
};

#[derive(Default)]
pub struct BatchRenameWindow {
    pub open: bool,
    pub ids: Vec<entry::Id>,
    pub sel_idx: Option<usize>,
    pub prefix: String,
}

pub(crate) fn do_frame(
    state: &mut State,
    egui_state: &mut super::EguiState,
    coll: &mut crate::collection::Collection,
    egui_ctx: &egui_sfml::egui::Context,
    rw: &RenderWindow,
) {
    if !egui_state.batch_rename_window.open {
        return;
    }
    egui::Window::new("Batch rename")
        .open(&mut egui_state.batch_rename_window.open)
        .show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Common prefix");
                ui.text_edit_singleline(&mut egui_state.batch_rename_window.prefix);
                if ui.button("Do it").clicked() {
                    let extensions: Vec<String> = egui_state
                        .batch_rename_window
                        .ids
                        .iter()
                        .map(|id| {
                            coll.entries[id]
                                .path
                                .extension()
                                .unwrap()
                                .to_string_lossy()
                                .into_owned()
                        })
                        .collect();
                    let new_names: Vec<String> = extensions
                        .into_iter()
                        .enumerate()
                        .map(|(i, ext)| {
                            format!("{}{i:04}.{ext}", egui_state.batch_rename_window.prefix)
                        })
                        .collect();
                    for path in &new_names {
                        if std::fs::exists(path).unwrap() {
                            native_dialog::error_blocking(
                                "Batch rename error",
                                "One ore more files already exist under a target name",
                            );
                            return;
                        }
                    }
                    for (id, path) in egui_state.batch_rename_window.ids.iter().zip(&new_names) {
                        coll.rename(*id, path).unwrap();
                    }
                }
                if ui.button("Sort by filename").clicked() {
                    egui_state
                        .batch_rename_window
                        .ids
                        .sort_by_key(|id| &coll.entries[id].path);
                }
            });
            ui.label("lmb: swap, rmb: insert before, alt+lmb: view image");
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (i, id) in egui_state.batch_rename_window.ids.iter().enumerate() {
                        let mut img = egui::ImageButton::new(egui::load::SizedTexture::new(
                            TextureId::User(id.0),
                            egui::vec2(256., 256.),
                        ));
                        if let Some(sel_idx) = egui_state.batch_rename_window.sel_idx {
                            if sel_idx == i {
                                img = img.tint(egui::Color32::BROWN);
                            }
                        }
                        let re = ui.add(img);
                        let alt = ui.input(|inp| inp.modifiers.alt);
                        if re.clicked() && alt {
                            crate::gui::open::builtin::open_list(
                                state,
                                egui_state.batch_rename_window.ids.clone(),
                                i,
                                rw,
                            );
                        } else if re.clicked() {
                            match egui_state.batch_rename_window.sel_idx {
                                Some(idx) => {
                                    egui_state.batch_rename_window.ids.swap(idx, i);
                                    egui_state.batch_rename_window.sel_idx = None;
                                    break;
                                }
                                None => egui_state.batch_rename_window.sel_idx = Some(i),
                            }
                        } else if re.clicked_by(PointerButton::Secondary) {
                            if let Some(idx) = egui_state.batch_rename_window.sel_idx {
                                let id = egui_state.batch_rename_window.ids.remove(idx);
                                egui_state.batch_rename_window.ids.insert(i, id);
                                egui_state.batch_rename_window.sel_idx = None;
                                break;
                            }
                        }
                    }
                });
            });
        });
}
