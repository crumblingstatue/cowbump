use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RecentlyUsedList<T> {
    items: Vec<T>,
    max_items: usize,
}

impl<T> Default for RecentlyUsedList<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            max_items: 7,
        }
    }
}

impl<T: PartialEq> RecentlyUsedList<T> {
    pub fn use_(&mut self, item: T) {
        let pos = self.items.iter().position(|it| it == &item);
        if let Some(pos) = pos {
            self.items.remove(pos);
        }
        self.items.push(item);
        if self.items.len() > self.max_items {
            self.items.remove(0);
        }
    }
}

impl<T> RecentlyUsedList<T> {
    pub fn most_recent(&self) -> Option<&T> {
        self.items.last()
    }
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter().rev()
    }
}

#[test]
fn test() {
    let mut ru = RecentlyUsedList::default();
    ru.use_(4);
    ru.use_(8);
    assert_eq!(ru.most_recent(), Some(&8));
    ru.use_(4);
    assert_eq!(ru.most_recent(), Some(&4));
    assert_eq!(ru.len(), 2);
    for i in 0..10 {
        ru.use_(i);
    }
    assert_eq!(ru.len(), 7);
    assert_eq!(ru.most_recent(), Some(&9));
    let items: Vec<_> = ru.iter().collect();
    assert_eq!(items[0], &9);
    assert_eq!(items[6], &3);
}
