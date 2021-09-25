mod application;
mod collection;
mod db;
mod entry;
mod filter_spec;
mod gui;
mod preferences;
mod recently_used_list;
mod sequence;
mod serialization;
pub mod set_ext;
mod tag;

use rfd::MessageDialog;

use crate::application::Application;

fn try_main() -> anyhow::Result<()> {
    let mut app = Application::new()?;
    gui::run(&mut app)
}

fn main() {
    if let Err(e) = try_main() {
        MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Error")
            .set_description(&e.to_string())
            .show();
    }
}
