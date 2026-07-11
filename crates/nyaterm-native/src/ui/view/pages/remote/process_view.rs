use super::*;
use gpui::{SharedString, prelude::*};

impl NyaTermApp {
    pub(in crate::ui::view) fn processes_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let can_list = self.active_ssh_config.is_some() && !self.process_pending;
        let normalized_query = self.process_search_draft.trim().to_ascii_lowercase();
        let mut filtered_processes = self
            .processes
            .iter()
            .filter(|process| process_matches(process, &normalized_query))
            .cloned()
            .collect::<Vec<_>>();
        // Responsive mode first so hidden columns do not keep invalid sort keys.
        let mode = process_display_mode(self.right_panel_width);
        if mode != ProcessDisplayMode::Wide
            && self.process_sort_key == RemoteProcessSortKey::User
        {
            self.process_sort_key = RemoteProcessSortKey::Cpu;
        }
        if matches!(mode, ProcessDisplayMode::Compact | ProcessDisplayMode::Narrow)
            && self.process_sort_key == RemoteProcessSortKey::Memory
        {
            self.process_sort_key = RemoteProcessSortKey::Cpu;
        }
        sort_processes(
            &mut filtered_processes,
            self.process_sort_key,
            self.process_sort_direction,
        );

        // Tauri-like virtual list: base row + expanded details height, spacer padding.
        let process_row_px = process_row_height_px(mode);
        let process_details_px = process_details_height_px(mode);
        const PROCESS_VIEWPORT_ROWS: usize = 28;
        const PROCESS_OVERSCAN: usize = 8;
        let selected_pid = self.process_selected_pid;
        let row_height = |process: &RemoteProcess| -> f32 {
            if selected_pid == Some(process.pid) {
                process_row_px + process_details_px
            } else {
                process_row_px
            }
        };
        let total_filtered = filtered_processes.len();
        let window_capacity = PROCESS_VIEWPORT_ROWS + PROCESS_OVERSCAN * 2;
        let max_offset = total_filtered.saturating_sub(PROCESS_VIEWPORT_ROWS.min(total_filtered));
        if self.process_list_offset > max_offset {
            self.process_list_offset = max_offset;
        }
        let scroll_row = self.process_list_offset.min(max_offset);
        let window_start = scroll_row.saturating_sub(PROCESS_OVERSCAN);
        let window_end = (window_start + window_capacity).min(total_filtered);
        let visible_processes = filtered_processes
            .get(window_start..window_end)
            .unwrap_or(&[])
            .to_vec();
        let pad_top = filtered_processes
            .iter()
            .take(window_start)
            .map(row_height)
            .sum::<f32>();
        let pad_bottom = filtered_processes
            .iter()
            .skip(window_end)
            .map(row_height)
            .sum::<f32>();

