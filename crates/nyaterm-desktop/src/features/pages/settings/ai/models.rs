use super::*;

#[path = "models/credential_rows.rs"]
mod credential_rows;
#[path = "models/model_groups.rs"]
mod model_groups;
impl NyaTermApp {
    pub(in crate::features) fn ai_models_settings_section(
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
        let models_summary =
            format!("{enabled_models} enabled · {total_models} total · default {ai_default_model}");
        let credentials_summary = format!("{enabled_credentials} enabled profiles");
        let active_ai_profile_id = self.ai_settings.active_profile_id.clone();
        let active_ai_api_key = ai_active_profile_api_key(&self.ai_settings);
        let ai_key_value = cloud_secret_display(&self.ai_secret_draft, &active_ai_api_key);
        let ai_discovery_label = if self.ai_discovery_pending {
            "Pending"
        } else {
            "Discover"
        };

        let model_groups = self.ai_model_groups(palette, cx);
        let credential_rows = self.ai_credential_rows(palette, cx);

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                palette,
                Some("Models"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(settings_form_row(
                        palette,
                        "Catalog",
                        Some(SharedString::from(models_summary)),
                        small_button(
                            palette,
                            "ai-models-discover",
                            ai_discovery_label,
                            cx.listener(|this, _, _, cx| {
                                this.discover_ai_models(cx);
                            }),
                        ),
                    ))
                    .child(model_groups),
            ))
            .child(settings_form_section(
                palette,
                Some("API keys"),
                Some("Per-provider credentials used for discovery and chat (Tauri AiModelsTab)."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "Profiles",
                        Some(SharedString::from(credentials_summary)),
                        small_button(
                            palette,
                            "ai-cred-add",
                            "+ Add",
                            cx.listener(|this, _, window, cx| {
                                this.add_ai_credential(window, cx);
                            }),
                        ),
                    ))
                    .child(credential_rows)
                    .child(settings_form_row(
                        palette,
                        "Legacy active profile",
                        Some(SharedString::from(
                            "Optional quick draft for the previously selected provider profile.",
                        )),
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .child(active_ai_profile_id.clone()),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(
                                palette,
                                "ai-model-provider-openai",
                                "OpenAI",
                                active_ai_profile_id == "openai",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("openai", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "ai-model-provider-anthropic",
                                "Anthropic",
                                active_ai_profile_id == "anthropic",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("anthropic", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "ai-model-provider-gemini",
                                "Gemini",
                                active_ai_profile_id == "gemini",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("gemini", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "ai-model-provider-deepseek",
                                "DeepSeek",
                                active_ai_profile_id == "deepseek",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("deepseek", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "ai-model-provider-ollama",
                                "Ollama",
                                active_ai_profile_id == "ollama",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("ollama", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
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
                    .child(settings_form_row(
                        palette,
                        "Actions",
                        Some(SharedString::from(self.ai_status.clone())),
                        small_button(
                            palette,
                            "ai-model-tab-save",
                            "Save profile draft",
                            cx.listener(|this, _, _, cx| {
                                this.save_ai_settings(cx);
                            }),
                        ),
                    )),
            ))
    }
}
