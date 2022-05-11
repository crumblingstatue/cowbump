use egui_sfml::{
    egui::{Color32, Context, TextEdit},
    sfml::graphics::{RenderTarget, RenderWindow},
};

use crate::{
    collection::Collection,
    gui::{search_goto_cursor, State},
};

use super::EguiState;

pub(super) fn do_frame(
    state: &mut State,
    egui_state: &mut EguiState,
    egui_ctx: &Context,
    coll: &mut Collection,
    win: &RenderWindow,
) {
    egui_state
        .find_popup
        .do_frame("find", egui_ctx, |popup, ui| {
            let mut te = TextEdit::singleline(&mut popup.string);
            if !state.search_success {
                te = te.text_color(Color32::RED);
            }
            let re = ui.add(te);
            match state.search_reqs.parse_and_resolve(&popup.string, coll) {
                Ok(()) => (),
                Err(e) => {
                    ui.label(&format!("Error: {}", e));
                }
            }
            // Avoid a deadlock with this let binding.
            // Inlining it into the if condition causes a deadlock
            let lost_focus = re.lost_focus();
            if ui.input().key_pressed(egui_sfml::egui::Key::Enter) || lost_focus {
                popup.on = false;
            }
            if re.changed() || ui.input().key_pressed(egui_sfml::egui::Key::Enter) {
                state.search_cursor = 0;
                search_goto_cursor(state, coll, win.size().y);
            }
            ui.memory().request_focus(re.id);
        });
}
