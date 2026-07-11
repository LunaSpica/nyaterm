use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn session_display_name_by_info(
        &self,
        session: &SessionInfo,
    ) -> String {
        self.session_custom_names
            .get(&session.id)
            .filter(|name| !name.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| session.name.clone())
    }

    pub(in crate::ui::view) fn session_display_name(&self, session_id: &str) -> Option<String> {
        if let Some(name) = self
            .session_custom_names
            .get(session_id)
            .filter(|name| !name.trim().is_empty())
        {
            return Some(name.clone());
        }
        self.session_manager
            .list_sessions()
            .ok()?
            .into_iter()
            .find(|session| session.id == session_id)
            .map(|session| session.name)
    }

    pub(in crate::ui::view) fn session_endpoint(&self, session_id: &str) -> Option<String> {
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

    pub(in crate::ui::view) fn session_ssh_host(&self, session_id: &str) -> Option<String> {
        let metadata = self.session_metadata.get(session_id)?;
        match &metadata.launch_config {
            SessionLaunchConfig::Ssh(config) if !config.host.trim().is_empty() => {
                Some(config.host.clone())
            }
            _ => None,
        }
    }

    pub(in crate::ui::view) fn session_ssh_address(&self, session_id: &str) -> Option<String> {
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

    fn active_session_info_line(&self) -> Option<String> {
        let session_id = self.active_session_id.as_deref()?;
        let name = self.session_display_name(session_id)?;
        let session = self
            .session_manager
            .list_sessions()
            .ok()?
            .into_iter()
            .find(|session| session.id == session_id)?;
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

    pub(in crate::ui::view) fn active_session_info_details(
        &self,
    ) -> Option<Vec<(&'static str, String)>> {
        let session_id = self.active_session_id.as_deref()?;
        let name = self.session_display_name(session_id)?;
        let session = self
            .session_manager
            .list_sessions()
            .ok()?
            .into_iter()
            .find(|session| session.id == session_id)?;
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

    pub(in crate::ui::view) fn activate_session_id(&mut self, session_id: &str) {
        // Session switch resets terminal-output credential autofill (Tauri XTerminal remount).
        self.credential_suggestions = None;
        self.credential_autofill_buffer.clear();
        self.credential_autofill_recent.clear();
        self.credential_autofill_pending = None;
        self.credential_autofill_sending = false;
        self.credential_prompt_input_until_ms = 0;
        self.command_input_tracker = TerminalInputState::new();
        self.command_suggestions = None;
        let previous_session_id = self.active_session_id.clone();
        if previous_session_id.as_deref() != Some(session_id)
            && let Some(previous_session_id) = previous_session_id.as_deref()
        {
            self.cache_transfer_browser_session(previous_session_id);
        }

        self.active_session_id = Some(session_id.to_string());
        self.transfer_auto_sync_cwd_last_at = None;
        if let Some(metadata) = self.session_metadata.get(session_id).cloned() {
            self.active_ssh_config = metadata.ssh_config;
            self.active_ai_execution_profile = metadata.ai_execution_profile;
        } else {
            self.active_ssh_config = None;
            self.active_ai_execution_profile = AiExecutionProfile::SendOnly;
        }
        self.sync_transfer_browser_favorites_for_active_session();
        if !self.restore_transfer_browser_session_cache(session_id) {
            self.reset_transfer_browser_for_active_session();
        }
        if let Some(view) = self.terminal_views.get_mut(session_id) {
            view.has_unread = false;
            self.terminal_output = view.output.clone();
            self.terminal_screen = terminal_screen_from_output(&view.output);
        } else {
            self.terminal_output.clear();
            self.terminal_screen.clear();
        }
    }

    pub(in crate::ui::view) fn select_session(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let exists = self
            .session_manager
            .list_sessions()
            .unwrap_or_default()
            .into_iter()
            .any(|session| session.id == session_id);
        if !exists {
            self.terminal_status = "session no longer exists".to_string();
            self.remove_session_state(&session_id);
            cx.notify();
            return;
        }
        self.activate_session_id(&session_id);
        self.terminal_status = format!("active {}", short_id(&session_id));
        self.selected_nav = NavItem::Workspace;
        cx.notify();
    }

    pub(in crate::ui::view) fn select_relative_session(
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
        self.activate_session_id(&session_id);
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        self.terminal_status = format!("active {}", short_id(&session_id));
        cx.notify();
    }

    pub(in crate::ui::view) fn select_session_index(
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
        self.activate_session_id(&session_id);
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        self.terminal_status = format!("active {}", short_id(&session_id));
        cx.notify();
    }

    pub(in crate::ui::view) fn open_tab_actions(
        &mut self,
        session_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_session(session_id.clone(), cx);
        if self.active_session_id.as_deref() != Some(session_id.as_str()) {
            return;
        }
        self.tab_actions_session_id = Some(session_id);
        self.terminal_status = "tab actions opened".to_string();
        window.focus(&self.tab_actions_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn close_tab_actions(&mut self, cx: &mut Context<Self>) {
        self.tab_actions_session_id = None;
        self.terminal_status = "tab actions closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn copy_active_session_name(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn copy_active_session_endpoint(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn copy_active_session_ssh_host(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn copy_active_session_ssh_address(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn open_active_session_info(
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

    pub(in crate::ui::view) fn close_active_session_info(&mut self, cx: &mut Context<Self>) {
        self.session_info_open = false;
        self.terminal_status = "session info closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn copy_active_session_info(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn open_tab_color_picker(
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

    pub(in crate::ui::view) fn close_tab_color_picker(&mut self, cx: &mut Context<Self>) {
        self.color_picker_open = false;
        self.terminal_status = "tab color picker closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn set_active_session_tab_color(
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
