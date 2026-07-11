use super::*;

#[path = "panels/helpers.rs"]
mod helpers;
#[path = "panels/lock_screen_overlay.rs"]
mod lock_screen_overlay;
#[path = "panels/multi_line_paste_overlay.rs"]
mod multi_line_paste_overlay;
#[path = "panels/quick_command_category_overlays.rs"]
mod quick_command_category_overlays;
#[path = "panels/quick_command_delete_overlay.rs"]
mod quick_command_delete_overlay;
#[path = "panels/quick_command_details_overlay.rs"]
mod quick_command_details_overlay;
#[path = "panels/quick_command_editor_overlay.rs"]
mod quick_command_editor_overlay;
#[path = "panels/quick_command_import_overlay.rs"]
mod quick_command_import_overlay;
#[path = "panels/quick_command_variable_overlay.rs"]
mod quick_command_variable_overlay;
#[path = "panels/quick_commands_panel.rs"]
mod quick_commands_panel;
#[path = "panels/quick_switch_overlay.rs"]
mod quick_switch_overlay;
#[path = "panels/recording_panel.rs"]
mod recording_panel;
#[path = "panels/session_confirm_overlays.rs"]
mod session_confirm_overlays;
#[path = "panels/session_overlays.rs"]
mod session_overlays;
#[path = "panels/sync_groups_overlay.rs"]
mod sync_groups_overlay;
#[path = "panels/tab_actions_overlay.rs"]
mod tab_actions_overlay;
#[path = "panels/temporary_ssh_link_overlay.rs"]
mod temporary_ssh_link_overlay;
#[path = "panels/terminal_actions_overlay.rs"]
mod terminal_actions_overlay;

