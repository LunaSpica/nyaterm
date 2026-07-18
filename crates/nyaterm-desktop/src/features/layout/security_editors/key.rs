use super::*;

impl NyaTermApp {
    pub(in crate::features) fn security_key_editor_view(
        &mut self,
        editor: SecurityKeyEditorState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = if editor.id.is_some() {
            self.tr("securityAuth.editKeyTitle")
        } else {
            self.tr("securityAuth.newKeyTitle")
        };
        let key_path_label = if !editor.key_file_path.trim().is_empty() {
            truncate_preview(&editor.key_file_path, 36)
        } else if editor.has_key_data {
            self.tr("securityAuth.loadedUnchanged").to_string()
        } else {
            self.tr("securityAuth.noKeySelected").to_string()
        };
        let cert_path_label = if !editor.cert_file_path.trim().is_empty() {
            truncate_preview(&editor.cert_file_path, 36)
        } else if editor.has_cert_data {
            self.tr("securityAuth.loadedUnchanged").to_string()
        } else {
            self.tr("securityAuth.optionalCertificate").to_string()
        };
        let passphrase_display = if editor.passphrase.is_empty() {
            " ".to_string()
        } else {
            "•".repeat(editor.passphrase.chars().count().min(24))
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
            .track_focus(&self.security_key_editor_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_security_key_editor_key_down(event, window, cx);
            }))
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight(800.))
                    .text_color(rgb(palette.text))
                    .child(title),
            )
            .child(security_editor_field(
                palette,
                "security-key-name",
                self.tr("securityAuth.nameLabel"),
                if editor.name.is_empty() {
                    " ".to_string()
                } else {
                    editor.name.clone()
                },
                editor.focused_field == SecurityKeyEditorField::Name,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_key_field(SecurityKeyEditorField::Name, window, cx);
                }),
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("securityAuth.privateKey")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .id(SharedString::from("security-key-path"))
                                    .flex_1()
                                    .min_w_0()
                                    .h(px(28.))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(
                                        if editor.focused_field == SecurityKeyEditorField::KeyPath {
                                            rgb(palette.link)
                                        } else {
                                            rgb(palette.border)
                                        },
                                    )
                                    .bg(rgb(palette.input))
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text))
                                    .cursor_pointer()
                                    .child(key_path_label)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.focus_security_key_field(
                                            SecurityKeyEditorField::KeyPath,
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(small_button(
                                palette,
                                "security-key-browse",
                                self.tr("securityAuth.browse"),
                                cx.listener(|this, _, window, cx| {
                                    this.pick_security_key_file(false, window, cx);
                                }),
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("securityAuth.certificate")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .id(SharedString::from("security-cert-path"))
                                    .flex_1()
                                    .min_w_0()
                                    .h(px(28.))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(
                                        if editor.focused_field == SecurityKeyEditorField::CertPath
                                        {
                                            rgb(palette.link)
                                        } else {
                                            rgb(palette.border)
                                        },
                                    )
                                    .bg(rgb(palette.input))
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text))
                                    .cursor_pointer()
                                    .child(cert_path_label)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.focus_security_key_field(
                                            SecurityKeyEditorField::CertPath,
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(small_button(
                                palette,
                                "security-cert-browse",
                                self.tr("securityAuth.browse"),
                                cx.listener(|this, _, window, cx| {
                                    this.pick_security_key_file(true, window, cx);
                                }),
                            )),
                    ),
            )
            .child(security_editor_field(
                palette,
                "security-key-passphrase",
                self.tr("securityAuth.passphrase"),
                passphrase_display,
                editor.focused_field == SecurityKeyEditorField::Passphrase,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_key_field(SecurityKeyEditorField::Passphrase, window, cx);
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
                        "security-key-save",
                        self.tr("common.save"),
                        cx.listener(|this, _, window, cx| {
                            this.save_security_key_editor(window, cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "security-key-cancel",
                        self.tr("common.cancel"),
                        cx.listener(|this, _, _, cx| {
                            this.close_security_key_editor(cx);
                        }),
                    )),
            )
    }
}
