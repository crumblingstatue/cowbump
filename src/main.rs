#![feature(decl_macro)]

mod db;
mod entry;
mod filter_spec;
mod gui;
mod sequence;
mod tag;

use crate::db::{local::LocalDb, Uid};
use std::env;

fn main() -> anyhow::Result<()> {
    if !atty::is(atty::Stream::Stdout) {
        return Ok(());
    }
    let dir = env::current_dir().unwrap();
    let mut db = LocalDb::load_from_fs().unwrap_or_else(|e| {
        eprintln!("Error loading db: {}, creating new default db.", e);
        LocalDb::default()
    });
    db.update_from_folder(&dir).unwrap();
    let mut no_save = false;
    gui::run(&mut db, &mut no_save).unwrap();
    if !no_save {
        db.save_to_fs()?;
    }
    Ok(())
}
