use super::state::SendCommandBarViewState;
use super::*;

impl NyaTermApp {
    pub(super) fn send_command_bar_controls(
        &mut self,
        state: &SendCommandBarViewState,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let palette = state.palette;
        let is_sending = state.is_sending;
        let is_serial = matches!(self.active_session_kind(), Some(SessionKind::Serial));
        let count_focused = self.send_command_control_focus == Some(SendCommandControlFocus::Count);
        let interval_focused =
            self.send_command_control_focus == Some(SendCommandControlFocus::Interval);
        let count_label = if count_focused {
            self.send_command_count_input.clone()
        } else {
            state.count_label.clone()
        };
        let interval_label = if interval_focused {
            self.send_command_interval_input.clone()
        } else {
            state.interval_label.clone()
        };
        let data_label = match self.send_command_data_type {
            SendCommandDataType::Text => "Text",
            SendCommandDataType::Hex => "Hex",
        };
        let mode_label =
            send_command_mode_label(self.send_command_data_type, self.send_command_mode);
        let target_label = self.send_command_target_label(&state.group_targets);
        let line_ending_label = state.line_ending_label;

        div()
            .flex_none()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .child(send_command_control_group(
                palette,
                "Data Type",
                send_command_select_trigger(
                    palette,
                    "bottom-command-data-select",
                    data_label,
                    self.send_command_data_menu_open,
                    is_sending,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_send_command_data_menu(cx);
                    }),
                )
                .when(self.send_command_data_menu_open, |this| {
                    this.child(
                        send_command_select_menu(palette, "bottom-command-data-menu")
                            .child(send_command_select_menu_item(
                                palette,
                                "bottom-command-data-text",
                                "Text",
                                self.send_command_data_type == SendCommandDataType::Text,
                                cx.listener(|this, _, _, cx| {
                                    this.set_send_command_data_type(SendCommandDataType::Text, cx);
                                }),
                            ))
                            .child(send_command_select_menu_item(
                                palette,
                                "bottom-command-data-hex",
                                "Hex",
                                self.send_command_data_type == SendCommandDataType::Hex,
                                cx.listener(|this, _, _, cx| {
                                    this.set_send_command_data_type(SendCommandDataType::Hex, cx);
                                }),
                            )),
                    )
                }),
            ))
            .child(send_command_control_group(
                palette,
                "Send Mode",
                send_command_select_trigger(
                    palette,
                    "bottom-command-mode-select",
                    mode_label,
                    self.send_command_mode_menu_open,
                    is_sending,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_send_command_mode_menu(cx);
                    }),
                )
                .when(self.send_command_mode_menu_open, |this| {
                    let menu = if self.send_command_data_type == SendCommandDataType::Hex {
                        send_command_select_menu(palette, "bottom-command-mode-menu")
                            .child(send_command_select_menu_item(
                                palette,
                                "bottom-command-mode-byte",
                                "Byte by byte",
                                self.send_command_mode == SendCommandMode::Byte,
                                cx.listener(|this, _, _, cx| {
                                    this.set_send_command_mode(SendCommandMode::Byte, cx);
                                }),
                            ))
                            .child(send_command_select_menu_item(
                                palette,
                                "bottom-command-mode-packet",
                                "Packet",
                                self.send_command_mode == SendCommandMode::Packet,
                                cx.listener(|this, _, _, cx| {
                                    this.set_send_command_mode(SendCommandMode::Packet, cx);
                                }),
                            ))
                    } else {
                        send_command_select_menu(palette, "bottom-command-mode-menu")
                            .child(send_command_select_menu_item(
                                palette,
                                "bottom-command-mode-line",
                                "Line by line",
                                self.send_command_mode == SendCommandMode::Line,
                                cx.listener(|this, _, _, cx| {
                                    this.set_send_command_mode(SendCommandMode::Line, cx);
                                }),
                            ))
                            .child(send_command_select_menu_item(
                                palette,
                                "bottom-command-mode-character",
                                "Character by character",
                                self.send_command_mode == SendCommandMode::Character,
                                cx.listener(|this, _, _, cx| {
                                    this.set_send_command_mode(SendCommandMode::Character, cx);
                                }),
                            ))
                    };
                    this.child(menu)
                }),
            ))
            .child(send_command_control_group(
                palette,
                "Target",
                send_command_select_trigger(
                    palette,
                    "bottom-command-target-select",
                    target_label,
                    self.send_command_target_menu_open,
                    is_sending,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_send_command_target_menu(cx);
                    }),
                )
                .when(self.send_command_target_menu_open, |this| {
                    let mut menu = send_command_select_menu(palette, "bottom-command-target-menu")
                        .child(send_command_select_menu_item(
                            palette,
                            "bottom-command-target-current",
                            "Current session",
                            matches!(self.send_command_target, SendCommandTarget::Current),
                            cx.listener(|this, _, _, cx| {
                                this.set_send_command_target(SendCommandTarget::Current, cx);
                            }),
                        ));
                    if !is_serial {
                        menu = menu.child(send_command_select_menu_item(
                            palette,
                            "bottom-command-target-all",
                            "All sessions",
                            matches!(self.send_command_target, SendCommandTarget::AllCompatible),
                            cx.listener(|this, _, _, cx| {
                                this.set_send_command_target(SendCommandTarget::AllCompatible, cx);
                            }),
                        ));
                    }
                    for (group_id, group_name, count) in state.group_targets.iter() {
                        let selected = matches!(
                            &self.send_command_target,
                            SendCommandTarget::Group(id) if id == group_id
                        );
                        let label = format!("Group: {group_name} ({count})");
                        let id = group_id.clone();
                        menu = menu.child(send_command_select_menu_item(
                            palette,
                            format!("bottom-command-target-group-{group_id}"),
                            label,
                            selected,
                            cx.listener(move |this, _, _, cx| {
                                this.set_send_command_target(
                                    SendCommandTarget::Group(id.clone()),
                                    cx,
                                );
                            }),
                        ));
                    }
                    this.child(menu)
                }),
            ))
            .child(send_command_control_group(
                palette,
                "Count",
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .child(send_command_stepper_button(
                        palette,
                        "bottom-command-count-down",
                        "-",
                        is_sending,
                        cx.listener(|this, _, _, cx| {
                            this.adjust_send_command_count(-1, cx);
                        }),
                    ))
                    .child(
                        div()
                            .id(SharedString::from("bottom-command-count-input"))
                            .min_w(px(44.))
                            .h_full()
                            .px_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_l_1()
                            .border_r_1()
                            .border_color(rgb(if count_focused {
                                palette.link
                            } else {
                                palette.border
                            }))
                            .bg(if count_focused {
                                rgb(palette.input)
                            } else {
                                rgb(0x00000000)
                            })
                            .font_family(crate::features::gpui_code_font_family())
                            .text_size(px(12.))
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(if is_sending {
                                palette.text_dimmed
                            } else {
                                palette.text
                            }))
                            .when(!is_sending, |this| this.cursor_pointer())
                            .child(count_label)
                            .track_focus(&self.send_command_controls_focus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.focus_send_command_control(
                                    SendCommandControlFocus::Count,
                                    window,
                                    cx,
                                );
                            }))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.handle_send_command_control_key_down(event, cx);
                            })),
                    )
                    .child(send_command_stepper_button(
                        palette,
                        "bottom-command-count-up",
                        "+",
                        is_sending,
                        cx.listener(|this, _, _, cx| {
                            this.adjust_send_command_count(1, cx);
                        }),
                    )),
            ))
            .child(send_command_control_group(
                palette,
                "Interval",
                div().h_full().flex().items_center().child(
                    div()
                        .id(SharedString::from("bottom-command-interval-input"))
                        .min_w(px(58.))
                        .h_full()
                        .px_2()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap_1()
                        .bg(if interval_focused {
                            rgb(palette.input)
                        } else {
                            rgb(0x00000000)
                        })
                        .font_family(crate::features::gpui_code_font_family())
                        .text_size(px(12.))
                        .font_weight(FontWeight(600.))
                        .text_color(rgb(if is_sending {
                            palette.text_dimmed
                        } else {
                            palette.text
                        }))
                        .when(!is_sending, |this| this.cursor_pointer())
                        .child(interval_label)
                        .child(
                            div()
                                .text_size(px(10.))
                                .font_weight(FontWeight(500.))
                                .text_color(rgb(palette.text_dimmed))
                                .child("s"),
                        )
                        .track_focus(&self.send_command_controls_focus)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.focus_send_command_control(
                                SendCommandControlFocus::Interval,
                                window,
                                cx,
                            );
                        }))
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                            cx.stop_propagation();
                            this.handle_send_command_control_key_down(event, cx);
                        })),
                ),
            ))
            .when(state.is_serial_text_line, |this| {
                this.child(send_command_control_group(
                    palette,
                    "Line Ending",
                    send_command_select_trigger(
                        palette,
                        "bottom-command-eol-select",
                        line_ending_label,
                        self.send_command_line_ending_menu_open,
                        is_sending,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_send_command_line_ending_menu(cx);
                        }),
                    )
                    .when(self.send_command_line_ending_menu_open, |this| {
                        this.child(
                            send_command_select_menu(palette, "bottom-command-eol-menu")
                                .child(send_command_line_ending_item(
                                    palette,
                                    "none",
                                    "None",
                                    SendCommandLineEnding::None,
                                    self.send_command_line_ending,
                                    cx,
                                ))
                                .child(send_command_line_ending_item(
                                    palette,
                                    "cr",
                                    "CR",
                                    SendCommandLineEnding::Cr,
                                    self.send_command_line_ending,
                                    cx,
                                ))
                                .child(send_command_line_ending_item(
                                    palette,
                                    "lf",
                                    "LF",
                                    SendCommandLineEnding::Lf,
                                    self.send_command_line_ending,
                                    cx,
                                ))
                                .child(send_command_line_ending_item(
                                    palette,
                                    "crlf",
                                    "CR+LF",
                                    SendCommandLineEnding::Crlf,
                                    self.send_command_line_ending,
                                    cx,
                                )),
                        )
                    }),
                ))
            })
            .into_any_element()
    }

    fn send_command_target_label(&self, group_targets: &[(String, String, usize)]) -> String {
        match &self.send_command_target {
            SendCommandTarget::Current => "Current session".to_string(),
            SendCommandTarget::AllCompatible => "All sessions".to_string(),
            SendCommandTarget::Group(group_id) => group_targets
                .iter()
                .find(|(id, _, _)| id == group_id)
                .map(|(_, name, count)| format!("Group: {name} ({count})"))
                .unwrap_or_else(|| "Group".to_string()),
        }
    }
}

fn send_command_mode_label(data_type: SendCommandDataType, mode: SendCommandMode) -> &'static str {
    match (data_type, mode) {
        (SendCommandDataType::Hex, SendCommandMode::Byte) => "Byte by byte",
        (SendCommandDataType::Hex, SendCommandMode::Packet) => "Packet",
        (SendCommandDataType::Text, SendCommandMode::Character) => "Character by character",
        _ => "Line by line",
    }
}

fn send_command_line_ending_item(
    palette: crate::theme::ThemePalette,
    id: &'static str,
    label: &'static str,
    value: SendCommandLineEnding,
    current: SendCommandLineEnding,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    send_command_select_menu_item(
        palette,
        format!("bottom-command-eol-{id}"),
        label,
        value == current,
        cx.listener(move |this, _, _, cx| {
            this.set_send_command_line_ending(value, cx);
        }),
    )
}
