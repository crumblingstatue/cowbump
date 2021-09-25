use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Preferences {
    pub open_last_coll_at_start: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            open_last_coll_at_start: true,
        }
    }
}
