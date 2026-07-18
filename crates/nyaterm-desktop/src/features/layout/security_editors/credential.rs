use super::*;

impl NyaTermApp {
    pub(in crate::features) fn security_credential_editor_view(
        &mut self,
        editor: SecurityCredentialEditorState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = if editor.id.is_some() {
            self.tr("credentialManager.editTitle")
        } else {
            self.tr("credentialManager.newTitle")
        };
        let password_display = if editor.password.is_empty() {
            if editor.has_password {
                self.tr("credentialManager.passwordUnchanged").to_string()
            } else {
                " ".to_string()
            }
        } else {
            "•".repeat(editor.password.chars().count().min(24))
        };
        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.bg))
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .track_focus(&self.security_credential_editor_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_security_credential_editor_key_down(event, window, cx);
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(title),
                    )
                    .child(small_button(
                        palette,
                        "security-cred-enabled",
                        if editor.enabled {
                            self.tr("credentialManager.enabled")
                        } else {
                            self.tr("credentialManager.disabled")
                        },
                        cx.listener(|this, _, _, cx| {
                            this.toggle_security_credential_enabled(cx);
                        }),
                    )),
            )
            .child(security_editor_field(
                palette,
                "security-cred-name",
                self.tr("credentialManager.nameLabel"),
                if editor.name.is_empty() {
                    " ".to_string()
                } else {
                    editor.name.clone()
                },
                editor.focused_field == SecurityCredentialEditorField::Name,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_credential_field(
                        SecurityCredentialEditorField::Name,
                        window,
                        cx,
                    );
                }),
            ))
            .child(security_editor_field(
                palette,
                "security-cred-user",
                self.tr("credentialManager.usernameLabel"),
                if editor.username.is_empty() {
                    " ".to_string()
                } else {
                    editor.username.clone()
                },
                editor.focused_field == SecurityCredentialEditorField::Username,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_credential_field(
                        SecurityCredentialEditorField::Username,
                        window,
                        cx,
                    );
                }),
            ))
            .child(security_editor_field(
                palette,
                "security-cred-pass",
                self.tr("credentialManager.passwordLabel"),
                password_display,
                editor.focused_field == SecurityCredentialEditorField::Password,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_credential_field(
                        SecurityCredentialEditorField::Password,
                        window,
                        cx,
                    );
                }),
            ))
            .child(security_editor_field(
                palette,
                "security-cred-user-re",
                self.tr("credentialManager.promptRegexLabel"),
                if editor.username_prompt_regex.is_empty() {
                    " ".to_string()
                } else {
                    editor.username_prompt_regex.clone()
                },
                editor.focused_field == SecurityCredentialEditorField::UsernameRegex,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_credential_field(
                        SecurityCredentialEditorField::UsernameRegex,
                        window,
                        cx,
                    );
                }),
            ))
            .child(security_editor_field(
                palette,
                "security-cred-pass-re",
                self.tr("credentialManager.passwordRegexPlaceholder"),
                if editor.password_prompt_regex.is_empty() {
                    " ".to_string()
                } else {
                    editor.password_prompt_regex.clone()
                },
                editor.focused_field == SecurityCredentialEditorField::PasswordRegex,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_credential_field(
                        SecurityCredentialEditorField::PasswordRegex,
                        window,
                        cx,
                    );
                }),
            ))
            .when_some(editor.error.clone(), |this, error| {
                this.child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette.danger))
                        .child(error),
                )
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(small_button(
                        palette,
                        "security-cred-save",
                        self.tr("common.save"),
                        cx.listener(|this, _, window, cx| {
                            this.save_security_credential_editor(window, cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "security-cred-cancel",
                        self.tr("common.cancel"),
                        cx.listener(|this, _, _, cx| {
                            this.close_security_credential_editor(cx);
                        }),
                    )),
            )
    }
}
