use super::*;

pub(super) fn process_matches(process: &RemoteProcess, normalized_query: &str) -> bool {
    if normalized_query.is_empty() {
        return true;
    }
    format!(
        "{} {} {} {} {} {}",
        process.pid,
        process.ppid,
        process.user,
        process.state,
        process.command,
        process.command_line
    )
    .to_ascii_lowercase()
    .contains(normalized_query)
}

pub(super) fn sort_processes(
    processes: &mut [RemoteProcess],
    key: RemoteProcessSortKey,
    direction: RemoteProcessSortDirection,
) {
    processes.sort_by(|left, right| {
        let ordering = match key {
            RemoteProcessSortKey::Command => left
                .command
                .cmp(&right.command)
                .then_with(|| left.pid.cmp(&right.pid)),
            RemoteProcessSortKey::Memory => left
                .memory_percent
                .partial_cmp(&right.memory_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    left.rss_kb
                        .partial_cmp(&right.rss_kb)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| left.pid.cmp(&right.pid)),
            RemoteProcessSortKey::Pid => left.pid.cmp(&right.pid),
            RemoteProcessSortKey::User => left
                .user
                .cmp(&right.user)
                .then_with(|| left.pid.cmp(&right.pid)),
            RemoteProcessSortKey::Cpu => left
                .cpu_percent
                .partial_cmp(&right.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    left.memory_percent
                        .partial_cmp(&right.memory_percent)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| left.pid.cmp(&right.pid)),
        };

        match direction {
            RemoteProcessSortDirection::Ascending => ordering,
            RemoteProcessSortDirection::Descending => ordering.reverse(),
        }
    });
}

pub(super) fn top_process_ratio(processes: &[RemoteProcess], cpu: bool) -> f64 {
    processes
        .iter()
        .map(|process| {
            if cpu {
                process.cpu_percent
            } else {
                process.memory_percent
            }
        })
        .fold(0.0, f64::max)
        / 100.
}

pub(super) fn process_summary_card(
    title: &'static str,
    value: String,
    ratio: f64,
) -> impl IntoElement {
    let ratio = ratio.clamp(0., 1.);
    // Compact metric chip for Process Manager summary strip.
    div()
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
                .text_size(px(10.))
                .font_weight(FontWeight(600.))
                .text_color(rgb(0x8b949e))
                .child(title),
        )
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight(700.))
                .text_color(usage_color(ratio))
                .child(value),
        )
        .child(stats_progress_bar(ratio))
}

pub(super) fn process_sort_button(
    id: impl Into<String>,
    label: &'static str,
    active: bool,
    direction: RemoteProcessSortDirection,
    numeric: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // Flat sortable header cell (Tauri table header).
    div()
        .id(gpui::SharedString::from(id.into()))
        .h_full()
        .min_w_0()
        .px_1()
        .flex()
        .items_center()
        .when(numeric, |this| this.justify_end())
        .rounded_sm()
        .text_size(px(10.))
        .font_weight(if active { FontWeight(700.) } else { FontWeight(600.) })
        .text_color(if active { rgb(0xc9d1d9) } else { rgb(0x6e7681) })
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xc9d1d9)))
        .child(if active {
            format!("{label} {}", direction.marker())
        } else {
            label.to_string()
        })
        .on_click(on_click)
}

pub(super) fn process_table_header() -> impl IntoElement {
    // Static fallback header; live header uses process_sort_button grid in process_view.
    div()
        .grid()
        .grid_cols(6)
        .gap_1()
        .h(px(26.))
        .flex_none()
        .border_b_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x0d1117))
        .px_2()
        .items_center()
        .text_size(px(10.))
        .font_weight(FontWeight(700.))
        .text_color(rgb(0x6e7681))
        .child("Process")
        .child(div().text_right().child("PID"))
        .child(div().text_right().child("CPU"))
        .child(div().text_right().child("Mem"))
        .child("User")
        .child("")
}

