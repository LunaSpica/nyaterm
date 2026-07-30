use gpui::{
    Context, FontWeight, IntoElement, div,
    prelude::{
        FluentBuilder, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled,
    },
    px, rgb, rgba, svg,
};

use crate::features::{ConnectionEditorToggle, NyaTermApp};
use crate::models::{ConnectionEditorField, ConnectionEditorSelect, ConnectionEditorTelnetTab};
use nyaterm_ui::{NyaSwitch, NyaTabItem, NyaTabs};

use super::super::super::list::{
    ConnectionEditorRenderContext, connection_editor_select, editor_field, editor_stepper_field,
    required,
};

use super::ConnectionEditorSectionContext;

fn telnet_switch_row(
    palette: crate::theme::ThemePalette,
    id: &'static str,
    label: &'static str,
    description: &'static str,
    checked: bool,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.bg))
        .px_3()
        .py_2()
        .opacity(if enabled { 1.0 } else { 0.55 })
        .flex()
        .items_start()
        .justify_between()
        .gap_3()
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
                        .font_weight(FontWeight(500.))
                        .text_color(rgb(palette.text))
                        .child(label),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette.text_muted))
                        .child(description),
                ),
        )
        .child(
            div().mt(px(2.)).flex_none().child(
                NyaSwitch::new(id)
                    .checked(checked)
                    .disabled(!enabled)
                    .on_click(move |_, window, cx| {
                        if enabled {
                            on_click(&gpui::ClickEvent::default(), window, cx);
                        }
                    }),
            ),
        )
}

