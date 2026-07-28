use gpui::{
    Context, FontWeight, IntoElement, KeyDownEvent, SharedString, div, prelude::*, px, rgb,
};

use crate::features::{NyaTermApp, TextInputSetup};
use crate::models::SecurityOtpEditorState;
use crate::widgets::small_button;

use super::super::view_helpers::{security_editor_field, security_type_chip};

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
        // A stored secret is never shown, so the box says so in its
        // placeholder rather than standing a row of bullets in for it.
        let secret_placeholder = if editor.has_secret {
            self.tr("otpManager.secretUnchanged")
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
                self,
                "otp-issuer",
                self.tr("otpManager.issuerLabel"),
                editor.issuer.clone(),
                TextInputSetup::default(),
                cx,
            ))
            .child(security_editor_field(
                self,
                "otp-username",
                self.tr("otpManager.usernameLabel"),
                editor.username.clone(),
                TextInputSetup::default(),
                cx,
            ))
            .child(security_editor_field(
                self,
                "otp-secret",
                self.tr("otpManager.secretLabel"),
                editor.secret.clone(),
                TextInputSetup {
                    placeholder: secret_placeholder.into(),
                    masked: true,
                    multi_line: false,
                },
                cx,
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(3)
                    .gap_2()
                    .child(security_editor_field(
                        self,
                        "otp-digits",
                        self.tr("otpManager.digits"),
                        editor.digits.clone(),
                        TextInputSetup::default(),
                        cx,
                    ))
                    .child(security_editor_field(
                        self,
                        "otp-period",
                        self.tr("otpManager.period"),
                        editor.period.clone(),
                        TextInputSetup::default(),
                        cx,
                    ))
                    .child(security_editor_field(
                        self,
                        "otp-counter",
                        self.tr("otpManager.counter"),
                        editor.counter.clone(),
                        TextInputSetup::default(),
                        cx,
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
