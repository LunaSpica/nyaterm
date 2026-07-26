use super::*;

use crate::models::AiDetectedErrorState;

#[derive(Clone, Copy)]
enum SessionOutputDrainStep {
    SidebandOnly { chunk_duration: Duration },
    Accepted { chunk_duration: Duration },
}

impl SessionOutputDrainStep {
    fn chunk_duration(self) -> Duration {
        match self {
            Self::SidebandOnly { chunk_duration } | Self::Accepted { chunk_duration } => {
                chunk_duration
            }
        }
    }
}

fn terminal_output_has_error_keyword(output: &str) -> bool {
    let lower = output.to_ascii_lowercase();
    [
        "permission denied",
        "no space left on device",
        "connection refused",
        "segmentation fault",
        "out of memory",
        "cannot allocate memory",
        "command not found",
        "module not found",
        "port already in use",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
        || ascii_word_present(&lower, "error")
        || ascii_word_present(&lower, "failed")
}

fn ascii_word_present(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(index, _)| {
        let before = index
            .checked_sub(1)
            .and_then(|before| text.as_bytes().get(before))
            .copied();
        let after = text.as_bytes().get(index + word.len()).copied();
        !before.is_some_and(|value| value.is_ascii_alphanumeric() || value == b'_')
            && !after.is_some_and(|value| value.is_ascii_alphanumeric() || value == b'_')
    })
}

fn terminal_error_notice_output(output: &str) -> String {
    const LIMIT: usize = 4_000;
    let char_count = output.chars().count();
    if char_count <= LIMIT {
        return output.to_string();
    }
    output
        .chars()
        .skip(char_count.saturating_sub(LIMIT))
        .collect()
}

impl NyaTermApp {
    pub(in crate::features) fn drain_session_events(&mut self, cx: &mut Context<Self>) -> bool {
        let settle = connect_settle_active(
            self.terminal.view.runtime.connect_settle_until,
            Instant::now(),
        );
        let mut drain_budget =
            session_event_drain_budget(self.runtime_output_pressure_active() || settle);
        if settle {
            // First frames after connect: smaller wall budget leaves room for paint.
            drain_budget.wall_budget = Duration::from_millis(4);
            drain_budget.max_output_bytes = drain_budget.max_output_bytes.min(4 * 1024);
        }
        self.drain_session_events_with_budget(cx, drain_budget, true)
    }

    pub(in crate::features) fn drain_session_events_for_input_wake(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        self.drain_session_events_with_budget(cx, session_event_input_wake_drain_budget(), false)
    }

