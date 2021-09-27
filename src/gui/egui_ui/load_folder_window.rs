use std::{
    borrow::Cow,
    io,
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, channel, Receiver, Sender},
        Arc,
    },
    thread::JoinHandle,
};

use egui::{
    vec2, Button, Checkbox, Color32, CtxRef, Label, ProgressBar, ScrollArea, Sense, Window,
};
use walkdir::WalkDir;

use crate::gui::State;

#[derive(Default)]
pub struct LoadFolderWindow {
    open: bool,
    state: Option<LoadingState>,
    results: Vec<PathAddResult>,
    root: Arc<PathBuf>,
    /// Selection marker for items in result window
    res_select: Option<usize>,
    res_hover: Option<usize>,
}

struct PathAdd {
    path: PathBuf,
    add: bool,
}

struct LoadingState {
    join_handle: Option<JoinHandle<()>>,
    receiver: Receiver<PathResult>,
}

type PathResult = io::Result<PathBuf>;
type PathAddResult = io::Result<PathAdd>;

fn start_loading(win: &mut LoadFolderWindow) {
    let path_clone = win.root.clone();
    let (sender, receiver) = channel();
    let join_handle = std::thread::spawn(move || {
        read_dir_entries(path_clone.as_ref(), sender);
    });
    let loading_state = LoadingState {
        join_handle: Some(join_handle),
        receiver,
    };
    win.state = Some(loading_state);
}

pub(super) fn open(win: &mut LoadFolderWindow, path: PathBuf) {
    let path_arc = Arc::new(path);
    win.open = true;
    win.root = path_arc;
}

pub(super) fn do_frame(state: &mut State, egui_ctx: &CtxRef) {
    let win = &mut state.egui_state.load_folder_window;
    if !win.open {
        return;
    }
    let mut cancel = false;
    Window::new("Load folder")
        .collapsible(false)
        .fixed_size(vec2(640., 640.))
        .show(egui_ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Load folder ");
                let label = Label::new(win.root.to_string_lossy().as_ref())
                    .heading()
                    .text_color(Color32::YELLOW);
                ui.add(label);
            });
            ui.separator();
            let mut done = false;
            if let Some(loading_state) = &mut win.state {
                done = update(loading_state, &mut win.results);
                ScrollArea::vertical().show(ui, |ui| {
                    for (i, res) in win.results.iter_mut().enumerate() {
                        match res {
                            Ok(path) => {
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut path.add, "");
                                    let mut label = Label::new(&path.path.to_string_lossy())
                                        .sense(Sense::click());
                                    if win.res_select == Some(i) {
                                        label =
                                            label.background_color(Color32::from_rgb(100, 40, 110));
                                    }
                                    if win.res_hover == Some(i) {
                                        label = label.text_color(Color32::WHITE);
                                    }
                                    let re = ui.add(label);
                                    if re.hovered() {
                                        win.res_hover = Some(i);
                                    }
                                    if re.clicked() {
                                        win.res_select = Some(i);
                                    }
                                });
                            }
                            Err(e) => {
                                let label =
                                    Label::new(Cow::Owned(e.to_string())).text_color(Color32::RED);
                                ui.add(label);
                            }
                        }
                    }
                });
            };
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
                let button;
                if win.state.is_some() {
                    button = Button::new("Add").enabled(done);
                    if ui.add(button).clicked() {}
                    let pb = ProgressBar::new(0.0).animate(!done).desired_width(16.0);
                    ui.add(pb);
                    ui.label(&format!("{} results", win.results.len()));
                } else if ui.button("Start").clicked() {
                    start_loading(win);
                }
            });
        });
    if cancel {
        *win = Default::default();
    }
}

fn path_result_conv(src: PathResult) -> PathAddResult {
    src.map(|path| PathAdd { path, add: true })
}

/// Returns whether we're finished
fn update(load_state: &mut LoadingState, result_vec: &mut Vec<Result<PathAdd, io::Error>>) -> bool {
    const UPDATE_CHUNK: usize = 128;
    for _ in 0..UPDATE_CHUNK {
        match load_state.receiver.try_recv() {
            Ok(data) => result_vec.push(path_result_conv(data)),
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => {
                if let Some(jh) = load_state.join_handle.take() {
                    jh.join().unwrap();
                }
                return true;
            }
        }
    }
    false
}

fn read_dir_entries(root: &Path, sender: Sender<PathResult>) {
    let wd = WalkDir::new(root).sort_by(|a, b| a.file_name().cmp(b.file_name()));
    for dir_entry in wd {
        let dir_entry = match dir_entry {
            Ok(en) => en,
            Err(e) => {
                sender.send(Err(e.into())).unwrap();
                continue;
            }
        };
        if dir_entry.file_type().is_dir() {
            continue;
        }
        let dir_entry_path = dir_entry.into_path();
        let dir_entry_path = match dir_entry_path.strip_prefix(root) {
            Ok(stripped) => stripped,
            Err(e) => {
                eprintln!("Failed to add entry {:?}: {}", dir_entry_path, e);
                continue;
            }
        };
        sender.send(Ok(dir_entry_path.to_owned())).unwrap();
    }
}

/*
match app.load_folder(dir_path) {
                                Ok(()) => {
                                    state.entries_view = EntriesView::from_collection(
                                        &app.database.collections[&app.active_collection.unwrap()],
                                    );
                                }
                                Err(e) => {
                                    MessageDialog::new()
                                        .set_title("Error")
                                        .set_description(&e.to_string())
                                        .show();
                                }
                            }
 */
