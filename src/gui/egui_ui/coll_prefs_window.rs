use {
    super::{
        EguiState,
        tag_autocomplete::{AcState, tag_autocomplete_popup},
    },
    crate::{
        collection::{Collection, TagsExt},
        preferences::{AppMapExt, Preferences},
    },
    egui_sfml::egui,
};

#[derive(Default)]
pub struct CollPrefsWindow {
    pub open: bool,
    add_buf: AddBuf,
    tab: Tab,
    ac_state: AcState,
    ac_closed: bool,
    err_msg: String,
}

#[derive(Default)]
struct AddBuf {
    tag: String,
    app: String,
}

#[derive(Default, PartialEq)]
enum Tab {
    #[default]
    IgnoredExts,
    TagSpecificApps,
}

pub(super) fn do_frame(
    egui_state: &mut EguiState,
    coll: &mut Collection,
    egui_ctx: &egui::Context,
    prefs: &Preferences,
) {
    let win = &mut egui_state.coll_prefs_window;
    let mut open = win.open;
    egui::Window::new("Collection preferences")
        .open(&mut open)
        .show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(win.tab == Tab::IgnoredExts, "Ignored extensions")
                    .clicked()
                {
                    win.tab = Tab::IgnoredExts;
                }
                if ui
                    .selectable_label(win.tab == Tab::TagSpecificApps, "Tag specific apps")
                    .clicked()
                {
                    win.tab = Tab::TagSpecificApps;
                }
            });
            ui.separator();
            match win.tab {
                Tab::IgnoredExts => ignored_exts_ui(ui, coll),
                Tab::TagSpecificApps => tag_specific_apps_ui(ui, coll, win, prefs),
            }
        });
    win.open = open;
}

fn ignored_exts_ui(ui: &mut egui::Ui, coll: &mut Collection) {
    coll.ignored_extensions.retain_mut(|ext| {
        let mut retain = true;
        ui.horizontal(|ui| {
            ui.text_edit_singleline(ext);
            if ui.button("🗑").clicked() {
                retain = false;
            }
        });
        retain
    });
    if ui.button("Add new").clicked() {
        coll.ignored_extensions.push(String::new());
    }
}

fn tag_specific_apps_ui(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    win: &mut CollPrefsWindow,
    prefs: &Preferences,
) {
    ui.heading("Tag specific applications");
    ui.label("new");
    ui.horizontal(|ui| {
        ui.label("tag");
        let re = ui.text_edit_singleline(&mut win.add_buf.tag);
        if re.changed() {
            win.ac_closed = false;
        }
        let (up_pressed, down_pressed) = ui.input(|inp| {
            (
                inp.key_pressed(egui::Key::ArrowUp),
                inp.key_pressed(egui::Key::ArrowDown),
            )
        });
        if !win.ac_closed
            && tag_autocomplete_popup(
                &mut win.add_buf.tag,
                &mut win.ac_state,
                coll,
                ui,
                &re,
                up_pressed,
                down_pressed,
            )
        {
            win.ac_closed = true;
        }
    });
    ui.horizontal(|ui| {
        ui.label("app");
        ui.text_edit_singleline(&mut win.add_buf.app);
        if ui.button("Add new").clicked() {
            win.err_msg.clear();
            if let Some(tag) = coll.resolve_tag(&win.add_buf.tag) {
                if let Some(app) = prefs.resolve_app(&win.add_buf.app) {
                    coll.tag_specific_apps.insert(tag, app);
                } else {
                    win.err_msg = format!("Unresolved app: '{}'", win.add_buf.app);
                }
            } else {
                win.err_msg = format!("Unresolved tag: '{}'", win.add_buf.tag);
            }
        }
    });
    if !win.err_msg.is_empty() {
        ui.label(egui::RichText::new(&win.err_msg).color(egui::Color32::DARK_RED));
    }

    ui.separator();
    coll.tag_specific_apps.retain(|tag_id, app_id| {
        let mut retain = true;
        ui.horizontal(|ui| {
            ui.label(coll.tags.first_name_of(tag_id));
            ui.label(prefs.applications.name_of(app_id));
            if ui.button("🗑").clicked() {
                retain = false;
            }
        });
        retain
    });
}
