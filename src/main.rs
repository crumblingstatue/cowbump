#![feature(decl_macro, btree_retain)]

mod db;
mod entry;
mod gui;
mod tag;

use crate::db::{Db, Uid};
use std::env;

fn main() {
    if !atty::is(atty::Stream::Stdout) {
        return;
    }
    let dir = env::current_dir().unwrap();
    let mut db = Db::load_from_fs().unwrap_or_else(|e| {
        eprintln!("Error loading db: {}, creating new default db.", e);
        Db::default()
    });
    db.update_from_folder(&dir).unwrap();
    gui::run(&mut db).unwrap();
    db.save_to_fs().unwrap();
}

pub struct FilterSpec {
    has_tags: Vec<Uid>,
    substring_match: String,
}

impl FilterSpec {
    /// Whether this filter actually filters anything or just shows everything
    pub fn active(&self) -> bool {
        !self.has_tags.is_empty() || !self.substring_match.is_empty()
    }
}
