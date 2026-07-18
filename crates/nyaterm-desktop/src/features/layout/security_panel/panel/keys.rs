use super::*;

impl NyaTermApp {
    pub(super) fn security_keys_body(
        &mut self,
        palette: ThemePalette,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let mut body = security_auth_body_base("security-keys-body");
        body = body.child(security_tab_toolbar(
            palette,
            self.tr("securityAuth.keyManagement"),
            "security-add-key",
            self.tr("securityAuth.addKey"),
            self.security_key_editor.is_none(),
            cx.listener(|this, _, window, cx| {
                this.open_security_key_editor(None, window, cx);
            }),
        ));
        if let Some(editor) = self.security_key_editor.clone() {
            body = body.child(self.security_key_editor_view(editor, cx));
        } else if self.connection_ssh_keys.is_empty() {
            body = body.child(empty_panel(
                self.tr("securityAuth.noKeys"),
                self.theme_palette(),
            ));
        } else {
            let entries = self.connection_ssh_keys.clone();
            let entry_count = entries.len();
            let mut rows = div()
                .rounded_md()
                .border_1()
                .border_color(rgb(palette.border))
                .overflow_hidden();
            for (index, key) in entries.into_iter().enumerate() {
                let key_id = key.id.clone();
                let edit_id = key.id.clone();
                let delete_id = key.id.clone();
                rows = rows.child(
                    // Tauri security-auth: dense single-row list items + trailing actions.
                    div()
                        .h(px(42.))
                        .when(index + 1 < entry_count, |this| {
                            this.border_b_1().border_color(rgb(palette.border))
                        })
                        .px_3()
                        .flex()
                        .items_center()
                        .gap_2()
                        .hover(|this| this.bg(rgb(palette.hover)))
                        .child(
                            div().min_w_0().flex_1().flex().flex_col().child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text))
                                    .overflow_hidden()
                                    .child(truncate_preview(&key.name, 28)),
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
                                    self.tr("common.edit"),
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
                                    self.tr("common.delete"),
                                    cx.listener(move |this, _, _, cx| {
                                        this.request_delete_security_key(delete_id.clone(), cx);
                                    }),
                                )),
                        ),
                );
            }
            body = body.child(rows);
        }
        body
    }
}
