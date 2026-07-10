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
        let ai_state_label = if self.ai_settings.enabled {
            "enabled"
        } else {
            "disabled"
        };

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x151923))
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(div().text_sm().font_weight(FontWeight(700.)).child("AI"))
                    .child(status_pill(
                        ai_state_label,
                        if self.ai_settings.enabled {
                            rgb(0x6ee7b7)
                        } else {
                            rgb(0x98a3b8)
                        },
                        if self.ai_settings.enabled {
                            rgb(0x12342a)
                        } else {
                            rgb(0x202633)
                        },
                    ))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
                            .child(self.ai_status.clone()),
                    ),
            )
            .child(
                div()
                    .mt_3()
                    .grid()
                    .grid_cols(7)
                    .gap_3()
                    .child(metric("State", ai_state_label.to_string()))
                    .child(metric("Providers", enabled_credentials.to_string()))
                    .child(metric("Models", enabled_ai_models.to_string()))
                    .child(metric("Default", ai_default_model))
                    .child(metric("Sessions", self.ai_session_count.to_string()))
                    .child(metric("Messages", self.ai_message_count.to_string()))
                    .child(metric("Audit", self.ai_audit_count.to_string())),
            )
            .child(
                div()
                    .mt_3()
                    .grid()
                    .grid_cols(4)
                    .gap_3()
                    .child(ai_boolean_state(
                        "Redaction",
                        self.ai_settings.redaction_enabled,
                    ))
                    .child(ai_boolean_state(
                        "Save Commands",
                        self.ai_settings.allow_save_command,
                    ))
                    .child(ai_boolean_state("History", self.ai_settings.record_history))
                    .child(compact_setting_state(
                        "User Agent",
                        truncate_preview(&self.ai_settings.request_user_agent, 34),
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(small_button(
                        "ai-redaction-toggle",
                        if self.ai_settings.redaction_enabled {
                            "Redact On"
                        } else {
                            "Redact Off"
                        },
                        cx.listener(|this, _, _, cx| {
                            this.toggle_ai_redaction(cx);
                        }),
                    ))
                    .child(small_button(
                        "ai-save-command-toggle",
                        if self.ai_settings.allow_save_command {
                            "Save On"
                        } else {
                            "Save Off"
                        },
                        cx.listener(|this, _, _, cx| {
                            this.toggle_ai_allow_save_command(cx);
                        }),
                    ))
                    .child(small_button(
                        "ai-history-toggle",
                        if self.ai_settings.record_history {
                            "History On"
                        } else {
                            "History Off"
                        },
                        cx.listener(|this, _, _, cx| {
                            this.toggle_ai_record_history(cx);
                        }),
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(policy_button(
                        "ai-provider-openai",
                        "OpenAI",
                        active_ai_profile_id == "openai",
                        cx.listener(|this, _, _, cx| {
                            this.update_ai_profile("openai", cx);
                        }),
                    ))
                    .child(policy_button(
                        "ai-provider-anthropic",
                        "Anthropic",
                        active_ai_profile_id == "anthropic",
                        cx.listener(|this, _, _, cx| {
                            this.update_ai_profile("anthropic", cx);
                        }),
                    ))
                    .child(policy_button(
                        "ai-provider-gemini",
                        "Gemini",
                        active_ai_profile_id == "gemini",
                        cx.listener(|this, _, _, cx| {
                            this.update_ai_profile("gemini", cx);
                        }),
                    ))
                    .child(policy_button(
                        "ai-provider-deepseek",
                        "DeepSeek",
                        active_ai_profile_id == "deepseek",
                        cx.listener(|this, _, _, cx| {
                            this.update_ai_profile("deepseek", cx);
                        }),
                    ))
                    .child(policy_button(
                        "ai-provider-ollama",
                        "Ollama",
                        active_ai_profile_id == "ollama",
                        cx.listener(|this, _, _, cx| {
                            this.update_ai_profile("ollama", cx);
                        }),
                    ))
                    .child(policy_button(
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
                    .mt_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(policy_button(
                        "ai-mode-ask",
                        "Ask",
                        self.ai_settings.default_mode == AiMode::Ask,
                        cx.listener(|this, _, _, cx| {
                            this.set_ai_mode(AiMode::Ask, cx);
                        }),
                    ))
                    .child(policy_button(
                        "ai-mode-agent",
                        "Agent",
                        self.ai_settings.default_mode == AiMode::Agent,
                        cx.listener(|this, _, _, cx| {
                            this.set_ai_mode(AiMode::Agent, cx);
                        }),
                    ))
                    .child(policy_button(
                        "ai-command-confirm",
                        "Confirm",
                        self.ai_settings.agent_command_execution_mode
                            == AgentCommandExecutionMode::ConfirmEach,
                        cx.listener(|this, _, _, cx| {
                            this.set_ai_command_mode(AgentCommandExecutionMode::ConfirmEach, cx);
                        }),
                    ))
                    .child(policy_button(
                        "ai-command-smart",
                        "Smart",
                        self.ai_settings.agent_command_execution_mode
                            == AgentCommandExecutionMode::Smart,
                        cx.listener(|this, _, _, cx| {
                            this.set_ai_command_mode(AgentCommandExecutionMode::Smart, cx);
                        }),
                    ))
                    .child(policy_button(
                        "ai-command-auto",
                        "Auto",
                        self.ai_settings.agent_command_execution_mode
                            == AgentCommandExecutionMode::Auto,
                        cx.listener(|this, _, _, cx| {
                            this.set_ai_command_mode(AgentCommandExecutionMode::Auto, cx);
                        }),
                    ))
                    .child(policy_button(
                        "ai-agent-bg-exec",
                        "BG Exec",
                        self.ai_settings.agent_background_execution_enabled,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_ai_background_execution(cx);
                        }),
                    ))
                    .child(small_button(
                        "ai-enabled",
                        if self.ai_settings.enabled {
                            "Enabled"
                        } else {
                            "Disabled"
                        },
                        cx.listener(|this, _, _, cx| {
                            this.toggle_ai_enabled(cx);
                        }),
                    ))
                    .child(small_button(
                        "ai-save",
                        "Save",
                        cx.listener(|this, _, _, cx| {
                            this.save_ai_settings(cx);
                        }),
                    ))
                    .child(small_button(
                        "ai-discover",
                        ai_discovery_label,
                        cx.listener(|this, _, _, cx| {
                            this.discover_ai_models(cx);
                        }),
                    )),
            )
            .child(
                div()
                    .mt_3()
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
            .child(
                div()
                    .mt_3()
                    .grid()
                    .grid_cols(3)
                    .gap_2()
                    .child(ai_setting_hint(
                        "General",
                        "Enable AI, redaction, history, and context limits.",
                    ))
                    .child(ai_setting_hint(
                        "Models",
                        "Discover provider models and choose a default.",
                    ))
                    .child(ai_setting_hint(
                        "Agent",
                        "Execution mode, risk gate, and background command policy.",
                    )),
            )
            .child(
                div()
                    .mt_3()
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
                            .child("Agent Limits"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(4)
                            .gap_3()
                            .child(metric(
                                "Context Lines",
                                self.ai_settings.context_line_limit.to_string(),
                            ))
                            .child(metric(
                                "Timeout",
                                format!("{} ms", self.ai_settings.timeout_ms),
                            ))
                            .child(metric(
                                "Steps",
                                self.ai_settings.max_agent_steps.unwrap_or(10).to_string(),
                            ))
                            .child(metric(
                                "Output Lines",
                                self.ai_settings.terminal_output_lines.to_string(),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .flex_wrap()
                            .child(small_button(
                                "ai-context-minus",
                                "-50 Lines",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_context_line_limit(-50, cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-context-plus",
                                "+50 Lines",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_context_line_limit(50, cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-timeout-minus",
                                "-1s",
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
                            ))
                            .child(small_button(
                                "ai-agent-steps-minus",
                                "-1 Step",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_agent_steps(-1, cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-agent-steps-plus",
                                "+1 Step",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_agent_steps(1, cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-output-lines-minus",
                                "-1 Out",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_terminal_output_lines(-1, cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-output-lines-plus",
                                "+1 Out",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_terminal_output_lines(1, cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::ui::view) fn ai_models_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let enabled_models = self
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
            .take(10)
            .enumerate()
            .fold(
                div().mt_3().flex().flex_col().gap_2(),
                |rows, (index, model)| {
                    let model_id = model.id.clone();
                    let default =
                        self.ai_settings.default_model_id.as_deref() == Some(model.id.as_str());
                    rows.child(
                        div()
                            .rounded_sm()
                            .border_1()
                            .border_color(if default {
                                rgb(0x4ade80)
                            } else {
                                rgb(0x263142)
                            })
                            .bg(rgb(0x0d1320))
                            .p_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight(800.))
                                                    .text_color(rgb(0xe5edf7))
                                                    .child(truncate_preview(&model.name, 48)),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .text_color(rgb(0x8f98aa))
                                                    .child(format!(
                                                        "{} / {}",
                                                        model
                                                            .provider_kind
                                                            .as_ref()
                                                            .map(ai_provider_kind_label)
                                                            .unwrap_or("unknown"),
                                                        ai_model_source_label(&model.source)
                                                    )),
                                            ),
                                    )
                                    .child(status_pill(
                                        if default {
                                            "default"
                                        } else if model.enabled {
                                            "enabled"
                                        } else {
                                            "off"
                                        },
                                        if model.enabled {
                                            rgb(0x6ee7b7)
                                        } else {
                                            rgb(0x98a3b8)
                                        },
                                        if model.enabled {
                                            rgb(0x12342a)
                                        } else {
                                            rgb(0x202633)
                                        },
                                    )),
                            )
                            .child(
                                div()
                                    .mt_2()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(small_button(
                                        format!("ai-model-toggle-{index}"),
                                        if model.enabled { "Disable" } else { "Enable" },
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
                            ),
                    )
                },
            );
        let credential_rows = self
            .ai_settings
            .provider_credentials
            .iter()
            .cloned()
            .take(8)
            .fold(
                div().mt_3().grid().grid_cols(2).gap_2(),
                |rows, credential| {
                    rows.child(
                        div()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(0x263142))
                            .bg(rgb(0x0d1320))
                            .p_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .child(
                                        div()
                                            .min_w_0()
                                            .text_xs()
                                            .font_weight(FontWeight(800.))
                                            .child(truncate_preview(&credential.name, 34)),
                                    )
                                    .child(status_pill(
                                        if credential.enabled { "enabled" } else { "off" },
                                        if credential.enabled {
                                            rgb(0x6ee7b7)
                                        } else {
                                            rgb(0x98a3b8)
                                        },
                                        if credential.enabled {
                                            rgb(0x12342a)
                                        } else {
                                            rgb(0x202633)
                                        },
                                    )),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .text_size(px(10.))
                                    .text_color(rgb(0x8f98aa))
                                    .child(format!(
                                        "{} / {}",
                                        ai_provider_kind_label(&credential.provider_kind),
                                        if credential.api_key.as_deref().unwrap_or("").is_empty() {
                                            "missing key"
                                        } else {
                                            "key set"
                                        }
                                    )),
                            ),
                    )
                },
            );

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Model List"),
                            )
                            .child(small_button(
                                "ai-models-discover",
                                ai_discovery_label,
                                cx.listener(|this, _, _, cx| {
                                    this.discover_ai_models(cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(4)
                            .gap_3()
                            .child(metric("Enabled", enabled_models.to_string()))
                            .child(metric("Total", self.ai_settings.models.len().to_string()))
                            .child(metric("Providers", enabled_credentials.to_string()))
                            .child(metric("Default", ai_default_model)),
                    )
                    .child(model_rows),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Provider Credentials"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .flex_wrap()
                            .child(policy_button(
                                "ai-model-provider-openai",
                                "OpenAI",
                                active_ai_profile_id == "openai",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("openai", cx);
                                }),
                            ))
                            .child(policy_button(
                                "ai-model-provider-anthropic",
                                "Anthropic",
                                active_ai_profile_id == "anthropic",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("anthropic", cx);
                                }),
                            ))
                            .child(policy_button(
                                "ai-model-provider-gemini",
                                "Gemini",
                                active_ai_profile_id == "gemini",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("gemini", cx);
                                }),
                            ))
                            .child(policy_button(
                                "ai-model-provider-deepseek",
                                "DeepSeek",
                                active_ai_profile_id == "deepseek",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("deepseek", cx);
                                }),
                            ))
                            .child(policy_button(
                                "ai-model-provider-ollama",
                                "Ollama",
                                active_ai_profile_id == "ollama",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("ollama", cx);
                                }),
                            ))
                            .child(policy_button(
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
                            .mt_3()
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
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "ai-model-tab-save",
                                "Save",
                                cx.listener(|this, _, _, cx| {
                                    this.save_ai_settings(cx);
                                }),
                            ))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x98a3b8))
                                    .child(self.ai_status.clone()),
                            ),
                    ),
            )
    }

    pub(in crate::ui::view) fn ai_rules_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(div().text_sm().font_weight(FontWeight(700.)).child("Rules"))
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(4)
                            .gap_3()
                            .child(metric("Max File", format!("{file_size_mb} MiB")))
                            .child(metric(
                                "Terminal Actions",
                                format!(
                                    "{terminal_enabled}/{}",
                                    self.ai_settings.terminal_ai_actions.len()
                                ),
                            ))
                            .child(metric(
                                "File Actions",
                                format!(
                                    "{file_enabled}/{}",
                                    self.ai_settings.file_ai_actions.len()
                                ),
                            ))
                            .child(metric(
                                "Smart Risk",
                                ai_risk_label(&self.ai_settings.agent_smart_auto_execute_max_risk),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .flex_wrap()
                            .child(small_button(
                                "ai-file-size-minus",
                                "-1 MiB",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_file_size_mb(-1, cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-file-size-plus",
                                "+1 MiB",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_file_size_mb(1, cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-agent-step-timeout-minus",
                                "-1s Step",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_agent_step_timeout_ms(-1_000, cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-agent-step-timeout-plus",
                                "+1s Step",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_agent_step_timeout_ms(1_000, cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-rules-save",
                                "Save",
                                cx.listener(|this, _, _, cx| {
                                    this.save_ai_settings(cx);
                                }),
                            )),
                    ),
            )
            .child(ai_action_list(
                "Terminal Actions",
                self.ai_settings
                    .terminal_ai_actions
                    .iter()
                    .cloned()
                    .collect(),
            ))
            .child(ai_action_list(
                "File Actions",
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

fn ai_action_list(title: &'static str, actions: Vec<AiCustomActionConfig>) -> impl IntoElement {
    let enabled = actions.iter().filter(|action| action.enabled).count();
    let rows =
        actions
            .into_iter()
            .take(6)
            .fold(div().mt_3().flex().flex_col().gap_2(), |rows, action| {
                rows.child(
                    div()
                        .rounded_sm()
                        .border_1()
                        .border_color(rgb(0x263142))
                        .bg(rgb(0x0d1320))
                        .p_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_3()
                                .child(
                                    div()
                                        .min_w_0()
                                        .text_xs()
                                        .font_weight(FontWeight(800.))
                                        .text_color(rgb(0xe5edf7))
                                        .child(truncate_preview(&action.name, 44)),
                                )
                                .child(status_pill(
                                    if action.enabled { "enabled" } else { "off" },
                                    if action.enabled {
                                        rgb(0x6ee7b7)
                                    } else {
                                        rgb(0x98a3b8)
                                    },
                                    if action.enabled {
                                        rgb(0x12342a)
                                    } else {
                                        rgb(0x202633)
                                    },
                                )),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_size(px(10.))
                                .text_color(rgb(0x8f98aa))
                                .line_height(px(14.))
                                .child(truncate_preview(&action.prompt, 120)),
                        ),
                )
            });

    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x151923))
        .p_4()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(div().text_sm().font_weight(FontWeight(700.)).child(title))
                .child(status_pill(
                    if enabled > 0 { "active" } else { "empty" },
                    if enabled > 0 {
                        rgb(0x6ee7b7)
                    } else {
                        rgb(0x98a3b8)
                    },
                    if enabled > 0 {
                        rgb(0x12342a)
                    } else {
                        rgb(0x202633)
                    },
                )),
        )
        .child(rows)
}
