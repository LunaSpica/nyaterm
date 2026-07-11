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

        // Tauri ProcessManager shell: dense search toolbar + sort strip + scrollable table.
        let _ = (top_cpu, top_memory, user_count);
        let count_label = format!(
            "{}/{}",
            filtered_processes.len(),
            self.processes.len()
        );
        div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(rgb(0x161b22))
            .child(
                div()
                    .h(px(36.))
                    .flex_none()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x12171f))
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
                            .child(icon_button(
                                "process-refresh",
                                "↻",
                                cx.listener(|this, _, window, cx| {
                                    this.refresh_processes(window, cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x6e7681))
                            .child(count_label),
                    ),
            )
            .when_some(self.process_signal_confirm.clone(), |this, confirm| {
                this.child(process_signal_confirm_panel(confirm, cx))
            })
            .child(
                div()
                    .h(px(28.))
                    .flex_none()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x0d1117))
                    .flex()
                    .items_center()
                    .gap_1()
                    .overflow_hidden()
                    .child(process_sort_button(
                        "process-sort-command",
                        "Process",
                        self.process_sort_key == RemoteProcessSortKey::Command,
                        self.process_sort_direction,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_process_sort(RemoteProcessSortKey::Command, cx);
                        }),
                    ))
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
                        "Mem",
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
                        "Cmd",
                        self.process_sort_key == RemoteProcessSortKey::Command,
                        self.process_sort_direction,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_process_sort(RemoteProcessSortKey::Command, cx);
                        }),
                    )),
            )
            .child(
                div()
                    .id(SharedString::from("process-list-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_scroll()
                    .scrollbar_width(px(6.))
                    .flex()
                    .flex_col()
                    .child(process_table_header())
                    .child(rows),
            )
    }
}
