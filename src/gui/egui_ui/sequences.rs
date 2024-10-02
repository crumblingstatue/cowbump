use {
    super::{icons, EguiState},
    crate::{
        collection::Collection,
        db::UidCounter,
        dlog, entry,
        gui::{
            open::{
                builtin,
                external::{self},
            },
            State,
        },
        preferences::Preferences,
        sequence::{self},
    },
    egui_sfml::{
        egui::{
            load::SizedTexture, Align, Button, Color32, Context, DragValue, ImageButton, Key,
            ScrollArea, TextEdit, TextureId, Window,
        },
        sfml::graphics::RenderWindow,
    },
};

pub(super) fn do_sequence_windows(
    state: &mut State,
    egui_state: &mut EguiState,
    coll: &mut Collection,
    egui_ctx: &Context,
    prefs: &mut Preferences,
    window: &RenderWindow,
) {
    enum Action {
        SwapLeft,
        SwapRight,
        SwapFirst,
        SwapLast,
        SwapAt(usize),
        Remove,
        Open,
    }

    egui_state.sequence_windows.retain_mut(|win| {
        let mut open = true;
        let Some(seq) = coll.sequences.get_mut(&win.uid) else {
            dlog!("Can't get sequence {:?}", win.uid);
            return false;
        };
        let name = &seq.name;
        let mut action = Action::SwapLeft;
        let mut subject = None;
        if egui_ctx.input(|inp| inp.key_pressed(Key::Escape)) {
            open = false;
            egui_state.just_closed_window_with_esc = true;
        }
        Window::new(format!("Sequence: {name}"))
            .hscroll(true)
            .min_width(3. * 256.)
            .open(&mut open)
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    let seq_entries_len = seq.entries.len();
                    for (i, &img_uid) in seq.entries.iter().enumerate() {
                        ui.vertical(|ui| {
                            let mut img_butt = ImageButton::new(SizedTexture::new(
                                TextureId::User(img_uid.0),
                                (256., 256.),
                            ));
                            if win.focus_req == Some(img_uid) {
                                img_butt = img_butt.tint(Color32::YELLOW);
                            }
                            let re = ui.add(img_butt);
                            if win.focus_req == Some(img_uid) {
                                re.scroll_to_me(Some(Align::Center));
                                win.focus_req = None;
                            }
                            if re.clicked() {
                                action = Action::Open;
                                subject = Some(img_uid);
                            }
                            ui.label(coll.entries[&img_uid].path.to_string_lossy().as_ref());
                            ui.horizontal(|ui| {
                                let mut pos = i;
                                let dv = DragValue::new(&mut pos).range(0..=seq.entries.len() - 1);
                                if ui.add(dv).changed()
                                    && egui_ctx.input(|inp| inp.key_pressed(Key::Enter))
                                {
                                    action = Action::SwapAt(pos);
                                    subject = Some(img_uid);
                                    win.focus_req = subject;
                                }
                                if ui.add_enabled(i > 0, Button::new("⏮")).clicked() {
                                    action = Action::SwapFirst;
                                    subject = Some(img_uid);
                                    win.focus_req = subject;
                                }
                                if ui.add_enabled(i > 0, Button::new("⏴")).clicked() {
                                    action = Action::SwapLeft;
                                    subject = Some(img_uid);
                                    win.focus_req = subject;
                                }
                                if ui
                                    .button(icons::REMOVE)
                                    .on_hover_text("remove from sequence")
                                    .clicked()
                                {
                                    action = Action::Remove;
                                    subject = Some(img_uid);
                                    win.focus_req = subject;
                                }
                                if ui
                                    .add_enabled(i < seq_entries_len - 1, Button::new("⏵"))
                                    .clicked()
                                {
                                    action = Action::SwapRight;
                                    subject = Some(img_uid);
                                    win.focus_req = subject;
                                }
                                if ui
                                    .add_enabled(i < seq_entries_len - 1, Button::new("⏭"))
                                    .clicked()
                                {
                                    action = Action::SwapLast;
                                    subject = Some(img_uid);
                                    win.focus_req = subject;
                                }
                            });
                        });
                    }
                });
            });
        if let Some(uid) = subject {
            match action {
                Action::SwapLeft => {
                    seq.swap_entry_left(uid);
                }
                Action::SwapRight => {
                    seq.swap_entry_right(uid);
                }
                Action::SwapFirst => {
                    seq.reinsert_first(uid);
                }
                Action::SwapLast => {
                    seq.reinsert_last(uid);
                }
                Action::SwapAt(pos) => {
                    seq.reinsert_at(uid, pos);
                }
                Action::Remove => {
                    seq.remove_entry(uid);
                }
                Action::Open => {
                    if prefs.use_built_in_viewer {
                        builtin::open_sequence(state, seq, uid, window);
                    } else {
                        external::open_sequence(seq, uid, &coll.entries, prefs);
                    }
                }
            }
        }
        open
    });
}