pub(super) fn connection_editor_telnet_section(
    section: ConnectionEditorSectionContext<'_>,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let ConnectionEditorSectionContext {
        palette,
        editor,
        language,
        fields,
    } = section;
    let tr = |key: &'static str| crate::i18n::text(language, key);
    let tabs = NyaTabs::new("connection-telnet-tabs")
        .items([
            NyaTabItem::new(tr("dialog.telnetInputSettings")),
            NyaTabItem::new(tr("dialog.telnetCompatibility")),
        ])
        .selected_index(
            if editor.telnet_advanced_tab == ConnectionEditorTelnetTab::Input {
                0
            } else {
                1
            },
        )
        .on_select(cx.listener(|this, index, _, cx| {
            let tab = match *index {
                0 => ConnectionEditorTelnetTab::Input,
                _ => ConnectionEditorTelnetTab::Compatibility,
            };
            this.set_connection_editor_telnet_tab(tab, cx);
        }));

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .gap_3()
                .child(div().min_w_0().flex_1().child(editor_field(
                    palette,
                    required(tr("dialog.host")),
                    ConnectionEditorField::Host,
                    fields,
                    cx,
                )))
                .child(div().w(px(150.)).flex_none().child(editor_stepper_field(
                    palette,
                    required(tr("dialog.port")),
                    ConnectionEditorField::Port,
                    fields,
                    cx,
                ))),
        )
        .child(
            div()
                .id("connection-telnet-advanced-toggle")
                .h(px(28.))
                .flex()
                .items_center()
                .gap_1()
                .cursor_pointer()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .hover(|this| this.text_color(rgb(palette.text)))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_connection_editor_flag(ConnectionEditorToggle::Advanced, cx);
                }))
                .child(
                    svg()
                        .size(px(14.))
                        .path(if editor.advanced_open {
                            "icons/chevron-down.svg"
                        } else {
                            "icons/fe/forward.svg"
                        })
                        .text_color(rgb(palette.text_muted)),
                )
                .child(tr("dialog.advancedConfig")),
        )
        .when(editor.advanced_open, |this| {
            this.child(tabs)
                .when(
                    editor.telnet_advanced_tab == ConnectionEditorTelnetTab::Input,
                    |this| {
                        this.child(
                            div()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .bg(rgba((palette.accent << 8) | 0x18))
                                .p_3()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight(600.))
                                        .child(tr("dialog.telnetInputBehavior")),
                                )
                                .child(
                                    div()
                                        .grid()
                                        .grid_cols(2)
                                        .gap_2()
                                        .child(connection_editor_select(
                                            ConnectionEditorRenderContext {
                                                palette,
                                                fields,
                                                cx,
                                            },
                                            "connection-editor-telnet-backspace",
                                            tr("dialog.backspaceMode"),
                                            ConnectionEditorSelect::Backspace,
                                        ))
                                        .child(connection_editor_select(
                                            ConnectionEditorRenderContext {
                                                palette,
                                                fields,
                                                cx,
                                            },
                                            "connection-editor-telnet-enter-mode",
                                            tr("dialog.telnetEnterMode"),
                                            ConnectionEditorSelect::TelnetEnterMode,
                                        )),
                                ),
                        )
                    },
                )
                .when(
                    editor.telnet_advanced_tab == ConnectionEditorTelnetTab::Compatibility,
                    |this| {
                        this.child(
                            div()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .bg(rgba((palette.accent << 8) | 0x18))
                                .p_3()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(rgb(palette.text_muted))
                                        .child(tr("dialog.telnetRawTcpCliDesc")),
                                )
                                .child(telnet_switch_row(
                                    palette,
                                    "connection-telnet-raw-tcp",
                                    tr("dialog.telnetRawTcpCli"),
                                    tr("dialog.telnetRawTcpCliLongDesc"),
                                    editor.raw_tcp_cli,
                                    true,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_connection_editor_flag(
                                            ConnectionEditorToggle::RawTcp,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(
                                    div()
                                        .grid()
                                        .grid_cols(2)
                                        .gap_2()
                                        .child(telnet_switch_row(
                                            palette,
                                            "connection-telnet-local-echo",
                                            tr("dialog.telnetLocalEcho"),
                                            tr("dialog.telnetLocalEchoDesc"),
                                            editor.local_echo,
                                            true,
                                            cx.listener(|this, _, _, cx| {
                                                this.toggle_connection_editor_flag(
                                                    ConnectionEditorToggle::LocalEcho,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(telnet_switch_row(
                                            palette,
                                            "connection-telnet-local-line-edit",
                                            tr("dialog.telnetLocalLineEdit"),
                                            tr("dialog.telnetLocalLineEditDesc"),
                                            editor.local_line_edit,
                                            true,
                                            cx.listener(|this, _, _, cx| {
                                                this.toggle_connection_editor_flag(
                                                    ConnectionEditorToggle::LocalLineEdit,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(telnet_switch_row(
                                            palette,
                                            "connection-telnet-force-character",
                                            tr("dialog.telnetForceCharAtATime"),
                                            tr("dialog.telnetForceCharAtATimeDesc"),
                                            editor.force_character_at_a_time,
                                            true,
                                            cx.listener(|this, _, _, cx| {
                                                this.toggle_connection_editor_flag(
                                                    ConnectionEditorToggle::ForceCharacterAtATime,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(telnet_switch_row(
                                            palette,
                                            "connection-telnet-send-naws",
                                            tr("dialog.telnetSendNaws"),
                                            tr("dialog.telnetSendNawsDesc"),
                                            editor.send_naws,
                                            !editor.raw_tcp_cli,
                                            cx.listener(|this, _, _, cx| {
                                                this.toggle_connection_editor_flag(
                                                    ConnectionEditorToggle::SendNaws,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(telnet_switch_row(
                                            palette,
                                            "connection-telnet-send-sga",
                                            tr("dialog.telnetSendSga"),
                                            tr("dialog.telnetSendSgaDesc"),
                                            editor.send_sga,
                                            !editor.raw_tcp_cli,
                                            cx.listener(|this, _, _, cx| {
                                                this.toggle_connection_editor_flag(
                                                    ConnectionEditorToggle::SendSga,
                                                    cx,
                                                );
                                            }),
                                        )),
                                ),
                        )
                    },
                )
        })
}