pub(super) fn process_table_row(
    process: &RemoteProcess,
    selected: bool,
    menu_open: bool,
    on_select: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_menu: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_copy_pid: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_copy_command: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_term: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_hup: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_stop: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cont: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_kill: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui::Div {
    // Tauri ProcessManager: denser mono table + ⋮ overflow actions.
    // Tauri: left accent based on load / selection.
    let accent = if process.cpu_percent >= 80.0 {
        rgb(0xf85149)
    } else if process.memory_percent >= 80.0 {
        rgb(0xd29922)
    } else if selected {
        rgb(0x1f6feb)
    } else {
        rgb(0x30363d)
    };
    div()
        .relative()
        .border_b_1()
        .border_color(rgb(0x21262d))
        .bg(if selected {
            rgb(0x122033)
        } else {
            rgb(0x161b22)
        })
        .hover(|this| this.bg(rgb(0x1c2128)))
        .child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .bottom_0()
                .w(px(2.))
                .bg(accent),
        )
        .child(
            div()
                .grid()
                .id(gpui::SharedString::from(format!(
                    "process-row-{}",
                    process.pid
                )))
                .grid_cols(6)
                .gap_1()
                .h(px(38.))
                .px_2()
                .pl(px(10.))
                .items_center()
                .cursor_pointer()
                .on_click(on_select)
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .justify_center()
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight(600.))
                                .text_color(rgb(0xe5edf7))
                                .overflow_hidden()
                                .child(truncate_preview(&process.command, 40)),
                        )
                        .child(
                            div()
                                .font_family("JetBrains Mono")
                                .text_size(px(10.))
                                .text_color(rgb(0x6e7681))
                                .overflow_hidden()
                                .child(truncate_preview(&process.command_line, 52)),
                        ),
                )
                .child(process_table_cell(process.pid.to_string(), None, true))
                .child(process_table_cell(
                    format!("{:.1}%", process.cpu_percent),
                    Some(usage_color(process.cpu_percent / 100.)),
                    true,
                ))
                .child(process_table_cell(
                    format!("{:.1}%", process.memory_percent),
                    Some(usage_color(process.memory_percent / 100.)),
                    true,
                ))
                .child(process_table_cell(
                    truncate_preview(&process.user, 12),
                    None,
                    false,
                ))
                .child(
                    div()
                        .relative()
                        .flex()
                        .items_center()
                        .justify_end()
                        .child(compact_remote_svg_button(
                            format!("process-menu-{}", process.pid),
                            "icons/conn/more.svg",
                            on_menu,
                        ))
                        .when(menu_open, |this| {
                            this.child(
                                div()
                                    .id(gpui::SharedString::from(format!(
                                        "process-menu-pop-{}",
                                        process.pid
                                    )))
                                    .absolute()
                                    .top(px(26.))
                                    .right_0()
                                    .w(px(148.))
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(0x30363d))
                                    .bg(rgb(0x161b22))
                                    .shadow_lg()
                                    .py_1()
                                    .flex()
                                    .flex_col()
                                    .on_mouse_down(gpui::MouseButton::Left, |_, _, _| {})
                                    .child(process_menu_item(
                                        format!("process-copy-pid-{}", process.pid),
                                        "Copy PID",
                                        on_copy_pid,
                                    ))
                                    .child(process_menu_item(
                                        format!("process-copy-cmd-{}", process.pid),
                                        "Copy Command",
                                        on_copy_command,
                                    ))
                                    .child(process_menu_sep())
                                    .child(process_menu_item(
                                        format!("process-term-{}", process.pid),
                                        "TERM",
                                        on_term,
                                    ))
                                    .child(process_menu_item(
                                        format!("process-hup-{}", process.pid),
                                        "HUP",
                                        on_hup,
                                    ))
                                    .child(process_menu_item(
                                        format!("process-stop-{}", process.pid),
                                        "STOP",
                                        on_stop,
                                    ))
                                    .child(process_menu_item(
                                        format!("process-cont-{}", process.pid),
                                        "CONT",
                                        on_cont,
                                    ))
                                    .child(process_menu_sep())
                                    .child(process_menu_item(
                                        format!("process-kill-{}", process.pid),
                                        "KILL",
                                        on_kill,
                                    )),
                            )
                        }),
                ),
        )
}

fn process_menu_item(
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(gpui::SharedString::from(id.into()))
        .h(px(24.))
        .px_3()
        .flex()
        .items_center()
        .text_size(px(12.))
        .text_color(rgb(0xc9d1d9))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)))
        .on_click(on_click)
        .child(label)
}

