#![feature(decl_macro)]

mod db;
mod entry;
mod gui;
mod tag;

use crate::db::{Db, Uid};
use std::env;
use thiserror::Error;

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

#[derive(Default)]
pub struct FilterSpec {
    has_tags: Vec<Uid>,
    doesnt_have_tags: Vec<Uid>,
    filename_substring: String,
}

#[derive(Error, Debug)]
pub enum ParseResolveError<'a> {
    #[error("Unknown meta tag: {0}")]
    UnknownMetaArg(&'a str),
    #[error("No such tag: {0}")]
    NoSuchTag(&'a str),
}

impl FilterSpec {
    /// Whether this filter actually filters anything or just shows everything
    pub fn active(&self) -> bool {
        !self.has_tags.is_empty() || !self.filename_substring.is_empty()
    }
    pub fn parse_and_resolve<'a>(string: &'a str, db: &Db) -> Result<Self, ParseResolveError<'a>> {
        let words = string.split_whitespace();
        let mut tags = Vec::new();
        let mut neg_tags = Vec::new();
        let mut fstring = String::new();
        for word in words {
            match word.find(':') {
                Some(pos) => {
                    let meta = &word[..pos];
                    let arg = &word[pos + 1..];
                    match meta {
                        "f" | "fname" => {
                            fstring = arg.to_owned();
                        }
                        _ => {
                            return Err(ParseResolveError::UnknownMetaArg(meta));
                        }
                    }
                }
                None => {
                    let tag_name;
                    let neg;
                    if word.bytes().next() == Some(b'!') {
                        tag_name = &word[1..];
                        neg = true;
                    } else {
                        tag_name = word;
                        neg = false;
                    }
                    let tag_id = match db.resolve_tag(tag_name) {
                        Some(id) => id,
                        None => return Err(ParseResolveError::NoSuchTag(tag_name)),
                    };
                    if !neg {
                        tags.push(tag_id);
                    } else {
                        neg_tags.push(tag_id)
                    }
                }
            }
        }
        Ok(Self {
            has_tags: tags,
            filename_substring: fstring,
            doesnt_have_tags: neg_tags,
        })
    }
}
