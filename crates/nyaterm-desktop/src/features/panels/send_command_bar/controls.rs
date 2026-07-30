use super::state::SendCommandBarViewState;
use gpui::{
    Context, FontWeight, IntoElement, KeyDownEvent, SharedString, div, prelude::*, px, rgb,
};
use nyaterm_transport::SessionKind;
use nyaterm_ui::NyaSelectOption;

use super::super::{send_command_control_group, send_command_stepper_button};
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
        let data_options = vec![
            NyaSelectOption::new("text", self.tr("serialSend.text")),
            NyaSelectOption::new("hex", self.tr("serialSend.hex")),
        ];
        let selected_data = match state.send.data_type {
            SendCommandDataType::Text => "text",
            SendCommandDataType::Hex => "hex",
        }
        .to_string();
        let (mode_options, selected_mode) = if state.send.data_type == SendCommandDataType::Hex {
            (
                vec![
                    NyaSelectOption::new("byte", self.tr("serialSend.byteByByte")),
                    NyaSelectOption::new("packet", self.tr("serialSend.packet")),
                ],
                match state.send.mode {
                    SendCommandMode::Packet => "packet",
                    _ => "byte",
                },
            )
        } else {
            (
                vec![
                    NyaSelectOption::new("line", self.tr("serialSend.lineByLine")),
                    NyaSelectOption::new("character", self.tr("serialSend.characterByCharacter")),
                ],
                match state.send.mode {
                    SendCommandMode::Character => "character",
                    _ => "line",
                },
            )
        };
        let mut target_options = vec![NyaSelectOption::new(
            "current",
            self.tr("serialSend.currentSession"),
        )];
        if !is_serial {
            target_options.push(NyaSelectOption::new(
                "all",
                self.tr("serialSend.allSessions"),
            ));
        }
        target_options.extend(state.group_targets.iter().map(|(group_id, name, count)| {
            NyaSelectOption::new(
                format!("group:{group_id}"),
                self.tr("serialSend.groupSession")
                    .replace("{{name}}", name)
                    .replace("{{count}}", &count.to_string()),
            )
        }));
        let selected_target = match &state.send.target {
            SendCommandTarget::Current => "current".to_string(),
            SendCommandTarget::AllCompatible if !is_serial => "all".to_string(),
            SendCommandTarget::AllCompatible => "current".to_string(),
            SendCommandTarget::Group(group_id) => format!("group:{group_id}"),
        };
        if !target_options
            .iter()
            .any(|option| option.value() == selected_target)
        {
            target_options.push(NyaSelectOption::new(
                selected_target.clone(),
                self.tr("network.group"),
            ));
        }
        let line_ending_options = vec![
            NyaSelectOption::new("none", self.tr("serialSend.noLineEnding")),
            NyaSelectOption::new("cr", "CR"),
            NyaSelectOption::new("lf", "LF"),
            NyaSelectOption::new("crlf", "CR+LF"),
        ];
        let selected_line_ending = match state.send.line_ending {
            SendCommandLineEnding::None => "none",
            SendCommandLineEnding::Cr => "cr",
            SendCommandLineEnding::Lf => "lf",
            SendCommandLineEnding::Crlf => "crlf",
        }
        .to_string();

        div()
            .flex_none()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .child(send_command_control_group(
                palette,
                self.tr("serialSend.dataType"),
                self.select_control(
                    "bottom-command-data-select",
                    data_options,
                    Some(selected_data),
                    is_sending,
                    cx,
                ),
            ))
            .child(send_command_control_group(
                palette,
                self.tr("serialSend.sendMode"),
                self.select_control(
                    "bottom-command-mode-select",
                    mode_options,
                    Some(selected_mode.to_string()),
                    is_sending,
                    cx,
                ),
            ))
            .child(send_command_control_group(
                palette,
                self.tr("serialSend.target"),
                self.select_control(
                    "bottom-command-target-select",
                    target_options,
                    Some(selected_target),
                    is_sending,
                    cx,
                ),
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
                    self.select_control(
                        "bottom-command-eol-select",
                        line_ending_options,
                        Some(selected_line_ending),
                        is_sending,
                        cx,
                    ),
                ))
            })
            .into_any_element()
    }
}
