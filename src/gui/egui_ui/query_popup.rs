use super::tag_autocomplete::AcState;

#[derive(Default)]
pub struct QueryPopup {
    pub on: bool,
    pub string: String,
    pub err_string: String,
    pub ac_state: AcState,
}
