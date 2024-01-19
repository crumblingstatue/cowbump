use std::cell::RefCell;

thread_local! {
    pub static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub macro dlog($($arg:tt) *) {
    LOG.with(|log| {
        log.borrow_mut().push(format!($($arg)*))
    })
}
