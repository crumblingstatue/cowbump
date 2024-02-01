use {
    rfd::{MessageDialog, MessageLevel},
    std::fmt::Debug,
};

pub fn error<E: Debug>(title: &str, err: E) {
    MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title(title)
        .set_description(format!("{:?}", err))
        .show();
}
