use {
    crate::{db::Uid, entry},
    serde_derive::{Deserialize, Serialize},
};

/// An ordered sequence of entries
#[derive(Default, Serialize, Deserialize)]
pub struct Sequence {
    pub name: String,
    pub entries: Vec<entry::Id>,
}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Id(pub Uid);

impl Sequence {
    pub fn new_with_name(name: &str) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
    pub(crate) fn reinsert_first(&mut self, id: entry::Id) {
        if let Some(pos) = self.entries.iter().position(|&uid| uid == id) {
            self.entries.remove(pos);
            self.entries.insert(0, id);
        }
    }
    pub(crate) fn reinsert_last(&mut self, id: entry::Id) {
        if let Some(pos) = self.entries.iter().position(|&uid| uid == id) {
            self.entries.remove(pos);
            self.entries.push(id);
        }
    }
    pub(crate) fn reinsert_at(&mut self, id: entry::Id, at: usize) {
        if let Some(pos) = self.entries.iter().position(|&uid| uid == id) {
            self.entries.remove(pos);
            self.entries.insert(at, id);
        }
    }
    pub(crate) fn swap_entry_left(&mut self, id: entry::Id) {
        if let Some(pos) = self.entries.iter().position(|&uid| uid == id) {
            self.entries.swap(pos - 1, pos);
        }
    }
    pub(crate) fn swap_entry_right(&mut self, id: entry::Id) {
        if let Some(pos) = self.entries.iter().position(|&uid| uid == id) {
            self.entries.swap(pos + 1, pos);
        }
    }
    pub(crate) fn remove_entry(&mut self, id: entry::Id) {
        if let Some(pos) = self.entries.iter().position(|&uid| uid == id) {
            self.entries.remove(pos);
        }
    }

    pub(crate) fn entry_uids_wrapped_from(&self, img_uid: entry::Id) -> Vec<entry::Id> {
        let mut uids = Vec::new();
        if let Some(pos) = self.entries.iter().position(|&uid| uid == img_uid) {
            uids.extend_from_slice(&self.entries[pos..]);
            uids.extend_from_slice(&self.entries[..pos]);
        }
        uids
    }
    pub(crate) fn contains_entry(&self, id: entry::Id) -> bool {
        self.entries.contains(&id)
    }
}
