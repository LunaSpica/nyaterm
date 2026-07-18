use super::*;

impl NyaTermApp {
    pub(in crate::features) fn close_ai_message_menu(&mut self, cx: &mut Context<Self>) {
        self.ai_message_menu = None;
        cx.notify();
    }

    pub(in crate::features) fn quote_ai_message_text(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let value = text.trim().to_string();
        if value.is_empty() {
            self.ai_status = "AI message is empty".to_string();
        } else {
            self.ai_quoted_text = Some(value);
            self.ai_status = "AI message quoted".to_string();
        }
        self.ai_message_menu = None;
        cx.notify();
    }

    pub(in crate::features) fn copy_ai_message_text(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let value = text.trim().to_string();
        if value.is_empty() {
            self.ai_status = "AI message is empty".to_string();
        } else {
            cx.write_to_clipboard(ClipboardItem::new_string(value));
            self.ai_status = "AI message copied".to_string();
        }
        self.ai_message_menu = None;
        cx.notify();
    }

    pub(in crate::features) fn clear_ai_quote(&mut self, cx: &mut Context<Self>) {
        self.ai_quoted_text = None;
        self.ai_status = "AI quote cleared".to_string();
        cx.notify();
    }

    pub(in crate::features) fn ai_message_context_menu_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let state = self.ai_message_menu.clone().unwrap_or(AiMessageMenuState {
            message_id: String::new(),
            role_label: "Message".to_string(),
            text: String::new(),
            x: px(24.),
            y: px(24.),
        });
        let quote_text = state.text.clone();
        let copy_text = state.text.clone();
        div()
            .id(SharedString::from("ai-message-context-menu-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_ai_message_menu(cx);
            }))
            .child(
                div()
                    .id(SharedString::from("ai-message-context-menu"))
                    .absolute()
                    .top(state.y)
                    .left(state.x)
                    .w(px(180.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .px_2()
                            .pb_1()
                            .text_size(px(10.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text_muted))
                            .child(state.role_label),
                    )
                    .child(ai_message_menu_button(
                        palette,
                        "ai-message-menu-quote",
                        "Quote",
                        cx.listener(move |this, _, _, cx| {
                            this.quote_ai_message_text(quote_text.clone(), cx);
                        }),
                    ))
                    .child(ai_message_menu_button(
                        palette,
                        "ai-message-menu-copy",
                        "Copy",
                        cx.listener(move |this, _, _, cx| {
                            this.copy_ai_message_text(copy_text.clone(), cx);
                        }),
                    )),
            )
    }

    pub(in crate::features) fn ai_message_bubble(
        &self,
        message: &AiMessage,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let is_user = matches!(message.role, AiMessageRole::User);
        let streaming = self
            .ai_streaming_assistant_id
            .as_deref()
            .is_some_and(|id| id == message.id);
        let role_label = if is_user { "User" } else { "AI" };
        let raw = if message.content.trim().is_empty() {
            String::new()
        } else {
            message.content.clone()
        };
        let (visible, think_reasoning) = extract_think_content(&raw);
        let mut reasoning = message
            .reasoning_content
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if reasoning.is_none() {
            reasoning = think_reasoning;
        }
        let display = if visible.trim().is_empty() {
            if streaming { String::new() } else { visible }
        } else {
            visible
        };
        let menu_text = if display.trim().is_empty() {
            raw.clone()
        } else {
            display.clone()
        };
        let menu_role = role_label.to_string();
        let menu_message_id = message.id.clone();

        // Tauri AssistantResponse/User: compact bubbles, softer borders.
        let mut bubble = div()
            .id(SharedString::from(format!("ai-msg-{}", message.id)))
            .rounded_md()
            .border_1()
            .border_color(if is_user {
                rgb(0x1f6feb)
            } else {
                rgb(palette.border)
            })
            .bg(if is_user {
                rgb(palette.hover)
            } else {
                rgb(palette.bg)
            })
            .px_2()
            .py_2()
            .flex()
            .flex_col()
            .gap_1()
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.ai_message_menu = Some(AiMessageMenuState {
                        message_id: menu_message_id.clone(),
                        role_label: menu_role.clone(),
                        text: menu_text.clone(),
                        x: event.position.x,
                        y: event.position.y,
                    });
                    this.ai_history_open = false;
                    this.ai_execution_menu_open = false;
                    this.ai_model_menu_open = false;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.text_muted))
                    .child(role_label),
            );

        if let Some(reasoning) = reasoning {
            bubble = bubble.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(if streaming {
                        rgb(0x1f6feb)
                    } else {
                        rgb(palette.border)
                    })
                    .bg(if streaming {
                        rgb(palette.hover)
                    } else {
                        rgb(palette.bg)
                    })
                    .px_2()
                    .py_2()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight(700.))
                            .text_color(if streaming {
                                rgb(palette.link)
                            } else {
                                rgb(palette.text_muted)
                            })
                            .child(if streaming { "Thinking…" } else { "Thought" }),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .line_height(px(16.))
                            .child(markdown_content_view(
                                palette,
                                &truncate_preview(&reasoning, 1200),
                            )),
                    ),
            );
        } else if streaming && display.trim().is_empty() {
            bubble = bubble.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x1f6feb))
                    .bg(rgb(palette.hover))
                    .px_2()
                    .py_2()
                    .text_size(px(11.))
                    .text_color(rgb(palette.link))
                    .child("Thinking…"),
            );
        }

        let has_display = !display.trim().is_empty();
        if has_display {
            if is_user {
                bubble = bubble.child(ai_user_pre_wrap_text(palette, &display));
            } else {
                let rendered = truncate_preview(&display, 8000);
                bubble = bubble.child(markdown_content_view(palette, &rendered));
            }
        }

        if !message.command_cards.is_empty() {
            // Tauri renders AICommandCardView inside assistant responses.
            for (card_index, card) in message.command_cards.iter().cloned().enumerate() {
                bubble = bubble.child(Self::ai_command_card_view_for_card(
                    palette,
                    format!("{}-{}", message.id, card_index),
                    card,
                    cx,
                ));
            }
        }
        if streaming && has_display {
            bubble = bubble.child(
                div()
                    .text_size(px(10.))
                    .text_color(rgb(palette.link))
                    .child("streaming…"),
            );
        }
        bubble
    }

    pub(in crate::features) fn ai_assistant_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Tauri AIAssistantPanel: toolbar + scroll transcript + bottom composer.
        // Shared stack already renders PanelHeader; body fills remaining height.
        self.ai_ask_panel(cx)
    }

    pub(in crate::features) fn right_ai_command_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.command_center_panel(cx))
            .child(self.ai_ask_panel(cx))
            .child(self.recording_panel(cx))
            .child(self.command_search_panel(cx))
            .child(self.quick_commands_panel(cx))
            .child(self.command_history_panel(cx))
    }
}

fn ai_user_pre_wrap_text(palette: crate::theme::ThemePalette, text: &str) -> gpui::AnyElement {
    let mut block = div()
        .min_w_0()
        .flex()
        .flex_col()
        .text_size(px(12.))
        .text_color(rgb(palette.text))
        .line_height(px(18.));
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let line_text = if line.is_empty() { " " } else { line }.to_string();
        block = block.child(div().min_w_0().line_height(px(18.)).child(line_text));
    }
    block.into_any_element()
}

fn ai_message_menu_button(
    palette: crate::theme::ThemePalette,
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id))
        .h(px(28.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .text_size(px(12.))
        .text_color(rgb(palette.text))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)))
        .on_click(on_click)
        .child(label)
}
