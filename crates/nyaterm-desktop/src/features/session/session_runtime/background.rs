use super::*;

impl NyaTermApp {
    pub(in crate::features) fn begin_background_ssh_start(
        &mut self,
        connection_name: String,
        config: SshSessionConfig,
        source_connection_id: Option<String>,
        ai_execution_profile: AiExecutionProfile,
        custom_name: Option<String>,
        tab_color: Option<u32>,
        after_session_id: Option<String>,
        insert_index: Option<usize>,
        seed_output: Option<String>,
        startup_command: Option<StartupCommandRequest>,
        cx: &mut Context<Self>,
    ) {
        let request_id = uuid();
        self.pending_session_name = Some(connection_name.clone());
        self.last_connect_failure_name = None;
        self.last_connect_failure_error = None;
        self.session_pane_states.insert(
            request_id.clone(),
            SessionPaneState::Connecting {
                request_id: request_id.clone(),
                name: connection_name.clone(),
                kind: SessionKind::Ssh,
            },
        );
        self.pending_session_starts.insert(
            request_id.clone(),
            PendingSessionStart {
                connection_name: connection_name.clone(),
                ssh_config: Some(config.clone()),
                ai_execution_profile,
                custom_name,
                tab_color,
                after_session_id,
                insert_index,
                seed_output,
                startup_command,
                multiplex_key: None,
                source_connection_id,
            },
        );
        self.terminal_status = format!("connecting to {connection_name}");
        if self.active_session_id.is_none() {
            self.append_terminal_log(format!("\n# connecting to {connection_name}\n"));
        }
        self.selected_nav = NavItem::Workspace;

        let session_manager = self.session_manager.clone();
        let session_start_tx = self.session_start_tx.clone();
        let request_id_for_worker = request_id.clone();
        std::thread::spawn(move || {
            let result = session_manager
                .create_ssh_session(config)
                .map(|info| SessionStartSuccess {
                    session_id: info.id,
                    multiplex_handle: None,
                })
                .map_err(|error| error.to_string());
            let _ = session_start_tx.send(SessionStartResult {
                request_id: request_id_for_worker,
                connection_name,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn begin_background_multiplex_ssh_start(
        &mut self,
        connection_name: String,
        config: SshSessionConfig,
        source_connection_id: Option<String>,
        ai_execution_profile: AiExecutionProfile,
        custom_name: Option<String>,
        tab_color: Option<u32>,
        after_session_id: Option<String>,
        startup_command: Option<StartupCommandRequest>,
        existing_multiplex: Option<SshMultiplexHandle>,
        cx: &mut Context<Self>,
    ) {
        let multiplex_key = ssh_multiplex_key(&config);
        let request_id = uuid();
        self.pending_session_name = Some(connection_name.clone());
        self.last_connect_failure_name = None;
        self.last_connect_failure_error = None;
        self.session_pane_states.insert(
            request_id.clone(),
            SessionPaneState::Connecting {
                request_id: request_id.clone(),
                name: connection_name.clone(),
                kind: SessionKind::Ssh,
            },
        );
        self.pending_session_starts.insert(
            request_id.clone(),
            PendingSessionStart {
                connection_name: connection_name.clone(),
                ssh_config: Some(config.clone()),
                ai_execution_profile,
                custom_name,
                tab_color,
                after_session_id,
                insert_index: None,
                seed_output: None,
                startup_command,
                multiplex_key: Some(multiplex_key.clone()),
                source_connection_id,
            },
        );
        self.terminal_status = format!("multiplexing SSH session {connection_name}");
        self.selected_nav = NavItem::Workspace;

        let session_manager = self.session_manager.clone();
        let session_start_tx = self.session_start_tx.clone();
        let request_id_for_worker = request_id.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let multiplex = match existing_multiplex {
                    Some(handle) if !handle.is_closed() => handle,
                    _ => open_ssh_multiplex_handle(config.clone())
                        .map_err(|error| error.to_string())?,
                };
                let info = session_manager
                    .create_ssh_session_with_multiplex(config, multiplex.clone())
                    .map_err(|error| error.to_string())?;
                Ok(SessionStartSuccess {
                    session_id: info.id,
                    multiplex_handle: Some(multiplex),
                })
            })();
            let _ = session_start_tx.send(SessionStartResult {
                request_id: request_id_for_worker,
                connection_name,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn send_probe_command(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.active_session_id.as_deref() else {
            self.terminal_status = "start a session first".to_string();
            cx.notify();
            return;
        };

        let command = if cfg!(target_os = "windows") {
            "echo nyaterm-app-ready\r\n"
        } else {
            "printf 'nyaterm-app-ready\\n'\n"
        };
        match self.session_manager.write(session_id, command.as_bytes()) {
            Ok(()) => {
                self.terminal_status = "probe command sent".to_string();
            }
            Err(error) => {
                self.terminal_status = format!("write failed: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn drain_session_start_events(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut dirty = false;
        while let Ok(event) = self.session_start_rx.try_recv() {
            dirty = true;
            let pending = self.pending_session_starts.remove(&event.request_id);
            match event.result {
                Ok(success) => {
                    self.last_connect_failure_name = None;
                    self.last_connect_failure_error = None;
                    let session_id = success.session_id;
                    let ssh_config = pending
                        .as_ref()
                        .and_then(|pending| pending.ssh_config.clone());
                    let launch_config = ssh_config
                        .clone()
                        .map(SessionLaunchConfig::Ssh)
                        .unwrap_or_else(|| SessionLaunchConfig::Ssh(SshSessionConfig::default()));
                    let ssh_multiplex_key = pending
                        .as_ref()
                        .and_then(|pending| pending.multiplex_key.clone());
                    if let (Some(key), Some(handle)) =
                        (ssh_multiplex_key.clone(), success.multiplex_handle)
                    {
                        self.ssh_multiplex_handles.insert(key, handle);
                    }
                    let source_connection_id = pending
                        .as_ref()
                        .and_then(|pending| pending.source_connection_id.clone());
                    let ai_execution_profile = pending
                        .as_ref()
                        .map(|pending| pending.ai_execution_profile)
                        .unwrap_or(AiExecutionProfile::SendOnly);
                    self.register_session(
                        &session_id,
                        SessionRuntimeMetadata {
                            ssh_config,
                            ssh_multiplex_key,
                            source_connection_id,
                            ai_execution_profile,
                            launch_config,
                            disconnected: false,
                        },
                    );
                    if let Some(custom_name) = pending
                        .as_ref()
                        .and_then(|pending| pending.custom_name.clone())
                    {
                        self.session_custom_names
                            .insert(session_id.clone(), custom_name);
                    }
                    if let Some(tab_color) = pending.as_ref().and_then(|pending| pending.tab_color)
                    {
                        self.session_tab_colors
                            .insert(session_id.clone(), tab_color);
                    }
                    if let Some(seed_output) = pending
                        .as_ref()
                        .and_then(|pending| pending.seed_output.clone())
                    {
                        self.terminal_views.insert(
                            session_id.clone(),
                            TerminalViewState::from_output(seed_output),
                        );
                    }
                    if let Some(after_session_id) = pending
                        .as_ref()
                        .and_then(|pending| pending.after_session_id.clone())
                    {
                        self.move_session_after(&session_id, &after_session_id);
                    }
                    if let Some(insert_index) =
                        pending.as_ref().and_then(|pending| pending.insert_index)
                    {
                        self.move_session_to_index(&session_id, insert_index);
                    }
                    if let Some(stale_id) = self.pending_reconnect_replace_id.take() {
                        if stale_id != session_id {
                            self.remove_session_state(&stale_id);
                        }
                    }
                    self.session_pane_states.insert(
                        event.request_id.clone(),
                        SessionPaneState::Live {
                            session_id: session_id.clone(),
                        },
                    );
                    self.activate_session_id(&session_id);
                    self.terminal_status = format!("running {}", short_id(&session_id));
                    self.append_terminal_log(format!(
                        "\n# started {} ({})\n",
                        event.connection_name,
                        short_id(&session_id)
                    ));
                    self.maybe_auto_start_recording(&session_id, &event.connection_name);
                    if let Some(startup_command) =
                        pending.and_then(|pending| pending.startup_command)
                    {
                        self.schedule_startup_command(session_id.clone(), startup_command, cx);
                    }
                    self.apply_pending_workspace_split_for_duplicate(&session_id);
                    self.selected_nav = NavItem::Workspace;
                }
                Err(error) => {
                    self.last_connect_failure_name = Some(event.connection_name.clone());
                    self.last_connect_failure_error = Some(error.clone());
                    self.session_pane_states.insert(
                        event.request_id.clone(),
                        SessionPaneState::Failed {
                            name: event.connection_name.clone(),
                            error: error.clone(),
                        },
                    );
                    self.terminal_status =
                        format!("failed to start {}: {error}", event.connection_name);
                    if self.active_session_id.is_none() {
                        self.append_terminal_log(format!(
                            "\n# failed to start {}: {error}\n",
                            event.connection_name
                        ));
                    }
                    self.selected_nav = NavItem::Workspace;
                }
            }
            self.refresh_pending_session_name();
        }
        dirty
    }

    pub(in crate::features) fn refresh_pending_session_name(&mut self) {
        self.pending_session_name = self
            .pending_session_starts
            .values()
            .next()
            .map(|pending| pending.connection_name.clone());
    }
}
