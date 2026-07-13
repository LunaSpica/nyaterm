use super::*;

impl NyaTermApp {
    pub(super) fn security_credentials_body(
        &mut self,
        palette: ThemePalette,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let mut body = security_auth_body_base();
        if let Some(editor) = self.security_credential_editor.clone() {
            body = body.child(self.security_credential_editor_view(editor, cx));
        } else if self.connection_saved_credentials.is_empty() {
            body = body.child(empty_panel(
                "No autofill credentials yet.",
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
                                                .child(if entry.enabled { "on" } else { "off" }),
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
                                    if entry.enabled { "Off" } else { "On" },
                                    cx.listener(move |this, _, _, cx| {
                                        this.toggle_security_credential_list_enabled(
                                            toggle_id.clone(),
                                            cx,
                                        );
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    format!("security-cred-show-{id}"),
                                    if is_revealed { "Hide" } else { "Show" },
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
                                    "Edit",
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
                                    "Del",
                                    cx.listener(move |this, _, _, cx| {
                                        this.request_delete_security_credential(
                                            delete_id.clone(),
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
