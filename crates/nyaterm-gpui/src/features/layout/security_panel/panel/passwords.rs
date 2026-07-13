use super::*;

impl NyaTermApp {
    pub(super) fn security_passwords_body(
        &mut self,
        palette: ThemePalette,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let mut body = security_auth_body_base();
        if let Some(editor) = self.security_password_editor.clone() {
            body = body.child(self.security_password_editor_view(editor, cx));
        } else if self.connection_saved_passwords.is_empty() {
            body = body.child(empty_panel("No saved passwords yet.", self.theme_palette()));
        } else {
            for entry in self.connection_saved_passwords.clone() {
                let id = entry.id.clone();
                let edit_id = entry.id.clone();
                let delete_id = entry.id.clone();
                let reveal_id = entry.id.clone();
                let copy_id = entry.id.clone();
                let is_revealed = self.security_revealed_passwords.contains_key(&entry.id);
                let revealed_value = self.security_revealed_passwords.get(&entry.id).cloned();
                // Tauri: masked until revealed; revealed shows secret + Copy.
                let secret_line = if is_revealed {
                    revealed_value
                        .clone()
                        .filter(|v| !v.is_empty())
                        .unwrap_or_else(|| "empty".to_string())
                } else if entry.has_password {
                    String::new()
                } else {
                    "empty".to_string()
                };
                body = body.child(
                    div()
                        .min_h(px(42.))
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
                                        .text_xs()
                                        .font_weight(FontWeight(600.))
                                        .text_color(rgb(palette.text))
                                        .overflow_hidden()
                                        .child(truncate_preview(&entry.name, 28)),
                                )
                                .when(is_revealed, |this| {
                                    this.child(
                                        div()
                                            .flex()
                                            .items_start()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .flex_1()
                                                    .font_family("JetBrains Mono")
                                                    .text_size(px(11.))
                                                    .text_color(rgb(palette.text_muted))
                                                    .child(truncate_preview(&secret_line, 36)),
                                            )
                                            .when(
                                                revealed_value
                                                    .as_ref()
                                                    .is_some_and(|v| !v.is_empty()),
                                                |this| {
                                                    this.child(small_button(
                                                        palette,
                                                        format!("security-pw-copy-{id}"),
                                                        "Copy",
                                                        cx.listener(move |this, _, window, cx| {
                                                            this.copy_security_password(
                                                                copy_id.clone(),
                                                                window,
                                                                cx,
                                                            );
                                                        }),
                                                    ))
                                                },
                                            ),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(small_button(
                                    palette,
                                    format!("security-pw-show-{id}"),
                                    if is_revealed { "Hide" } else { "Show" },
                                    cx.listener(move |this, _, window, cx| {
                                        this.reveal_security_password(
                                            reveal_id.clone(),
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    format!("security-pw-edit-{id}"),
                                    "Edit",
                                    cx.listener(move |this, _, window, cx| {
                                        this.open_security_password_editor(
                                            Some(edit_id.clone()),
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    format!("security-pw-del-{id}"),
                                    "Del",
                                    cx.listener(move |this, _, _, cx| {
                                        this.request_delete_security_password(
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
