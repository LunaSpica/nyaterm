use super::*;

use crate::models::{MainMode, StartupCommandRequest};

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
        if self.has_pending_session_start() {
            self.terminal.view.status =
                "wait for the pending session to finish connecting".to_string();
            cx.notify();
            return;
        }
        let Some(source_session_id) = self.active_session_id.clone() else {
            self.terminal.view.status = "no active session to duplicate".to_string();
            cx.notify();
            return;
        };
        let Some(metadata) = self.session_metadata.get(&source_session_id).cloned() else {
            self.terminal.view.status = "active session cannot be duplicated".to_string();
            cx.notify();
            return;
        };
        let custom_name = self.session_custom_names.get(&source_session_id).cloned();
        let custom_color = self.session_tab_colors.get(&source_session_id).copied();

        match metadata.launch_config.clone() {
            SessionLaunchConfig::Local(mut config) => {
                self.apply_desired_geometry_to_local_config(&mut config);
                self.begin_background_session_start(
                    format!("{} duplicate", config.name),
                    SessionLaunchConfig::Local(config),
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
            SessionLaunchConfig::Telnet(config) => {
                self.begin_background_session_start(
                    format!("{} duplicate", config.name),
                    SessionLaunchConfig::Telnet(config),
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
            SessionLaunchConfig::Serial(config) => {
                self.begin_background_session_start(
                    format!("{} duplicate", config.name),
                    SessionLaunchConfig::Serial(config),
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
        if self.has_pending_session_start() {
            self.terminal.view.status =
                "wait for the pending session to finish connecting".to_string();
            cx.notify();
            return;
        }
        let Some(source_session_id) = self.active_session_id.clone() else {
            self.terminal.view.status = "no active SSH session to multiplex".to_string();
            cx.notify();
            return;
        };
        let Some(metadata) = self.session_metadata.get(&source_session_id).cloned() else {
            self.terminal.view.status = "active session cannot be multiplexed".to_string();
            cx.notify();
            return;
        };
        let SessionLaunchConfig::Ssh(config) = metadata.launch_config.clone() else {
            self.terminal.view.status = "active session is not SSH".to_string();
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
            self.terminal.view.status = "no active session to reconnect".to_string();
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
            self.terminal.view.status = "session action already in progress".to_string();
            cx.notify();
            return;
        }
        if self.is_session_disconnected(&session_id) {
            self.terminal.view.status = "session already disconnected".to_string();
            cx.notify();
            return;
        }
        if !self.session_metadata.contains_key(&session_id) {
            self.terminal.view.status = "session no longer exists".to_string();
            cx.notify();
            return;
        }

        self.active_session_busy_actions
            .insert(session_id.clone(), "disconnect".to_string());
        self.active_session_menu = None;
        // Backend may already be gone (race with Exited); still mark disconnected.
        let _ = self.session_manager.close(&session_id);
        self.cleanup_recording_for_session(&session_id);
        self.mark_session_disconnected(&session_id, cx);
        self.active_session_busy_actions.remove(&session_id);
        self.terminal.view.status = format!("disconnected {}", short_id(&session_id));
        cx.notify();
    }

    pub(in crate::features) fn mark_session_disconnected(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        self.clear_terminal_mouse_report_for_session(session_id);
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
        if let Some(view) = self.terminal.view.views.get_mut(session_id) {
            view.append_text(banner);
        } else {
            let mut view = TerminalViewState::new();
            view.set_encoding(&self.settings.interaction_default_encoding);
            view.append_text(banner);
            self.terminal
                .view
                .views
                .insert(session_id.to_string(), view);
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
            self.terminal.view.status = "session action already in progress".to_string();
            cx.notify();
            return;
        }
        if self.has_pending_session_start() {
            self.terminal.view.status =
                "wait for the pending session to finish connecting".to_string();
            cx.notify();
            return;
        }
        if !self.session_metadata.contains_key(&session_id) {
            self.terminal.view.status = "session cannot be reconnected".to_string();
            cx.notify();
            return;
        }
        self.active_session_busy_actions
            .insert(session_id.clone(), "reconnect".to_string());
        self.active_session_menu = None;
        self.active_pending_session_start = None;
        self.active_failed_session_start = None;
        let old_id = session_id;
        self.reconnect_session_failures.remove(&old_id);
        let source_index = self
            .session_order
            .iter()
            .position(|id| id == &old_id)
            .unwrap_or(self.session_order.len());
        let custom_name = self.session_custom_names.get(&old_id).cloned();
        let custom_color = self.session_tab_colors.get(&old_id).copied();
        let seed_output = self
            .terminal
            .view
            .views
            .get(&old_id)
            .map(|view| view.output.clone())
            .unwrap_or_default();

        // Tauri: write cyan reconnecting line into the buffer before recreating.
        if let Some(view) = self.terminal.view.views.get_mut(&old_id) {
            view.append_text(
                "
[36m[Reconnecting…][0m
",
            );
        }
        let seed_output = self
            .terminal
            .view
            .views
            .get(&old_id)
            .map(|view| view.output.clone())
            .unwrap_or(seed_output);

        // Close live backend if still present.
        let _ = self.session_manager.close(&old_id);
        self.cleanup_recording_for_session(&old_id);
        self.clear_terminal_mouse_report_for_session(&old_id);
        let Some(metadata) = self.session_metadata.get_mut(&old_id) else {
            self.active_session_busy_actions.remove(&old_id);
            self.terminal.view.status = "session cannot be reconnected".to_string();
            cx.notify();
            return;
        };
        metadata.disconnected = true;
        let launch_config = metadata.launch_config.clone();
        let source_connection_id = metadata.source_connection_id.clone();
        let ai_execution_profile = metadata.ai_execution_profile;
        let seed = Some(seed_output);
        self.pending_reconnect_replace_id = Some(old_id.clone());

        match launch_config {
            SessionLaunchConfig::Local(mut config) => {
                self.apply_desired_geometry_to_local_config(&mut config);
                self.begin_background_session_start(
                    format!("{} reconnect", config.name),
                    SessionLaunchConfig::Local(config),
                    source_connection_id,
                    ai_execution_profile,
                    custom_name,
                    custom_color,
                    None,
                    Some(source_index),
                    seed,
                    None,
                    cx,
                );
            }
            SessionLaunchConfig::Telnet(config) => {
                self.begin_background_session_start(
                    format!("{} reconnect", config.name),
                    SessionLaunchConfig::Telnet(config),
                    source_connection_id,
                    ai_execution_profile,
                    custom_name,
                    custom_color,
                    None,
                    Some(source_index),
                    seed,
                    None,
                    cx,
                );
            }
            SessionLaunchConfig::Serial(config) => {
                self.begin_background_session_start(
                    format!("{} reconnect", config.name),
                    SessionLaunchConfig::Serial(config),
                    source_connection_id,
                    ai_execution_profile,
                    custom_name,
                    custom_color,
                    None,
                    Some(source_index),
                    seed,
                    None,
                    cx,
                );
            }
            SessionLaunchConfig::Ssh(config) => {
                self.begin_background_ssh_start(
                    format!("{} reconnect", config.name),
                    config,
                    source_connection_id,
                    ai_execution_profile,
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

    pub(in crate::features) fn migrate_reconnected_session_state(
        &mut self,
        old_id: &str,
        new_id: &str,
    ) {
        self.reconnect_session_failures.remove(old_id);
        if let Some(bounds) = self.terminal.layout.session_surface_bounds.remove(old_id) {
            self.terminal
                .layout
                .session_surface_bounds
                .insert(new_id.to_string(), bounds);
        }
        if !self.session_custom_names.contains_key(new_id) {
            if let Some(custom_name) = self.session_custom_names.remove(old_id) {
                self.session_custom_names
                    .insert(new_id.to_string(), custom_name);
            }
        }
        if let Some(title) = self.session_dynamic_titles.remove(old_id) {
            self.session_dynamic_titles
                .insert(new_id.to_string(), title);
        }
        if let Some(cwd) = self.session_cwds.remove(old_id) {
            self.session_cwds.insert(new_id.to_string(), cwd);
        }
        if !self.session_tab_colors.contains_key(new_id) {
            if let Some(color) = self.session_tab_colors.remove(old_id) {
                self.session_tab_colors.insert(new_id.to_string(), color);
            }
        }
        if let Some(history) = self.session_command_history.remove(old_id) {
            self.session_command_history
                .insert(new_id.to_string(), history);
        }

        let mut pane_roots = std::mem::take(&mut self.session_pane_roots);
        for root in pane_roots.values_mut() {
            root.replace_session_id(old_id, new_id);
        }
        if let Some(root) = pane_roots.remove(old_id) {
            pane_roots.insert(new_id.to_string(), root);
        }
        self.session_pane_roots = pane_roots;
        if let Some(root) = self.workspace_split.as_mut() {
            root.replace_session_id(old_id, new_id);
        }
        if let Some(root) = self.terminal.windows.tree.as_mut() {
            root.replace_tab_id(old_id, new_id);
        }
        for group in &mut self.sync_groups {
            for session_id in &mut group.session_ids {
                if session_id == old_id {
                    *session_id = new_id.to_string();
                }
            }
            if group.paused_session_ids.iter().any(|id| id == old_id) {
                group.paused_session_ids.retain(|id| id != old_id);
                if !group.paused_session_ids.iter().any(|id| id == new_id) {
                    group.paused_session_ids.push(new_id.to_string());
                }
            }
        }
        if self.active_session_id.as_deref() == Some(old_id) {
            self.activate_session_id(new_id);
        }
        self.rebuild_session_tab_owners();
        self.sync_workspace_split_from_active_tab();
    }
}
