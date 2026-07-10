use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn duplicate_active_session(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.duplicate_active_session_with_startup(None, window, cx);
    }

    pub(in crate::ui::view) fn duplicate_active_session_with_startup(
        &mut self,
        startup_command: Option<StartupCommandRequest>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_session_name.is_some() {
            self.terminal_status = "wait for the pending session to finish connecting".to_string();
            cx.notify();
            return;
        }
        let Some(source_session_id) = self.active_session_id.clone() else {
            self.terminal_status = "no active session to duplicate".to_string();
            cx.notify();
            return;
        };
        let Some(metadata) = self.session_metadata.get(&source_session_id).cloned() else {
            self.terminal_status = "active session cannot be duplicated".to_string();
            cx.notify();
            return;
        };
        let custom_name = self.session_custom_names.get(&source_session_id).cloned();
        let custom_color = self.session_tab_colors.get(&source_session_id).copied();

        match metadata.launch_config.clone() {
            SessionLaunchConfig::Local(config) => {
                match self.session_manager.create_local_session(config.clone()) {
                    Ok(info) => {
                        self.register_session(&info.id, metadata);
                        self.move_session_after(&info.id, &source_session_id);
                        if let Some(custom_name) = custom_name {
                            self.session_custom_names
                                .insert(info.id.clone(), custom_name);
                        }
                        if let Some(custom_color) = custom_color {
                            self.session_tab_colors
                                .insert(info.id.clone(), custom_color);
                        }
                        self.activate_session_id(&info.id);
                        self.terminal_status = format!("duplicated {}", short_id(&info.id));
                        self.append_terminal_log(format!(
                            "\n# duplicated local PTY {}\n",
                            short_id(&info.id)
                        ));
                        self.maybe_auto_start_recording(&info.id, &info.name);
                        if let Some(startup_command) = startup_command.clone() {
                            self.schedule_startup_command(info.id.clone(), startup_command, cx);
                        }
                        self.apply_pending_workspace_split_for_duplicate(&info.id);
                    }
                    Err(error) => {
                        self.pending_workspace_split = None;
                        self.terminal_status = format!("duplicate failed: {error}");
                    }
                }
            }
            SessionLaunchConfig::Telnet(config) => {
                match self.session_manager.create_telnet_session(config.clone()) {
                    Ok(info) => {
                        self.register_session(&info.id, metadata);
                        self.move_session_after(&info.id, &source_session_id);
                        if let Some(custom_name) = custom_name {
                            self.session_custom_names
                                .insert(info.id.clone(), custom_name);
                        }
                        if let Some(custom_color) = custom_color {
                            self.session_tab_colors
                                .insert(info.id.clone(), custom_color);
                        }
                        self.activate_session_id(&info.id);
                        self.terminal_status = format!("duplicated {}", short_id(&info.id));
                        self.append_terminal_log(format!(
                            "\n# duplicated telnet session {}\n",
                            short_id(&info.id)
                        ));
                        self.maybe_auto_start_recording(&info.id, &info.name);
                        if let Some(startup_command) = startup_command.clone() {
                            self.schedule_startup_command(info.id.clone(), startup_command, cx);
                        }
                        self.apply_pending_workspace_split_for_duplicate(&info.id);
                    }
                    Err(error) => {
                        self.pending_workspace_split = None;
                        self.terminal_status = format!("duplicate failed: {error}");
                    }
                }
            }
            SessionLaunchConfig::Serial(config) => {
                match self.session_manager.create_serial_session(config.clone()) {
                    Ok(info) => {
                        self.register_session(&info.id, metadata);
                        self.move_session_after(&info.id, &source_session_id);
                        if let Some(custom_name) = custom_name {
                            self.session_custom_names
                                .insert(info.id.clone(), custom_name);
                        }
                        if let Some(custom_color) = custom_color {
                            self.session_tab_colors
                                .insert(info.id.clone(), custom_color);
                        }
                        self.activate_session_id(&info.id);
                        self.terminal_status = format!("duplicated {}", short_id(&info.id));
                        self.append_terminal_log(format!(
                            "\n# duplicated serial session {}\n",
                            short_id(&info.id)
                        ));
                        self.maybe_auto_start_recording(&info.id, &info.name);
                        if let Some(startup_command) = startup_command.clone() {
                            self.schedule_startup_command(info.id.clone(), startup_command, cx);
                        }
                        self.apply_pending_workspace_split_for_duplicate(&info.id);
                    }
                    Err(error) => {
                        self.pending_workspace_split = None;
                        self.terminal_status = format!("duplicate failed: {error}");
                    }
                }
            }
            SessionLaunchConfig::Ssh(config) => {
                self.ensure_event_pump(window, cx);
                self.begin_background_ssh_start(
                    format!("{} duplicate", config.name),
                    config,
                    metadata.source_connection_id.clone(),
                    metadata.ai_execution_profile,
                    custom_name,
                    custom_color,
                    Some(source_session_id),
                    None,
                    None,
                    startup_command,
                    cx,
                );
            }
        }
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        cx.notify();
    }

    pub(in crate::ui::view) fn multiplex_active_ssh_session(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.multiplex_active_ssh_session_with_startup(None, window, cx);
    }

    pub(in crate::ui::view) fn multiplex_active_ssh_session_with_startup(
        &mut self,
        startup_command: Option<StartupCommandRequest>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_session_name.is_some() {
            self.terminal_status = "wait for the pending session to finish connecting".to_string();
            cx.notify();
            return;
        }
        let Some(source_session_id) = self.active_session_id.clone() else {
            self.terminal_status = "no active SSH session to multiplex".to_string();
            cx.notify();
            return;
        };
        let Some(metadata) = self.session_metadata.get(&source_session_id).cloned() else {
            self.terminal_status = "active session cannot be multiplexed".to_string();
            cx.notify();
            return;
        };
        let SessionLaunchConfig::Ssh(config) = metadata.launch_config.clone() else {
            self.terminal_status = "active session is not SSH".to_string();
            cx.notify();
            return;
        };
        let multiplex_key = ssh_multiplex_key(&config);
        if self
            .ssh_multiplex_handles
            .get(&multiplex_key)
            .is_some_and(SshMultiplexHandle::is_closed)
        {
            self.ssh_multiplex_handles.remove(&multiplex_key);
        }
        let existing_multiplex = self.ssh_multiplex_handles.get(&multiplex_key).cloned();
        let custom_name = self.session_custom_names.get(&source_session_id).cloned();
        let custom_color = self.session_tab_colors.get(&source_session_id).copied();
        self.ensure_event_pump(window, cx);
        self.begin_background_multiplex_ssh_start(
            format!("{} multiplex", config.name),
            config,
            metadata.source_connection_id.clone(),
            metadata.ai_execution_profile,
            custom_name,
            custom_color,
            Some(source_session_id),
            startup_command,
            existing_multiplex,
            cx,
        );
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        cx.notify();
    }

    pub(in crate::ui::view) fn reconnect_active_session(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_session_name.is_some() {
            self.terminal_status = "wait for the pending session to finish connecting".to_string();
            cx.notify();
            return;
        }
        let Some(source_session_id) = self.active_session_id.clone() else {
            self.terminal_status = "no active session to reconnect".to_string();
            cx.notify();
            return;
        };
        let Some(metadata) = self.session_metadata.get(&source_session_id).cloned() else {
            self.terminal_status = "active session cannot be reconnected".to_string();
            cx.notify();
            return;
        };
        let source_index = self
            .session_order
            .iter()
            .position(|id| id == &source_session_id)
            .unwrap_or(self.session_order.len());
        let custom_name = self.session_custom_names.get(&source_session_id).cloned();
        let custom_color = self.session_tab_colors.get(&source_session_id).copied();
        let seed_output = self
            .terminal_views
            .get(&source_session_id)
            .map(|view| view.output.clone())
            .unwrap_or_else(|| self.terminal_output.clone());

        match self.session_manager.close(&source_session_id) {
            Ok(()) => {
                self.recording_manager.cleanup_session(&source_session_id);
                self.remove_session_state(&source_session_id);
                self.active_session_id = None;
                self.active_ssh_config = None;
                self.active_ai_execution_profile = AiExecutionProfile::SendOnly;
                self.ai_agent_loop = None;
                self.ai_agent_capture = AgentOutputCaptureProcessor::new();
            }
            Err(error) => {
                self.terminal_status = format!("reconnect failed: {error}");
                cx.notify();
                return;
            }
        }

        match metadata.launch_config.clone() {
            SessionLaunchConfig::Local(config) => {
                match self.session_manager.create_local_session(config.clone()) {
                    Ok(info) => {
                        self.register_session(&info.id, metadata);
                        self.terminal_views
                            .insert(info.id.clone(), TerminalViewState::from_output(seed_output));
                        self.move_session_to_index(&info.id, source_index);
                        if let Some(custom_name) = custom_name {
                            self.session_custom_names
                                .insert(info.id.clone(), custom_name);
                        }
                        if let Some(custom_color) = custom_color {
                            self.session_tab_colors
                                .insert(info.id.clone(), custom_color);
                        }
                        self.activate_session_id(&info.id);
                        self.terminal_status = format!("reconnected {}", short_id(&info.id));
                        self.append_terminal_log(format!(
                            "\n# reconnected local PTY {}\n",
                            short_id(&info.id)
                        ));
                        self.maybe_auto_start_recording(&info.id, &info.name);
                    }
                    Err(error) => {
                        self.terminal_output = seed_output.clone();
                        self.terminal_screen = terminal_screen_from_output(&seed_output);
                        self.terminal_status = format!("reconnect failed: {error}");
                    }
                }
            }
            SessionLaunchConfig::Telnet(config) => {
                match self.session_manager.create_telnet_session(config.clone()) {
                    Ok(info) => {
                        self.register_session(&info.id, metadata);
                        self.terminal_views
                            .insert(info.id.clone(), TerminalViewState::from_output(seed_output));
                        self.move_session_to_index(&info.id, source_index);
                        if let Some(custom_name) = custom_name {
                            self.session_custom_names
                                .insert(info.id.clone(), custom_name);
                        }
                        if let Some(custom_color) = custom_color {
                            self.session_tab_colors
                                .insert(info.id.clone(), custom_color);
                        }
                        self.activate_session_id(&info.id);
                        self.terminal_status = format!("reconnected {}", short_id(&info.id));
                        self.append_terminal_log(format!(
                            "\n# reconnected telnet session {}\n",
                            short_id(&info.id)
                        ));
                        self.maybe_auto_start_recording(&info.id, &info.name);
                    }
                    Err(error) => {
                        self.terminal_output = seed_output.clone();
                        self.terminal_screen = terminal_screen_from_output(&seed_output);
                        self.terminal_status = format!("reconnect failed: {error}");
                    }
                }
            }
            SessionLaunchConfig::Serial(config) => {
                match self.session_manager.create_serial_session(config.clone()) {
                    Ok(info) => {
                        self.register_session(&info.id, metadata);
                        self.terminal_views
                            .insert(info.id.clone(), TerminalViewState::from_output(seed_output));
                        self.move_session_to_index(&info.id, source_index);
                        if let Some(custom_name) = custom_name {
                            self.session_custom_names
                                .insert(info.id.clone(), custom_name);
                        }
                        if let Some(custom_color) = custom_color {
                            self.session_tab_colors
                                .insert(info.id.clone(), custom_color);
                        }
                        self.activate_session_id(&info.id);
                        self.terminal_status = format!("reconnected {}", short_id(&info.id));
                        self.append_terminal_log(format!(
                            "\n# reconnected serial session {}\n",
                            short_id(&info.id)
                        ));
                        self.maybe_auto_start_recording(&info.id, &info.name);
                    }
                    Err(error) => {
                        self.terminal_output = seed_output.clone();
                        self.terminal_screen = terminal_screen_from_output(&seed_output);
                        self.terminal_status = format!("reconnect failed: {error}");
                    }
                }
            }
            SessionLaunchConfig::Ssh(config) => {
                self.ensure_event_pump(window, cx);
                self.begin_background_ssh_start(
                    format!("{} reconnect", config.name),
                    config,
                    metadata.source_connection_id.clone(),
                    metadata.ai_execution_profile,
                    custom_name,
                    custom_color,
                    None,
                    Some(source_index),
                    Some(seed_output),
                    None,
                    cx,
                );
            }
        }
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        cx.notify();
    }
}
