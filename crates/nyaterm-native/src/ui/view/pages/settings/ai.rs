use super::*;

impl NyaTermApp {
    fn ai_input(
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

    pub(in crate::ui::view) fn ai_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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
            .child(settings_form_section(
                Some("General"),
                Some("Assistant availability and safety preferences."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        "Enable AI",
                        Some(SharedString::from(self.ai_status.clone())),
                        settings_switch(
                            "ai-enabled",
                            self.ai_settings.enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_ai_enabled(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        "Redaction",
                        Some(SharedString::from(
                            "Strip secrets from prompts and observations before they leave the device.",
                        )),
                        settings_switch(
                            "ai-redaction-toggle",
                            self.ai_settings.redaction_enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_ai_redaction(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        "Allow save command",
                        Some(SharedString::from(
                            "Let the assistant persist generated commands into Quick Commands.",
                        )),
                        settings_switch(
                            "ai-save-command-toggle",
                            self.ai_settings.allow_save_command,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_ai_allow_save_command(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        "Record history",
                        Some(SharedString::from(
                            "Keep AI chat transcripts for later review.",
                        )),
                        settings_switch(
                            "ai-history-toggle",
                            self.ai_settings.record_history,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_ai_record_history(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
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
                            .text_color(rgb(0x8b949e))
                            .child("Live"),
                    )),
            ))
            .child(settings_form_section(
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
                            .child(settings_choice_chip(
                                "ai-provider-openai",
                                "OpenAI",
                                active_ai_profile_id == "openai",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("openai", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "ai-provider-anthropic",
                                "Anthropic",
                                active_ai_profile_id == "anthropic",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("anthropic", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "ai-provider-gemini",
                                "Gemini",
                                active_ai_profile_id == "gemini",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("gemini", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "ai-provider-deepseek",
                                "DeepSeek",
                                active_ai_profile_id == "deepseek",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("deepseek", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "ai-provider-ollama",
                                "Ollama",
                                active_ai_profile_id == "ollama",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("ollama", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
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
                    .child(settings_form_row(
                        "Provider actions",
                        None,
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                "ai-discover",
                                ai_discovery_label,
                                cx.listener(|this, _, _, cx| {
                                    this.discover_ai_models(cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-save",
                                "Save",
                                cx.listener(|this, _, _, cx| {
                                    this.save_ai_settings(cx);
                                }),
                            )),
                    )),
            ))
            .child(settings_form_section(
                Some("Agent defaults"),
                Some("How the assistant proposes and runs terminal commands."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        "Default mode",
                        Some(SharedString::from("Ask answers questions; Agent can run tools.")),
                        div()
                            .flex()
                            .gap_1()
                            .child(settings_choice_chip(
                                "ai-mode-ask",
                                "Ask",
                                self.ai_settings.default_mode == AiMode::Ask,
                                cx.listener(|this, _, _, cx| {
                                    this.set_ai_mode(AiMode::Ask, cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "ai-mode-agent",
                                "Agent",
                                self.ai_settings.default_mode == AiMode::Agent,
                                cx.listener(|this, _, _, cx| {
                                    this.set_ai_mode(AiMode::Agent, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        "Command execution",
                        Some(SharedString::from(
                            "Confirm each, smart risk gate, or auto-run low-risk commands.",
                        )),
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(
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
                            .child(settings_choice_chip(
                                "ai-command-smart",
                                "Smart",
                                self.ai_settings.agent_command_execution_mode
                                    == AgentCommandExecutionMode::Smart,
                                cx.listener(|this, _, _, cx| {
                                    this.set_ai_command_mode(AgentCommandExecutionMode::Smart, cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "ai-command-auto",
                                "Auto",
                                self.ai_settings.agent_command_execution_mode
                                    == AgentCommandExecutionMode::Auto,
                                cx.listener(|this, _, _, cx| {
                                    this.set_ai_command_mode(AgentCommandExecutionMode::Auto, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        "Background execution",
                        Some(SharedString::from(
                            "Allow the agent to continue command work while the UI stays interactive.",
                        )),
                        settings_switch(
                            "ai-agent-bg-exec",
                            self.ai_settings.agent_background_execution_enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_ai_background_execution(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                Some("Limits"),
                Some("Context window, timeouts, and agent step caps."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        "Context lines",
                        Some(SharedString::from(format!(
                            "{} lines of terminal context",
                            self.ai_settings.context_line_limit
                        ))),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(
                                "ai-context-minus",
                                "−50",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_context_line_limit(-50, cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-context-plus",
                                "+50",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_context_line_limit(50, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        "Request timeout",
                        Some(SharedString::from(format!(
                            "{} ms",
                            self.ai_settings.timeout_ms
                        ))),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(
                                "ai-timeout-minus",
                                "−1s",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_timeout_ms(-1_000, cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-timeout-plus",
                                "+1s",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_timeout_ms(1_000, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        "Max agent steps",
                        Some(SharedString::from(format!(
                            "{} steps",
                            self.ai_settings.max_agent_steps.unwrap_or(10)
                        ))),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(
                                "ai-agent-steps-minus",
                                "−1",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_agent_steps(-1, cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-agent-steps-plus",
                                "+1",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_agent_steps(1, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        "Terminal output lines",
                        Some(SharedString::from(format!(
                            "{} captured lines per observation",
                            self.ai_settings.terminal_output_lines
                        ))),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(
                                "ai-output-lines-minus",
                                "−1",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_terminal_output_lines(-1, cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-output-lines-plus",
                                "+1",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_terminal_output_lines(1, cx);
                                }),
                            )),
                    )),
            ))
    }

    pub(in crate::ui::view) fn ai_models_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Tauri AiModelsTab density: compact model rows + credential sections.
        let enabled_models = self
            .ai_settings
            .models
            .iter()
            .filter(|model| model.enabled)
            .count();
        let total_models = self.ai_settings.models.len();
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
        let models_summary = format!(
            "{enabled_models} enabled · {total_models} total · default {ai_default_model}"
        );
        let credentials_summary = format!("{enabled_credentials} enabled profiles");
        let active_ai_profile_id = self.ai_settings.active_profile_id.clone();
        let active_ai_api_key = ai_active_profile_api_key(&self.ai_settings);
        let ai_key_value = cloud_secret_display(&self.ai_secret_draft, &active_ai_api_key);
        let ai_discovery_label = if self.ai_discovery_pending {
            "Pending"
        } else {
            "Discover"
        };

        let model_rows = self
            .ai_settings
            .models
            .iter()
            .cloned()
            .take(12)
            .enumerate()
            .fold(
                div().flex().flex_col().gap_1(),
                |rows, (index, model)| {
                    let model_id = model.id.clone();
                    let default =
                        self.ai_settings.default_model_id.as_deref() == Some(model.id.as_str());
                    let provider = model
                        .provider_kind
                        .as_ref()
                        .map(ai_provider_kind_label)
                        .unwrap_or("unknown");
                    rows.child(
                        div()
                            .rounded_md()
                            .px_2()
                            .py_1()
                            .border_1()
                            .border_color(if default {
                                rgb(0x1f6feb)
                            } else {
                                rgb(0x21262d)
                            })
                            .bg(if default {
                                rgb(0x122033)
                            } else {
                                rgb(0x0d1117)
                            })
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .size(px(8.))
                                    .rounded_full()
                                    .flex_none()
                                    .bg(if model.enabled {
                                        rgb(0x3fb950)
                                    } else {
                                        rgb(0x484f58)
                                    }),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight(600.))
                                            .text_color(rgb(0xc9d1d9))
                                            .overflow_hidden()
                                            .child(truncate_preview(&model.name, 42)),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(rgb(0x6e7681))
                                            .child(format!(
                                                "{provider} · {}",
                                                ai_model_source_label(&model.source)
                                            )),
                                    ),
                            )
                            .when(default, |this| {
                                this.child(
                                    div()
                                        .text_size(px(10.))
                                        .font_weight(FontWeight(600.))
                                        .text_color(rgb(0x58a6ff))
                                        .child("default"),
                                )
                            })
                            .child(settings_switch(
                                format!("ai-model-toggle-{index}"),
                                model.enabled,
                                cx.listener({
                                    let model_id = model_id.clone();
                                    move |this, _, _, cx| {
                                        this.toggle_ai_model_enabled(model_id.clone(), cx);
                                    }
                                }),
                            ))
                            .child(small_button(
                                format!("ai-model-default-{index}"),
                                "Default",
                                cx.listener(move |this, _, _, cx| {
                                    this.set_ai_default_model(model_id.clone(), cx);
                                }),
                            )),
                    )
                },
            );

        let credential_rows = self
            .ai_settings
            .provider_credentials
            .iter()
            .cloned()
            .take(8)
            .fold(div().flex().flex_col().gap_1(), |rows, credential| {
                rows.child(
                    div()
                        .rounded_md()
                        .px_2()
                        .py_1()
                        .border_1()
                        .border_color(rgb(0x21262d))
                        .bg(rgb(0x0d1117))
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .size(px(8.))
                                .rounded_full()
                                .flex_none()
                                .bg(if credential.enabled {
                                    rgb(0x3fb950)
                                } else {
                                    rgb(0x484f58)
                                }),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight(600.))
                                        .text_color(rgb(0xc9d1d9))
                                        .overflow_hidden()
                                        .child(truncate_preview(&credential.name, 36)),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(rgb(0x6e7681))
                                        .child(format!(
                                            "{} · {}",
                                            ai_provider_kind_label(&credential.provider_kind),
                                            if credential
                                                .api_key
                                                .as_deref()
                                                .unwrap_or("")
                                                .is_empty()
                                            {
                                                "missing key"
                                            } else {
                                                "key set"
                                            }
                                        )),
                                ),
                        ),
                )
            });

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                Some("Models"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(settings_form_row(
                        "Catalog",
                        Some(SharedString::from(models_summary)),
                        small_button(
                            "ai-models-discover",
                            ai_discovery_label,
                            cx.listener(|this, _, _, cx| {
                                this.discover_ai_models(cx);
                            }),
                        ),
                    ))
                    .child(model_rows),
            ))
            .child(settings_form_section(
                Some("Provider credentials"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        "Profiles",
                        Some(SharedString::from(credentials_summary)),
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x8b949e))
                            .child("Stored"),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(
                                "ai-model-provider-openai",
                                "OpenAI",
                                active_ai_profile_id == "openai",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("openai", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "ai-model-provider-anthropic",
                                "Anthropic",
                                active_ai_profile_id == "anthropic",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("anthropic", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "ai-model-provider-gemini",
                                "Gemini",
                                active_ai_profile_id == "gemini",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("gemini", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "ai-model-provider-deepseek",
                                "DeepSeek",
                                active_ai_profile_id == "deepseek",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("deepseek", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "ai-model-provider-ollama",
                                "Ollama",
                                active_ai_profile_id == "ollama",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("ollama", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "ai-model-provider-xai",
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
                                "ai-model-tab-model",
                                "Model",
                                self.ai_model_draft.clone(),
                                AiInputField::Model,
                                cx,
                            ))
                            .child(self.ai_input(
                                "ai-model-tab-base-url",
                                "Base URL",
                                self.ai_base_url_draft.clone(),
                                AiInputField::BaseUrl,
                                cx,
                            ))
                            .child(self.ai_input(
                                "ai-model-tab-api-key",
                                "API Key",
                                ai_key_value,
                                AiInputField::ApiKey,
                                cx,
                            )),
                    )
                    .child(credential_rows)
                    .child(settings_form_row(
                        "Actions",
                        Some(SharedString::from(self.ai_status.clone())),
                        small_button(
                            "ai-model-tab-save",
                            "Save",
                            cx.listener(|this, _, _, cx| {
                                this.save_ai_settings(cx);
                            }),
                        ),
                    )),
            ))
    }

    pub(in crate::ui::view) fn ai_rules_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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
        let step_timeout_s = (self.ai_settings.agent_step_timeout_ms.unwrap_or(30_000) / 1000).max(1);
        let smart_risk = ai_risk_label(&self.ai_settings.agent_smart_auto_execute_max_risk);

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                Some("Rules"),
                Some("Limits and auto-execute risk for AI-assisted actions."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        "Max AI file size",
                        Some(SharedString::from(format!("{file_size_mb} MiB per attachment"))),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
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
                                    .text_color(rgb(0xc9d1d9))
                                    .child(format!("{file_size_mb}")),
                            )
                            .child(small_button(
                                "ai-file-size-plus",
                                "+1 MiB",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_file_size_mb(1, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        "Agent step timeout",
                        Some(SharedString::from(format!("{step_timeout_s}s per agent step"))),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
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
                                    .text_color(rgb(0xc9d1d9))
                                    .child(format!("{step_timeout_s}s")),
                            )
                            .child(small_button(
                                "ai-agent-step-timeout-plus",
                                "+1s",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_agent_step_timeout_ms(1_000, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        "Smart auto-execute risk",
                        Some(SharedString::from(format!("current: {smart_risk}"))),
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(
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
                        "Actions",
                        Some(SharedString::from(self.ai_status.clone())),
                        small_button(
                            "ai-rules-save",
                            "Save",
                            cx.listener(|this, _, _, cx| {
                                this.save_ai_settings(cx);
                            }),
                        ),
                    )),
            ))
            .child(ai_action_list(
                "Terminal Actions",
                format!(
                    "{terminal_enabled}/{} enabled",
                    self.ai_settings.terminal_ai_actions.len()
                ),
                self.ai_settings
                    .terminal_ai_actions
                    .iter()
                    .cloned()
                    .collect(),
            ))
            .child(ai_action_list(
                "File Actions",
                format!(
                    "{file_enabled}/{} enabled",
                    self.ai_settings.file_ai_actions.len()
                ),
                self.ai_settings.file_ai_actions.iter().cloned().collect(),
            ))
    }
}


fn ai_setting_hint(title: &'static str, detail: &'static str) -> impl IntoElement {
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x263142))
        .bg(rgb(0x0d1320))
        .p_3()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(800.))
                .text_color(rgb(0xe5edf7))
                .child(title),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(10.))
                .text_color(rgb(0x8f98aa))
                .line_height(px(14.))
                .child(detail),
        )
}

fn ai_boolean_state(label: &'static str, enabled: bool) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x111722))
        .p_3()
        .child(div().text_xs().text_color(rgb(0x98a3b8)).child(label))
        .child(
            div()
                .mt_1()
                .text_sm()
                .font_weight(FontWeight(700.))
                .text_color(if enabled {
                    rgb(0x86efac)
                } else {
                    rgb(0x98a3b8)
                })
                .child(if enabled { "enabled" } else { "disabled" }),
        )
}

fn ai_provider_kind_label(kind: &AiProviderKind) -> &'static str {
    match kind {
        AiProviderKind::Openai => "OpenAI",
        AiProviderKind::Anthropic => "Anthropic",
        AiProviderKind::Gemini => "Gemini",
        AiProviderKind::Deepseek => "DeepSeek",
        AiProviderKind::Groq => "Groq",
        AiProviderKind::Ollama => "Ollama",
        AiProviderKind::Xai => "xAI",
        AiProviderKind::Cohere => "Cohere",
        AiProviderKind::Mimo => "Mimo",
        AiProviderKind::Zai => "Z.ai",
        AiProviderKind::OpenaiCompatible => "OpenAI Compatible",
    }
}

fn ai_model_source_label(source: &AiModelSource) -> &'static str {
    match source {
        AiModelSource::RustGenai => "discovered",
        AiModelSource::Manual => "manual",
    }
}

fn ai_risk_label(risk: &RiskLevel) -> String {
    match risk {
        RiskLevel::Low => "low".to_string(),
        RiskLevel::Medium => "medium".to_string(),
        RiskLevel::High => "high".to_string(),
        RiskLevel::Critical => "critical".to_string(),
    }
}

fn ai_action_list(
    title: &'static str,
    summary: String,
    actions: Vec<AiCustomActionConfig>,
) -> impl IntoElement {
    let rows = actions.into_iter().take(8).fold(
        div().flex().flex_col().gap_1(),
        |rows, action| {
            rows.child(
                div()
                    .rounded_md()
                    .px_2()
                    .py_1()
                    .border_1()
                    .border_color(rgb(0x21262d))
                    .bg(rgb(0x0d1117))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .size(px(8.))
                            .rounded_full()
                            .flex_none()
                            .bg(if action.enabled {
                                rgb(0x3fb950)
                            } else {
                                rgb(0x484f58)
                            }),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(600.))
                                    .text_color(rgb(0xc9d1d9))
                                    .overflow_hidden()
                                    .child(truncate_preview(&action.name, 44)),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(0x6e7681))
                                    .overflow_hidden()
                                    .child(truncate_preview(&action.prompt, 96)),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight(600.))
                            .text_color(if action.enabled {
                                rgb(0x3fb950)
                            } else {
                                rgb(0x8b949e)
                            })
                            .child(if action.enabled { "on" } else { "off" }),
                    ),
            )
        },
    );

    settings_form_section(
        Some(title),
        None,
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(settings_form_row(
                "Catalog",
                Some(SharedString::from(summary)),
                div()
                    .text_size(px(11.))
                    .text_color(rgb(0x8b949e))
                    .child("Custom"),
            ))
            .child(rows),
    )
}
