use super::*;

fn telnet_segment_tab(
    palette: crate::theme::ThemePalette,
    id: &'static str,
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .h(px(28.))
        .min_w_0()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(if selected {
            rgba((palette.primary << 8) | 0x66)
        } else {
            rgba(0x00000000)
        })
        .bg(if selected {
            rgba((palette.primary << 8) | 0x18)
        } else {
            rgba(0x00000000)
        })
        .text_xs()
        .font_weight(FontWeight(600.))
        .text_color(if selected {
            rgb(palette.primary)
        } else {
            rgb(palette.text_muted)
        })
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
        .child(label)
        .on_click(on_click)
}

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
            div()
                .id(id)
                .mt(px(2.))
                .w(px(36.))
                .h(px(20.))
                .flex_none()
                .flex()
                .items_center()
                .px(px(2.))
                .rounded_full()
                .bg(if checked {
                    rgb(palette.primary)
                } else {
                    rgb(palette.border)
                })
                .when(enabled, |this| {
                    this.cursor_pointer()
                        .hover(|this| this.opacity(0.85))
                        .on_click(on_click)
                })
                .child(
                    div()
                        .size(px(16.))
                        .rounded_full()
                        .bg(rgb(palette.on_primary))
                        .when(checked, |this| this.ml_auto()),
                ),
        )
}

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
    let enter_value = match editor.telnet_enter_mode.as_str() {
        "crlf" => "CRLF (\\r\\n)",
        "lf" => "LF (\\n)",
        _ => "CR (\\r)",
    };
    let enter_options = [
        ("crlf", "CRLF (\\r\\n)"),
        ("cr", "CR (\\r)"),
        ("lf", "LF (\\n)"),
    ]
    .into_iter()
    .map(|(value, label)| ConnectionEditorChoice {
        value: Some(value.to_string()),
        label: label.to_string(),
        selected: editor.telnet_enter_mode == value,
    })
    .collect::<Vec<_>>();

    let tabs = div()
        .h(px(32.))
        .p_1()
        .flex()
        .gap_1()
        .rounded_md()
        .bg(rgb(palette.surface_elevated))
        .child(telnet_segment_tab(
            palette,
            "connection-telnet-input-tab",
            tr("dialog.telnetInputSettings"),
            editor.telnet_advanced_tab == ConnectionEditorTelnetTab::Input,
            cx.listener(|this, _, _, cx| {
                this.set_connection_editor_telnet_tab(ConnectionEditorTelnetTab::Input, cx);
            }),
        ))
        .child(telnet_segment_tab(
            palette,
            "connection-telnet-compatibility-tab",
            tr("dialog.telnetCompatibility"),
            editor.telnet_advanced_tab == ConnectionEditorTelnetTab::Compatibility,
            cx.listener(|this, _, _, cx| {
                this.set_connection_editor_telnet_tab(ConnectionEditorTelnetTab::Compatibility, cx);
            }),
        ));

    div()
        .flex()
        .flex_col()
        .gap_3()
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
                .child(svg().size(px(14.)).path(if editor.advanced_open {
                    "icons/chevron-down.svg"
                } else {
                    "icons/fe/forward.svg"
                }))
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
                                            palette,
                                            "connection-editor-telnet-backspace",
                                            tr("dialog.backspaceMode"),
                                            backspace_value,
                                            ConnectionEditorMenu::Backspace,
                                            open_menu == Some(ConnectionEditorMenu::Backspace),
                                            backspace_options,
                                            cx,
                                        ))
                                        .child(connection_editor_select(
                                            palette,
                                            "connection-editor-telnet-enter-mode",
                                            tr("dialog.telnetEnterMode"),
                                            enter_value,
                                            ConnectionEditorMenu::TelnetEnterMode,
                                            open_menu
                                                == Some(ConnectionEditorMenu::TelnetEnterMode),
                                            enter_options,
                                            cx,
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
