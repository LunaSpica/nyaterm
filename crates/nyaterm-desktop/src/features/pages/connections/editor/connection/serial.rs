use gpui::{
    Context, div,
    prelude::{ParentElement, Styled},
    px,
};

use crate::features::NyaTermApp;
use crate::models::{ConnectionEditorField, ConnectionEditorSelect};

use super::super::super::list::{
    ConnectionEditorRenderContext, connection_editor_select, editor_field, required,
};

use super::ConnectionEditorSectionContext;

pub(super) fn connection_editor_serial_section(
    section: ConnectionEditorSectionContext<'_>,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let ConnectionEditorSectionContext {
        palette,
        editor: _,
        language,
        fields,
    } = section;
    let tr = |key: &'static str| crate::i18n::text(language, key);
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
                    ConnectionEditorSelect::SerialPort,
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
                            ConnectionEditorSelect::BaudRate,
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
                    ConnectionEditorSelect::DataBits,
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
                            ConnectionEditorSelect::Parity,
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
                    ConnectionEditorSelect::StopBits,
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
                            ConnectionEditorSelect::Backspace,
                        )),
                ),
        )
        .child(connection_editor_select(
            ConnectionEditorRenderContext {
                palette,
                fields,
                cx,
            },
            "connection-editor-serial-encoding",
            tr("connection.encoding"),
            ConnectionEditorSelect::Encoding,
        ))
}