    fn drain_session_events_with_budget(
        &mut self,
        cx: &mut Context<Self>,
        drain_budget: SessionEventDrainBudget,
        drain_sideband_workers: bool,
    ) -> bool {
        let drain_started_at = Instant::now();
        let mut dirty = false;
        // Common calm path: no local pending events, no bridge UI work, no file
        // transfer sideband. Skip harvest/atomics so idle and window-drag ticks
        // do not touch the session event pipeline at all.
        if self.pending_session_events.is_empty()
            && !self.session_event_bridge.has_pending_ui_work()
            && (!drain_sideband_workers
                || (self.zmodem_sessions.is_empty() && self.trzsz_sessions.is_empty()))
        {
            if self.terminal.view.runtime.session_event_queued_events != 0
                || self.terminal.view.runtime.session_event_queued_output_bytes != 0
                || self.terminal.view.runtime.session_event_backlog_active
                || self
                    .terminal
                    .view
                    .runtime
                    .session_event_last_output_event_count
                    != 0
                || self
                    .terminal
                    .view
                    .runtime
                    .session_event_last_drained_output_bytes
                    != 0
            {
                self.terminal.view.runtime.session_event_queued_events = 0;
                self.terminal.view.runtime.session_event_queued_output_bytes = 0;
                self.terminal.view.runtime.session_event_backlog_active = false;
                self.terminal
                    .view
                    .runtime
                    .session_event_last_output_event_count = 0;
                self.terminal
                    .view
                    .runtime
                    .session_event_last_drained_output_bytes = 0;
            }
            return false;
        }
        // Bridge encoding/scrollback and per-session routing are updated on the
        // state transitions that need them, not on every runtime tick.
        if drain_sideband_workers {
            dirty |= self.drain_zmodem_worker_events(cx);
            dirty |= self.drain_trzsz_download_worker_events(cx);
            dirty |= self.drain_trzsz_upload_prepare_events(cx);
            dirty |= self.drain_trzsz_upload_worker_events(cx);
        }
        let mut drained_events = 0usize;
        let mut output_event_count = 0usize;
        let mut drain_timings = SessionEventDrainTimings::default();
        let mut max_output_chunk_duration = Duration::ZERO;
        let mut processed_output_bytes = 0usize;
        let mut transport_queued_events = 0usize;
        let mut transport_queued_output_bytes = 0usize;
        let mut bridge_direct_output_events = 0u64;
        let mut bridge_direct_output_bytes = 0u64;
        let mut bridge_direct_backpressure_events = 0u64;
        let mut bridge_direct_backpressure_bytes = 0u64;
        let mut bridge_drained_ui_events = 0usize;
        let mut bridge_drained_ui_output_bytes = 0usize;
        let mut pending_frame_outputs: Vec<(String, Vec<u8>)> = Vec::new();
        if self.pending_session_events.is_empty() {
            if self.session_event_bridge.has_pending_ui_work() {
                let drain = self.session_event_bridge.drain_events_with_output_budget(
                    drain_budget.max_events,
                    drain_budget.max_output_bytes,
                );
                transport_queued_events = drain
                    .stats
                    .source_queued_events
                    .saturating_add(drain.stats.ui_queued_events);
                transport_queued_output_bytes = drain
                    .stats
                    .source_queued_output_bytes
                    .saturating_add(drain.stats.ui_queued_output_bytes);
                bridge_direct_output_events = drain.stats.direct_output_events;
                bridge_direct_output_bytes = drain.stats.direct_output_bytes;
                bridge_direct_backpressure_events = drain.stats.direct_backpressure_events;
                bridge_direct_backpressure_bytes = drain.stats.direct_backpressure_bytes;
                bridge_drained_ui_events = drain.stats.drained_ui_events;
                bridge_drained_ui_output_bytes = drain.stats.drained_ui_output_bytes;
                if drain.stats.dropped_output_bytes > 0 {
                    self.terminal
                        .view
                        .runtime
                        .session_event_dropped_output_bytes = self
                        .terminal
                        .view
                        .runtime
                        .session_event_dropped_output_bytes
                        .saturating_add(drain.stats.dropped_output_bytes as u64);
                }
                self.pending_session_events.extend(drain.events);
            } else {
                // Direct-output-only ticks: harvest counters without UI queue lock.
                let stats = self.session_event_bridge.harvest_direct_stats();
                transport_queued_events = stats
                    .source_queued_events
                    .saturating_add(stats.ui_queued_events);
                transport_queued_output_bytes = stats
                    .source_queued_output_bytes
                    .saturating_add(stats.ui_queued_output_bytes);
                bridge_direct_output_events = stats.direct_output_events;
                bridge_direct_output_bytes = stats.direct_output_bytes;
                bridge_direct_backpressure_events = stats.direct_backpressure_events;
                bridge_direct_backpressure_bytes = stats.direct_backpressure_bytes;
            }
        }

        if !self.pending_session_events.is_empty() {
            while let Some(event) = self.pending_session_events.pop_front() {
                drained_events += 1;
                match event {
                    SessionEvent::Output { session_id, data } => {
                        output_event_count += 1;
                        let chunk_input_bytes = data.len();
                        processed_output_bytes =
                            processed_output_bytes.saturating_add(chunk_input_bytes);
                        let step = self.handle_session_output_event(
                            session_id,
                            data,
                            &mut pending_frame_outputs,
                            &mut drain_timings,
                            cx,
                        );
                        max_output_chunk_duration =
                            max_output_chunk_duration.max(step.chunk_duration());
                        if matches!(step, SessionOutputDrainStep::SidebandOnly { .. })
                            && session_event_drain_should_yield(
                                drain_started_at,
                                !self.pending_session_events.is_empty(),
                                transport_queued_events,
                                transport_queued_output_bytes,
                                drain_budget,
                            )
                        {
                            break;
                        }
                    }
                    SessionEvent::OutputDropped { session_id, bytes } => {
                        self.flush_pending_session_frame_outputs(
                            &mut pending_frame_outputs,
                            &mut drain_timings,
                        );
                        dirty |= self.handle_session_output_dropped_event(session_id, bytes, cx);
                    }
                    SessionEvent::Exited { session_id, reason } => {
                        self.flush_pending_session_frame_outputs(
                            &mut pending_frame_outputs,
                            &mut drain_timings,
                        );
                        dirty |= self.handle_session_exited_event(session_id, reason, cx);
                    }
                    SessionEvent::Error {
                        session_id,
                        message,
                    } => {
                        self.flush_pending_session_frame_outputs(
                            &mut pending_frame_outputs,
                            &mut drain_timings,
                        );
                        dirty |= self.handle_session_error_event(session_id, message);
                    }
                }
                if session_event_drain_should_yield(
                    drain_started_at,
                    !self.pending_session_events.is_empty(),
                    transport_queued_events,
                    transport_queued_output_bytes,
                    drain_budget,
                ) {
                    break;
                }
            }
        }
        self.flush_pending_session_frame_outputs(&mut pending_frame_outputs, &mut drain_timings);

        let queued_events =
            transport_queued_events.saturating_add(self.pending_session_events.len());
        let queued_output_bytes = transport_queued_output_bytes
            .saturating_add(session_events_output_bytes(&self.pending_session_events));
        let drained_output_bytes = processed_output_bytes;
        self.terminal.view.runtime.session_event_queued_events = queued_events;
        self.terminal.view.runtime.session_event_queued_output_bytes = queued_output_bytes;
        self.terminal
            .view
            .runtime
            .session_event_last_output_event_count = output_event_count;
        self.terminal
            .view
            .runtime
            .session_event_last_drained_output_bytes = drained_output_bytes;

        self.terminal.view.runtime.session_event_backlog_active = session_event_backlog_active(
            drained_events,
            drained_output_bytes,
            queued_output_bytes,
            drain_budget,
        );

        if session_event_drain_is_slow(drain_timings.output_total, max_output_chunk_duration)
            && self.should_log_slow_diagnostic("session_event_drain", Instant::now())
        {
            tracing::warn!(
                diagnostic = "session_event_drain",
                drained_events,
                output_event_count,
                drained_output_bytes,
                drain_output_budget = drain_budget.max_output_bytes,
                queued_events,
                queued_output_bytes,
                bridge_direct_output_events,
                bridge_direct_output_bytes,
                bridge_direct_backpressure_events,
                bridge_direct_backpressure_bytes,
                bridge_drained_ui_events,
                bridge_drained_ui_output_bytes,
                dropped_output_bytes = self
                    .terminal
                    .view
                    .runtime
                    .session_event_dropped_output_bytes,
                drain_total_ms = drain_started_at.elapsed().as_millis(),
                output_total_ms = drain_timings.output_total.as_millis(),
                max_output_chunk_ms = max_output_chunk_duration.as_millis(),
                zmodem_us = drain_timings.zmodem.as_micros(),
                trzsz_us = drain_timings.trzsz.as_micros(),
                decode_us = drain_timings.decode.as_micros(),
                recording_us = drain_timings.recording.as_micros(),
                terminal_append_us = drain_timings.terminal_append.as_micros(),
                credential_autofill_us = drain_timings.credential_autofill.as_micros(),
                ai_capture_us = drain_timings.ai_capture.as_micros(),
                "slow session event drain"
            );
        }
        dirty
    }

