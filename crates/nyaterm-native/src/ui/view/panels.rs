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
        let target_status = if self.active_session_id.is_some() {
            "current"
        } else {
            "offline"
        };
        let target_kind = match active_kind {
            Some(SessionKind::Serial) => "serial",
            Some(SessionKind::RawTcp) => "raw tcp",
            Some(SessionKind::Telnet) => "telnet",
            Some(SessionKind::Ssh | SessionKind::LocalPty) => "shell",
            None => "current",
        };
        let unit_result =
            self.build_send_command_units(&self.send_command_draft.clone(), active_kind);
        let (validation_text, validation_error) = match &unit_result {
            Ok(units) => {
                let bytes = units.iter().map(Vec::len).sum::<usize>();
                (
                    format!(
                        "{} unit(s) / {} byte(s) · {target_kind} · {target_status}",
                        units.len(),
                        bytes
                    ),
                    false,
                )
            }
            Err(error) => (format!("{error} · {target_kind}"), true),
        };
        let preview = if self.send_command_data_type == SendCommandDataType::Hex {
            send_command_hex_preview(&self.send_command_draft)
        } else {
            truncate_preview(&self.send_command_draft.replace('\n', "\\n"), 96)
        };
        let input_hint = if self.send_command_data_type == SendCommandDataType::Hex {
            "HEX Editor"
        } else {
            "Shell Command"
        };

        // Tauri SendCommandPanel: compact toolbar controls + flex editor + action footer.
        div()
            .h(px(220.))
            .flex_none()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(rgb(0x30363d))
            .bg(rgb(0x161b22))
            .child(
                div()
                    .h(px(32.))
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
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(0x8b949e))
                            .child("COMMAND SEND"),
                    )
                    .child(status_pill(
                        target_status,
                        if self.active_session_id.is_some() {
                            rgb(0x3fb950)
                        } else {
                            rgb(0xff7b72)
                        },
                        if self.active_session_id.is_some() {
                            rgb(0x12261a)
                        } else {
                            rgb(0x3d1418)
                        },
                    ))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .font_family("JetBrains Mono")
                            .text_size(px(10.))
                            .text_color(rgb(0x6e7681))
                            .overflow_hidden()
                            .child(truncate_preview(&active_target, 42)),
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
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .flex_wrap()
                            .child(mode_button(
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
                                    this.terminal_status = "command send data: Text".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(mode_button(
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
                                    this.terminal_status = "command send data: Hex".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(mode_button(
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
                            .child(mode_button(
                                "bottom-command-send-mode-secondary",
                                if self.send_command_data_type == SendCommandDataType::Hex {
                                    "Packet"
                                } else {
                                    "Character"
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
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "bottom-command-count-down",
                                "- Count",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_send_command_count(-1, cx);
                                }),
                            ))
                            .child(bottom_send_field(
                                "Count",
                                self.send_command_count.to_string(),
                            ))
                            .child(small_button(
                                "bottom-command-count-up",
                                "+ Count",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_send_command_count(1, cx);
                                }),
                            )),
                    ),
            )
            .child(
                div()
                    .mt_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .flex_wrap()
                            .child(mode_button(
                                "bottom-command-line-ending-none",
                                "None",
                                self.send_command_line_ending == SendCommandLineEnding::None,
                                cx.listener(|this, _, _, cx| {
                                    this.send_command_line_ending = SendCommandLineEnding::None;
                                    cx.notify();
                                }),
                            ))
                            .child(mode_button(
                                "bottom-command-line-ending-cr",
                                "CR",
                                self.send_command_line_ending == SendCommandLineEnding::Cr,
                                cx.listener(|this, _, _, cx| {
                                    this.send_command_line_ending = SendCommandLineEnding::Cr;
                                    cx.notify();
                                }),
                            ))
                            .child(mode_button(
                                "bottom-command-line-ending-lf",
                                "LF",
                                self.send_command_line_ending == SendCommandLineEnding::Lf,
                                cx.listener(|this, _, _, cx| {
                                    this.send_command_line_ending = SendCommandLineEnding::Lf;
                                    cx.notify();
                                }),
                            ))
                            .child(mode_button(
                                "bottom-command-line-ending-crlf",
                                "CRLF",
                                self.send_command_line_ending == SendCommandLineEnding::Crlf,
                                cx.listener(|this, _, _, cx| {
                                    this.send_command_line_ending = SendCommandLineEnding::Crlf;
                                    cx.notify();
                                }),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "bottom-command-interval-down",
                                "- 0.1s",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_send_command_interval(-0.1, cx);
                                }),
                            ))
                            .child(small_button(
                                "bottom-command-interval-up",
                                "+ 0.1s",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_send_command_interval(0.1, cx);
                                }),
                            )),
                    ),
            )
            .child(
                transfer_input(
                    "bottom-command-send-input",
                    input_hint,
                    self.send_command_draft.clone(),
                    true,
                )
                .flex_1()
                .min_h(px(72.))
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
            .child(
                div()
                    .flex_none()
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
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(if validation_error {
                                        rgb(0xff7b72)
                                    } else {
                                        rgb(0x98a3b8)
                                    })
                                    .overflow_hidden()
                                    .child(validation_text),
                            )
                            .child(
                                div()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(10.))
                                    .text_color(rgb(0x64748b))
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
                            .gap_2()
                            .child(small_button(
                                "bottom-command-send-clear",
                                "Clear",
                                cx.listener(|this, _, _, cx| {
                                    this.send_command_draft.clear();
                                    this.terminal_status = "command send cleared".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(small_button(
                                "bottom-command-send-now",
                                "Send",
                                cx.listener(|this, _, _, cx| {
                                    this.send_bottom_command(false, cx);
                                }),
                            ))
                            .child(small_button(
                                "bottom-command-send-enter",
                                "Send + Enter",
                                cx.listener(|this, _, _, cx| {
                                    this.send_bottom_command(true, cx);
                                }),
                            )),
                    ),
            ),
            )
    }
}
