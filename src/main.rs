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
    clippy::missing_panics_doc
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

use crate::application::Application;

fn try_main() -> anyhow::Result<()> {
    let mut app = Application::new()?;
    gui::run(&mut app)
}

fn main() {
    env_logger::init();
    if let Err(e) = try_main() {
        gui::native_dialog::error_blocking("Fatal runtime error", e);
    }
}
