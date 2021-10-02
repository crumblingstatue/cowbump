mod application;
mod collection;
mod db;
mod entry;
mod filter_spec;
mod folder_scan;
mod gui;
mod preferences;
mod recently_used_list;
mod sequence;
mod serialization;
pub mod set_ext;
mod tag;

use crate::application::Application;

fn try_main() -> anyhow::Result<()> {
    let mut app = Application::new()?;
    gui::run(&mut app)
}

fn main() {
    if let Err(e) = try_main() {
        gui::native_dialog::error("Fatal runtime error", e)
    }
}
