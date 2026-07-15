use std::fmt::Write as _;

use super::*;

impl NyaTermApp {
    pub(in crate::features) fn publish_store_snapshots(&mut self, cx: &mut Context<Self>) {
        self.terminal_runtime.last_store_snapshot_publish_at = Some(Instant::now());

        let workspace = crate::entities::WorkspaceSnapshot {
            active_session_id: self.active_session_id.clone(),
            ordered_tab_roots: self
                .ordered_tab_sessions()
                .into_iter()
                .map(|session| session.id)
                .collect(),
            selected_nav: self.selected_nav.label().to_string(),
            main_mode: match self.main_mode {
                MainMode::Workspace => "Workspace",
                MainMode::Page => "Page",
            }
            .to_string(),
            active_left_panel: self.active_left_panel.map(|item| item.label().to_string()),
            active_right_panel: self.active_right_panel.map(|item| item.label().to_string()),
            left_sidebar_collapsed: self.left_sidebar_collapsed,
            right_inspector_collapsed: self.right_inspector_collapsed,
            workspace_split_active: self.workspace_split.is_some(),
            terminal_windows_active: self.terminal_windows.is_some(),
        };

        let live_session_ids = self
            .session_manager
            .list_sessions()
            .unwrap_or_default()
            .into_iter()
            .map(|session| session.id)
            .collect();
        let pending_start_count = usize::from(self.pending_session_name.is_some());
        let sessions = crate::entities::SessionSnapshot {
            active_session_id: self.active_session_id.clone(),
            ordered_session_ids: self.session_order.clone(),
            live_session_ids,
            metadata_count: self.session_metadata.len(),
            terminal_view_count: self.terminal_views.len(),
            pending_start_count,
            host_prompt_active: self.active_host_key_prompt.is_some(),
            credential_prompt_active: self.active_credential_prompt.is_some(),
            zmodem_session_count: self.zmodem_sessions.len(),
        };

        let overlays = crate::entities::OverlaySnapshot {
            quick_switch_open: self.quick_switch_open,
            tab_actions_open: self.tab_actions_session_id.is_some(),
            rename_open: self.rename_session_id.is_some(),
            color_picker_open: self.color_picker_open,
            session_info_open: self.session_info_open,
            startup_command_open: self.startup_command_open,
            temporary_ssh_link_open: self.temporary_ssh_link_open,
            multi_line_paste_open: self.multi_line_paste.is_some(),
            terminal_actions_open: self.terminal_actions_open,
            terminal_context_menu_open: self.terminal_context_menu.is_some(),
            action_link_menu_open: self.action_link_menu.is_some(),
            action_link_tooltip_open: self.action_link_tooltip.is_some(),
            command_suggestions_open: self.command_suggestions.is_some(),
            credential_suggestions_open: self.credential_suggestions.is_some(),
            close_all_sessions_confirm_open: self.close_all_sessions_confirm_open,
            locked: self.is_locked,
        };

        self.stores.workspace.update(cx, |store, cx| {
            if store.replace_snapshot(workspace) {
                cx.notify();
            }
        });
        self.stores.sessions.update(cx, |store, cx| {
            if store.replace_snapshot(sessions) {
                cx.notify();
            }
        });
        self.stores.overlays.update(cx, |store, cx| {
            if store.replace_snapshot(overlays) {
                cx.notify();
            }
        });

        let settings = crate::entities::SettingsSnapshot {
            active_tab: self.settings_active_tab.label().to_string(),
            has_master_password: self.settings.has_master_password,
            security_unlocked: self.security_secrets_unlocked,
            cloud_sync_enabled: self.cloud_sync_settings.enabled,
            startup_restore: self.settings.startup_restore,
        };
        let connections = crate::entities::ConnectionsSnapshot {
            connection_count: self.connections.len(),
            group_count: self.connection_groups.len(),
            search_active: !self.connection_search_draft.trim().is_empty(),
            editor_open: self.connection_editor.is_some(),
            group_editor_open: self.connection_group_editor.is_some(),
            delete_confirm_open: self.connection_delete_confirm.is_some()
                || self.connection_group_delete_confirm.is_some(),
            sort_mode: format!("{:?}", self.connection_sort_mode),
        };
        let active_job_count = self
            .transfer_jobs
            .iter()
            .filter(|job| {
                !matches!(
                    job.status,
                    TransferJobStatus::Completed
                        | TransferJobStatus::Failed
                        | TransferJobStatus::Cancelled
                )
            })
            .count();
        let transfers = crate::entities::TransferSnapshot {
            job_count: self.transfer_jobs.len(),
            active_job_count,
            browser_path: self.transfer_browser_path.clone(),
            selected_count: self.transfer_selected_remote_paths.len(),
            browser_busy: self.transfer_browser_home_dir_pending
                || self.transfer_path_prompt.is_some(),
        };
        let ai = crate::entities::AiSnapshot {
            chat_pending: self.ai_chat_pending,
            message_count: self.ai_chat_messages.len(),
            session_id: self.ai_chat_session_id.clone(),
            agent_active: self.ai_agent_loop.is_some(),
        };
        let cloud_sync = crate::entities::CloudSyncSnapshot {
            enabled: self.cloud_sync_settings.enabled,
            provider: self.cloud_sync_settings.provider.clone(),
            conflict_active: self.cloud_sync_conflict.is_some(),
            last_status: self.cloud_sync_status.clone(),
        };
        let remote_ops = crate::entities::RemoteOpsSnapshot {
            process_count: self.processes.len(),
            docker_tab: self.docker_tab.label().to_string(),
            stats_ready: self.remote_stats.is_some(),
            confirm_open: self.docker_confirm.is_some() || self.process_signal_confirm.is_some(),
        };

        self.stores.settings.update(cx, |store, cx| {
            if store.replace_snapshot(settings) {
                cx.notify();
            }
        });
        self.stores.connections.update(cx, |store, cx| {
            if store.replace_snapshot(connections) {
                cx.notify();
            }
        });
        self.stores.transfers.update(cx, |store, cx| {
            if store.replace_snapshot(transfers) {
                cx.notify();
            }
        });
        self.stores.ai.update(cx, |store, cx| {
            if store.replace_snapshot(ai) {
                cx.notify();
            }
        });
        self.stores.cloud_sync.update(cx, |store, cx| {
            if store.replace_snapshot(cloud_sync) {
                cx.notify();
            }
        });
        self.stores.remote_ops.update(cx, |store, cx| {
            if store.replace_snapshot(remote_ops) {
                cx.notify();
            }
        });
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
        self.refresh_terminal_cell_metrics(cx);
        if self.terminal_cell_metrics != before_metrics {
            self.sync_terminal_cell_metrics_to_screens();
            self.resize_all_known_terminal_surfaces();
        }
        self.last_viewport_size != before_viewport || self.terminal_cell_metrics != before_metrics
    }

