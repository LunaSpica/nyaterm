use super::*;
use futures::StreamExt;

use crate::models::{TerminalSearchMode, terminal_frame_search_result_is_current};

mod bridge;
mod helpers;
mod planes;
mod publish;
mod session_events;

use crate::features::terminal::terminal_runtime::TERMINAL_INPUT_LATENCY_WINDOW;
use helpers::*;

// These intervals produce wake deadlines at 4ms, 12ms, and 24ms. `Timer::after`
// calls below are sequential, so storing the absolute deadlines here would
// accidentally move the final echo poll out to 40ms.
const TERMINAL_INPUT_WAKE_INTERVALS: [Duration; 3] = [
    Duration::from_millis(4),
    Duration::from_millis(8),
    Duration::from_millis(12),
];

impl NyaTermApp {
    pub(in crate::features) fn start_terminal_frame_event_wake(&mut self, cx: &mut Context<Self>) {
        let Some(mut wake_rx) = self.terminal_frame_pipeline.take_event_wake_receiver() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            while wake_rx.next().await.is_some() {
                if this
                    .update(cx, |this, cx| {
                        this.drain_terminal_frame_event_wake(cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    pub(in crate::features) fn refresh_window_render_inputs(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let before_viewport = self.last_viewport_size;
        let before_metrics = self.terminal_cell_metrics;
        let vs = window.viewport_size();
        self.last_viewport_size = (f32::from(vs.width), f32::from(vs.height));
        if self.last_viewport_size != before_viewport {
            // Geometry churn (resize / some window managers during move).
            self.last_viewport_change_at = Some(Instant::now());
            self.notify_terminal_surfaces_for_viewport_change(cx);
        }
        if terminal_cell_metrics_refresh_needed(self.terminal_cell_metrics) {
            self.refresh_terminal_cell_metrics(cx);
        }
        if self.terminal_cell_metrics != before_metrics {
            self.sync_terminal_cell_metrics_to_screens();
            self.resize_all_known_terminal_surfaces();
            self.refresh_visible_terminal_surfaces(cx);
        }
        self.last_viewport_size != before_viewport || self.terminal_cell_metrics != before_metrics
    }

    fn notify_terminal_surfaces_for_viewport_change(&mut self, cx: &mut Context<Self>) {
        let session_ids =
            viewport_change_terminal_session_ids(&self.visible_terminal_session_ids());
        for session_id in session_ids {
            self.notify_terminal_surface_only(Some(session_id.as_str()), cx);
        }
    }

    pub(in crate::features) fn mark_user_activity(&mut self) {
        if !self.is_locked {
            self.last_user_activity_at = Instant::now();
        }
    }

    pub(in crate::features) fn arm_terminal_input_wake(&mut self, cx: &mut Context<Self>) {
        self.mark_terminal_input_latency_activity(cx);
        self.terminal_runtime.terminal_input_wake_generation = self
            .terminal_runtime
            .terminal_input_wake_generation
            .saturating_add(1);
        if self.terminal_runtime.terminal_input_wake_armed {
            return;
        }
        self.terminal_runtime.terminal_input_wake_armed = true;
        // Keep key dispatch limited to encoding and the PTY notifier. Pull
        // through already queued echo after the current input event. The armed
        // state coalesces a burst of keys into one deferred drain.
        let app = cx.entity();
        cx.defer(move |cx| {
            let _ = app.update(cx, |this, cx| {
                this.drain_terminal_input_wake(cx);
            });
        });
        let mut observed_generation = self.terminal_runtime.terminal_input_wake_generation;
        cx.spawn(async move |this, cx| {
            loop {
                for delay in TERMINAL_INPUT_WAKE_INTERVALS {
                    Timer::after(delay).await;
                    let _ = this.update(cx, |this, cx| {
                        this.drain_terminal_input_wake(cx);
                    });
                }
                let (next_generation, finished) = this
                    .update(cx, |this, _| {
                        let next_generation = this.terminal_runtime.terminal_input_wake_generation;
                        if next_generation == observed_generation {
                            this.terminal_runtime.terminal_input_wake_armed = false;
                            (next_generation, true)
                        } else {
                            (next_generation, false)
                        }
                    })
                    .unwrap_or((observed_generation, true));
                if finished {
                    break;
                }
                observed_generation = next_generation;
            }
        })
        .detach();
    }

    fn drain_terminal_input_wake(&mut self, cx: &mut Context<Self>) {
        let chrome_dirty = self.drain_session_events_for_input_wake(cx)
            | self.drain_terminal_frame_events_for_input_wake(cx);
        if chrome_dirty {
            cx.notify();
            self.terminal_runtime.last_ui_notify_at = Some(Instant::now());
            self.terminal_runtime.pending_ui_notify = false;
        }
    }

    fn drain_terminal_frame_event_wake(&mut self, cx: &mut Context<Self>) {
        let chrome_dirty = self.drain_terminal_frame_events(cx);
        if chrome_dirty {
            cx.notify();
            self.terminal_runtime.last_ui_notify_at = Some(Instant::now());
            self.terminal_runtime.pending_ui_notify = false;
        }
    }

    fn mark_terminal_input_latency_activity(&mut self, cx: &mut Context<Self>) {
        self.terminal_runtime.last_terminal_input_at = Some(Instant::now());
        if let Some(session_id) = self
            .active_session_id
            .as_deref()
            .filter(|session_id| !session_id.is_empty())
        {
            self.terminal_runtime
                .pending_terminal_input_idle_sessions
                .insert(session_id.to_string());
        }
        if self.terminal_runtime.terminal_input_idle_notify_armed {
            return;
        }
        self.terminal_runtime.terminal_input_idle_notify_armed = true;
        cx.spawn(async move |this, cx| {
            Timer::after(TERMINAL_INPUT_LATENCY_WINDOW).await;
            let _ = this.update(cx, |this, cx| {
                this.flush_terminal_input_idle_notify(cx);
            });
        })
        .detach();
    }

    fn flush_terminal_input_idle_notify(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        if let Some(delay) = terminal_input_idle_remaining_delay(
            self.terminal_runtime.last_terminal_input_at,
            now,
            TERMINAL_INPUT_LATENCY_WINDOW,
        ) {
            cx.spawn(async move |this, cx| {
                Timer::after(delay).await;
                let _ = this.update(cx, |this, cx| {
                    this.flush_terminal_input_idle_notify(cx);
                });
            })
            .detach();
            return;
        }
        self.terminal_runtime.terminal_input_idle_notify_armed = false;
        let session_ids = self
            .terminal_runtime
            .pending_terminal_input_idle_sessions
            .drain()
            .collect::<Vec<_>>();
        for session_id in session_ids {
            self.notify_terminal_surface_only(Some(session_id.as_str()), cx);
        }
    }

    pub(in crate::features) fn mark_title_drag_activity(&mut self) {
        self.title_drag_active_until = Some(Instant::now() + TITLE_DRAG_ACTIVE_HOLD);
    }

    pub(in crate::features) fn title_drag_active(&self, now: Instant) -> bool {
        title_drag_active(self.title_drag_active_until, now)
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

    pub(in crate::features) fn visible_terminal_layout_cache_stats(&self) -> (u64, u64) {
        self.visible_terminal_session_ids()
            .into_iter()
            .filter_map(|session_id| self.terminal_views.get(session_id))
            .filter_map(|view| view.render_cache.layout_cache.lock().ok())
            .fold((0u64, 0u64), |(hits, misses), cache| {
                (
                    hits.saturating_add(cache.hits),
                    misses.saturating_add(cache.misses),
                )
            })
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

    pub(crate) fn window_runtime_running(&self) -> bool {
        self.terminal_runtime.event_pump_started
    }

    pub(crate) fn window_runtime_tick_delay(&self) -> Duration {
        // During recent viewport geometry churn (window resize/drag), prefer the
        // idle cadence so full plane ticks do not stack on compositor paints.
        let now = Instant::now();
        if self.title_drag_active(now)
            || window_geometry_churn_active(self.last_viewport_change_at, now)
        {
            return RUNTIME_IDLE_TICK_INTERVAL;
        }
        if self.runtime_quiet_tick_allowed() {
            return RUNTIME_QUIET_TICK_INTERVAL;
        }
        runtime_tick_interval_for_pressure(self.runtime_output_pressure_active())
    }

    pub(crate) fn window_runtime_tick_needs_update(
        &self,
        viewport_size: (f32, f32),
        now: Instant,
    ) -> bool {
        if !self.terminal_runtime.event_pump_started {
            return false;
        }
        if self.last_viewport_size != viewport_size
            || terminal_cell_metrics_refresh_needed(self.terminal_cell_metrics)
        {
            return true;
        }
        if self
            .terminal_runtime
            .connect_settle_until
            .is_some_and(|until| now >= until)
        {
            return true;
        }
        if self.title_drag_active(now) {
            return true;
        }

        let output_pressure = self.runtime_output_pressure_active();
        let connect_settle = connect_settle_active(self.terminal_runtime.connect_settle_until, now);
        if runtime_ui_notify_allowed(
            false,
            self.terminal_runtime.pending_ui_notify,
            false,
            output_pressure || connect_settle,
            self.terminal_runtime.last_ui_notify_at,
            now,
        ) {
            return true;
        }
        if !self.runtime_quiet_tick_allowed() {
            return true;
        }
        self.window_runtime_quiet_tick_has_due_work(now)
    }

    fn window_runtime_quiet_tick_has_due_work(&self, now: Instant) -> bool {
        if self.ai.chat.focus_pending
            || self.transfer.file_ops.rename_focus_pending
            || self.credential_prompt_focus_pending
        {
            return true;
        }
        if self.terminal_file_drop_hover.is_some() {
            return true;
        }
        if self.terminal_runtime.visual_bell_ticks > 0 {
            return true;
        }
        if self.settings.cursor_blink
            && !self.visible_terminal_session_ids().is_empty()
            && self
                .terminal_runtime
                .cursor_blink_next_at
                .is_some_and(|next| now >= next)
        {
            return true;
        }
        if self.visible_terminal_performance_recovery_due() {
            return true;
        }
        self.terminal_render_requests_pending()
    }

    fn visible_terminal_performance_recovery_due(&self) -> bool {
        self.visible_terminal_session_ids()
            .into_iter()
            .any(|session_id| {
                self.terminal_views.get(session_id).is_some_and(|view| {
                    view.render_degraded
                        || view.performance_overlay.is_some()
                        || view.output_burst_bytes > 0
                })
            })
    }

    fn terminal_render_requests_pending(&self) -> bool {
        let visible_session_ids = self.visible_terminal_session_ids();
        if visible_session_ids.iter().any(|session_id| {
            self.terminal_views
                .get(*session_id)
                .is_some_and(|view| view.frame_snapshot.is_none() && view.scroll_offset == 0)
        }) {
            return true;
        }
        if visible_session_ids
            .iter()
            .any(|session_id| self.terminal_visual_scroll_active_for_session(Some(session_id)))
        {
            return true;
        }
        if !self.terminal_search_open || self.terminal_search_mode != TerminalSearchMode::Buffer {
            return false;
        }
        let Some(session_id) = self.active_session_id.as_deref() else {
            return false;
        };
        let Some(key) = self.terminal_search_key() else {
            return false;
        };
        self.terminal_views.get(session_id).is_some_and(|view| {
            view.pending_search_key.as_ref() != Some(&key)
                && !view.search_result.as_ref().is_some_and(|result| {
                    terminal_frame_search_result_is_current(result, &key, view.screen_revision)
                })
        })
    }

    pub(in crate::features) fn enter_connect_settle(&mut self) {
        self.terminal_runtime.connect_settle_until = Some(connect_settle_deadline(Instant::now()));
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

    pub(in crate::features) fn runtime_quiet_tick_allowed(&self) -> bool {
        !self.runtime_output_pressure_active()
            && self.pending_session_starts.is_empty()
            && self.pending_saved_connection_queue.is_empty()
            && self.pending_session_events.is_empty()
            && !self.session_event_bridge.has_pending_ui_work()
            && !self.terminal_frame_backlog_active()
            && self.zmodem_sessions.is_empty()
            && self.trzsz_sessions.is_empty()
            && self.active_host_key_prompt.is_none()
            && self.active_credential_prompt.is_none()
            && self.active_keyboard_interactive_prompt.is_none()
            && self.active_duplicate_prompt.is_none()
            && !self.host_key_prompts.has_pending()
            && !self.credential_prompts.has_pending()
            && !self.duplicate_prompts.has_pending()
            && self.connection_state.list.pending_hover_is_clear()
            && self.action_link_hover_pending.is_none()
            && self.pending_auto_recording_session.is_none()
            && self.pending_tunnels.is_empty()
            && self.transfer.queue.jobs.is_empty()
            && self.command_persistence_pending == 0
            && !self.terminal_runtime.open_tabs_persist_dirty
            && !self.terminal_runtime.window_layout_persist_dirty
            && self.terminal_windows_restored
            && !self.ai.chat.pending
            && self.ai.agent.loop_state.is_none()
            && !self.ai.discovery.pending
            && !self.remote_ops.stats.pending
            && !self.remote_ops.process.pending
            && !self.remote_ops.docker.pending
            && !self.translate_pending
            && !self.update_pending
            && !self.ai.chat.focus_pending
            && !self.transfer.file_ops.rename_focus_pending
            && !self.credential_prompt_focus_pending
            && !((self.active_ssh_config.is_some()
                && matches!(
                    self.current_right_panel(),
                    Some(NavItem::Stats | NavItem::Processes | NavItem::Docker)
                ))
                || self.current_left_panel() == Some(NavItem::Transfers))
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
            && !self.remote_ops.stats.pending
            && remote_refresh_due(
                self.remote_ops.stats.last_refresh_at,
                self.settings.ui_remote_stats_interval.max(1),
            )
        {
            self.refresh_stats(window, cx);
            dirty = true;
        } else if right_panel == Some(NavItem::Processes)
            && self.settings.ui_show_process_manager
            && !self.remote_ops.process.pending
            && remote_refresh_due(
                self.remote_ops.process.last_refresh_at,
                self.settings.ui_process_manager_interval.max(3),
            )
        {
            self.refresh_processes(window, cx);
            dirty = true;
        } else if right_panel == Some(NavItem::Docker)
            && self.settings.ui_show_docker_manager
            && !self.remote_ops.docker.pending
        {
            let interval = self.settings.ui_docker_manager_interval.max(3);
            if remote_refresh_due(self.remote_ops.docker.last_refresh_at, interval) {
                self.refresh_docker(window, cx);
                dirty = true;
            } else if self.remote_ops.docker.details.is_some()
                && remote_refresh_due(self.remote_ops.docker.details_last_refresh_at, interval)
                && let Some(container_id) = self.remote_ops.docker.details_container_id.clone()
            {
                self.load_docker_details(container_id, window, cx);
                dirty = true;
            }
        }

        if left_panel == Some(NavItem::Transfers)
            && self.transfer_browser_auto_sync_cwd_enabled()
            && !self.transfer_sync_cwd_job_running()
            && remote_refresh_due(
                self.transfer.browser.auto_sync_cwd_last_at,
                TRANSFER_AUTO_SYNC_CWD_INTERVAL_SECONDS,
            )
        {
            self.transfer.browser.auto_sync_cwd_last_at = Some(Instant::now());
            self.start_transfer_sync_cwd_job(window, cx);
            dirty = true;
        }
        dirty
    }

    fn pending_session_auth_wait(&self) -> Option<PendingSessionAuthWait> {
        if let Some(prompt) = self.active_keyboard_interactive_prompt.as_ref() {
            return Some(PendingSessionAuthWait::Credential {
                target: keyboard_interactive_prompt_target(&prompt.request),
            });
        }
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
