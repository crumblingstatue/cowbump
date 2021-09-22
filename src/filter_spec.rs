use crate::{
    db::{
        local::{LocalDb, Tags},
        Uid, UidSet,
    },
    set_ext::SetExt,
};
use thiserror::Error;

#[derive(Default)]
pub struct FilterSpec {
    pub has_tags: UidSet,
    pub doesnt_have_tags: UidSet,
    pub filename_substring: String,
    pub doesnt_have_any_tags: bool,
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
    pub fn parse_and_resolve<'a>(
        string: &'a str,
        db: &LocalDb,
    ) -> Result<Self, ParseResolveError<'a>> {
        let words = string.split_whitespace();
        let mut tags = UidSet::default();
        let mut neg_tags = UidSet::default();
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
                    tags.insert(tag_id);
                } else {
                    neg_tags.insert(tag_id);
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
    pub fn to_spec_string(&self, tags: &Tags) -> String {
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
        self.has_tags.toggle(uid);
    }
    pub fn set_has(&mut self, uid: Uid, set: bool) {
        self.has_tags.toggle_by(uid, set);
    }
    pub fn toggle_doesnt_have(&mut self, uid: Uid) {
        self.doesnt_have_tags.toggle(uid);
    }
    pub fn set_doesnt_have(&mut self, uid: Uid, set: bool) {
        self.doesnt_have_tags.toggle_by(uid, set);
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
