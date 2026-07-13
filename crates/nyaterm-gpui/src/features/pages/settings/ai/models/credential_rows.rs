use super::*;

impl NyaTermApp {
    pub(super) fn ai_credential_rows(
        &mut self,
        palette: crate::theme::ThemePalette,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let expanded_credential_id = self.ai_credential_expanded_id.clone();
        let credential_edit = self.ai_credential_edit.clone();
        let credential_secret_drafts = self.ai_credential_secret_drafts.clone();
        let credential_rows = self.ai_settings.provider_credentials.iter().cloned().fold(
            div().flex().flex_col().gap_2(),
            |rows, credential| {
                let credential_id = credential.id.clone();
                let credential_id_header = credential.id.clone();
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
                let is_expanded = expanded_credential_id.as_deref() == Some(credential.id.as_str());
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
                        .border_color(rgb(if is_expanded {
                            palette.accent
                        } else {
                            palette.border
                        }))
                        .bg(rgb(palette.input))
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .id(SharedString::from(format!(
                                    "ai-cred-header-{}",
                                    credential.id
                                )))
                                .px_3()
                                .py_2()
                                .flex()
                                .items_center()
                                .gap_2()
                                .cursor_pointer()
                                .hover(|this| this.bg(rgb(palette.hover)))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.expand_ai_credential(credential_id_header.clone(), cx);
                                }))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .child(if is_expanded { "▾" } else { "▸" }),
                                )
                                .child(div().size(px(8.)).rounded_full().flex_none().bg(
                                    if credential.enabled {
                                        rgb(palette.success)
                                    } else {
                                        rgb(palette.border)
                                    },
                                ))
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
                                                .child(format!(
                                                    "{}{}",
                                                    ai_provider_kind_label(
                                                        &credential.provider_kind
                                                    ),
                                                    if is_builtin {
                                                        " · built-in"
                                                    } else {
                                                        " · custom"
                                                    }
                                                )),
                                        ),
                                )
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
                                        "Del",
                                        cx.listener(move |this, _, _, cx| {
                                            this.remove_ai_credential(
                                                credential_id_delete.clone(),
                                                cx,
                                            );
                                        }),
                                    ))
                                }),
                        )
                        .when(is_expanded, |this| {
                            this.child(
                            div()
                                .border_t_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.bg))
                                .px_3()
                                .py_2()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .track_focus(&self.ai_credential_focus)
                                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.handle_ai_credential_key_down(event, cx);
                                }))
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
                                                    "Profile name",
                                                    name_display.clone(),
                                                    active_field
                                                        == Some(AiCredentialEditorField::Name),
                                                    palette,
                                                )
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    this.focus_ai_credential_field(
                                                        cred_name.clone(),
                                                        AiCredentialEditorField::Name,
                                                        window,
                                                        cx,
                                                    );
                                                })),
                                            )
                                            .child(
                                                transfer_input(
                                                    format!("ai-cred-base-{}", credential_id),
                                                    "Base URL",
                                                    base_url_display.clone(),
                                                    active_field
                                                        == Some(AiCredentialEditorField::BaseUrl),
                                                    palette,
                                                )
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    this.focus_ai_credential_field(
                                                        cred_base.clone(),
                                                        AiCredentialEditorField::BaseUrl,
                                                        window,
                                                        cx,
                                                    );
                                                })),
                                            ),
                                    )
                                })
                                .child({
                                    let cred_key = credential_id.clone();
                                    transfer_input(
                                        format!("ai-cred-key-{}", credential_id),
                                        "API Key",
                                        api_key_display,
                                        active_field == Some(AiCredentialEditorField::ApiKey),
                                        palette,
                                    )
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.focus_ai_credential_field(
                                            cred_key.clone(),
                                            AiCredentialEditorField::ApiKey,
                                            window,
                                            cx,
                                        );
                                    }))
                                })
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(rgb(palette.text_dimmed))
                                                .child(
                                                    "Tab switches fields · Enter saves · Esc blurs",
                                                ),
                                        )
                                        .child(small_button(
                                            palette,
                                            format!("ai-cred-save-{}", credential_id),
                                            "Save",
                                            cx.listener(move |this, _, _, cx| {
                                                this.persist_ai_credential_edits(
                                                    &credential_id_save,
                                                    cx,
                                                );
                                            }),
                                        )),
                                ),
                        )
                        }),
                )
            },
        );
        credential_rows
    }
}
