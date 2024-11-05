use {
    crate::{
        collection::Collection,
        entry,
        gui::{Activity, State},
        sequence::Sequence,
    },
    egui_sfml::sfml::graphics::RenderWindow,
};

/// Open functionality when enter is pressed in thumbnails view
pub(in crate::gui) fn on_enter_open(state: &mut State, window: &RenderWindow) {
    if state.sel.none_selected() {
        open_list(state, state.thumbs_view.uids.clone(), 0, window);
    } else if let Some(id_vec) = state.sel.current_as_nonempty_id_vec() {
        open_list(state, id_vec.clone(), 0, window);
    }
}

/// Opens the built-in viewer with a list of images and a starting index in that list
pub(in crate::gui) fn open_list(
    state: &mut State,
    image_list: Vec<entry::Id>,
    starting_index: usize,
    window: &RenderWindow,
) {
    state.activity = Activity::Viewer;
    state.viewer_state.image_list = image_list;
    state.viewer_state.index = starting_index;
    state.viewer_state.zoom_to_fit(window);
}

/// Opens a single (usually clicked) entry, and:
///
/// - If it has related seqeuence(s), it opens the first related sequence images along with it
/// - Otherwise, it opens all the currently filtered images in the collection along with it
///
/// The built-in viewer will start on the provided entry
pub(in crate::gui) fn open_single_with_others(
    entry_id: entry::Id,
    coll: &Collection,
    state: &mut State,
    window: &RenderWindow,
    thumb_index: usize,
) -> anyhow::Result<()> {
    if let Some(seq) = coll.get_first_related_sequence_of(entry_id) {
        let Some(image_list) = seq.entry_uids_wrapped_from(entry_id) else {
            anyhow::bail!("Couldn't get wrapped uids");
        };
        open_list(state, image_list, 0, window);
    } else {
        open_list(state, state.thumbs_view.uids.clone(), thumb_index, window);
    };
    Ok(())
}

pub(in crate::gui) fn open_sequence(
    state: &mut State,
    seq: &Sequence,
    start_uid: entry::Id,
    window: &RenderWindow,
) -> anyhow::Result<()> {
    let Some(uids) = seq.entry_uids_wrapped_from(start_uid) else {
        anyhow::bail!("Couldn't get wrapped uids");
    };
    open_list(state, uids, 0, window);
    Ok(())
}
