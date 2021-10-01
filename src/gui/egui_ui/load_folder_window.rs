use std::{
    borrow::Cow,
    io, mem,
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, channel, Receiver, Sender},
        Arc,
    },
    thread::JoinHandle,
};

use egui::{
    vec2, Align, Button, Color32, CtxRef, Key, Label, ProgressBar, ScrollArea, Sense, Window,
};
use sfml::{graphics::Texture, SfBox};

use crate::{
    application::Application,
    collection::Collection,
    folder_scan::walkdir,
    gui::{thumbnail_loader, Resources, State},
};

#[derive(Default)]
pub struct LoadFolderWindow {
    open: bool,
    state: Option<LoadingState>,
    results: Vec<PathAddResult>,
    root: Arc<PathBuf>,
    /// Selection marker for items in result window
    res_select: Option<usize>,
    res_hover: Option<usize>,
    pub texture: Option<SfBox<Texture>>,
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
    start_loading(win);
}

pub(super) fn do_frame(
    state: &mut State,
    egui_ctx: &CtxRef,
    resources: &Resources,
    app: &mut Application,
) {
    let win = &mut state.egui_state.load_folder_window;
    let input = egui_ctx.input();
    let mut new_sel = None;
    if input.key_pressed(Key::ArrowUp) {
        if let Some(sel) = win.res_select.as_mut() {
            if *sel > 0 {
                *sel -= 1;
                new_sel = Some(*sel);
            }
        }
    }
    if input.key_pressed(Key::ArrowDown) {
        if let Some(sel) = win.res_select.as_mut() {
            *sel += 1;
            new_sel = Some(*sel);
        }
    }
    if !win.open {
        return;
    }
    let mut cancel = false;
    Window::new("Load folder")
        .collapsible(false)
        .fixed_size(vec2(640., 640.))
        .show(egui_ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Folder ");
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
                                    let mut did_select_new = false;
                                    if re.clicked() {
                                        win.res_select = Some(i);
                                        did_select_new = true;
                                    }
                                    if new_sel == Some(i) {
                                        re.scroll_to_me(Align::Center);
                                        did_select_new = true;
                                    }
                                    if did_select_new {
                                        if let Ok(image) = image::open(win.root.join(&path.path)) {
                                            let buf = image.to_rgba8();
                                            let tex = thumbnail_loader::imagebuf_to_sf_tex(buf);
                                            win.texture = Some(tex);
                                        } else {
                                            win.texture = Some(resources.error_texture.clone());
                                        }
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
                    button = Button::new("Create new collection").enabled(done);
                    if ui.add(button).clicked() {
                        let paths = win
                            .results
                            .drain(..)
                            .filter_map(|res| match res {
                                Ok(mut path_add) => {
                                    if path_add.add {
                                        Some(mem::take(&mut path_add.path))
                                    } else {
                                        None
                                    }
                                }
                                Err(_) => None,
                            })
                            .collect::<Vec<_>>();
                        let coll =
                            Collection::make_new(&mut app.database.uid_counter, &paths).unwrap();
                        let id = app.add_collection(coll, (*win.root).clone());
                        crate::gui::set_active_collection(&mut state.entries_view, app, id)
                            .unwrap();
                        *win = Default::default();
                    }
                    let pb = ProgressBar::new(0.0).animate(!done).desired_width(16.0);
                    ui.add(pb);
                    ui.label(&format!("{} results", win.results.len()));
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
    let wd = walkdir(root);
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
