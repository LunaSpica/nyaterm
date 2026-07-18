use super::*;
use gpui::SharedString;

impl NyaTermApp {
    pub(in crate::features) fn processes_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let table_labels = ProcessTableLabels {
            process: self.tr("processManager.process"),
            pid: self.tr("processManager.sortPid"),
            cpu: self.tr("processManager.sortCpu"),
            memory: self.tr("processManager.sortMemory"),
            user: self.tr("processManager.user"),
            copy_pid: self.tr("processManager.copyPid"),
            copy_command: self.tr("processManager.copyCommand"),
            signal_term: self.tr("processManager.signalTerm"),
            signal_hup: self.tr("processManager.signalHup"),
            signal_stop: self.tr("processManager.signalStop"),
            signal_cont: self.tr("processManager.signalCont"),
            signal_kill: self.tr("processManager.signalKill"),
        };
        let detail_labels = ProcessDetailLabels {
            cpu: self.tr("processManager.sortCpu"),
            memory: self.tr("resourceMonitor.memory"),
            rss: "RSS",
            elapsed: self.tr("processManager.elapsed"),
            copy_command: self.tr("processManager.copyCommand"),
            nice_value: self.tr("processManager.niceValue"),
            apply_nice: self.tr("processManager.applyNice"),
        };
        let signal_labels = ProcessSignalLabels {
            title: self.tr("processManager.confirmSignalTitle"),
            description: self.tr("processManager.confirmSignalDesc"),
            cancel: self.tr("common.cancel"),
            confirm: self.tr("common.confirm"),
        };
        let normalized_query = self.process_search_draft.trim().to_ascii_lowercase();
        let mut filtered_processes = self
            .processes
            .iter()
            .filter(|process| process_matches(process, &normalized_query))
            .cloned()
            .collect::<Vec<_>>();
        // Responsive mode first so hidden columns do not keep invalid sort keys.
        let mode = process_display_mode(self.right_panel_width);
        if mode != ProcessDisplayMode::Wide && self.process_sort_key == RemoteProcessSortKey::User {
            self.process_sort_key = RemoteProcessSortKey::Cpu;
        }
        if matches!(
            mode,
            ProcessDisplayMode::Compact | ProcessDisplayMode::Narrow
        ) && self.process_sort_key == RemoteProcessSortKey::Memory
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
            rows = rows.child(empty_panel(
                if self.active_ssh_config.is_some() {
                    self.tr("processManager.error")
                } else {
                    self.tr("processManager.noSession")
                },
                self.theme_palette(),
            ));
        } else if filtered_processes.is_empty() {
            rows = rows.child(empty_panel(
                self.tr("processManager.noMatches"),
                self.theme_palette(),
            ));
        } else {
            if pad_top > 0. {
                rows = rows.child(div().h(px(pad_top)).w_full().flex_none());
            }
            for process in visible_processes.iter() {
                let pid = process.pid;
                let selected = self.process_selected_pid == Some(pid);
                rows = rows.child(
                    process_table_row(
                        palette,
                        process,
                        mode,
                        table_labels,
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
                                    palette,
                                    selected_process,
                                    mode,
                                    detail_labels,
                                    self.process_nice_draft.clone(),
                                    &self.process_nice_focus,
                                    cx.listener({
                                        let value =
                                            if selected_process.command_line.trim().is_empty() {
                                                selected_process.command.clone()
                                            } else {
                                                selected_process.command_line.clone()
                                            };
                                        move |this, _, _, cx| {
                                            this.copy_process_text(value.clone(), "command", cx);
                                        }
                                    }),
                                    cx,
                                )
                            })
                            .unwrap_or_else(|| div().into_any_element()),
                    ),
                );
            }
            if pad_bottom > 0. {
                rows = rows.child(div().h(px(pad_bottom)).w_full().flex_none());
            }
        }

        // Tauri ProcessManager shell: dense search toolbar + sort strip + scrollable table.
        let palette = self.theme_palette();
        let count_label = if total_filtered > PROCESS_VIEWPORT_ROWS {
            format!(
                "{window_start}-{window_end}/{total_filtered} · {} {} · {} {}",
                self.processes.len(),
                self.tr("processManager.total"),
                user_count,
                self.tr("processManager.users")
            )
        } else {
            format!(
                "{}/{} · {} {}",
                filtered_processes.len(),
                self.processes.len(),
                user_count,
                self.tr("processManager.users")
            )
        };
        let top_label = format!(
            "CPU {} · MEM {}",
            truncate_preview(&top_cpu, 28),
            truncate_preview(&top_memory, 28)
        );
        div()
            .flex()
            .flex_col()
            .size_full()
            .relative()
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
                        div().flex_1().min_w_0().child(
                            transfer_input(
                                "process-search-input",
                                self.tr("processManager.search"),
                                self.process_search_draft.clone(),
                                true,
                                self.theme_palette(),
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
                            .flex()
                            .flex_col()
                            .items_end()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(count_label),
                            )
                            .child(
                                div()
                                    .max_w(px(220.))
                                    .font_family(crate::features::gpui_code_font_family())
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text_dimmed))
                                    .overflow_hidden()
                                    .child(top_label),
                            ),
                    ),
            )
            .child(
                // Match Tauri's responsive columns and hide the header entirely in compact mode.
                div().when(mode != ProcessDisplayMode::Compact, |this| {
                    let cols = match mode {
                        ProcessDisplayMode::Narrow => 4,
                        ProcessDisplayMode::Medium => 5,
                        _ => 6,
                    };
                    this.h(px(26.))
                        .flex_none()
                        .px_2()
                        .border_b_1()
                        .border_color(rgb(palette.border))
                        .bg(rgb(palette.input))
                        .grid()
                        .grid_cols(cols)
                        .gap_1()
                        .items_center()
                        .overflow_hidden()
                        .child(process_sort_button(
                            palette,
                            "process-sort-command",
                            self.tr("processManager.process"),
                            self.process_sort_key == RemoteProcessSortKey::Command,
                            self.process_sort_direction,
                            false,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_process_sort(RemoteProcessSortKey::Command, cx);
                            }),
                        ))
                        .child(process_sort_button(
                            palette,
                            "process-sort-pid",
                            self.tr("processManager.sortPid"),
                            self.process_sort_key == RemoteProcessSortKey::Pid,
                            self.process_sort_direction,
                            true,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_process_sort(RemoteProcessSortKey::Pid, cx);
                            }),
                        ))
                        .child(process_sort_button(
                            palette,
                            "process-sort-cpu",
                            self.tr("processManager.sortCpu"),
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
                                    palette,
                                    "process-sort-memory",
                                    self.tr("processManager.sortMemory"),
                                    self.process_sort_key == RemoteProcessSortKey::Memory,
                                    self.process_sort_direction,
                                    true,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_process_sort(RemoteProcessSortKey::Memory, cx);
                                    }),
                                ))
                            },
                        )
                        .when(mode == ProcessDisplayMode::Wide, |this| {
                            this.child(process_sort_button(
                                palette,
                                "process-sort-user",
                                self.tr("processManager.user"),
                                self.process_sort_key == RemoteProcessSortKey::User,
                                self.process_sort_direction,
                                false,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_process_sort(RemoteProcessSortKey::User, cx);
                                }),
                            ))
                        })
                        .child(div().w_full())
                }),
            )
            .child(
                div()
                    .id(SharedString::from("process-list-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .on_scroll_wheel(cx.listener(move |this, event: &ScrollWheelEvent, _, cx| {
                        let max_offset = total_filtered
                            .saturating_sub(PROCESS_VIEWPORT_ROWS.min(total_filtered));
                        if max_offset == 0 {
                            return;
                        }
                        let delta_rows = match event.delta {
                            ScrollDelta::Lines(delta) => delta.y,
                            ScrollDelta::Pixels(delta) => f32::from(delta.y) / process_row_px,
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
                    }))
                    .child(rows),
            )
            .when_some(self.process_signal_confirm.clone(), |this, confirm| {
                this.child(process_signal_confirm_panel(
                    palette,
                    confirm,
                    signal_labels,
                    cx,
                ))
            })
    }
}
