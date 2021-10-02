use rfd::{MessageDialog, MessageLevel};

pub fn error(err: anyhow::Error) {
    MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title("Error")
        .set_description(&format!("{:?}", err))
        .show();
}