fn process_menu_sep() -> impl IntoElement {
    div().h(px(1.)).mx_2().my_1().bg(rgb(0x30363d))
}

pub(super) fn process_table_cell(
    value: String,
    color: Option<gpui::Hsla>,
    numeric: bool,
) -> impl IntoElement {
    // Tauri ProcessManager numeric columns are mono + right-aligned.
    div()
        .min_w_0()
        .font_family("JetBrains Mono")
        .text_xs()
        .when(numeric, |this| this.text_right())
        .text_color(color.unwrap_or_else(|| rgb(0xcbd5e1).into()))
        .overflow_hidden()
        .child(value)
}

pub(super) fn icon_action_button(
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(gpui::SharedString::from(id.into()))
        .h(px(24.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x303848))
        .bg(rgb(0x0d1320))
        .text_color(rgb(0xdbeafe))
        .text_xs()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x223047)))
        .child(label)
        .on_click(on_click)
}

pub(super) fn process_details(
    process: &RemoteProcess,
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
    div()
        .mx_2()
        .mb_1()
        .h(px(112.))
        .overflow_hidden()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x0d1117))
        .px_2()
        .py_2()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .font_family("JetBrains Mono")
                .text_size(px(11.))
                .line_height(px(15.))
                .text_color(rgb(0xc9d1d9))
                .child(truncate_preview(&command, 180)),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_1()
                .child(process_detail_chip("PPID", process.ppid.to_string()))
                .child(process_detail_chip(
                    "RSS",
                    format_file_size(Some(process.rss_kb.saturating_mul(1024))),
                ))
                .child(process_detail_chip("State", process.state.clone()))
                .child(process_detail_chip("User", process.user.clone()))
                .child(process_detail_chip("PID", process.pid.to_string()))
                .child(process_detail_chip("Elapsed", process.elapsed.clone())),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .flex_wrap()
                .child(
                    transfer_input("process-nice-input", "Nice", nice_draft, true)
                        .w(px(88.))
                        .h(px(26.))
                        .track_focus(nice_focus)
                        .on_click(cx.listener(|this, _, window, cx| {
                            window.focus(&this.process_nice_focus);
                            cx.notify();
                        }))
                        .on_key_down(cx.listener(
                            |this, event: &KeyDownEvent, window, cx| {
                                cx.stop_propagation();
                                this.handle_process_nice_key_down(event, window, cx);
                            },
                        )),
                )
                .child(small_button(
                    format!("process-nice-apply-{pid}"),
                    "Apply",
                    cx.listener(move |this, _, window, cx| {
                        this.apply_process_nice_draft(window, cx);
                    }),
                ))
                .child(small_button(
                    format!("process-nice-low-{pid}"),
                    "-5",
                    cx.listener(move |this, _, window, cx| {
                        this.renice_process(pid, -5, window, cx);
                    }),
                ))
                .child(small_button(
                    format!("process-nice-zero-{pid}"),
                    "0",
                    cx.listener(move |this, _, window, cx| {
                        this.renice_process(pid, 0, window, cx);
                    }),
                ))
                .child(small_button(
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
                        .text_color(rgb(0x6e7681))
                        .child("SIG"),
                )
                .child(small_button(
                    format!("process-signal-term-{pid}"),
                    "TERM",
                    cx.listener(move |this, _, window, cx| {
                        this.request_process_signal(pid, "TERM", window, cx);
                    }),
                ))
                .child(small_button(
                    format!("process-signal-hup-{pid}"),
                    "HUP",
                    cx.listener(move |this, _, window, cx| {
                        this.request_process_signal(pid, "HUP", window, cx);
                    }),
                ))
                .child(small_button(
                    format!("process-signal-stop-{pid}"),
                    "STOP",
                    cx.listener(move |this, _, window, cx| {
                        this.request_process_signal(pid, "STOP", window, cx);
                    }),
                ))
                .child(small_button(
                    format!("process-signal-cont-{pid}"),
                    "CONT",
                    cx.listener(move |this, _, window, cx| {
                        this.request_process_signal(pid, "CONT", window, cx);
                    }),
                ))
                .child(small_button(
                    format!("process-signal-kill-{pid}"),
                    "KILL",
                    cx.listener(move |this, _, window, cx| {
                        this.request_process_signal(pid, "KILL", window, cx);
                    }),
                )),
        )
        .into_any_element()
}

