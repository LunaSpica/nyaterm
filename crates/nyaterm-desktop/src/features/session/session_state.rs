use gpui::{ClipboardItem, Context, Window};
use nyaterm_core::AiExecutionProfile;
use nyaterm_transport::SessionInfo;

use crate::features::NyaTermApp;
use crate::features::formatting::{session_kind_label, short_id};
use crate::models::{MainMode, NavItem, SessionLaunchConfig};

impl NyaTermApp {
    pub(in crate::features) fn session_display_name_by_info(
        &self,
        session: &SessionInfo,
    ) -> String {
        if let Some(name) = self
            .session
            .custom_names
            .get(&session.id)
            .filter(|name| !name.trim().is_empty())
        {
            return name.clone();
        }
        if let Some(name) = self
            .session
            .dynamic_titles
            .get(&session.id)
            .filter(|name| !name.trim().is_empty())
        {
            return name.clone();
        }
        session.name.clone()
    }

    pub(in crate::features) fn session_display_name(&self, session_id: &str) -> Option<String> {
        if let Some(name) = self
            .session
            .custom_names
            .get(session_id)
            .filter(|name| !name.trim().is_empty())
        {
            return Some(name.clone());
        }
        if let Some(name) = self
            .session
            .dynamic_titles
            .get(session_id)
            .filter(|name| !name.trim().is_empty())
        {
            return Some(name.clone());
        }
        // Prefer local metadata — never take the transport session map lock for chrome.
        self.session_info(session_id).map(|session| session.name)
    }

    pub(in crate::features) fn session_endpoint(&self, session_id: &str) -> Option<String> {
        let metadata = self.session.metadata.get(session_id)?;
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
        let metadata = self.session.metadata.get(session_id)?;
        match &metadata.launch_config {
            SessionLaunchConfig::Ssh(config) if !config.host.trim().is_empty() => {
                Some(config.host.clone())
            }
            _ => None,
        }
    }

    pub(in crate::features) fn session_ssh_address(&self, session_id: &str) -> Option<String> {
        let metadata = self.session.metadata.get(session_id)?;
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
        if let Some(cwd) = self.session.cwds.get(session_id) {
            if !cwd.trim().is_empty() {
                lines.push(format!("cwd {cwd}"));
            }
        }
        lines
    }

    fn active_session_info_line(&self) -> Option<String> {
        let session_id = self.session.active_id.as_deref()?;
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
        let session_id = self.session.active_id.as_deref()?;
        let name = self.session_display_name(session_id)?;
        let session = self.session_info(session_id)?;
        let metadata = self.session.metadata.get(session_id)?;
        let endpoint = self
            .session_endpoint(session_id)
            .unwrap_or_else(|| "unknown endpoint".to_string());

        let mut details = vec![
            (self.tr("sessionInfo.name"), name),
            (
                self.tr("sessionInfo.kind"),
                session_kind_label(session.kind).to_string(),
            ),
            (self.tr("sessionInfo.sessionId"), session_id.to_string()),
            (
                self.tr("sessionInfo.size"),
                format!("{} x {}", session.cols, session.rows),
            ),
            (self.tr("sessionInfo.endpoint"), endpoint),
            (
                self.tr("sessionInfo.aiProfile"),
                format!("{:?}", metadata.ai_execution_profile),
            ),
        ];
        if let Some(cwd) = self.session.cwds.get(session_id) {
            details.push((self.tr("sessionInfo.cwd"), cwd.clone()));
        }

        match &metadata.launch_config {
            SessionLaunchConfig::Local(config) => {
                details.push((
                    self.tr("sessionInfo.launch"),
                    self.tr("sessionInfo.localShell").to_string(),
                ));
                if let Some(shell) = config
                    .shell_path
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    details.push((self.tr("sessionInfo.shell"), shell.to_string()));
                }
                if let Some(dir) = config.working_dir.as_ref() {
                    details.push((self.tr("sessionInfo.workingDir"), dir.display().to_string()));
                }
            }
            SessionLaunchConfig::Ssh(config) => {
                details.push((
                    self.tr("sessionInfo.launch"),
                    self.tr("sessionInfo.ssh").to_string(),
                ));
                details.push((self.tr("sessionInfo.host"), config.host.clone()));
                details.push((self.tr("sessionInfo.port"), config.port.to_string()));
                details.push((self.tr("sessionInfo.username"), config.username.clone()));
                if let Some(address) = self.session_ssh_address(session_id) {
                    details.push((self.tr("sessionInfo.sshAddress"), address));
                }
                if let Some(proxy_jump) = config.proxy_jump.as_ref() {
                    details.push((
                        self.tr("sessionInfo.proxyJump"),
                        format!(
                            "{}@{}:{}",
                            proxy_jump.username, proxy_jump.host, proxy_jump.port
                        ),
                    ));
                }
            }
            SessionLaunchConfig::Telnet(config) => {
                details.push((
                    self.tr("sessionInfo.launch"),
                    self.tr("sessionInfo.telnet").to_string(),
                ));
                details.push((self.tr("sessionInfo.host"), config.host.clone()));
                details.push((self.tr("sessionInfo.port"), config.port.to_string()));
            }
            SessionLaunchConfig::Serial(config) => {
                details.push((
                    self.tr("sessionInfo.launch"),
                    self.tr("sessionInfo.serial").to_string(),
                ));
                details.push((self.tr("sessionInfo.port"), config.port_name.clone()));
                details.push((self.tr("sessionInfo.baud"), config.baud_rate.to_string()));
                details.push((
                    self.tr("sessionInfo.frame"),
                    format!("{}{}{}", config.data_bits, config.parity, config.stop_bits),
                ));
            }
        }

