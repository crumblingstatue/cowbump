use {
    super::{icons, EguiState},
    crate::{
        gui::State,
        preferences::{
            App, AppId, ScrollWheelMultiplier, ThumbnailsPerRow, UpDownArrowScrollSpeed, ValuePref,
        },
    },
    constcat::concat,
    egui_colors::{tokens::ColorPreset, Colorix},
    egui_file_dialog::FileDialog,
    egui_sfml::{
        egui::{
            self, collapsing_header::CollapsingState, Button, ComboBox, Context, Grid, ScrollArea,
            SidePanel, Slider, TextEdit, Ui, Window,
        },
        sfml::graphics::RenderTarget,
    },
    rand::Rng,
    std::path::PathBuf,
};

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
    ColorTheme,
}

/// A slider for font sizes. Returns true if the value should be considered updated
fn font_slider(ui: &mut Ui, label: &str, value: &mut f32) -> bool {
    let re = ui.add(Slider::new(value, 8.0..=64.0).integer().text(label));
    re.drag_stopped() || re.lost_focus()
}

pub(in crate::gui) fn do_frame(
    state: &mut State,
    egui_state: &mut EguiState,
    app: &mut crate::application::Application,
    egui_ctx: &Context,
    rw: &egui_sfml::sfml::graphics::RenderWindow,
) {
    let mut open = egui_state.preferences_window.on;
    Window::new("Preferences")
        .open(&mut open)
        .collapsible(false)
        .show(egui_ctx, |ui| {
            SidePanel::left("prefs_left_panel").show_inside(ui, |ui| {
                ui.selectable_value(
                    &mut egui_state.preferences_window.category,
                    Category::Ui,
                    "User Interface",
                );
                ui.selectable_value(
                    &mut egui_state.preferences_window.category,
                    Category::Startup,
                    "Startup",
                );
                ui.selectable_value(
                    &mut egui_state.preferences_window.category,
                    Category::FileAssoc,
                    "File associations",
                );
                ui.selectable_value(
                    &mut egui_state.preferences_window.category,
                    Category::ColorTheme,
                    "Color theme",
                );
            });
            match egui_state.preferences_window.category {
                Category::Ui => ui_categ_ui(ui, &mut app.database.preferences, state, rw, egui_ctx),
                Category::Startup => startup_categ_ui(ui, &mut app.database.preferences),
                Category::FileAssoc => file_assoc_categ_ui(
                    ui,
                    egui_ctx,
                    &mut egui_state.preferences_window,
                    app,
                    &mut egui_state.file_dialog,
                ),
                Category::ColorTheme => {
                    color_theme_categ_ui(egui_state, egui_ctx, ui, &mut app.database.preferences);
                }
            };
        });
    egui_state.preferences_window.on = open;
}

fn color_theme_categ_ui(
    egui_state: &mut EguiState,
    egui_ctx: &Context,
    ui: &mut Ui,
    prefs: &mut crate::preferences::Preferences,
) {
    let colorix = egui_state
        .colorix
        .get_or_insert_with(|| Colorix::init(egui_ctx, egui_colors::utils::EGUI_THEME));
    ui.horizontal(|ui| {
        ui.label("Preset");
        colorix.themes_dropdown(egui_ctx, ui, None, false);
    });
    ui.separator();
    colorix.ui_combo_12(egui_ctx, ui);
    ui.separator();
    ui.horizontal(|ui| {
        ui.group(|ui| {
            ui.label("Dark/light toggle");
            colorix.light_dark_toggle_button(egui_ctx, ui);
        });
        if ui.button(concat!(icons::SORT, " Randomize")).clicked() {
            let mut rng = rand::thread_rng();
            let theme =
                std::array::from_fn(|_| ColorPreset::Custom([rng.gen(), rng.gen(), rng.gen()]));
            *colorix = Colorix::init(egui_ctx, theme);
        }
    });
    ui.separator();
    ui.horizontal(|ui| {
        if let Some(theme) = &prefs.color_theme {
            if ui.button(concat!(icons::CANCEL, " Restore")).clicked() {
                *colorix = Colorix::init(egui_ctx, theme.to_colorix());
            }
        }
        if ui.button(concat!(icons::SAVE, " Save custom")).clicked() {
            prefs.set_color_theme_from_colorix(colorix);
        }
        if ui
            .button(concat!(icons::SAVE, " Reset default egui theme and save"))
            .clicked()
        {
            prefs.color_theme = None;
            egui_ctx.set_visuals(egui::Visuals::dark());
        }
    });
}

