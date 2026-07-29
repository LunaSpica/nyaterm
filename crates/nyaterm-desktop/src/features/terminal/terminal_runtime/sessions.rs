use std::collections::HashSet;
use std::time::Duration;

use gpui::{Context, Timer, Window};
use nyaterm_core::AgentOutputCaptureProcessor;

use crate::features::formatting::{normalize_startup_command, short_id};
use crate::features::{INITIAL_TERMINAL_BANNER, NyaTermApp};
use crate::models::StartupCommandRequest;
use crate::terminal::initial_terminal_screen;

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
        self.terminal.view.status =
            format!("scheduled startup command for {}", short_id(&session_id));
        cx.spawn(async move |this, cx| {
            if delay_ms > 0 {
                Timer::after(Duration::from_millis(delay_ms)).await;
            }
            let _ = this.update(cx, |this, cx| {
                if this.send_terminal_input_to_session(session_id, command.into_bytes(), cx) {
                    this.terminal.view.status = "startup command sent".to_string();
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(in crate::features) fn close_active_session(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.session.active_id_owned() else {
            self.terminal.view.status = "no active session".to_string();
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
        let was_active = self.session.active_id() == Some(session_id.as_str());
        // Tauri: closing a strip tab closes the whole tab tree; closing a secondary leaf
        // only removes that pane. Strip close uses the tab-root id.
        let close_ids = if !self.is_secondary_pane_session(&session_id) {
            if let Some(root) = self.shell.workspace.pane_roots.get(&session_id) {
                root.session_ids()
            } else {
                vec![session_id.clone()]
            }
        } else {
            vec![session_id.clone()]
        };
        for close_id in &close_ids {
            let disconnected = self.is_session_disconnected(close_id);
            match self.session.manager().close(close_id) {
                Ok(()) => {}
                Err(_) if disconnected => {}
                Err(error) if !disconnected && !self.session.has_session(close_id) => {
                    self.terminal.view.status = format!("close failed: {error}");
                    cx.notify();
                    return;
                }
                Err(_) => {}
            }
            self.cleanup_recording_for_session(close_id);
            self.remove_session_state(close_id);
        }
        self.prune_workspace_split();
        if was_active {
            self.ai.agent.loop_state = None;
            self.ai.agent.capture = AgentOutputCaptureProcessor::new();
            self.sync_session_event_bridge_policy();
            if let Some(next_session_id) = self.next_session_after(&session_id) {
                self.activate_session_id(&next_session_id);
                self.terminal.view.status =
                    format!("session closed; active {}", short_id(&next_session_id));
            } else {
                self.session.clear_active_session();
                self.terminal.view.output = String::from(INITIAL_TERMINAL_BANNER);
                self.terminal.view.output_decoder.reset_decoder();
                self.terminal.view.screen = initial_terminal_screen();
                self.terminal
                    .view
                    .screen
                    .set_encoding(&self.settings.summary.interaction_default_encoding);
                self.terminal.view.status = "session closed".to_string();
            }
        } else {
            self.terminal.view.status = format!("closed {}", short_id(&session_id));
        }
        cx.notify();
    }

    pub(in crate::features) fn close_session_batch(
        &mut self,
        session_ids: Vec<String>,
        label: &'static str,
    ) {
        if session_ids.is_empty() {
            self.terminal.view.status = format!("no {label} sessions to close");
            return;
        }

        let active_before = self.session.active_id_owned();
        let mut closed = 0usize;
        let mut failed = 0usize;
        for session_id in session_ids {
            match self.session.manager().close(&session_id) {
                Ok(()) => {
                    self.cleanup_recording_for_session(&session_id);
                    self.remove_session_state(&session_id);
                    closed += 1;
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }
        self.prune_workspace_split();

        // After close_session_batch, local metadata is the source of truth for
        // remaining tabs (includes disconnected). Avoid transport map lock.
        let live_ids = self
            .session
            .session_ids()
            .map(ToOwned::to_owned)
            .collect::<HashSet<_>>();
        let active_is_live = active_before
            .as_deref()
            .is_some_and(|session_id| live_ids.contains(session_id));

        if !active_is_live {
            self.ai.agent.loop_state = None;
            self.ai.agent.capture = AgentOutputCaptureProcessor::new();
            self.sync_session_event_bridge_policy();
            if let Some(next_session_id) = self
                .session
                .session_order()
                .iter()
                .find(|session_id| live_ids.contains(*session_id))
                .cloned()
                .or_else(|| live_ids.iter().next().cloned())
            {
                self.activate_session_id(&next_session_id);
            } else {
                self.session.clear_active_session();
                self.terminal.view.output = String::from(INITIAL_TERMINAL_BANNER);
                self.terminal.view.output_decoder.reset_decoder();
                self.terminal.view.screen = initial_terminal_screen();
                self.terminal
                    .view
                    .screen
                    .set_encoding(&self.settings.summary.interaction_default_encoding);
            }
        }

        self.terminal.view.status = if failed == 0 {
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
        if self.settings.summary.minimize_to_tray {
            window.minimize_window();
            self.terminal.view.status =
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
        if self.settings.summary.confirm_on_close && open_sessions > 0 {
            // Reuse close-all confirmation as quit-with-sessions gate (Tauri confirm_on_close).
            self.session.dialogs.request_quit_after_close_all();
            self.open_close_all_sessions_confirm(window, cx);
            self.terminal.view.status =
                format!("confirm close: {open_sessions} session(s) still open");
            cx.notify();
            return;
        }
        // Persist workspace before exit when startup restore is enabled.
        if self.settings.summary.startup_restore {
            self.flush_open_tabs_now();
        }
        window.remove_window();
    }

    pub(in crate::features) fn open_close_all_sessions_confirm(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.ordered_sessions().is_empty() {
            self.terminal.view.status = "no sessions to close".to_string();
            cx.notify();
            return;
        }
        self.session.dialogs.open_close_all_sessions_confirm();
        self.terminal.view.status = "close all sessions confirmation opened".to_string();
        window.focus(self.session.dialogs.close_all_sessions_confirm_focus());
        cx.notify();
    }

    pub(in crate::features) fn cancel_close_all_sessions_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.session.dialogs.cancel_close_all_sessions_confirm();
        self.terminal.view.status = "close all sessions cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn confirm_close_all_sessions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let quit_after = self.session.dialogs.take_close_all_sessions_confirm();
        self.close_all_sessions(cx);
        if quit_after {
            if self.settings.summary.startup_restore {
                self.flush_open_tabs_now();
            }
            self.terminal.view.status = "sessions closed; closing window".to_string();
            window.remove_window();
            return;
        }
        cx.notify();
    }

    pub(in crate::features) fn close_all_sessions(&mut self, cx: &mut Context<Self>) {
        let ids = self
            .session
            .session_ids()
            .map(ToOwned::to_owned)
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
        self.activate_session_id_with_surface_sync(&keep_session_id, cx);
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
            self.terminal.view.status = "session no longer exists".to_string();
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
        self.clear_terminal_selection(cx);
        if let Some(session_id) = self.session.active_id()
            && let Some(view) = self.terminal.view.views.get_mut(session_id)
        {
            view.clear();
        }
        self.terminal.view.output.clear();
        self.terminal.view.output_decoder.reset_decoder();
        self.terminal.view.screen.clear();
        self.terminal.view.status = "terminal cleared".to_string();
        cx.notify();
    }

    pub(in crate::features) fn append_terminal_log(&mut self, text: impl AsRef<str>) {
        let session_id = self.session.active_id_owned();
        self.append_terminal_log_for_session(session_id.as_deref(), text.as_ref(), false);
    }
}
