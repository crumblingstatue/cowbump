use {
    super::{
        entries_window::text_edit_cursor_set_to_end, tag_autocomplete::tag_autocomplete_popup,
        EguiState,
    },
    crate::{
        collection::Collection,
        gui::{thumbnails_view::search_goto_cursor, State},
    },
    egui_sfml::{
        egui::{Context, Key, Modifiers, TextEdit},
        sfml::graphics::{RenderTarget, RenderWindow},
    },
};

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
            let up_pressed =
                ui.input_mut(|inp| inp.consume_key(Modifiers::default(), Key::ArrowUp));
            let down_pressed =
                ui.input_mut(|inp| inp.consume_key(Modifiers::default(), Key::ArrowDown));
            let te = TextEdit::singleline(&mut popup.string).lock_focus(true);
            let re = ui.add(te);
            if popup.ac_state.applied {
                text_edit_cursor_set_to_end(ui, re.id);
            }
            let mut text_changed = false;
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
            popup.string.make_ascii_lowercase();
            let enter_pressed = egui_ctx.input(|inp| inp.key_pressed(Key::Enter));
            if enter_pressed || egui_ctx.input(|inp| inp.key_pressed(Key::Escape)) {
                popup.on = false;
            }
            if re.changed() || text_changed || enter_pressed {
                popup.err_string.clear();
                match state.find_reqs.parse_and_resolve(&popup.string, coll) {
                    Ok(()) => {
                        if enter_pressed {
                            state.search_cursor = 0;
                            search_goto_cursor(state, coll, win.size().y);
                        }
                    }
                    Err(e) => {
                        popup.err_string = format!("Error: {e}");
                    }
                }
                popup.ac_state.input_changed = true;
            }
            ui.memory_mut(|mem| mem.request_focus(re.id));
        });
}