        let top_cpu = self
            .processes
            .iter()
            .max_by(|left, right| {
                left.cpu_percent
                    .partial_cmp(&right.cpu_percent)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|process| format!("{:.1}% · {}", process.cpu_percent, process.command))
            .unwrap_or_else(|| "0.0%".to_string());
        let top_memory = self
            .processes
            .iter()
            .max_by(|left, right| {
                left.memory_percent
                    .partial_cmp(&right.memory_percent)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|process| format!("{:.1}% · {}", process.memory_percent, process.command))
            .unwrap_or_else(|| "0.0%".to_string());
        let user_count = self
            .processes
            .iter()
            .map(|process| process.user.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len();
        let selected_process = self
            .process_selected_pid
            .and_then(|pid| self.processes.iter().find(|process| process.pid == pid))
            .cloned();

        let mut rows = div().flex().flex_col();
        if self.processes.is_empty() {
            rows = rows.child(empty_panel(if self.active_ssh_config.is_some() {
                "No process snapshot loaded."
            } else {
                "Start an SSH session to list remote processes."
            }, self.theme_palette()));
        } else if filtered_processes.is_empty() {
            rows = rows.child(empty_panel("No processes match the current search.", self.theme_palette()));
        } else {
            if pad_top > 0. {
                rows = rows.child(
                    div()
                        .h(px(pad_top))
                        .w_full()
                        .flex_none(),
                );
            }
            for process in visible_processes.iter() {
                let pid = process.pid;
                let selected = self.process_selected_pid == Some(pid);
                rows = rows.child(
                    process_table_row(
                        process,
                        mode,
                        selected,
                        self.process_menu_pid == Some(pid),
                        cx.listener(move |this, _, _, cx| {
                            this.process_menu_pid = None;
                            this.toggle_process_selection(pid, cx);
                        }),
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            if this.process_menu_pid == Some(pid) {
                                this.process_menu_pid = None;
                            } else {
                                this.process_menu_pid = Some(pid);
                            }
                            cx.notify();
                        }),
                        cx.listener({
                            let value = pid.to_string();
                            move |this, _, _, cx| {
                                this.process_menu_pid = None;
                                this.copy_process_text(value.clone(), "pid", cx);
                            }
                        }),
                        cx.listener({
                            let value = if process.command_line.trim().is_empty() {
                                process.command.clone()
                            } else {
                                process.command_line.clone()
                            };
                            move |this, _, _, cx| {
                                this.process_menu_pid = None;
                                this.copy_process_text(value.clone(), "command", cx);
                            }
                        }),
                        cx.listener(move |this, _, window, cx| {
                            this.process_menu_pid = None;
                            this.request_process_signal(pid, "TERM", window, cx);
                        }),
                        cx.listener(move |this, _, window, cx| {
                            this.process_menu_pid = None;
                            this.request_process_signal(pid, "HUP", window, cx);
                        }),
                        cx.listener(move |this, _, window, cx| {
                            this.process_menu_pid = None;
                            this.request_process_signal(pid, "STOP", window, cx);
                        }),
                        cx.listener(move |this, _, window, cx| {
                            this.process_menu_pid = None;
                            this.request_process_signal(pid, "CONT", window, cx);
                        }),
                        cx.listener(move |this, _, window, cx| {
                            this.process_menu_pid = None;
                            this.request_process_signal(pid, "KILL", window, cx);
                        }),
                    )
                    .child(
                        selected_process
                            .as_ref()
                            .filter(|selected_process| selected_process.pid == pid)
                            .map(|selected_process| {
                                process_details(
                                    selected_process,
                                    mode,
                                    self.process_nice_draft.clone(),
                                    &self.process_nice_focus,
                                    cx,
                                )
                            })
                            .unwrap_or_else(|| div().into_any_element()),
                    ),
                );
            }
            if pad_bottom > 0. {
                rows = rows.child(
                    div()
                        .h(px(pad_bottom))
                        .w_full()
                        .flex_none(),
                );
            }
        }

        // Tauri ProcessManager shell: dense search toolbar + sort strip + scrollable table.
        let palette = self.theme_palette();
        let count_label = if total_filtered > PROCESS_VIEWPORT_ROWS {
            format!(
                "{window_start}-{window_end}/{total_filtered} · {} total · {} users",
                self.processes.len(),
                user_count
            )
        } else {
            format!(
                "{}/{} · {} users",
                filtered_processes.len(),
                self.processes.len(),
                user_count
            )
        };
        let top_label = format!("CPU {} · MEM {}", truncate_preview(&top_cpu, 28), truncate_preview(&top_memory, 28));
        div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(rgb(palette.surface))
            .child(
                div()
                    .h(px(36.))
                    .flex_none()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.section_header))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                transfer_input(
                                    "process-search-input",
                                    "Search processes…",
                                    self.process_search_draft.clone(),
                                    true,
                                )
                                .h(px(28.))
                                .track_focus(&self.process_search_focus)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    window.focus(&this.process_search_focus);
                                    cx.notify();
                                }))
                                .on_key_down(cx.listener(
                                    |this, event: &KeyDownEvent, _, cx| {
                                        cx.stop_propagation();
                                        this.handle_process_search_key_down(event, cx);
                                    },
                                )),
                            ),
                    )
                    .child(
                        div()
                            .when(!can_list, |this| this.opacity(0.45))
                            .child(compact_remote_svg_button(
                                "process-refresh",
                                "icons/fe/refresh.svg",
                                cx.listener(|this, _, window, cx| {
                                    this.refresh_processes(window, cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_end()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(0x8b949e))
                                    .child(count_label),
                            )
                            .child(
                                div()
                                    .max_w(px(220.))
                                    .font_family("JetBrains Mono")
                                    .text_size(px(10.))
                                    .text_color(rgb(0x6e7681))
                                    .overflow_hidden()
                                    .child(top_label),
                            ),
                    ),
            )
            .when_some(self.process_signal_confirm.clone(), |this, confirm| {
                this.child(process_signal_confirm_panel(confirm, cx))
            })
            .child(
                // Column header follows Tauri mode: hide Mem (narrow) / User (non-wide); compact label strip.
                div()
                    .when(mode != ProcessDisplayMode::Compact, |this| {
                        let cols = match mode {
                            ProcessDisplayMode::Narrow => 4,
                            ProcessDisplayMode::Medium => 5,
                            _ => 6,
                        };
                        this.h(px(26.))
                            .flex_none()
                            .px_2()
                            .border_b_1()
                            .border_color(rgb(0x30363d))
                            .bg(rgb(0x0d1117))
                            .grid()
                            .grid_cols(cols)
                            .gap_1()
                            .items_center()
                            .overflow_hidden()
                            .child(process_sort_button(
                                "process-sort-command",
                                "Process",
                                self.process_sort_key == RemoteProcessSortKey::Command,
                                self.process_sort_direction,
                                false,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_process_sort(RemoteProcessSortKey::Command, cx);
                                }),
                            ))
                            .child(process_sort_button(
                                "process-sort-pid",
                                "PID",
                                self.process_sort_key == RemoteProcessSortKey::Pid,
                                self.process_sort_direction,
                                true,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_process_sort(RemoteProcessSortKey::Pid, cx);
                                }),
                            ))
                            .child(process_sort_button(
                                "process-sort-cpu",
                                "CPU",
                                self.process_sort_key == RemoteProcessSortKey::Cpu,
                                self.process_sort_direction,
                                true,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_process_sort(RemoteProcessSortKey::Cpu, cx);
                                }),
                            ))
                            .when(
                                !matches!(
                                    mode,
                                    ProcessDisplayMode::Narrow | ProcessDisplayMode::Compact
                                ),
                                |this| {
                                    this.child(process_sort_button(
                                        "process-sort-memory",
                                        "Mem",
                                        self.process_sort_key == RemoteProcessSortKey::Memory,
                                        self.process_sort_direction,
                                        true,
                                        cx.listener(|this, _, _, cx| {
                                            this.toggle_process_sort(
                                                RemoteProcessSortKey::Memory,
                                                cx,
                                            );
                                        }),
                                    ))
                                },
                            )
                            .when(mode == ProcessDisplayMode::Wide, |this| {
                                this.child(process_sort_button(
                                    "process-sort-user",
                                    "User",
                                    self.process_sort_key == RemoteProcessSortKey::User,
                                    self.process_sort_direction,
                                    false,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_process_sort(RemoteProcessSortKey::User, cx);
                                    }),
                                ))
                            })
                            .child(div().w_full())
                    })
                    .when(mode == ProcessDisplayMode::Compact, |this| {
                        this.h(px(22.))
                            .flex_none()
                            .px_2()
                            .border_b_1()
                            .border_color(rgb(0x30363d))
                            .bg(rgb(0x0d1117))
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(0x6e7681))
                                    .child("Processes · compact"),
                            )
                    })
            )
            .child(
                div()
                    .id(SharedString::from("process-list-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .on_scroll_wheel(cx.listener(
                        move |this, event: &ScrollWheelEvent, _, cx| {
                            let max_offset = total_filtered
                                .saturating_sub(PROCESS_VIEWPORT_ROWS.min(total_filtered));
                            if max_offset == 0 {
                                return;
                            }
                            let delta_rows = match event.delta {
                                ScrollDelta::Lines(delta) => delta.y,
                                ScrollDelta::Pixels(delta) => {
                                    f32::from(delta.y) / process_row_px
                                }
                            };
                            // Match GPUI list semantics: scroll_top -= delta.y
                            let next = (this.process_list_offset as f32 - delta_rows)
                                .round()
                                .clamp(0., max_offset as f32)
                                as usize;
                            if next != this.process_list_offset {
                                this.process_list_offset = next;
                                cx.stop_propagation();
                                cx.notify();
                            }
                        },
                    ))
                    .child(rows),
            )
    }
}
