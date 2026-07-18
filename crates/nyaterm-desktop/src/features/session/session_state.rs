use super::*;

impl NyaTermApp {
    pub(in crate::features) fn session_display_name_by_info(
        &self,
        session: &SessionInfo,
    ) -> String {
        if let Some(name) = self
            .session_custom_names
            .get(&session.id)
            .filter(|name| !name.trim().is_empty())
        {
            return name.clone();
        }
        if let Some(name) = self
            .session_dynamic_titles
            .get(&session.id)
            .filter(|name| !name.trim().is_empty())
        {
            return name.clone();
        }
        session.name.clone()
    }

    pub(in crate::features) fn session_display_name(&self, session_id: &str) -> Option<String> {
        if let Some(name) = self
            .session_custom_names
            .get(session_id)
            .filter(|name| !name.trim().is_empty())
        {
            return Some(name.clone());
        }
        if let Some(name) = self
            .session_dynamic_titles
            .get(session_id)
            .filter(|name| !name.trim().is_empty())
        {
            return Some(name.clone());
        }
        // Prefer local metadata — never take the transport session map lock for chrome.
        self.session_info(session_id).map(|session| session.name)
    }

    pub(in crate::features) fn session_endpoint(&self, session_id: &str) -> Option<String> {
        let metadata = self.session_metadata.get(session_id)?;
        match &metadata.launch_config {
            SessionLaunchConfig::Local(config) => {
                let shell = config
                    .shell_path
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or("system shell");
                Some(match &config.working_dir {
                    Some(dir) => format!("{shell} in {}", dir.display()),
                    None => shell.to_string(),
                })
            }
            SessionLaunchConfig::Ssh(config) => Some(format!(
                "{}@{}:{}",
                config.username, config.host, config.port
            )),
            SessionLaunchConfig::Telnet(config) => Some(format!("{}:{}", config.host, config.port)),
            SessionLaunchConfig::Serial(config) => Some(format!(
                "{} @ {} {}{}{}",
                config.port_name,
                config.baud_rate,
                config.data_bits,
                config.parity,
                config.stop_bits
            )),
        }
    }

    pub(in crate::features) fn session_ssh_host(&self, session_id: &str) -> Option<String> {
        let metadata = self.session_metadata.get(session_id)?;
        match &metadata.launch_config {
            SessionLaunchConfig::Ssh(config) if !config.host.trim().is_empty() => {
                Some(config.host.clone())
            }
            _ => None,
        }
    }

    pub(in crate::features) fn session_ssh_address(&self, session_id: &str) -> Option<String> {
        let metadata = self.session_metadata.get(session_id)?;
        match &metadata.launch_config {
            SessionLaunchConfig::Ssh(config)
                if !config.username.trim().is_empty() && !config.host.trim().is_empty() =>
            {
                Some(format!(
                    "ssh -p {} {}@{}",
                    config.port, config.username, config.host
                ))
            }
            _ => None,
        }
    }

