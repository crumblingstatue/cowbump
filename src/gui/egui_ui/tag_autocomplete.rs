use std::ops::Range;

use egui::{popup_below_widget, InputState, Key};

use crate::{collection::Collection, gui::debug_log::dlog, tag};

/// Popup for autocompleting tags.
///
/// Returns whether a suggestion was applied or not.
pub(super) fn tag_autocomplete_popup(
    input: &InputState,
    string: &mut String,
    selection: &mut Option<usize>,
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
        *selection.get_or_insert(0) += 1;
    }
    if let Some(selection) = selection {
        if input.key_pressed(Key::ArrowUp) && *selection > 0 {
            *selection -= 1;
        }
    }
    if !string.is_empty() {
        let mut exact_match = false;
        macro filt_predicate($tag:expr) {
            $tag.names[0].contains(last)
        }
        // Get length of list and also whether there is an exact match
        let len = coll
            .tags
            .iter()
            .filter(|(_id, tag)| {
                if tag.names[0] == last {
                    exact_match = true;
                }
                filt_predicate!(tag)
            })
            .count();
        match selection {
            None if !exact_match => {
                dlog!("Setting cursor to 0");
                *selection = Some(0);
            }
            Some(_) if exact_match => {
                dlog!("Exact match, clearing");
                *selection = None;
            }
            _ => {}
        }
        if len > 0 {
            if let Some(selection) = selection {
                if *selection >= len {
                    *selection = len - 1;
                }
            }
            enum C {
                Id(tag::Id),
                Special(&'static str),
                Nothing,
            }
            let mut complete = C::Nothing;
            popup_below_widget(ui, popup_id, response, |ui| {
                if last.bytes().next() == Some(b':') {
                    if ui
                        .selectable_label(*selection == Some(0), ":no-tag")
                        .clicked()
                    {
                        complete = C::Special(":no-tag");
                    }
                    if *selection == Some(0)
                        && (input.key_pressed(Key::Tab) || input.key_pressed(Key::Enter))
                    {
                        complete = C::Special(":no-tag");
                    }
                } else {
                    for (i, (&id, tag)) in coll
                        .tags
                        .iter()
                        .filter(|(_id, tag)| filt_predicate!(tag))
                        .enumerate()
                    {
                        if ui
                            .selectable_label(*selection == Some(i), &tag.names[0])
                            .clicked()
                        {
                            complete = C::Id(id);
                        }
                        if *selection == Some(i)
                            && (input.key_pressed(Key::Tab) || input.key_pressed(Key::Enter))
                        {
                            complete = C::Id(id);
                        }
                    }
                }
            });
            match complete {
                C::Id(id) => {
                    let range = str_range(string, last);
                    string.replace_range(range, &coll.tags[&id].names[0]);
                    return true;
                }
                C::Special(special) => {
                    let range = str_range(string, last);
                    string.replace_range(range, special);
                    return true;
                }
                C::Nothing => {}
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
