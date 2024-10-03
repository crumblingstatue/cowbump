use {
    super::EguiState,
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
    egui::Window::new("Collection preferences")
        .open(&mut win.open)
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
                Tab::IgnoredExts => {
                    coll.ignored_extensions.retain_mut(|ext| {
                        let mut retain = true;
                        ui.text_edit_singleline(ext);
                        if ui.button("-").clicked() {
                            retain = false;
                        }
                        retain
                    });
                    if ui.button("Add new").clicked() {
                        coll.ignored_extensions.push(String::new());
                    }
                }
                Tab::TagSpecificApps => {
                    ui.heading("Tag specific applications");
                    ui.label("new");
                    ui.label("tag");
                    ui.text_edit_singleline(&mut win.add_buf.tag);
                    ui.label("app");
                    ui.text_edit_singleline(&mut win.add_buf.app);
                    if ui.button("Add new").clicked() {
                        if let Some(tag) = coll.resolve_tag(&win.add_buf.tag) {
                            if let Some(app) = prefs.resolve_app(&win.add_buf.app) {
                                coll.tag_specific_apps.insert(tag, app);
                            }
                        }
                    }
                    ui.separator();
                    for (tag_id, app_id) in &coll.tag_specific_apps {
                        ui.horizontal(|ui| {
                            ui.label(coll.tags.first_name_of(tag_id));
                            ui.label(prefs.applications.name_of(app_id));
                        });
                    }
                }
            }
        });
}