pub(in crate::ui::view::panels) use helpers::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn bottom_command_send_bar(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active_kind = self.active_session_kind();
        let active_target = self
            .active_session_name()
            .or_else(|| {
                self.active_session_id
                    .as_ref()
                    .map(|session_id| format!("Session {}", short_id(session_id)))
            })
            .unwrap_or_else(|| "No active session".to_string());
        let target_available = self.active_session_id.is_some();
        let group_targets = self.send_command_group_target_options();
        let target_scope_label = match &self.send_command_target {
            SendCommandTarget::Current => active_target.clone(),
            SendCommandTarget::AllCompatible => {
                let n = self.send_command_target_session_ids().len();
                if n == 0 {
                    "No compatible sessions".to_string()
                } else {
                    format!("All compatible ({n})")
                }
            }
            SendCommandTarget::Group(group_id) => {
                let n = self.send_command_target_session_ids().len();
                let name = group_targets
                    .iter()
                    .find(|(id, _, _)| id == group_id)
                    .map(|(_, name, _)| name.clone())
                    .or_else(|| {
                        self.sync_groups
                            .iter()
                            .find(|group| &group.id == group_id)
                            .map(|group| group.name.clone())
                    })
                    .unwrap_or_else(|| "Group".to_string());
                if n == 0 {
                    format!("Group: {name} (empty)")
                } else {
                    format!("Group: {name} ({n})")
                }
            }
        };
        let target_kind = match active_kind {
            Some(SessionKind::Serial) => "Serial Data",
            Some(SessionKind::RawTcp) => "Raw TCP",
            Some(SessionKind::Telnet) => "Telnet",
            Some(SessionKind::Ssh | SessionKind::LocalPty) => "Shell Command",
            None => "No session",
        };
        let is_serial_text_line = matches!(active_kind, Some(SessionKind::Serial))
            && self.send_command_data_type == SendCommandDataType::Text
            && self.send_command_mode == SendCommandMode::Line;
        let unit_result =
            self.build_send_command_units(&self.send_command_draft.clone(), active_kind);
        let (validation_text, validation_error, unit_count, byte_count) = match &unit_result {
            Ok(units) => {
                let bytes = units.iter().map(Vec::len).sum::<usize>();
                (
                    format!(
                        "{} unit(s) · {} byte(s) · count {} · interval {:.2}s",
                        units.len(),
                        bytes,
                        self.send_command_count,
                        self.send_command_interval_seconds
                    ),
                    false,
                    units.len(),
                    bytes,
                )
            }
            Err(error) => (error.clone(), true, 0usize, 0usize),
        };
        let preview = if self.send_command_data_type == SendCommandDataType::Hex {
            send_command_hex_preview(&self.send_command_draft)
        } else {
            truncate_preview(&self.send_command_draft.replace('\n', "\\n"), 96)
        };
        let input_hint = if self.send_command_data_type == SendCommandDataType::Hex {
            "e.g. 48 65 6C 6C 6F"
        } else {
            "Type command or payload…"
        };
        let count_label = self.send_command_count.to_string();
        let interval_label = format!("{:.2}", self.send_command_interval_seconds);
        let line_ending_label = match self.send_command_line_ending {
            SendCommandLineEnding::None => "None",
            SendCommandLineEnding::Cr => "CR",
            SendCommandLineEnding::Lf => "LF",
            SendCommandLineEnding::Crlf => "CR+LF",
        };
        let _ = (unit_count, byte_count);
        let is_sending = self.send_command_sending;
        let progress_total = self.send_command_progress_total.max(1);
        let progress_completed = self.send_command_progress_completed.min(progress_total);
        let progress_ratio = progress_completed as f32 / progress_total as f32;
        let progress_label = if is_sending {
            format!(
                "Sending {}/{} · round {}/{}",
                progress_completed,
                self.send_command_progress_total,
                self.send_command_progress_round.max(1),
                self.send_command_progress_rounds.max(1)
            )
        } else {
            validation_text.clone()
        };

        // Tauri SendCommandPanel: title row + labeled control groups + editor + action footer.
        div()
            .h(px(240.))
            .flex_none()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(rgb(0x30363d))
            .bg(rgb(0x161b22))
            .child(
                div()
                    .h(px(28.))
                    .flex_none()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x12171f))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(0xc9d1d9))
                            .child("Command Send"),
                    )
                    .child(
                        div()
                            .ml_auto()
                            .text_size(px(10.))
                            .text_color(rgb(0x6e7681))
                            .child(target_kind),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .max_w(px(160.))
                            .font_family("JetBrains Mono")
                            .text_size(px(10.))
                            .text_color(if target_available {
                                rgb(0x8b949e)
                            } else {
                                rgb(0xff7b72)
                            })
                            .overflow_hidden()
                            .child(truncate_preview(&target_scope_label, 28)),
                    )
                    .child(small_button(
                        "bottom-command-send-hide",
                        "Hide",
                        cx.listener(|this, _, _, cx| {
                            this.bottom_panel = BottomPanelMode::Hidden;
                            cx.notify();
                        }),
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .px_2()
                    .py_1()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_1()
                            .child(send_command_control_group(
                                "Data",
                                div()
                                    .flex()
                                    .items_center()
                                    .child(send_command_chip(
                                        "bottom-command-send-text",
                                        "Text",
                                        self.send_command_data_type == SendCommandDataType::Text,
                                        cx.listener(|this, _, _, cx| {
                                            this.send_command_data_type = SendCommandDataType::Text;
                                            if matches!(
                                                this.send_command_mode,
                                                SendCommandMode::Packet | SendCommandMode::Byte
                                            ) {
                                                this.send_command_mode = SendCommandMode::Line;
                                            }
                                            this.terminal_status =
                                                "command send data: Text".to_string();
                                            cx.notify();
                                        }),
                                    ))
                                    .child(send_command_chip(
                                        "bottom-command-send-hex",
                                        "Hex",
                                        self.send_command_data_type == SendCommandDataType::Hex,
                                        cx.listener(|this, _, _, cx| {
                                            this.send_command_data_type = SendCommandDataType::Hex;
                                            if matches!(
                                                this.send_command_mode,
                                                SendCommandMode::Line | SendCommandMode::Character
                                            ) {
                                                this.send_command_mode = SendCommandMode::Byte;
                                            }
                                            this.terminal_status =
                                                "command send data: Hex".to_string();
                                            cx.notify();
                                        }),
                                    )),
                            ))
                            .child(send_command_control_group(
                                "Target",
                                {
                                    let mut chips = div().flex().items_center().flex_wrap().gap_0();
                                    chips = chips
                                        .child(send_command_chip(
                                            "bottom-command-target-current",
                                            "Current",
                                            matches!(
                                                self.send_command_target,
                                                SendCommandTarget::Current
                                            ),
                                            cx.listener(|this, _, _, cx| {
                                                this.set_send_command_target(
                                                    SendCommandTarget::Current,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(send_command_chip(
                                            "bottom-command-target-all",
                                            "All",
                                            matches!(
                                                self.send_command_target,
                                                SendCommandTarget::AllCompatible
                                            ),
                                            cx.listener(|this, _, _, cx| {
                                                this.set_send_command_target(
                                                    SendCommandTarget::AllCompatible,
                                                    cx,
                                                );
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
                                },
                            ))
                            .child(send_command_control_group(
                                "Mode",
                                div()
                                    .flex()
                                    .items_center()
                                    .child(send_command_chip(
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
                                            this.send_command_mode = if this.send_command_data_type
                                                == SendCommandDataType::Hex
                                            {
                                                SendCommandMode::Byte
                                            } else {
                                                SendCommandMode::Line
                                            };
                                            cx.notify();
                                        }),
                                    ))
                                    .child(send_command_chip(
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
                                            this.send_command_mode = if this.send_command_data_type
                                                == SendCommandDataType::Hex
                                            {
                                                SendCommandMode::Packet
                                            } else {
                                                SendCommandMode::Character
                                            };
                                            cx.notify();
                                        }),
                                    )),
                            ))
                            .child(send_command_control_group(
                                "Count",
                                div()
                                    .flex()
                                    .items_center()
                                    .child(send_command_stepper_button(
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
                                            .font_family("JetBrains Mono")
                                            .text_size(px(11.))
                                            .font_weight(FontWeight(600.))
                                            .text_color(rgb(0xc9d1d9))
                                            .child(count_label),
                                    )
                                    .child(send_command_stepper_button(
                                        "bottom-command-count-up",
                                        "+",
                                        cx.listener(|this, _, _, cx| {
                                            this.adjust_send_command_count(1, cx);
                                        }),
                                    )),
                            ))
                            .child(send_command_control_group(
                                "Interval",
                                div()
                                    .flex()
                                    .items_center()
                                    .child(send_command_stepper_button(
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
                                            .font_family("JetBrains Mono")
                                            .text_size(px(11.))
                                            .font_weight(FontWeight(600.))
                                            .text_color(rgb(0xc9d1d9))
                                            .child(interval_label)
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .font_weight(FontWeight(500.))
                                                    .text_color(rgb(0x6e7681))
                                                    .child("s"),
                                            ),
                                    )
                                    .child(send_command_stepper_button(
                                        "bottom-command-interval-up",
                                        "+",
                                        cx.listener(|this, _, _, cx| {
                                            this.adjust_send_command_interval(0.1, cx);
                                        }),
                                    )),
                            ))
                            .when(is_serial_text_line, |this| {
                                this.child(send_command_control_group(
                                    "EOL",
                                    div()
                                        .flex()
                                        .items_center()
                                        .child(send_command_chip(
                                            "bottom-command-eol",
                                            line_ending_label,
                                            true,
                                            cx.listener(|this, _, _, cx| {
                                                this.send_command_line_ending =
                                                    match this.send_command_line_ending {
                                                        SendCommandLineEnding::None => {
                                                            SendCommandLineEnding::Cr
                                                        }
                                                        SendCommandLineEnding::Cr => {
                                                            SendCommandLineEnding::Lf
                                                        }
                                                        SendCommandLineEnding::Lf => {
                                                            SendCommandLineEnding::Crlf
                                                        }
                                                        SendCommandLineEnding::Crlf => {
                                                            SendCommandLineEnding::None
                                                        }
                                                    };
                                                cx.notify();
                                            }),
                                        )),
                                ))
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h(px(72.))
                            .flex()
                            .gap_1()
                            .child(
                                transfer_input(
                                    "bottom-command-send-input",
                                    input_hint,
                                    if self.send_command_data_type == SendCommandDataType::Hex {
                                        format_send_command_hex_display(&self.send_command_draft)
                                    } else {
                                        self.send_command_draft.clone()
                                    },
                                    true,
                                )
                                .flex_1()
                                .min_h(px(72.))
                                .font_family("JetBrains Mono")
                                .track_focus(&self.send_command_focus)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    window.focus(&this.send_command_focus);
                                    cx.notify();
                                }))
                                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.handle_send_command_key_down(event, cx);
                                })),
                            )
                            .when(
                                self.send_command_data_type == SendCommandDataType::Hex,
                                |this| {
                                    let byte_count =
                                        send_command_hex_byte_count(&self.send_command_draft);
                                    let guide_count =
                                        send_command_hex_guide_count(&self.send_command_draft);
                                    this.child(
                                        div()
                                            .w(px(180.))
                                            .flex_none()
                                            .min_h(px(72.))
                                            .rounded_md()
                                            .border_1()
                                            .border_color(rgb(0x30363d))
                                            .bg(rgb(0x0d1117))
                                            .px_2()
                                            .py_1()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_size(px(10.))
                                                            .font_weight(FontWeight(600.))
                                                            .text_color(rgb(0x6e7681))
                                                            .child("Preview"),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(10.))
                                                            .text_color(rgb(0x6e7681))
                                                            .child(match byte_count {
                                                                Some(n) => format!("{n} B"),
                                                                None => "invalid".to_string(),
                                                            }),
                                                    ),
                                            )
                                            .when(guide_count > 0, |this| {
                                                this.child(
                                                    div()
                                                        .text_size(px(10.))
                                                        .text_color(rgb(0x388bfd))
                                                        .child(format!(
                                                            "guides ×{guide_count} (4-byte)"
                                                        )),
                                                )
                                            })
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .font_family("JetBrains Mono")
                                                    .text_size(px(11.))
                                                    .line_height(px(15.))
                                                    .text_color(if validation_error {
                                                        rgb(0xff7b72)
                                                    } else {
                                                        rgb(0xc9d1d9)
                                                    })
                                                    .child(if preview.trim().is_empty() {
                                                        "·".to_string()
                                                    } else {
                                                        preview.clone()
                                                    }),
                                            ),
                                    )
                                },
                            ),
                    )
                    .when(is_sending, |this| {
                        this.child(
                            div()
                                .flex_none()
                                .px_1()
                                .child(
                                    div()
                                        .rounded_md()
                                        .border_1()
                                        .border_color(rgb(0x1f6feb))
                                        .bg(rgb(0x0d1117))
                                        .px_2()
                                        .py_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .text_size(px(10.))
                                                        .font_weight(FontWeight(600.))
                                                        .text_color(rgb(0xc9d1d9))
                                                        .child(progress_label.clone()),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(10.))
                                                        .text_color(rgb(0x6e7681))
                                                        .child(format!(
                                                            "{:.0}%",
                                                            progress_ratio * 100.0
                                                        )),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .h(px(6.))
                                                .w_full()
                                                .rounded_full()
                                                .bg(rgb(0x21262d))
                                                .overflow_hidden()
                                                .child(
                                                    div()
                                                        .h_full()
                                                        .w(px((280.0 * progress_ratio).max(2.0)))
                                                        .rounded_full()
                                                        .bg(rgb(0x1f6feb)),
                                                ),
                                        ),
                                ),
                        )
                    })
                    .child(
                        div()
                            .flex_none()
                            .h(px(34.))
                            .pt_1()
                            .border_t_1()
                            .border_color(rgb(0x30363d))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap_0()
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(if validation_error && !is_sending {
                                                rgb(0xff7b72)
                                            } else if is_sending {
                                                rgb(0x58a6ff)
                                            } else {
                                                rgb(0x8b949e)
                                            })
                                            .overflow_hidden()
                                            .child(if is_sending {
                                                progress_label
                                            } else {
                                                validation_text
                                            }),
                                    )
                                    .child(
                                        div()
                                            .font_family("JetBrains Mono")
                                            .text_size(px(10.))
                                            .text_color(rgb(0x6e7681))
                                            .overflow_hidden()
                                            .child(if preview.trim().is_empty() {
                                                "preview empty".to_string()
                                            } else {
                                                preview
                                            }),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(small_button(
                                        "bottom-command-send-clear",
                                        "Clear",
                                        cx.listener(|this, _, _, cx| {
                                            this.send_command_draft.clear();
                                            this.terminal_status =
                                                "command send cleared".to_string();
                                            cx.notify();
                                        }),
                                    ))
                                    .child(small_button(
                                        "bottom-command-send-now",
                                        if is_sending { "Stop" } else { "Send" },
                                        cx.listener(|this, _, _, cx| {
                                            this.send_bottom_command(false, cx);
                                        }),
                                    ))
                                    .child(small_button(
                                        "bottom-command-send-enter",
                                        if is_sending { "Stop" } else { "Send ↵" },
                                        cx.listener(|this, _, _, cx| {
                                            if this.send_command_sending {
                                                this.stop_send_command(cx);
                                            } else {
                                                this.send_bottom_command(true, cx);
                                            }
                                        }),
                                    )),
                            ),
                    ),
            )
    }

}