fn file_assoc_categ_ui(
    ui: &mut Ui,
    egui_ctx: &Context,
    win: &mut PreferencesWindow,
    app: &mut crate::application::Application,
    file_dialog: &mut FileDialog,
) {
    let prefs = &mut app.database.preferences;
    ui.heading("Applications");
    ui.group(|ui| {
        let collap = CollapsingState::load_with_default_open(
            egui_ctx,
            egui::Id::new("add_new_collap"),
            false,
        );
        let head_re = collap.show_header(ui, |ui| {
            ui.label("Add new");
        });
        head_re.body(|ui| {
            app_edit_ui(
                &mut win.new_app,
                &mut win.new_app_path_string,
                ui,
                file_dialog,
            );
            let butt = Button::new(concat!(icons::CHECK, " Add new application"));
            if ui
                .add_enabled(
                    !win.new_app.name.is_empty() && !win.new_app.path.as_os_str().is_empty(),
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
    });
    ui.separator();
    prefs.applications.retain(|k, app| {
        let mut retain = true;
        let collap = CollapsingState::load_with_default_open(
            egui_ctx,
            egui::Id::new(&app.name).with(k.0),
            false,
        );
        let head_re = collap.show_header(ui, |ui| {
            ui.label(&app.name);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(icons::REMOVE).on_hover_text("Delete").clicked() {
                    retain = false;
                }
            });
        });
        head_re.body(|ui| {
            win.path_scratch_buffer = app.path.to_string_lossy().into_owned();
            app_edit_ui(app, &mut win.path_scratch_buffer, ui, file_dialog);
        });
        ui.separator();
        retain
    });
    ui.separator();
    ui.heading("Associations");
    ui.horizontal(|ui| {
        let te = TextEdit::singleline(&mut win.new_ext_buffer).hint_text("New extension");
        ui.add(te);
        win.new_ext_buffer.make_ascii_lowercase();
        if ui.button("Add").clicked() {
            prefs.associations.insert(win.new_ext_buffer.clone(), None);
        }
    });
    ui.separator();
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        Grid::new("prefs_assoc_grid").show(ui, |ui| {
            prefs.associations.retain(|k, v| {
                let mut retain = true;
                let text = match v {
                    None => "None",
                    Some(id) => &prefs.applications[id].name,
                };
                let ext_name = if k.is_empty() { "<no extension>" } else { k };
                ComboBox::from_label(ext_name)
                    .selected_text(text)
                    .show_ui(ui, |ui| {
                        for (&id, app) in &prefs.applications {
                            ui.selectable_value(v, Some(id), &app.name);
                        }
                    });
                if ui.button(icons::REMOVE).clicked() {
                    retain = false;
                }
                ui.end_row();
                retain
            });
        });
    });
}

fn startup_categ_ui(ui: &mut Ui, prefs: &mut crate::preferences::Preferences) {
    ui.checkbox(&mut prefs.start_fullscreen, "Start in fullscreen mode");
    ui.checkbox(&mut prefs.open_last_coll_at_start, "Open last collection");
}

fn ui_categ_ui(
    ui: &mut Ui,
    prefs: &mut crate::preferences::Preferences,
    state: &mut State,
    rw: &egui_sfml::sfml::graphics::RenderWindow,
    egui_ctx: &Context,
) {
    ui.heading("Thumbnails view");
    if slider_with_default::<ThumbnailsPerRow>(ui, &mut prefs.thumbs_per_row) {
        state.thumbs_view.resize(rw.size().x, prefs);
    }
    ui.heading("Scrolling");
    slider_with_default::<ScrollWheelMultiplier>(ui, &mut prefs.scroll_wheel_multiplier);
    slider_with_default::<UpDownArrowScrollSpeed>(ui, &mut prefs.arrow_key_scroll_speed);
    ui.separator();
    ui.heading("Font sizes");
    let mut style_changed = false;
    style_changed |= font_slider(ui, "Heading", &mut prefs.style.heading_size);
    style_changed |= font_slider(ui, "Body", &mut prefs.style.body_size);
    style_changed |= font_slider(ui, "Button", &mut prefs.style.button_size);
    style_changed |= font_slider(ui, "Monospace", &mut prefs.style.monospace_size);
    if ui.button("Restore default sizes").clicked() {
        prefs.style = crate::preferences::Style::default();
        style_changed = true;
    }
    if style_changed {
        crate::gui::egui_ui::set_up_style(egui_ctx, &prefs.style);
    }
    ui.separator();
    ui.heading("Opening");
    ui.checkbox(
        &mut prefs.use_built_in_viewer,
        "Use built-in viewer for supported formats",
    );
}

/// Returns whether the value changes
fn slider_with_default<T: ValuePref>(ui: &mut Ui, attribute: &mut T::Type) -> bool {
    let mut changed = false;
    ui.label(T::NAME);
    ui.horizontal(|ui| {
        changed = ui.add(Slider::new(attribute, T::RANGE)).changed();
        if ui.button("Restore default").clicked() {
            *attribute = T::DEFAULT;
            changed = true;
        }
    });
    changed
}

fn app_edit_ui(app: &mut App, path_buffer: &mut String, ui: &mut Ui, file_dialog: &mut FileDialog) {
    Grid::new("grid").num_columns(2).show(ui, |ui| {
        ui.label("Name");
        let te = TextEdit::singleline(&mut app.name).hint_text("Name");
        ui.add(te);
        ui.end_row();
        ui.label("Path");
        let te = TextEdit::singleline(path_buffer).hint_text("Path");
        if ui.add(te).changed() {
            app.path = PathBuf::from(path_buffer.clone());
        }
        if ui.button("...").clicked() {
            file_dialog.select_file();
        }
        ui.end_row();
        ui.label("Arg list");
        let te = TextEdit::singleline(&mut app.args_string).hint_text("Argument list");
        ui.add(te).on_hover_text(
            "Use {} as an argument placeholder. \
                                                        Empty argument list will automatically \
                                                        append entries as arguments",
        );
    });
    if let Some(path) = file_dialog.take_selected() {
        *path_buffer = path.to_string_lossy().into_owned();
        app.path = path;
    }
}
