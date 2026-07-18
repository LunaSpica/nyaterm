use super::*;

pub(super) fn connection_editor_serial_section(
    palette: crate::theme::ThemePalette,
    editor: &ConnectionEditorState,
    serial_port_options: Vec<ConnectionEditorChoice>,
    baud_options: Vec<ConnectionEditorChoice>,
    data_bits_options: Vec<ConnectionEditorChoice>,
    parity_options: Vec<ConnectionEditorChoice>,
    stop_bits_options: Vec<ConnectionEditorChoice>,
    backspace_options: Vec<ConnectionEditorChoice>,
    open_menu: Option<ConnectionEditorMenu>,
    language: &str,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let tr = |key: &'static str| crate::i18n::text(language, key);
    let parity_value = match editor.parity.as_str() {
        "even" => tr("dialog.parityEven"),
        "odd" => tr("dialog.parityOdd"),
        "mark" => tr("dialog.parityMark"),
        "space" => tr("dialog.paritySpace"),
        _ => tr("dialog.parityNone"),
    };
    let backspace_value = match editor.backspace_mode.as_str() {
        "ctrl-h" | "bs" | "ctrl_h" => tr("dialog.backspaceCtrlH"),
        _ => tr("dialog.backspaceDel"),
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(connection_editor_select(
            palette,
            "connection-editor-serial-port",
            tr("dialog.serialPort"),
            if editor.serial_port.is_empty() {
                tr("dialog.selectSerialPort").to_string()
            } else {
                editor.serial_port.clone()
            },
            ConnectionEditorMenu::SerialPort,
            open_menu == Some(ConnectionEditorMenu::SerialPort),
            serial_port_options,
            cx,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .items_end()
                .gap_2()
                .child(div().min_w_0().flex_1().child(editor_field(
                    palette,
                    "connection-editor-baud",
                    tr("dialog.baudRate"),
                    editor.baud_rate.clone(),
                    editor.focused_field == ConnectionEditorField::BaudRate,
                    cx.listener(|this, _, window, cx| {
                        this.focus_connection_editor_field(
                            ConnectionEditorField::BaudRate,
                            window,
                            cx,
                        );
                    }),
                )))
                .child(div().w(px(144.)).child(connection_editor_select(
                    palette,
                    "connection-editor-baud-preset",
                    "",
                    editor.baud_rate.clone(),
                    ConnectionEditorMenu::BaudRate,
                    open_menu == Some(ConnectionEditorMenu::BaudRate),
                    baud_options,
                    cx,
                ))),
        )
        .child(connection_editor_select(
            palette,
            "connection-editor-data-bits",
            tr("dialog.dataBits"),
            editor.data_bits.clone(),
            ConnectionEditorMenu::DataBits,
            open_menu == Some(ConnectionEditorMenu::DataBits),
            data_bits_options,
            cx,
        ))
        .child(connection_editor_select(
            palette,
            "connection-editor-parity",
            tr("dialog.parity"),
            parity_value,
            ConnectionEditorMenu::Parity,
            open_menu == Some(ConnectionEditorMenu::Parity),
            parity_options,
            cx,
        ))
        .child(connection_editor_select(
            palette,
            "connection-editor-stop-bits",
            tr("dialog.stopBits"),
            editor.stop_bits.clone(),
            ConnectionEditorMenu::StopBits,
            open_menu == Some(ConnectionEditorMenu::StopBits),
            stop_bits_options,
            cx,
        ))
        .child(connection_editor_select(
            palette,
            "connection-editor-serial-backspace",
            tr("dialog.backspaceMode"),
            backspace_value,
            ConnectionEditorMenu::Backspace,
            open_menu == Some(ConnectionEditorMenu::Backspace),
            backspace_options,
            cx,
        ))
}