pub(super) fn do_sequences_window(
    state: &mut State,
    egui_state: &mut EguiState,
    coll: &mut Collection,
    uid_counter: &mut UidCounter,
    egui_ctx: &Context,
    preferences: &mut Preferences,
    window: &RenderWindow,
) {
    let seq_win = &mut egui_state.sequences_window;
    if seq_win.on {
        let enter_pressed = egui_ctx.input(|inp| inp.key_pressed(Key::Enter));
        let esc_pressed = egui_ctx.input(|inp| inp.key_pressed(Key::Escape));
        if esc_pressed {
            seq_win.on = false;
            egui_state.just_closed_window_with_esc = true;
        }
        Window::new("Sequences")
            .open(&mut seq_win.on)
            .show(egui_ctx, |ui| {
                let mut focus = false;
                ui.horizontal(|ui| {
                    let te = TextEdit::singleline(&mut seq_win.filter_string).hint_text("Filter");
                    ui.add(te);
                    if ui.button(icons::CANCEL).clicked() {
                        seq_win.filter_string.clear();
                    }
                    let txt = if seq_win.pick_mode {
                        "✚ Add to new"
                    } else {
                        "✚ Add new"
                    };
                    if ui.button(txt).clicked() {
                        seq_win.add_new ^= true;
                        focus = true;
                    }
                    if seq_win.pick_mode && ui.button(icons::CANCEL_TEXT).clicked() {
                        seq_win.pick_mode = false;
                    }
                });
                if seq_win.add_new {
                    let line_edit = TextEdit::singleline(&mut seq_win.add_new_buffer)
                        .hint_text("New sequence name");
                    let re = ui.add(line_edit);
                    if focus {
                        re.request_focus();
                    }
                    if enter_pressed {
                        let id = coll.add_new_sequence(&seq_win.add_new_buffer, uid_counter);
                        if seq_win.pick_mode {
                            seq_win.pick_result = Some(id);
                        }
                        seq_win.add_new_buffer.clear();
                        seq_win.add_new = false;
                    }
                }
                ui.separator();
                ScrollArea::vertical().show(ui, |ui| {
                    coll.sequences.retain(|&uid, seq| {
                        let mut retain = true;
                        if !seq
                            .name
                            .to_lowercase()
                            .contains(&seq_win.filter_string.to_lowercase())
                        {
                            return true;
                        }
                        ui.horizontal(|ui| {
                            ui.heading(&seq.name);
                            if seq_win.pick_mode && ui.button("✚ Add to this").clicked() {
                                seq_win.pick_result = Some(uid);
                            }
                            if ui.button("✏ Edit").clicked() {
                                egui_state
                                    .sequence_windows
                                    .push(SequenceWindow::new(uid, None));
                            }
                            let del_butt = Button::new([icons::REMOVE, " Delete"].concat())
                                .fill(Color32::from_rgb(130, 14, 14));
                            if ui.add(del_butt).clicked() {
                                retain = false;
                            }
                        });
                        // Display the first 7 images of the sequence
                        ui.horizontal(|ui| {
                            for en in seq.entries.iter().take(7) {
                                let but = ImageButton::new(SizedTexture::new(
                                    TextureId::User(en.0),
                                    (128., 128.),
                                ));
                                if ui.add(but).clicked() {
                                    if preferences.use_built_in_viewer {
                                        builtin::open_sequence(state, seq, *en, window);
                                    } else {
                                        external::open_sequence(
                                            seq,
                                            *en,
                                            &coll.entries,
                                            preferences,
                                        );
                                    }
                                }
                            }
                        });
                        retain
                    });
                });
            });
    }
}

pub struct SequenceWindow {
    uid: sequence::Id,
    focus_req: Option<entry::Id>,
}

impl SequenceWindow {
    pub fn new(uid: sequence::Id, focus_req: Option<entry::Id>) -> Self {
        Self { uid, focus_req }
    }
}

#[derive(Default)]
pub struct SequencesWindow {
    pub on: bool,
    add_new: bool,
    add_new_buffer: String,
    /// When this is on, we can pick out a sequence and return its id
    pub pick_mode: bool,
    pub pick_result: Option<sequence::Id>,
    filter_string: String,
}
