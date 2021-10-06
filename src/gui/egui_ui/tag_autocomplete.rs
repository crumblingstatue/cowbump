use std::ops::Range;

use egui::{popup_below_widget, InputState, Key};

use crate::collection::Collection;

/// Popup for autocompleting tags.
///
/// Returns whether a suggestion was applied or not.
pub(super) fn tag_autocomplete_popup(
    input: &InputState,
    string: &mut String,
    selection: &mut usize,
    coll: &mut Collection,
    ui: &mut egui::Ui,
    response: &egui::Response,
) -> bool {
    let popup_id = ui.make_persistent_id("tag_completion");
    let mut last = string.split_ascii_whitespace().last().unwrap_or("");
    // Ignore '!' character
    if last.bytes().next() == Some(b'!') {
        last = &last[1..];
    }
    if input.key_pressed(Key::ArrowDown) {
        *selection += 1;
    }
    if input.key_pressed(Key::ArrowUp) && *selection > 0 {
        *selection -= 1;
    }

    if !string.is_empty() {
        let filt = coll.tags.iter().filter(|(_id, tag)| {
            let name = &tag.names[0];
            name.contains(last) && name != last
        });
        let len = filt.clone().count();
        if len > 0 {
            if *selection >= len {
                *selection = len - 1;
            }
            let mut complete = None;
            popup_below_widget(ui, popup_id, response, |ui| {
                for (i, (id, tag)) in filt.enumerate() {
                    if ui
                        .selectable_label(*selection == i, &tag.names[0])
                        .clicked()
                    {
                        complete = Some(id);
                    }
                    if *selection == i
                        && (input.key_pressed(Key::Tab) || input.key_pressed(Key::Enter))
                    {
                        complete = Some(id);
                    }
                }
            });
            if let Some(id) = complete {
                let range = str_range(string, last);
                string.replace_range(range, &coll.tags[id].names[0]);
                return true;
            }
            if !string.is_empty() {
                ui.memory().open_popup(popup_id);
            } else {
                ui.memory().close_popup();
            }
        }
    }
    false
}

fn str_range(parent: &str, sub: &str) -> Range<usize> {
    let beg = sub.as_ptr() as usize - parent.as_ptr() as usize;
    let end = beg + sub.len();
    beg..end
}
