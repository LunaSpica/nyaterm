use super::state::SendCommandBarViewState;
use gpui::{
    Context, FontWeight, IntoElement, KeyDownEvent, SharedString, div, prelude::*, px, rgb,
};
use nyaterm_transport::SessionKind;

use super::super::{
    send_command_control_group, send_command_select_menu, send_command_select_menu_item,
    send_command_select_trigger, send_command_stepper_button,
};
use crate::features::{NyaTermApp, TextInputSetup};
use crate::send_command::{
    SendCommandControlFocus, SendCommandDataType, SendCommandLineEnding, SendCommandMode,
    SendCommandTarget,
};

impl NyaTermApp {
    pub(super) fn send_command_bar_controls(
        &mut self,
        state: &SendCommandBarViewState,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let palette = state.palette;
        let is_sending = state.is_sending;
        let is_serial = matches!(self.active_session_kind(), Some(SessionKind::Serial));
        let count_input = self.text_input(
            "send-command.count",
            &state.send.count_input,
            TextInputSetup::default(),
            cx,
        );
        let count_focused = count_input.read(cx).has_focus();
        let interval_input = self.text_input(
            "send-command.interval",
            &state.send.interval_input,
            TextInputSetup::default(),
            cx,
        );
        let interval_focused = interval_input.read(cx).has_focus();
        let data_label = match state.send.data_type {
            SendCommandDataType::Text => self.tr("serialSend.text"),
            SendCommandDataType::Hex => self.tr("serialSend.hex"),
        };
        let mode_label = match (state.send.data_type, state.send.mode) {
            (SendCommandDataType::Hex, SendCommandMode::Byte) => self.tr("serialSend.byteByByte"),
            (SendCommandDataType::Hex, SendCommandMode::Packet) => self.tr("serialSend.packet"),
            (SendCommandDataType::Text, SendCommandMode::Character) => {
                self.tr("serialSend.characterByCharacter")
            }
            _ => self.tr("serialSend.lineByLine"),
        };
        let target_label = self.send_command_target_label(&state.send.target, &state.group_targets);
        let line_ending_label = state.line_ending_label;

        div()
            .flex_none()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .child(send_command_control_group(
                palette,
                self.tr("serialSend.dataType"),
                send_command_select_trigger(
                    palette,
                    "bottom-command-data-select",
                    data_label,
                    state.send.data_menu_open,
                    is_sending,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_send_command_data_menu(cx);
                    }),
                )
                .when(state.send.data_menu_open, |this| {
                    this.child(
                        send_command_select_menu(
                            palette,
                            self.shell_surface_color(palette.surface),
                            "bottom-command-data-menu",
                        )
                        .child(send_command_select_menu_item(
                            palette,
                            "bottom-command-data-text",
                            self.tr("serialSend.text"),
                            state.send.data_type == SendCommandDataType::Text,
                            cx.listener(|this, _, _, cx| {
                                this.set_send_command_data_type(SendCommandDataType::Text, cx);
                            }),
                        ))
                        .child(send_command_select_menu_item(
                            palette,
                            "bottom-command-data-hex",
                            self.tr("serialSend.hex"),
                            state.send.data_type == SendCommandDataType::Hex,
                            cx.listener(|this, _, _, cx| {
                                this.set_send_command_data_type(SendCommandDataType::Hex, cx);
                            }),
                        )),
                    )
                }),
            ))
            .child(send_command_control_group(
                palette,
                self.tr("serialSend.sendMode"),
                send_command_select_trigger(
                    palette,
                    "bottom-command-mode-select",
                    mode_label,
                    state.send.mode_menu_open,
                    is_sending,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_send_command_mode_menu(cx);
                    }),
                )
                .when(state.send.mode_menu_open, |this| {
                    let menu = if state.send.data_type == SendCommandDataType::Hex {
                        send_command_select_menu(
                            palette,
                            self.shell_surface_color(palette.surface),
                            "bottom-command-mode-menu",
                        )
                        .child(send_command_select_menu_item(
                            palette,
                            "bottom-command-mode-byte",
                            self.tr("serialSend.byteByByte"),
                            state.send.mode == SendCommandMode::Byte,
                            cx.listener(|this, _, _, cx| {
                                this.set_send_command_mode(SendCommandMode::Byte, cx);
                            }),
                        ))
                        .child(send_command_select_menu_item(
                            palette,
                            "bottom-command-mode-packet",
                            self.tr("serialSend.packet"),
                            state.send.mode == SendCommandMode::Packet,
                            cx.listener(|this, _, _, cx| {
                                this.set_send_command_mode(SendCommandMode::Packet, cx);
                            }),
                        ))
                    } else {
                        send_command_select_menu(
                            palette,
                            self.shell_surface_color(palette.surface),
                            "bottom-command-mode-menu",
                        )
                        .child(send_command_select_menu_item(
                            palette,
                            "bottom-command-mode-line",
                            self.tr("serialSend.lineByLine"),
                            state.send.mode == SendCommandMode::Line,
                            cx.listener(|this, _, _, cx| {
                                this.set_send_command_mode(SendCommandMode::Line, cx);
                            }),
                        ))
                        .child(send_command_select_menu_item(
                            palette,
                            "bottom-command-mode-character",
                            self.tr("serialSend.characterByCharacter"),
                            state.send.mode == SendCommandMode::Character,
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
                self.tr("serialSend.target"),
                send_command_select_trigger(
                    palette,
                    "bottom-command-target-select",
                    target_label,
                    state.send.target_menu_open,
                    is_sending,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_send_command_target_menu(cx);
                    }),
                )
                .when(state.send.target_menu_open, |this| {
                    let mut menu = send_command_select_menu(
                        palette,
                        self.shell_surface_color(palette.surface),
                        "bottom-command-target-menu",
                    )
                    .child(send_command_select_menu_item(
                        palette,
                        "bottom-command-target-current",
                        self.tr("serialSend.currentSession"),
                        matches!(state.send.target, SendCommandTarget::Current),
                        cx.listener(|this, _, _, cx| {
                            this.set_send_command_target(SendCommandTarget::Current, cx);
                        }),
                    ));
                    if !is_serial {
                        menu = menu.child(send_command_select_menu_item(
                            palette,
                            "bottom-command-target-all",
                            self.tr("serialSend.allSessions"),
                            matches!(state.send.target, SendCommandTarget::AllCompatible),
                            cx.listener(|this, _, _, cx| {
                                this.set_send_command_target(SendCommandTarget::AllCompatible, cx);
                            }),
                        ));
                    }
                    for (group_id, group_name, count) in state.group_targets.iter() {
                        let selected = matches!(
                            &state.send.target,
                            SendCommandTarget::Group(id) if id == group_id
                        );
                        let label = self
                            .tr("serialSend.groupSession")
                            .replace("{{name}}", group_name)
                            .replace("{{count}}", &count.to_string());
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
                self.tr("serialSend.count"),
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
                            .when(!is_sending, |this| this.cursor_text())
                            .child(div().min_w(px(36.)).flex_1().child(count_input))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.focus_send_command_control(
                                    SendCommandControlFocus::Count,
                                    window,
                                    cx,
                                );
                            }))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                                cx.stop_propagation();
                                this.handle_send_command_control_key_down(event, window, cx);
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
                self.tr("serialSend.interval"),
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
                        .when(!is_sending, |this| this.cursor_text())
                        .child(div().min_w(px(42.)).flex_1().child(interval_input))
                        .child(
                            div()
                                .text_size(px(10.))
                                .font_weight(FontWeight(500.))
                                .text_color(rgb(palette.text_dimmed))
                                .child(self.tr("serialSend.seconds")),
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.focus_send_command_control(
                                SendCommandControlFocus::Interval,
                                window,
                                cx,
                            );
                        }))
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                            cx.stop_propagation();
                            this.handle_send_command_control_key_down(event, window, cx);
                        })),
                ),
            ))
            .when(state.is_serial_text_line, |this| {
                this.child(send_command_control_group(
                    palette,
                    self.tr("serialSend.lineEnding"),
                    send_command_select_trigger(
                        palette,
                        "bottom-command-eol-select",
                        line_ending_label,
                        state.send.line_ending_menu_open,
                        is_sending,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_send_command_line_ending_menu(cx);
                        }),
                    )
                    .when(state.send.line_ending_menu_open, |this| {
                        this.child(
                            send_command_select_menu(
                                palette,
                                self.shell_surface_color(palette.surface),
                                "bottom-command-eol-menu",
                            )
                            .child(send_command_line_ending_item(
                                palette,
                                "none",
                                self.tr("serialSend.noLineEnding"),
                                SendCommandLineEnding::None,
                                state.send.line_ending,
                                cx,
                            ))
                            .child(send_command_line_ending_item(
                                palette,
                                "cr",
                                "CR",
                                SendCommandLineEnding::Cr,
                                state.send.line_ending,
                                cx,
                            ))
                            .child(send_command_line_ending_item(
                                palette,
                                "lf",
                                "LF",
                                SendCommandLineEnding::Lf,
                                state.send.line_ending,
                                cx,
                            ))
                            .child(send_command_line_ending_item(
                                palette,
                                "crlf",
                                "CR+LF",
                                SendCommandLineEnding::Crlf,
                                state.send.line_ending,
                                cx,
                            )),
                        )
                    }),
                ))
            })
            .into_any_element()
    }

    fn send_command_target_label(
        &self,
        target: &SendCommandTarget,
        group_targets: &[(String, String, usize)],
    ) -> String {
        match target {
            SendCommandTarget::Current => self.tr("serialSend.currentSession").to_string(),
            SendCommandTarget::AllCompatible => self.tr("serialSend.allSessions").to_string(),
            SendCommandTarget::Group(group_id) => group_targets
                .iter()
                .find(|(id, _, _)| id == group_id)
                .map(|(_, name, count)| {
                    self.tr("serialSend.groupSession")
                        .replace("{{name}}", name)
                        .replace("{{count}}", &count.to_string())
                })
                .unwrap_or_else(|| self.tr("network.group").to_string()),
        }
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
