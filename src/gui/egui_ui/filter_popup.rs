use egui::{Align2, Color32, CtxRef, Key, TextEdit};

use crate::{
    collection::Collection,
    filter_spec::FilterSpec,
    gui::{
        egui_ui::{
            entries_window::text_edit_cursor_set_to_end, tag_autocomplete::tag_autocomplete_popup,
        },
        State,
    },
};

use super::{tag_autocomplete::AcState, EguiState};

#[derive(Default)]
pub struct FilterPopup {
    pub on: bool,
    pub string: String,
    ac_state: AcState,
}

/// Returns whether filter state changed
pub(super) fn do_frame(
    state: &mut State,
    egui_state: &mut EguiState,
    egui_ctx: &CtxRef,
    coll: &mut Collection,
) -> bool {
    let popup = &mut egui_state.filter_popup;
    let mut text_changed = false;
    let mut success = false;
    if popup.on {
        egui::Window::new("Filter")
            .anchor(Align2::LEFT_TOP, [32.0, 32.0])
            .title_bar(false)
            .auto_sized()
            .show(egui_ctx, |ui| {
                let mut err = None;
                ui.horizontal(|ui| {
                    ui.label("filter");
                    let count = coll.filter(&state.filter).count();
                    let te_id = ui.make_persistent_id("text_edit_tag_popup");
                    let mut te = TextEdit::singleline(&mut popup.string)
                        .lock_focus(true)
                        .ignore_up_and_down_keys()
                        .id(te_id);
                    if count == 0 {
                        te = te.text_color(Color32::RED);
                    }
                    if popup.ac_state.applied {
                        text_edit_cursor_set_to_end(ui, te_id);
                    }
                    let re = ui.add(te);
                    let input = egui_ctx.input();
                    if tag_autocomplete_popup(
                        input,
                        &mut popup.string,
                        &mut popup.ac_state,
                        coll,
                        ui,
                        &re,
                    ) {
                        state.wipe_search();
                        text_changed = true;
                    }
                    ui.label(&format!("{} results", count));
                    popup.string.make_ascii_lowercase();
                    match FilterSpec::parse_and_resolve(&popup.string, coll) {
                        Ok(spec) => {
                            state.filter = spec;
                            success = true;
                        }
                        Err(e) => {
                            err = Some(format!("Error: {}", e));
                            success = false;
                        }
                    };
                    if input.key_pressed(Key::Enter) || input.key_pressed(Key::Escape) {
                        popup.on = false;
                    }
                    if re.changed() {
                        popup.ac_state.input_changed = true;
                        state.wipe_search();
                        text_changed = true;
                    }
                    ui.memory().request_focus(re.id);
                });
                if let Some(e) = err {
                    ui.label(e);
                }
            });
    }
    text_changed && success
}
