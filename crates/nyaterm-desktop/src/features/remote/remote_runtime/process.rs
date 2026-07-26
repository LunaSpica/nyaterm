use super::*;

const PROCESS_EVENT_DRAIN_LIMIT: usize = 8;

impl NyaTermApp {
    pub(in crate::features) fn set_docker_tab(&mut self, tab: DockerTab, cx: &mut Context<Self>) {
        self.remote_ops.docker.container_menu_id = None;
        self.remote_ops.docker.compose_menu_id = None;
        self.remote_ops.docker.tab_menu_open = false;
        self.remote_ops.docker.header_menu_open = false;
        if tab == DockerTab::Compose
            && self
                .remote_ops
                .docker
                .overview
                .as_ref()
                .is_some_and(|overview| !overview.compose_available)
        {
            self.remote_ops.docker.status =
                "Docker Compose is not available on this host".to_string();
            cx.notify();
            return;
        }
        self.remote_ops.docker.tab = tab;
        self.remote_ops.docker.list_offset = 0;
        self.remote_ops.docker.resource_list_offset = 0;
        self.remote_ops.docker.status = format!("Docker tab: {}", tab.label());
        cx.notify();
    }

    pub(in crate::features) fn toggle_docker_tab_menu(&mut self, cx: &mut Context<Self>) {
        self.remote_ops.docker.tab_menu_open = !self.remote_ops.docker.tab_menu_open;
        cx.notify();
    }

