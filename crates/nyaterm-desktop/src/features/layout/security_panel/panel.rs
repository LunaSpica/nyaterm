use super::*;

#[path = "panel/credentials.rs"]
mod credentials;
#[path = "panel/keys.rs"]
mod keys;
#[path = "panel/otp.rs"]
mod otp;
#[path = "panel/passwords.rs"]
mod passwords;

impl NyaTermApp {
    pub(in crate::features) fn security_auth_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active_tab = self.security_auth_tab;
        let key_count = self.connection_ssh_keys.len();
        let password_count = self.connection_saved_passwords.len();
        let credential_count = self.connection_saved_credentials.len();
        let otp_count = self.connection_otp_entries.len();
        let master = if self.settings.has_master_password {
            "configured"
        } else {
            "not set"
        };
        let palette = self.theme_palette();

        let mut body = match active_tab {
            SecurityAuthTab::Keys => self.security_keys_body(palette, cx),
            SecurityAuthTab::Passwords => self.security_passwords_body(palette, cx),
            SecurityAuthTab::Credentials => self.security_credentials_body(palette, cx),
            SecurityAuthTab::Otp => self.security_otp_body(palette, cx),
        };

        if let Some(confirm) = self.security_delete_confirm.clone() {
            body = body.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.danger))
                    .bg(rgb(palette.hover))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.danger))
                            .child(format!("Delete {}?", confirm.label)),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "security-delete-confirm",
                                "Delete",
                                cx.listener(|this, _, _, cx| {
                                    this.confirm_security_delete(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "security-delete-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_security_delete(cx);
                                }),
                            )),
                    ),
            );
        }

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(rgb(palette.surface))
            .child(
                div()
                    .px_3()
                    .pt_3()
                    .pb_2()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.section_header))
                    .flex()
                    .flex_col()
                    .gap_2()
                    // Tauri SecurityAuthPanel: full-width 4-col segment tabs under PanelHeader.
                    .child(
                        div()
                            .h(px(32.))
                            .w_full()
                            .rounded_md()
                            .bg(rgb(palette.surface_elevated))
                            .p(px(2.))
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .child(self.security_tab_chip(SecurityAuthTab::Keys, cx))
                            .child(self.security_tab_chip(SecurityAuthTab::Passwords, cx))
                            .child(self.security_tab_chip(SecurityAuthTab::Otp, cx))
                            .child(self.security_tab_chip(SecurityAuthTab::Credentials, cx)),
                    )
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
                                    .child(self.security_status.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(rgb(palette.text_dimmed))
                                            .child(format!(
                                                "MP {master} · K{key_count}/P{password_count}/C{credential_count}/O{otp_count}"
                                            )),
                                    )
                                    .when(
                                        self.security_auth_tab == SecurityAuthTab::Otp
                                            && !self.connection_otp_entries.is_empty(),
                                        |this| {
                                            this.child(small_button(
                                                palette,
                                                "security-otp-refresh-all",
                                                "Refresh",
                                                cx.listener(|this, _, window, cx| {
                                                    this.refresh_visible_security_otp_codes(
                                                        window, cx,
                                                    );
                                                }),
                                            ))
                                        },
                                    )
                                    .child(small_button(palette,
                                        "security-add-item",
                                        "Add",
                                        cx.listener(|this, _, window, cx| {
                                            match this.security_auth_tab {
                                                SecurityAuthTab::Keys => {
                                                    this.open_security_key_editor(
                                                        None, window, cx,
                                                    );
                                                }
                                                SecurityAuthTab::Passwords => {
                                                    this.open_security_password_editor(
                                                        None, window, cx,
                                                    );
                                                }
                                                SecurityAuthTab::Credentials => {
                                                    this.open_security_credential_editor(
                                                        None, window, cx,
                                                    );
                                                }
                                                SecurityAuthTab::Otp => {
                                                    this.open_security_otp_editor(
                                                        None, window, cx,
                                                    );
                                                }
                                            }
                                        }),
                                    )),
                            ),
                    ),
            )
            .child(body)
            .child(self.security_secret_footer(cx))
            .when(self.security_unlock_prompt_open, |this| {
                this.child(self.security_unlock_prompt(cx))
            })
    }
}

fn security_auth_body_base() -> gpui::Div {
    div()
        .flex_1()
        .min_h_0()
        .overflow_hidden()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
}
