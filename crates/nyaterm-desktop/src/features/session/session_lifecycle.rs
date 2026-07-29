use gpui::{Context, Window};
use nyaterm_core::TerminalInputState;

use crate::features::NyaTermApp;
use crate::features::formatting::{short_id, ssh_multiplex_key};
use crate::models::{
    MainMode, NavItem, SessionLaunchConfig, StartupCommandRequest, TerminalViewState,
};

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
        let Some(source_session_id) = self.session.active_id_owned() else {
            self.terminal.view.status = "no active session to duplicate".to_string();
            cx.notify();
            return;
        };
        let Some(metadata) = self.session.metadata(&source_session_id).cloned() else {
            self.terminal.view.status = "active session cannot be duplicated".to_string();
            cx.notify();
            return;
        };
        let custom_name = self
            .session
            .custom_name(&source_session_id)
            .map(str::to_string);
        let custom_color = self.session.tab_color(&source_session_id);

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
        self.shell.navigation.selected_nav = NavItem::Workspace;
        self.shell.navigation.main_mode = MainMode::Workspace;
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
        let Some(source_session_id) = self.session.active_id_owned() else {
            self.terminal.view.status = "no active SSH session to multiplex".to_string();
            cx.notify();
            return;
        };
        let Some(metadata) = self.session.metadata(&source_session_id).cloned() else {
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
        let existing_multiplex = self.session.reusable_multiplex_handle(&multiplex_key);
        let custom_name = self
            .session
            .custom_name(&source_session_id)
            .map(str::to_string);
        let custom_color = self.session.tab_color(&source_session_id);
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
        self.shell.navigation.selected_nav = NavItem::Workspace;
        self.shell.navigation.main_mode = MainMode::Workspace;
        cx.notify();
    }

    pub(in crate::features) fn reconnect_active_session(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(source_session_id) = self.session.active_id_owned() else {
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
        if self.session.session_is_busy(&session_id) {
            self.terminal.view.status = "session action already in progress".to_string();
            cx.notify();
            return;
        }
        if self.is_session_disconnected(&session_id) {
            self.terminal.view.status = "session already disconnected".to_string();
            cx.notify();
            return;
        }
        if !self.session.has_session(&session_id) {
            self.terminal.view.status = "session no longer exists".to_string();
            cx.notify();
            return;
        }

        self.session.begin_disconnect_action(session_id.clone());
        // Backend may already be gone (race with Exited); still mark disconnected.
        let _ = self.session.manager().close(&session_id);
        self.cleanup_recording_for_session(&session_id);
        self.mark_session_disconnected(&session_id, cx);
        self.session.finish_busy_action(&session_id);
        self.terminal.view.status = format!("disconnected {}", short_id(&session_id));
        cx.notify();
    }

    pub(in crate::features) fn mark_session_disconnected(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        self.clear_terminal_mouse_report_for_session(session_id);
        let Some(update) = self.session.mark_session_disconnected(session_id) else {
            return;
        };
        if update.already_disconnected {
            return;
        }
        // Drop multiplex handle association for this session key if unused.
        if let Some(multiplex_key) = update.multiplex_key {
            if let Some(handle) = self
                .session
                .take_multiplex_handle_if_no_other_live_reference(session_id, &multiplex_key)
            {
                super::disconnect_multiplex_handle(handle);
            }
        }

        let banner = "\r\n\x1b[31m[Session disconnected]\x1b[0m\r\n\x1b[33m[Press Enter to reconnect]\x1b[0m\r\n";
        if let Some(view) = self.terminal.view.views.get_mut(session_id) {
            view.append_text(banner);
        } else {
            let mut view = TerminalViewState::new();
            view.set_encoding(&self.settings.summary.interaction_default_encoding);
            view.append_text(banner);
            self.terminal
                .view
                .views
                .insert(session_id.to_string(), view);
        }

        if self.session.active_id() == Some(session_id) {
            self.terminal.assist.command_input_tracker = TerminalInputState::new();
            self.terminal.assist.command_suggestions = None;
            self.terminal.assist.credential_suggestions = None;
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
        if self.session.session_is_busy(&session_id) {
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
        if !self.session.has_session(&session_id) {
            self.terminal.view.status = "session cannot be reconnected".to_string();
            cx.notify();
            return;
        }
        self.session.begin_reconnect_action(session_id.clone());
        self.session.start.clear_active_selection();
        let old_id = session_id;
        self.session.start.clear_reconnect_failure(&old_id);
        let source_index = self
            .session
            .session_index(&old_id)
            .unwrap_or(self.session.session_order_len());
        let custom_name = self.session.custom_name(&old_id).map(str::to_string);
        let custom_color = self.session.tab_color(&old_id);
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
        let _ = self.session.manager().close(&old_id);
        self.cleanup_recording_for_session(&old_id);
        self.clear_terminal_mouse_report_for_session(&old_id);
        let Some(metadata) = self.session.metadata(&old_id).cloned() else {
            self.session.finish_busy_action(&old_id);
            self.terminal.view.status = "session cannot be reconnected".to_string();
            cx.notify();
            return;
        };
        self.session.mark_session_disconnected(&old_id);
        let launch_config = metadata.launch_config;
        let source_connection_id = metadata.source_connection_id;
        let ai_execution_profile = metadata.ai_execution_profile;
        let seed = Some(seed_output);
        self.session.start.set_reconnect_target(old_id.clone());

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
        self.session.finish_busy_action(&old_id);
        self.session.retain_busy_actions_for_live_sessions();
        self.shell.navigation.selected_nav = NavItem::Workspace;
        self.shell.navigation.main_mode = MainMode::Workspace;
        cx.notify();
    }

    pub(in crate::features) fn migrate_reconnected_session_state(
        &mut self,
        old_id: &str,
        new_id: &str,
    ) {
        self.session.start.clear_reconnect_failure(old_id);
        if let Some(bounds) = self.terminal.layout.session_surface_bounds.remove(old_id) {
            self.terminal
                .layout
                .session_surface_bounds
                .insert(new_id.to_string(), bounds);
        }
        self.session.migrate_session_presentation(old_id, new_id);

        self.shell.workspace.replace_session_id(old_id, new_id);
        if let Some(root) = self.terminal.windows.tree.as_mut() {
            root.replace_tab_id(old_id, new_id);
        }
        self.sync_input.replace_session_id(old_id, new_id);
        if self.session.active_id() == Some(old_id) {
            self.activate_session_id(new_id);
        }
        self.sync_workspace_split_from_active_tab();
    }
}
