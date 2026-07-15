use super::*;

impl NyaTermApp {
    pub(in crate::features) fn schedule_startup_command(
        &mut self,
        session_id: String,
        startup_command: StartupCommandRequest,
        cx: &mut Context<Self>,
    ) {
        let command = normalize_startup_command(&startup_command.command);
        if command.trim().is_empty() {
            return;
        }
        let delay_ms = startup_command.delay_ms.min(60_000);
        self.terminal_status = format!("scheduled startup command for {}", short_id(&session_id));
        cx.spawn(async move |this, cx| {
            if delay_ms > 0 {
                Timer::after(Duration::from_millis(delay_ms)).await;
            }
            let _ = this.update(cx, |this, cx| {
                if this.send_terminal_input_to_session(session_id, command.into_bytes(), cx) {
                    this.terminal_status = "startup command sent".to_string();
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(in crate::features) fn close_active_session(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.active_session_id.clone() else {
            self.terminal_status = "no active session".to_string();
            cx.notify();
            return;
        };
        self.close_session(session_id, cx);
    }

    pub(in crate::features) fn close_session(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let was_active = self.active_session_id.as_deref() == Some(session_id.as_str());
        // Tauri: closing a strip tab closes the whole tab tree; closing a secondary leaf
        // only removes that pane. Strip close uses the tab-root id.
        let close_ids = if !self.is_secondary_pane_session(&session_id) {
            if let Some(root) = self.session_pane_roots.get(&session_id) {
                root.session_ids()
            } else {
                vec![session_id.clone()]
            }
        } else {
            vec![session_id.clone()]
        };
        for close_id in &close_ids {
            let disconnected = self.is_session_disconnected(close_id);
            match self.session_manager.close(close_id) {
                Ok(()) => {}
                Err(_) if disconnected => {}
                Err(error) if !disconnected && !self.session_metadata.contains_key(close_id) => {
                    self.terminal_status = format!("close failed: {error}");
                    cx.notify();
                    return;
                }
                Err(_) => {}
            }
            self.recording_manager.cleanup_session(close_id);
            self.remove_session_state(close_id);
        }
        self.prune_workspace_split();
        if was_active {
            self.ai_agent_loop = None;
            self.ai_agent_capture = AgentOutputCaptureProcessor::new();
            if let Some(next_session_id) = self.next_session_after(&session_id) {
                self.activate_session_id(&next_session_id);
                self.terminal_status =
                    format!("session closed; active {}", short_id(&next_session_id));
            } else {
                self.active_session_id = None;
                self.active_ssh_config = None;
                self.active_ai_execution_profile = AiExecutionProfile::SendOnly;
                self.terminal_output = String::from(INITIAL_TERMINAL_BANNER);
                self.terminal_output_decoder.reset_decoder();
                self.terminal_screen = initial_terminal_screen();
                self.terminal_screen
                    .set_encoding(&self.settings.interaction_default_encoding);
                self.terminal_status = "session closed".to_string();
            }
        } else {
            self.terminal_status = format!("closed {}", short_id(&session_id));
        }
        cx.notify();
    }

    pub(in crate::features) fn close_session_batch(
        &mut self,
        session_ids: Vec<String>,
        label: &'static str,
    ) {
        if session_ids.is_empty() {
            self.terminal_status = format!("no {label} sessions to close");
            return;
        }

        let active_before = self.active_session_id.clone();
        let mut closed = 0usize;
        let mut failed = 0usize;
        for session_id in session_ids {
            match self.session_manager.close(&session_id) {
                Ok(()) => {
                    self.recording_manager.cleanup_session(&session_id);
                    self.remove_session_state(&session_id);
                    closed += 1;
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }
        self.prune_workspace_split();

        let live_ids = self
            .session_manager
            .list_sessions()
            .unwrap_or_default()
            .into_iter()
            .map(|session| session.id)
            .collect::<HashSet<_>>();
        let active_is_live = active_before
            .as_deref()
            .is_some_and(|session_id| live_ids.contains(session_id));

        if !active_is_live {
            self.ai_agent_loop = None;
            self.ai_agent_capture = AgentOutputCaptureProcessor::new();
            if let Some(next_session_id) = self
                .session_order
                .iter()
                .find(|session_id| live_ids.contains(*session_id))
                .cloned()
                .or_else(|| live_ids.iter().next().cloned())
            {
                self.activate_session_id(&next_session_id);
            } else {
                self.active_session_id = None;
                self.active_ssh_config = None;
                self.active_ai_execution_profile = AiExecutionProfile::SendOnly;
                self.terminal_output = String::from(INITIAL_TERMINAL_BANNER);
                self.terminal_output_decoder.reset_decoder();
                self.terminal_screen = initial_terminal_screen();
                self.terminal_screen
                    .set_encoding(&self.settings.interaction_default_encoding);
            }
        }

        self.terminal_status = if failed == 0 {
            format!("closed {closed} {label} session(s)")
        } else {
            format!("closed {closed} {label} session(s), {failed} failed")
        };
    }

    pub(in crate::features) fn handle_window_minimize(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Tauri minimize_to_tray: hide window instead of taskbar minimize when enabled.
        // GPUI lacks a portable tray today; minimize still uses the platform minimize path,
        // and the flag is honored as a documented no-op tray intent with status feedback.
        if self.settings.minimize_to_tray {
            window.minimize_window();
            self.terminal_status =
                "minimized (tray mode preferred; OS tray polish pending)".to_string();
            cx.notify();
            return;
        }
        window.minimize_window();
    }

    pub(in crate::features) fn handle_window_close_request(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let open_sessions = self.ordered_sessions().len();
        if self.settings.confirm_on_close && open_sessions > 0 {
            // Reuse close-all confirmation as quit-with-sessions gate (Tauri confirm_on_close).
            self.pending_quit_after_close_all = true;
            self.open_close_all_sessions_confirm(window, cx);
            self.terminal_status = format!("confirm close: {open_sessions} session(s) still open");
            cx.notify();
            return;
        }
        // Persist workspace before exit when startup restore is enabled.
        if self.settings.startup_restore {
            self.persist_open_tabs();
        }
        window.remove_window();
    }

    pub(in crate::features) fn open_close_all_sessions_confirm(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.ordered_sessions().is_empty() {
            self.terminal_status = "no sessions to close".to_string();
            cx.notify();
            return;
        }
        self.tab_actions_session_id = None;
        self.tab_actions_anchor = None;
        self.close_all_sessions_confirm_open = true;
        self.terminal_status = "close all sessions confirmation opened".to_string();
        window.focus(&self.close_all_sessions_confirm_focus);
        cx.notify();
    }

    pub(in crate::features) fn cancel_close_all_sessions_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.close_all_sessions_confirm_open = false;
        self.pending_quit_after_close_all = false;
        self.pending_window_quit = false;
        self.terminal_status = "close all sessions cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn confirm_close_all_sessions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_all_sessions_confirm_open = false;
        let quit_after = self.pending_quit_after_close_all;
        self.pending_quit_after_close_all = false;
        self.pending_window_quit = false;
        self.close_all_sessions(cx);
        if quit_after {
            if self.settings.startup_restore {
                self.persist_open_tabs();
            }
            self.terminal_status = "sessions closed; closing window".to_string();
            window.remove_window();
            return;
        }
        cx.notify();
    }

    pub(in crate::features) fn close_all_sessions(&mut self, cx: &mut Context<Self>) {
        let ids = self
            .session_manager
            .list_sessions()
            .unwrap_or_default()
            .into_iter()
            .map(|session| session.id)
            .collect::<Vec<_>>();
        self.close_session_batch(ids, "active");
        cx.notify();
    }

    pub(in crate::features) fn close_inactive_sessions(
        &mut self,
        keep_session_id: String,
        cx: &mut Context<Self>,
    ) {
        let ids = self
            .ordered_sessions()
            .into_iter()
            .filter_map(|session| (session.id != keep_session_id).then_some(session.id))
            .collect::<Vec<_>>();
        self.activate_session_id(&keep_session_id);
        self.close_session_batch(ids, "inactive");
        cx.notify();
    }

    pub(in crate::features) fn close_sessions_to_right(
        &mut self,
        anchor_session_id: String,
        cx: &mut Context<Self>,
    ) {
        let sessions = self.ordered_sessions();
        let Some(anchor_index) = sessions
            .iter()
            .position(|session| session.id == anchor_session_id)
        else {
            self.terminal_status = "session no longer exists".to_string();
            cx.notify();
            return;
        };
        let ids = sessions
            .into_iter()
            .skip(anchor_index + 1)
            .map(|session| session.id)
            .collect::<Vec<_>>();
        self.close_session_batch(ids, "right-side");
        cx.notify();
    }

    pub(in crate::features) fn clear_terminal(&mut self, cx: &mut Context<Self>) {
        if let Some(session_id) = self.active_session_id.as_deref()
            && let Some(view) = self.terminal_views.get_mut(session_id)
        {
            view.clear();
        }
        self.terminal_output.clear();
        self.terminal_output_decoder.reset_decoder();
        self.terminal_screen.clear();
        self.terminal_status = "terminal cleared".to_string();
        cx.notify();
    }

    pub(in crate::features) fn append_terminal_log(&mut self, text: impl AsRef<str>) {
        let session_id = self.active_session_id.clone();
        self.append_terminal_log_for_session(session_id.as_deref(), text.as_ref(), false);
    }
}
