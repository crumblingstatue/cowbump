use std::{path::Path, process::Command};

use anyhow::bail;

use crate::{
    collection::{Collection, Entries},
    entry,
    gui::{
        native_dialog::{self, error},
        State,
    },
    preferences::{AppId, Preferences},
    sequence::Sequence,
};

/// Open functionality when enter is pressed in thumbnails view
pub(in crate::gui) fn on_enter_open(
    state: &mut State,
    coll: &mut Collection,
    preferences: &mut Preferences,
) {
    let mut candidates: Vec<OpenExternCandidate> = Vec::new();
    for &uid in state.selected_uids.iter() {
        candidates.push(OpenExternCandidate {
            path: &coll.entries[&uid].path,
            open_with: None,
        });
    }
    if candidates.is_empty() && !state.filter.is_empty() {
        for uid in coll.filter(&state.filter) {
            candidates.push(OpenExternCandidate {
                path: &coll.entries[&uid].path,
                open_with: None,
            });
        }
    }
    candidates.sort_by_key(|c| c.path);
    if let Err(e) = open(&candidates, preferences) {
        native_dialog::error("Failed to open file", e);
    }
}

pub fn open_single_with_others(
    coll: &mut Collection,
    uid: entry::Id,
    preferences: &mut Preferences,
) {
    if let Some(seq_id) = coll.find_related_sequences(&[uid]).pop() {
        let seq = &coll.sequences[&seq_id];
        open_sequence(seq, uid, &coll.entries, preferences);
    } else if let Err(e) = open(
        {
            let en = &coll.entries[&uid];
            &[OpenExternCandidate {
                path: &en.path,
                open_with: find_open_with_for_entry(en, coll),
            }]
        },
        preferences,
    ) {
        native_dialog::error("Failed to open file", e);
    }
}

/// Candidate for opening with extern app
pub struct OpenExternCandidate<'a> {
    pub path: &'a Path,
    pub open_with: Option<AppId>,
}

pub fn open(
    candidates: &[OpenExternCandidate],
    preferences: &mut Preferences,
) -> anyhow::Result<()> {
    let built_tasks = build_tasks(candidates, preferences)?;
    for task in built_tasks.tasks {
        let app = &preferences.applications[&task.app];
        let mut cmd = Command::new(&app.path);
        feed_args(&app.args_string, &task.args, &mut cmd);
        cmd.spawn()?;
    }
    if built_tasks.remainder.len() >= 5 {
        let msg = "\
        You are trying to open too many unassociated files. This is unsupported.\n\
        See File->Preferences->Associations for app associations.";
        bail!(msg);
    }
    for path in built_tasks.remainder {
        if let Err(e) = open::that(path) {
            error("Error opening", e);
        }
    }
    Ok(())
}

fn find_open_with_for_entry(en: &entry::Entry, coll: &Collection) -> Option<AppId> {
    en.tags
        .iter()
        .find_map(|tag_id| coll.tag_specific_apps.get(tag_id).cloned())
}

struct BuiltTasks<'p> {
    tasks: Vec<Task<'p>>,
    remainder: Vec<&'p Path>,
}

fn build_tasks<'p>(
    candidates: &[OpenExternCandidate<'p>],
    preferences: &mut Preferences,
) -> anyhow::Result<BuiltTasks<'p>> {
    let mut tasks: Vec<Task> = Vec::new();
    let mut remainder = Vec::new();
    for candidate in candidates {
        // Specially handle candidates that have an open_with
        if let Some(app_id) = candidate.open_with {
            tasks.push(Task {
                app: app_id,
                args: vec![candidate.path],
            });
            continue;
        }
        let ext = candidate
            .path
            .extension()
            .map(|ext| ext.to_str().unwrap())
            .unwrap_or("")
            .to_ascii_lowercase();
        match preferences.associations.get(&ext) {
            Some(Some(app_id)) => {
                if let Some(task) = tasks.iter_mut().find(|task| task.app == *app_id) {
                    task.args.push(candidate.path);
                } else {
                    tasks.push(Task {
                        app: *app_id,
                        args: vec![candidate.path],
                    });
                }
            }
            _ => {
                // Make sure extension preference exists, so the user doesn't
                // have to add it manually to the list.
                preferences.associations.insert(ext, None);
                remainder.push(candidate.path);
            }
        }
    }
    Ok(BuiltTasks { tasks, remainder })
}

#[derive(Debug)]
struct Task<'p> {
    app: AppId,
    args: Vec<&'p Path>,
}

pub(crate) fn open_sequence(
    seq: &Sequence,
    start_uid: entry::Id,
    entries: &Entries,
    prefs: &mut Preferences,
) {
    let mut candidates = Vec::new();
    for img_uid in seq.entry_uids_wrapped_from(start_uid) {
        candidates.push(OpenExternCandidate {
            path: entries[&img_uid].path.as_ref(),
            open_with: None,
        });
    }
    if let Err(e) = open(&candidates, prefs) {
        native_dialog::error("Failed to open file", e);
    }
}

pub fn feed_args(args_string: &str, paths: &[&Path], command: &mut Command) {
    if args_string.is_empty() {
        command.args(paths);
    } else {
        args_string.split_whitespace().for_each(|word| {
            if word == "{}" {
                command.args(paths);
            } else {
                command.arg(word);
            }
        })
    }
}
