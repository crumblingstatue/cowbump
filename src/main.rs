#![feature(decl_macro)]

mod db;
mod entry;
mod gui;
mod sequence;
mod tag;

use crate::db::{Db, Uid};
use std::{collections::HashMap, env};
use tag::Tag;
use thiserror::Error;

fn main() -> anyhow::Result<()> {
    if !atty::is(atty::Stream::Stdout) {
        return Ok(());
    }
    let dir = env::current_dir().unwrap();
    let mut db = Db::load_from_fs().unwrap_or_else(|e| {
        eprintln!("Error loading db: {}, creating new default db.", e);
        Db::default()
    });
    db.update_from_folder(&dir).unwrap();
    let mut no_save = false;
    gui::run(&mut db, &mut no_save).unwrap();
    if !no_save {
        db.save_to_fs()?;
    }
    Ok(())
}

#[derive(Default)]
pub struct FilterSpec {
    has_tags: Vec<Uid>,
    doesnt_have_tags: Vec<Uid>,
    filename_substring: String,
    doesnt_have_any_tags: bool,
}

#[derive(Error, Debug)]
pub enum ParseResolveError<'a> {
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
        let mut doesnt_have_any_tags = false;
        for word in words {
            let mut is_meta = false;
            if let Some(pos) = word.find(':') {
                let meta = &word[..pos];
                let arg = &word[pos + 1..];
                match (meta, arg) {
                    ("f" | "fname", _) => {
                        fstring = arg.to_owned();
                        is_meta = true;
                    }
                    ("no-tag", _) | (_, "no-tag") => {
                        doesnt_have_any_tags = true;
                        is_meta = true;
                    }
                    _ => {}
                }
            }
            if !is_meta {
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
        Ok(Self {
            has_tags: tags,
            filename_substring: fstring,
            doesnt_have_tags: neg_tags,
            doesnt_have_any_tags,
        })
    }
    pub fn to_spec_string(&self, tags: &HashMap<Uid, Tag>) -> String {
        if self.doesnt_have_any_tags {
            ":notag".into()
        } else {
            let mut out = String::new();
            for tag in &self.has_tags {
                let name = &tags[tag].names[0];
                out.push_str(name);
                out.push(' ');
            }
            for tag in &self.doesnt_have_tags {
                let name = &tags[tag].names[0];
                out.push('!');
                out.push_str(name);
                out.push(' ');
            }
            out.push_str(&self.filename_substring);
            out
        }
    }
    pub fn toggle_has(&mut self, uid: Uid) {
        toggle_vec_elem(&mut self.has_tags, uid);
    }
    pub fn set_has(&mut self, uid: Uid, set: bool) {
        set_vec_elem(&mut self.has_tags, uid, set);
    }
    pub fn toggle_doesnt_have(&mut self, uid: Uid) {
        toggle_vec_elem(&mut self.doesnt_have_tags, uid);
    }
    pub fn set_doesnt_have(&mut self, uid: Uid, set: bool) {
        set_vec_elem(&mut self.doesnt_have_tags, uid, set);
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

fn toggle_vec_elem(vec: &mut Vec<Uid>, uid: Uid) {
    if !remove_vec_elem(vec, uid) {
        insert_vec_elem(vec, uid);
    }
}

fn set_vec_elem(vec: &mut Vec<Uid>, uid: Uid, set: bool) {
    if set {
        insert_vec_elem(vec, uid);
    } else {
        remove_vec_elem(vec, uid);
    }
}

fn remove_vec_elem(vec: &mut Vec<Uid>, uid: Uid) -> bool {
    if let Some(pos) = vec.iter().position(|uid2| *uid2 == uid) {
        vec.remove(pos);
        true
    } else {
        false
    }
}

fn insert_vec_elem(vec: &mut Vec<Uid>, uid: Uid) {
    if !vec.contains(&uid) {
        vec.push(uid);
    }
}