pub(super) fn process_detail_chip(label: &'static str, value: String) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x21262d))
        .bg(rgb(0x161b22))
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
                .text_color(rgb(0x6e7681))
                .child(label),
        )
        .child(
            div()
                .font_family("JetBrains Mono")
                .text_size(px(11.))
                .text_color(rgb(0xc9d1d9))
                .child(truncate_preview(&value, 24)),
        )
}

pub(super) fn process_signal_confirm_panel(
    confirm: RemoteProcessSignalConfirmState,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xfb7185))
        .bg(rgb(0x2a121a))
        .p_3()
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
                        .text_sm()
                        .font_weight(FontWeight(800.))
                        .text_color(rgb(0xfda4af))
                        .child(format!(
                            "Confirm {} for PID {}",
                            confirm.signal, confirm.pid
                        )),
                )
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_xs()
                        .text_color(rgb(0xfecdd3))
                        .child(format!(
                            "kill -{} -- {} · {}",
                            confirm.signal,
                            confirm.pid,
                            truncate_preview(&confirm.command, 96)
                        )),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(small_button(
                    "process-signal-cancel",
                    "Cancel",
                    cx.listener(|this, _, _, cx| {
                        this.cancel_process_signal_confirm(cx);
                    }),
                ))
                .child(small_button(
                    "process-signal-confirm",
                    "Confirm",
                    cx.listener(|this, _, window, cx| {
                        this.confirm_process_signal(window, cx);
                    }),
                )),
        )
}

pub(super) fn resource_gauge_card(
    title: &'static str,
    value: String,
    detail: String,
    ratio: f64,
) -> impl IntoElement {
    // Tauri ResourceMonitor ring-ish card: compact height, dense mono value.
    let ratio = ratio.clamp(0., 1.);
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x0d1117))
        .px_2()
        .py_2()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .size(px(44.))
                .rounded_full()
                .border_1()
                .border_color(usage_color(ratio))
                .bg(rgb(0x161b22))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_size(px(11.))
                        .font_weight(FontWeight(700.))
                        .text_color(usage_color(ratio))
                        .child(value),
                ),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight(600.))
                        .text_color(rgb(0xc9d1d9))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .line_height(px(13.))
                        .text_color(rgb(0x6e7681))
                        .child(truncate_preview(&detail, 48)),
                )
                .child(stats_progress_bar(ratio)),
        )
}

pub(super) fn resource_summary_card(
    title: &'static str,
    value: String,
    detail: String,
    ratio: f64,
) -> impl IntoElement {
    let ratio = ratio.clamp(0., 1.);
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x0d1117))
        .px_2()
        .py_2()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_size(px(10.))
                .font_weight(FontWeight(700.))
                .text_color(rgb(0x6e7681))
                .child(title),
        )
        .child(
            div()
                .font_family("JetBrains Mono")
                .text_size(px(12.))
                .font_weight(FontWeight(700.))
                .text_color(usage_color(ratio))
                .child(value),
        )
        .child(
            div()
                .text_size(px(10.))
                .line_height(px(13.))
                .text_color(rgb(0x8b949e))
                .child(truncate_preview(&detail, 56)),
        )
        .child(stats_progress_bar(ratio))
}

pub(super) fn usage_color(ratio: f64) -> gpui::Hsla {
    if ratio >= 0.9 {
        rgb(0xfb7185).into()
    } else if ratio >= 0.7 {
        rgb(0xfacc15).into()
    } else {
        rgb(0x38bdf8).into()
    }
}

pub(super) fn load_ratio(load1: f64, cores: u32) -> f64 {
    let cores = cores.max(1) as f64;
    (load1 / cores).clamp(0., 1.)
}


pub(super) fn compact_remote_svg_button(
    id: impl Into<String>,
    icon_path: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(gpui::SharedString::from(id.into()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(0x8b949e))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xc9d1d9)))
        .child(
            svg()
                .size(px(16.))
                .flex_none()
                .path(icon_path),
        )
        .on_click(on_click)
}
