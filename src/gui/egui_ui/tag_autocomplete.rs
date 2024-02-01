use {
    crate::{collection::Collection, tag},
    egui_sfml::egui::{popup_below_widget, Key, Ui},
    std::ops::Range,
};

pub struct AcState {
    /// Selection index in the autocomplet list
    select: Option<usize>,
    /// Input changed this frame
    pub input_changed: bool,
    /// An autocomplete suggestion was applied
    pub applied: bool,
}

impl Default for AcState {
    fn default() -> Self {
        Self {
            select: Some(0),
            input_changed: true,
            applied: false,
        }
    }
}

/// Popup for autocompleting tags.
///
/// Returns whether a suggestion was applied or not.
pub(super) fn tag_autocomplete_popup(
    string: &mut String,
    state: &mut AcState,
    coll: &mut Collection,
    ui: &mut Ui,
    response: &egui_sfml::egui::Response,
    up_pressed: bool,
    down_pressed: bool,
) -> bool {
    macro ret($x:expr) {
        state.input_changed = false;
        return $x;
    }
    state.applied = false;
    let popup_id = ui.make_persistent_id("tag_completion");
    let last = find_word_to_complete(string);
    if down_pressed {
        match &mut state.select {
            None => state.select = Some(0),
            Some(sel) => *sel += 1,
        }
    }
    if let Some(sel) = &mut state.select {
        if up_pressed {
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
    if !string.is_empty() && !last.is_empty() {
        let mut exact_match = None;
        macro filt_predicate($tag:expr) {
            $tag.names.iter().any(|tag| tag.contains(last))
        }
        // Get length of list and also whether there is an exact match
        let mut i = 0;
        let mut len = coll
            .tags
            .iter()
            .filter(|(_id, tag)| {
                if tag.names.iter().any(|tag| tag == last) {
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
        let specials = ["@any", "@all", "@none", "@f", "@seq", "@untagged"];
        let last_is_special = last.bytes().next() == Some(b'@');
        if last_is_special {
            len += specials.len();
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
                if last_is_special {
                    for (i, special) in specials.into_iter().enumerate() {
                        if ui
                            .selectable_label(state.select == Some(i), special)
                            .clicked()
                        {
                            complete = C::Special(special);
                        }
                        if state.select == Some(i)
                            && (ui.input(|inp| inp.key_pressed(Key::Tab))
                                || ui.input(|inp| inp.key_pressed(Key::Enter)))
                        {
                            complete = C::Special(special);
                        }
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
                            && (ui.input(|inp| inp.key_pressed(Key::Tab))
                                || ui.input(|inp| inp.key_pressed(Key::Enter)))
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
                    state.applied = true;
                    ret!(true);
                }
                C::Special(special) => {
                    let range = str_range(string, last);
                    string.replace_range(range, special);
                    state.applied = true;
                    ret!(true);
                }
                C::Nothing => {}
            }
            if !string.is_empty() {
                ui.memory_mut(|mem| mem.open_popup(popup_id));
            } else {
                ui.memory_mut(|mem| mem.close_popup());
            }
        }
    }
    ret!(false);
}

fn find_word_to_complete(string: &str) -> &str {
    let last_begin = string
        .rfind(|c: char| matches!(c, '[' | ']' | '!') || c.is_whitespace())
        .map(|pos| pos + 1)
        .unwrap_or(0);
    &string[last_begin..]
}

fn str_range(parent: &str, sub: &str) -> Range<usize> {
    let beg = sub.as_ptr() as usize - parent.as_ptr() as usize;
    let end = beg + sub.len();
    beg..end
}
