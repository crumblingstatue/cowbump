pub fn insert_char_at(old: &str, ch: char, at: usize) -> String {
    // Unicode strings are hard. Let's just do the slow but lazy and somewhat working approach.
    let mut old_chars = old.chars();
    let mut new_string = String::new();
    for _ in 0..at {
        let ch = old_chars.next().unwrap();
        new_string.push(ch);
    }
    new_string.push(ch);
    for old_ch in old_chars {
        new_string.push(old_ch);
    }
    new_string
}

pub fn remove_char_at(old: &str, at: usize) -> String {
    let mut old_chars = old.chars();
    let mut new_string = String::new();
    for _ in 0..at {
        let ch = old_chars.next().unwrap();
        new_string.push(ch);
    }
    // Skip the char we want to delete
    let _ = old_chars.next();
    for old_ch in old_chars {
        new_string.push(old_ch);
    }
    new_string
}
