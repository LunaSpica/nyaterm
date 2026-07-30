use gpui::{
    Context, div,
    prelude::{ParentElement, Styled},
    px,
};

use crate::features::NyaTermApp;
use crate::models::{ConnectionEditorField, ConnectionEditorMenu};

use super::super::super::list::{
    ConnectionEditorChoice, ConnectionEditorRenderContext, connection_editor_select, editor_field,
    required,
};

use super::ConnectionEditorSectionContext;

pub(super) struct SerialConnectionSectionOptions {
    pub(super) serial_ports: Vec<ConnectionEditorChoice>,
    pub(super) baud_rates: Vec<ConnectionEditorChoice>,
    pub(super) data_bits: Vec<ConnectionEditorChoice>,
    pub(super) parity: Vec<ConnectionEditorChoice>,
    pub(super) stop_bits: Vec<ConnectionEditorChoice>,
    pub(super) backspace: Vec<ConnectionEditorChoice>,
}

pub(super) fn connection_editor_serial_section(
    section: ConnectionEditorSectionContext<'_>,
    options: SerialConnectionSectionOptions,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let ConnectionEditorSectionContext {
        palette,
        editor,
        active_menu: open_menu,
        language,
        fields,
    } = section;
    let SerialConnectionSectionOptions {
        serial_ports: serial_port_options,
        baud_rates: baud_options,
        data_bits: data_bits_options,
        parity: parity_options,
        stop_bits: stop_bits_options,
        backspace: backspace_options,
    } = options;
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
        .gap_3()
        .child(
            div()
                .flex()
                .items_end()
                .gap_3()
                .child(div().min_w_0().flex_1().child(connection_editor_select(
                    ConnectionEditorRenderContext {
                        palette,
                        fields,
                        cx,
                    },
                    "connection-editor-serial-port",
                    required(tr("dialog.serialPort")),
                    if editor.serial_port.is_empty() {
                        tr("dialog.selectSerialPort").to_string()
                    } else {
                        editor.serial_port.clone()
                    },
                    ConnectionEditorMenu::SerialPort,
                    open_menu == Some(ConnectionEditorMenu::SerialPort),
                    serial_port_options,
                )))
                .child(
                    div()
                        .w(px(216.))
                        .flex_none()
                        .flex()
                        .items_end()
                        .gap_1()
                        .child(div().min_w_0().flex_1().child(editor_field(
                            palette,
                            tr("dialog.baudRate"),
                            ConnectionEditorField::BaudRate,
                            fields,
                            cx,
                        )))
                        .child(div().w(px(72.)).flex_none().child(connection_editor_select(
                            ConnectionEditorRenderContext {
                                palette,
                                fields,
                                cx,
                            },
                            "connection-editor-baud-preset",
                            "",
                            tr("dialog.customBaudRate"),
                            ConnectionEditorMenu::BaudRate,
                            open_menu == Some(ConnectionEditorMenu::BaudRate),
                            baud_options,
                        ))),
                ),
        )
        .child(
            div()
                .flex()
                .items_end()
                .gap_3()
                .child(div().w(px(72.)).flex_none().child(connection_editor_select(
                    ConnectionEditorRenderContext {
                        palette,
                        fields,
                        cx,
                    },
                    "connection-editor-data-bits",
                    tr("dialog.dataBits"),
                    editor.data_bits.clone(),
                    ConnectionEditorMenu::DataBits,
                    open_menu == Some(ConnectionEditorMenu::DataBits),
                    data_bits_options,
                )))
                .child(
                    div()
                        .min_w(px(112.))
                        .flex_1()
                        .child(connection_editor_select(
                            ConnectionEditorRenderContext {
                                palette,
                                fields,
                                cx,
                            },
                            "connection-editor-parity",
                            tr("dialog.parity"),
                            parity_value,
                            ConnectionEditorMenu::Parity,
                            open_menu == Some(ConnectionEditorMenu::Parity),
                            parity_options,
                        )),
                )
                .child(div().w(px(72.)).flex_none().child(connection_editor_select(
                    ConnectionEditorRenderContext {
                        palette,
                        fields,
                        cx,
                    },
                    "connection-editor-stop-bits",
                    tr("dialog.stopBits"),
                    editor.stop_bits.clone(),
                    ConnectionEditorMenu::StopBits,
                    open_menu == Some(ConnectionEditorMenu::StopBits),
                    stop_bits_options,
                )))
                .child(
                    div()
                        .w(px(144.))
                        .flex_none()
                        .child(connection_editor_select(
                            ConnectionEditorRenderContext {
                                palette,
                                fields,
                                cx,
                            },
                            "connection-editor-serial-backspace",
                            tr("dialog.backspaceMode"),
                            backspace_value,
                            ConnectionEditorMenu::Backspace,
                            open_menu == Some(ConnectionEditorMenu::Backspace),
                            backspace_options,
                        )),
                ),
        )
}
