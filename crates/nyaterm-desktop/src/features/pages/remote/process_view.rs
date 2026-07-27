use super::*;
use gpui::{SharedString, rgba};

impl NyaTermApp {
    pub(in crate::features) fn processes_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Built before the view, which reads `self` throughout: creating the
        // box needs it mutably.
        let process_search_input = self
            .text_input_box(
                "remote.process.filter",
                &self.remote_ops.process.search_draft.clone(),
                TextInputSetup::placeholder(self.tr("processManager.search")),
                cx,
            )
            .into_any_element();
        if self.active_ssh_config.is_none() {
            return div()
                .size_full()
                .bg(self.shell_transparent_color(palette.surface))
                .child(empty_panel(self.tr("processManager.noSession"), palette));
        }
        if !self.remote_ops.process.snapshot_loaded {
            let message = if self.remote_ops.process.pending
                || !self.remote_ops.process.status.contains("failed")
            {
                self.tr("common.loading")
            } else if self
                .remote_ops
                .process
                .status
                .contains(nyaterm_transport::PROCESS_LIST_UNSUPPORTED_ERROR)
            {
                self.tr("processManager.unsupported")
            } else {
                self.tr("processManager.error")
            };
            return div()
                .size_full()
                .bg(self.shell_transparent_color(palette.surface))
                .child(empty_panel(message, palette));
        }
        let menu_bg = self.shell_surface_color(palette.surface);
        let dialog_bg = self.shell_surface_color(palette.bg);
        let table_labels = ProcessTableLabels {
            more: self.tr("common.more"),
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
        let normalized_query = self
            .remote_ops
            .process
            .search_draft
            .trim()
            .to_ascii_lowercase();
        let mut filtered_processes = self
            .remote_ops
            .process
            .items
            .iter()
            .filter(|process| process_matches(process, &normalized_query))
            .cloned()
            .collect::<Vec<_>>();
        // Responsive mode first so hidden columns do not keep invalid sort keys.
        let mode = process_display_mode(self.right_panel_width);
        if mode != ProcessDisplayMode::Wide
            && self.remote_ops.process.sort_key == RemoteProcessSortKey::User
        {
            self.remote_ops.process.sort_key = RemoteProcessSortKey::Cpu;
        }
        if matches!(
            mode,
            ProcessDisplayMode::Compact | ProcessDisplayMode::Narrow
        ) && self.remote_ops.process.sort_key == RemoteProcessSortKey::Memory
        {
            self.remote_ops.process.sort_key = RemoteProcessSortKey::Cpu;
        }
        sort_processes(
            &mut filtered_processes,
            self.remote_ops.process.sort_key,
            self.remote_ops.process.sort_direction,
        );

        // Tauri-like virtual list: base row + expanded details height, spacer padding.
        let process_row_px = process_row_height_px(mode);
        let process_details_px = process_details_height_px(mode);
        const PROCESS_VIEWPORT_ROWS: usize = 28;
        const PROCESS_OVERSCAN: usize = 8;
        let selected_pid = self.remote_ops.process.selected_pid;
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
        if self.remote_ops.process.list_offset > max_offset {
            self.remote_ops.process.list_offset = max_offset;
        }
        let scroll_row = self.remote_ops.process.list_offset.min(max_offset);
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

        let selected_process = self
            .remote_ops
            .process
            .selected_pid
            .and_then(|pid| {
                self.remote_ops
                    .process
                    .items
                    .iter()
                    .find(|process| process.pid == pid)
            })
            .cloned();

        let mut rows = div().flex().flex_col();
        if filtered_processes.is_empty() {
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
                let selected = self.remote_ops.process.selected_pid == Some(pid);
                rows = rows.child(
                    process_table_row(
                        palette,
                        menu_bg,
                        process,
                        mode,
                        table_labels,
                        selected,
                        self.remote_ops.process.menu_pid == Some(pid),
                        cx.listener(move |this, _, _, cx| {
                            this.remote_ops.process.menu_pid = None;
                            this.toggle_process_selection(pid, cx);
                        }),
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            if this.remote_ops.process.menu_pid == Some(pid) {
                                this.remote_ops.process.menu_pid = None;
                            } else {
                                this.remote_ops.process.menu_pid = Some(pid);
                            }
                            cx.notify();
                        }),
                        cx.listener({
                            let value = pid.to_string();
                            move |this, _, _, cx| {
                                this.remote_ops.process.menu_pid = None;
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
                                this.remote_ops.process.menu_pid = None;
                                this.copy_process_text(value.clone(), "command", cx);
                            }
                        }),
                        cx.listener(move |this, _, window, cx| {
                            this.remote_ops.process.menu_pid = None;
                            this.request_process_signal(pid, "TERM", window, cx);
                        }),
                        cx.listener(move |this, _, window, cx| {
                            this.remote_ops.process.menu_pid = None;
                            this.request_process_signal(pid, "HUP", window, cx);
                        }),
                        cx.listener(move |this, _, window, cx| {
                            this.remote_ops.process.menu_pid = None;
                            this.request_process_signal(pid, "STOP", window, cx);
                        }),
                        cx.listener(move |this, _, window, cx| {
                            this.remote_ops.process.menu_pid = None;
                            this.request_process_signal(pid, "CONT", window, cx);
                        }),
                        cx.listener(move |this, _, window, cx| {
                            this.remote_ops.process.menu_pid = None;
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
                                    self.remote_ops.process.nice_draft.clone(),
                                    &self.remote_ops.process.nice_focus,
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
        let count_label = self.remote_ops.process.items.len().to_string();
        div()
            .flex()
            .flex_col()
            .size_full()
            .relative()
            .overflow_hidden()
            .p(px(10.))
            .gap(px(10.))
            .bg(self.shell_transparent_color(palette.surface))
            .child(
                div()
                    .h(px(32.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_1().min_w_0().child(process_search_input))
                    .child(
                        div()
                            .h(px(32.))
                            .px_2()
                            .rounded_md()
                            .border_1()
                            .border_color(rgba((palette.link << 8) | 0x4d))
                            .bg(rgba((palette.link << 8) | 0x1a))
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.link))
                                    .child(count_label),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .flex()
                    .flex_col()
                    .child(
                        // Match Tauri's responsive columns and hide the header entirely in compact mode.
                        div().when(mode != ProcessDisplayMode::Compact, |this| {
                            let cols = match mode {
                                ProcessDisplayMode::Narrow => 4,
                                ProcessDisplayMode::Medium => 5,
                                _ => 6,
                            };
                            this.h(px(32.))
                                .flex_none()
                                .px_2()
                                .border_b_1()
                                .border_color(rgb(palette.border))
                                .grid()
                                .grid_cols(cols)
                                .gap_1()
                                .items_center()
                                .overflow_hidden()
                                .child(process_sort_button(
                                    palette,
                                    "process-sort-command",
                                    self.tr("processManager.process"),
                                    self.remote_ops.process.sort_key
                                        == RemoteProcessSortKey::Command,
                                    self.remote_ops.process.sort_direction,
                                    false,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_process_sort(RemoteProcessSortKey::Command, cx);
                                    }),
                                ))
                                .child(process_sort_button(
                                    palette,
                                    "process-sort-pid",
                                    self.tr("processManager.sortPid"),
                                    self.remote_ops.process.sort_key == RemoteProcessSortKey::Pid,
                                    self.remote_ops.process.sort_direction,
                                    true,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_process_sort(RemoteProcessSortKey::Pid, cx);
                                    }),
                                ))
                                .child(process_sort_button(
                                    palette,
                                    "process-sort-cpu",
                                    self.tr("processManager.sortCpu"),
                                    self.remote_ops.process.sort_key == RemoteProcessSortKey::Cpu,
                                    self.remote_ops.process.sort_direction,
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
                                            self.remote_ops.process.sort_key
                                                == RemoteProcessSortKey::Memory,
                                            self.remote_ops.process.sort_direction,
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
                                        palette,
                                        "process-sort-user",
                                        self.tr("processManager.user"),
                                        self.remote_ops.process.sort_key
                                            == RemoteProcessSortKey::User,
                                        self.remote_ops.process.sort_direction,
                                        false,
                                        cx.listener(|this, _, _, cx| {
                                            this.toggle_process_sort(
                                                RemoteProcessSortKey::User,
                                                cx,
                                            );
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
                                    let next = (this.remote_ops.process.list_offset as f32
                                        - delta_rows)
                                        .round()
                                        .clamp(0., max_offset as f32)
                                        as usize;
                                    if next != this.remote_ops.process.list_offset {
                                        this.remote_ops.process.list_offset = next;
                                        cx.stop_propagation();
                                        cx.notify();
                                    }
                                },
                            ))
                            .child(rows),
                    ),
            )
            .when_some(
                self.remote_ops.process.signal_confirm.clone(),
                |this, confirm| {
                    this.child(process_signal_confirm_panel(
                        palette,
                        dialog_bg,
                        confirm,
                        signal_labels,
                        cx,
                    ))
                },
            )
    }
}
