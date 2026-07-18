use super::*;

pub(in crate::features::pages::remote) fn process_details(
    palette: ThemePalette,
    process: &RemoteProcess,
    mode: ProcessDisplayMode,
    nice_draft: String,
    nice_focus: &gpui::FocusHandle,
    cx: &mut Context<NyaTermApp>,
) -> gpui::AnyElement {
    // Tauri expanded process details: compact mono command + meta chips + dense actions.
    let command = if process.command_line.trim().is_empty() {
        process.command.clone()
    } else {
        process.command_line.clone()
    };
    let pid = process.pid;
    let details_h = process_details_height_px(mode) - 2.; // account for mb_1
    div()
        .mx_2()
        .mb_1()
        .h(px(details_h))
        .overflow_hidden()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.bg))
        .px_2()
        .py_2()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(11.))
                .line_height(px(15.))
                .text_color(rgb(palette.text))
                .child(truncate_preview(&command, 180)),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_1()
                .child(process_detail_chip(
                    palette,
                    "PPID",
                    process.ppid.to_string(),
                ))
                .child(process_detail_chip(
                    palette,
                    "RSS",
                    format_file_size(Some(process.rss_kb.saturating_mul(1024))),
                ))
                .child(process_detail_chip(palette, "State", process.state.clone()))
                .child(process_detail_chip(palette, "User", process.user.clone()))
                .child(process_detail_chip(palette, "PID", process.pid.to_string()))
                .child(process_detail_chip(
                    palette,
                    "Elapsed",
                    process.elapsed.clone(),
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .flex_wrap()
                .child(
                    transfer_input("process-nice-input", "Nice", nice_draft, true, palette)
                        .w(px(88.))
                        .h(px(26.))
                        .track_focus(nice_focus)
                        .on_click(cx.listener(|this, _, window, cx| {
                            window.focus(&this.process_nice_focus);
                            cx.notify();
                        }))
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                            cx.stop_propagation();
                            this.handle_process_nice_key_down(event, window, cx);
                        })),
                )
                .child(small_button(
                    palette,
                    format!("process-nice-apply-{pid}"),
                    "Apply",
                    cx.listener(move |this, _, window, cx| {
                        this.apply_process_nice_draft(window, cx);
                    }),
                ))
                .child(small_button(
                    palette,
                    format!("process-nice-low-{pid}"),
                    "-5",
                    cx.listener(move |this, _, window, cx| {
                        this.renice_process(pid, -5, window, cx);
                    }),
                ))
                .child(small_button(
                    palette,
                    format!("process-nice-zero-{pid}"),
                    "0",
                    cx.listener(move |this, _, window, cx| {
                        this.renice_process(pid, 0, window, cx);
                    }),
                ))
                .child(small_button(
                    palette,
                    format!("process-nice-high-{pid}"),
                    "+5",
                    cx.listener(move |this, _, window, cx| {
                        this.renice_process(pid, 5, window, cx);
                    }),
                ))
                .child(
                    div()
                        .mx_1()
                        .text_size(px(10.))
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(palette.text_dimmed))
                        .child("SIG"),
                )
                .child(small_button(
                    palette,
                    format!("process-signal-term-{pid}"),
                    "TERM",
                    cx.listener(move |this, _, window, cx| {
                        this.request_process_signal(pid, "TERM", window, cx);
                    }),
                ))
                .child(small_button(
                    palette,
                    format!("process-signal-hup-{pid}"),
                    "HUP",
                    cx.listener(move |this, _, window, cx| {
                        this.request_process_signal(pid, "HUP", window, cx);
                    }),
                ))
                .child(small_button(
                    palette,
                    format!("process-signal-stop-{pid}"),
                    "STOP",
                    cx.listener(move |this, _, window, cx| {
                        this.request_process_signal(pid, "STOP", window, cx);
                    }),
                ))
                .child(small_button(
                    palette,
                    format!("process-signal-cont-{pid}"),
                    "CONT",
                    cx.listener(move |this, _, window, cx| {
                        this.request_process_signal(pid, "CONT", window, cx);
                    }),
                ))
                .child(small_button(
                    palette,
                    format!("process-signal-kill-{pid}"),
                    "KILL",
                    cx.listener(move |this, _, window, cx| {
                        this.request_process_signal(pid, "KILL", window, cx);
                    }),
                )),
        )
        .into_any_element()
}

pub(in crate::features::pages::remote) fn process_detail_chip(
    palette: ThemePalette,
    label: &'static str,
    value: String,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.surface_elevated))
        .bg(rgb(palette.surface))
        .px_2()
        .py_0()
        .h(px(28.))
        .flex()
        .items_center()
        .gap_1()
        .child(
            div()
                .text_size(px(10.))
                .font_weight(FontWeight(700.))
                .text_color(rgb(palette.text_dimmed))
                .child(label),
        )
        .child(
            div()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(11.))
                .text_color(rgb(palette.text))
                .child(truncate_preview(&value, 24)),
        )
}

pub(in crate::features::pages::remote) fn process_signal_confirm_panel(
    palette: ThemePalette,
    confirm: RemoteProcessSignalConfirmState,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let card = div()
        .p_4()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight(800.))
                .text_color(rgb(palette.danger))
                .child(format!(
                    "Confirm {} for PID {}",
                    confirm.signal, confirm.pid
                )),
        )
        .child(
            div()
                .font_family(crate::features::gpui_code_font_family())
                .text_xs()
                .line_height(px(17.))
                .text_color(rgb(0xfecdd3))
                .child(format!(
                    "kill -{} -- {} · {}",
                    confirm.signal,
                    confirm.pid,
                    truncate_preview(&confirm.command, 96)
                )),
        )
        .child(
            div()
                .pt_2()
                .flex()
                .justify_end()
                .gap_2()
                .child(small_button(
                    palette,
                    "process-signal-cancel",
                    "Cancel",
                    cx.listener(|this, _, _, cx| {
                        this.cancel_process_signal_confirm(cx);
                    }),
                ))
                .child(small_button(
                    palette,
                    "process-signal-confirm",
                    "Confirm",
                    cx.listener(|this, _, window, cx| {
                        this.confirm_process_signal(window, cx);
                    }),
                )),
        );
    modal_dialog_shell(palette, "process-signal-confirm-modal", 420., card)
}
