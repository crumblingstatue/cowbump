use egui_sfml::{
    egui::{Align2, Color32, Context, TextEdit},
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
    if egui_state.find_popup.on {
        egui_sfml::egui::Window::new("Find")
            .anchor(Align2::LEFT_TOP, [32.0, 32.0])
            .title_bar(false)
            .auto_sized()
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("find");
                    let mut te = TextEdit::singleline(&mut egui_state.find_popup.string);
                    if !state.search_success {
                        te = te.text_color(Color32::RED);
                    }
                    let re = ui.add(te);
                    match state
                        .search_reqs
                        .parse_and_resolve(&egui_state.find_popup.string, coll)
                    {
                        Ok(()) => (),
                        Err(e) => {
                            ui.label(&format!("Error: {}", e));
                        }
                    }
                    // Avoid a deadlock with this let binding.
                    // Inlining it into the if condition causes a deadlock
                    let lost_focus = re.lost_focus();
                    if ui.input().key_pressed(egui_sfml::egui::Key::Enter) || lost_focus {
                        egui_state.find_popup.on = false;
                    }
                    if re.changed() || ui.input().key_pressed(egui_sfml::egui::Key::Enter) {
                        state.search_cursor = 0;
                        search_goto_cursor(state, coll, win.size().y);
                    }
                    ui.memory().request_focus(re.id);
                });
            });
    }
}