    fn handle_session_output_dropped_event(
        &mut self,
        session_id: String,
        bytes: usize,
        cx: &mut Context<Self>,
    ) -> bool {
        self.note_trzsz_output_discontinuity(&session_id);
        self.note_zmodem_output_discontinuity(&session_id, bytes, cx);
        self.note_ai_agent_output_discontinuity(&session_id, bytes, cx);
        self.session_event_bridge.route_session_to_ui(&session_id);
        let encoding = self.settings.interaction_default_encoding.clone();
        let view = self
            .terminal
            .view
            .views
            .entry(session_id.clone())
            .or_insert_with(TerminalViewState::new);
        view.set_encoding(&encoding);
        view.note_output_discontinuity(bytes);
        let marker = terminal_output_dropped_marker(bytes);
        self.recording_write_pipeline
            .write_output(session_id.clone(), marker.clone());
        self.append_terminal_log_for_session(Some(&session_id), &marker, true);
        if self.active_session_id.as_deref() == Some(session_id.as_str()) {
            self.terminal.view.status = format!(
                "terminal output overloaded; dropped {} queued byte(s)",
                bytes
            );
            return true;
        }
        false
    }

    fn handle_session_exited_event(
        &mut self,
        session_id: String,
        reason: String,
        cx: &mut Context<Self>,
    ) -> bool {
        let known_session = self.session_metadata.contains_key(&session_id);
        tracing::warn!(
            diagnostic = "session_exited",
            session_id = %session_id,
            reason = %reason,
            known_session,
            "session exited or disconnected"
        );
        let log_reason = terminal_log_plain_text(&reason);
        let log = format!("\n# session disconnected: {log_reason}\n");
        if !session_id.is_empty() {
            self.recording_write_pipeline
                .write_output(session_id.clone(), log.clone());
            self.append_terminal_log_for_session(Some(&session_id), &log, true);
        }
        self.clear_trzsz_session(&session_id);
        self.clear_zmodem_session(&session_id);
        self.session_event_bridge.clear_session(&session_id);
        self.cleanup_recording_for_session(&session_id);
        let _ = self.session_manager.close(&session_id);
        if known_session {
            // Keep the tab so the user can reconnect (Tauri disconnected pane).
            self.mark_session_disconnected(&session_id, cx);
            self.terminal.view.status = format!("session disconnected {}", short_id(&session_id));
        } else {
            self.terminal.view.status = format!("session exited {}", short_id(&session_id));
        }
        true
    }

