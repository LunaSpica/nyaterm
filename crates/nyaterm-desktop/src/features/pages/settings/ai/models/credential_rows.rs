use super::*;

use crate::models::AiCredentialEditorField;

impl NyaTermApp {
    pub(super) fn ai_credential_rows(
        &mut self,
        palette: crate::theme::ThemePalette,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let credential_edit = self.ai.settings.credential_edit.clone();
        let credential_secret_drafts = self.ai.settings.credential_secret_drafts.clone();
        let profile_name_label = self.tr("ai.profileName");
        let base_url_label = self.tr("ai.baseUrl");
        let api_key_label = self.tr("settings.apiKey");
        let delete_label = self.tr("common.delete");
        let save_label = self.tr("common.save");

        self.ai
            .settings
            .config
            .provider_credentials
            .iter()
            .cloned()
            .fold(div().flex().flex_col().gap_4(), |rows, credential| {
                let credential_id = credential.id.clone();
                let credential_id_toggle = credential.id.clone();
                let credential_id_delete = credential.id.clone();
                let credential_id_save = credential.id.clone();
                let is_builtin = matches!(
                    credential.id.as_str(),
                    "openai"
                        | "anthropic"
                        | "gemini"
                        | "deepseek"
                        | "ollama"
                        | "xai"
                        | "cohere"
                        | "mimo"
                        | "zai"
                        | "groq"
                );
                let active_field = credential_edit
                    .as_ref()
                    .and_then(|(id, field)| (id == &credential.id).then_some(*field));
                let secret_draft = credential_secret_drafts
                    .get(&credential.id)
                    .cloned()
                    .unwrap_or_default();
                let api_key_display = cloud_secret_display(&secret_draft, &credential.api_key);
                let base_url_display = credential
                    .base_url
                    .clone()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| " ".to_string());
                let name_display = if credential.name.trim().is_empty() {
                    " ".to_string()
                } else {
                    credential.name.clone()
                };

                rows.child(
                    div()
                        .id(SharedString::from(format!(
                            "ai-cred-card-{}",
                            credential.id
                        )))
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .bg(rgb(palette.bg))
                        .p_4()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .track_focus(&self.ai.settings.credential_focus)
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                            cx.stop_propagation();
                            this.handle_ai_credential_key_down(event, cx);
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
                                        .child(credential.name.clone()),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(settings_switch(
                                            palette,
                                            format!("ai-cred-list-enabled-{}", credential.id),
                                            credential.enabled,
                                            cx.listener(move |this, _, _, cx| {
                                                this.toggle_ai_credential_enabled(
                                                    credential_id_toggle.clone(),
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .when(!is_builtin, |this| {
                                            this.child(small_button(
                                                palette,
                                                format!("ai-cred-delete-{}", credential.id),
                                                delete_label,
                                                cx.listener(move |this, _, _, cx| {
                                                    this.remove_ai_credential(
                                                        credential_id_delete.clone(),
                                                        cx,
                                                    );
                                                }),
                                            ))
                                        }),
                                ),
                        )
                        .when(!is_builtin, |body| {
                            let cred_name = credential_id.clone();
                            let cred_base = credential_id.clone();
                            body.child(
                                div()
                                    .grid()
                                    .grid_cols(2)
                                    .gap_2()
                                    .child(
                                        transfer_input(
                                            format!("ai-cred-name-{}", credential_id),
                                            profile_name_label,
                                            name_display.clone(),
                                            active_field == Some(AiCredentialEditorField::Name),
                                            palette,
                                        )
                                        .on_click(
                                            cx.listener(move |this, _, window, cx| {
                                                this.focus_ai_credential_field(
                                                    cred_name.clone(),
                                                    AiCredentialEditorField::Name,
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ),
                                    )
                                    .child(
                                        transfer_input(
                                            format!("ai-cred-base-{}", credential_id),
                                            base_url_label,
                                            base_url_display.clone(),
                                            active_field == Some(AiCredentialEditorField::BaseUrl),
                                            palette,
                                        )
                                        .on_click(
                                            cx.listener(move |this, _, window, cx| {
                                                this.focus_ai_credential_field(
                                                    cred_base.clone(),
                                                    AiCredentialEditorField::BaseUrl,
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ),
                                    ),
                            )
                        })
                        .child({
                            let cred_key = credential_id.clone();
                            transfer_input(
                                format!("ai-cred-key-{}", credential_id),
                                api_key_label,
                                api_key_display,
                                active_field == Some(AiCredentialEditorField::ApiKey),
                                palette,
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.focus_ai_credential_field(
                                        cred_key.clone(),
                                        AiCredentialEditorField::ApiKey,
                                        window,
                                        cx,
                                    );
                                },
                            ))
                        })
                        .child(div().flex().justify_end().child(small_button(
                            palette,
                            format!("ai-cred-save-{}", credential_id),
                            save_label,
                            cx.listener(move |this, _, _, cx| {
                                this.persist_ai_credential_edits(&credential_id_save, cx);
                            }),
                        ))),
                )
            })
    }
}
