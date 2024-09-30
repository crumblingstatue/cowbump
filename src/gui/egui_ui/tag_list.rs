use {
    super::{icons, ui_ext::UiExt, EguiState},
    crate::{
        collection::Collection,
        db::{TagSet, UidCounter},
        gui::{
            egui_ui::{prompt, PromptAction},
            State,
        },
        tag,
    },
    egui_sfml::egui::{Button, Color32, Context, Grid, Key, RichText, ScrollArea, TextEdit},
};

#[derive(Default)]
pub struct TagWindow {
    pub on: bool,
    pub filter_string: String,
    pub selected_uids: TagSet,
    pub prop_active: Option<tag::Id>,
    pub new_name: Option<String>,
    pub new_imply: Option<String>,
    pub new_tag: Option<String>,
}

impl TagWindow {
    pub fn toggle(&mut self) {
        self.on ^= true;
    }
}

pub(super) fn do_frame(
    state: &mut State,
    egui_state: &mut EguiState,
    coll: &mut Collection,
    egui_ctx: &Context,
    uid_counter: &mut UidCounter,
) {
    if !egui_state.tag_window.on {
        return;
    }
    let mut close = false;
    let close_ref = &mut close;
    let tag_filter_string = &mut egui_state.tag_window.filter_string;
    let entries_view = &mut state.thumbs_view;
    let filter_string = &mut egui_state.filter_popup.string;
    let reqs = &mut state.filter;
    let selected_uids = &mut egui_state.tag_window.selected_uids;
    let active = &mut egui_state.tag_window.prop_active;
    let new_name = &mut egui_state.tag_window.new_name;
    let new_imply = &mut egui_state.tag_window.new_imply;
    let new_tag = &mut egui_state.tag_window.new_tag;
    // Clear selected uids that have already been deleted
    selected_uids.retain(|uid| coll.tags.contains_key(uid));
    let prompts = &mut egui_state.prompts;
    egui_sfml::egui::Window::new([icons::TAG, " Tag list"].concat())
        .open(&mut egui_state.tag_window.on)
        .show(egui_ctx, move |ui| {
            ui.horizontal(|ui| {
                let te = TextEdit::singleline(tag_filter_string).hint_text("Filter");
                ui.add(te);
                if ui
                    .button(icons::CLEAR)
                    .on_hover_text("Clear filter string")
                    .clicked()
                {
                    tag_filter_string.clear();
                }
                if ui.button("Clear all filters").clicked() {
                    reqs.clear();
                    entries_view.update_from_collection(coll, reqs);
                }
                if new_tag.is_none() {
                    if ui.button("Add new tag").clicked() {
                        *new_tag = Some(String::new());
                    }
                } else {
                    let mut cancel = false;
                    let mut confirm = false;
                    if let Some(tag) = new_tag.take_if(|tag| {
                        let re = ui.add(TextEdit::singleline(tag).hint_text("New tag"));
                        if ui.button(icons::CANCEL).clicked() {
                            cancel = true;
                        }
                        if ui.button(icons::CHECK).clicked() {
                            confirm = true;
                        }
                        (re.lost_focus() && ui.input(|inp| inp.key_pressed(Key::Enter))) | confirm
                    }) {
                        coll.add_new_tag_from_text(tag, uid_counter);
                    }
                    if cancel {
                        *new_tag = None;
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_min_height(600.);
                    ui.set_width(400.0);
                    let scroll = ScrollArea::vertical()
                        .max_height(600.0)
                        .auto_shrink([false, false]);
                    scroll.show(ui, |ui| {
                        Grid::new("tag_window_grid")
                            .spacing((8.0, 8.0))
                            .striped(true)
                            .num_columns(4)
                            .show(ui, |ui| {
                                let mut uids: Vec<tag::Id> = coll.tags.keys().cloned().collect();
                                uids.sort_by_key(|uid| &coll.tags[uid].names[0]);
                                for tag_uid in &uids {
                                    let tag = &coll.tags[tag_uid];
                                    let name = &tag.names[0];
                                    if !name.contains(&tag_filter_string[..]) {
                                        continue;
                                    }
                                    let mut button = Button::new(name);
                                    let mut checked = selected_uids.contains(tag_uid);
                                    if active == &Some(*tag_uid) {
                                        button = button.fill(Color32::from_rgb(95, 69, 8));
                                    } else if checked {
                                        button = button.fill(Color32::from_rgb(189, 145, 85));
                                    }
                                    if ui.add(button).clicked() {
                                        *active = Some(*tag_uid);
                                    }
                                    let has_this_tag = reqs.have_tag(*tag_uid);
                                    let doesnt_have_this_tag = reqs.not_have_tag(*tag_uid);
                                    let button = Button::new(icons::CHECK).fill(if has_this_tag {
                                        Color32::from_rgb(43, 109, 57)
                                    } else {
                                        Color32::from_rgb(45, 45, 45)
                                    });
                                    let mut clicked_any = false;
                                    if ui
                                        .add(button)
                                        .on_hover_text(format!("Filter for {name}"))
                                        .clicked()
                                    {
                                        reqs.toggle_have_tag(*tag_uid);
                                        reqs.set_not_have_tag(*tag_uid, false);
                                        clicked_any = true;
                                    }
                                    let neg_button =
                                        Button::new(RichText::new("！").color(Color32::RED)).fill(
                                            if doesnt_have_this_tag {
                                                Color32::from_rgb(109, 47, 43)
                                            } else {
                                                Color32::from_rgb(45, 45, 45)
                                            },
                                        );
                                    if ui
                                        .add(neg_button)
                                        .on_hover_text(format!("Filter for !{name}"))
                                        .clicked()
                                    {
                                        reqs.toggle_not_have_tag(*tag_uid);
                                        reqs.set_have_tag(*tag_uid, false);
                                        clicked_any = true;
                                    }
                                    ui.checkbox(&mut checked, "");
                                    if checked {
                                        selected_uids.insert(*tag_uid);
                                    } else {
                                        selected_uids.remove(tag_uid);
                                    }
                                    ui.end_row();
                                    if clicked_any {
                                        *filter_string = reqs.to_string(&coll.tags);
                                        entries_view.update_from_collection(coll, reqs);
                                    }
                                }
                            });
                    });
                    if !selected_uids.is_empty() {
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button([icons::REMOVE, " Delete"].concat()).clicked() {
                                let n = selected_uids.len();
                                let fstring;
                                let msg = format!(
                                    "Delete the selected {}tag{}?",
                                    if n == 1 {
                                        ""
                                    } else {
                                        fstring = format!("{n} ");
                                        &fstring
                                    },
                                    if n == 1 { "" } else { "s" }
                                );
                                prompt(
                                    prompts,
                                    "Tag deletion",
                                    msg,
                                    PromptAction::DeleteTags(
                                        selected_uids.iter().cloned().collect(),
                                    ),
                                );
                            }
                            if ui
                                .button([icons::CLEAR, " Clear selection"].concat())
                                .clicked()
                            {
                                selected_uids.clear();
                            }
                        });
                    }
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_min_width(400.);
                    match active {
                        None => {
                            ui.heading("Click a tag to edit properties");
                        }
                        Some(id) => {
                            if !coll.tags.contains_key(id) {
                                ui.label(format!("No such tag with id {id:?}"));
                                return;
                            }
                            ui.horizontal(|ui| {
                                let name = format!(
                                    "{} {} (#{})",
                                    icons::TAG,
                                    coll.tags[id].names[0],
                                    id.0
                                );
                                ui.heading(&name);
                                ui.rtl(|ui| {
                                    if ui
                                        .button(icons::REMOVE)
                                        .on_hover_text("Delete tag")
                                        .clicked()
                                    {
                                        prompt(
                                            prompts,
                                            "Tag deletion",
                                            format!("Really delete the tag \"{name}\"?"),
                                            PromptAction::DeleteTags([*id].to_vec()),
                                        );
                                    }
                                });
                            });
                            ui.separator();
                            let tag = coll.tags.get_mut(id).unwrap();
                            ui.horizontal(|ui| {
                                ui.label("Names");
                                ui.rtl(|ui| {
                                    let mut confirm = false;
                                    match new_name {
                                        Some(_) => {
                                            if ui.button(icons::CANCEL).clicked() {
                                                *new_name = None;
                                            }
                                            if ui.button(icons::CHECK).clicked() {
                                                confirm = true;
                                            }
                                        }
                                        None => {
                                            if ui.button(icons::ADD).clicked() {
                                                *new_name = Some(String::new());
                                            }
                                        }
                                    }
                                    if let Some(new) = new_name.take_if(|name| {
                                        let re = ui
                                            .add(TextEdit::singleline(name).hint_text("New alias"));
                                        (re.lost_focus()
                                            && ui.input(|inp| inp.key_pressed(Key::Enter)))
                                            | confirm
                                    }) {
                                        tag.names.push(new);
                                    };
                                });
                            });
                            ui.add_space(4.0);
                            let only_one = tag.names.len() == 1;
                            tag.names.retain_mut(|name| {
                                let mut retain = true;
                                ui.horizontal(|ui| {
                                    ui.text_edit_singleline(name);
                                    if ui
                                        .add_enabled(!only_one, Button::new(icons::REMOVE))
                                        .clicked()
                                    {
                                        retain = false;
                                    }
                                });
                                retain
                            });
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                ui.label("Implies");
                                ui.rtl(|ui| {
                                    let mut confirm = false;
                                    match new_imply {
                                        Some(_) => {
                                            if ui.button(icons::CANCEL).clicked() {
                                                *new_imply = None;
                                            }
                                            if ui.button(icons::CHECK).clicked() {
                                                confirm = true;
                                            }
                                        }
                                        None => {
                                            if ui.button(icons::ADD).clicked() {
                                                *new_imply = Some(String::new());
                                            }
                                        }
                                    }
                                    if let Some(imply) = new_imply.take_if(|imply| {
                                        let re = ui.add(
                                            TextEdit::singleline(imply)
                                                .hint_text("New implication"),
                                        );
                                        (re.lost_focus()
                                            && ui.input(|inp| inp.key_pressed(Key::Enter)))
                                            | confirm
                                    }) {
                                        if let Some(resolved_id) = coll.resolve_tag(&imply) {
                                            let tag = coll.tags.get_mut(id).unwrap();
                                            tag.implies.insert(resolved_id);
                                        }
                                    }
                                })
                            });
                            ui.add_space(4.0);
                            let mut remove = None;
                            for imply_id in &coll.tags[id].implies {
                                ui.horizontal(|ui| {
                                    ui.label(&coll.tags[imply_id].names[0]);
                                    if ui.button(icons::REMOVE).clicked() {
                                        remove = Some(*imply_id);
                                    }
                                });
                            }
                            if let Some(imply_id) = remove {
                                coll.tags.get_mut(id).unwrap().implies.remove(&imply_id);
                            }
                        }
                    }
                });
            });

            if egui_ctx.input(|inp| inp.key_pressed(Key::Escape)) {
                *close_ref = true;
            }
        });
    if close {
        egui_state.just_closed_window_with_esc = true;
        egui_state.tag_window.on = false;
    }
}