    fn handle_session_error_event(&mut self, session_id: String, message: String) -> bool {
        tracing::warn!(
            diagnostic = "session_error",
            session_id = %session_id,
            message = %message,
            "session error"
        );
        let log_message = terminal_log_plain_text(&message);
        let log = format!("\n# session error: {log_message}\n");
        if !session_id.is_empty() {
            self.sync_session_event_bridge_session_policy(&session_id);
            self.recording_write_pipeline
                .write_output(session_id.clone(), log.clone());
        }
        if session_id.is_empty() || self.active_session_id.as_deref() == Some(session_id.as_str()) {
            self.terminal.view.status = format!("session error: {message}");
            self.append_terminal_log(log);
        } else {
            self.append_terminal_log_for_session(Some(&session_id), &log, true);
        }
        true
    }

    fn handle_session_output_event(
        &mut self,
        session_id: String,
        data: Vec<u8>,
        pending_frame_outputs: &mut Vec<(String, Vec<u8>)>,
        drain_timings: &mut SessionEventDrainTimings,
        cx: &mut Context<Self>,
    ) -> SessionOutputDrainStep {
        let chunk_started_at = Instant::now();
        let chunk_input_bytes = data.len();
        let mut chunk_timings = SessionEventDrainTimings::default();
        let sideband_bypass = self.session_output_can_bypass_sideband_detectors(&session_id, &data);
        let data = if sideband_bypass {
            data
        } else {
            let stage_started_at = Instant::now();
            let data = self.process_zmodem_output(&session_id, &data, cx);
            let stage_duration = stage_started_at.elapsed();
            drain_timings.zmodem += stage_duration;
            chunk_timings.zmodem += stage_duration;
            if data.is_empty() {
                let chunk_duration = chunk_started_at.elapsed();
                drain_timings.output_total += chunk_duration;
                chunk_timings.output_total += chunk_duration;
                self.maybe_log_slow_session_output_chunk(
                    &session_id,
                    chunk_input_bytes,
                    chunk_duration,
                    &chunk_timings,
                );
                return SessionOutputDrainStep::SidebandOnly { chunk_duration };
            }
            // Consume side-band markers after active transfer payloads are removed.
            let stage_started_at = Instant::now();
            let data = self.process_trzsz_output(&session_id, &data, cx);
            let stage_duration = stage_started_at.elapsed();
            drain_timings.trzsz += stage_duration;
            chunk_timings.trzsz += stage_duration;
            if data.is_empty() {
                let chunk_duration = chunk_started_at.elapsed();
                drain_timings.output_total += chunk_duration;
                chunk_timings.output_total += chunk_duration;
                self.maybe_log_slow_session_output_chunk(
                    &session_id,
                    chunk_input_bytes,
                    chunk_duration,
                    &chunk_timings,
                );
                return SessionOutputDrainStep::SidebandOnly { chunk_duration };
            }
            data
        };
        if self.session_has_active_ai_capture(&session_id) {
            self.flush_pending_session_frame_outputs(pending_frame_outputs, drain_timings);
            let stage_started_at = Instant::now();
            let text = self.decode_session_output_for_recording(&session_id, &data);
            let stage_duration = stage_started_at.elapsed();
            drain_timings.decode += stage_duration;
            chunk_timings.decode += stage_duration;
            let stage_started_at = Instant::now();
            let result = self.ai.agent.capture.process(&text);
            let stage_duration = stage_started_at.elapsed();
            drain_timings.ai_capture += stage_duration;
            chunk_timings.ai_capture += stage_duration;
            if !result.visible_text.is_empty() {
                let stage_started_at = Instant::now();
                let visible_bytes =
                    self.encode_visible_terminal_text_for_output(&session_id, &result.visible_text);
                self.submit_terminal_frame_output(&session_id, visible_bytes);
                let stage_duration = stage_started_at.elapsed();
                drain_timings.terminal_append += stage_duration;
                chunk_timings.terminal_append += stage_duration;
            }
            let stage_started_at = Instant::now();
            for captured in result.completed {
                self.handle_ai_agent_captured_output(captured, cx);
            }
            let stage_duration = stage_started_at.elapsed();
            drain_timings.ai_capture += stage_duration;
            chunk_timings.ai_capture += stage_duration;
        } else {
            self.maybe_detect_ai_terminal_error(&session_id, &data, cx);
            pending_frame_outputs.push((session_id.clone(), data));
        }
        // Routing only changes when sideband detectors activate/deactivate.
        if !sideband_bypass {
            self.sync_session_event_bridge_session_policy(&session_id);
        }
        let chunk_duration = chunk_started_at.elapsed();
        drain_timings.output_total += chunk_duration;
        chunk_timings.output_total += chunk_duration;
        self.maybe_log_slow_session_output_chunk(
            &session_id,
            chunk_input_bytes,
            chunk_duration,
            &chunk_timings,
        );
        SessionOutputDrainStep::Accepted { chunk_duration }
    }

