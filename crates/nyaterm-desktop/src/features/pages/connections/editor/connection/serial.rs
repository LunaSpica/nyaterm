use super::*;

pub(super) fn connection_editor_serial_section(
    palette: crate::theme::ThemePalette,
    editor: &ConnectionEditorState,
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
                            "{} · {}",
                            tr("dialog.serialPort"),
                            if editor.serial_port.is_empty() {
                                tr("dialog.selectSerialPort")
                            } else {
                                &editor.serial_port
                            }
                        )),
                )
                .child(small_button(
                    palette,
                    "connection-editor-serial-port",
                    tr("common.more"),
                    cx.listener(|this, _, _, cx| {
                        this.cycle_connection_editor_serial_port(cx);
                    }),
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(editor_field(
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
                ))
                .child(small_button(
                    palette,
                    "connection-editor-baud-preset",
                    tr("common.more"),
                    cx.listener(|this, _, _, cx| {
                        this.cycle_connection_editor_baud_preset(cx);
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
                        .child(format!("{} · {}", tr("dialog.dataBits"), editor.data_bits)),
                )
                .child(small_button(
                    palette,
                    "connection-editor-data-bits",
                    tr("common.more"),
                    cx.listener(|this, _, _, cx| {
                        this.cycle_connection_editor_data_bits(cx);
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
                        .child(format!("{} · {parity_value}", tr("dialog.parity"))),
                )
                .child(small_button(
                    palette,
                    "connection-editor-parity",
                    tr("common.more"),
                    cx.listener(|this, _, _, cx| {
                        this.cycle_connection_editor_parity(cx);
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
                        .child(format!("{} · {}", tr("dialog.stopBits"), editor.stop_bits)),
                )
                .child(small_button(
                    palette,
                    "connection-editor-stop-bits",
                    tr("common.more"),
                    cx.listener(|this, _, _, cx| {
                        this.cycle_connection_editor_stop_bits(cx);
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
                            "{} · {backspace_value}",
                            tr("dialog.backspaceMode")
                        )),
                )
                .child(small_button(
                    palette,
                    "connection-editor-serial-backspace",
                    tr("common.more"),
                    cx.listener(|this, _, _, cx| {
                        this.cycle_connection_editor_backspace(cx);
                    }),
                )),
        )
}
