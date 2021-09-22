use crate::Uid;
use serde_derive::{Deserialize, Serialize};

/// An ordered sequence of entries
#[derive(Default, Serialize, Deserialize)]
pub struct Sequence {
    pub name: String,
    pub entries: Vec<Uid>,
}

impl Sequence {
    pub fn new_with_name(name: &str) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub(crate) fn swap_entry_left(&mut self, img_uid: u64) {
        let pos = self.entries.iter().position(|&uid| uid == img_uid).unwrap();
        self.entries.swap(pos - 1, pos);
    }
    pub(crate) fn swap_entry_right(&mut self, img_uid: u64) {
        let pos = self.entries.iter().position(|&uid| uid == img_uid).unwrap();
        self.entries.swap(pos + 1, pos);
    }
    pub(crate) fn remove_entry(&mut self, img_uid: u64) {
        let pos = self.entries.iter().position(|&uid| uid == img_uid).unwrap();
        self.entries.remove(pos);
    }

    pub(crate) fn iage_uids_wrapped_from(&self, img_uid: u64) -> Vec<Uid> {
        let pos = self.entries.iter().position(|&uid| uid == img_uid).unwrap();
        let mut uids = Vec::new();
        uids.extend_from_slice(&self.entries[pos..]);
        uids.extend_from_slice(&self.entries[..pos]);
        uids
    }
}
