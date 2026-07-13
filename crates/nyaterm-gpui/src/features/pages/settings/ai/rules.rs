use super::*;

impl NyaTermApp {
    pub(in crate::features) fn ai_rules_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Tauri AiRulesTab: max file size + terminal/file action lists.
        let terminal_enabled = self
            .ai_settings
            .terminal_ai_actions
            .iter()
            .filter(|action| action.enabled)
            .count();
        let file_enabled = self
            .ai_settings
            .file_ai_actions
            .iter()
            .filter(|action| action.enabled)
            .count();
        let file_size_mb = (self.ai_settings.max_ai_file_size_bytes / (1024 * 1024)).max(1);
        let step_timeout_s =
            (self.ai_settings.agent_step_timeout_ms.unwrap_or(30_000) / 1000).max(1);
        let smart_risk = ai_risk_label(&self.ai_settings.agent_smart_auto_execute_max_risk);

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                palette,
                Some("Rules"),
                Some("Limits and auto-execute risk for AI-assisted actions."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "Max AI file size",
                        Some(SharedString::from(format!(
                            "{file_size_mb} MiB per attachment"
                        ))),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                palette,
                                "ai-file-size-minus",
                                "-1 MiB",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_file_size_mb(-1, cx);
                                }),
                            ))
                            .child(
                                div()
                                    .min_w(px(42.))
                                    .text_center()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(palette.text))
                                    .child(format!("{file_size_mb}")),
                            )
                            .child(small_button(
                                palette,
                                "ai-file-size-plus",
                                "+1 MiB",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_file_size_mb(1, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Agent step timeout",
                        Some(SharedString::from(format!(
                            "{step_timeout_s}s per agent step"
                        ))),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                palette,
                                "ai-agent-step-timeout-minus",
                                "-1s",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_agent_step_timeout_ms(-1_000, cx);
                                }),
                            ))
                            .child(
                                div()
                                    .min_w(px(42.))
                                    .text_center()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(palette.text))
                                    .child(format!("{step_timeout_s}s")),
                            )
                            .child(small_button(
                                palette,
                                "ai-agent-step-timeout-plus",
                                "+1s",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_agent_step_timeout_ms(1_000, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Smart auto-execute risk",
                        Some(SharedString::from(format!("current: {smart_risk}"))),
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(
                                palette,
                                "ai-risk-low",
                                "Low",
                                matches!(
                                    self.ai_settings.agent_smart_auto_execute_max_risk,
                                    RiskLevel::Low
                                ),
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_smart_auto_execute_max_risk(RiskLevel::Low, cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "ai-risk-medium",
                                "Medium",
                                matches!(
                                    self.ai_settings.agent_smart_auto_execute_max_risk,
                                    RiskLevel::Medium
                                ),
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_smart_auto_execute_max_risk(
                                        RiskLevel::Medium,
                                        cx,
                                    );
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "ai-risk-high",
                                "High",
                                matches!(
                                    self.ai_settings.agent_smart_auto_execute_max_risk,
                                    RiskLevel::High
                                ),
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_smart_auto_execute_max_risk(RiskLevel::High, cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "ai-risk-critical",
                                "Critical",
                                matches!(
                                    self.ai_settings.agent_smart_auto_execute_max_risk,
                                    RiskLevel::Critical
                                ),
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_smart_auto_execute_max_risk(
                                        RiskLevel::Critical,
                                        cx,
                                    );
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Actions",
                        Some(SharedString::from(self.ai_status.clone())),
                        small_button(
                            palette,
                            "ai-rules-save",
                            "Save",
                            cx.listener(|this, _, _, cx| {
                                this.save_ai_settings(cx);
                            }),
                        ),
                    )),
            ))
            .child(self.ai_action_editor(
                palette,
                AiActionListKind::Terminal,
                "Terminal Actions",
                format!(
                    "{terminal_enabled}/{} enabled",
                    self.ai_settings.terminal_ai_actions.len()
                ),
                cx,
            ))
            .child(self.ai_action_editor(
                palette,
                AiActionListKind::File,
                "File Actions",
                format!(
                    "{file_enabled}/{} enabled",
                    self.ai_settings.file_ai_actions.len()
                ),
                cx,
            ))
    }

    pub(in crate::features) fn ai_action_editor(
        &mut self,
        palette: crate::theme::ThemePalette,
        kind: AiActionListKind,
        title: &'static str,
        summary: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let actions = match kind {
            AiActionListKind::Terminal => self.ai_settings.terminal_ai_actions.clone(),
            AiActionListKind::File => self.ai_settings.file_ai_actions.clone(),
        };
        let expanded = self.ai_action_expanded.clone();
        let edit = self.ai_action_edit.clone();

        settings_form_section(
            palette,
            Some(title),
            None,
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(settings_form_row(
                    palette,
                    "Catalog",
                    Some(SharedString::from(summary)),
                    small_button(
                        palette,
                        format!("ai-action-add-{:?}", kind),
                        "Add",
                        cx.listener(move |this, _, window, cx| {
                            this.add_ai_action(kind, window, cx);
                        }),
                    ),
                ))
                .when(actions.is_empty(), |this| {
                    this.child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.input))
                            .px_4()
                            .py_5()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(palette.text_dimmed))
                                    .child("No custom actions — Add one."),
                            ),
                    )
                })
                .children(actions.into_iter().map(|action| {
                    let is_open = expanded
                        .as_ref()
                        .is_some_and(|(k, id)| *k == kind && id == &action.id);
                    let name_active = edit.as_ref().is_some_and(|(k, id, field)| {
                        *k == kind && id == &action.id && *field == AiActionEditorField::Name
                    });
                    let prompt_active = edit.as_ref().is_some_and(|(k, id, field)| {
                        *k == kind && id == &action.id && *field == AiActionEditorField::Prompt
                    });
                    let action_id = action.id.clone();
                    let action_id_toggle = action.id.clone();
                    let action_id_delete = action.id.clone();
                    let name_value = if action.name.is_empty() {
                        " ".to_string()
                    } else {
                        action.name.clone()
                    };
                    let prompt_value = if action.prompt.is_empty() {
                        " ".to_string()
                    } else {
                        action.prompt.clone()
                    };
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
                        .bg(rgb(palette.input))
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .id(SharedString::from(format!(
                                    "ai-action-header-{}-{}",
                                    match kind {
                                        AiActionListKind::Terminal => "term",
                                        AiActionListKind::File => "file",
                                    },
                                    action.id
                                )))
                                .px_3()
                                .py_2()
                                .flex()
                                .items_center()
                                .gap_2()
                                .cursor_pointer()
                                .hover(|this| this.bg(rgb(palette.hover)))
                                .on_click(cx.listener({
                                    let action_id = action_id.clone();
                                    move |this, _, _, cx| {
                                        this.expand_ai_action(kind, action_id.clone(), cx);
                                    }
                                }))
                                .child(div().size(px(8.)).rounded_full().flex_none().bg(
                                    if action.enabled {
                                        rgb(palette.success)
                                    } else {
                                        rgb(palette.border)
                                    },
                                ))
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight(600.))
                                        .text_color(rgb(palette.text))
                                        .overflow_hidden()
                                        .child(if action.name.trim().is_empty() {
                                            "Untitled action".to_string()
                                        } else {
                                            action.name.clone()
                                        }),
                                )
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
                                    "Delete",
                                    cx.listener(move |this, _, _, cx| {
                                        this.remove_ai_action(kind, action_id_delete.clone(), cx);
                                    }),
                                ))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .child(if is_open { "▾" } else { "▸" }),
                                ),
                        )
                        .when(is_open, |this| {
                            this.child(
                                div()
                                    .border_t_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.bg))
                                    .px_3()
                                    .py_3()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .track_focus(&self.ai_action_focus)
                                    .on_key_down(cx.listener(
                                        |this, event: &KeyDownEvent, _, cx| {
                                            this.handle_ai_action_key_down(event, cx);
                                        },
                                    ))
                                    .child(
                                        div()
                                            .id(SharedString::from(format!(
                                                "ai-action-name-{}",
                                                action.id
                                            )))
                                            .h(px(28.))
                                            .px_2()
                                            .rounded_md()
                                            .border_1()
                                            .border_color(if name_active {
                                                rgb(palette.accent)
                                            } else {
                                                rgb(palette.border)
                                            })
                                            .bg(rgb(palette.input))
                                            .flex()
                                            .items_center()
                                            .text_size(px(12.))
                                            .text_color(rgb(palette.text))
                                            .cursor_pointer()
                                            .child(name_value)
                                            .on_click(cx.listener({
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
                                            })),
                                    )
                                    .child(
                                        div()
                                            .id(SharedString::from(format!(
                                                "ai-action-prompt-{}",
                                                action.id
                                            )))
                                            .min_h(px(72.))
                                            .px_2()
                                            .py_2()
                                            .rounded_md()
                                            .border_1()
                                            .border_color(if prompt_active {
                                                rgb(palette.accent)
                                            } else {
                                                rgb(palette.border)
                                            })
                                            .bg(rgb(palette.input))
                                            .font_family("JetBrains Mono")
                                            .text_size(px(11.))
                                            .text_color(rgb(palette.text))
                                            .line_height(px(16.))
                                            .cursor_pointer()
                                            .child(prompt_value)
                                            .on_click(cx.listener({
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
                                            })),
                                    ),
                            )
                        })
                })),
        )
    }

}
