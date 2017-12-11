#![feature(conservative_impl_trait)]

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
mod tag;
mod gui;

use db::{Db, Uid};
use std::env;

fn main() {
    let mut db = Db::load_from_fs().unwrap_or_else(|e| {
        eprintln!("Error loading db: {}, creating new default db.", e);
        let mut db = Db::default();
        db.add_from_folder(
            env::args_os()
                .nth(1)
                .expect("Need path to image folder")
                .as_ref(),
        ).unwrap();
        db
    });
    gui::run(&mut db).unwrap();
    db.save_to_fs().unwrap();
}

pub struct FilterSpec {
    has_tags: Vec<Uid>,
}
