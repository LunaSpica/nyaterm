use super::*;

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
        sort_processes(
            &mut filtered_processes,
            self.process_sort_key,
            self.process_sort_direction,
        );

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
            }));
        } else if filtered_processes.is_empty() {
            rows = rows.child(empty_panel("No processes match the current search."));
        } else {
            for process in filtered_processes.iter() {
                let pid = process.pid;
                let selected = self.process_selected_pid == Some(pid);
                rows = rows.child(
                    process_table_row(
                        process,
                        selected,
                        cx.listener(move |this, _, _, cx| {
                            this.toggle_process_selection(pid, cx);
                        }),
                        cx.listener({
                            let value = pid.to_string();
                            move |this, _, _, cx| {
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
                                this.copy_process_text(value.clone(), "command", cx);
                            }
                        }),
                        cx.listener(move |this, _, window, cx| {
                            this.request_process_signal(pid, "TERM", window, cx);
                        }),
                        cx.listener(move |this, _, window, cx| {
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
                                    self.process_nice_draft.clone(),
                                    &self.process_nice_focus,
                                    cx,
                                )
                            })
                            .unwrap_or_else(|| div().into_any_element()),
                    ),
                );
            }
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_5()
            .gap_4()
            .child(section_header(
                "Processes",
                "Native SSH exec process inspector for the active remote session.",
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(4)
                    .gap_3()
                    .child(metric(
                        "SSH",
                        if self.active_ssh_config.is_some() {
                            "ready".to_string()
                        } else {
                            "none".to_string()
                        },
                    ))
                    .child(metric("Processes", self.processes.len().to_string()))
                    .child(metric("Visible", filtered_processes.len().to_string()))
                    .child(metric(
                        "Status",
                        if self.process_pending {
                            "running".to_string()
                        } else {
                            "idle".to_string()
                        },
                    )),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(4)
                    .gap_3()
                    .child(process_summary_card(
                        "Top CPU",
                        truncate_preview(&top_cpu, 44),
                        top_process_ratio(&self.processes, true),
                    ))
                    .child(process_summary_card(
                        "Top Memory",
                        truncate_preview(&top_memory, 44),
                        top_process_ratio(&self.processes, false),
                    ))
                    .child(process_summary_card(
                        "Users",
                        format!("{user_count} owner(s)"),
                        (user_count as f64 / 12.).clamp(0., 1.),
                    ))
                    .child(process_summary_card(
                        "Sort",
                        format!(
                            "{} {}",
                            self.process_sort_key.label(),
                            self.process_sort_direction.marker()
                        ),
                        filtered_processes.len() as f64 / self.processes.len().max(1) as f64,
                    )),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xe5edf7))
                                    .child(self.process_status.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .when(!can_list, |this| this.opacity(0.45))
                                    .child(small_button(
                                        "process-refresh",
                                        "Refresh",
                                        cx.listener(|this, _, window, cx| {
                                            this.refresh_processes(window, cx);
                                        }),
                                    )),
                            ),
                    ),
            )
            .when_some(self.process_signal_confirm.clone(), |this, confirm| {
                this.child(process_signal_confirm_panel(confirm, cx))
            })
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                transfer_input(
                                    "process-search-input",
                                    "Search",
                                    self.process_search_draft.clone(),
                                    true,
                                )
                                .flex_1()
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
                            )
                            .child(status_pill("filtered", rgb(0x93c5fd), rgb(0x17233a)))
                            .child(
                                div()
                                    .font_family("JetBrains Mono")
                                    .text_xs()
                                    .text_color(rgb(0x98a3b8))
                                    .child(format!(
                                        "{} / {}",
                                        filtered_processes.len(),
                                        self.processes.len()
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .flex_wrap()
                            .child(process_sort_button(
                                "process-sort-cpu",
                                "CPU",
                                self.process_sort_key == RemoteProcessSortKey::Cpu,
                                self.process_sort_direction,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_process_sort(RemoteProcessSortKey::Cpu, cx);
                                }),
                            ))
                            .child(process_sort_button(
                                "process-sort-memory",
                                "Memory",
                                self.process_sort_key == RemoteProcessSortKey::Memory,
                                self.process_sort_direction,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_process_sort(RemoteProcessSortKey::Memory, cx);
                                }),
                            ))
                            .child(process_sort_button(
                                "process-sort-pid",
                                "PID",
                                self.process_sort_key == RemoteProcessSortKey::Pid,
                                self.process_sort_direction,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_process_sort(RemoteProcessSortKey::Pid, cx);
                                }),
                            ))
                            .child(process_sort_button(
                                "process-sort-user",
                                "User",
                                self.process_sort_key == RemoteProcessSortKey::User,
                                self.process_sort_direction,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_process_sort(RemoteProcessSortKey::User, cx);
                                }),
                            ))
                            .child(process_sort_button(
                                "process-sort-command",
                                "Command",
                                self.process_sort_key == RemoteProcessSortKey::Command,
                                self.process_sort_direction,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_process_sort(RemoteProcessSortKey::Command, cx);
                                }),
                            )),
                    )
                    .child(process_table_header())
                    .child(rows),
            )
    }
}
