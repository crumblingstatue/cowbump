use {
    super::icons,
    crate::{dlog, tag},
    egui_flex::{item, Flex, FlexAlign, FlexAlignContent},
    egui_sfml::egui::{self, TextWrapMode},
};

#[derive(Clone)]
pub enum PromptAction {
    QuitNoSave,
    DeleteTags(Vec<tag::Id>),
    MergeTag { merge: tag::Id, into: tag::Id },
}

#[derive(Default)]
pub struct ModalDialog {
    payload: Option<ModalPayload>,
}

enum ModalPayload {
    Err(String),
    Success(String),
    About,
    Prompt {
        title: String,
        message: String,
        action: PromptAction,
    },
}

impl ModalDialog {
    pub fn err(&mut self, body: impl std::fmt::Display) {
        self.payload = Some(ModalPayload::Err(body.to_string()));
    }
    pub fn about(&mut self) {
        self.payload = Some(ModalPayload::About);
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
    pub fn show_payload(
        &mut self,
        ctx: &egui::Context,
        clipboard: &mut arboard::Clipboard,
    ) -> Option<PromptAction> {
        let mut action = None;
        if let Some(payload) = &self.payload {
            let (key_enter, key_esc) = ctx.input_mut(|inp| {
                (
                    inp.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                    inp.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                )
            });
            let mut close = false;
            show_modal_ui(ctx, |ui| match payload {
                ModalPayload::Err(s) => {
                    Flex::vertical()
                        .align_content(FlexAlignContent::Center)
                        .align_items(FlexAlign::Center)
                        .wrap(false)
                        .gap(egui::vec2(ui.style().spacing.item_spacing.x, 16.0))
                        .show(ui, |flex| {
                            flex.add(
                                item(),
                                egui::Label::new(
                                    egui::RichText::new([icons::WARN, " Error"].concat()).heading(),
                                )
                                .wrap_mode(TextWrapMode::Extend),
                            );
                            if s.lines().count() > 5 {
                                flex.add_simple(item().basis(200.0).grow(1.0), |ui| {
                                    ui.set_width(1000.0);
                                    egui::ScrollArea::vertical().show(ui, |ui| {
                                        ui.add_sized(
                                            ui.available_size(),
                                            egui::TextEdit::multiline(&mut s.as_str())
                                                .code_editor(),
                                        );
                                    });
                                });
                            } else {
                                flex.add(
                                    item(),
                                    egui::Label::new(s).wrap_mode(TextWrapMode::Extend),
                                );
                            }
                            flex.add_flex(
                                item().align_self(FlexAlign::End),
                                Flex::horizontal(),
                                |flex| {
                                    let ok = flex
                                        .add(
                                            item(),
                                            egui::Button::new([icons::CANCEL, " Close"].concat()),
                                        )
                                        .inner;
                                    let ccopy = flex
                                        .add(
                                            item(),
                                            egui::Button::new(
                                                [icons::COPY, " Copy to clipboard"].concat(),
                                            ),
                                        )
                                        .inner;
                                    if ok.clicked() || key_enter || key_esc {
                                        close = true;
                                    }
                                    if ccopy.clicked() {
                                        if let Err(e) = clipboard.set_text(s) {
                                            dlog!("Failed to set clipboard text: {e}");
                                        }
                                    }
                                },
                            );
                        });
                }
                ModalPayload::Success(s) => {
                    ui.vertical_centered(|ui| {
                        ui.label(s);
                        ui.add_space(16.0);
                        if ui.button("Close").clicked() || key_enter || key_esc {
                            close = true;
                        }
                    });
                }
                ModalPayload::About => {
                    ui.vertical_centered(|ui| {
                        ui.label(["Cowbump version ", crate::VERSION].concat());
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
                                egui::Label::new(egui::RichText::new(title).heading())
                                    .wrap_mode(TextWrapMode::Extend),
                            );
                            flex.add(
                                item(),
                                egui::Label::new(message).wrap_mode(TextWrapMode::Extend),
                            );
                            flex.add_flex(
                                item().align_self(FlexAlign::End),
                                Flex::horizontal(),
                                |flex| {
                                    let ok = flex
                                        .add(
                                            item(),
                                            egui::Button::new([icons::CHECK, " Ok"].concat()),
                                        )
                                        .inner;
                                    let cancel = flex
                                        .add(item(), egui::Button::new(icons::CANCEL_TEXT))
                                        .inner;
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
                egui::Rounding::ZERO,
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
