use parking_lot::Mutex;

pub static LOG: Mutex<Vec<String>> = const { Mutex::new(Vec::new()) };

#[macro_export]
macro_rules! dlog {
    ($($arg:tt) *) => {
        $crate::gui::debug_log::LOG
            .lock()
            .push(format!("{}:{}: {}", file!(), line!(), format_args!($($arg)*)))
    }
}
