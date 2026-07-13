use super::*;

pub(super) fn connection_editor_telnet_section(
    palette: crate::theme::ThemePalette,
    editor: &ConnectionEditorState,
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
                    "connection-editor-telnet-host",
                    "Host",
                    editor.host.clone(),
                    editor.focused_field == ConnectionEditorField::Host,
                    cx.listener(|this, _, window, cx| {
                        this.focus_connection_editor_field(ConnectionEditorField::Host, window, cx);
                    }),
                ))
                .child(editor_field(
                    palette,
                    "connection-editor-telnet-port",
                    "Port",
                    editor.port.clone(),
                    editor.focused_field == ConnectionEditorField::Port,
                    cx.listener(|this, _, window, cx| {
                        this.focus_connection_editor_field(ConnectionEditorField::Port, window, cx);
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
                .child(small_button(
                    palette,
                    "connection-editor-telnet-backspace",
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
                    "Raw TCP",
                    editor.raw_tcp_cli,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_connection_editor_flag(ConnectionEditorToggle::RawTcp, cx);
                    }),
                ))
                .child(toggle_chip(
                    palette,
                    "Local Echo",
                    editor.local_echo,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_connection_editor_flag(ConnectionEditorToggle::LocalEcho, cx);
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
}