    pub(in crate::features) fn handle_docker_search_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.remote_ops.docker.search_draft.pop();
                self.remote_ops.docker.list_offset = 0;
                self.remote_ops.docker.resource_list_offset = 0;
                self.remote_ops.docker.status = "Docker search updated".to_string();
                cx.notify();
            }
            "escape" => {
                self.remote_ops.docker.search_draft.clear();
                self.remote_ops.docker.list_offset = 0;
                self.remote_ops.docker.resource_list_offset = 0;
                self.remote_ops.docker.status = "Docker search cleared".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.remote_ops.docker.search_draft.push_str(input);
                    self.remote_ops.docker.list_offset = 0;
                    self.remote_ops.docker.resource_list_offset = 0;
                    self.remote_ops.docker.status = "Docker search updated".to_string();
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn handle_process_search_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.alt || keystroke.modifiers.function {
            return;
        }

        match keystroke.key.as_str() {
            "escape" => {
                self.remote_ops.process.search_draft.clear();
                self.remote_ops.process.selected_pid = None;
                self.remote_ops.process.list_offset = 0;
                self.remote_ops.process.status = "process search cleared".to_string();
                cx.notify();
            }
            "backspace" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                self.remote_ops.process.search_draft.pop();
                self.remote_ops.process.selected_pid = None;
                self.remote_ops.process.list_offset = 0;
                cx.notify();
            }
            _ if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.remote_ops.process.search_draft.push_str(input);
                    self.remote_ops.process.selected_pid = None;
                    self.remote_ops.process.list_offset = 0;
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    pub(in crate::features) fn toggle_process_sort(
        &mut self,
        key: RemoteProcessSortKey,
        cx: &mut Context<Self>,
    ) {
        if self.remote_ops.process.sort_key == key {
            self.remote_ops.process.sort_direction =
                self.remote_ops.process.sort_direction.reversed();
        } else {
            self.remote_ops.process.sort_key = key;
            self.remote_ops.process.sort_direction = match key {
                RemoteProcessSortKey::Cpu | RemoteProcessSortKey::Memory => {
                    RemoteProcessSortDirection::Descending
                }
                RemoteProcessSortKey::Pid
                | RemoteProcessSortKey::User
                | RemoteProcessSortKey::Command => RemoteProcessSortDirection::Ascending,
            };
        }
        self.remote_ops.process.list_offset = 0;
        self.remote_ops.process.status = format!(
            "sorted processes by {} {}",
            self.remote_ops.process.sort_key.label(),
            self.remote_ops.process.sort_direction.marker()
        );
        cx.notify();
    }

    pub(in crate::features) fn toggle_process_selection(
        &mut self,
        pid: u32,
        cx: &mut Context<Self>,
    ) {
        self.remote_ops.process.menu_pid = None;
        self.remote_ops.process.selected_pid = if self.remote_ops.process.selected_pid == Some(pid)
        {
            self.remote_ops.process.nice_draft = "0".to_string();
            None
        } else {
            self.remote_ops.process.nice_draft = "0".to_string();
            Some(pid)
        };
        cx.notify();
    }

    pub(in crate::features) fn handle_process_nice_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "enter" => {
                cx.stop_propagation();
                self.apply_process_nice_draft(window, cx);
            }
            "escape" => {
                cx.stop_propagation();
                self.remote_ops.process.nice_draft = "0".to_string();
                cx.notify();
            }
            "backspace" => {
                cx.stop_propagation();
                self.remote_ops.process.nice_draft.pop();
                cx.notify();
            }
            _ => {
                if keystroke.modifiers.shift {
                    return;
                }
                let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                else {
                    return;
                };
                if self.remote_ops.process.nice_draft.is_empty() && input == "-" {
                    cx.stop_propagation();
                    self.remote_ops.process.nice_draft.push('-');
                    cx.notify();
                    return;
                }
                let digits = input
                    .chars()
                    .filter(|character| character.is_ascii_digit())
                    .collect::<String>();
                if !digits.is_empty() && self.remote_ops.process.nice_draft.len() < 3 {
                    cx.stop_propagation();
                    self.remote_ops.process.nice_draft.push_str(&digits);
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn apply_process_nice_draft(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(pid) = self.remote_ops.process.selected_pid else {
            self.remote_ops.process.status = "select a process before applying nice".to_string();
            cx.notify();
            return;
        };
        let Ok(nice) = self.remote_ops.process.nice_draft.trim().parse::<i32>() else {
            self.remote_ops.process.status = "nice must be an integer from -20 to 19".to_string();
            cx.notify();
            return;
        };
        if !(-20..=19).contains(&nice) {
            self.remote_ops.process.status = "nice must be between -20 and 19".to_string();
            cx.notify();
            return;
        }
        self.renice_process(pid, nice, window, cx);
    }

    pub(in crate::features) fn copy_process_text(
        &mut self,
        value: String,
        label: &'static str,
        cx: &mut Context<Self>,
    ) {
        cx.write_to_clipboard(ClipboardItem::new_string(value));
        self.remote_ops.process.status = format!("copied process {label}");
        self.terminal_status = self.remote_ops.process.status.clone();
        cx.notify();
    }

    pub(in crate::features) fn copy_docker_text(
        &mut self,
        value: String,
        label: &'static str,
        cx: &mut Context<Self>,
    ) {
        cx.write_to_clipboard(ClipboardItem::new_string(value));
        self.remote_ops.docker.status = format!("copied Docker {label}");
        self.terminal_status = self.remote_ops.docker.status.clone();
        cx.notify();
    }

    pub(in crate::features) fn request_process_signal(
        &mut self,
        pid: u32,
        signal: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if signal == "KILL" {
            let command = self
                .remote_ops
                .process
                .items
                .iter()
                .find(|process| process.pid == pid)
                .map(|process| process.command_line.clone())
                .filter(|command| !command.trim().is_empty())
                .or_else(|| {
                    self.remote_ops
                        .process
                        .items
                        .iter()
                        .find(|process| process.pid == pid)
                        .map(|process| process.command.clone())
                })
                .unwrap_or_else(|| "unknown process".to_string());
            self.remote_ops.process.signal_confirm = Some(RemoteProcessSignalConfirmState {
                pid,
                signal,
                command,
            });
            self.remote_ops.process.status = format!("confirm {signal} for pid {pid}");
            cx.notify();
        } else {
            self.signal_process(pid, signal, window, cx);
        }
    }

    pub(in crate::features) fn cancel_process_signal_confirm(&mut self, cx: &mut Context<Self>) {
        self.remote_ops.process.signal_confirm = None;
        self.remote_ops.process.status = "process signal cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn confirm_process_signal(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(confirm) = self.remote_ops.process.signal_confirm.take() else {
            self.remote_ops.process.status = "no process signal pending".to_string();
            cx.notify();
            return;
        };
        self.signal_process(confirm.pid, confirm.signal, window, cx);
    }

    pub(in crate::features) fn refresh_processes(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.remote_ops.process.status =
                "start an SSH session before listing processes".to_string();
            self.terminal_status = self.remote_ops.process.status.clone();
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.active_session_id.clone() else {
            self.remote_ops.process.status =
                "start an SSH session before listing processes".to_string();
            cx.notify();
            return;
        };
        if self.remote_ops.process.pending
            && self.remote_ops.process.job_session_id.as_deref() == Some(job_session_id.as_str())
        {
            self.remote_ops.process.status = "process operation already running".to_string();
            cx.notify();
            return;
        }

        let job_id = self.begin_process_job(job_session_id.clone());
        self.remote_ops.process.menu_pid = None;
        self.remote_ops.process.last_refresh_at = Some(Instant::now());
        self.remote_ops.process.status = "listing remote processes".to_string();
        let tx = self.remote_ops.process.tx.clone();
        std::thread::spawn(move || {
            let result = SshProcessService::new(config)
                .list_processes()
                .map(ProcessJobOutput::Listed)
                .map_err(|error| error.to_string());
            let _ = tx.send(ProcessJobResult {
                job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn signal_process(
        &mut self,
        pid: u32,
        signal: &'static str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.remote_ops.process.status =
                "start an SSH session before signalling processes".to_string();
            self.terminal_status = self.remote_ops.process.status.clone();
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.active_session_id.clone() else {
            self.remote_ops.process.status =
                "start an SSH session before signalling processes".to_string();
            cx.notify();
            return;
        };
        if self.remote_ops.process.pending
            && self.remote_ops.process.job_session_id.as_deref() == Some(job_session_id.as_str())
        {
            self.remote_ops.process.status = "process operation already running".to_string();
            cx.notify();
            return;
        }

        let job_id = self.begin_process_job(job_session_id.clone());
        self.remote_ops.process.status = format!("sending {signal} to pid {pid}");
        let tx = self.remote_ops.process.tx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let service = SshProcessService::new(config);
                service.signal_process(pid, signal)?;
                let processes = service.list_processes()?;
                Ok(ProcessJobOutput::Signalled {
                    pid,
                    signal: signal.to_string(),
                    processes,
                })
            })()
            .map_err(|error: anyhow::Error| error.to_string());
            let _ = tx.send(ProcessJobResult {
                job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn renice_process(
        &mut self,
        pid: u32,
        nice: i32,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.remote_ops.process.status =
                "start an SSH session before renicing processes".to_string();
            self.terminal_status = self.remote_ops.process.status.clone();
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.active_session_id.clone() else {
            self.remote_ops.process.status =
                "start an SSH session before renicing processes".to_string();
            cx.notify();
            return;
        };
        if self.remote_ops.process.pending
            && self.remote_ops.process.job_session_id.as_deref() == Some(job_session_id.as_str())
        {
            self.remote_ops.process.status = "process operation already running".to_string();
            cx.notify();
            return;
        }

        let job_id = self.begin_process_job(job_session_id.clone());
        self.remote_ops.process.status = format!("renicing pid {pid} to {nice}");
        let tx = self.remote_ops.process.tx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let service = SshProcessService::new(config);
                service.renice_process(pid, nice)?;
                let processes = service.list_processes()?;
                Ok(ProcessJobOutput::Reniced {
                    pid,
                    nice,
                    processes,
                })
            })()
            .map_err(|error: anyhow::Error| error.to_string());
            let _ = tx.send(ProcessJobResult {
                job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn drain_process_events(&mut self) -> bool {
        let mut dirty = false;
        for _ in 0..PROCESS_EVENT_DRAIN_LIMIT {
            let Ok(event) = self.remote_ops.process.rx.try_recv() else {
                break;
            };
            if !remote_job_event_matches(
                self.remote_ops.process.job_id,
                self.remote_ops.process.job_session_id.as_deref(),
                event.job_id,
                &event.session_id,
            ) {
                continue;
            }
            dirty = true;
            self.remote_ops.process.pending = false;
            self.remote_ops.process.job_session_id = None;
            if self.active_session_id.as_deref() != Some(event.session_id.as_str()) {
                continue;
            }
            let was_list_refresh = self.remote_ops.process.status == "listing remote processes";
            match event.result {
                Ok(ProcessJobOutput::Listed(processes)) => {
                    self.remote_ops.process.consecutive_refresh_failures = 0;
                    self.remote_ops.process.status =
                        format!("loaded {} remote process(es)", processes.len());
                    self.terminal_status = self.remote_ops.process.status.clone();
                    self.apply_processes(processes);
                }
                Ok(ProcessJobOutput::Signalled {
                    pid,
                    signal,
                    processes,
                }) => {
                    self.remote_ops.process.status = format!("sent {signal} to pid {pid}");
                    self.terminal_status = self.remote_ops.process.status.clone();
                    self.remote_ops.process.signal_confirm = None;
                    self.apply_processes(processes);
                }
                Ok(ProcessJobOutput::Reniced {
                    pid,
                    nice,
                    processes,
                }) => {
                    self.remote_ops.process.status = format!("reniced pid {pid} to {nice}");
                    self.terminal_status = self.remote_ops.process.status.clone();
                    self.apply_processes(processes);
                }
                Err(error) => {
                    if was_list_refresh {
                        self.remote_ops.process.consecutive_refresh_failures =
                            if error.contains(nyaterm_transport::PROCESS_LIST_UNSUPPORTED_ERROR) {
                                3
                            } else {
                                self.remote_ops
                                    .process
                                    .consecutive_refresh_failures
                                    .saturating_add(1)
                            };
                        if self.remote_ops.process.consecutive_refresh_failures >= 3 {
                            self.remote_ops.process.items.clear();
                            self.remote_ops.process.snapshot_loaded = false;
                        }
                    }
                    self.remote_ops.process.status = format!("process operation failed: {error}");
                    self.terminal_status = self.remote_ops.process.status.clone();
                }
            }
        }
        dirty
    }

    fn begin_process_job(&mut self, session_id: String) -> u64 {
        self.remote_ops.process.job_id = self.remote_ops.process.job_id.wrapping_add(1).max(1);
        self.remote_ops.process.job_session_id = Some(session_id);
        self.remote_ops.process.pending = true;
        self.remote_ops.process.job_id
    }

    pub(in crate::features) fn apply_processes(&mut self, processes: Vec<RemoteProcess>) {
        if self
            .remote_ops
            .process
            .selected_pid
            .is_some_and(|pid| !processes.iter().any(|process| process.pid == pid))
        {
            self.remote_ops.process.selected_pid = None;
            self.remote_ops.process.nice_draft = "0".to_string();
        }
        self.remote_ops.process.items = processes;
        self.remote_ops.process.snapshot_loaded = true;
    }
}
