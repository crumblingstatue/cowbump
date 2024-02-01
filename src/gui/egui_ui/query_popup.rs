use {
    super::tag_autocomplete::AcState,
    egui_sfml::egui::{Align2, Context, Ui, Window},
};

#[derive(Default)]
pub struct QueryPopup {
    pub on: bool,
    pub string: String,
    pub err_string: String,
    pub ac_state: AcState,
}

/// Create an egui window in the corner for queries (filter/search)
fn query_window(title: &str) -> Window {
    Window::new(title)
        .anchor(Align2::LEFT_TOP, [32.0, 32.0])
        .title_bar(false)
        .auto_sized()
}

impl QueryPopup {
    pub fn do_frame(
        &mut self,
        title: &str,
        egui_ctx: &Context,
        mut inner_fn: impl FnMut(&mut Self, &mut Ui),
    ) {
        if self.on {
            query_window(title).show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(title);
                    inner_fn(self, ui);
                });
                if !self.err_string.is_empty() {
                    ui.label(&self.err_string);
                }
            });
        }
    }
}
