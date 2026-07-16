use super::*;

impl NyaTermApp {
    fn drive_startup_restore_queue_tick(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let should_pump = self.stores.startup_restore.update(cx, |store, _| {
            store.can_pump_queue(self.has_pending_session_start())
        });
        if !should_pump {
            return false;
        }
        self.pump_startup_restore_queue(window, cx);
        true
    }

    fn drive_pending_focus(&mut self, window: &mut Window) -> bool {
        let mut dirty = false;
        if self.ai_chat_focus_pending {
            window.focus(&self.ai_chat_focus);
            self.ai_chat_focus_pending = false;
            dirty = true;
        }
        if self.transfer_rename_focus_pending && self.transfer_rename.is_some() {
            window.focus(&self.transfer_rename_focus);
            self.transfer_rename_focus_pending = false;
            dirty = true;
        }
        if self.credential_prompt_focus_pending && self.active_credential_prompt.is_some() {
            window.focus(&self.credential_focus);
            self.credential_prompt_focus_pending = false;
            dirty = true;
        }
        dirty
    }

    pub(super) fn drain_runtime_background_events(
        &mut self,
        cx: &mut Context<Self>,
        started_at: Instant,
        timings: &mut RuntimeBackgroundDrainTimings,
        critical_only: bool,
        defer_terminal_frames: bool,
    ) -> bool {
        let mut dirty = false;
        macro_rules! drain_stage {
            ($field:ident, $expr:expr) => {{
                let stage_started_at = Instant::now();
                dirty |= $expr;
                timings.$field += stage_started_at.elapsed();
                if runtime_background_event_drain_budget_exhausted(started_at) {
                    timings.budget_exhausted = true;
                    return dirty;
                }
            }};
        }

        // Data plane only. Session start / prompts already ran on the control plane.
        if defer_terminal_frames {
            // Leave room for paint after a fresh output drain.
            timings.terminal_frames_deferred = true;
            return dirty;
        }
        drain_stage!(terminal_frames, self.drain_terminal_frame_events(cx));
        if critical_only {
            // Autofill / recording / transfer / remote are idle-plane sideband.
            return dirty;
        }
        drain_stage!(
            credential_autofill,
            self.drain_pending_credential_autofill_detection(cx)
        );
        drain_stage!(recording, self.drain_recording_pipeline_events());
        drain_stage!(transfer, self.drain_transfer_events(cx));
        drain_stage!(
            ai,
            self.drain_ai_discovery_events()
                | self.drain_ai_chat_events(cx)
                | self.drive_ai_agent_loop(cx)
        );
        drain_stage!(
            remote,
            self.drain_tunnel_events()
                | self.drain_process_events()
                | self.drain_stats_events()
                | self.drain_docker_events()
        );
        drain_stage!(
            maintenance,
            self.drain_translate_events() | self.drain_update_events()
        );

        dirty
    }

