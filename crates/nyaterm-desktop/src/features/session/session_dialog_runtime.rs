use super::*;

impl NyaTermApp {
    pub(in crate::features) fn open_rename_session(
        &mut self,
        session_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(current_name) = self.session_display_name(&session_id) else {
            self.terminal_status = "session no longer exists".to_string();
            cx.notify();
            return;
        };
        self.rename_session_id = Some(session_id);
        self.rename_draft = current_name.chars().take(64).collect();
        self.terminal_status = "rename tab opened".to_string();
        window.focus(&self.rename_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_rename_session(&mut self, cx: &mut Context<Self>) {
        self.rename_session_id = None;
        self.rename_draft.clear();
        self.terminal_status = "rename tab cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn submit_rename_session(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.rename_session_id.take() else {
            self.terminal_status = "no tab rename is active".to_string();
            cx.notify();
            return;
        };
        let trimmed = self
            .rename_draft
            .trim()
            .chars()
            .take(64)
            .collect::<String>();
        self.rename_draft.clear();
        if trimmed.is_empty() {
            self.terminal_status = "tab name cannot be empty".to_string();
            self.rename_session_id = Some(session_id);
            cx.notify();
            return;
        }
        self.session_custom_names
            .insert(session_id.clone(), trimmed.clone());
        self.terminal_status = format!("renamed tab to {trimmed}");
        cx.notify();
    }

    pub(in crate::features) fn handle_rename_key_down(
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
            "escape" => self.close_rename_session(cx),
            "enter" => self.submit_rename_session(cx),
            "backspace" => {
                self.rename_draft.pop();
                cx.notify();
            }
            _ => {
                if self.rename_draft.chars().count() >= 64 {
                    return;
                }
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    let remaining = 64usize.saturating_sub(self.rename_draft.chars().count());
                    self.rename_draft.extend(input.chars().take(remaining));
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn open_startup_command_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_startup_command_dialog_for(StartupCommandAction::Duplicate, window, cx);
    }

    pub(in crate::features) fn open_startup_command_dialog_for(
        &mut self,
        action: StartupCommandAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_session_id.is_none() {
            self.terminal_status = match action {
                StartupCommandAction::Duplicate => {
                    "select a session before duplicating with a command"
                }
                StartupCommandAction::Multiplex => {
                    "select an SSH session before multiplexing with a command"
                }
            }
            .to_string();
            cx.notify();
            return;
        }
        if action == StartupCommandAction::Multiplex
            && self
                .active_session_id
                .as_deref()
                .and_then(|session_id| self.session_metadata.get(session_id))
                .is_none_or(|metadata| {
                    !matches!(metadata.launch_config, SessionLaunchConfig::Ssh(_))
                })
        {
            self.terminal_status = "active session is not SSH".to_string();
            cx.notify();
            return;
        }
        self.startup_command_open = true;
        self.startup_command_action = action;
        self.startup_command_draft.clear();
        self.startup_command_delay_ms = u64::from(
            self.settings
                .interaction_duplicate_session_command_delay_ms
                .min(60_000),
        );
        self.terminal_status = action.status_opened().to_string();
        window.focus(&self.startup_command_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_startup_command_dialog(&mut self, cx: &mut Context<Self>) {
        let action = self.startup_command_action;
        self.startup_command_open = false;
        self.startup_command_action = StartupCommandAction::Duplicate;
        self.startup_command_draft.clear();
        self.startup_command_delay_ms = DEFAULT_DUPLICATE_STARTUP_DELAY_MS;
        self.terminal_status = action.status_cancelled().to_string();
        cx.notify();
    }

    pub(in crate::features) fn adjust_startup_command_delay(
        &mut self,
        delta_ms: i64,
        cx: &mut Context<Self>,
    ) {
        let next = (self.startup_command_delay_ms as i64 + delta_ms).clamp(0, 60_000);
        self.startup_command_delay_ms = next as u64;
        cx.notify();
    }

    pub(in crate::features) fn submit_startup_command_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let command = self.startup_command_draft.trim().to_string();
        if command.is_empty() {
            self.terminal_status = "startup command cannot be empty".to_string();
            cx.notify();
            return;
        }
        let startup_command = StartupCommandRequest {
            command,
            delay_ms: self.startup_command_delay_ms.min(60_000),
        };
        let action = self.startup_command_action;
        self.startup_command_open = false;
        self.startup_command_action = StartupCommandAction::Duplicate;
        self.startup_command_draft.clear();
        match action {
            StartupCommandAction::Duplicate => {
                self.duplicate_active_session_with_startup(Some(startup_command), window, cx);
            }
            StartupCommandAction::Multiplex => {
                self.multiplex_active_ssh_session_with_startup(Some(startup_command), window, cx);
            }
        }
    }

    pub(in crate::features) fn handle_startup_command_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "escape" => self.close_startup_command_dialog(cx),
            "enter" => self.submit_startup_command_dialog(window, cx),
            "backspace" => {
                self.startup_command_draft.pop();
                cx.notify();
            }
            "up" => self.adjust_startup_command_delay(100, cx),
            "down" => self.adjust_startup_command_delay(-100, cx),
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.startup_command_draft.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn remove_session_state(&mut self, session_id: &str) {
        self.clear_terminal_mouse_report_for_session(session_id);
        self.session_order.retain(|id| id != session_id);
        self.session_tab_owner.remove(session_id);
        // If this leaf was a tab root, drop its pane tree (prune will rekey survivors).
        self.session_pane_roots.remove(session_id);
        let multiplex_key = self
            .session_metadata
            .remove(session_id)
            .and_then(|metadata| metadata.ssh_multiplex_key);
        self.session_custom_names.remove(session_id);
        self.session_dynamic_titles.remove(session_id);
        self.session_cwds.remove(session_id);
        self.clear_zmodem_session(session_id);
        self.clear_trzsz_session(session_id);
        self.session_tab_colors.remove(session_id);
        self.terminal_views.remove(session_id);
        self.terminal_frame_pipeline
            .remove_session(session_id.to_string());
        self.terminal_session_surface_bounds.remove(session_id);
        self.session_command_history.remove(session_id);
        self.transfer_browser_session_cache.remove(session_id);
        self.purge_session_from_sync_groups(session_id);
        self.reconcile_terminal_windows();
        if self.startup_restore_complete {
            self.persist_open_tabs();
        }
        if let Some(multiplex_key) = multiplex_key {
            let still_in_use = self
                .session_metadata
                .values()
                .any(|metadata| metadata.ssh_multiplex_key.as_deref() == Some(&multiplex_key));
            if !still_in_use {
                if let Some(handle) = self.ssh_multiplex_handles.remove(&multiplex_key) {
                    let _ = handle.disconnect();
                }
            }
        }
    }

    pub(in crate::features) fn next_session_after(&self, session_id: &str) -> Option<String> {
        let mut known_ids = self
            .session_manager
            .list_sessions()
            .unwrap_or_default()
            .into_iter()
            .map(|session| session.id)
            .collect::<HashSet<_>>();
        for (id, meta) in &self.session_metadata {
            if meta.disconnected {
                known_ids.insert(id.clone());
            }
        }
        self.session_order
            .iter()
            .find(|candidate| candidate.as_str() != session_id && known_ids.contains(*candidate))
            .cloned()
            .or_else(|| {
                known_ids
                    .into_iter()
                    .find(|candidate| candidate.as_str() != session_id)
            })
    }

    pub(in crate::features) fn active_session_name(&self) -> Option<String> {
        let session_id = self.active_session_id.as_deref()?;
        self.session_display_name(session_id)
    }
}
