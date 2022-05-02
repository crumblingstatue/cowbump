use std::{
    io::Read,
    process::{Child, Command, ExitStatus, Stdio},
};

use egui_sfml::{
    egui::{
        epaint::text::cursor::{CCursor, Cursor, PCursor, RCursor},
        text_edit::CursorRange,
        vec2, Button, Color32, Context, ImageButton, Key, Label, Modifiers, PointerButton,
        Response, Rgba, RichText, ScrollArea, Sense, TextEdit, TextureId, Ui, Widget,
    },
    sfml::graphics::{RenderTarget, RenderWindow},
};

use crate::{
    collection::Collection,
    db::Db,
    entry,
    filter_spec::FilterSpec,
    gui::{
        clamp_bottom, common_tags, entries_view::EntriesView, feed_args, get_tex_for_entry,
        native_dialog, open_sequence, open_with_external, Resources, State,
    },
    tag,
};

use super::{
    sequences::SequenceWindow,
    tag_autocomplete::{tag_autocomplete_popup, AcState},
    EguiState,
};

#[derive(Default)]
pub struct EntriesWindow {
    ids: Vec<entry::Id>,
    add_tag_buffer: String,
    rename_buffer: String,
    editing_tags: bool,
    renaming: bool,
    delete_confirm: bool,
    custom_command_prompt: bool,
    cmd_buffer: String,
    args_buffer: String,
    err_str: String,
    new_tags: Vec<String>,
    children: Vec<ChildWrapper>,
    ac_state: AcState,
    window_id: u64,
}

struct ChildWrapper {
    child: Child,
    exit_status: Option<ExitStatus>,
    stdout: String,
    stderr: String,
    name: String,
}

impl ChildWrapper {
    fn new(child: Child, name: String) -> Self {
        Self {
            child,
            exit_status: None,
            stdout: String::new(),
            stderr: String::new(),
            name,
        }
    }
}

impl EntriesWindow {
    pub fn new(ids: Vec<entry::Id>, window_id: u64) -> Self {
        Self {
            ids,
            window_id,
            ..Default::default()
        }
    }
}

fn tag_ui(
    ui: &mut Ui,
    name: &str,
    id: tag::Id,
    del: Option<&mut bool>,
    filter: &mut FilterSpec,
    coll: &Collection,
    filter_string: &mut String,
    changed_filter: &mut bool,
    entries_view: &mut EntriesView,
) -> Response {
    ui.allocate_ui(vec2(200., ui.spacing().interact_size.y + 10.), |ui| {
        ui.group(|ui| {
            let mut text = RichText::new(name);
            if filter.has_tag_by_name(name, coll) {
                text = text.background_color(Color32::from_rgb(20, 100, 20));
            } else if filter.doesnt_have_tag_by_name(name, coll) {
                text = text.background_color(Color32::from_rgb(100, 20, 20))
            }
            let re = ui.add(Label::new(text).sense(Sense::click()));
            if re.clicked_by(PointerButton::Primary) {
                filter.toggle_has(id);
                filter.set_doesnt_have(id, false);
                *filter_string = filter.to_spec_string(&coll.tags);
                *changed_filter = true;
                entries_view.update_from_collection(coll, filter);
            } else if re.clicked_by(PointerButton::Secondary) {
                filter.toggle_doesnt_have(id);
                filter.set_has(id, false);
                *filter_string = filter.to_spec_string(&coll.tags);
                *changed_filter = true;
                entries_view.update_from_collection(coll, filter);
            }
            if let Some(del) = del {
                if ui.button("x").clicked() {
                    *del = true;
                }
            }
        });
    })
    .response
}

fn tag<'a>(
    name: &'a str,
    id: tag::Id,
    del: Option<&'a mut bool>,
    filter: &'a mut FilterSpec,
    coll: &'a Collection,
    filter_string: &'a mut String,
    changed_filter: &'a mut bool,
    entries_view: &'a mut EntriesView,
) -> impl Widget + 'a {
    move |ui: &mut Ui| {
        tag_ui(
            ui,
            name,
            id,
            del,
            filter,
            coll,
            filter_string,
            changed_filter,
            entries_view,
        )
    }
}

pub fn text_edit_cursor_set_to_end(ui: &mut Ui, te_id: egui_sfml::egui::Id) {
    let mut state = TextEdit::load_state(ui.ctx(), te_id).unwrap();
    state.set_cursor_range(Some(CursorRange::one(Cursor {
        ccursor: CCursor {
            index: 0,
            prefer_next_row: false,
        },
        rcursor: RCursor { row: 0, column: 0 },
        pcursor: PCursor {
            paragraph: 0,
            offset: 10000,
            prefer_next_row: false,
        },
    })));
    TextEdit::store_state(ui.ctx(), te_id, state);
}

