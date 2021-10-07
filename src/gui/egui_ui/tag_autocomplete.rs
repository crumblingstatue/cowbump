use std::ops::Range;

use egui::{popup_below_widget, InputState, Key};

use crate::{collection::Collection, tag};

pub struct AcState {
    /// Selection index in the autocomplet list
    select: Option<usize>,
    /// Input changed this frame
    pub input_changed: bool,
}

impl Default for AcState {
    fn default() -> Self {
        Self {
            select: Some(0),
            input_changed: true,
        }
    }
}

/// Popup for autocompleting tags.
///
/// Returns whether a suggestion was applied or not.
pub(super) fn tag_autocomplete_popup(
    input: &InputState,
    string: &mut String,
    state: &mut AcState,
    coll: &mut Collection,
    ui: &mut egui::Ui,
    response: &egui::Response,
) -> bool {
    macro ret($x:expr) {
        state.input_changed = false;
        return $x;
    }
    let popup_id = ui.make_persistent_id("tag_completion");
    let mut last = string.split_ascii_whitespace().last().unwrap_or("");
    // Ignore '!' character
    if last.bytes().next() == Some(b'!') {
        last = &last[1..];
    }
    if input.key_pressed(Key::ArrowDown) {
        match &mut state.select {
            None => state.select = Some(0),
            Some(sel) => *sel += 1,
        }
    }
    if let Some(sel) = &mut state.select {
        if input.key_pressed(Key::ArrowUp) {
            if *sel > 0 {
                *sel -= 1;
            } else {
                // Allow selecting "Nothing" by going above first element
                state.select = None;
            }
        }
    } else if state.input_changed {
        // Always select index 0 when input was changed for convenience
        state.select = Some(0);
    }
    if !string.is_empty() {
        let mut exact_match = None;
        macro filt_predicate($tag:expr) {
            $tag.names[0].contains(last)
        }
        // Get length of list and also whether there is an exact match
        let mut i = 0;
        let len = coll
            .tags
            .iter()
            .filter(|(_id, tag)| {
                if tag.names[0] == last {
                    exact_match = Some(i);
                }
                let predicate = filt_predicate!(tag);
                if predicate {
                    i += 1;
                }
                predicate
            })
            .count();
        match exact_match {
            Some(idx) if state.input_changed => state.select = Some(idx),
            _ => {}
        }
        if len > 0 {
            if let Some(selection) = &mut state.select {
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
                        .selectable_label(state.select == Some(0), ":no-tag")
                        .clicked()
                    {
                        complete = C::Special(":no-tag");
                    }
                    if state.select == Some(0)
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
                            .selectable_label(state.select == Some(i), &tag.names[0])
                            .clicked()
                        {
                            complete = C::Id(id);
                        }
                        if state.select == Some(i)
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
                    ret!(true);
                }
                C::Special(special) => {
                    let range = str_range(string, last);
                    string.replace_range(range, special);
                    ret!(true);
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
    ret!(false);
}

fn str_range(parent: &str, sub: &str) -> Range<usize> {
    let beg = sub.as_ptr() as usize - parent.as_ptr() as usize;
    let end = beg + sub.len();
    beg..end
}
