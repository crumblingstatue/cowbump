use std::path::PathBuf;

use egui::{
    Button, CollapsingHeader, ComboBox, CtxRef, Grid, ScrollArea, SidePanel, Slider, TextEdit,
    Window,
};
use rfd::FileDialog;

use crate::preferences::{self, App, AppId, SCROLL_WHEEL_MAX, SCROLL_WHEEL_MIN};

use super::EguiState;

pub struct PreferencesWindow {
    pub on: bool,
    category: Category,
    new_app: App,
    new_app_path_string: String,
    path_scratch_buffer: String,
    new_ext_buffer: String,
}

impl Default for PreferencesWindow {
    fn default() -> Self {
        Self {
            on: false,
            category: Category::Ui,
            new_app: Default::default(),
            new_app_path_string: Default::default(),
            path_scratch_buffer: Default::default(),
            new_ext_buffer: Default::default(),
        }
    }
}

impl PreferencesWindow {
    pub fn toggle(&mut self) {
        self.on ^= true;
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Category {
    Ui,
    Startup,
    FileAssoc,
}

pub(crate) fn do_frame(
    egui_state: &mut EguiState,
    app: &mut crate::application::Application,
    egui_ctx: &CtxRef,
) {
    let win = &mut egui_state.preferences_window;
    let prefs = &mut app.database.preferences;
    Window::new("Preferences")
        .open(&mut win.on)
        .collapsible(false)
        .show(egui_ctx, |ui| {
            SidePanel::left("prefs_left_panel").show_inside(ui, |ui| {
                ui.selectable_value(&mut win.category, Category::Ui, "User Interface");
                ui.selectable_value(&mut win.category, Category::Startup, "Startup");
                ui.selectable_value(&mut win.category, Category::FileAssoc, "File associations");
            });
            match win.category {
                Category::Ui => {
                    ui.label("Scroll wheel multiplier");
                    ui.horizontal(|ui| {
                        ui.add(Slider::new(
                            &mut prefs.scroll_wheel_multiplier,
                            SCROLL_WHEEL_MIN..=SCROLL_WHEEL_MAX,
                        ));
                        if ui.button("Restore default").clicked() {
                            prefs.scroll_wheel_multiplier = preferences::scroll_wheel_default();
                        }
                    });
                }
                Category::Startup => {
                    ui.checkbox(&mut prefs.open_last_coll_at_start, "Open last collection");
                }
                Category::FileAssoc => {
                    ui.heading("Applications");
                    ui.separator();
                    ui.group(|ui| {
                        app_edit_ui(&mut win.new_app, &mut win.new_app_path_string, ui);
                        let butt = Button::new("Add new").enabled(
                            !win.new_app.name.is_empty()
                                && !win.new_app.path.as_os_str().is_empty(),
                        );
                        if ui.add(butt).clicked() {
                            let uid = AppId(app.database.uid_counter.next());
                            prefs.applications.insert(uid, win.new_app.clone());
                            win.new_app = Default::default();
                            win.new_app_path_string.clear();
                        }
                    });
                    ui.separator();
                    prefs.applications.retain(|k, app| {
                        let mut retain = true;
                        CollapsingHeader::new(&app.name)
                            .id_source(k.0)
                            .show(ui, |ui| {
                                win.path_scratch_buffer = app.path.to_string_lossy().into_owned();
                                app_edit_ui(app, &mut win.path_scratch_buffer, ui);
                                if ui.button("Delete").clicked() {
                                    retain = false;
                                }
                            });
                        retain
                    });
                    ui.separator();
                    ui.heading("Associations");
                    ui.horizontal(|ui| {
                        let te = TextEdit::singleline(&mut win.new_ext_buffer)
                            .hint_text("New extension");
                        ui.add(te);
                        if ui.button("Add").clicked() {
                            prefs.associations.insert(win.new_ext_buffer.clone(), None);
                        }
                    });
                    ui.separator();
                    ScrollArea::vertical().show(ui, |ui| {
                        Grid::new("prefs_assoc_grid").show(ui, |ui| {
                            prefs.associations.retain(|k, v| {
                                let mut retain = true;
                                let text = match v {
                                    None => "None",
                                    Some(id) => &prefs.applications[id].name,
                                };
                                ComboBox::from_label(k)
                                    .selected_text(text)
                                    .show_ui(ui, |ui| {
                                        for (&id, app) in &prefs.applications {
                                            ui.selectable_value(v, Some(id), &app.name);
                                        }
                                    });
                                if ui.button("🗑").clicked() {
                                    retain = false;
                                }
                                ui.end_row();
                                retain
                            });
                        });
                    });
                }
            }
        });
}

fn app_edit_ui(app: &mut App, path_buffer: &mut String, ui: &mut egui::Ui) {
    let te = TextEdit::singleline(&mut app.name).hint_text("Name");
    ui.add(te);
    ui.horizontal(|ui| {
        let te = TextEdit::singleline(path_buffer).hint_text("Path");
        if ui.add(te).changed() {
            app.path = PathBuf::from(path_buffer.clone());
        }
        if ui.button("...").clicked() {
            if let Some(path) = FileDialog::new().pick_file() {
                *path_buffer = path.to_string_lossy().into_owned();
                app.path = path;
            }
        }
    });
    let te = TextEdit::singleline(&mut app.args_string).hint_text("Argument list");
    ui.add(te).on_hover_text(
        "Use {} as an argument placeholder. \
                                                        Empty argument list will automatically \
                                                        append entries as arguments",
    );
}
