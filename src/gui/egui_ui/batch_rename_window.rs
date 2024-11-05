use {
    crate::{entry, gui::State},
    anyhow::Context,
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

fn do_batch_rename(
    ids: &[entry::Id],
    coll: &mut crate::collection::Collection,
    prefix: &str,
) -> anyhow::Result<()> {
    let extensions: Vec<String> = ids
        .iter()
        .map(|id| {
            Ok(coll
                .entries
                .get(id)
                .context("Dangling entry id")?
                .path
                .extension()
                .context("Only filenames with extensions are supported")?
                .to_string_lossy()
                .into_owned())
        })
        .collect::<anyhow::Result<_>>()?;
    let new_names: Vec<String> = extensions
        .into_iter()
        .enumerate()
        .map(|(i, ext)| format!("{prefix}{i:04}.{ext}"))
        .collect();
    for path in &new_names {
        if std::fs::exists(path)? {
            anyhow::bail!("One ore more files already exist under a target name");
        }
    }
    for (id, path) in ids.iter().zip(&new_names) {
        coll.rename(*id, path)?;
    }
    Ok(())
}

pub(crate) fn do_frame(
    state: &mut State,
    egui_state: &mut super::EguiState,
    coll: &mut crate::collection::Collection,
    egui_ctx: &egui::Context,
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
                    if let Err(e) = do_batch_rename(
                        &egui_state.batch_rename_window.ids,
                        coll,
                        &egui_state.batch_rename_window.prefix,
                    ) {
                        egui_state.modal.err(format!("Batch rename error: {e:?}"));
                    } else {
                        egui_state.modal.success("Successful batch rename");
                    }
                }
                if ui.button("Sort by filename").clicked() {
                    egui_state
                        .batch_rename_window
                        .ids
                        .sort_by_key(|id| coll.entries.get(id).map(|en| &en.path));
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
                        if state.viewer_state.shown_entry().is_some_and(|en| en == *id) {
                            img = img.tint(egui::Color32::GREEN);
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
