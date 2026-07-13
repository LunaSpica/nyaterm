use super::*;

pub(super) fn connection_editor_ssh_section(
    palette: crate::theme::ThemePalette,
    editor: &ConnectionEditorState,
    password_display: String,
    key_label: String,
    otp_label: String,
    proxy_label: String,
    jump_label: String,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(editor_field(
                    palette,
                    "connection-editor-host",
                    "Host",
                    editor.host.clone(),
                    editor.focused_field == ConnectionEditorField::Host,
                    cx.listener(|this, _, window, cx| {
                        this.focus_connection_editor_field(ConnectionEditorField::Host, window, cx);
                    }),
                ))
                .child(editor_field(
                    palette,
                    "connection-editor-port",
                    "Port",
                    editor.port.clone(),
                    editor.focused_field == ConnectionEditorField::Port,
                    cx.listener(|this, _, window, cx| {
                        this.focus_connection_editor_field(ConnectionEditorField::Port, window, cx);
                    }),
                )),
        )
        .child(editor_field(
            palette,
            "connection-editor-username",
            "Username",
            editor.username.clone(),
            editor.focused_field == ConnectionEditorField::Username,
            cx.listener(|this, _, window, cx| {
                this.focus_connection_editor_field(ConnectionEditorField::Username, window, cx);
            }),
        ))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.text_muted))
                        .child(format!("Auth · {}", editor.auth_mode)),
                )
                .child(small_button(
                    palette,
                    "connection-editor-auth",
                    "Cycle",
                    cx.listener(|this, _, _, cx| {
                        this.cycle_connection_editor_auth_mode(cx);
                    }),
                )),
        )
        .when(editor.auth_mode == "password", |this| {
            this.child(editor_field(
                palette,
                "connection-editor-password",
                "Password",
                password_display.clone(),
                editor.focused_field == ConnectionEditorField::Password,
                cx.listener(|this, _, window, cx| {
                    this.focus_connection_editor_field(ConnectionEditorField::Password, window, cx);
                }),
            ))
        })
        .when(
            editor.auth_mode == "key" || editor.auth_mode == "certificate",
            |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(palette.text_muted))
                                .child(format!("Key · {}", truncate_preview(&key_label, 24))),
                        )
                        .child(small_button(
                            palette,
                            "connection-editor-key",
                            "Cycle",
                            cx.listener(|this, _, _, cx| {
                                this.cycle_connection_editor_key(cx);
                            }),
                        )),
                )
            },
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.text_muted))
                        .child(format!("OTP · {}", truncate_preview(&otp_label, 24))),
                )
                .child(small_button(
                    palette,
                    "connection-editor-otp",
                    "Cycle",
                    cx.listener(|this, _, _, cx| {
                        this.cycle_connection_editor_otp(cx);
                    }),
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(palette.text_muted))
                                .child(format!("Proxy · {}", truncate_preview(&proxy_label, 36))),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(palette.text_dimmed))
                                .child("Optional SOCKS/HTTP/ProxyCommand for this SSH target."),
                        ),
                )
                .child(small_button(
                    palette,
                    "connection-editor-proxy",
                    "Cycle",
                    cx.listener(|this, _, _, cx| {
                        this.cycle_connection_editor_proxy(cx);
                    }),
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(palette.text_muted))
                                .child(format!(
                                    "Jump Host · {}",
                                    truncate_preview(&jump_label, 36)
                                )),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(palette.text_dimmed))
                                .child("ProxyJump via another saved SSH connection."),
                        ),
                )
                .child(small_button(
                    palette,
                    "connection-editor-jump",
                    "Cycle",
                    cx.listener(|this, _, _, cx| {
                        this.cycle_connection_editor_jump(cx);
                    }),
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(palette.text_muted))
                                .child(format!(
                                    "Backspace · {}",
                                    match editor.backspace_mode.as_str() {
                                        "ctrl-h" | "bs" | "ctrl_h" => "Ctrl+H (BS)",
                                        _ => "DEL (0x7F)",
                                    }
                                )),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(palette.text_dimmed))
                                .child("Terminal backspace key encoding for remote shells."),
                        ),
                )
                .child(small_button(
                    palette,
                    "connection-editor-backspace",
                    "Cycle",
                    cx.listener(|this, _, _, cx| {
                        this.cycle_connection_editor_backspace(cx);
                    }),
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(toggle_chip(
                    palette,
                    "OTP Fill",
                    editor.auto_fill_otp,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_connection_editor_flag(ConnectionEditorToggle::AutoFillOtp, cx);
                    }),
                ))
                .child(toggle_chip(
                    palette,
                    "X11",
                    editor.x11_forwarding,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_connection_editor_flag(ConnectionEditorToggle::X11, cx);
                    }),
                ))
                .child(toggle_chip(
                    palette,
                    "Post Login",
                    editor.post_login_enabled,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_connection_editor_flag(ConnectionEditorToggle::PostLogin, cx);
                    }),
                ))
                .child(toggle_chip(
                    palette,
                    "Open After Save",
                    editor.connect_after_save,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_connection_editor_flag(
                            ConnectionEditorToggle::ConnectAfterSave,
                            cx,
                        );
                    }),
                )),
        )
        .when(editor.post_login_enabled, |this| {
            this.child(
                div()
                    .pl_3()
                    .ml_1()
                    .border_l_1()
                    .border_color(rgb(palette.border))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(editor_field(
                        palette,
                        "connection-editor-post-login-command",
                        "Post-login command",
                        editor.post_login_command.clone(),
                        editor.focused_field == ConnectionEditorField::PostLoginCommand,
                        cx.listener(|this, _, window, cx| {
                            this.focus_connection_editor_field(
                                ConnectionEditorField::PostLoginCommand,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(editor_field(
                        palette,
                        "connection-editor-post-login-delay",
                        "Delay (ms)",
                        editor.post_login_delay_ms.clone(),
                        editor.focused_field == ConnectionEditorField::PostLoginDelay,
                        cx.listener(|this, _, window, cx| {
                            this.focus_connection_editor_field(
                                ConnectionEditorField::PostLoginDelay,
                                window,
                                cx,
                            );
                        }),
                    )),
            )
        })
}
