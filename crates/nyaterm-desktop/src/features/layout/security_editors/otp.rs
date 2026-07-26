use super::*;

use crate::models::{SecurityOtpEditorField, SecurityOtpEditorState};

impl NyaTermApp {
    pub(in crate::features) fn security_otp_editor_view(
        &mut self,
        editor: SecurityOtpEditorState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = if editor.id.is_some() {
            self.tr("otpManager.editTitle")
        } else {
            self.tr("otpManager.newTitle")
        };
        let secret_display = if editor.secret.is_empty() {
            if editor.has_secret {
                self.tr("otpManager.secretUnchanged").to_string()
            } else {
                " ".to_string()
            }
        } else {
            "•".repeat(editor.secret.chars().count().min(24))
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
            .track_focus(&self.security.editors.otp_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_security_otp_editor_key_down(event, window, cx);
            }))
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight(800.))
                    .text_color(rgb(palette.text))
                    .child(title),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(security_type_chip(
                        palette,
                        "TOTP",
                        editor.otp_type != "hotp",
                        cx.listener(|this, _, _, cx| {
                            this.set_security_otp_type("totp", cx);
                        }),
                    ))
                    .child(security_type_chip(
                        palette,
                        "HOTP",
                        editor.otp_type == "hotp",
                        cx.listener(|this, _, _, cx| {
                            this.set_security_otp_type("hotp", cx);
                        }),
                    ))
                    .child(div().flex_1())
                    .child(
                        div()
                            .id(SharedString::from("security-otp-algo"))
                            .h(px(22.))
                            .px_2()
                            .flex()
                            .items_center()
                            .rounded_sm()
                            .text_size(px(10.))
                            .font_weight(FontWeight(700.))
                            .cursor_pointer()
                            .text_color(rgb(palette.text))
                            .bg(rgb(palette.surface_elevated))
                            .hover(|this| this.bg(rgb(palette.border)))
                            .child(editor.algorithm.clone())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cycle_security_otp_algorithm(cx);
                            })),
                    ),
            )
            .child(security_editor_field(
                palette,
                "security-otp-issuer",
                self.tr("otpManager.issuerLabel"),
                if editor.issuer.is_empty() {
                    " ".to_string()
                } else {
                    editor.issuer.clone()
                },
                editor.focused_field == SecurityOtpEditorField::Issuer,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_otp_field(SecurityOtpEditorField::Issuer, window, cx);
                }),
            ))
            .child(security_editor_field(
                palette,
                "security-otp-username",
                self.tr("otpManager.usernameLabel"),
                if editor.username.is_empty() {
                    " ".to_string()
                } else {
                    editor.username.clone()
                },
                editor.focused_field == SecurityOtpEditorField::Username,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_otp_field(SecurityOtpEditorField::Username, window, cx);
                }),
            ))
            .child(security_editor_field(
                palette,
                "security-otp-secret",
                self.tr("otpManager.secretLabel"),
                secret_display,
                editor.focused_field == SecurityOtpEditorField::Secret,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_otp_field(SecurityOtpEditorField::Secret, window, cx);
                }),
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(3)
                    .gap_2()
                    .child(security_editor_field(
                        palette,
                        "security-otp-digits",
                        self.tr("otpManager.digits"),
                        if editor.digits.is_empty() {
                            " ".to_string()
                        } else {
                            editor.digits.clone()
                        },
                        editor.focused_field == SecurityOtpEditorField::Digits,
                        cx.listener(|this, _, window, cx| {
                            this.focus_security_otp_field(
                                SecurityOtpEditorField::Digits,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(security_editor_field(
                        palette,
                        "security-otp-period",
                        self.tr("otpManager.period"),
                        if editor.period.is_empty() {
                            " ".to_string()
                        } else {
                            editor.period.clone()
                        },
                        editor.focused_field == SecurityOtpEditorField::Period,
                        cx.listener(|this, _, window, cx| {
                            this.focus_security_otp_field(
                                SecurityOtpEditorField::Period,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(security_editor_field(
                        palette,
                        "security-otp-counter",
                        self.tr("otpManager.counter"),
                        if editor.counter.is_empty() {
                            " ".to_string()
                        } else {
                            editor.counter.clone()
                        },
                        editor.focused_field == SecurityOtpEditorField::Counter,
                        cx.listener(|this, _, window, cx| {
                            this.focus_security_otp_field(
                                SecurityOtpEditorField::Counter,
                                window,
                                cx,
                            );
                        }),
                    )),
            )
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
                        "security-otp-save",
                        self.tr("common.save"),
                        cx.listener(|this, _, window, cx| {
                            this.save_security_otp_editor(window, cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "security-otp-cancel",
                        self.tr("common.cancel"),
                        cx.listener(|this, _, _, cx| {
                            this.close_security_otp_editor(cx);
                        }),
                    )),
            )
    }
}
