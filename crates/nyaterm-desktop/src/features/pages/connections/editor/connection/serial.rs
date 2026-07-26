use gpui::{
    Context, div,
    prelude::{ParentElement, Styled},
    px,
};

use crate::features::NyaTermApp;
use crate::models::{ConnectionEditorField, ConnectionEditorMenu, ConnectionEditorState};

use super::super::super::list::{
    ConnectionEditorChoice, ConnectionEditorFields, connection_editor_select, editor_field,
};

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
    fields: &ConnectionEditorFields,
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
        .gap_3()
        .child(
            div()
                .flex()
                .items_end()
                .gap_3()
                .child(div().min_w_0().flex_1().child(connection_editor_select(
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
                            palette,
                            "connection-editor-baud-preset",
                            "",
                            tr("dialog.customBaudRate"),
                            ConnectionEditorMenu::BaudRate,
                            open_menu == Some(ConnectionEditorMenu::BaudRate),
                            baud_options,
                            cx,
                        ))),
                ),
        )
        .child(
            div()
                .flex()
                .items_end()
                .gap_3()
                .child(div().w(px(72.)).flex_none().child(connection_editor_select(
                    palette,
                    "connection-editor-data-bits",
                    tr("dialog.dataBits"),
                    editor.data_bits.clone(),
                    ConnectionEditorMenu::DataBits,
                    open_menu == Some(ConnectionEditorMenu::DataBits),
                    data_bits_options,
                    cx,
                )))
                .child(
                    div()
                        .min_w(px(112.))
                        .flex_1()
                        .child(connection_editor_select(
                            palette,
                            "connection-editor-parity",
                            tr("dialog.parity"),
                            parity_value,
                            ConnectionEditorMenu::Parity,
                            open_menu == Some(ConnectionEditorMenu::Parity),
                            parity_options,
                            cx,
                        )),
                )
                .child(div().w(px(72.)).flex_none().child(connection_editor_select(
                    palette,
                    "connection-editor-stop-bits",
                    tr("dialog.stopBits"),
                    editor.stop_bits.clone(),
                    ConnectionEditorMenu::StopBits,
                    open_menu == Some(ConnectionEditorMenu::StopBits),
                    stop_bits_options,
                    cx,
                )))
                .child(
                    div()
                        .w(px(144.))
                        .flex_none()
                        .child(connection_editor_select(
                            palette,
                            "connection-editor-serial-backspace",
                            tr("dialog.backspaceMode"),
                            backspace_value,
                            ConnectionEditorMenu::Backspace,
                            open_menu == Some(ConnectionEditorMenu::Backspace),
                            backspace_options,
                            cx,
                        )),
                ),
        )
}
