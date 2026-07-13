use super::*;

impl NyaTermApp {
    pub(in crate::features) fn duplicate_active_session(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.duplicate_active_session_with_startup(None, window, cx);
    }

    pub(in crate::features) fn duplicate_active_session_with_startup(
        &mut self,
        startup_command: Option<StartupCommandRequest>,
        _window: &mut Window,
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

    pub(in crate::features) fn multiplex_active_ssh_session(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.multiplex_active_ssh_session_with_startup(None, window, cx);
    }

    pub(in crate::features) fn multiplex_active_ssh_session_with_startup(
        &mut self,
        startup_command: Option<StartupCommandRequest>,
        _window: &mut Window,
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

    pub(in crate::features) fn reconnect_active_session(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(source_session_id) = self.active_session_id.clone() else {
            self.terminal_status = "no active session to reconnect".to_string();
            cx.notify();
            return;
        };
        self.reconnect_session(source_session_id, window, cx);
    }

    /// Close the backend session but keep the tab for reconnect (Tauri Disconnect).
    pub(in crate::features) fn disconnect_session(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.active_session_busy_actions.get(&session_id).is_some() {
            self.terminal_status = "session action already in progress".to_string();
            cx.notify();
            return;
        }
        if self.is_session_disconnected(&session_id) {
            self.terminal_status = "session already disconnected".to_string();
            cx.notify();
            return;
        }
        if !self.session_metadata.contains_key(&session_id) {
            self.terminal_status = "session no longer exists".to_string();
            cx.notify();
            return;
        }

        self.active_session_busy_actions
            .insert(session_id.clone(), "disconnect".to_string());
        self.active_session_menu_id = None;
        // Backend may already be gone (race with Exited); still mark disconnected.
        let _ = self.session_manager.close(&session_id);
        self.recording_manager.cleanup_session(&session_id);
        self.mark_session_disconnected(&session_id, cx);
        self.active_session_busy_actions.remove(&session_id);
        self.terminal_status = format!("disconnected {}", short_id(&session_id));
        cx.notify();
    }

    pub(in crate::features) fn disconnect_active_session(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.active_session_id.clone() else {
            self.terminal_status = "no active session to disconnect".to_string();
            cx.notify();
            return;
        };
        self.disconnect_session(session_id, cx);
    }

    pub(in crate::features) fn mark_session_disconnected(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(metadata) = self.session_metadata.get_mut(session_id) else {
            return;
        };
        if metadata.disconnected {
            return;
        }
        metadata.disconnected = true;
        // Drop multiplex handle association for this session key if unused.
        let multiplex_key = metadata.ssh_multiplex_key.clone();
        if let Some(multiplex_key) = multiplex_key {
            let still_in_use = self.session_metadata.iter().any(|(id, meta)| {
                id != session_id
                    && !meta.disconnected
                    && meta.ssh_multiplex_key.as_deref() == Some(multiplex_key.as_str())
            });
            if !still_in_use {
                if let Some(handle) = self.ssh_multiplex_handles.remove(&multiplex_key) {
                    let _ = handle.disconnect();
                }
            }
        }

        let banner = "\r\n\x1b[31m[Session disconnected]\x1b[0m\r\n\x1b[33m[Press Enter to reconnect]\x1b[0m\r\n";
        if let Some(view) = self.terminal_views.get_mut(session_id) {
            view.append_text(banner);
        } else {
            let mut view = TerminalViewState::new();
            view.append_text(banner);
            self.terminal_views.insert(session_id.to_string(), view);
        }

        if self.active_session_id.as_deref() == Some(session_id) {
            self.command_input_tracker = TerminalInputState::new();
            self.command_suggestions = None;
            self.credential_suggestions = None;
        }
        self.prune_workspace_split();
        cx.notify();
    }

    /// Reconnect a disconnected tab (or force-recreate a live one) by id.
    pub(in crate::features) fn reconnect_session(
        &mut self,
        session_id: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_session_busy_actions.get(&session_id).is_some() {
            self.terminal_status = "session action already in progress".to_string();
            cx.notify();
            return;
        }
        if self.pending_session_name.is_some() {
            self.terminal_status = "wait for the pending session to finish connecting".to_string();
            cx.notify();
            return;
        }
        if !self.session_metadata.contains_key(&session_id) {
            self.terminal_status = "session cannot be reconnected".to_string();
            cx.notify();
            return;
        }
        self.active_session_busy_actions
            .insert(session_id.clone(), "reconnect".to_string());
        self.active_session_menu_id = None;
        let source_index = self
            .session_order
            .iter()
            .position(|id| id == &session_id)
            .unwrap_or(self.session_order.len());
        let custom_name = self.session_custom_names.get(&session_id).cloned();
        let custom_color = self.session_tab_colors.get(&session_id).copied();
        let seed_output = self
            .terminal_views
            .get(&session_id)
            .map(|view| view.output.clone())
            .unwrap_or_default();
        let was_active = self.active_session_id.as_deref() == Some(session_id.as_str());

        // Tauri: write cyan reconnecting line into the buffer before recreating.
        if let Some(view) = self.terminal_views.get_mut(&session_id) {
            view.append_text(
                "
[36m[Reconnecting…][0m
",
            );
        }
        let seed_output = self
            .terminal_views
            .get(&session_id)
            .map(|view| view.output.clone())
            .unwrap_or(seed_output);

        // Close live backend if still present.
        let _ = self.session_manager.close(&session_id);
        self.recording_manager.cleanup_session(&session_id);

        // Soft-remove UI state without dropping order/metadata, then recreate under same id path:
        // We allocate a new backend id and migrate UI maps to the new id.
        let old_id = session_id;
        self.session_order.retain(|id| id != &old_id);
        let metadata = self
            .session_metadata
            .remove(&old_id)
            .expect("metadata present");
        let view = self.terminal_views.remove(&old_id);
        let history = self.session_command_history.remove(&old_id);
        self.session_custom_names.remove(&old_id);
        let dynamic_title = self.session_dynamic_titles.remove(&old_id);
        let session_cwd = self.session_cwds.remove(&old_id);
        self.session_tab_colors.remove(&old_id);
        self.transfer_browser_session_cache.remove(&old_id);
        self.purge_session_from_sync_groups(&old_id);
        if was_active {
            self.active_session_id = None;
            self.active_ssh_config = None;
            self.active_ai_execution_profile = AiExecutionProfile::SendOnly;
            self.ai_agent_loop = None;
            self.ai_agent_capture = AgentOutputCaptureProcessor::new();
        }

        let mut metadata = metadata;
        metadata.disconnected = false;

        let restore_maps = |this: &mut Self, new_id: &str| {
            this.move_session_to_index(new_id, source_index);
            if let Some(custom_name) = custom_name.clone() {
                this.session_custom_names
                    .insert(new_id.to_string(), custom_name);
            }
            if let Some(title) = dynamic_title.clone() {
                this.session_dynamic_titles
                    .insert(new_id.to_string(), title);
            }
            if let Some(cwd) = session_cwd.clone() {
                this.session_cwds.insert(new_id.to_string(), cwd);
            }
            if let Some(custom_color) = custom_color {
                this.session_tab_colors
                    .insert(new_id.to_string(), custom_color);
            }
            if let Some(history) = history.clone() {
                this.session_command_history
                    .insert(new_id.to_string(), history);
            }
        };

        match metadata.launch_config.clone() {
            SessionLaunchConfig::Local(config) => {
                match self.session_manager.create_local_session(config.clone()) {
                    Ok(info) => {
                        self.register_session(&info.id, metadata);
                        self.terminal_views.insert(
                            info.id.clone(),
                            view.unwrap_or_else(|| TerminalViewState::from_output(seed_output)),
                        );
                        restore_maps(self, &info.id);
                        self.activate_session_id(&info.id);
                        self.terminal_status = format!("reconnected {}", short_id(&info.id));
                        self.append_terminal_log(format!(
                            "\n# reconnected local PTY {}\n",
                            short_id(&info.id)
                        ));
                        self.maybe_auto_start_recording(&info.id, &info.name);
                    }
                    Err(error) => {
                        // Put disconnected tab back on failure.
                        self.restore_failed_reconnect(
                            old_id.clone(),
                            metadata,
                            view,
                            seed_output,
                            source_index,
                            custom_name,
                            custom_color,
                            history,
                            error.to_string(),
                            was_active,
                            cx,
                        );
                    }
                }
            }
            SessionLaunchConfig::Telnet(config) => {
                match self.session_manager.create_telnet_session(config.clone()) {
                    Ok(info) => {
                        self.register_session(&info.id, metadata);
                        self.terminal_views.insert(
                            info.id.clone(),
                            view.unwrap_or_else(|| TerminalViewState::from_output(seed_output)),
                        );
                        restore_maps(self, &info.id);
                        self.activate_session_id(&info.id);
                        self.terminal_status = format!("reconnected {}", short_id(&info.id));
                        self.append_terminal_log(format!(
                            "\n# reconnected telnet session {}\n",
                            short_id(&info.id)
                        ));
                        self.maybe_auto_start_recording(&info.id, &info.name);
                    }
                    Err(error) => {
                        self.restore_failed_reconnect(
                            old_id.clone(),
                            metadata,
                            view,
                            seed_output,
                            source_index,
                            custom_name,
                            custom_color,
                            history,
                            error.to_string(),
                            was_active,
                            cx,
                        );
                    }
                }
            }
            SessionLaunchConfig::Serial(config) => {
                match self.session_manager.create_serial_session(config.clone()) {
                    Ok(info) => {
                        self.register_session(&info.id, metadata);
                        self.terminal_views.insert(
                            info.id.clone(),
                            view.unwrap_or_else(|| TerminalViewState::from_output(seed_output)),
                        );
                        restore_maps(self, &info.id);
                        self.activate_session_id(&info.id);
                        self.terminal_status = format!("reconnected {}", short_id(&info.id));
                        self.append_terminal_log(format!(
                            "\n# reconnected serial session {}\n",
                            short_id(&info.id)
                        ));
                        self.maybe_auto_start_recording(&info.id, &info.name);
                    }
                    Err(error) => {
                        self.restore_failed_reconnect(
                            old_id.clone(),
                            metadata,
                            view,
                            seed_output,
                            source_index,
                            custom_name,
                            custom_color,
                            history,
                            error.to_string(),
                            was_active,
                            cx,
                        );
                    }
                }
            }
            SessionLaunchConfig::Ssh(config) => {
                // Keep the disconnected tab until the new SSH session registers.
                // Re-insert UI maps under the old id as disconnected, then start a
                // replacement session at the same tab index (old id is closed on success
                // when the new session activates and we close leftovers separately).
                let seed = if let Some(ref view) = view {
                    Some(view.output.clone())
                } else {
                    Some(seed_output.clone())
                };
                let mut keep = metadata.clone();
                keep.disconnected = true;
                self.session_metadata.insert(old_id.clone(), keep);
                if !self.session_order.iter().any(|id| id == &old_id) {
                    let index = source_index.min(self.session_order.len());
                    self.session_order.insert(index, old_id.clone());
                }
                if let Some(view) = view {
                    self.terminal_views.insert(old_id.clone(), view);
                } else {
                    self.terminal_views
                        .insert(old_id.clone(), TerminalViewState::from_output(seed_output));
                }
                if let Some(custom_name_keep) = custom_name.clone() {
                    self.session_custom_names
                        .insert(old_id.clone(), custom_name_keep);
                }
                if let Some(title_keep) = dynamic_title.clone() {
                    self.session_dynamic_titles
                        .insert(old_id.clone(), title_keep);
                }
                if let Some(cwd_keep) = session_cwd.clone() {
                    self.session_cwds.insert(old_id.clone(), cwd_keep);
                }
                if let Some(custom_color_keep) = custom_color {
                    self.session_tab_colors
                        .insert(old_id.clone(), custom_color_keep);
                }
                if let Some(history_keep) = history.clone() {
                    self.session_command_history
                        .insert(old_id.clone(), history_keep);
                }
                if was_active {
                    self.activate_session_id(&old_id);
                }
                self.pending_reconnect_replace_id = Some(old_id.clone());
                self.begin_background_ssh_start(
                    format!("{} reconnect", config.name),
                    config,
                    metadata.source_connection_id.clone(),
                    metadata.ai_execution_profile,
                    custom_name,
                    custom_color,
                    None,
                    Some(source_index),
                    seed,
                    None,
                    cx,
                );
            }
        }
        // Tauri clears busy when reconnect action returns (even if SSH still connecting).
        self.active_session_busy_actions.remove(&old_id);
        self.active_session_busy_actions
            .retain(|id, _| self.session_metadata.contains_key(id));
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        cx.notify();
    }

    fn restore_failed_reconnect(
        &mut self,
        old_id: String,
        mut metadata: SessionRuntimeMetadata,
        view: Option<TerminalViewState>,
        seed_output: String,
        source_index: usize,
        custom_name: Option<String>,
        custom_color: Option<u32>,
        history: Option<Vec<String>>,
        error: String,
        was_active: bool,
        cx: &mut Context<Self>,
    ) {
        self.active_session_busy_actions.remove(&old_id);

        metadata.disconnected = true;
        self.session_metadata.insert(old_id.clone(), metadata);
        if !self.session_order.iter().any(|id| id == &old_id) {
            let index = source_index.min(self.session_order.len());
            self.session_order.insert(index, old_id.clone());
        }
        if let Some(view) = view {
            self.terminal_views.insert(old_id.clone(), view);
        } else {
            self.terminal_views
                .insert(old_id.clone(), TerminalViewState::from_output(seed_output));
        }
        if let Some(custom_name) = custom_name {
            self.session_custom_names
                .insert(old_id.clone(), custom_name);
        }
        if let Some(custom_color) = custom_color {
            self.session_tab_colors.insert(old_id.clone(), custom_color);
        }
        if let Some(history) = history {
            self.session_command_history.insert(old_id.clone(), history);
        }
        if was_active {
            self.activate_session_id(&old_id);
        }
        self.terminal_status = format!("reconnect failed: {error}");
        cx.notify();
    }
}