    fn maybe_detect_ai_terminal_error(
        &mut self,
        session_id: &str,
        data: &[u8],
        cx: &mut Context<Self>,
    ) {
        if data.is_empty() {
            return;
        }
        let watched = self.active_session_id.as_deref() == Some(session_id)
            || self
                .ai
                .chat
                .target_session_ids
                .iter()
                .any(|target_id| target_id == session_id);
        if !watched {
            return;
        }

        let output = String::from_utf8_lossy(data);
        if !terminal_output_has_error_keyword(&output) {
            return;
        }

        let now = Instant::now();
        if self
            .ai
            .panel
            .error_notice_at
            .get(session_id)
            .is_some_and(|last| now.duration_since(*last) < Duration::from_secs(30))
        {
            return;
        }

        self.ai
            .panel
            .error_notice_at
            .insert(session_id.to_string(), now);
        self.ai.panel.detected_error = Some(AiDetectedErrorState {
            session_id: session_id.to_string(),
            output: terminal_error_notice_output(&output),
        });
        self.ai.panel.status = "terminal error detected".to_string();
        cx.notify();
    }

    pub(super) fn maybe_log_slow_session_output_chunk(
        &mut self,
        session_id: &str,
        chunk_input_bytes: usize,
        chunk_duration: Duration,
        timings: &SessionEventDrainTimings,
    ) {
        if chunk_duration < SESSION_EVENT_DRAIN_SLOW_CHUNK {
            return;
        }
        if !self.should_log_slow_diagnostic("session_event_output_chunk", Instant::now()) {
            return;
        }
        tracing::warn!(
            diagnostic = "session_event_drain",
            session_id = %session_id,
            chunk_input_bytes,
            chunk_duration_ms = chunk_duration.as_millis(),
            zmodem_us = timings.zmodem.as_micros(),
            trzsz_us = timings.trzsz.as_micros(),
            decode_us = timings.decode.as_micros(),
            recording_us = timings.recording.as_micros(),
            terminal_append_us = timings.terminal_append.as_micros(),
            credential_autofill_us = timings.credential_autofill.as_micros(),
            ai_capture_us = timings.ai_capture.as_micros(),
            "slow session output chunk"
        );
    }