fn send_command_control_group(
    label: &'static str,
    content: impl IntoElement,
) -> impl IntoElement {
    // Tauri labeled control: h-8 bordered group with muted label prefix.
    div()
        .h(px(30.))
        .flex()
        .items_center()
        .overflow_hidden()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x0d1117))
        .child(
            div()
                .flex_none()
                .px_2()
                .text_size(px(10.))
                .text_color(rgb(0x6e7681))
                .child(label),
        )
        .child(
            div()
                .h_full()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .border_l_1()
                .border_color(rgb(0x30363d))
                .child(content),
        )
}

fn send_command_chip(
    id: impl Into<String>,
    label: &'static str,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_2()
        .flex()
        .items_center()
        .text_size(px(11.))
        .font_weight(if active {
            FontWeight(600.)
        } else {
            FontWeight(500.)
        })
        .text_color(if active {
            rgb(0x58a6ff)
        } else {
            rgb(0x8b949e)
        })
        .bg(if active {
            rgb(0x122033)
        } else {
            rgb(0x00000000)
        })
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xc9d1d9)))
        .child(label)
        .on_click(on_click)
}

fn send_command_target_chip(
    id: impl Into<String>,
    label: impl Into<SharedString>,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_2()
        .flex()
        .items_center()
        .text_size(px(11.))
        .font_weight(if active {
            FontWeight(600.)
        } else {
            FontWeight(500.)
        })
        .text_color(if active {
            rgb(0x58a6ff)
        } else {
            rgb(0x8b949e)
        })
        .bg(if active {
            rgb(0x122033)
        } else {
            rgb(0x00000000)
        })
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xc9d1d9)))
        .child(label.into())
        .on_click(on_click)
}


fn send_command_stepper_button(
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(12.))
        .text_color(rgb(0x8b949e))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xc9d1d9)))
        .child(label)
        .on_click(on_click)
}
