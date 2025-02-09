use {
    super::icons,
    crate::{dlog, tag},
    constcat::concat,
    egui_flex::{Flex, FlexAlign, FlexAlignContent, item},
    egui_sfml::egui::{self, TextWrapMode},
    std::backtrace::Backtrace,
};

#[derive(Clone)]
pub enum PromptAction {
    QuitNoSave,
    DeleteTags(Vec<tag::Id>),
    MergeTag { merge: tag::Id, into: tag::Id },
    PanicTest,
}

#[derive(Default)]
pub struct ModalDialog {
    payload: Option<ModalPayload>,
}

struct ErrPayload {
    message: String,
    backtrace: Backtrace,
    show_bt: bool,
}

enum ModalPayload {
    Err(ErrPayload),
    Success(String),
    About,
    Keybinds,
    Prompt {
        title: String,
        message: String,
        action: PromptAction,
    },
}

impl ModalDialog {
    pub fn err(&mut self, body: impl std::fmt::Display) {
        self.payload = Some(ModalPayload::Err(ErrPayload {
            message: body.to_string(),
            backtrace: Backtrace::force_capture(),
            show_bt: false,
        }));
    }
    pub fn about(&mut self) {
        self.payload = Some(ModalPayload::About);
    }
    pub fn keybinds(&mut self) {
        self.payload = Some(ModalPayload::Keybinds);
    }
    pub fn success(&mut self, msg: impl std::fmt::Display) {
        self.payload = Some(ModalPayload::Success(msg.to_string()));
    }
    pub fn prompt(
        &mut self,
        title: impl Into<String>,
        message: impl Into<String>,
        action: PromptAction,
    ) {
        self.payload = Some(ModalPayload::Prompt {
            title: title.into(),
            message: message.into(),
            action,
        });
    }
    pub(super) fn show_payload(
        &mut self,
        ctx: &egui::Context,
        clipboard: &mut arboard::Clipboard,
    ) -> Option<PromptAction> {
        let mut action = None;
        if let Some(payload) = &mut self.payload {
            let (key_enter, key_esc) = ctx.input_mut(|inp| {
                (
                    inp.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                    inp.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                )
            });
            let mut close = false;
            show_modal_ui(ctx, |ui| match payload {
                ModalPayload::Err(payload) => {
                    Flex::vertical()
                        .align_content(FlexAlignContent::Center)
                        .align_items(FlexAlign::Center)
                        .wrap(false)
                        .gap(egui::vec2(ui.style().spacing.item_spacing.x, 16.0))
                        .show(ui, |flex| {
                            flex.add(
                                item(),
                                egui::Label::new(
                                    egui::RichText::new(concat!(icons::WARN, " Error")).heading(),
                                )
                                .wrap_mode(TextWrapMode::Extend),
                            );
                            if payload.message.lines().count() > 5 || payload.show_bt {
                                flex.add_ui(item().basis(200.0).grow(1.0), |ui| {
                                    ui.set_width(1000.0);
                                    egui::ScrollArea::vertical().show(ui, |ui| {
                                        let bt_s;
                                        let mut s;
                                        ui.add_sized(
                                            ui.available_size(),
                                            egui::TextEdit::multiline(if payload.show_bt {
                                                bt_s = payload.backtrace.to_string();
                                                s = bt_s.as_str();
                                                &mut s
                                            } else {
                                                s = payload.message.as_str();
                                                &mut s
                                            })
                                            .code_editor(),
                                        );
                                    });
                                });
                            } else {
                                flex.add(
                                    item(),
                                    egui::Label::new(payload.message.as_str())
                                        .wrap_mode(TextWrapMode::Extend),
                                );
                            }
                            flex.add_flex(
                                item().align_self(FlexAlign::End),
                                Flex::horizontal(),
                                |flex| {
                                    flex.add(
                                        item(),
                                        egui::Checkbox::new(&mut payload.show_bt, "backtrace"),
                                    );
                                    let ok = flex.add(
                                        item(),
                                        egui::Button::new(concat!(icons::CANCEL, " Close")),
                                    );
                                    let ccopy = flex.add(
                                        item(),
                                        egui::Button::new(concat!(
                                            icons::COPY,
                                            " Copy to clipboard"
                                        )),
                                    );
                                    if ok.clicked() || key_enter || key_esc {
                                        close = true;
                                    }
                                    if ccopy.clicked() {
                                        let bt_s;
                                        let copy_payload = if payload.show_bt {
                                            bt_s = payload.backtrace.to_string();
                                            bt_s.as_str()
                                        } else {
                                            &payload.message
                                        };
                                        if let Err(e) = clipboard.set_text(copy_payload) {
                                            dlog!("Failed to set clipboard text: {e}");
                                        }
                                    }
                                },
                            );
                        });
                }
                ModalPayload::Success(s) => {
                    ui.vertical_centered(|ui| {
                        ui.set_max_width(1000.0);
                        ui.heading(concat!(icons::CHECK, " Success"));
                        ui.label(s.as_str());
                        ui.add_space(16.0);
                        if ui.button("Close").clicked() || key_enter || key_esc {
                            close = true;
                        }
                    });
                }
                ModalPayload::About => {
                    ui.vertical_centered(|ui| {
                        ui.label(concat!("Cowbump version ", crate::VERSION));
                        ui.add_space(16.0);
                        if ui.button("Close").clicked() || key_enter || key_esc {
                            close = true;
                        }
                    });
                }
                ModalPayload::Keybinds => {
                    ui.vertical(|ui| {
                        ui.set_width(1000.0);
                        let keybinds_text = include_str!("../../../KEYBINDS.md");
                        ui.label(keybinds_text);
                        ui.add_space(16.0);
                        if ui.button("Close").clicked() || key_enter || key_esc {
                            close = true;
                        }
                    });
                }
                ModalPayload::Prompt {
                    title,
                    message,
                    action: prompt_action,
                } => {
                    Flex::vertical()
                        .align_content(FlexAlignContent::Center)
                        .align_items(FlexAlign::Center)
                        .gap(egui::vec2(ui.style().spacing.item_spacing.x, 16.0))
                        .show(ui, |flex| {
                            flex.add(
                                item(),
                                egui::Label::new(egui::RichText::new(title.as_str()).heading())
                                    .wrap_mode(TextWrapMode::Extend),
                            );
                            flex.add(
                                item(),
                                egui::Label::new(message.as_str()).wrap_mode(TextWrapMode::Extend),
                            );
                            flex.add_flex(
                                item().align_self(FlexAlign::End),
                                Flex::horizontal(),
                                |flex| {
                                    let ok = flex.add(
                                        item(),
                                        egui::Button::new(concat!(icons::CHECK, " Ok")),
                                    );
                                    let cancel =
                                        flex.add(item(), egui::Button::new(icons::CANCEL_TEXT));
                                    if ok.clicked() || key_enter {
                                        action = Some(prompt_action.clone());
                                        close = true;
                                    }
                                    if cancel.clicked() || key_esc {
                                        close = true;
                                    }
                                },
                            );
                        });
                }
            });
            if close {
                self.payload = None;
            }
        }
        action
    }
}

fn show_modal_ui(ctx: &egui::Context, ui_fn: impl FnOnce(&mut egui::Ui)) {
    let re = egui::Area::new(egui::Id::new("modal_area"))
        .fixed_pos(egui::Pos2::ZERO)
        .show(ctx, |ui| {
            let screen_rect = ui.ctx().input(|inp| inp.screen_rect);
            ui.allocate_response(screen_rect.size(), egui::Sense::click());
            ui.painter().rect_filled(
                screen_rect,
                egui::CornerRadius::ZERO,
                egui::Color32::from_rgba_premultiplied(0, 0, 0, 200),
            );
        });
    ctx.move_to_top(re.response.layer_id);
    let re = egui::Window::new("egui_modal_popup")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui_fn(ui);
        });
    if let Some(re) = re {
        // This helps steal keyboard focus from underlying ui and app
        re.response.request_focus();
        ctx.move_to_top(re.response.layer_id);
    }
}
