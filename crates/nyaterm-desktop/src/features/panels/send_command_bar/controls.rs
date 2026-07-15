use super::state::SendCommandBarViewState;
use super::*;

impl NyaTermApp {
    pub(super) fn send_command_bar_controls(
        &mut self,
        state: &SendCommandBarViewState,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let palette = state.palette;
        let group_targets = &state.group_targets;
        let count_label = state.count_label.clone();
        let interval_label = state.interval_label.clone();
        let line_ending_label = state.line_ending_label;
        let is_serial_text_line = state.is_serial_text_line;
        div()
            .flex_none()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .child(send_command_control_group(
                palette,
                "Data",
                div()
                    .flex()
                    .items_center()
                    .child(send_command_chip(
                        palette,
                        "bottom-command-send-text",
                        "Text",
                        self.send_command_data_type == SendCommandDataType::Text,
                        cx.listener(|this, _, _, cx| {
                            this.set_send_command_data_type(SendCommandDataType::Text, cx);
                        }),
                    ))
                    .child(send_command_chip(
                        palette,
                        "bottom-command-send-hex",
                        "Hex",
                        self.send_command_data_type == SendCommandDataType::Hex,
                        cx.listener(|this, _, _, cx| {
                            this.set_send_command_data_type(SendCommandDataType::Hex, cx);
                        }),
                    )),
            ))
            .child(send_command_control_group(palette, "Target", {
                let mut chips = div().flex().items_center().flex_wrap().gap_0();
                chips = chips
                    .child(send_command_chip(
                        palette,
                        "bottom-command-target-current",
                        "Current",
                        matches!(self.send_command_target, SendCommandTarget::Current),
                        cx.listener(|this, _, _, cx| {
                            this.set_send_command_target(SendCommandTarget::Current, cx);
                        }),
                    ))
                    .child(send_command_chip(
                        palette,
                        "bottom-command-target-all",
                        "All",
                        matches!(self.send_command_target, SendCommandTarget::AllCompatible),
                        cx.listener(|this, _, _, cx| {
                            this.set_send_command_target(SendCommandTarget::AllCompatible, cx);
                        }),
                    ));
                for (group_id, group_name, count) in group_targets.iter().take(4) {
                    let selected = matches!(
                        &self.send_command_target,
                        SendCommandTarget::Group(id) if id == group_id
                    );
                    let label = if group_name.chars().count() > 10 {
                        format!(
                            "{}…({})",
                            group_name.chars().take(8).collect::<String>(),
                            count
                        )
                    } else {
                        format!("{group_name}({count})")
                    };
                    let group_id = group_id.clone();
                    // send_command_chip needs &'static str for label - use dynamic via local helper
                    chips = chips.child(send_command_target_chip(
                        palette,
                        format!("bottom-command-target-group-{group_id}"),
                        label,
                        selected,
                        cx.listener(move |this, _, _, cx| {
                            this.set_send_command_target(
                                SendCommandTarget::Group(group_id.clone()),
                                cx,
                            );
                        }),
                    ));
                }
                chips
            }))
            .child(send_command_control_group(
                palette,
                "Mode",
                div()
                    .flex()
                    .items_center()
                    .child(send_command_chip(
                        palette,
                        "bottom-command-send-mode-primary",
                        if self.send_command_data_type == SendCommandDataType::Hex {
                            "Byte"
                        } else {
                            "Line"
                        },
                        matches!(
                            self.send_command_mode,
                            SendCommandMode::Line | SendCommandMode::Byte
                        ),
                        cx.listener(|this, _, _, cx| {
                            let mode = if this.send_command_data_type == SendCommandDataType::Hex {
                                SendCommandMode::Byte
                            } else {
                                SendCommandMode::Line
                            };
                            this.set_send_command_mode(mode, cx);
                        }),
                    ))
                    .child(send_command_chip(
                        palette,
                        "bottom-command-send-mode-secondary",
                        if self.send_command_data_type == SendCommandDataType::Hex {
                            "Packet"
                        } else {
                            "Char"
                        },
                        matches!(
                            self.send_command_mode,
                            SendCommandMode::Character | SendCommandMode::Packet
                        ),
                        cx.listener(|this, _, _, cx| {
                            let mode = if this.send_command_data_type == SendCommandDataType::Hex {
                                SendCommandMode::Packet
                            } else {
                                SendCommandMode::Character
                            };
                            this.set_send_command_mode(mode, cx);
                        }),
                    )),
            ))
            .child(send_command_control_group(
                palette,
                "Count",
                div()
                    .flex()
                    .items_center()
                    .child(send_command_stepper_button(
                        palette,
                        "bottom-command-count-down",
                        "−",
                        cx.listener(|this, _, _, cx| {
                            this.adjust_send_command_count(-1, cx);
                        }),
                    ))
                    .child(
                        div()
                            .min_w(px(28.))
                            .h(px(28.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .font_family(crate::features::gpui_code_font_family())
                            .text_size(px(11.))
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(palette.text))
                            .child(count_label),
                    )
                    .child(send_command_stepper_button(
                        palette,
                        "bottom-command-count-up",
                        "+",
                        cx.listener(|this, _, _, cx| {
                            this.adjust_send_command_count(1, cx);
                        }),
                    )),
            ))
            .child(send_command_control_group(
                palette,
                "Interval",
                div()
                    .flex()
                    .items_center()
                    .child(send_command_stepper_button(
                        palette,
                        "bottom-command-interval-down",
                        "−",
                        cx.listener(|this, _, _, cx| {
                            this.adjust_send_command_interval(-0.1, cx);
                        }),
                    ))
                    .child(
                        div()
                            .min_w(px(40.))
                            .h(px(28.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap_1()
                            .font_family(crate::features::gpui_code_font_family())
                            .text_size(px(11.))
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(palette.text))
                            .child(interval_label)
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight(500.))
                                    .text_color(rgb(palette.text_dimmed))
                                    .child("s"),
                            ),
                    )
                    .child(send_command_stepper_button(
                        palette,
                        "bottom-command-interval-up",
                        "+",
                        cx.listener(|this, _, _, cx| {
                            this.adjust_send_command_interval(0.1, cx);
                        }),
                    )),
            ))
            .when(is_serial_text_line, |this| {
                this.child(send_command_control_group(
                    palette,
                    "EOL",
                    div().flex().items_center().child(send_command_chip(
                        palette,
                        "bottom-command-eol",
                        line_ending_label,
                        true,
                        cx.listener(|this, _, _, cx| {
                            this.send_command_line_ending = match this.send_command_line_ending {
                                SendCommandLineEnding::None => SendCommandLineEnding::Cr,
                                SendCommandLineEnding::Cr => SendCommandLineEnding::Lf,
                                SendCommandLineEnding::Lf => SendCommandLineEnding::Crlf,
                                SendCommandLineEnding::Crlf => SendCommandLineEnding::None,
                            };
                            cx.notify();
                        }),
                    )),
                ))
            })
            .into_any_element()
    }
}
