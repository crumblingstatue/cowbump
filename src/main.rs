mod db;
mod entry;
mod filter_spec;
mod gui;
mod sequence;
pub mod set_ext;
mod tag;

use crate::db::local::LocalDb;
use std::env;

fn main() -> anyhow::Result<()> {
    if !atty::is(atty::Stream::Stdout) {
        return Ok(());
    }
    let dir = env::current_dir()?;
    let mut db = LocalDb::load_from_fs().unwrap_or_else(|e| {
        eprintln!("Error loading db: {}, creating new default db.", e);
        LocalDb::default()
    });
    db.update_from_folder(&dir)?;
    let mut no_save = false;
    gui::run(&mut db, &mut no_save)?;
    if !no_save {
        db.save_to_fs()?;
    }
    Ok(())
}
