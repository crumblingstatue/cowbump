use egui_sfml::egui;

pub trait UiExt {
    fn rtl(&mut self, add_contents: impl FnOnce(&mut egui::Ui)) -> egui::InnerResponse<()>;
}

impl UiExt for egui::Ui {
    fn rtl(&mut self, add_contents: impl FnOnce(&mut egui::Ui)) -> egui::InnerResponse<()> {
        self.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            add_contents,
        )
    }
}
