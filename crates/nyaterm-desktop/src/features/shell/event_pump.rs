use super::*;

#[path = "event_pump/bridge.rs"]
mod bridge;
#[path = "event_pump/helpers.rs"]
mod helpers;
#[path = "event_pump/planes.rs"]
mod planes;
#[path = "event_pump/publish.rs"]
mod publish;
#[path = "event_pump/session_events.rs"]
mod session_events;

use helpers::*;

impl NyaTermApp {
    pub(in crate::features) fn refresh_window_render_inputs(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let before_viewport = self.last_viewport_size;
        let before_metrics = self.terminal_cell_metrics;
        let vs = window.viewport_size();
        self.last_viewport_size = (f32::from(vs.width), f32::from(vs.height));
        if terminal_cell_metrics_refresh_needed(self.terminal_cell_metrics) {
            self.refresh_terminal_cell_metrics(cx);
        }
        if self.terminal_cell_metrics != before_metrics {
            self.sync_terminal_cell_metrics_to_screens();
            self.resize_all_known_terminal_surfaces();
        }
        self.last_viewport_size != before_viewport || self.terminal_cell_metrics != before_metrics
    }

    pub(in crate::features) fn mark_user_activity(&mut self) {
        if !self.is_locked {
            self.last_user_activity_at = Instant::now();
        }
    }

    pub(in crate::features) fn should_log_slow_diagnostic(
        &mut self,
        key: &'static str,
        now: Instant,
    ) -> bool {
        if diagnostic_log_due(
            self.diagnostic_log_last_at.get(key).copied(),
            now,
            SLOW_DIAGNOSTIC_THROTTLE,
        ) {
            self.diagnostic_log_last_at.insert(key, now);
            true
        } else {
            false
        }
    }

    pub(in crate::features) fn drive_idle_lock(&mut self) -> bool {
        if self.is_locked
            || !self.settings.enable_screen_lock
            || self.settings.idle_lock_minutes == 0
        {
            return false;
        }
        let idle_for = self.last_user_activity_at.elapsed();
        let lock_after = Duration::from_secs(u64::from(self.settings.idle_lock_minutes) * 60);
        if idle_for < lock_after {
            return false;
        }
        self.is_locked = true;
        self.lock_password_draft.clear();
        self.lock_status = if self.settings.has_master_password {
            "Enter the master password to unlock.".to_string()
        } else {
            "No master password is configured.".to_string()
        };
        self.terminal_status = format!(
            "screen locked after {} minute(s) idle",
            self.settings.idle_lock_minutes
        );
        true
    }

    pub(crate) fn mark_window_runtime_started(&mut self) {
        self.terminal_runtime.event_pump_started = true;
    }

    pub(crate) fn window_runtime_tick_delay(&self) -> Duration {
        runtime_tick_interval_for_pressure(self.runtime_output_pressure_active())
    }

    pub(in crate::features) fn runtime_output_pressure_active(&self) -> bool {
        runtime_output_pressure_active_from_counts(
            self.terminal_runtime.session_event_backlog_active,
            self.terminal_runtime.session_event_queued_output_bytes,
            self.pending_session_events.len(),
            self.session_event_bridge.queued_event_count()
                + self.session_event_bridge.source_queued_event_count(),
            self.session_event_bridge.queued_output_bytes()
                + self.session_event_bridge.source_queued_output_bytes(),
            self.pending_terminal_frame_events.len(),
            self.terminal_frame_pipeline.queued_event_count(),
            self.terminal_frame_pipeline.queued_output_bytes(),
        )
    }

    pub(super) fn drive_pending_session_status(&mut self) -> bool {
        let Some((name, requested_at)) = self.pending_session_status_source() else {
            self.terminal_runtime.last_pending_session_status_at = None;
            return false;
        };
        let auth_wait = self.pending_session_auth_wait();
        if auth_wait.is_none() && requested_at.elapsed() < PENDING_SESSION_STILL_CONNECTING_AFTER {
            return false;
        }
        let now = Instant::now();
        if self
            .terminal_runtime
            .last_pending_session_status_at
            .is_some_and(|last_at| {
                now.saturating_duration_since(last_at) < PENDING_SESSION_STATUS_INTERVAL
            })
        {
            return false;
        }
        self.terminal_runtime.last_pending_session_status_at = Some(now);
        let message = pending_session_status_message(&name, auth_wait.as_ref());
        if self.terminal_status == message {
            return false;
        }
        self.terminal_status = message;
        true
    }

    pub(super) fn drive_remote_auto_refresh(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.active_ssh_config.is_none() {
            return false;
        }

        let mut dirty = false;
        let left_panel = self.current_left_panel();
        let right_panel = self.current_right_panel();

        if right_panel == Some(NavItem::Stats)
            && self.settings.ui_show_remote_stats
            && !self.stats_pending
            && remote_refresh_due(
                self.stats_last_refresh_at,
                self.settings.ui_remote_stats_interval.max(1),
            )
        {
            self.refresh_stats(window, cx);
            dirty = true;
        } else if right_panel == Some(NavItem::Processes)
            && self.settings.ui_show_process_manager
            && !self.process_pending
            && remote_refresh_due(
                self.process_last_refresh_at,
                self.settings.ui_process_manager_interval.max(3),
            )
        {
            self.refresh_processes(window, cx);
            dirty = true;
        } else if right_panel == Some(NavItem::Docker)
            && self.settings.ui_show_docker_manager
            && !self.docker_pending
        {
            let interval = self.settings.ui_docker_manager_interval.max(3);
            if remote_refresh_due(self.docker_last_refresh_at, interval) {
                self.refresh_docker(window, cx);
                dirty = true;
            } else if self.docker_details.is_some()
                && remote_refresh_due(self.docker_details_last_refresh_at, interval)
                && let Some(container_id) = self.docker_details_container_id.clone()
            {
                self.load_docker_details(container_id, window, cx);
                dirty = true;
            }
        }

        if left_panel == Some(NavItem::Transfers)
            && self.transfer_browser_auto_sync_cwd_enabled()
            && !self.transfer_sync_cwd_job_running()
            && remote_refresh_due(
                self.transfer_auto_sync_cwd_last_at,
                TRANSFER_AUTO_SYNC_CWD_INTERVAL_SECONDS,
            )
        {
            self.transfer_auto_sync_cwd_last_at = Some(Instant::now());
            self.start_transfer_sync_cwd_job(window, cx);
            dirty = true;
        }
        dirty
    }

    pub(in crate::features) fn pending_session_status_label(&self) -> Option<String> {
        let name = self.pending_session_display_name()?;
        Some(pending_session_status_message(
            &name,
            self.pending_session_auth_wait().as_ref(),
        ))
    }

    pub(in crate::features) fn pending_session_tab_detail(&self) -> Option<&'static str> {
        if !self.has_pending_session_start() {
            return None;
        }
        Some(match self.pending_session_auth_wait() {
            Some(PendingSessionAuthWait::Credential { .. }) => "Credential required",
            Some(PendingSessionAuthWait::HostKey { .. }) => "Host key required",
            None => "Connecting...",
        })
    }

    fn pending_session_auth_wait(&self) -> Option<PendingSessionAuthWait> {
        if let Some(prompt) = self.active_credential_prompt.as_ref() {
            return Some(PendingSessionAuthWait::Credential {
                target: credential_prompt_target(&prompt.prompt),
            });
        }
        if let Some(prompt) = self.active_host_key_prompt.as_ref() {
            return Some(PendingSessionAuthWait::HostKey {
                host: prompt.host_key.host_identifier.clone(),
            });
        }
        None
    }
}
