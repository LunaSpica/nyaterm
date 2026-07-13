use super::*;

impl NyaTermApp {
    pub(in crate::features) fn ai_input(
        &mut self,
        id: &'static str,
        label: &'static str,
        value: String,
        field: AiInputField,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        transfer_input(
            id,
            label,
            if value.is_empty() {
                " ".to_string()
            } else {
                value
            },
            self.ai_focused_field == field,
            self.theme_palette(),
        )
        .track_focus(&self.ai_focus)
        .on_click(cx.listener(move |this, _, window, cx| {
            this.ai_focused_field = field;
            window.focus(&this.ai_focus);
            cx.notify();
        }))
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
            cx.stop_propagation();
            this.handle_ai_key_down(event, cx);
        }))
    }

    pub(in crate::features) fn ai_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Tauri AiGeneralTab: section/switch rows instead of metric grids.
        let active_ai_profile_id = self.ai_settings.active_profile_id.clone();
        let active_ai_api_key = ai_active_profile_api_key(&self.ai_settings);
        let ai_key_value = cloud_secret_display(&self.ai_secret_draft, &active_ai_api_key);
        let enabled_ai_models = self
            .ai_settings
            .models
            .iter()
            .filter(|model| model.enabled)
            .count();
        let enabled_credentials = self
            .ai_settings
            .provider_credentials
            .iter()
            .filter(|credential| credential.enabled)
            .count();
        let ai_default_model = self
            .ai_settings
            .default_model_id
            .as_deref()
            .map(compact_id)
            .unwrap_or_else(|| "none".to_string());
        let ai_discovery_label = if self.ai_discovery_pending {
            "Pending"
        } else {
            "Discover"
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(palette,
                Some("General"),
                Some("Assistant availability and safety preferences."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette,
                        "Enable AI",
                        Some(SharedString::from(self.ai_status.clone())),
                        settings_switch(palette,
                            "ai-enabled",
                            self.ai_settings.enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_ai_enabled(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette,
                        "Redaction",
                        Some(SharedString::from(
                            "Strip secrets from prompts and observations before they leave the device.",
                        )),
                        settings_switch(palette,
                            "ai-redaction-toggle",
                            self.ai_settings.redaction_enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_ai_redaction(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette,
                        "Allow save command",
                        Some(SharedString::from(
                            "Let the assistant persist generated commands into Quick Commands.",
                        )),
                        settings_switch(palette,
                            "ai-save-command-toggle",
                            self.ai_settings.allow_save_command,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_ai_allow_save_command(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette,
                        "Record history",
                        Some(SharedString::from(
                            "Keep AI chat transcripts for later review.",
                        )),
                        settings_switch(palette,
                            "ai-history-toggle",
                            self.ai_settings.record_history,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_ai_record_history(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Request User-Agent",
                        Some(SharedString::from(
                            "HTTP User-Agent for provider API requests (Tauri ai.request_user_agent).",
                        )),
                        self.ai_input(
                            "ai-request-user-agent",
                            "User-Agent",
                            if self.ai_settings.request_user_agent.is_empty() {
                                " ".to_string()
                            } else {
                                self.ai_settings.request_user_agent.clone()
                            },
                            AiInputField::RequestUserAgent,
                            cx,
                        ),
                    ))
                    .child(settings_form_row(palette,
                        "Usage snapshot",
                        Some(SharedString::from(format!(
                            "{} providers · {} models · default {} · {} sessions / {} messages / {} audits",
                            enabled_credentials,
                            enabled_ai_models,
                            ai_default_model,
                            self.ai_session_count,
                            self.ai_message_count,
                            self.ai_audit_count
                        ))),
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .child("Live"),
                    )),
            ))
            .child(settings_form_section(palette,
                Some("Active provider"),
                Some("Profile used for chat, discovery, and agent steps."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(palette,
                                "ai-provider-openai",
                                "OpenAI",
                                active_ai_profile_id == "openai",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("openai", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette,
                                "ai-provider-anthropic",
                                "Anthropic",
                                active_ai_profile_id == "anthropic",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("anthropic", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette,
                                "ai-provider-gemini",
                                "Gemini",
                                active_ai_profile_id == "gemini",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("gemini", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette,
                                "ai-provider-deepseek",
                                "DeepSeek",
                                active_ai_profile_id == "deepseek",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("deepseek", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette,
                                "ai-provider-ollama",
                                "Ollama",
                                active_ai_profile_id == "ollama",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("ollama", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette,
                                "ai-provider-xai",
                                "xAI",
                                active_ai_profile_id == "xai",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("xai", cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .grid()
                            .grid_cols(3)
                            .gap_2()
                            .child(self.ai_input(
                                "ai-model",
                                "Model",
                                self.ai_model_draft.clone(),
                                AiInputField::Model,
                                cx,
                            ))
                            .child(self.ai_input(
                                "ai-base-url",
                                "Base URL",
                                self.ai_base_url_draft.clone(),
                                AiInputField::BaseUrl,
                                cx,
                            ))
                            .child(self.ai_input(
                                "ai-api-key",
                                "API Key",
                                ai_key_value,
                                AiInputField::ApiKey,
                                cx,
                            )),
                    )
                    .child(settings_form_row(palette,
                        "Provider actions",
                        None,
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(palette,
                                "ai-discover",
                                ai_discovery_label,
                                cx.listener(|this, _, _, cx| {
                                    this.discover_ai_models(cx);
                                }),
                            ))
                            .child(small_button(palette,
                                "ai-save",
                                "Save",
                                cx.listener(|this, _, _, cx| {
                                    this.save_ai_settings(cx);
                                }),
                            )),
                    )),
            ))
            .child(settings_form_section(palette,
                Some("Agent defaults"),
                Some("How the assistant proposes and runs terminal commands."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette,
                        "Default mode",
                        Some(SharedString::from("Ask answers questions; Agent can run tools.")),
                        div()
                            .flex()
                            .gap_1()
                            .child(settings_choice_chip(palette,
                                "ai-mode-ask",
                                "Ask",
                                self.ai_settings.default_mode == AiMode::Ask,
                                cx.listener(|this, _, _, cx| {
                                    this.set_ai_mode(AiMode::Ask, cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette,
                                "ai-mode-agent",
                                "Agent",
                                self.ai_settings.default_mode == AiMode::Agent,
                                cx.listener(|this, _, _, cx| {
                                    this.set_ai_mode(AiMode::Agent, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette,
                        "Command execution",
                        Some(SharedString::from(
                            "Confirm each, smart risk gate, or auto-run low-risk commands.",
                        )),
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(palette,
                                "ai-command-confirm",
                                "Confirm",
                                self.ai_settings.agent_command_execution_mode
                                    == AgentCommandExecutionMode::ConfirmEach,
                                cx.listener(|this, _, _, cx| {
                                    this.set_ai_command_mode(
                                        AgentCommandExecutionMode::ConfirmEach,
                                        cx,
                                    );
                                }),
                            ))
                            .child(settings_choice_chip(palette,
                                "ai-command-smart",
                                "Smart",
                                self.ai_settings.agent_command_execution_mode
                                    == AgentCommandExecutionMode::Smart,
                                cx.listener(|this, _, _, cx| {
                                    this.set_ai_command_mode(AgentCommandExecutionMode::Smart, cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette,
                                "ai-command-auto",
                                "Auto",
                                self.ai_settings.agent_command_execution_mode
                                    == AgentCommandExecutionMode::Auto,
                                cx.listener(|this, _, _, cx| {
                                    this.set_ai_command_mode(AgentCommandExecutionMode::Auto, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette,
                        "Background execution",
                        Some(SharedString::from(
                            "Allow the agent to continue command work while the UI stays interactive.",
                        )),
                        settings_switch(palette,
                            "ai-agent-bg-exec",
                            self.ai_settings.agent_background_execution_enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_ai_background_execution(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(palette,
                Some("Limits"),
                Some("Context window, timeouts, and agent step caps."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette,
                        "Context lines",
                        Some(SharedString::from(format!(
                            "{} lines of terminal context",
                            self.ai_settings.context_line_limit
                        ))),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(palette,
                                "ai-context-minus",
                                "−50",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_context_line_limit(-50, cx);
                                }),
                            ))
                            .child(small_button(palette,
                                "ai-context-plus",
                                "+50",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_context_line_limit(50, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette,
                        "Request timeout",
                        Some(SharedString::from(format!(
                            "{} ms",
                            self.ai_settings.timeout_ms
                        ))),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(palette,
                                "ai-timeout-minus",
                                "−1s",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_timeout_ms(-1_000, cx);
                                }),
                            ))
                            .child(small_button(palette,
                                "ai-timeout-plus",
                                "+1s",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_timeout_ms(1_000, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette,
                        "Max agent steps",
                        Some(SharedString::from(format!(
                            "{} steps",
                            self.ai_settings.max_agent_steps.unwrap_or(10)
                        ))),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(palette,
                                "ai-agent-steps-minus",
                                "−1",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_agent_steps(-1, cx);
                                }),
                            ))
                            .child(small_button(palette,
                                "ai-agent-steps-plus",
                                "+1",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_agent_steps(1, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Agent step timeout",
                        Some(SharedString::from(format!(
                            "{} ms per agent step",
                            self.ai_settings.agent_step_timeout_ms.unwrap_or(30_000)
                        ))),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(
                                palette,
                                "ai-agent-step-timeout-minus",
                                "−1s",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_agent_step_timeout_ms(-1_000, cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "ai-agent-step-timeout-plus",
                                "+1s",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_agent_step_timeout_ms(1_000, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette,
                        "Terminal output lines",
                        Some(SharedString::from(format!(
                            "{} captured lines per observation",
                            self.ai_settings.terminal_output_lines
                        ))),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(palette,
                                "ai-output-lines-minus",
                                "−1",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_terminal_output_lines(-1, cx);
                                }),
                            ))
                            .child(small_button(palette,
                                "ai-output-lines-plus",
                                "+1",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_terminal_output_lines(1, cx);
                                }),
                            )),
                    )),
            ))
    }
}
