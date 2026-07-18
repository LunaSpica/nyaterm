use super::*;

impl NyaTermApp {
    pub(super) fn security_keys_body(
        &mut self,
        palette: ThemePalette,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let mut body = security_auth_body_base();
        body = body.child(security_tab_toolbar(
            palette,
            "SSH Keys",
            "security-add-key",
            "Add Key",
            self.security_key_editor.is_none(),
            cx.listener(|this, _, window, cx| {
                this.open_security_key_editor(None, window, cx);
            }),
        ));
        if let Some(editor) = self.security_key_editor.clone() {
            body = body.child(self.security_key_editor_view(editor, cx));
        } else if self.connection_ssh_keys.is_empty() {
            body = body.child(empty_panel(
                "No SSH keys yet. Add a private key to use key auth.",
                self.theme_palette(),
            ));
        } else {
            for key in self.connection_ssh_keys.clone() {
                let key_id = key.id.clone();
                let edit_id = key.id.clone();
                let delete_id = key.id.clone();
                body = body.child(
                    // Tauri security-auth: dense single-row list items + trailing actions.
                    div()
                        .h(px(42.))
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .bg(rgb(palette.input))
                        .px_2()
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
                                                .child(truncate_preview(&key.name, 28)),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(if key.has_key_data {
                                                    rgb(palette.success)
                                                } else {
                                                    rgb(palette.text_muted)
                                                })
                                                .child(if key.has_key_data {
                                                    "ready"
                                                } else {
                                                    "empty"
                                                }),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .overflow_hidden()
                                        .child(format!(
                                            "{} · cert {}",
                                            compact_id(&key_id),
                                            if key.has_cert_data { "yes" } else { "no" }
                                        )),
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
                                    format!("security-key-edit-{key_id}"),
                                    "Edit",
                                    cx.listener(move |this, _, window, cx| {
                                        this.open_security_key_editor(
                                            Some(edit_id.clone()),
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    format!("security-key-del-{key_id}"),
                                    "Del",
                                    cx.listener(move |this, _, _, cx| {
                                        this.request_delete_security_key(delete_id.clone(), cx);
                                    }),
                                )),
                        ),
                );
            }
        }
        body
    }
}
