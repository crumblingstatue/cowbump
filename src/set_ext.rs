use std::{
    collections::HashSet,
    hash::{BuildHasher, Hash},
};

pub trait SetExt<T> {
    fn toggle(&mut self, item: T);
    fn toggle_by(&mut self, uid: T, on: bool);
}

impl<T: Hash + Eq, S: BuildHasher> SetExt<T> for HashSet<T, S> {
    fn toggle(&mut self, item: T) {
        let on = !self.contains(&item);
        self.toggle_by(item, on);
    }
    fn toggle_by(&mut self, item: T, on: bool) {
        if on {
            self.insert(item);
        } else {
            self.remove(&item);
        }
    }
}
