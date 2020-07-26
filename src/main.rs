extern crate bincode;
extern crate failure;
extern crate image;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate sfml;
extern crate walkdir;

mod db;
mod entry;
mod gui;
mod tag;

use crate::db::{Db, Uid};
use std::env;

fn main() {
    let dir = env::current_dir().unwrap();
    let mut db = Db::load_from_fs().unwrap_or_else(|e| {
        eprintln!("Error loading db: {}, creating new default db.", e);
        let mut db = Db::default();
        db.add_from_folder(&dir).unwrap();
        db
    });
    gui::run(&mut db).unwrap();
    db.save_to_fs().unwrap();
}

pub struct FilterSpec {
    has_tags: Vec<Uid>,
}
