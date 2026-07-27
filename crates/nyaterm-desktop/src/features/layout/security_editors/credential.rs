use super::*;

use crate::models::SecurityCredentialEditorState;

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
        // A stored secret is never shown, so the box says so in its
        // placeholder rather than standing a row of bullets in for it.
        let password_placeholder = if editor.has_password {
            self.tr("credentialManager.passwordUnchanged")
        } else {
            ""
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
            .track_focus(&self.security.editors.credential_focus)
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
                self,
                "cred-name",
                self.tr("credentialManager.nameLabel"),
                editor.name.clone(),
                TextInputSetup::default(),
                cx,
            ))
            .child(security_editor_field(
                self,
                "cred-user",
                self.tr("credentialManager.usernameLabel"),
                editor.username.clone(),
                TextInputSetup::default(),
                cx,
            ))
            .child(security_editor_field(
                self,
                "cred-pass",
                self.tr("credentialManager.passwordLabel"),
                editor.password.clone(),
                TextInputSetup {
                    placeholder: password_placeholder.into(),
                    masked: true,
                    multi_line: false,
                },
                cx,
            ))
            .child(security_editor_field(
                self,
                "cred-user-re",
                self.tr("credentialManager.promptRegexLabel"),
                editor.username_prompt_regex.clone(),
                TextInputSetup::default(),
                cx,
            ))
            .child(security_editor_field(
                self,
                "cred-pass-re",
                self.tr("credentialManager.passwordRegexPlaceholder"),
                editor.password_prompt_regex.clone(),
                TextInputSetup::default(),
                cx,
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
