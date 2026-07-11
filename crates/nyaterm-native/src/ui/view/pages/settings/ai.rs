use super::*;
use crate::ui::theme::ThemePalette;

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

    pub(in crate::ui::view) fn ai_settings_section(
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

    pub(in crate::ui::view) fn ai_models_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
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


        // Group models by credential/provider (Tauri AiModelsTab groupModels).
        let credentials = self.ai_settings.provider_credentials.clone();
        let mut groups: Vec<(String, String, Option<nyaterm_domain::AiProviderCredential>, Vec<nyaterm_domain::AiModelConfigItem>)> =
            Vec::new();
        let mut group_index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for credential in &credentials {
            let key = credential.id.clone();
            group_index.insert(key.clone(), groups.len());
            groups.push((
                key,
                credential.name.clone(),
                Some(credential.clone()),
                Vec::new(),
            ));
        }
        for model in self.ai_settings.models.iter().cloned() {
            let key = model
                .credential_id
                .clone()
                .or_else(|| {
                    model
                        .provider_kind
                        .as_ref()
                        .map(|kind| format!("{kind:?}").to_ascii_lowercase())
                })
                .unwrap_or_else(|| "unknown".to_string());
            if let Some(index) = group_index.get(&key).copied() {
                groups[index].3.push(model);
            } else if let Some(index) = model
                .provider_kind
                .as_ref()
                .and_then(|kind| {
                    credentials
                        .iter()
                        .position(|credential| &credential.provider_kind == kind)
                })
            {
                groups[index].3.push(model);
            } else {
                let label = model
                    .provider_kind
                    .as_ref()
                    .map(ai_provider_kind_label)
                    .unwrap_or("Unknown")
                    .to_string();
                group_index.insert(key.clone(), groups.len());
                groups.push((key, label, None, vec![model]));
            }
        }
        // Sort models in group: enabled first.
        for group in &mut groups {
            group.3.sort_by_key(|model| (!model.enabled, model.name.to_ascii_lowercase()));
        }
        let collapsed = self.ai_model_collapsed_groups.clone();
        let default_id = self.ai_settings.default_model_id.clone();
        let manual_drafts = self.ai_manual_model_drafts.clone();
        let manual_edit_group = self.ai_manual_model_edit_group.clone();

        let model_groups = groups.into_iter().fold(
            div().flex().flex_col().gap_2(),
            |rows, (group_key, label, credential, models)| {
                let is_collapsed = collapsed.contains(&group_key);
                let enabled_in_group = models.iter().filter(|model| model.enabled).count();
                let total_in_group = models.len();
                let group_key_toggle = group_key.clone();
                let group_key_add = group_key.clone();
                let draft = manual_drafts
                    .get(&group_key)
                    .cloned()
                    .unwrap_or_else(|| " ".to_string());
                let draft_active = manual_edit_group.as_deref() == Some(group_key.as_str());
                let credential_id_for_add = credential.as_ref().map(|c| c.id.clone());
                let credential_enabled = credential.as_ref().map(|c| c.enabled).unwrap_or(false);
                let credential_id_toggle = credential.as_ref().map(|c| c.id.clone());

                rows.child(
                    div()
                        .id(SharedString::from(format!("ai-model-group-{group_key}")))
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
                                    "ai-model-group-header-{group_key}"
                                )))
                                .px_3()
                                .py_2()
                                .flex()
                                .items_center()
                                .gap_2()
                                .cursor_pointer()
                                .hover(|this| this.bg(rgb(palette.hover)))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.toggle_ai_model_group(group_key_toggle.clone(), cx);
                                }))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .child(if is_collapsed { "▸" } else { "▾" }),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight(600.))
                                        .text_color(rgb(palette.text))
                                        .overflow_hidden()
                                        .child(label),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(rgb(palette.text_muted))
                                        .child(format!(
                                            "{enabled_in_group}/{total_in_group} enabled"
                                        )),
                                )
                                .when_some(credential_id_toggle, |this, credential_id| {
                                    this.child(settings_switch(
                                        palette,
                                        format!("ai-cred-enabled-{credential_id}"),
                                        credential_enabled,
                                        cx.listener(move |this, _, _, cx| {
                                            this.toggle_ai_credential_enabled(
                                                credential_id.clone(),
                                                cx,
                                            );
                                        }),
                                    ))
                                }),
                        )
                        .when(!is_collapsed, |this| {
                            this.child(
                                div()
                                    .border_t_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.bg))
                                    .flex()
                                    .flex_col()
                                    .children(models.into_iter().map(|model| {
                                        let model_id = model.id.clone();
                                        let model_id_default = model.id.clone();
                                        let model_id_delete = model.id.clone();
                                        let is_default =
                                            default_id.as_deref() == Some(model.id.as_str());
                                        let is_manual =
                                            model.source == nyaterm_domain::AiModelSource::Manual;
                                        div()
                                            .id(SharedString::from(format!(
                                                "ai-model-row-{}",
                                                model.id
                                            )))
                                            .px_3()
                                            .py_1()
                                            .border_b_1()
                                            .border_color(rgb(palette.border))
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .bg(if is_default {
                                                rgb(palette.hover)
                                            } else {
                                                rgb(palette.bg)
                                            })
                                            .child(
                                                div()
                                                    .size(px(8.))
                                                    .rounded_full()
                                                    .flex_none()
                                                    .bg(if model.enabled {
                                                        rgb(palette.success)
                                                    } else {
                                                        rgb(palette.border)
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
                                                            .text_color(rgb(palette.text))
                                                            .overflow_hidden()
                                                            .child(truncate_preview(
                                                                &model.name, 48,
                                                            )),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(10.))
                                                            .text_color(rgb(palette.text_dimmed))
                                                            .child(format!(
                                                                "{}{}",
                                                                ai_model_source_label(&model.source),
                                                                if is_default {
                                                                    " · default"
                                                                } else {
                                                                    ""
                                                                }
                                                            )),
                                                    ),
                                            )
                                            .child(settings_switch(
                                                palette,
                                                format!("ai-model-enabled-{}", model.id),
                                                model.enabled,
                                                cx.listener(move |this, _, _, cx| {
                                                    this.toggle_ai_model_enabled(
                                                        model_id.clone(),
                                                        cx,
                                                    );
                                                }),
                                            ))
                                            .child(small_button(
                                                palette,
                                                format!("ai-model-default-{}", model.id),
                                                if is_default { "Default" } else { "Set" },
                                                cx.listener(move |this, _, _, cx| {
                                                    this.set_ai_default_model(
                                                        model_id_default.clone(),
                                                        cx,
                                                    );
                                                }),
                                            ))
                                            .when(is_manual, |this| {
                                                this.child(small_button(
                                                    palette,
                                                    format!("ai-model-delete-{}", model.id),
                                                    "Delete",
                                                    cx.listener(move |this, _, _, cx| {
                                                        this.remove_ai_manual_model(
                                                            model_id_delete.clone(),
                                                            cx,
                                                        );
                                                    }),
                                                ))
                                            })
                                    }))
                                    .when_some(credential_id_for_add, |this, credential_id| {
                                        let group_for_focus = group_key_add.clone();
                                        let group_for_add = group_key_add.clone();
                                        let draft_value = if draft.trim().is_empty() {
                                            " ".to_string()
                                        } else {
                                            draft.clone()
                                        };
                                        this.child(
                                            div()
                                                .px_3()
                                                .py_2()
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .id(SharedString::from(format!(
                                                            "ai-manual-model-{group_for_focus}"
                                                        )))
                                                        .min_w_0()
                                                        .flex_1()
                                                        .h(px(28.))
                                                        .px_2()
                                                        .rounded_md()
                                                        .border_1()
                                                        .border_color(if draft_active {
                                                            rgb(palette.accent)
                                                        } else {
                                                            rgb(palette.border)
                                                        })
                                                        .bg(rgb(palette.input))
                                                        .flex()
                                                        .items_center()
                                                        .text_size(px(12.))
                                                        .text_color(if draft.trim().is_empty() {
                                                            rgb(palette.text_dimmed)
                                                        } else {
                                                            rgb(palette.text)
                                                        })
                                                        .cursor_pointer()
                                                        .child(if draft.trim().is_empty() {
                                                            "Manual model name".to_string()
                                                        } else {
                                                            draft_value
                                                        })
                                                        .track_focus(&self.ai_manual_model_focus)
                                                        .on_click(cx.listener({
                                                            let group = group_for_focus.clone();
                                                            move |this, _, window, cx| {
                                                                this.ai_manual_model_edit_group =
                                                                    Some(group.clone());
                                                                window.focus(
                                                                    &this.ai_manual_model_focus,
                                                                );
                                                                cx.notify();
                                                            }
                                                        }))
                                                        .on_key_down(cx.listener({
                                                            let group = group_for_focus.clone();
                                                            move |this, event: &KeyDownEvent, _, cx| {
                                                                this.handle_ai_manual_model_key_down(
                                                                    &group, event, cx,
                                                                );
                                                            }
                                                        })),
                                                )
                                                .child(small_button(
                                                    palette,
                                                    format!("ai-manual-add-{group_for_add}"),
                                                    "Add",
                                                    cx.listener({
                                                        let credential_id = credential_id.clone();
                                                        let group = group_for_add.clone();
                                                        move |this, _, _, cx| {
                                                            let name = this
                                                                .ai_manual_model_drafts
                                                                .get(&group)
                                                                .cloned()
                                                                .unwrap_or_default();
                                                            this.add_ai_manual_model(
                                                                credential_id.clone(),
                                                                name,
                                                                cx,
                                                            );
                                                            this.ai_manual_model_drafts
                                                                .insert(group.clone(), String::new());
                                                        }
                                                    }),
                                                )),
                                        )
                                    }),
                            )
                        }),
                )
            },
        );


        let credential_rows = self
            .ai_settings
            .provider_credentials
            .iter()
            .cloned()
            .fold(div().flex().flex_col().gap_1(), |rows, credential| {
                let credential_id = credential.id.clone();
                rows.child(
                    div()
                        .id(SharedString::from(format!(
                            "ai-cred-row-{}",
                            credential.id
                        )))
                        .rounded_md()
                        .px_2()
                        .py_1()
                        .border_1()
                        .border_color(rgb(palette.surface_elevated))
                        .bg(rgb(palette.bg))
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .size(px(8.))
                                .rounded_full()
                                .flex_none()
                                .bg(if credential.enabled {
                                    rgb(palette.success)
                                } else {
                                    rgb(palette.border)
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
                                        .text_color(rgb(palette.text))
                                        .overflow_hidden()
                                        .child(truncate_preview(&credential.name, 36)),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .child(ai_provider_kind_label(&credential.provider_kind)),
                                ),
                        )
                        .child(settings_switch(
                            palette,
                            format!("ai-cred-list-enabled-{}", credential.id),
                            credential.enabled,
                            cx.listener(move |this, _, _, cx| {
                                this.toggle_ai_credential_enabled(credential_id.clone(), cx);
                            }),
                        )),
                )
            });

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(palette, 
                Some("Models"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(settings_form_row(palette, 
                        "Catalog",
                        Some(SharedString::from(models_summary)),
                        small_button(palette, 
                            "ai-models-discover",
                            ai_discovery_label,
                            cx.listener(|this, _, _, cx| {
                                this.discover_ai_models(cx);
                            }),
                        ),
                    ))
                    .child(model_groups),
            ))
            .child(settings_form_section(palette, 
                Some("Provider credentials"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Profiles",
                        Some(SharedString::from(credentials_summary)),
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .child("Stored"),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(palette, 
                                "ai-model-provider-openai",
                                "OpenAI",
                                active_ai_profile_id == "openai",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("openai", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "ai-model-provider-anthropic",
                                "Anthropic",
                                active_ai_profile_id == "anthropic",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("anthropic", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "ai-model-provider-gemini",
                                "Gemini",
                                active_ai_profile_id == "gemini",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("gemini", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "ai-model-provider-deepseek",
                                "DeepSeek",
                                active_ai_profile_id == "deepseek",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("deepseek", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "ai-model-provider-ollama",
                                "Ollama",
                                active_ai_profile_id == "ollama",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("ollama", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
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
                    .child(settings_form_row(palette, 
                        "Actions",
                        Some(SharedString::from(self.ai_status.clone())),
                        small_button(palette, 
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
        let step_timeout_s = (self.ai_settings.agent_step_timeout_ms.unwrap_or(30_000) / 1000).max(1);
        let smart_risk = ai_risk_label(&self.ai_settings.agent_smart_auto_execute_max_risk);

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(palette, 
                Some("Rules"),
                Some("Limits and auto-execute risk for AI-assisted actions."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Max AI file size",
                        Some(SharedString::from(format!("{file_size_mb} MiB per attachment"))),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(palette, 
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
                            .child(small_button(palette, 
                                "ai-file-size-plus",
                                "+1 MiB",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_file_size_mb(1, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Agent step timeout",
                        Some(SharedString::from(format!("{step_timeout_s}s per agent step"))),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(palette, 
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
                            .child(small_button(palette, 
                                "ai-agent-step-timeout-plus",
                                "+1s",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_ai_agent_step_timeout_ms(1_000, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Smart auto-execute risk",
                        Some(SharedString::from(format!("current: {smart_risk}"))),
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(palette, 
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
                            .child(settings_choice_chip(palette, 
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
                            .child(settings_choice_chip(palette, 
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
                            .child(settings_choice_chip(palette, 
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
                    .child(settings_form_row(palette, 
                        "Actions",
                        Some(SharedString::from(self.ai_status.clone())),
                        small_button(palette, 
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

    fn ai_action_editor(
        &mut self,
        palette: crate::ui::theme::ThemePalette,
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
                                .child(
                                    div()
                                        .size(px(8.))
                                        .rounded_full()
                                        .flex_none()
                                        .bg(if action.enabled {
                                            rgb(palette.success)
                                        } else {
                                            rgb(palette.border)
                                        }),
                                )
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
                                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                        this.handle_ai_action_key_down(event, cx);
                                    }))
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

fn ai_setting_hint(palette: crate::ui::theme::ThemePalette, title: &'static str, detail: &'static str) -> impl IntoElement {    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(800.))
                .text_color(rgb(palette.text))
                .child(title),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
                .line_height(px(14.))
                .child(detail),
        )
}

fn ai_boolean_state(palette: crate::ui::theme::ThemePalette, label: &'static str, enabled: bool) -> impl IntoElement {    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .child(div().text_xs().text_color(rgb(palette.text_muted)).child(label))
        .child(
            div()
                .mt_1()
                .text_sm()
                .font_weight(FontWeight(700.))
                .text_color(if enabled {
                    rgb(0x86efac)
                } else {
                    rgb(palette.text_muted)
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
