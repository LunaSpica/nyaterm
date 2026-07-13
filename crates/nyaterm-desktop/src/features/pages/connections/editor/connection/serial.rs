use super::*;

pub(super) fn connection_editor_serial_section(
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
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.text_muted))
                        .child(format!(
                            "Port · {}",
                            if editor.serial_port.is_empty() {
                                "Select port"
                            } else {
                                &editor.serial_port
                            }
                        )),
                )
                .child(small_button(
                    palette,
                    "connection-editor-serial-port",
                    "Cycle",
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
                    "Baud",
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
                    "Preset",
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
                        .child(format!("Data bits · {}", editor.data_bits)),
                )
                .child(small_button(
                    palette,
                    "connection-editor-data-bits",
                    "Cycle",
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
                        .child(format!("Parity · {}", editor.parity.to_ascii_uppercase())),
                )
                .child(small_button(
                    palette,
                    "connection-editor-parity",
                    "Cycle",
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
                        .child(format!("Stop bits · {}", editor.stop_bits)),
                )
                .child(small_button(
                    palette,
                    "connection-editor-stop-bits",
                    "Cycle",
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
                            "Backspace · {}",
                            match editor.backspace_mode.as_str() {
                                "ctrl-h" | "bs" | "ctrl_h" => "Ctrl+H (BS)",
                                _ => "DEL (0x7F)",
                            }
                        )),
                )
                .child(small_button(
                    palette,
                    "connection-editor-serial-backspace",
                    "Cycle",
                    cx.listener(|this, _, _, cx| {
                        this.cycle_connection_editor_backspace(cx);
                    }),
                )),
        )
}