pub(super) fn do_frame(
    state: &mut State,
    egui_state: &mut EguiState,
    coll: &mut Collection,
    egui_ctx: &Context,
    rend_win: &RenderWindow,
    db: &mut Db,
    res: &Resources,
) {
    egui_state.entries_windows.retain_mut(|win| {
        let mut open = true;
        let n_entries = win.ids.len();
        let title = {
            if win.ids.len() == 1 {
                coll.entries[&win.ids[0]]
                    .path
                    .to_string_lossy()
                    .into_owned()
            } else {
                format!("{} entries", n_entries)
            }
        };
        let esc_pressed = egui_ctx.input().key_pressed(Key::Escape);
        let mut close = esc_pressed;
        egui_sfml::egui::Window::new(title)
            .id(egui_sfml::egui::Id::new("en_window").with(win.window_id))
            .open(&mut open)
            .min_width(960.)
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.set_max_width(512.0);
                        let n_visible_entries = n_entries.min(64);
                        for &id in win.ids.iter().take(n_visible_entries) {
                            let tex_size = get_tex_for_entry(
                                &state.thumbnail_cache,
                                id,
                                coll,
                                &mut state.thumbnail_loader,
                                state.thumbnail_size,
                                res,
                            )
                            .1
                            .size();
                            let ratio = tex_size.x as f32 / tex_size.y as f32;
                            let ts = state.thumbnail_size as f32;
                            let h = match n_entries as u32 {
                                0..=2 => ts,
                                3..=6 => ts / 2.0,
                                7..=15 => ts / 3.0,
                                16..=26 => ts / 4.0,
                                27..=36 => ts / 5.0,
                                37..=56 => ts / 6.0,
                                57.. => ts / 7.0,
                            };
                            let w = h * ratio;
                            if ui
                                .add(ImageButton::new(TextureId::User(id.0), (w, h)))
                                .clicked()
                                && !state.highlight_and_seek_to_entry(id, rend_win.size().y)
                            {
                                // Can't find in view, open it in external instead
                                let paths = [&*coll.entries[&id].path];
                                if let Err(e) = open_with_external(&paths, &mut db.preferences) {
                                    native_dialog::error("Error opening with external", e);
                                }
                            }
                        }
                    });
                    ui.vertical(|ui| {
                        // region: Tags
                        ui.horizontal_wrapped(|ui| {
                            for tagid in common_tags(&win.ids, coll) {
                                let tag_name = match coll.tags.get(&tagid) {
                                    Some(tag) => &tag.names[0],
                                    None => "<unknown tag>",
                                };
                                let mut changed_filter = false;

                                if win.editing_tags {
                                    let mut del = false;
                                    ui.add(tag(
                                        tag_name,
                                        tagid,
                                        Some(&mut del),
                                        &mut state.filter,
                                        coll,
                                        &mut egui_state.filter_popup.string,
                                        &mut changed_filter,
                                        &mut state.entries_view,
                                    ));
                                    if del {
                                        // TODO: This only works for 1 item windows
                                        coll.entries
                                            .get_mut(&win.ids[0])
                                            .unwrap()
                                            .tags
                                            .retain(|&t| t != tagid);
                                        state
                                            .entries_view
                                            .update_from_collection(coll, &state.filter);
                                    }
                                } else {
                                    ui.add(tag(
                                        tag_name,
                                        tagid,
                                        None,
                                        &mut state.filter,
                                        coll,
                                        &mut egui_state.filter_popup.string,
                                        &mut changed_filter,
                                        &mut state.entries_view,
                                    ));
                                }
                                if changed_filter {
                                    state
                                        .entries_view
                                        .update_from_collection(coll, &state.filter);
                                    clamp_bottom(rend_win, state);
                                }
                            }
                        });
                        // endregion

                        let txt = if win.editing_tags {
                            "Stop editing"
                        } else {
                            "Edit tags"
                        };
                        let plus_re = ui.button(txt);
                        if plus_re.clicked() {
                            win.editing_tags ^= true;
                        }
                        if win.editing_tags {
                            let te_id = ui.make_persistent_id("text_edit_add_tag");
                            let up_pressed = ui
                                .input_mut()
                                .consume_key(Modifiers::default(), Key::ArrowUp);
                            let down_pressed = ui
                                .input_mut()
                                .consume_key(Modifiers::default(), Key::ArrowDown);
                            let te = TextEdit::singleline(&mut win.add_tag_buffer)
                                .hint_text("New tags")
                                .id(te_id);
                            if win.ac_state.applied {
                                text_edit_cursor_set_to_end(ui, te_id);
                            }
                            let re = ui.add(te);
                            if re.changed() {
                                win.ac_state.input_changed = true;
                            }
                            tag_autocomplete_popup(
                                &mut win.add_tag_buffer,
                                &mut win.ac_state,
                                coll,
                                ui,
                                &re,
                                up_pressed,
                                down_pressed,
                            );
                            win.add_tag_buffer.make_ascii_lowercase();
                            re.request_focus();
                            if esc_pressed {
                                win.editing_tags = false;
                                win.add_tag_buffer.clear();
                                close = false;
                            }
                            if re.ctx.input().key_pressed(Key::Enter) {
                                let add_tag_buffer: &str = &win.add_tag_buffer;
                                let entry_uids: &[entry::Id] = &win.ids;
                                let tags = add_tag_buffer.split_whitespace();
                                for tag in tags {
                                    match coll.resolve_tag(tag) {
                                        Some(tag_uid) => {
                                            coll.add_tag_for_multi(entry_uids, tag_uid);
                                        }
                                        None => {
                                            win.new_tags.push(tag.to_owned());
                                        }
                                    }
                                }
                                win.add_tag_buffer.clear();
                                win.editing_tags = false;
                                state
                                    .entries_view
                                    .update_from_collection(coll, &state.filter);
                            }
                        }

                        if !win.new_tags.is_empty() {
                            ui.label(
                                "You added the following tags to the entry,\
                                 but they aren't present in the database: ",
                            );
                        }
                        win.new_tags.retain_mut(|tag| {
                            let mut retain = true;
                            ui.horizontal(|ui| {
                                ui.label(&tag[..]);
                                if ui.button("Add").clicked() {
                                    match coll
                                        .add_new_tag_from_text(tag.to_owned(), &mut db.uid_counter)
                                    {
                                        Some(id) => {
                                            coll.add_tag_for_multi(&win.ids, id);
                                            retain = false;
                                        }
                                        None => native_dialog::error(
                                            "Error inserting tag",
                                            anyhow::anyhow!("Already exists"),
                                        ),
                                    }
                                }
                                if ui.button("Cancel").clicked() {
                                    retain = false;
                                }
                            });
                            retain
                        });
                        if ui.button("Add to sequence").clicked() {
                            egui_state.sequences_window.on = true;
                            egui_state.sequences_window.pick_mode = true;
                        }
                        if let Some(uid) = egui_state.sequences_window.pick_result {
                            coll.add_entries_to_sequence(uid, &win.ids);
                            egui_state.sequences_window.pick_mode = false;
                            egui_state.sequences_window.pick_result = None;
                        }
                        if ui
                            .add(Button::new("Run custom command").wrap(false))
                            .clicked()
                        {
                            win.custom_command_prompt ^= true;
                        }
                        if win.custom_command_prompt {
                            if esc_pressed {
                                win.custom_command_prompt = false;
                                close = false;
                            }
                            ui.label("Command");
                            let re = ui.text_edit_singleline(&mut win.cmd_buffer);
                            ui.label("Args (use {} for entry path, or leave empty)");
                            ui.text_edit_singleline(&mut win.args_buffer);
                            if re.ctx.input().key_pressed(egui_sfml::egui::Key::Enter) {
                                let mut cmd = Command::new(&win.cmd_buffer);
                                cmd.stderr(Stdio::piped());
                                cmd.stdin(Stdio::piped());
                                cmd.stdout(Stdio::piped());
                                for uid in &win.ids {
                                    let en = &coll.entries[uid];
                                    feed_args(&win.args_buffer, &[&en.path], &mut cmd);
                                }
                                match cmd.spawn() {
                                    Ok(child) => {
                                        win.err_str.clear();
                                        win.custom_command_prompt = false;
                                        win.children
                                            .push(ChildWrapper::new(child, win.cmd_buffer.clone()));
                                    }
                                    Err(e) => win.err_str = e.to_string(),
                                }
                            }
                            if !win.err_str.is_empty() {
                                ui.add(Label::new(
                                    RichText::new(format!("Error: {}", win.err_str))
                                        .color(Rgba::RED),
                                ));
                            }
                        }
                        win.children.retain_mut(|c_wrap| {
                            ui.separator();
                            ui.heading(&c_wrap.name);
                            let mut retain = true;
                            if let Some(status) = c_wrap.exit_status {
                                ui.label("stdout:");
                                ui.code(&c_wrap.stdout);
                                ui.label("stderr:");
                                ui.code(&c_wrap.stderr);
                                let exit_code_msg = match status.code() {
                                    Some(code) => code.to_string(),
                                    None => "<terminated>".to_string(),
                                };
                                ui.label(&format!(
                                    "Exit code: {} ({})",
                                    exit_code_msg,
                                    status.success()
                                ));
                                return !ui.button("x").clicked();
                            }
                            let mut clicked = false;
                            ui.horizontal(|ui| {
                                clicked = ui.button("x").clicked();
                                ui.label(&format!("[running] ({})", c_wrap.child.id()));
                            });
                            if clicked {
                                let _ = c_wrap.child.kill();
                                return false;
                            }
                            match c_wrap.child.try_wait() {
                                Ok(opt_status) => {
                                    c_wrap.exit_status = opt_status;
                                    if let Some(status) = opt_status {
                                        if !status.success() {
                                            if let Some(stdout) = &mut c_wrap.child.stdout {
                                                let mut buf = String::new();
                                                let _ = stdout.read_to_string(&mut buf);
                                                c_wrap.stdout = buf;
                                            }
                                            if let Some(stderr) = &mut c_wrap.child.stderr {
                                                let mut buf = String::new();
                                                let _ = stderr.read_to_string(&mut buf);
                                                c_wrap.stderr = buf;
                                            }
                                        } else {
                                            retain = false;
                                        }
                                    }
                                }
                                Err(e) => {
                                    win.err_str = e.to_string();
                                }
                            }
                            retain
                        });
                        ui.separator();
                        // region: Rename button
                        if ui
                            .add_enabled(win.ids.len() == 1, Button::new("Rename file").wrap(false))
                            .clicked()
                        {
                            win.renaming ^= true;
                        }
                        if win.renaming {
                            let re = ui.text_edit_singleline(&mut win.rename_buffer);
                            if re.ctx.input().key_pressed(egui_sfml::egui::Key::Enter) {
                                if let Err(e) = coll.rename(win.ids[0], &win.rename_buffer) {
                                    native_dialog::error("File rename error", e);
                                }
                                win.renaming = false;
                            }
                            if re.lost_focus() {
                                win.renaming = false;
                                close = false;
                            }
                            ui.memory().request_focus(re.id);
                        }
                        // endregion
                        // region: Delete button
                        if !win.delete_confirm {
                            if ui
                                .add(Button::new("Delete from disk").wrap(false))
                                .clicked()
                            {
                                win.delete_confirm ^= true;
                            }
                        } else {
                            let del_uids = &mut win.ids;
                            let del_len = del_uids.len();
                            let label_string = if del_len == 1 {
                                format!(
                                    "About to delete {}",
                                    coll.entries[&del_uids[0]].path.display()
                                )
                            } else {
                                format!("About to delete {} entries", del_len)
                            };
                            ui.label(&label_string);
                            ui.horizontal(|ui| {
                                if ui.add(Button::new("Confirm").fill(Color32::RED)).clicked() {
                                    if let Err(e) = remove_entries(
                                        &mut state.entries_view,
                                        del_uids,
                                        coll,
                                        &state.filter,
                                    ) {
                                        native_dialog::error("Error deleting entries", e);
                                    }
                                    win.delete_confirm = false;
                                    close = true;
                                }
                                if esc_pressed || ui.add(Button::new("Cancel")).clicked() {
                                    win.delete_confirm = false;
                                    close = false;
                                }
                            });
                        }
                        // endregion
                    });
                });
                let seqs = coll.find_related_sequences(&win.ids);
                if !seqs.is_empty() {
                    ui.separator();
                    ui.heading("Related sequences");
                    for seq_id in seqs {
                        let seq = &coll.sequences[&seq_id];
                        ui.horizontal(|ui| {
                            ui.label(&seq.name);
                            if ui.button("Edit").clicked() {
                                egui_state
                                    .sequence_windows
                                    .push(SequenceWindow::new(seq_id, None));
                            }
                            if ui.button("Select all").clicked() {
                                state.selected_uids.clear();
                                state.selected_uids.extend(&seq.entries);
                                win.ids = state.selected_uids.clone();
                            }
                        });
                        ui.horizontal(|ui| {
                            ScrollArea::horizontal().show(ui, |ui| {
                                for &img_id in seq.entries.iter() {
                                    let img_but =
                                        ImageButton::new(TextureId::User(img_id.0), (128., 128.));
                                    if ui.add(img_but).clicked() {
                                        open_sequence(
                                            seq,
                                            img_id,
                                            &coll.entries,
                                            &mut db.preferences,
                                        )
                                    }
                                }
                            });
                        });
                    }
                }
            });
        if close {
            egui_state.just_closed_window_with_esc = true;
            open = false;
        }
        open
    });
}

fn remove_entries(
    view: &mut EntriesView,
    entries: &mut Vec<entry::Id>,
    coll: &mut Collection,
    filter_spec: &FilterSpec,
) -> anyhow::Result<()> {
    for uid in entries.drain(..) {
        let path = &coll.entries[&uid].path;
        std::fs::remove_file(path)?;
        view.update_from_collection(coll, filter_spec);
        coll.entries.remove(&uid);
    }
    Ok(())
}
