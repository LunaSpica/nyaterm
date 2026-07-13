use super::*;

impl NyaTermApp {
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
                                rgb(palette.accent)
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
                    .text_color(rgb(palette.accent))
                    .child("Thinking…"),
            );
        }

        let has_display = !display.trim().is_empty();
        if has_display {
            if is_user {
                bubble = bubble.child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette.text))
                        .line_height(px(18.))
                        .child(display.clone()),
                );
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
                    .text_color(rgb(palette.accent))
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
