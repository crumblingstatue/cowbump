use {
    super::EguiState,
    egui_sfml::egui::{Align, Color32, Context, Label, RichText, ScrollArea, Window},
};

pub struct DebugWindow {
    open: bool,
    auto_scroll: bool,
}

impl Default for DebugWindow {
    fn default() -> Self {
        Self {
            open: false,
            auto_scroll: true,
        }
    }
}

impl DebugWindow {
    pub fn toggle(&mut self) {
        self.open ^= true;
    }
}

pub(super) fn do_frame(egui_state: &mut EguiState, egui_ctx: &Context) {
    let win = &mut egui_state.debug_window;
    if !win.open {
        return;
    }
    Window::new("Debug window")
        .open(&mut win.open)
        .show(egui_ctx, |ui| {
            ui.heading("Debug log");
            crate::gui::debug_log::LOG.with(|log| {
                ui.group(|ui| {
                    let log = log.borrow();
                    ScrollArea::vertical().max_height(500.).show(ui, |ui| {
                        if log.is_empty() {
                            ui.label("<empty>");
                        }
                        for (i, entry) in log.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.add(Label::new(
                                    RichText::new(i.to_string()).color(Color32::DARK_GRAY),
                                ));
                                ui.label(entry);
                            });
                        }
                        if win.auto_scroll {
                            ui.scroll_to_cursor(Some(Align::BOTTOM));
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut win.auto_scroll, "Auto scroll");
                    if ui.button("Clear").clicked() {
                        log.borrow_mut().clear();
                    }
                });
            });
        });
}
