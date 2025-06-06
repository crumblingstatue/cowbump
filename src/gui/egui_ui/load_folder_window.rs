use {
    super::{EguiState, icons},
    crate::{
        application::Application,
        collection::Collection,
        dlog,
        folder_scan::walkdir,
        gui::{State, resources::Resources, thumbnail_loader},
    },
    constcat::concat,
    egui_sf2g::{
        egui::{
            self, Align, Button, Color32, Context, Key, Label, ProgressBar, RichText, ScrollArea,
            Sense, Window, vec2,
        },
        sf2g::{cpp::FBox, graphics::Texture},
    },
    std::{
        ffi::OsStr,
        io, mem,
        path::{Path, PathBuf},
        sync::{
            Arc,
            mpsc::{self, Receiver, Sender, channel},
        },
        thread::JoinHandle,
    },
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
    pub texture: Option<FBox<Texture>>,
    ign_ext_buf: String,
}

struct PathAdd {
    path: PathBuf,
    add: bool,
}

struct LoadingState {
    join_handle: Option<JoinHandle<anyhow::Result<()>>>,
    receiver: Receiver<PathResult>,
}

type PathResult = io::Result<PathBuf>;
type PathAddResult = io::Result<PathAdd>;

fn start_loading(win: &mut LoadFolderWindow) {
    let path_clone = win.root.clone();
    let (sender, receiver) = channel();
    let join_handle = std::thread::spawn(move || read_dir_entries(path_clone.as_ref(), &sender));
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
    egui_state: &mut EguiState,
    egui_ctx: &Context,
    resources: &Resources,
    app: &mut Application,
    window_width: u32,
) {
    let win = &mut egui_state.load_folder_window;
    let mut new_sel = None;
    if egui_ctx.input(|inp| inp.key_pressed(Key::ArrowUp))
        && let Some(sel) = win.res_select.as_mut()
        && *sel > 0
    {
        *sel -= 1;
        new_sel = Some(*sel);
    }
    if egui_ctx.input(|inp| inp.key_pressed(Key::ArrowDown))
        && let Some(sel) = win.res_select.as_mut()
    {
        *sel += 1;
        new_sel = Some(*sel);
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
                let label = Label::new(
                    RichText::new(win.root.to_string_lossy().as_ref())
                        .heading()
                        .color(Color32::YELLOW),
                );
                ui.add(label);
            });
            ui.separator();
            let mut done = false;
            if let Some(loading_state) = &mut win.state {
                done = update(loading_state, &mut win.results);
                // Rough row height calc
                let row_h = ui
                    .vertical_centered(|ui| {
                        ui.label("contents")
                            .on_hover_text("Egui shenanigans, lol")
                            .rect
                            .height()
                    })
                    .inner;
                ScrollArea::vertical().auto_shrink(false).show_rows(
                    ui,
                    row_h,
                    win.results.len(),
                    |ui: &mut egui::Ui, range| {
                        for (i, res) in win.results[range].iter_mut().enumerate() {
                            match res {
                                Ok(path) => {
                                    ui.horizontal(|ui| {
                                        ui.checkbox(&mut path.add, "");
                                        let mut rich_text =
                                            RichText::new(&*path.path.to_string_lossy());
                                        if win.res_select == Some(i) {
                                            rich_text = rich_text
                                                .background_color(Color32::from_rgb(100, 40, 110));
                                        }
                                        if win.res_hover == Some(i) {
                                            rich_text = rich_text.color(Color32::WHITE);
                                        }
                                        let re =
                                            ui.add(Label::new(rich_text).sense(Sense::click()));
                                        if re.hovered() {
                                            win.res_hover = Some(i);
                                        }
                                        let mut did_select_new = false;
                                        if re.clicked() {
                                            win.res_select = Some(i);
                                            did_select_new = true;
                                        }
                                        if new_sel == Some(i) {
                                            re.scroll_to_me(Some(Align::Center));
                                            did_select_new = true;
                                        }
                                        if did_select_new {
                                            if let Ok(image) =
                                                image::open(win.root.join(&path.path))
                                            {
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
                                    let rich_text =
                                        RichText::new(e.to_string()).color(Color32::RED);
                                    ui.add(Label::new(rich_text));
                                }
                            }
                        }
                    },
                );
            };
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Ignored extensions (comma separated)");
                ui.text_edit_singleline(&mut win.ign_ext_buf);
                if ui.button(concat!(icons::CHECK, " Apply")).clicked() {
                    let ign_exts = win.ign_ext_buf.to_ignore_vec();
                    win.results.retain(|res| {
                        let mut retain = true;
                        if let Ok(en) = res {
                            let ext_matches = en.path.extension().is_some_and(|ext| {
                                ign_exts
                                    .iter()
                                    .any(|block_ext| ext == AsRef::<OsStr>::as_ref(block_ext))
                            });
                            if ext_matches {
                                retain = false;
                            }
                        }
                        retain
                    });
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(icons::CANCEL_TEXT).clicked() {
                    cancel = true;
                }
                let button;
                if win.state.is_some() {
                    button = Button::new("🗋 Create new collection");
                    if ui.add_enabled(done, button).clicked() {
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
                        let mut coll = Collection::make_new(&mut app.database.uid_counter, &paths);
                        coll.ignored_extensions = win
                            .ign_ext_buf
                            .to_ignore_vec()
                            .into_iter()
                            .map(ToOwned::to_owned)
                            .collect();
                        let id = app.add_collection(coll, (*win.root).clone());
                        if let Err(e) = crate::gui::set_active_collection(
                            &mut state.thumbs_view,
                            app,
                            id,
                            &state.filter,
                            window_width,
                        ) {
                            egui_state
                                .modal
                                .err(format!("Failed to set active collection: {e:?}"));
                        }
                        *win = Default::default();
                    }
                    let pb = ProgressBar::new(0.0).animate(!done).desired_width(16.0);
                    ui.add(pb);
                    ui.label(format!("{} results", win.results.len()));
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
///
/// # Panics
///
/// Some weird thread shenanigans might cause a panic
fn update(load_state: &mut LoadingState, result_vec: &mut Vec<Result<PathAdd, io::Error>>) -> bool {
    const UPDATE_CHUNK: usize = 128;
    for _ in 0..UPDATE_CHUNK {
        match load_state.receiver.try_recv() {
            Ok(data) => result_vec.push(path_result_conv(data)),
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => {
                if let Some(jh) = load_state.join_handle.take() {
                    let result = jh.join().unwrap();
                    if let Err(e) = result {
                        dlog!("Load folder update error: {e}");
                    }
                }
                return true;
            }
        }
    }
    false
}

fn read_dir_entries(root: &Path, sender: &Sender<PathResult>) -> anyhow::Result<()> {
    let wd = walkdir(root);
    for dir_entry in wd {
        let dir_entry = match dir_entry {
            Ok(en) => en,
            Err(e) => {
                sender.send(Err(e.into()))?;
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
                eprintln!("Failed to add entry {dir_entry_path:?}: {e}");
                continue;
            }
        };
        sender.send(Ok(dir_entry_path.to_owned()))?;
    }
    Ok(())
}

trait IgnoreStrExt {
    fn to_ignore_vec(&self) -> Vec<&str>;
}

impl IgnoreStrExt for str {
    fn to_ignore_vec(&self) -> Vec<&str> {
        self.split(',').map(str::trim).collect()
    }
}
