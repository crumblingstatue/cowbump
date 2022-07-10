use std::path::PathBuf;

use egui_sfml::egui::{
    Button, CollapsingHeader, ComboBox, Context, Grid, ScrollArea, SidePanel, Slider, TextEdit, Ui,
    Window,
};
use rfd::FileDialog;

use crate::preferences::{App, AppId, FloatPref, ScrollWheelMultiplier, UpDownArrowScrollSpeed};

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

/// A slider for font sizes. Returns true if the value should be considered updated
fn font_slider(ui: &mut Ui, label: &str, value: &mut f32) -> bool {
    let re = ui.add(Slider::new(value, 8.0..=64.0).integer().text(label));
    re.drag_released || re.lost_focus()
}

pub(crate) fn do_frame(
    egui_state: &mut EguiState,
    app: &mut crate::application::Application,
    egui_ctx: &Context,
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
                    ui.heading("Scrolling");
                    slider_with_default::<ScrollWheelMultiplier>(
                        ui,
                        &mut prefs.scroll_wheel_multiplier,
                    );
                    slider_with_default::<UpDownArrowScrollSpeed>(
                        ui,
                        &mut prefs.arrow_key_scroll_speed,
                    );
                    ui.separator();
                    ui.heading("Font sizes");
                    let mut style_changed = false;
                    style_changed |= font_slider(ui, "Heading", &mut prefs.style.heading_size);
                    style_changed |= font_slider(ui, "Body", &mut prefs.style.body_size);
                    style_changed |= font_slider(ui, "Button", &mut prefs.style.button_size);
                    style_changed |= font_slider(ui, "Monospace", &mut prefs.style.monospace_size);
                    if style_changed {
                        crate::gui::set_up_style(egui_ctx, &prefs.style);
                    }
                    ui.heading("Viewer");
                    ui.checkbox(
                        &mut prefs.use_built_in_viewer,
                        "Use built-in viewer for supported formats",
                    );
                }
                Category::Startup => {
                    ui.checkbox(&mut prefs.open_last_coll_at_start, "Open last collection");
                }
                Category::FileAssoc => {
                    ui.heading("Applications");
                    ui.separator();
                    ui.group(|ui| {
                        app_edit_ui(&mut win.new_app, &mut win.new_app_path_string, ui);
                        let butt = Button::new("Add new");
                        if ui
                            .add_enabled(
                                !win.new_app.name.is_empty()
                                    && !win.new_app.path.as_os_str().is_empty(),
                                butt,
                            )
                            .clicked()
                        {
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
                        win.new_ext_buffer.make_ascii_lowercase();
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
                                let ext_name = if k.is_empty() { "<no extension>" } else { k };
                                ComboBox::from_label(ext_name).selected_text(text).show_ui(
                                    ui,
                                    |ui| {
                                        for (&id, app) in &prefs.applications {
                                            ui.selectable_value(v, Some(id), &app.name);
                                        }
                                    },
                                );
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

fn slider_with_default<T: FloatPref>(ui: &mut Ui, attribute: &mut f32) {
    ui.label(T::NAME);
    ui.horizontal(|ui| {
        ui.add(Slider::new(attribute, T::RANGE));
        if ui.button("Restore default").clicked() {
            *attribute = T::DEFAULT;
        }
    });
}

fn app_edit_ui(app: &mut App, path_buffer: &mut String, ui: &mut Ui) {
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
