use super::*;

pub(super) fn connection_editor_telnet_section(
    palette: crate::theme::ThemePalette,
    editor: &ConnectionEditorState,
    backspace_options: Vec<ConnectionEditorChoice>,
    open_menu: Option<ConnectionEditorMenu>,
    language: &str,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let tr = |key: &'static str| crate::i18n::text(language, key);
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
                    "connection-editor-telnet-host",
                    tr("dialog.host"),
                    editor.host.clone(),
                    editor.focused_field == ConnectionEditorField::Host,
                    cx.listener(|this, _, window, cx| {
                        this.focus_connection_editor_field(ConnectionEditorField::Host, window, cx);
                    }),
                ))
                .child(editor_field(
                    palette,
                    "connection-editor-telnet-port",
                    tr("dialog.port"),
                    editor.port.clone(),
                    editor.focused_field == ConnectionEditorField::Port,
                    cx.listener(|this, _, window, cx| {
                        this.focus_connection_editor_field(ConnectionEditorField::Port, window, cx);
                    }),
                )),
        )
        .child(connection_editor_select(
            palette,
            "connection-editor-telnet-backspace",
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
                    tr("dialog.telnetRawTcpCli"),
                    editor.raw_tcp_cli,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_connection_editor_flag(ConnectionEditorToggle::RawTcp, cx);
                    }),
                ))
                .child(toggle_chip(
                    palette,
                    tr("dialog.telnetLocalEcho"),
                    editor.local_echo,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_connection_editor_flag(ConnectionEditorToggle::LocalEcho, cx);
                    }),
                )),
        )
}