    /// Lines for tab hover tooltip (endpoint + SSH address when available).
    pub(in crate::features) fn session_tab_tooltip_lines(&self, session_id: &str) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(endpoint) = self.session_endpoint(session_id) {
            lines.push(endpoint);
        }
        if let Some(address) = self.session_ssh_address(session_id) {
            if lines.first().map(String::as_str) != Some(address.as_str()) {
                lines.push(address);
            }
        }
        if self.is_session_disconnected(session_id) {
            lines.push("Disconnected — press Enter to reconnect".to_string());
        }
        if let Some(cwd) = self.session_cwds.get(session_id) {
            if !cwd.trim().is_empty() {
                lines.push(format!("cwd {cwd}"));
            }
        }
        lines
    }

    fn active_session_info_line(&self) -> Option<String> {
        let session_id = self.active_session_id.as_deref()?;
        let name = self.session_display_name(session_id)?;
        let session = self.session_info(session_id)?;
        let endpoint = self
            .session_endpoint(session_id)
            .unwrap_or_else(|| "unknown endpoint".to_string());
        Some(format!(
            "{} · {} · {} · {}x{} · {}",
            name,
            session_kind_label(session.kind),
            short_id(session_id),
            session.cols,
            session.rows,
            endpoint
        ))
    }

    pub(in crate::features) fn active_session_info_details(
        &self,
    ) -> Option<Vec<(&'static str, String)>> {
        let session_id = self.active_session_id.as_deref()?;
        let name = self.session_display_name(session_id)?;
        let session = self.session_info(session_id)?;
        let metadata = self.session_metadata.get(session_id)?;
        let endpoint = self
            .session_endpoint(session_id)
            .unwrap_or_else(|| "unknown endpoint".to_string());

        let mut details = vec![
            ("Name", name),
            ("Kind", session_kind_label(session.kind).to_string()),
            ("Session ID", session_id.to_string()),
            ("Size", format!("{} x {}", session.cols, session.rows)),
            ("Endpoint", endpoint),
            ("AI Profile", format!("{:?}", metadata.ai_execution_profile)),
        ];
        if let Some(cwd) = self.session_cwds.get(session_id) {
            details.push(("CWD (OSC 7)", cwd.clone()));
        }

        match &metadata.launch_config {
            SessionLaunchConfig::Local(config) => {
                details.push(("Launch", "Local shell".to_string()));
                if let Some(shell) = config
                    .shell_path
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    details.push(("Shell", shell.to_string()));
                }
                if let Some(dir) = config.working_dir.as_ref() {
                    details.push(("Working Dir", dir.display().to_string()));
                }
            }
            SessionLaunchConfig::Ssh(config) => {
                details.push(("Launch", "SSH".to_string()));
                details.push(("Host", config.host.clone()));
                details.push(("Port", config.port.to_string()));
                details.push(("Username", config.username.clone()));
                if let Some(address) = self.session_ssh_address(session_id) {
                    details.push(("SSH Address", address));
                }
                if let Some(proxy_jump) = config.proxy_jump.as_ref() {
                    details.push((
                        "Proxy Jump",
                        format!(
                            "{}@{}:{}",
                            proxy_jump.username, proxy_jump.host, proxy_jump.port
                        ),
                    ));
                }
            }
            SessionLaunchConfig::Telnet(config) => {
                details.push(("Launch", "Telnet".to_string()));
                details.push(("Host", config.host.clone()));
                details.push(("Port", config.port.to_string()));
            }
            SessionLaunchConfig::Serial(config) => {
                details.push(("Launch", "Serial".to_string()));
                details.push(("Port", config.port_name.clone()));
                details.push(("Baud", config.baud_rate.to_string()));
                details.push((
                    "Frame",
                    format!("{}{}{}", config.data_bits, config.parity, config.stop_bits),
                ));
            }
        }

        Some(details)
    }

    pub(in crate::features) fn activate_session_id(&mut self, session_id: &str) -> Option<String> {
        self.open_tabs_menu_open = false;
        self.new_session_menu_open = false;
        // Session switch resets terminal-output credential autofill (Tauri XTerminal remount).
        self.credential_suggestions = None;
        self.credential_autofill_buffer.clear();
        self.credential_autofill_recent.clear();
        self.credential_autofill_pending = None;
        self.credential_autofill_sending = false;
        self.credential_prompt_input_until_ms = 0;
        self.command_input_tracker = TerminalInputState::new();
        self.command_suggestions = None;
        self.command_suggestions_suppressed = false;
        self.pending_command_history_entry = None;
        self.command_suggestion_search_gen = self.command_suggestion_search_gen.saturating_add(1);
        let previous_session_id = self.active_session_id.clone();
        let switching_sessions = previous_session_id.as_deref() != Some(session_id);
        if previous_session_id.as_deref() != Some(session_id)
            && let Some(previous_session_id) = previous_session_id.as_deref()
        {
            self.cache_transfer_browser_session(previous_session_id);
        }

        self.active_session_id = Some(session_id.to_string());
        if switching_sessions {
            self.transfer_selected_job_id = None;
            self.transfer_job_menu = None;
            self.transfer_job_delete = None;
        }
        self.session_tab_scroll_into_view_pending = true;
        // Keep workspace_split mirrored to the active tab's per-tab pane root.
        self.sync_workspace_split_from_active_tab();
        self.transfer_auto_sync_cwd_last_at = None;
        if let Some(metadata) = self.session_metadata.get(session_id).cloned() {
            self.active_ssh_config = metadata.ssh_config;
            self.active_ai_execution_profile = metadata.ai_execution_profile;
        } else {
            self.active_ssh_config = None;
            self.active_ai_execution_profile = AiExecutionProfile::SendOnly;
        }
        // Transfer browser state is only needed when the transfers panel is open
        // or we already have cached browser state for this session. Skipping the
        // full reset on every activate keeps connect/switch chrome responsive.
        let transfers_panel_visible = self.active_left_panel == Some(NavItem::Transfers)
            || self.active_right_panel == Some(NavItem::Transfers)
            || self.selected_nav == NavItem::Transfers;
        if transfers_panel_visible
            || self.transfer_browser_session_cache.contains_key(session_id)
            || !self.transfer_browser_entries.is_empty()
        {
            self.sync_transfer_browser_favorites_for_active_session();
            if !self.restore_transfer_browser_session_cache(session_id) {
                self.reset_transfer_browser_for_active_session();
            }
        } else {
            // Keep favorites map coherent for the active connection without wiping UI.
            self.sync_transfer_browser_favorites_for_active_session();
        }
        if let Some(view) = self.terminal_views.get_mut(session_id) {
            view.has_unread = false;
        } else {
            self.terminal_output.clear();
            self.terminal_output_decoder.reset_decoder();
            self.terminal_screen.clear();
        }
        if switching_sessions && self.terminal_focus_active {
            if let Some(previous_session_id) = previous_session_id.as_deref() {
                self.write_terminal_focus_report_to_session(previous_session_id, false);
            }
            self.write_terminal_focus_report_to_session(session_id, true);
        }
        self.sync_terminal_windows_active_tab(session_id);
        // Priority was refreshed via sync_workspace_split_from_active_tab.
        // Recover paint immediately if this tab was backgrounded without grids.
        if self
            .terminal_views
            .get(session_id)
            .is_some_and(|view| view.frame_snapshot.is_none())
        {
            self.request_terminal_live_snapshot(session_id);
        }
        previous_session_id
    }

    pub(in crate::features) fn activate_session_id_with_surface_sync(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        let previous_session_id = self.activate_session_id(session_id);
        self.sync_terminal_activation_surfaces(previous_session_id, session_id, cx);
    }

    fn sync_terminal_activation_surfaces(
        &mut self,
        previous_session_id: Option<String>,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        let notify_session_ids =
            terminal_activation_surface_notify_ids(previous_session_id.as_deref(), session_id);
        if notify_session_ids.is_empty() {
            return;
        }
        let chrome_changed = self.clear_terminal_activation_interaction_state();
        for session_id in notify_session_ids {
            self.notify_terminal_surface_only(Some(session_id.as_str()), cx);
        }
        if chrome_changed {
            cx.notify();
        }
    }

    fn clear_terminal_activation_interaction_state(&mut self) -> bool {
        let mut chrome_changed = false;
        if self.terminal_selection.take().is_some() {
            chrome_changed = true;
        }
        if self.terminal_selection_dragging {
            self.terminal_selection_dragging = false;
            chrome_changed = true;
        }
        if self.terminal_context_menu.take().is_some() {
            chrome_changed = true;
        }
        if self.action_link_menu.take().is_some() {
            chrome_changed = true;
        }
        if self.action_link_tooltip.take().is_some() {
            chrome_changed = true;
        }
        self.action_link_hover_pending = None;
        chrome_changed
    }

    pub(in crate::features) fn select_session(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        // Local metadata is authoritative for tab existence; transport lock not needed.
        let known = self.session_metadata.contains_key(&session_id);
        let disconnected = self.is_session_disconnected(&session_id);
        if !known && !disconnected {
            self.terminal_status = "session no longer exists".to_string();
            self.remove_session_state(&session_id);
            cx.notify();
            return;
        }
        // Strip selection targets tab roots; focus the preferred leaf under that tab.
        let focus_id = if !self.is_secondary_pane_session(&session_id) {
            self.active_pane_for_tab_root(&session_id)
        } else {
            session_id.clone()
        };
        let disconnected = self.is_session_disconnected(&focus_id) || disconnected;
        self.activate_session_id_with_surface_sync(&focus_id, cx);
        self.terminal_status = if disconnected {
            format!("disconnected {}", short_id(&focus_id))
        } else {
            format!("active {}", short_id(&focus_id))
        };
        self.selected_nav = NavItem::Workspace;
        cx.notify();
    }

    pub(in crate::features) fn select_relative_session(
        &mut self,
        offset: isize,
        cx: &mut Context<Self>,
    ) {
        let sessions = self.ordered_sessions();
        if sessions.is_empty() {
            self.terminal_status = "no sessions to switch".to_string();
            cx.notify();
            return;
        }
        let active_index = self
            .active_session_id
            .as_deref()
            .and_then(|active_id| {
                sessions
                    .iter()
                    .position(|session| session.id.as_str() == active_id)
            })
            .unwrap_or(0);
        let len = sessions.len() as isize;
        let next_index = (active_index as isize + offset).rem_euclid(len) as usize;
        let session_id = sessions[next_index].id.clone();
        self.activate_session_id_with_surface_sync(&session_id, cx);
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        self.terminal_status = format!("active {}", short_id(&session_id));
        cx.notify();
    }

    pub(in crate::features) fn select_session_index(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let sessions = self.ordered_sessions();
        if sessions.is_empty() {
            self.terminal_status = "no sessions to switch".to_string();
            cx.notify();
            return;
        }
        let index = index.min(sessions.len().saturating_sub(1));
        let session_id = sessions[index].id.clone();
        self.activate_session_id_with_surface_sync(&session_id, cx);
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        self.terminal_status = format!("active {}", short_id(&session_id));
        cx.notify();
    }

    pub(in crate::features) fn open_tab_actions(
        &mut self,
        session_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_tab_actions_at(session_id, None, window, cx);
    }

    pub(in crate::features) fn toggle_open_tabs_menu(&mut self, cx: &mut Context<Self>) {
        self.open_tabs_menu_open = !self.open_tabs_menu_open;
        if self.open_tabs_menu_open {
            self.new_session_menu_open = false;
            self.title_menu_open = None;
        }
        cx.notify();
    }

    pub(in crate::features) fn close_open_tabs_menu(&mut self, cx: &mut Context<Self>) {
        if self.open_tabs_menu_open {
            self.open_tabs_menu_open = false;
            cx.notify();
        }
    }

    pub(in crate::features) fn toggle_new_session_menu(&mut self, cx: &mut Context<Self>) {
        self.new_session_menu_open = !self.new_session_menu_open;
        if self.new_session_menu_open {
            self.open_tabs_menu_open = false;
            self.title_menu_open = None;
        }
        cx.notify();
    }

    pub(in crate::features) fn close_new_session_menu(&mut self, cx: &mut Context<Self>) {
        if self.new_session_menu_open {
            self.new_session_menu_open = false;
            cx.notify();
        }
    }

    pub(in crate::features) fn open_tab_actions_at(
        &mut self,
        session_id: String,
        anchor: Option<(f32, f32)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_session(session_id.clone(), cx);
        if self.active_session_id.as_deref() != Some(session_id.as_str()) {
            return;
        }
        self.tab_actions_session_id = Some(session_id);
        self.tab_actions_anchor = anchor;
        self.terminal_status = "tab actions opened".to_string();
        window.focus(&self.tab_actions_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_tab_actions(&mut self, cx: &mut Context<Self>) {
        self.tab_actions_session_id = None;
        self.tab_actions_anchor = None;
        self.terminal_status = "tab actions closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn copy_active_session_name(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.active_session_id.as_deref() else {
            self.terminal_status = "no active session name to copy".to_string();
            cx.notify();
            return;
        };
        let Some(name) = self.session_display_name(session_id) else {
            self.terminal_status = "active session name is unavailable".to_string();
            cx.notify();
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(name.clone()));
        self.terminal_status = format!("copied tab name '{name}'");
        cx.notify();
    }

    pub(in crate::features) fn copy_active_session_endpoint(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.active_session_id.as_deref() else {
            self.terminal_status = "no active session endpoint to copy".to_string();
            cx.notify();
            return;
        };
        let Some(endpoint) = self.session_endpoint(session_id) else {
            self.terminal_status = "active session endpoint is unavailable".to_string();
            cx.notify();
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(endpoint.clone()));
        self.terminal_status = format!("copied endpoint '{endpoint}'");
        cx.notify();
    }

    pub(in crate::features) fn copy_active_session_ssh_host(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.active_session_id.as_deref() else {
            self.terminal_status = "no active SSH host to copy".to_string();
            cx.notify();
            return;
        };
        let Some(host) = self.session_ssh_host(session_id) else {
            self.terminal_status = "active session is not SSH".to_string();
            cx.notify();
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(host.clone()));
        self.terminal_status = format!("copied SSH host '{host}'");
        cx.notify();
    }

    pub(in crate::features) fn copy_active_session_ssh_address(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.active_session_id.as_deref() else {
            self.terminal_status = "no active SSH address to copy".to_string();
            cx.notify();
            return;
        };
        let Some(address) = self.session_ssh_address(session_id) else {
            self.terminal_status = "active session is not SSH".to_string();
            cx.notify();
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(address.clone()));
        self.terminal_status = format!("copied SSH address '{address}'");
        cx.notify();
    }

    pub(in crate::features) fn open_active_session_info(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_session_id.is_none() {
            self.terminal_status = "no active session info".to_string();
            cx.notify();
            return;
        }
        self.session_info_open = true;
        self.terminal_status = self
            .active_session_info_line()
            .unwrap_or_else(|| "session info opened".to_string());
        window.focus(&self.session_info_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_active_session_info(&mut self, cx: &mut Context<Self>) {
        self.session_info_open = false;
        self.terminal_status = "session info closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn copy_active_session_info(&mut self, cx: &mut Context<Self>) {
        let Some(details) = self.active_session_info_details() else {
            self.terminal_status = "no active session info to copy".to_string();
            cx.notify();
            return;
        };
        let text = details
            .into_iter()
            .map(|(label, value)| format!("{label}: {value}"))
            .collect::<Vec<_>>()
            .join("\n");
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.terminal_status = "copied session info".to_string();
        cx.notify();
    }

    pub(in crate::features) fn open_tab_color_picker(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_session_id.is_none() {
            self.terminal_status = "select a session before setting tab color".to_string();
            cx.notify();
            return;
        }
        self.color_picker_open = true;
        self.terminal_status = "tab color picker opened".to_string();
        window.focus(&self.color_picker_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_tab_color_picker(&mut self, cx: &mut Context<Self>) {
        self.color_picker_open = false;
        self.terminal_status = "tab color picker closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn set_active_session_tab_color(
        &mut self,
        color: Option<u32>,
        cx: &mut Context<Self>,
    ) {
        let Some(session_id) = self.active_session_id.clone() else {
            self.terminal_status = "no active session color to set".to_string();
            cx.notify();
            return;
        };
        match color {
            Some(color) => {
                self.session_tab_colors.insert(session_id, color);
                self.terminal_status = "tab color updated".to_string();
            }
            None => {
                self.session_tab_colors.remove(&session_id);
                self.terminal_status = "tab color reset".to_string();
            }
        }
        self.color_picker_open = false;
        cx.notify();
    }
}

fn terminal_activation_surface_notify_ids(
    previous_session_id: Option<&str>,
    session_id: &str,
) -> Vec<String> {
    if previous_session_id == Some(session_id) {
        return Vec::new();
    }
    let mut ids = Vec::with_capacity(2);
    if let Some(previous_session_id) = previous_session_id.filter(|id| !id.is_empty()) {
        ids.push(previous_session_id.to_string());
    }
    if !session_id.is_empty() {
        ids.push(session_id.to_string());
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_surface_notify_skips_unchanged_session() {
        assert!(terminal_activation_surface_notify_ids(Some("a"), "a").is_empty());
    }

    #[test]
    fn activation_surface_notify_targets_previous_and_current_sessions() {
        assert_eq!(
            terminal_activation_surface_notify_ids(Some("old"), "new"),
            vec!["old".to_string(), "new".to_string()]
        );
    }

    #[test]
    fn activation_surface_notify_ignores_empty_session_ids() {
        assert_eq!(
            terminal_activation_surface_notify_ids(None, "new"),
            vec!["new".to_string()]
        );
        assert_eq!(
            terminal_activation_surface_notify_ids(Some("old"), ""),
            vec!["old".to_string()]
        );
    }
}