        Some(details)
    }

    pub(in crate::features) fn activate_session_id(&mut self, session_id: &str) -> Option<String> {
        self.session.start.active_pending = None;
        self.session.start.active_failed = None;
        self.open_tabs_menu_open = false;
        self.new_session_menu_open = false;
        self.new_session_all_sessions_open = false;
        self.new_session_group_menu_path.clear();
        // Session switch resets terminal-output credential autofill (Tauri XTerminal remount).
        self.terminal.assist.reset_for_session_switch();
        let previous_session_id = self.session.active_id.clone();
        let switching_sessions = previous_session_id.as_deref() != Some(session_id);
        if previous_session_id.as_deref() != Some(session_id)
            && let Some(previous_session_id) = previous_session_id.as_deref()
        {
            self.cache_transfer_browser_session(previous_session_id);
        }

        self.session.active_id = Some(session_id.to_string());
        if switching_sessions {
            self.transfer.queue.selected_job_id = None;
            self.transfer.queue.job_menu = None;
            self.transfer.queue.job_delete = None;
            self.reset_remote_runtime_for_session_switch();
        }
        self.session_tab_scroll_into_view_pending = true;
        // Keep workspace_split mirrored to the active tab's per-tab pane root.
        self.sync_workspace_split_from_active_tab();
        self.transfer.browser.auto_sync_cwd_last_at = None;
        if let Some(metadata) = self.session.metadata.get(session_id).cloned() {
            self.session.active_ssh_config = metadata.ssh_config;
            self.session.active_ai_execution_profile = metadata.ai_execution_profile;
        } else {
            self.session.active_ssh_config = None;
            self.session.active_ai_execution_profile = AiExecutionProfile::SendOnly;
        }
        // Transfer browser state is only needed when the transfers panel is open
        // or we already have cached browser state for this session. Skipping the
        // full reset on every activate keeps connect/switch chrome responsive.
        let transfers_panel_visible = self.active_left_panel == Some(NavItem::Transfers)
            || self.active_right_panel == Some(NavItem::Transfers)
            || self.selected_nav == NavItem::Transfers;
        if transfers_panel_visible
            || self.transfer.browser.session_cache.contains_key(session_id)
            || !self.transfer.browser.entries.is_empty()
        {
            self.sync_transfer_browser_favorites_for_active_session();
            if !self.restore_transfer_browser_session_cache(session_id) {
                self.reset_transfer_browser_for_active_session();
            }
        } else {
            // Keep favorites map coherent for the active connection without wiping UI.
            self.sync_transfer_browser_favorites_for_active_session();
        }
        if let Some(view) = self.terminal.view.views.get_mut(session_id) {
            view.has_unread = false;
        } else {
            self.terminal.view.output.clear();
            self.terminal.view.output_decoder.reset_decoder();
            self.terminal.view.screen.clear();
        }
        if switching_sessions && self.terminal.input.focus_active {
            if let Some(previous_session_id) = previous_session_id.as_deref() {
                self.write_terminal_focus_report_to_session(previous_session_id, false);
            }
            self.write_terminal_focus_report_to_session(session_id, true);
        }
        self.sync_terminal_windows_active_tab(session_id);
        // Priority was refreshed via sync_workspace_split_from_active_tab.
        // Recover paint immediately if this tab was backgrounded without grids.
        if self
            .terminal
            .view
            .views
            .get(session_id)
            .is_some_and(|view| view.frame_snapshot.is_none())
        {
            self.request_terminal_live_snapshot(session_id);
        }
        previous_session_id
    }

    fn reset_remote_runtime_for_session_switch(&mut self) {
        self.remote_ops.process.items.clear();
        self.remote_ops.process.snapshot_loaded = false;
        self.remote_ops.process.pending = false;
        self.remote_ops.process.job_session_id = None;
        self.remote_ops.process.consecutive_refresh_failures = 0;
        self.remote_ops.process.last_refresh_at = None;
        self.remote_ops.process.selected_pid = None;
        self.remote_ops.process.menu_pid = None;
        self.remote_ops.process.signal_confirm = None;
        self.remote_ops.process.status = "ready".to_string();

        self.remote_ops.stats.data = None;
        self.remote_ops.stats.pending = false;
        self.remote_ops.stats.job_session_id = None;
        self.remote_ops.stats.consecutive_refresh_failures = 0;
        self.remote_ops.stats.last_refresh_at = None;
        self.remote_ops.stats.status = "start an SSH session to inspect remote stats".to_string();

        self.remote_ops.docker.overview = None;
        self.remote_ops.docker.pending = false;
        self.remote_ops.docker.job_session_id = None;
        self.remote_ops.docker.consecutive_refresh_failures = 0;
        self.remote_ops.docker.last_refresh_at = None;
        self.remote_ops.docker.details = None;
        self.remote_ops.docker.details_container_id = None;
        self.remote_ops.docker.details_last_refresh_at = None;
        self.remote_ops.docker.confirm = None;
        self.remote_ops.docker.container_menu_id = None;
        self.remote_ops.docker.compose_menu_id = None;
        self.remote_ops.docker.compose_services.clear();
        self.remote_ops.docker.compose_service_errors.clear();
        self.remote_ops.docker.status = "start an SSH session to inspect Docker".to_string();
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
        if self.terminal.selection.selection.take().is_some() {
            chrome_changed = true;
        }
        if self.terminal.selection.dragging {
            self.terminal.selection.dragging = false;
            chrome_changed = true;
        }
        if self.terminal.menus.context_menu.take().is_some() {
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
        let known = self.session.metadata.contains_key(&session_id);
        let disconnected = self.is_session_disconnected(&session_id);
        if !known && !disconnected {
            self.terminal.view.status = "session no longer exists".to_string();
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
        self.terminal.view.status = if disconnected {
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
            self.terminal.view.status = "no sessions to switch".to_string();
            cx.notify();
            return;
        }
        let active_index = self
            .session
            .active_id
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
        self.terminal.view.status = format!("active {}", short_id(&session_id));
        cx.notify();
    }

    pub(in crate::features) fn select_session_index(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let sessions = self.ordered_sessions();
        if sessions.is_empty() {
            self.terminal.view.status = "no sessions to switch".to_string();
            cx.notify();
            return;
        }
        let index = index.min(sessions.len().saturating_sub(1));
        let session_id = sessions[index].id.clone();
        self.activate_session_id_with_surface_sync(&session_id, cx);
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        self.terminal.view.status = format!("active {}", short_id(&session_id));
        cx.notify();
    }

    pub(in crate::features) fn toggle_open_tabs_menu(&mut self, cx: &mut Context<Self>) {
        self.open_tabs_menu_open = !self.open_tabs_menu_open;
        if self.open_tabs_menu_open {
            self.new_session_menu_open = false;
            self.new_session_all_sessions_open = false;
            self.new_session_group_menu_path.clear();
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
        self.new_session_all_sessions_open = false;
        self.new_session_group_menu_path.clear();
        cx.notify();
    }

    pub(in crate::features) fn close_new_session_menu(&mut self, cx: &mut Context<Self>) {
        let changed = self.new_session_menu_open
            || self.new_session_all_sessions_open
            || !self.new_session_group_menu_path.is_empty();
        self.new_session_menu_open = false;
        self.new_session_all_sessions_open = false;
        self.new_session_group_menu_path.clear();
        if changed {
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
        if self.session.active_id.as_deref() != Some(session_id.as_str()) {
            return;
        }
        self.tab_actions_session_id = Some(session_id);
        self.tab_actions_anchor = anchor;
        self.tab_actions_submenu = None;
        self.terminal.view.status = "tab actions opened".to_string();
        window.focus(&self.tab_actions_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_tab_actions(&mut self, cx: &mut Context<Self>) {
        self.tab_actions_session_id = None;
        self.tab_actions_anchor = None;
        self.tab_actions_submenu = None;
        self.terminal.view.status = "tab actions closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn copy_active_session_name(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.session.active_id.as_deref() else {
            self.terminal.view.status = "no active session name to copy".to_string();
            cx.notify();
            return;
        };
        let Some(name) = self.session_display_name(session_id) else {
            self.terminal.view.status = "active session name is unavailable".to_string();
            cx.notify();
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(name.clone()));
        self.terminal.view.status = format!("copied tab name '{name}'");
        cx.notify();
    }

    pub(in crate::features) fn copy_active_session_ssh_host(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.session.active_id.as_deref() else {
            self.terminal.view.status = "no active SSH host to copy".to_string();
            cx.notify();
            return;
        };
        let Some(host) = self.session_ssh_host(session_id) else {
            self.terminal.view.status = "active session is not SSH".to_string();
            cx.notify();
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(host.clone()));
        self.terminal.view.status = format!("copied SSH host '{host}'");
        cx.notify();
    }

    pub(in crate::features) fn open_active_session_info(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.session.active_id.is_none() {
            self.terminal.view.status = "no active session info".to_string();
            cx.notify();
            return;
        }
        self.session_info_open = true;
        self.terminal.view.status = self
            .active_session_info_line()
            .unwrap_or_else(|| "session info opened".to_string());
        window.focus(&self.session_info_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_active_session_info(&mut self, cx: &mut Context<Self>) {
        self.session_info_open = false;
        self.terminal.view.status = "session info closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn copy_active_session_info(&mut self, cx: &mut Context<Self>) {
        let Some(details) = self.active_session_info_details() else {
            self.terminal.view.status = "no active session info to copy".to_string();
            cx.notify();
            return;
        };
        let text = details
            .into_iter()
            .map(|(label, value)| format!("{label}: {value}"))
            .collect::<Vec<_>>()
            .join("\n");
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.terminal.view.status = "copied session info".to_string();
        cx.notify();
    }

    pub(in crate::features) fn close_tab_color_picker(&mut self, cx: &mut Context<Self>) {
        self.color_picker_open = false;
        self.terminal.view.status = "tab color picker closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn set_active_session_tab_color(
        &mut self,
        color: Option<u32>,
        cx: &mut Context<Self>,
    ) {
        let Some(session_id) = self.session.active_id.clone() else {
            self.terminal.view.status = "no active session color to set".to_string();
            cx.notify();
            return;
        };
        match color {
            Some(color) => {
                self.session.tab_colors.insert(session_id, color);
                self.terminal.view.status = "tab color updated".to_string();
            }
            None => {
                self.session.tab_colors.remove(&session_id);
                self.terminal.view.status = "tab color reset".to_string();
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
    use super::terminal_activation_surface_notify_ids;

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
