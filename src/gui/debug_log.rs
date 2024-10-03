use std::cell::RefCell;

thread_local! {
    pub static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

#[macro_export]
macro_rules! dlog {
    ($fmt:literal $($arg:tt) *) => {
        $crate::gui::debug_log::LOG.with(|log| {
            log.borrow_mut().push(format!(concat!(file!(), ":", line!(), ": ", $fmt) $($arg)*))
        })
    }
}
