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

#[macro_export]
macro_rules! ddbg {
    ($val:expr $(,)?) => {
        match $val {
            tmp => {
                $crate::dlog!("{} = {:#?}", ::std::stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::ddbg!($val)),+,)
    };
}
