#![feature(try_blocks, let_chains)]
#![windows_subsystem = "windows"]
#![warn(
    unused_qualifications,
    single_use_lifetimes,
    redundant_imports,
    trivial_casts,
    clippy::unnecessary_wraps,
    clippy::uninlined_format_args,
    clippy::semicolon_if_nothing_returned,
    clippy::doc_markdown,
    clippy::missing_panics_doc,
    clippy::explicit_iter_loop,
    clippy::redundant_closure_for_method_calls,
    clippy::cast_lossless,
    clippy::cloned_instead_of_copied,
    clippy::map_unwrap_or,
    clippy::items_after_statements,
    clippy::manual_let_else,
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    //clippy::indexing_slicing <- TODO: Enable when I'm feeling more motivated
)]

mod application;
mod collection;
mod db;
mod entry;
mod entry_utils;
mod filter_reqs;
mod folder_scan;
mod gui;
mod preferences;
mod sequence;
mod serialization;
mod tag;

const VERSION: &str = env!("CARGO_PKG_VERSION");

use {
    crate::application::Application,
    egui_sfml::{
        egui,
        sfml::{
            graphics::RenderWindow,
            window::{Event, Style},
        },
        SfEgui,
    },
};

fn try_main() -> anyhow::Result<()> {
    let mut app = Application::new()?;
    gui::run(&mut app)
}

fn main() {
    env_logger::init();
    if let Err(e) = try_main() {
        error_blocking("Fatal runtime error", e);
    }
}

/// Show a blocking error window
///
/// # Panics
///
/// If the egui pass fails, and maybe some other catastrophic stuff
fn error_blocking<E: std::fmt::Debug>(title: &str, err: E) {
    let mut rw = RenderWindow::new((800, 600), title, Style::DEFAULT, &Default::default());
    rw.set_framerate_limit(60);
    let mut sf_egui = SfEgui::new(&rw);
    while rw.is_open() {
        while let Some(ev) = rw.poll_event() {
            sf_egui.add_event(&ev);
            sf_egui.begin_pass();
            egui::CentralPanel::default().show(sf_egui.context(), |ui| {
                ui.label(format!("{err:?}"));
            });
            sf_egui.end_pass(&mut rw).unwrap();
            if let Event::Closed = ev {
                rw.close();
            }
        }
        sf_egui.draw(&mut rw, None);
        rw.display();
    }
}
