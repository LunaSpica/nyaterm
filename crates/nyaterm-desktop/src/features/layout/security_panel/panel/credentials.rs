use super::*;

impl NyaTermApp {
    pub(super) fn security_credentials_body(
        &mut self,
        palette: ThemePalette,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let mut body = security_auth_body_base("security-credentials-body");
        body = body.child(security_tab_toolbar(
            palette,
            self.tr("credentialManager.title"),
            "security-add-credential",
            self.tr("credentialManager.add"),
            self.security_credential_editor.is_none(),
            cx.listener(|this, _, window, cx| {
                this.open_security_credential_editor(None, window, cx);
            }),
        ));
        if let Some(editor) = self.security_credential_editor.clone() {
            body = body.child(self.security_credential_editor_view(editor, cx));
        } else if self.connection_saved_credentials.is_empty() {
            body = body.child(empty_panel(
                self.tr("credentialManager.noCredentials"),
                self.theme_palette(),
            ));
        } else {
            for entry in self.connection_saved_credentials.clone() {
                let id = entry.id.clone();
                let edit_id = entry.id.clone();
                let delete_id = entry.id.clone();
                let reveal_id = entry.id.clone();
                let toggle_id = entry.id.clone();
                let is_revealed = self.security_revealed_credentials.contains_key(&entry.id);
                let secret = self
                    .security_revealed_credentials
                    .get(&entry.id)
                    .cloned()
                    .unwrap_or_default();
                body = body.child(
                    div()
                        .min_h(px(48.))
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .bg(rgb(palette.input))
                        .px_2()
                        .py_1()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(1.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .min_w_0()
                                                .flex_1()
                                                .text_xs()
                                                .font_weight(FontWeight(600.))
                                                .text_color(rgb(palette.text))
                                                .overflow_hidden()
                                                .child(truncate_preview(&entry.name, 24)),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(if entry.enabled {
                                                    rgb(palette.success)
                                                } else {
                                                    rgb(palette.text_muted)
                                                })
                                                .child(self.tr(if entry.enabled {
                                                    "credentialManager.enabled"
                                                } else {
                                                    "credentialManager.disabled"
                                                })),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .overflow_hidden()
                                        .child(if is_revealed {
                                            format!(
                                                "{} · {}",
                                                truncate_preview(&entry.username, 18),
                                                truncate_preview(&secret, 16)
                                            )
                                        } else {
                                            truncate_preview(&entry.username, 28)
                                        }),
                                ),
                        )
                        .child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(small_button(
                                    palette,
                                    format!("security-cred-toggle-{id}"),
                                    self.tr(if entry.enabled {
                                        "credentialManager.disabled"
                                    } else {
                                        "credentialManager.enabled"
                                    }),
                                    cx.listener(move |this, _, window, cx| {
                                        this.toggle_security_credential_list_enabled(
                                            toggle_id.clone(),
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    format!("security-cred-show-{id}"),
                                    self.tr(if is_revealed {
                                        "credentialManager.hidePassword"
                                    } else {
                                        "credentialManager.showPassword"
                                    }),
                                    cx.listener(move |this, _, window, cx| {
                                        this.reveal_security_credential_password(
                                            reveal_id.clone(),
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    format!("security-cred-edit-{id}"),
                                    self.tr("common.edit"),
                                    cx.listener(move |this, _, window, cx| {
                                        this.open_security_credential_editor(
                                            Some(edit_id.clone()),
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    format!("security-cred-del-{id}"),
                                    self.tr("common.delete"),
                                    cx.listener(move |this, _, window, cx| {
                                        this.request_delete_security_credential(
                                            delete_id.clone(),
                                            window,
                                            cx,
                                        );
                                    }),
                                )),
                        ),
                );
            }
        }
        body
    }
}