    fn drive_startup_restore_queue_tick(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let should_pump = self.stores.startup_restore.update(cx, |store, _| {
            store.can_pump_queue(self.pending_session_name.is_some())
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
        dirty
    }

    pub(in crate::features) fn drain_session_events(&mut self, cx: &mut Context<Self>) -> bool {
        let drain_started_at = Instant::now();
        let mut dirty = self.drain_terminal_frame_events(cx);
        dirty |= self.drain_trzsz_download_worker_events(cx);
        let mut drained_events = 0usize;
        let mut output_event_count = 0usize;
        let mut drain_timings = SessionEventDrainTimings::default();
        let mut max_output_chunk_duration = Duration::ZERO;
        let mut processed_output_bytes = 0usize;
        let mut transport_queued_events = 0usize;
        let mut transport_queued_output_bytes = 0usize;

        if self.pending_session_events.is_empty() {
            let Ok(drain) = self.session_manager.drain_events_with_output_budget(
                SESSION_EVENT_DRAIN_BATCH,
                SESSION_EVENT_DRAIN_OUTPUT_BUDGET,
            ) else {
                self.terminal_status = "failed to drain session events".to_string();
                return true;
            };
            transport_queued_events = drain.stats.queued_events;
            transport_queued_output_bytes = drain.stats.queued_output_bytes;
            if drain.stats.dropped_output_bytes > 0 {
                self.terminal_runtime.session_event_dropped_output_bytes = self
                    .terminal_runtime
                    .session_event_dropped_output_bytes
                    .saturating_add(drain.stats.dropped_output_bytes as u64);
            }
            self.pending_session_events.extend(drain.events);
        }

        if !self.pending_session_events.is_empty() {
            dirty = true;

            while let Some(event) = self.pending_session_events.pop_front() {
                drained_events += 1;
                match event {
                    SessionEvent::Output { session_id, data } => {
                        output_event_count += 1;
                        let chunk_started_at = Instant::now();
                        let chunk_input_bytes = data.len();
                        processed_output_bytes =
                            processed_output_bytes.saturating_add(chunk_input_bytes);
                        let mut chunk_timings = SessionEventDrainTimings::default();
                        let stage_started_at = Instant::now();
                        let data = self.process_zmodem_output(&session_id, &data, cx);
                        let stage_duration = stage_started_at.elapsed();
                        drain_timings.zmodem += stage_duration;
                        chunk_timings.zmodem += stage_duration;
                        if data.is_empty() {
                            let chunk_duration = chunk_started_at.elapsed();
                            drain_timings.output_total += chunk_duration;
                            chunk_timings.output_total += chunk_duration;
                            max_output_chunk_duration =
                                max_output_chunk_duration.max(chunk_duration);
                            self.maybe_log_slow_session_output_chunk(
                                &session_id,
                                chunk_input_bytes,
                                chunk_duration,
                                &chunk_timings,
                            );
                            if session_event_drain_wall_budget_exhausted(
                                drain_started_at,
                                !self.pending_session_events.is_empty(),
                            ) {
                                break;
                            }
                            continue;
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
                            max_output_chunk_duration =
                                max_output_chunk_duration.max(chunk_duration);
                            self.maybe_log_slow_session_output_chunk(
                                &session_id,
                                chunk_input_bytes,
                                chunk_duration,
                                &chunk_timings,
                            );
                            if session_event_drain_wall_budget_exhausted(
                                drain_started_at,
                                !self.pending_session_events.is_empty(),
                            ) {
                                break;
                            }
                            continue;
                        }
                        if self.session_has_active_ai_capture(&session_id) {
                            let stage_started_at = Instant::now();
                            let text = self.decode_session_output_for_recording(&session_id, &data);
                            let stage_duration = stage_started_at.elapsed();
                            drain_timings.decode += stage_duration;
                            chunk_timings.decode += stage_duration;
                            let stage_started_at = Instant::now();
                            let result = self.ai_agent_capture.process(&text);
                            let stage_duration = stage_started_at.elapsed();
                            drain_timings.ai_capture += stage_duration;
                            chunk_timings.ai_capture += stage_duration;
                            if !result.visible_text.is_empty() {
                                let stage_started_at = Instant::now();
                                let visible_bytes = self.encode_visible_terminal_text_for_output(
                                    &session_id,
                                    &result.visible_text,
                                );
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
                            let stage_started_at = Instant::now();
                            self.submit_terminal_frame_output(&session_id, data);
                            let stage_duration = stage_started_at.elapsed();
                            drain_timings.terminal_append += stage_duration;
                            chunk_timings.terminal_append += stage_duration;
                        }
                        let chunk_duration = chunk_started_at.elapsed();
                        drain_timings.output_total += chunk_duration;
                        chunk_timings.output_total += chunk_duration;
                        max_output_chunk_duration = max_output_chunk_duration.max(chunk_duration);
                        self.maybe_log_slow_session_output_chunk(
                            &session_id,
                            chunk_input_bytes,
                            chunk_duration,
                            &chunk_timings,
                        );
                    }
                    SessionEvent::OutputDropped { session_id, bytes } => {
                        self.note_trzsz_output_discontinuity(&session_id);
                        self.note_zmodem_output_discontinuity(&session_id, bytes, cx);
                        self.note_ai_agent_output_discontinuity(&session_id, bytes, cx);
                        let encoding = self.settings.interaction_default_encoding.clone();
                        let view = self
                            .terminal_views
                            .entry(session_id.clone())
                            .or_insert_with(TerminalViewState::new);
                        view.set_encoding(&encoding);
                        view.note_output_discontinuity(bytes);
                        let marker = terminal_output_dropped_marker(bytes);
                        self.recording_write_pipeline
                            .write_output(session_id.clone(), marker.clone());
                        self.append_terminal_log_for_session(Some(&session_id), &marker, true);
                        if self.active_session_id.as_deref() == Some(session_id.as_str()) {
                            self.terminal_status = format!(
                                "terminal output overloaded; dropped {} queued byte(s)",
                                bytes
                            );
                        }
                    }
                    SessionEvent::Exited { session_id } => {
                        self.clear_trzsz_session(&session_id);
                        self.clear_zmodem_session(&session_id);
                        self.recording_write_pipeline
                            .cleanup_session(session_id.clone());
                        let _ = self.session_manager.close(&session_id);
                        if self.session_metadata.contains_key(&session_id) {
                            // Keep the tab so the user can reconnect (Tauri disconnected pane).
                            self.mark_session_disconnected(&session_id, cx);
                            self.terminal_status =
                                format!("session disconnected {}", short_id(&session_id));
                        } else {
                            self.terminal_status =
                                format!("session exited {}", short_id(&session_id));
                        }
                    }
                    SessionEvent::Error {
                        session_id,
                        message,
                    } => {
                        let log_message = terminal_log_plain_text(&message);
                        let log = format!("\n# session error: {log_message}\n");
                        if !session_id.is_empty() {
                            self.recording_write_pipeline
                                .write_output(session_id.clone(), log.clone());
                        }
                        if session_id.is_empty()
                            || self.active_session_id.as_deref() == Some(session_id.as_str())
                        {
                            self.terminal_status = format!("session error: {message}");
                            self.append_terminal_log(log);
                        } else {
                            self.append_terminal_log_for_session(Some(&session_id), &log, true);
                        }
                    }
                }
                if session_event_drain_wall_budget_exhausted(
                    drain_started_at,
                    !self.pending_session_events.is_empty(),
                ) {
                    break;
                }
            }
        }

        let queued_events =
            transport_queued_events.saturating_add(self.pending_session_events.len());
        let queued_output_bytes = transport_queued_output_bytes
            .saturating_add(session_events_output_bytes(&self.pending_session_events));
        let drained_output_bytes = processed_output_bytes;
        self.terminal_runtime.session_event_queued_events = queued_events;
        self.terminal_runtime.session_event_queued_output_bytes = queued_output_bytes;
        self.terminal_runtime
            .session_event_last_drained_output_bytes = drained_output_bytes;

        if drained_events >= SESSION_EVENT_DRAIN_BATCH
            || drained_output_bytes >= SESSION_EVENT_DRAIN_OUTPUT_BUDGET
            || queued_output_bytes > 0
        {
            if !self.terminal_runtime.session_event_backlog_active {
                self.terminal_status = format!(
                    "terminal output busy; processed {drained_events} event(s), {queued_output_bytes} byte(s) queued"
                );
                dirty = true;
            }
            self.terminal_runtime.session_event_backlog_active = true;
        } else if self.terminal_runtime.session_event_backlog_active {
            self.terminal_runtime.session_event_backlog_active = false;
            self.terminal_status = "terminal output caught up".to_string();
            dirty = true;
        }

        let background_started_at = Instant::now();
        let mut background_timings = RuntimeBackgroundDrainTimings::default();
        dirty |= self.drain_runtime_background_events(
            cx,
            background_started_at,
            &mut background_timings,
        );
        self.terminal_runtime.last_session_start_drain_duration = background_timings.session_start;
        let background_total = background_started_at.elapsed();
        if (background_timings.budget_exhausted
            || background_total >= RUNTIME_BACKGROUND_EVENT_DRAIN_SLOW)
            && self.should_log_slow_diagnostic("runtime_background_event_drain", Instant::now())
        {
            tracing::warn!(
                diagnostic = "runtime_background_event_drain",
                total_ms = background_total.as_millis(),
                session_start_ms = background_timings.session_start.as_millis(),
                prompts_ms = background_timings.prompts.as_millis(),
                terminal_frames_ms = background_timings.terminal_frames.as_millis(),
                credential_autofill_ms = background_timings.credential_autofill.as_millis(),
                recording_ms = background_timings.recording.as_millis(),
                startup_restore_ms = background_timings.startup_restore.as_millis(),
                transfer_ms = background_timings.transfer.as_millis(),
                ai_ms = background_timings.ai.as_millis(),
                remote_ms = background_timings.remote.as_millis(),
                maintenance_ms = background_timings.maintenance.as_millis(),
                budget_exhausted = background_timings.budget_exhausted,
                "slow runtime background event drain"
            );
        }
        if session_event_drain_is_slow(drain_timings.output_total, max_output_chunk_duration)
            && self.should_log_slow_diagnostic("session_event_drain", Instant::now())
        {
            tracing::warn!(
                diagnostic = "session_event_drain",
                drained_events,
                output_event_count,
                drained_output_bytes,
                queued_events,
                queued_output_bytes,
                dropped_output_bytes = self.terminal_runtime.session_event_dropped_output_bytes,
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

    fn drain_runtime_background_events(
        &mut self,
        cx: &mut Context<Self>,
        started_at: Instant,
        timings: &mut RuntimeBackgroundDrainTimings,
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

        drain_stage!(session_start, self.drain_session_start_events(cx));
        drain_stage!(
            prompts,
            self.drain_host_key_prompts()
                | self.drain_credential_prompts()
                | self.drain_duplicate_prompts()
        );
        drain_stage!(terminal_frames, self.drain_terminal_frame_events(cx));
        drain_stage!(
            credential_autofill,
            self.drain_pending_credential_autofill_detection(cx)
        );
        drain_stage!(recording, self.drain_recording_pipeline_events());
        // Continue sequential startup restore after async SSH connects complete.
        // Window handle is not available here; pump only when not waiting on pending.
        drain_stage!(
            startup_restore,
            self.stores.startup_restore.update(cx, |store, _| {
                store.can_pump_queue(self.pending_session_name.is_some())
            })
        );
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

    fn maybe_log_slow_session_output_chunk(
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

    fn runtime_output_pressure_active(&self) -> bool {
        self.terminal_runtime.session_event_backlog_active
            || self.terminal_runtime.session_event_queued_output_bytes > 0
            || !self.pending_session_events.is_empty()
            || !self.pending_terminal_frame_events.is_empty()
            || !self.pending_session_starts.is_empty()
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
        let stage_started_at = Instant::now();
        dirty |= self.drain_session_events(cx);
        let session_events_duration = stage_started_at.elapsed();
        let session_start_duration = self.terminal_runtime.last_session_start_drain_duration;
        let stage_started_at = Instant::now();
        dirty |= self.drive_startup_restore_queue_tick(window, cx);
        let startup_restore_duration = stage_started_at.elapsed();
        let stage_started_at = Instant::now();
        dirty |= self.drive_terminal_resize();
        let terminal_resize_duration = stage_started_at.elapsed();
        let stage_started_at = Instant::now();
        dirty |= self.drive_pending_focus(window);
        let pending_focus_duration = stage_started_at.elapsed();
        let stage_started_at = Instant::now();
        dirty |= self.poll_action_link_tooltip_delay(cx);
        let action_link_tooltip_duration = stage_started_at.elapsed();
        let output_pressure = self.runtime_output_pressure_active();
        let stage_started_at = Instant::now();
        if !output_pressure {
            dirty |= self.drive_remote_auto_refresh(window, cx);
        }
        let remote_refresh_duration = stage_started_at.elapsed();
        let stage_started_at = Instant::now();
        dirty |= self.drive_idle_lock();
        let idle_lock_duration = stage_started_at.elapsed();
        let visual_stage_started_at = Instant::now();
        // ~530ms blink half-period (50ms * 11 ticks) when enabled.
        if self.settings.cursor_blink {
            self.terminal_runtime.cursor_blink_tick =
                self.terminal_runtime.cursor_blink_tick.wrapping_add(1);
            if self.terminal_runtime.cursor_blink_tick >= 11 {
                self.terminal_runtime.cursor_blink_tick = 0;
                self.terminal_runtime.cursor_blink_on = !self.terminal_runtime.cursor_blink_on;
                dirty = true;
            }
        } else {
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
        // Large-output protection recovery accounting.
        for view in self.terminal_views.values_mut() {
            let before = view.performance_overlay;
            view.tick_performance_overlay();
            if view.performance_overlay != before {
                dirty = true;
            }
        }
        // Drop overlay only while a platform drag is active.
        if self.terminal_file_drop_hover.is_some() && !cx.has_active_drag() {
            self.terminal_file_drop_hover = None;
            dirty = true;
        }
        let visual_runtime_duration = visual_stage_started_at.elapsed();
        let notify_started_at = Instant::now();
        if dirty {
            cx.notify();
        }
        let notify_duration = notify_started_at.elapsed();
        let publish_started_at = Instant::now();
        let should_publish_snapshots = dirty
            || store_snapshot_publish_due(
                self.terminal_runtime.last_store_snapshot_publish_at,
                publish_started_at,
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
                session_events_ms = session_events_duration.as_millis(),
                session_start_ms = session_start_duration.as_millis(),
                startup_restore_ms = startup_restore_duration.as_millis(),
                terminal_resize_ms = terminal_resize_duration.as_millis(),
                pending_focus_ms = pending_focus_duration.as_millis(),
                action_link_tooltip_ms = action_link_tooltip_duration.as_millis(),
                remote_refresh_ms = remote_refresh_duration.as_millis(),
                idle_lock_ms = idle_lock_duration.as_millis(),
                visual_runtime_ms = visual_runtime_duration.as_millis(),
                notify_ms = notify_duration.as_millis(),
                publish_snapshots_ms = publish_duration.as_millis(),
                queued_events = self.terminal_runtime.session_event_queued_events,
                queued_output_bytes = self.terminal_runtime.session_event_queued_output_bytes,
                pending_session_starts = self.pending_session_starts.len(),
                output_pressure,
                next_tick_delay_ms = self.window_runtime_tick_delay().as_millis(),
                dirty,
                "slow runtime tick"
            );
        }
        self.terminal_runtime.event_pump_started
    }

    fn drive_remote_auto_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
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

    fn session_has_active_ai_capture(&self, session_id: &str) -> bool {
        self.ai_agent_capture.has_active()
            && self
                .ai_agent_loop
                .as_ref()
                .is_some_and(|state| state.terminal_session_id == session_id)
    }
}

const TRANSFER_AUTO_SYNC_CWD_INTERVAL_SECONDS: u32 = 3;
const SESSION_EVENT_DRAIN_BATCH: usize = 256;
const SESSION_EVENT_DRAIN_OUTPUT_BUDGET: usize = 32 * 1024;
const SESSION_EVENT_DRAIN_WALL_BUDGET: Duration = Duration::from_millis(8);
const RUNTIME_BACKGROUND_EVENT_DRAIN_WALL_BUDGET: Duration = Duration::from_millis(6);
const RUNTIME_BACKGROUND_EVENT_DRAIN_SLOW: Duration = Duration::from_millis(12);
const RUNTIME_IDLE_TICK_INTERVAL: Duration = Duration::from_millis(50);
const RUNTIME_PRESSURE_TICK_INTERVAL: Duration = Duration::from_millis(8);
const SLOW_DIAGNOSTIC_THROTTLE: Duration = Duration::from_secs(2);
const RUNTIME_TICK_SLOW_THRESHOLD: Duration = Duration::from_millis(40);
const SESSION_EVENT_DRAIN_SLOW_TOTAL: Duration = Duration::from_millis(20);
const SESSION_EVENT_DRAIN_SLOW_CHUNK: Duration = Duration::from_millis(8);
const STORE_SNAPSHOT_HEARTBEAT: Duration = Duration::from_secs(1);

#[derive(Default)]
struct SessionEventDrainTimings {
    output_total: Duration,
    zmodem: Duration,
    trzsz: Duration,
    decode: Duration,
    recording: Duration,
    terminal_append: Duration,
    credential_autofill: Duration,
    ai_capture: Duration,
}

#[derive(Default)]
struct RuntimeBackgroundDrainTimings {
    session_start: Duration,
    prompts: Duration,
    terminal_frames: Duration,
    credential_autofill: Duration,
    recording: Duration,
    startup_restore: Duration,
    transfer: Duration,
    ai: Duration,
    remote: Duration,
    maintenance: Duration,
    budget_exhausted: bool,
}

fn diagnostic_log_due(last_at: Option<Instant>, now: Instant, throttle: Duration) -> bool {
    last_at.is_none_or(|last_at| {
        now.checked_duration_since(last_at)
            .is_none_or(|elapsed| elapsed >= throttle)
    })
}

fn store_snapshot_publish_due(last_at: Option<Instant>, now: Instant) -> bool {
    diagnostic_log_due(last_at, now, STORE_SNAPSHOT_HEARTBEAT)
}

fn runtime_tick_interval_for_pressure(output_pressure: bool) -> Duration {
    if output_pressure {
        RUNTIME_PRESSURE_TICK_INTERVAL
    } else {
        RUNTIME_IDLE_TICK_INTERVAL
    }
}

fn session_event_drain_is_slow(total: Duration, max_chunk: Duration) -> bool {
    total >= SESSION_EVENT_DRAIN_SLOW_TOTAL || max_chunk >= SESSION_EVENT_DRAIN_SLOW_CHUNK
}

fn session_event_drain_wall_budget_exhausted(started_at: Instant, has_pending: bool) -> bool {
    has_pending && started_at.elapsed() >= SESSION_EVENT_DRAIN_WALL_BUDGET
}

fn session_events_output_bytes(events: &VecDeque<SessionEvent>) -> usize {
    events
        .iter()
        .map(|event| match event {
            SessionEvent::Output { data, .. } => data.len(),
            _ => 0,
        })
        .sum()
}

fn runtime_background_event_drain_budget_exhausted(started_at: Instant) -> bool {
    started_at.elapsed() >= RUNTIME_BACKGROUND_EVENT_DRAIN_WALL_BUDGET
}

fn remote_refresh_due(last_refresh_at: Option<Instant>, interval_seconds: u32) -> bool {
    last_refresh_at.is_none_or(|last_refresh_at| {
        last_refresh_at.elapsed() >= Duration::from_secs(u64::from(interval_seconds))
    })
}

fn terminal_output_dropped_marker(bytes: usize) -> String {
    format!("\r\n[nyaterm: dropped {bytes} queued output byte(s)]\r\n")
}

fn terminal_log_plain_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x1b' => out.push_str("\\x1b"),
            ch if ch.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_dropped_marker_is_plain_terminal_text() {
        let marker = terminal_output_dropped_marker(42);
        assert_eq!(
            marker,
            "\r\n[nyaterm: dropped 42 queued output byte(s)]\r\n"
        );
        assert!(marker.is_ascii());
        assert!(!marker.contains('\x1b'));
    }

    #[test]
    fn terminal_log_plain_text_escapes_control_sequences() {
        let message = "失败\x1b]52;c;AAAA\x07\nnext\tfield";
        let escaped = terminal_log_plain_text(message);

        assert_eq!(escaped, "失败\\x1b]52;c;AAAA\\u{7}\\nnext\\tfield");
        assert!(!escaped.contains('\x1b'));
        assert!(!escaped.contains('\x07'));
        assert!(!escaped.contains('\n'));
        assert!(escaped.contains("失败"));
    }

    #[test]
    fn diagnostic_log_due_respects_throttle_window() {
        let start = Instant::now();

        assert!(diagnostic_log_due(None, start, SLOW_DIAGNOSTIC_THROTTLE));
        assert!(!diagnostic_log_due(
            Some(start),
            start + Duration::from_millis(1999),
            SLOW_DIAGNOSTIC_THROTTLE
        ));
        assert!(diagnostic_log_due(
            Some(start),
            start + SLOW_DIAGNOSTIC_THROTTLE,
            SLOW_DIAGNOSTIC_THROTTLE
        ));
    }

    #[test]
    fn store_snapshot_publish_due_uses_low_frequency_heartbeat() {
        let start = Instant::now();

        assert!(store_snapshot_publish_due(None, start));
        assert!(!store_snapshot_publish_due(
            Some(start),
            start + STORE_SNAPSHOT_HEARTBEAT - Duration::from_millis(1)
        ));
        assert!(store_snapshot_publish_due(
            Some(start),
            start + STORE_SNAPSHOT_HEARTBEAT
        ));
    }

    #[test]
    fn runtime_tick_interval_uses_fast_cadence_under_output_pressure() {
        assert_eq!(
            runtime_tick_interval_for_pressure(false),
            RUNTIME_IDLE_TICK_INTERVAL
        );
        assert_eq!(
            runtime_tick_interval_for_pressure(true),
            RUNTIME_PRESSURE_TICK_INTERVAL
        );
    }

    #[test]
    fn session_event_drain_slow_budget_flags_total_or_chunk() {
        assert!(!session_event_drain_is_slow(
            Duration::from_millis(19),
            Duration::from_millis(7)
        ));
        assert!(session_event_drain_is_slow(
            SESSION_EVENT_DRAIN_SLOW_TOTAL,
            Duration::from_millis(1)
        ));
        assert!(session_event_drain_is_slow(
            Duration::from_millis(1),
            SESSION_EVENT_DRAIN_SLOW_CHUNK
        ));
    }

    #[test]
    fn session_event_drain_wall_budget_only_stops_with_pending_events() {
        let start = Instant::now() - SESSION_EVENT_DRAIN_WALL_BUDGET;

        assert!(!session_event_drain_wall_budget_exhausted(start, false));
        assert!(session_event_drain_wall_budget_exhausted(start, true));
    }

    #[test]
    fn session_events_output_bytes_counts_only_output_payloads() {
        let mut events = VecDeque::new();
        events.push_back(SessionEvent::Output {
            session_id: "a".to_string(),
            data: vec![1, 2, 3],
        });
        events.push_back(SessionEvent::Error {
            session_id: "a".to_string(),
            message: "nope".to_string(),
        });
        events.push_back(SessionEvent::Output {
            session_id: "b".to_string(),
            data: vec![4, 5],
        });

        assert_eq!(session_events_output_bytes(&events), 5);
    }

    #[test]
    fn runtime_background_event_drain_budget_exhaustion_tracks_elapsed_time() {
        let start = Instant::now() - RUNTIME_BACKGROUND_EVENT_DRAIN_WALL_BUDGET;

        assert!(runtime_background_event_drain_budget_exhausted(start));
        assert!(!runtime_background_event_drain_budget_exhausted(
            Instant::now()
        ));
    }
}
