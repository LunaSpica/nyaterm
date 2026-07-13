use super::*;

impl NyaTermApp {
    pub(in crate::features) fn ai_ask_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let agent_mode = self.ai_settings.default_mode == AiMode::Agent;
        let file_action_ready = self
            .ai_prepared_request
            .as_ref()
            .is_some_and(|request| request.action == AiAction::CustomFileAction);
        let ai_running = self.ai_chat_pending || self.ai_agent_loop.is_some();
        let _action_label = if ai_running {
            "Cancel"
        } else if file_action_ready {
            "Run"
        } else if agent_mode {
            "Agent"
        } else {
            "Ask"
        };
        let command_rows = self.ai_command_card_list(cx);
        let mut agent_step_rows = div();
        if agent_mode || !self.ai_agent_steps.is_empty() {
            agent_step_rows = agent_step_rows
                .mt_2()
                .border_t_1()
                .border_color(rgb(palette.border))
                .pt_2()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(10.))
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(palette.text_muted))
                        .child("Agent Steps"),
                );
            if self.ai_agent_steps.is_empty() {
                agent_step_rows = agent_step_rows.child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.text_dimmed))
                        .child("No Agent steps yet."),
                );
            } else {
                for step in self.ai_agent_steps.iter().cloned().rev().take(16).rev() {
                    agent_step_rows = agent_step_rows.child(self.ai_agent_step_card(step, cx));
                }
            }
        }
        let model_label = self
            .ai_settings
            .default_model_id
            .clone()
            .unwrap_or_else(|| "default model".to_string());
        let mode_label = if agent_mode { "Agent" } else { "Ask" };
        let enabled = self.ai_settings.enabled;

        // Tauri AIAssistantPanel: PanelHeader(title+model meta + history/settings/new) already
        // provided by side stack; body keeps optional in-panel action strip when not stacked header.
        // Here we only add a compact action strip under shared header for history toggle + shortcuts.
        let model_meta = if model_label.trim().is_empty() {
            "not configured".to_string()
        } else {
            model_label.clone()
        };
        let _ = (mode_label, model_meta);
        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(palette.surface))
            .relative()
            .child(
                div()
                    // Secondary action strip under shared PanelHeader (Tauri keeps actions in header).
                    .h(px(30.))
                    .flex_none()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.section_header))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .overflow_hidden()
                            .child(truncate_preview(
                                &if enabled {
                                    model_label.clone()
                                } else {
                                    "AI disabled".to_string()
                                },
                                36,
                            )),
                    )
                    .child(ai_svg_icon_button(
                        palette,
                        "ai-execution-mode-toggle",
                        match self.ai_settings.agent_command_execution_mode {
                            AgentCommandExecutionMode::Auto => "icons/ai/exec-auto.svg",
                            AgentCommandExecutionMode::Smart => "icons/ai/exec-smart.svg",
                            AgentCommandExecutionMode::ConfirmEach => "icons/ai/exec-confirm.svg",
                        },
                        cx.listener(|this, _, _, cx| {
                            this.ai_history_open = false;
                            this.ai_execution_menu_open = !this.ai_execution_menu_open;
                            cx.notify();
                        }),
                    ))
                    .child(ai_svg_icon_button(
                        palette,
                        "ai-history-toggle",
                        "icons/ai/history.svg",
                        cx.listener(|this, _, window, cx| {
                            this.ai_execution_menu_open = false;
                            this.ai_history_open = !this.ai_history_open;
                            if this.ai_history_open {
                                this.refresh_ai_session_list(cx);
                                window.focus(&this.ai_history_search_focus);
                            } else {
                                this.ai_history_query.clear();
                            }
                            cx.notify();
                        }),
                    ))
                    .child(ai_svg_icon_button(
                        palette,
                        "ai-open-settings",
                        "icons/ai/settings.svg",
                        cx.listener(|this, _, _, cx| {
                            this.settings_active_tab = SettingsTab::AiGeneral;
                            this.open_page(NavItem::Settings, cx);
                        }),
                    ))
                    .child(ai_svg_icon_button(
                        palette,
                        "ai-new-chat",
                        "icons/ai/new.svg",
                        cx.listener(|this, _, _, cx| {
                            this.ai_prompt_draft.clear();
                            this.ai_response_preview =
                                if this.ai_settings.default_mode == AiMode::Agent {
                                    "Agent mode ready".to_string()
                                } else {
                                    "Ask mode ready".to_string()
                                };
                            this.ai_command_cards.clear();
                            this.ai_agent_steps.clear();
                            this.ai_agent_thought_expanded.clear();
                            this.ai_agent_output_expanded.clear();
                            this.ai_chat_messages.clear();
                            this.ai_streaming_assistant_id = None;
                            this.ai_prepared_request = None;
                            this.ai_chat_session_id = format!("ai-session-{}", uuid());
                            this.ai_status = "new AI chat".to_string();
                            cx.notify();
                        }),
                    )),
            )
            .when(self.ai_history_open, |this| {
                this.child(self.ai_history_popover(cx))
            })
            .when(self.ai_execution_menu_open, |this| {
                this.child(self.ai_execution_mode_menu(cx))
            })
            .child(
                div()
                    .id(SharedString::from("ai-transcript-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_scroll()
                    .scrollbar_width(px(6.))
                    .px_3()
                    .py_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(self.ai_transcript_body(
                        mode_label,
                        enabled,
                        agent_step_rows,
                        command_rows,
                        cx,
                    )),
            )
            .child(
                div()
                    .flex_none()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.section_header))
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        transfer_input(
                            "ai-ask-prompt",
                            if agent_mode {
                                "Describe a task for the agent…"
                            } else {
                                "Ask about the terminal or generate a command…"
                            },
                            self.ai_prompt_draft.clone(),
                            true,
                            self.theme_palette(),
                        )
                        .h(px(64.))
                        .track_focus(&self.ai_chat_focus)
                        .on_click(cx.listener(|this, _, window, cx| {
                            window.focus(&this.ai_chat_focus);
                            cx.notify();
                        }))
                        .on_key_down(cx.listener(
                            |this, event: &KeyDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.handle_ai_prompt_key_down(event, cx);
                            },
                        )),
                    )
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
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .h(px(28.))
                                            .flex()
                                            .items_center()
                                            .rounded_md()
                                            .border_1()
                                            .border_color(rgb(palette.border))
                                            .bg(rgb(palette.input))
                                            .p(px(1.))
                                            .gap_0()
                                            .child(mode_button(
                                                "ai-mode-ask",
                                                "Ask",
                                                !agent_mode,
                                                self.theme_palette(),
                                                cx.listener(|this, _, _, cx| {
                                                    this.set_ai_mode(AiMode::Ask, cx);
                                                }),
                                            ))
                                            .child(mode_button(
                                                "ai-mode-agent",
                                                "Agent",
                                                agent_mode,
                                                self.theme_palette(),
                                                cx.listener(|this, _, _, cx| {
                                                    this.set_ai_mode(AiMode::Agent, cx);
                                                }),
                                            )),
                                    )
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .text_size(px(11.))
                                            .text_color(rgb(palette.text_muted))
                                            .overflow_hidden()
                                            .child(truncate_preview(&model_label, 24)),
                                    ),
                            )
                            .child(ai_svg_icon_button(
                                palette,
                                "ai-ask-run",
                                if ai_running {
                                    "icons/ai/stop.svg"
                                } else {
                                    "icons/ai/send.svg"
                                },
                                cx.listener(|this, _, _, cx| {
                                    if this.ai_chat_pending || this.ai_agent_loop.is_some() {
                                        this.cancel_ai_chat(cx);
                                    } else {
                                        this.start_ai_ask(cx);
                                    }
                                }),
                            )),
                    )
                    .when(file_action_ready, |this| {
                        this.child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(palette.warning))
                                .child("File action ready — press send to run"),
                        )
                    }),
            )
    }


}
