use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn set_docker_tab(&mut self, tab: DockerTab, cx: &mut Context<Self>) {
        self.docker_container_menu_id = None;
        self.docker_compose_menu_id = None;
        if tab == DockerTab::Compose
            && self
                .docker_overview
                .as_ref()
                .is_some_and(|overview| !overview.compose_available)
        {
            self.docker_status = "Docker Compose is not available on this host".to_string();
            cx.notify();
            return;
        }
        self.docker_tab = tab;
        self.docker_list_offset = 0;
        self.docker_resource_list_offset = 0;
        self.docker_status = format!("Docker tab: {}", tab.label());
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_docker_search_key_down(
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
                self.docker_search_draft.pop();
                self.docker_list_offset = 0;
                self.docker_resource_list_offset = 0;
                self.docker_status = "Docker search updated".to_string();
                cx.notify();
            }
            "escape" => {
                self.docker_search_draft.clear();
                self.docker_list_offset = 0;
                self.docker_resource_list_offset = 0;
                self.docker_status = "Docker search cleared".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.docker_search_draft.push_str(input);
                    self.docker_list_offset = 0;
                    self.docker_resource_list_offset = 0;
                    self.docker_status = "Docker search updated".to_string();
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::ui::view) fn handle_process_search_key_down(
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
                self.process_search_draft.clear();
                self.process_selected_pid = None;
                self.process_list_offset = 0;
                self.process_status = "process search cleared".to_string();
                cx.notify();
            }
            "backspace" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                self.process_search_draft.pop();
                self.process_selected_pid = None;
                self.process_list_offset = 0;
                cx.notify();
            }
            _ if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.process_search_draft.push_str(input);
                    self.process_selected_pid = None;
                    self.process_list_offset = 0;
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    pub(in crate::ui::view) fn toggle_process_sort(
        &mut self,
        key: RemoteProcessSortKey,
        cx: &mut Context<Self>,
    ) {
        if self.process_sort_key == key {
            self.process_sort_direction = self.process_sort_direction.reversed();
        } else {
            self.process_sort_key = key;
            self.process_sort_direction = match key {
                RemoteProcessSortKey::Cpu | RemoteProcessSortKey::Memory => {
                    RemoteProcessSortDirection::Descending
                }
                RemoteProcessSortKey::Pid
                | RemoteProcessSortKey::User
                | RemoteProcessSortKey::Command => RemoteProcessSortDirection::Ascending,
            };
        }
        self.process_list_offset = 0;
        self.process_status = format!(
            "sorted processes by {} {}",
            self.process_sort_key.label(),
            self.process_sort_direction.marker()
        );
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_process_selection(
        &mut self,
        pid: u32,
        cx: &mut Context<Self>,
    ) {
        self.process_menu_pid = None;
        self.process_selected_pid = if self.process_selected_pid == Some(pid) {
            self.process_nice_draft = "0".to_string();
            None
        } else {
            self.process_nice_draft = "0".to_string();
            Some(pid)
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_process_nice_key_down(
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
                self.process_nice_draft = "0".to_string();
                cx.notify();
            }
            "backspace" => {
                cx.stop_propagation();
                self.process_nice_draft.pop();
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
                if self.process_nice_draft.is_empty() && input == "-" {
                    cx.stop_propagation();
                    self.process_nice_draft.push('-');
                    cx.notify();
                    return;
                }
                let digits = input
                    .chars()
                    .filter(|character| character.is_ascii_digit())
                    .collect::<String>();
                if !digits.is_empty() && self.process_nice_draft.len() < 3 {
                    cx.stop_propagation();
                    self.process_nice_draft.push_str(&digits);
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::ui::view) fn apply_process_nice_draft(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(pid) = self.process_selected_pid else {
            self.process_status = "select a process before applying nice".to_string();
            cx.notify();
            return;
        };
        let Ok(nice) = self.process_nice_draft.trim().parse::<i32>() else {
            self.process_status = "nice must be an integer from -20 to 19".to_string();
            cx.notify();
            return;
        };
        if !(-20..=19).contains(&nice) {
            self.process_status = "nice must be between -20 and 19".to_string();
            cx.notify();
            return;
        }
        self.renice_process(pid, nice, window, cx);
    }

    pub(in crate::ui::view) fn copy_process_text(
        &mut self,
        value: String,
        label: &'static str,
        cx: &mut Context<Self>,
    ) {
        cx.write_to_clipboard(ClipboardItem::new_string(value));
        self.process_status = format!("copied process {label}");
        self.terminal_status = self.process_status.clone();
        cx.notify();
    }

    pub(in crate::ui::view) fn copy_docker_text(
        &mut self,
        value: String,
        label: &'static str,
        cx: &mut Context<Self>,
    ) {
        cx.write_to_clipboard(ClipboardItem::new_string(value));
        self.docker_status = format!("copied Docker {label}");
        self.terminal_status = self.docker_status.clone();
        cx.notify();
    }

    pub(in crate::ui::view) fn request_process_signal(
        &mut self,
        pid: u32,
        signal: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(signal, "KILL" | "STOP") {
            let command = self
                .processes
                .iter()
                .find(|process| process.pid == pid)
                .map(|process| process.command_line.clone())
                .filter(|command| !command.trim().is_empty())
                .or_else(|| {
                    self.processes
                        .iter()
                        .find(|process| process.pid == pid)
                        .map(|process| process.command.clone())
                })
                .unwrap_or_else(|| "unknown process".to_string());
            self.process_signal_confirm = Some(RemoteProcessSignalConfirmState {
                pid,
                signal,
                command,
            });
            self.process_status = format!("confirm {signal} for pid {pid}");
            cx.notify();
        } else {
            self.signal_process(pid, signal, window, cx);
        }
    }

    pub(in crate::ui::view) fn cancel_process_signal_confirm(&mut self, cx: &mut Context<Self>) {
        self.process_signal_confirm = None;
        self.process_status = "process signal cancelled".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn confirm_process_signal(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(confirm) = self.process_signal_confirm.take() else {
            self.process_status = "no process signal pending".to_string();
            cx.notify();
            return;
        };
        self.signal_process(confirm.pid, confirm.signal, window, cx);
    }

    pub(in crate::ui::view) fn refresh_processes(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.process_status = "start an SSH session before listing processes".to_string();
            self.terminal_status = self.process_status.clone();
            cx.notify();
            return;
        };
        if self.process_pending {
            self.process_status = "process operation already running".to_string();
            cx.notify();
            return;
        }

        self.process_pending = true;
        self.process_menu_pid = None;
        self.process_last_refresh_at = Some(Instant::now());
        self.process_status = "listing remote processes".to_string();
        self.ensure_event_pump(window, cx);
        let tx = self.process_tx.clone();
        std::thread::spawn(move || {
            let result = SshProcessService::new(config)
                .list_processes()
                .map(ProcessJobOutput::Listed)
                .map_err(|error| error.to_string());
            let _ = tx.send(ProcessJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn signal_process(
        &mut self,
        pid: u32,
        signal: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.process_status = "start an SSH session before signalling processes".to_string();
            self.terminal_status = self.process_status.clone();
            cx.notify();
            return;
        };
        if self.process_pending {
            self.process_status = "process operation already running".to_string();
            cx.notify();
            return;
        }

        self.process_pending = true;
        self.process_status = format!("sending {signal} to pid {pid}");
        self.ensure_event_pump(window, cx);
        let tx = self.process_tx.clone();
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
            let _ = tx.send(ProcessJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn renice_process(
        &mut self,
        pid: u32,
        nice: i32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.process_status = "start an SSH session before renicing processes".to_string();
            self.terminal_status = self.process_status.clone();
            cx.notify();
            return;
        };
        if self.process_pending {
            self.process_status = "process operation already running".to_string();
            cx.notify();
            return;
        }

        self.process_pending = true;
        self.process_status = format!("renicing pid {pid} to {nice}");
        self.ensure_event_pump(window, cx);
        let tx = self.process_tx.clone();
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
            let _ = tx.send(ProcessJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn refresh_stats(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.stats_status = "start an SSH session before inspecting stats".to_string();
            self.terminal_status = self.stats_status.clone();
            cx.notify();
            return;
        };
        if self.stats_pending {
            self.stats_status = "stats refresh already running".to_string();
            cx.notify();
            return;
        }

        self.stats_pending = true;
        self.stats_last_refresh_at = Some(Instant::now());
        self.stats_status = "loading remote system stats".to_string();
        self.ensure_event_pump(window, cx);
        let tx = self.stats_tx.clone();
        std::thread::spawn(move || {
            let result = RemoteStatsService::new(config)
                .snapshot()
                .map_err(|error| error.to_string());
            let _ = tx.send(StatsJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_stats_cpu_expanded(&mut self, cx: &mut Context<Self>) {
        self.stats_cpu_expanded = !self.stats_cpu_expanded;
        self.stats_status = if self.stats_cpu_expanded {
            "showing per-core CPU usage".to_string()
        } else {
            "collapsed per-core CPU usage".to_string()
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn refresh_docker(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status = "start an SSH session before inspecting Docker".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        self.docker_pending = true;
        self.docker_last_refresh_at = Some(Instant::now());
        self.docker_status = "loading Docker overview".to_string();
        self.ensure_event_pump(window, cx);
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = DockerService::new(config)
                .overview()
                .map(DockerJobOutput::Overview)
                .map_err(|error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn docker_container_action(
        &mut self,
        container_id: String,
        action: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status = "start an SSH session before changing containers".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        self.docker_pending = true;
        self.docker_status = format!("Docker {action} {}", compact_id(&container_id));
        self.docker_details = None;
        self.docker_details_container_id = None;
        self.ensure_event_pump(window, cx);
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let service = DockerService::new(config);
                service.container_action(&container_id, action)?;
                let overview = service.overview()?;
                Ok(DockerJobOutput::RefreshedAfterAction {
                    label: format!("Docker {action} {}", compact_id(&container_id)),
                    overview,
                })
            })()
            .map_err(|error: anyhow::Error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn load_docker_details(
        &mut self,
        container_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status = "start an SSH session before reading Docker details".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        self.docker_pending = true;
        self.docker_details_container_id = Some(container_id.clone());
        self.docker_details_last_refresh_at = Some(Instant::now());
        self.docker_status = format!("loading details for {}", compact_id(&container_id));
        self.ensure_event_pump(window, cx);
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = DockerService::new(config)
                .container_details(&container_id)
                .map(|details| DockerJobOutput::Details {
                    container_id,
                    details,
                })
                .map_err(|error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn close_docker_details(&mut self, cx: &mut Context<Self>) {
        self.docker_details = None;
        self.docker_details_container_id = None;
        self.docker_details_last_refresh_at = None;
        self.docker_status = "container details closed".to_string();
        self.terminal_status = self.docker_status.clone();
        cx.notify();
    }

    pub(in crate::ui::view) fn load_docker_logs(
        &mut self,
        container_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status = "start an SSH session before reading Docker logs".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        self.docker_pending = true;
        self.docker_status = format!("loading logs for {}", compact_id(&container_id));
        self.ensure_event_pump(window, cx);
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = DockerService::new(config)
                .container_logs(&container_id, 200)
                .map(|output| DockerJobOutput::Logs {
                    container_id,
                    text: if output.stderr.trim().is_empty() {
                        output.stdout
                    } else {
                        format!("{}\n{}", output.stdout, output.stderr)
                    },
                })
                .map_err(|error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn send_docker_container_logs_to_terminal(
        &mut self,
        container_id: String,
        cx: &mut Context<Self>,
    ) {
        self.send_docker_terminal_command(
            format!("docker logs -f --tail 100 {}", shell_quote(&container_id)),
            format!("following logs for {}", compact_id(&container_id)),
            cx,
        );
    }

    pub(in crate::ui::view) fn enter_docker_container_terminal(
        &mut self,
        container_id: String,
        cx: &mut Context<Self>,
    ) {
        self.send_docker_terminal_command(
            format!(
                "docker exec -it {} sh -lc {}",
                shell_quote(&container_id),
                shell_quote(DOCKER_SHELL_SELECTOR)
            ),
            format!("entering container {}", compact_id(&container_id)),
            cx,
        );
    }

    pub(in crate::ui::view) fn send_docker_compose_service_logs_to_terminal(
        &mut self,
        project_name: String,
        config_files: Option<String>,
        service_name: String,
        cx: &mut Context<Self>,
    ) {
        self.send_docker_terminal_command(
            format!(
                "{} logs -f --tail 100 {}",
                docker_compose_terminal_base(&project_name, config_files.as_deref()),
                shell_quote(&service_name)
            ),
            format!("following compose logs for {service_name}"),
            cx,
        );
    }

    fn send_docker_terminal_command(
        &mut self,
        mut command: String,
        status: String,
        cx: &mut Context<Self>,
    ) {
        if self.active_session_id.is_none() {
            self.docker_status =
                "start a terminal session before sending Docker commands".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        }
        if !command.ends_with('\n') {
            command.push('\n');
        }
        self.selected_nav = NavItem::Workspace;
        self.send_terminal_input(command.into_bytes(), cx);
        self.docker_status = status;
        self.terminal_status = self.docker_status.clone();
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_docker_compose_project(
        &mut self,
        project_name: String,
        config_files: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = docker_compose_project_key(&project_name, config_files.as_deref());
        if self.docker_compose_expanded.remove(&key) {
            self.docker_status = format!("collapsed compose project {project_name}");
            cx.notify();
            return;
        }

        self.docker_compose_expanded.insert(key.clone());
        self.docker_status = format!("expanded compose project {project_name}");
        if !self.docker_compose_services.contains_key(&key)
            && !self.docker_compose_service_errors.contains_key(&key)
        {
            self.load_docker_compose_services(project_name, config_files, window, cx);
        } else {
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn load_docker_compose_services(
        &mut self,
        project_name: String,
        config_files: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status = "start an SSH session before reading compose services".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        let key = docker_compose_project_key(&project_name, config_files.as_deref());
        self.docker_pending = true;
        self.docker_status = format!("loading compose services for {project_name}");
        self.docker_compose_service_errors.remove(&key);
        self.ensure_event_pump(window, cx);
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = DockerService::new(config)
                .compose_services(&project_name, config_files.as_deref())
                .map(|services| DockerJobOutput::ComposeServices {
                    key,
                    project_name,
                    services,
                })
                .map_err(|error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn docker_compose_service_action(
        &mut self,
        project_name: String,
        config_files: Option<String>,
        service_name: String,
        action: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status =
                "start an SSH session before changing compose services".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        let key = docker_compose_project_key(&project_name, config_files.as_deref());
        self.docker_pending = true;
        self.docker_status = format!("compose {action} {service_name}");
        self.ensure_event_pump(window, cx);
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let service = DockerService::new(config);
                service.compose_service_action(
                    &project_name,
                    config_files.as_deref(),
                    &service_name,
                    action,
                )?;
                let overview = service.overview()?;
                let services = service.compose_services(&project_name, config_files.as_deref())?;
                Ok(DockerJobOutput::ComposeServiceAction {
                    key,
                    service_name,
                    action: action.to_string(),
                    overview,
                    services,
                })
            })()
            .map_err(|error: anyhow::Error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn docker_compose_action(
        &mut self,
        project_name: String,
        config_files: Option<String>,
        action: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status =
                "start an SSH session before changing compose projects".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        let key = docker_compose_project_key(&project_name, config_files.as_deref());
        self.docker_pending = true;
        self.docker_status = format!("compose {action} {project_name}");
        self.docker_compose_service_errors.remove(&key);
        self.ensure_event_pump(window, cx);
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let service = DockerService::new(config);
                service.compose_action(&project_name, config_files.as_deref(), action)?;
                let overview = service.overview()?;
                let service_result =
                    service.compose_services(&project_name, config_files.as_deref());
                let (services, service_error) = match service_result {
                    Ok(services) => (Some(services), None),
                    Err(error) => (None, Some(error.to_string())),
                };
                Ok(DockerJobOutput::ComposeProjectAction {
                    key,
                    project_name,
                    action: action.to_string(),
                    overview,
                    services,
                    service_error,
                })
            })()
            .map_err(|error: anyhow::Error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn request_docker_confirm(
        &mut self,
        confirm: DockerConfirmState,
        cx: &mut Context<Self>,
    ) {
        self.docker_confirm = Some(confirm);
        self.docker_status = "confirm Docker operation".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn cancel_docker_confirm(&mut self, cx: &mut Context<Self>) {
        self.docker_confirm = None;
        self.docker_status = "Docker operation cancelled".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn confirm_docker_action(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(confirm) = self.docker_confirm.clone() else {
            return;
        };
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status =
                "start an SSH session before changing Docker resources".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        self.docker_pending = true;
        self.docker_status = format!("running {}", confirm.title);
        self.ensure_event_pump(window, cx);
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let label = confirm.title.clone();
                let service = DockerService::new(config);
                match confirm.action {
                    DockerConfirmAction::ContainerAction {
                        container_id,
                        action,
                    } => {
                        service.container_action(&container_id, action)?;
                    }
                    DockerConfirmAction::ImageRemove { image_id, force } => {
                        service.image_remove(&image_id, force)?;
                    }
                    DockerConfirmAction::VolumeRemove { volume_name, force } => {
                        service.volume_remove(&volume_name, force)?;
                    }
                    DockerConfirmAction::NetworkRemove { network_id } => {
                        service.network_remove(&network_id)?;
                    }
                    DockerConfirmAction::ComposeAction {
                        project_name,
                        config_files,
                        action,
                    } => {
                        service.compose_action(&project_name, config_files.as_deref(), action)?;
                        let key =
                            docker_compose_project_key(&project_name, config_files.as_deref());
                        let overview = service.overview()?;
                        let service_result =
                            service.compose_services(&project_name, config_files.as_deref());
                        let (services, service_error) = match service_result {
                            Ok(services) => (Some(services), None),
                            Err(error) => (None, Some(error.to_string())),
                        };
                        return Ok(DockerJobOutput::ComposeProjectAction {
                            key,
                            project_name,
                            action: action.to_string(),
                            overview,
                            services,
                            service_error,
                        });
                    }
                    DockerConfirmAction::Prune { volumes } => {
                        service.system_prune(volumes)?;
                    }
                }
                let overview = service.overview()?;
                Ok(DockerJobOutput::RefreshedAfterAction { label, overview })
            })()
            .map_err(|error: anyhow::Error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn prune_docker_system(&mut self, cx: &mut Context<Self>) {
        self.request_docker_confirm(
            DockerConfirmState {
                title: "Docker system prune".to_string(),
                detail: "docker system prune -f --volumes".to_string(),
                action: DockerConfirmAction::Prune { volumes: true },
            },
            cx,
        );
    }

    pub(super) fn drain_process_events(&mut self) {
        while let Ok(event) = self.process_rx.try_recv() {
            self.process_pending = false;
            match event.result {
                Ok(ProcessJobOutput::Listed(processes)) => {
                    self.process_status = format!("loaded {} remote process(es)", processes.len());
                    self.terminal_status = self.process_status.clone();
                    self.apply_processes(processes);
                }
                Ok(ProcessJobOutput::Signalled {
                    pid,
                    signal,
                    processes,
                }) => {
                    self.process_status = format!("sent {signal} to pid {pid}");
                    self.terminal_status = self.process_status.clone();
                    self.process_signal_confirm = None;
                    self.apply_processes(processes);
                }
                Ok(ProcessJobOutput::Reniced {
                    pid,
                    nice,
                    processes,
                }) => {
                    self.process_status = format!("reniced pid {pid} to {nice}");
                    self.terminal_status = self.process_status.clone();
                    self.apply_processes(processes);
                }
                Err(error) => {
                    self.process_status = format!("process operation failed: {error}");
                    self.terminal_status = self.process_status.clone();
                }
            }
        }
    }

    fn apply_processes(&mut self, processes: Vec<RemoteProcess>) {
        if self
            .process_selected_pid
            .is_some_and(|pid| !processes.iter().any(|process| process.pid == pid))
        {
            self.process_selected_pid = None;
            self.process_nice_draft = "0".to_string();
        }
        self.processes = processes;
    }

    pub(super) fn drain_docker_events(&mut self) {
        while let Ok(event) = self.docker_rx.try_recv() {
            self.docker_pending = false;
            match event.result {
                Ok(DockerJobOutput::Overview(overview)) => {
                    self.docker_status = docker_overview_status(&overview);
                    self.terminal_status = self.docker_status.clone();
                    self.apply_docker_overview(overview);
                }
                Ok(DockerJobOutput::Details {
                    container_id,
                    details,
                }) => {
                    self.docker_status =
                        format!("loaded details for {}", compact_id(&container_id));
                    self.terminal_status = self.docker_status.clone();
                    self.docker_details = Some(details);
                    self.docker_details_container_id = Some(container_id);
                }
                Ok(DockerJobOutput::Logs { container_id, text }) => {
                    self.docker_status = format!("loaded logs for {}", compact_id(&container_id));
                    self.terminal_status = self.docker_status.clone();
                    self.docker_logs_container_id = Some(container_id);
                    self.docker_logs = truncate_preview(&text, 4000);
                }
                Ok(DockerJobOutput::ComposeServices {
                    key,
                    project_name,
                    services,
                }) => {
                    self.docker_status =
                        format!("loaded {} service(s) for {project_name}", services.len());
                    self.terminal_status = self.docker_status.clone();
                    self.docker_compose_service_errors.remove(&key);
                    self.docker_compose_services.insert(key, services);
                }
                Ok(DockerJobOutput::ComposeServiceAction {
                    key,
                    service_name,
                    action,
                    overview,
                    services,
                }) => {
                    self.docker_status = format!("compose {action} {service_name}");
                    self.terminal_status = self.docker_status.clone();
                    self.apply_docker_overview(overview);
                    self.docker_compose_services.insert(key.clone(), services);
                    self.docker_compose_service_errors.remove(&key);
                }
                Ok(DockerJobOutput::ComposeProjectAction {
                    key,
                    project_name,
                    action,
                    overview,
                    services,
                    service_error,
                }) => {
                    self.docker_status = format!("compose {action} {project_name}");
                    self.terminal_status = self.docker_status.clone();
                    self.apply_docker_overview(overview);
                    if let Some(services) = services {
                        self.docker_compose_services.insert(key.clone(), services);
                        self.docker_compose_service_errors.remove(&key);
                    } else if let Some(error) = service_error {
                        self.docker_compose_services.remove(&key);
                        self.docker_compose_service_errors
                            .insert(key.clone(), error);
                    }
                    self.docker_confirm = None;
                }
                Ok(DockerJobOutput::RefreshedAfterAction { label, overview }) => {
                    let container_count = overview.containers.len();
                    self.apply_docker_overview(overview);
                    self.docker_status =
                        format!("{label} completed · {container_count} container(s)");
                    self.terminal_status = self.docker_status.clone();
                    self.docker_confirm = None;
                }
                Err(error) => {
                    self.docker_status = format!("Docker operation failed: {error}");
                    self.terminal_status = self.docker_status.clone();
                }
            }
        }
    }

    fn apply_docker_overview(&mut self, overview: RemoteDockerOverview) {
        if let Some(details_id) = self.docker_details_container_id.as_deref()
            && !overview
                .containers
                .iter()
                .any(|container| container.id == details_id)
        {
            self.docker_details = None;
            self.docker_details_container_id = None;
        }
        let active_compose_keys = overview
            .compose_projects
            .iter()
            .map(|project| {
                docker_compose_project_key(&project.name, Some(project.config_files.as_str()))
            })
            .collect::<HashSet<_>>();
        self.docker_compose_expanded
            .retain(|key| active_compose_keys.contains(key));
        self.docker_compose_services
            .retain(|key, _| active_compose_keys.contains(key));
        self.docker_compose_service_errors
            .retain(|key, _| active_compose_keys.contains(key));
        self.docker_overview = Some(overview);
    }

    pub(super) fn drain_stats_events(&mut self) {
        while let Ok(event) = self.stats_rx.try_recv() {
            self.stats_pending = false;
            match event.result {
                Ok(stats) => {
                    self.stats_status = format!(
                        "loaded stats for {} · load {:.2}/{:.2}/{:.2}",
                        if stats.system.hostname.trim().is_empty() {
                            "remote host"
                        } else {
                            stats.system.hostname.as_str()
                        },
                        stats.load.load1,
                        stats.load.load5,
                        stats.load.load15
                    );
                    self.terminal_status = self.stats_status.clone();
                    self.remote_stats = Some(stats);
                }
                Err(error) => {
                    self.stats_status = format!("stats refresh failed: {error}");
                    self.terminal_status = self.stats_status.clone();
                }
            }
        }
    }
}

const DOCKER_SHELL_SELECTOR: &str = "if command -v bash >/dev/null 2>&1; then exec bash; elif command -v zsh >/dev/null 2>&1; then exec zsh; elif command -v fish >/dev/null 2>&1; then exec fish; elif command -v ash >/dev/null 2>&1; then exec ash; else exec sh; fi";

fn docker_compose_terminal_base(project_name: &str, config_files: Option<&str>) -> String {
    let mut command = String::from("docker compose");
    for file in config_files.unwrap_or_default().split(',') {
        let file = file.trim();
        if !file.is_empty() && !file.eq_ignore_ascii_case("n/a") {
            command.push_str(" -f ");
            command.push_str(&shell_quote(file));
        }
    }
    command.push_str(" -p ");
    command.push_str(&shell_quote(project_name));
    command
}

fn docker_overview_status(overview: &RemoteDockerOverview) -> String {
    if overview.available {
        format!(
            "Docker {} · {} container(s)",
            if overview.version.trim().is_empty() {
                "available".to_string()
            } else {
                overview.version.clone()
            },
            overview.containers.len()
        )
    } else {
        "Docker is not available on this SSH host".to_string()
    }
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
