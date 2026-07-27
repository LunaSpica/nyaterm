use super::*;

use crate::models::{AiActionEditorField, AiActionListKind};

impl NyaTermApp {
    pub(in crate::features) fn ai_rules_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let file_size_mb = (self.ai.settings.config.max_ai_file_size_bytes / (1024 * 1024)).max(1);
        let terminal_actions = self.ai_action_editor(
            palette,
            AiActionListKind::Terminal,
            self.tr("ai.terminalActions"),
            cx,
        );
        let file_actions = self.ai_action_editor(
            palette,
            AiActionListKind::File,
            self.tr("ai.fileActions"),
            cx,
        );

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(settings_form_section(
                palette,
                Some(self.tr("ai.rules")),
                None,
                settings_form_row(
                    palette,
                    format!("{} (MB)", self.tr("ai.maxAiFileSize")),
                    Some(SharedString::from(self.tr("ai.maxAiFileSizeDesc"))),
                    ai_rules_number_stepper(
                        palette,
                        file_size_mb,
                        cx.listener(|this, _, _, cx| {
                            this.adjust_ai_file_size_mb(-1, cx);
                        }),
                        cx.listener(|this, _, _, cx| {
                            this.adjust_ai_file_size_mb(1, cx);
                        }),
                    ),
                ),
            ))
            .child(terminal_actions)
            .child(file_actions)
    }

    fn ai_action_editor(
        &mut self,
        palette: crate::theme::ThemePalette,
        kind: AiActionListKind,
        title: &'static str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let actions = match kind {
            AiActionListKind::Terminal => self.ai.settings.config.terminal_ai_actions.clone(),
            AiActionListKind::File => self.ai.settings.config.file_ai_actions.clone(),
        };
        let add_label = self.tr("common.add");
        let delete_label = self.tr("common.delete");
        let name_placeholder = self.tr("ai.actionName");
        let prompt_placeholder = self.tr("ai.actionPrompt");

        settings_form_section(
            palette,
            Some(title),
            None,
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(div().flex().justify_end().child(small_button(
                    palette,
                    format!("ai-action-add-{:?}", kind),
                    add_label,
                    cx.listener(move |this, _, window, cx| {
                        this.add_ai_action(kind, window, cx);
                    }),
                )))
                .children(actions.into_iter().map(|action| {
                    let action_id = action.id.clone();
                    let action_id_toggle = action.id.clone();
                    let action_id_delete = action.id.clone();
                    let name_empty = action.name.is_empty();
                    let name_input_id =
                        Self::ai_action_text_input_id(kind, &action.id, AiActionEditorField::Name);
                    let prompt_input_id = Self::ai_action_text_input_id(
                        kind,
                        &action.id,
                        AiActionEditorField::Prompt,
                    );
                    let name_click = cx.listener({
                        let action_id = action_id.clone();
                        move |this, _, window, cx| {
                            this.focus_ai_action_field(
                                kind,
                                action_id.clone(),
                                AiActionEditorField::Name,
                                window,
                                cx,
                            );
                        }
                    });
                    let prompt_click = cx.listener({
                        let action_id = action_id.clone();
                        move |this, _, window, cx| {
                            this.focus_ai_action_field(
                                kind,
                                action_id.clone(),
                                AiActionEditorField::Prompt,
                                window,
                                cx,
                            );
                        }
                    });
                    let name_input = self.text_input_box(
                        name_input_id,
                        &action.name,
                        TextInputSetup::placeholder(name_placeholder),
                        cx,
                    );
                    let prompt_input = self.text_input_box(
                        prompt_input_id,
                        &action.prompt,
                        TextInputSetup::multi_line(prompt_placeholder),
                        cx,
                    );

                    div()
                        .id(SharedString::from(format!(
                            "ai-action-{}-{}",
                            match kind {
                                AiActionListKind::Terminal => "term",
                                AiActionListKind::File => "file",
                            },
                            action.id
                        )))
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .bg(rgb(palette.bg))
                        .p_4()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .track_focus(&self.ai.settings.action_focus)
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                            this.handle_ai_action_key_down(event, window, cx);
                        }))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_3()
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight(500.))
                                        .text_color(rgb(palette.text))
                                        .overflow_hidden()
                                        .child(if name_empty {
                                            name_placeholder.to_string()
                                        } else {
                                            action.name.clone()
                                        }),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(settings_switch(
                                            palette,
                                            format!("ai-action-enabled-{}", action.id),
                                            action.enabled,
                                            cx.listener(move |this, _, _, cx| {
                                                this.toggle_ai_action_enabled(
                                                    kind,
                                                    action_id_toggle.clone(),
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            format!("ai-action-delete-{}", action.id),
                                            delete_label,
                                            cx.listener(move |this, _, _, cx| {
                                                this.remove_ai_action(
                                                    kind,
                                                    action_id_delete.clone(),
                                                    cx,
                                                );
                                            }),
                                        )),
                                ),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!(
                                    "ai-action-name-click-{}",
                                    action.id
                                )))
                                .on_click(name_click)
                                .child(name_input),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!(
                                    "ai-action-prompt-click-{}",
                                    action.id
                                )))
                                .on_click(prompt_click)
                                .child(prompt_input),
                        )
                })),
        )
        .into_any_element()
    }
}

fn ai_rules_number_stepper(
    palette: ThemePalette,
    value: u64,
    on_minus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_plus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(small_button(palette, "ai-file-size-minus", "-", on_minus))
        .child(
            div()
                .min_w(px(56.))
                .text_center()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(11.))
                .text_color(rgb(palette.text))
                .child(value.to_string()),
        )
        .child(small_button(palette, "ai-file-size-plus", "+", on_plus))
}
