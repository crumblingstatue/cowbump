use {
    super::EguiState,
    crate::{
        collection::Collection,
        gui::{
            egui_ui::{
                entries_window::text_edit_cursor_set_to_end,
                tag_autocomplete::tag_autocomplete_popup,
            },
            State,
        },
    },
    egui_sfml::egui::{Color32, Context, Key, Modifiers, TextEdit},
};

/// Returns whether filter state changed
pub(super) fn do_frame(
    state: &mut State,
    egui_state: &mut EguiState,
    egui_ctx: &Context,
    coll: &mut Collection,
) -> bool {
    let mut text_changed = false;
    let mut success = false;
    egui_state
        .filter_popup
        .do_frame("filter", egui_ctx, |popup, ui| {
            let count = coll.filter(&state.filter).count();
            let up_pressed =
                ui.input_mut(|inp| inp.consume_key(Modifiers::default(), Key::ArrowUp));
            let down_pressed =
                ui.input_mut(|inp| inp.consume_key(Modifiers::default(), Key::ArrowDown));
            let mut te = TextEdit::singleline(&mut popup.string).lock_focus(true);
            if count == 0 {
                te = te.text_color(Color32::RED);
            }
            let re = ui.add(te);
            if popup.ac_state.applied {
                text_edit_cursor_set_to_end(ui, re.id);
            }
            if tag_autocomplete_popup(
                &mut popup.string,
                &mut popup.ac_state,
                coll,
                ui,
                &re,
                up_pressed,
                down_pressed,
            ) {
                state.wipe_search();
                text_changed = true;
            }
            ui.label(format!("{count} results"));
            popup.string.make_ascii_lowercase();
            let enter_pressed = egui_ctx.input(|inp| inp.key_pressed(Key::Enter));
            if enter_pressed || egui_ctx.input(|inp| inp.key_pressed(Key::Escape)) {
                popup.on = false;
            }
            if re.changed() || text_changed || enter_pressed {
                popup.err_string.clear();
                match state.filter.parse_and_resolve(&popup.string, coll) {
                    Ok(()) => {
                        success = true;
                    }
                    Err(e) => {
                        popup.err_string = format!("Error: {e}");
                        success = false;
                    }
                }
                popup.ac_state.input_changed = true;
                state.wipe_search();
                text_changed = true;
            }
            ui.memory_mut(|mem| mem.request_focus(re.id));
        });
    text_changed && success
}
