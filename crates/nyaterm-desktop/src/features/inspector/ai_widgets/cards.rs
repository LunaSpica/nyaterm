use gpui::{
    App, ClickEvent, ClipboardItem, Context, FontWeight, SharedString, Window, div, prelude::*, px,
    rgb,
};
use nyaterm_core::{AiCommandCard, truncate_preview};

use crate::features::NyaTermApp;
use crate::features::formatting::risk_label;
use crate::features::shell::gpui_code_font_family;
use crate::widgets::{small_button, status_pill};

impl NyaTermApp {
    pub(in crate::features) fn ai_command_card_list(
        &self,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        // Tauri AICommandCardView list under transcript.
        // Return AnyElement so listeners do not pin cx/self lifetimes across the panel tree.
        let mut rows = div().mt_2().flex().flex_col().gap_2();
        if self.ai.chat.command_cards.is_empty() {
            return rows.into_any_element();
        }
        let palette = self.theme_palette();
        for (index, card) in self
            .ai
            .chat
            .command_cards
            .iter()
            .cloned()
            .take(8)
            .enumerate()
        {
            rows = rows.child(Self::ai_command_card_view(palette, index, card, cx));
        }
        rows.into_any_element()
    }

    pub(in crate::features) fn ai_command_card_view(
        palette: crate::theme::ThemePalette,
        index: usize,
        card: AiCommandCard,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        // Keep list-index handlers for the live ai_command_cards strip.
        let risk = risk_label(card.risk_level.as_ref());
        let title = if card.title.trim().is_empty() {
            "Command".to_string()
        } else {
            card.title.clone()
        };
        let command = card.command.clone();
        let command_for_copy = command.clone();
        let explanation = card.explanation.clone();
        let expected = card.expected_effect.clone();
        let rollback = card.rollback.clone().unwrap_or_default();

        Self::ai_command_card_shell(
            palette,
            format!("idx-{index}"),
            risk,
            title,
            command,
            command_for_copy,
            explanation,
            expected,
            rollback,
            cx.listener(move |this, _, _, cx| {
                this.insert_ai_command_card(index, cx);
            }),
            cx.listener(move |this, _, _, cx| {
                this.save_ai_command_card(index, cx);
            }),
            cx.listener(move |this, _, _, cx| {
                this.run_ai_command_card(index, cx);
            }),
            cx,
        )
    }

    pub(in crate::features) fn ai_command_card_view_for_card(
        palette: crate::theme::ThemePalette,
        key: String,
        card: AiCommandCard,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        // Tauri AICommandCardView: title, mono command, explanation/effect/rollback, actions.
        let card_id = card.id.clone();
        let risk = risk_label(card.risk_level.as_ref());
        let title = if card.title.trim().is_empty() {
            "Command".to_string()
        } else {
            card.title.clone()
        };
        let command = card.command.clone();
        let command_for_copy = command.clone();
        let explanation = card.explanation.clone();
        let expected = card.expected_effect.clone();
        let rollback = card.rollback.clone().unwrap_or_default();
        let insert_id = card_id.clone();
        let save_id = card_id.clone();
        let run_id = card_id.clone();

        Self::ai_command_card_shell(
            palette,
            key,
            risk,
            title,
            command,
            command_for_copy,
            explanation,
            expected,
            rollback,
            cx.listener(move |this, _, _, cx| {
                this.insert_ai_command_card_by_id(insert_id.clone(), cx);
            }),
            cx.listener(move |this, _, _, cx| {
                this.save_ai_command_card_by_id(save_id.clone(), cx);
            }),
            cx.listener(move |this, _, _, cx| {
                this.run_ai_command_card_by_id(run_id.clone(), cx);
            }),
            cx,
        )
    }

    pub(in crate::features) fn ai_command_card_shell(
        palette: crate::theme::ThemePalette,
        key: String,
        risk: &'static str,
        title: String,
        command: String,
        command_for_copy: String,
        explanation: String,
        expected: String,
        rollback: String,
        on_insert: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        on_run: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        div()
            .id(SharedString::from(format!("ai-command-card-{key}")))
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.bg))
            .p_2()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(12.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text))
                            .overflow_hidden()
                            .child(truncate_preview(&title, 48)),
                    )
                    .child(status_pill(risk, rgb(palette.warning), rgb(palette.hover))),
            )
            .child(
                div()
                    .id(SharedString::from(format!("ai-command-body-{key}")))
                    .max_h(px(128.))
                    .overflow_hidden()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .px_2()
                    .py_1()
                    .font_family(gpui_code_font_family())
                    .text_size(px(11.))
                    .text_color(rgb(palette.text))
                    .line_height(px(16.))
                    .child(truncate_preview(&command, 1600)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .line_height(px(16.))
                            .child(truncate_preview(&explanation, 320)),
                    )
                    .when(!expected.trim().is_empty(), |this| {
                        this.child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette.text_dimmed))
                                .line_height(px(16.))
                                .child(truncate_preview(&expected, 220)),
                        )
                    })
                    .when(!rollback.trim().is_empty(), |this| {
                        this.child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette.text_dimmed))
                                .line_height(px(16.))
                                .child(format!("Rollback: {}", truncate_preview(&rollback, 160))),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_1()
                    .child(small_button(
                        palette,
                        format!("ai-command-insert-{key}"),
                        "Insert",
                        on_insert,
                    ))
                    .child(small_button(
                        palette,
                        format!("ai-command-copy-{key}"),
                        "Copy",
                        cx.listener(move |this, _, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                command_for_copy.clone(),
                            ));
                            this.ai.panel.status = "command copied".to_string();
                            cx.notify();
                        }),
                    ))
                    .child(small_button(
                        palette,
                        format!("ai-command-save-{key}"),
                        "Save",
                        on_save,
                    ))
                    .child(small_button(
                        palette,
                        format!("ai-command-run-{key}"),
                        "Run",
                        on_run,
                    )),
            )
            .into_any_element()
    }
}