    pub(crate) fn drive_window_runtime_tick(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let tick_started_at = Instant::now();
        let stage_started_at = Instant::now();
        let mut dirty = self.refresh_window_render_inputs(window, cx);
        let render_input_duration = stage_started_at.elapsed();

        let control = self.drive_runtime_control_plane(window, cx);
        dirty |= control.dirty;

        let stage_started_at = Instant::now();
        dirty |= self.drain_session_events(cx);
        let session_events_duration = stage_started_at.elapsed();

        let data = self.drive_runtime_data_plane(tick_started_at, cx);
        dirty |= data.dirty;
        self.terminal_runtime.last_session_start_drain_duration = control.timings.session_start;
        self.maybe_log_slow_runtime_background_event_drain(
            data.background_total,
            &data.background_timings,
            data.defer_terminal_frame_after_output,
            data.terminal_frame_apply_paced,
        );

        let idle = self.drive_runtime_idle_plane(window, cx);
        dirty |= idle.dirty;

        let visual = self.drive_runtime_visual_plane(cx);
        dirty |= visual.dirty;

        let pending_session_stage_started_at = Instant::now();
        dirty |= self.drive_pending_session_status();
        let pending_session_status_duration = pending_session_stage_started_at.elapsed();
        let visual_dirty = dirty;
        let notify_started_at = Instant::now();
        if visual_dirty {
            cx.notify();
        }
        let notify_duration = notify_started_at.elapsed();
        let publish_started_at = Instant::now();
        // Planes above do not drain more output; reuse one final pressure sample.
        let output_pressure = self.runtime_output_pressure_active();
        let should_publish_snapshots = should_publish_store_snapshots(
            visual_dirty,
            output_pressure,
            store_snapshot_publish_due(
                self.terminal_runtime.last_store_snapshot_publish_at,
                publish_started_at,
            ),
        );
        if should_publish_snapshots {
            self.publish_store_snapshots(cx);
        }
        let publish_duration = publish_started_at.elapsed();
        let tick_duration = tick_started_at.elapsed();
        if tick_duration >= RUNTIME_TICK_SLOW_THRESHOLD
            && self.should_log_slow_diagnostic("runtime_tick", Instant::now())
        {
            tracing::warn!(
                diagnostic = "runtime_tick",
                total_ms = tick_duration.as_millis(),
                render_input_ms = render_input_duration.as_millis(),
                control_plane_ms = control.duration.as_millis(),
                control_session_start_ms = control.timings.session_start.as_millis(),
                control_prompts_ms = control.timings.prompts.as_millis(),
                control_saved_connection_queue_ms =
                    control.timings.saved_connection_queue.as_millis(),
                session_events_ms = session_events_duration.as_millis(),
                background_runtime_ms = data.background_total.as_millis(),
                terminal_frames_deferred = data.background_timings.terminal_frames_deferred,
                terminal_frames_deferred_after_output = data.defer_terminal_frame_after_output,
                terminal_frames_deferred_for_pacing = data.terminal_frame_apply_paced,
                startup_restore_ms = idle.startup_restore.as_millis(),
                saved_connection_queue_ms = control.timings.saved_connection_queue.as_millis(),
                terminal_resize_ms = idle.terminal_resize.as_millis(),
                render_requests_ms = idle.render_requests.as_millis(),
                render_requests_output_pressure = idle.render_request_output_pressure,
                pending_focus_ms = idle.pending_focus.as_millis(),
                connection_hover_ms = idle.connection_hover.as_millis(),
                action_link_tooltip_ms = idle.action_link_tooltip.as_millis(),
                remote_refresh_ms = idle.remote_refresh.as_millis(),
                idle_lock_ms = idle.idle_lock.as_millis(),
                visual_runtime_ms = visual.duration.as_millis(),
                pending_session_status_ms = pending_session_status_duration.as_millis(),
                notify_ms = notify_duration.as_millis(),
                publish_snapshots_ms = publish_duration.as_millis(),
                queued_events = self.terminal_runtime.session_event_queued_events,
                queued_output_bytes = self.terminal_runtime.session_event_queued_output_bytes,
                frame_command_count = self.terminal_frame_pipeline.queued_command_count(),
                frame_command_output_bytes = self.terminal_frame_pipeline.queued_output_bytes(),
                frame_event_count = self.terminal_frame_pipeline.queued_event_count(),
                pending_frame_events = self.pending_terminal_frame_events.len(),
                pending_session_starts = self.pending_session_starts.len(),
                queued_saved_connection_starts = self.pending_saved_connection_queue.len(),
                output_pressure,
                next_tick_delay_ms = self.window_runtime_tick_delay().as_millis(),
                visual_dirty,
                notify_requested = visual_dirty,
                publish_snapshots = should_publish_snapshots,
                "slow runtime tick"
            );
        }
        self.terminal_runtime.event_pump_started
    }

