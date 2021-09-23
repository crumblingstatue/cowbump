use egui::{CtxRef, ImageButton, Key, TextureId, Window};
use retain_mut::RetainMut;

use crate::{
    db::local::LocalDb,
    gui::{open_with_external, State},
    sequence,
};

pub(super) fn do_sequence_windows(state: &mut State, db: &mut LocalDb, egui_ctx: &CtxRef) {
    state.egui_state.sequence_windows.retain_mut(|win| {
        let mut open = true;
        let seq = db.sequences.get_mut(&win.uid).unwrap();
        let name = &seq.name;
        enum Action {
            SwapLeft,
            SwapRight,
            Remove,
            Open,
        }
        let mut action = Action::SwapLeft;
        let mut subject = None;
        Window::new(&format!("Sequence: {}", name))
            .hscroll(true)
            .min_width(3. * 256.)
            .open(&mut open)
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    let seq_entries_len = seq.entries.len();
                    for (i, &img_uid) in seq.entries.iter().enumerate() {
                        ui.vertical(|ui| {
                            let img_butt =
                                ImageButton::new(TextureId::User(img_uid.0), (256., 256.));
                            if ui.add(img_butt).clicked() {
                                action = Action::Open;
                                subject = Some(img_uid);
                            }
                            ui.horizontal(|ui| {
                                if i > 0 && ui.button("<").clicked() {
                                    action = Action::SwapLeft;
                                    subject = Some(img_uid);
                                }
                                if ui.button("-").clicked() {
                                    action = Action::Remove;
                                    subject = Some(img_uid);
                                }
                                if i < seq_entries_len - 1 && ui.button(">").clicked() {
                                    action = Action::SwapRight;
                                    subject = Some(img_uid);
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
                Action::Remove => {
                    seq.remove_entry(uid);
                }
                Action::Open => {
                    let mut paths = Vec::new();
                    for img_uid in seq.entry_uids_wrapped_from(uid) {
                        paths.push(db.entries[&img_uid].path.as_ref());
                    }
                    open_with_external(&paths);
                }
            }
        }
        open
    });
}

pub(super) fn do_sequences_window(state: &mut State, db: &mut LocalDb, egui_ctx: &CtxRef) {
    let seq_win = &mut state.egui_state.sequences_window;
    if seq_win.on {
        let enter_pressed = egui_ctx.input().key_pressed(Key::Enter);
        let esc_pressed = egui_ctx.input().key_pressed(Key::Escape);
        if esc_pressed {
            seq_win.on = false;
            state.egui_state.just_closed_window_with_esc = true;
        }
        Window::new("Sequences")
            .open(&mut seq_win.on)
            .show(egui_ctx, |ui| {
                if ui.button("+").clicked() {
                    seq_win.add_new ^= true;
                }
                if seq_win.add_new {
                    ui.text_edit_singleline(&mut seq_win.add_new_buffer);
                    if enter_pressed {
                        db.add_new_sequence(&seq_win.add_new_buffer);
                    }
                }
                ui.separator();
                db.sequences.retain(|&uid, seq| {
                    if ui.button(&seq.name).clicked() {
                        state
                            .egui_state
                            .sequence_windows
                            .push(SequenceWindow::new(uid));
                    }
                    true
                });
            });
    }
}

pub struct SequenceWindow {
    uid: sequence::Id,
}

impl SequenceWindow {
    fn new(uid: sequence::Id) -> Self {
        Self { uid }
    }
}

#[derive(Default)]
pub struct SequencesWindow {
    pub on: bool,
    add_new: bool,
    add_new_buffer: String,
}
