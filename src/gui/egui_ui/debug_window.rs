use {
    super::EguiState,
    egui_sf2g::egui::{self, Align, Color32, Context, Label, RichText, ScrollArea, Window},
};

pub struct DebugWindow {
    open: bool,
    auto_scroll: bool,
    max_entries: usize,
}

impl Default for DebugWindow {
    fn default() -> Self {
        Self {
            open: false,
            auto_scroll: true,
            max_entries: 1000,
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
            let log = &crate::gui::debug_log::LOG;
            ui.group(|ui| {
                let mut log = log.lock();
                let overflow = log.len() as isize - win.max_entries as isize;
                if overflow > 0 {
                    log.drain(0..overflow as usize);
                }
                ScrollArea::vertical()
                    .auto_shrink(false)
                    .max_height(ui.available_height() - 32.0)
                    .show(ui, |ui| {
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
                ui.label("Max entries");
                ui.add(egui::DragValue::new(&mut win.max_entries));
                if ui.button("Clear").clicked() {
                    log.lock().clear();
                }
            });
        });
}
