use super::*;

pub(super) fn connection_editor_ssh_section(
    palette: crate::theme::ThemePalette,
    editor: &ConnectionEditorState,
    password_display: String,
    key_label: String,
    otp_label: String,
    proxy_label: String,
    jump_label: String,
    auth_options: Vec<ConnectionEditorChoice>,
    key_options: Vec<ConnectionEditorChoice>,
    otp_options: Vec<ConnectionEditorChoice>,
    proxy_options: Vec<ConnectionEditorChoice>,
    jump_options: Vec<ConnectionEditorChoice>,
    backspace_options: Vec<ConnectionEditorChoice>,
    open_menu: Option<ConnectionEditorMenu>,
    language: &str,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let tr = |key: &'static str| crate::i18n::text(language, key);
    let auth_value = match editor.auth_mode.as_str() {
        "key" | "certificate" => tr("dialog.privateKey"),
        "none" => tr("dialog.noAuthentication"),
        _ => tr("dialog.password"),
    };
    let backspace_value = match editor.backspace_mode.as_str() {
        "ctrl-h" | "bs" | "ctrl_h" => tr("dialog.backspaceCtrlH"),
        _ => tr("dialog.backspaceDel"),
    };
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
                    tr("dialog.host"),
                    editor.host.clone(),
                    editor.focused_field == ConnectionEditorField::Host,
                    cx.listener(|this, _, window, cx| {
                        this.focus_connection_editor_field(ConnectionEditorField::Host, window, cx);
                    }),
                ))
                .child(editor_field(
                    palette,
                    "connection-editor-port",
                    tr("dialog.port"),
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
            tr("dialog.username"),
            editor.username.clone(),
            editor.focused_field == ConnectionEditorField::Username,
            cx.listener(|this, _, window, cx| {
                this.focus_connection_editor_field(ConnectionEditorField::Username, window, cx);
            }),
        ))
        .child(connection_editor_select(
            palette,
            "connection-editor-auth",
            tr("dialog.authentication"),
            auth_value,
            ConnectionEditorMenu::Authentication,
            open_menu == Some(ConnectionEditorMenu::Authentication),
            auth_options,
            cx,
        ))
        .when(editor.auth_mode == "password", |this| {
            this.child(editor_field(
                palette,
                "connection-editor-password",
                tr("dialog.password"),
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
                this.child(connection_editor_select(
                    palette,
                    "connection-editor-key",
                    tr("dialog.privateKey"),
                    truncate_preview(&key_label, 36),
                    ConnectionEditorMenu::SshKey,
                    open_menu == Some(ConnectionEditorMenu::SshKey),
                    key_options,
                    cx,
                ))
            },
        )
        .child(connection_editor_select(
            palette,
            "connection-editor-otp",
            tr("dialog.selectOtp"),
            truncate_preview(&otp_label, 36),
            ConnectionEditorMenu::Otp,
            open_menu == Some(ConnectionEditorMenu::Otp),
            otp_options,
            cx,
        ))
        .child(connection_editor_select(
            palette,
            "connection-editor-proxy",
            tr("dialog.proxySelect"),
            truncate_preview(&proxy_label, 48),
            ConnectionEditorMenu::Proxy,
            open_menu == Some(ConnectionEditorMenu::Proxy),
            proxy_options,
            cx,
        ))
        .child(connection_editor_select(
            palette,
            "connection-editor-jump",
            tr("dialog.proxyJump"),
            truncate_preview(&jump_label, 48),
            ConnectionEditorMenu::ProxyJump,
            open_menu == Some(ConnectionEditorMenu::ProxyJump),
            jump_options,
            cx,
        ))
        .child(connection_editor_select(
            palette,
            "connection-editor-backspace",
            tr("dialog.backspaceMode"),
            backspace_value,
            ConnectionEditorMenu::Backspace,
            open_menu == Some(ConnectionEditorMenu::Backspace),
            backspace_options,
            cx,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .flex_wrap()
                .gap_1()
                .child(toggle_chip(
                    palette,
                    tr("dialog.autoFillOtp"),
                    editor.auto_fill_otp,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_connection_editor_flag(ConnectionEditorToggle::AutoFillOtp, cx);
                    }),
                ))
                .child(toggle_chip(
                    palette,
                    tr("dialog.x11Forwarding"),
                    editor.x11_forwarding,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_connection_editor_flag(ConnectionEditorToggle::X11, cx);
                    }),
                ))
                .child(toggle_chip(
                    palette,
                    tr("dialog.postLoginCommand"),
                    editor.post_login_enabled,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_connection_editor_flag(ConnectionEditorToggle::PostLogin, cx);
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
                        tr("dialog.postLoginCommandContent"),
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
                        tr("dialog.postLoginDelay"),
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