    pub(super) fn session_has_active_ai_capture(&self, session_id: &str) -> bool {
        self.ai.agent.capture.has_active()
            && self
                .ai
                .agent
                .loop_state
                .as_ref()
                .is_some_and(|state| state.terminal_session_id == session_id)
    }

    pub(super) fn flush_pending_session_frame_outputs(
        &self,
        pending_frame_outputs: &mut Vec<(String, Vec<u8>)>,
        timings: &mut SessionEventDrainTimings,
    ) {
        if pending_frame_outputs.is_empty() {
            return;
        }
        let stage_started_at = Instant::now();
        self.submit_terminal_frame_outputs(std::mem::take(pending_frame_outputs));
        let stage_duration = stage_started_at.elapsed();
        timings.terminal_append += stage_duration;
        timings.output_total += stage_duration;
    }

    pub(super) fn terminal_frame_backlog_active(&self) -> bool {
        terminal_frame_backlog_active_from_counts(
            self.pending_terminal_frame_events.len(),
            self.terminal.view.frame_pipeline.queued_event_count(),
            self.terminal.view.frame_pipeline.queued_command_count(),
        )
    }

    pub(super) fn session_sideband_detectors_idle(&self, session_id: &str) -> bool {
        self.zmodem_output_can_bypass_detector(session_id, &[])
            && self.trzsz_output_can_bypass_detector(session_id, &[])
    }

    pub(super) fn session_output_can_bypass_sideband_detectors(
        &self,
        session_id: &str,
        data: &[u8],
    ) -> bool {
        self.zmodem_output_can_bypass_detector(session_id, data)
            && self.trzsz_output_can_bypass_detector(session_id, data)
    }
}