    pub(super) fn drive_runtime_control_plane(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> RuntimeControlPlaneResult {
        let started_at = Instant::now();
        let mut timings = RuntimeControlPlaneDrainTimings::default();
        let mut dirty = false;

        let stage_started_at = Instant::now();
        dirty |= self.drain_session_start_events(cx);
        timings.session_start = stage_started_at.elapsed();

        let stage_started_at = Instant::now();
        dirty |= self.drain_host_key_prompts()
            | self.drain_credential_prompts()
            | self.drain_duplicate_prompts();
        timings.prompts = stage_started_at.elapsed();

        let stage_started_at = Instant::now();
        dirty |= self.drive_saved_connection_start_queue(window, cx);
        timings.saved_connection_queue = stage_started_at.elapsed();

        RuntimeControlPlaneResult {
            dirty,
            duration: started_at.elapsed(),
            timings,
        }
    }

    pub(super) fn drive_runtime_data_plane(
        &mut self,
        tick_started_at: Instant,
        cx: &mut Context<Self>,
    ) -> RuntimeDataPlaneResult {
        let background_started_at = Instant::now();
        let mut background_timings = RuntimeBackgroundDrainTimings::default();
        let critical_background_only = self.runtime_output_pressure_active();
        let terminal_frame_backlog_active = self.terminal_frame_backlog_active();
        let terminal_frame_apply_paced = terminal_frame_backlog_active
            && terminal_frame_apply_should_defer(
                self.terminal_runtime.last_terminal_frame_apply_at,
                tick_started_at,
                critical_background_only,
            );
        let defer_terminal_frame_after_output = runtime_background_should_defer_terminal_frames(
            self.terminal_runtime.session_event_last_output_event_count,
            self.terminal_runtime
                .session_event_last_drained_output_bytes,
            terminal_frame_backlog_active,
            terminal_frame_apply_paced,
        );
        let defer_terminal_frame_apply =
            defer_terminal_frame_after_output || terminal_frame_apply_paced;
        let dirty = self.drain_runtime_background_events(
            cx,
            background_started_at,
            &mut background_timings,
            critical_background_only,
            defer_terminal_frame_apply,
        );
        RuntimeDataPlaneResult {
            dirty,
            background_total: background_started_at.elapsed(),
            background_timings,
            defer_terminal_frame_after_output,
            terminal_frame_apply_paced,
        }
    }

    pub(super) fn maybe_log_slow_runtime_background_event_drain(
        &mut self,
        background_total: Duration,
        background_timings: &RuntimeBackgroundDrainTimings,
        defer_terminal_frame_after_output: bool,
        terminal_frame_apply_paced: bool,
    ) {
        if !(background_timings.budget_exhausted
            || background_total >= RUNTIME_BACKGROUND_EVENT_DRAIN_SLOW)
            || !self.should_log_slow_diagnostic("runtime_background_event_drain", Instant::now())
        {
            return;
        }
        tracing::warn!(
            diagnostic = "runtime_background_event_drain",
            total_ms = background_total.as_millis(),
            terminal_frames_ms = background_timings.terminal_frames.as_millis(),
            terminal_frames_deferred = background_timings.terminal_frames_deferred,
            terminal_frames_deferred_after_output = defer_terminal_frame_after_output,
            terminal_frames_deferred_for_pacing = terminal_frame_apply_paced,
            credential_autofill_ms = background_timings.credential_autofill.as_millis(),
            recording_ms = background_timings.recording.as_millis(),
            transfer_ms = background_timings.transfer.as_millis(),
            ai_ms = background_timings.ai.as_millis(),
            remote_ms = background_timings.remote.as_millis(),
            maintenance_ms = background_timings.maintenance.as_millis(),
            budget_exhausted = background_timings.budget_exhausted,
            "slow runtime background event drain"
        );
    }

    pub(super) fn drive_runtime_idle_plane(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> RuntimeIdlePlaneResult {
        let mut dirty = false;
        let mut result = RuntimeIdlePlaneResult::default();
        // Idle-plane work does not drain output; one pressure sample is enough for the stage.
        let output_pressure = self.runtime_output_pressure_active();
        let pending_session_start = self.has_pending_session_start();
        let queued_saved_connection_start = !self.pending_saved_connection_queue.is_empty();
        let idle_plane_allowed = runtime_idle_plane_allowed(output_pressure);

        let stage_started_at = Instant::now();
        if idle_plane_allowed {
            dirty |= self.drive_startup_restore_queue_tick(window, cx);
        }
        result.startup_restore = stage_started_at.elapsed();

        let stage_started_at = Instant::now();
        // Bounds paint path already resizes; polling is idle-plane maintenance.
        if idle_plane_allowed {
            dirty |= self.drive_terminal_resize();
        }
        result.terminal_resize = stage_started_at.elapsed();

        let stage_started_at = Instant::now();
        dirty |= self.drive_terminal_render_requests(!output_pressure);
        result.render_request_output_pressure = output_pressure;
        result.render_requests = stage_started_at.elapsed();

        let stage_started_at = Instant::now();
        dirty |= self.drive_pending_focus(window);
        result.pending_focus = stage_started_at.elapsed();

        let stage_started_at = Instant::now();
        if connection_hover_poll_allowed(
            output_pressure,
            pending_session_start,
            queued_saved_connection_start,
        ) {
            dirty |= self.poll_connection_hover_delay();
        }
        result.connection_hover = stage_started_at.elapsed();

        let stage_started_at = Instant::now();
        if idle_plane_allowed {
            dirty |= self.poll_action_link_tooltip_delay(cx);
        }
        result.action_link_tooltip = stage_started_at.elapsed();

        let stage_started_at = Instant::now();
        if idle_plane_allowed {
            dirty |= self.drive_remote_auto_refresh(window, cx);
        }
        result.remote_refresh = stage_started_at.elapsed();

        let stage_started_at = Instant::now();
        if idle_plane_allowed {
            dirty |= self.drive_idle_lock();
        }
        result.idle_lock = stage_started_at.elapsed();
        result.dirty = dirty;
        result
    }

    pub(super) fn drive_runtime_visual_plane(
        &mut self,
        cx: &mut Context<Self>,
    ) -> RuntimeVisualPlaneResult {
        let visual_stage_started_at = Instant::now();
        let mut dirty = false;
        let output_pressure = self.runtime_output_pressure_active();
        // ~530ms blink half-period (50ms * 11 ticks) when enabled.
        // Under output pressure, keep last blink phase so we do not force redraws.
        if runtime_cursor_blink_allowed(output_pressure, self.settings.cursor_blink) {
            self.terminal_runtime.cursor_blink_tick =
                self.terminal_runtime.cursor_blink_tick.wrapping_add(1);
            if self.terminal_runtime.cursor_blink_tick >= 11 {
                self.terminal_runtime.cursor_blink_tick = 0;
                self.terminal_runtime.cursor_blink_on = !self.terminal_runtime.cursor_blink_on;
                dirty = true;
            }
        } else if !self.settings.cursor_blink {
            if !self.terminal_runtime.cursor_blink_on
                || self.terminal_runtime.cursor_blink_tick != 0
            {
                dirty = true;
            }
            self.terminal_runtime.cursor_blink_on = true;
            self.terminal_runtime.cursor_blink_tick = 0;
        }
        // Visual BEL flash (~200ms at 50ms ticks).
        if self.terminal_runtime.visual_bell_ticks > 0 {
            self.terminal_runtime.visual_bell_ticks =
                self.terminal_runtime.visual_bell_ticks.saturating_sub(1);
            dirty = true;
        }
        let render_work_pressure = terminal_render_work_pressure_active(
            output_pressure,
            self.has_pending_session_start(),
            !self.pending_saved_connection_queue.is_empty(),
        );
        // Large-output protection recovery accounting.
        let visible_session_ids = self.visible_terminal_session_ids();
        for session_id in terminal_performance_tick_session_ids(&visible_session_ids) {
            if let Some(view) = self.terminal_views.get_mut(&session_id) {
                let before = view.performance_overlay;
                let was_render_degraded = view.render_degraded;
                view.tick_performance_overlay(render_work_pressure);
                if view.performance_overlay != before || view.render_degraded != was_render_degraded
                {
                    dirty = true;
                }
            }
        }
        // Drop overlay only while a platform drag is active.
        if self.terminal_file_drop_hover.is_some() && !cx.has_active_drag() {
            self.terminal_file_drop_hover = None;
            dirty = true;
        }
        RuntimeVisualPlaneResult {
            dirty,
            duration: visual_stage_started_at.elapsed(),
        }
    }
}
