use egui::{Align, Color32, Label, ScrollArea, Window};

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

pub(super) fn do_frame(state: &mut crate::gui::State, egui_ctx: &egui::CtxRef) {
    let win = &mut state.egui_state.debug_window;
    if !win.open {
        return;
    }
    Window::new("Debug window")
        .open(&mut win.open)
        .show(egui_ctx, |ui| {
            ui.heading("Debug log");
            let mut debug_log = state.debug_log.borrow_mut();
            ui.group(|ui| {
                ScrollArea::vertical().max_height(500.).show(ui, |ui| {
                    if debug_log.is_empty() {
                        ui.label("<empty>");
                    }
                    for (i, entry) in debug_log.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add(Label::new(i.to_string()).text_color(Color32::DARK_GRAY));
                            ui.label(entry);
                        });
                    }
                    if win.auto_scroll {
                        ui.scroll_to_cursor(Align::BOTTOM)
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut win.auto_scroll, "Auto scroll");
                if ui.button("Clear").clicked() {
                    debug_log.clear();
                }
            });
        });
}
