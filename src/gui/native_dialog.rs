use rfd::{MessageDialog, MessageLevel};

pub fn error(title: &str, err: anyhow::Error) {
    MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title(title)
        .set_description(&format!("{:?}", err))
        .show();
}
