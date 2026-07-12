use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn drain_session_events(&mut self, cx: &mut Context<Self>) -> bool {
        let mut dirty = false;
        let mut drained_events = 0usize;
        let drained_output_bytes;
        let queued_events: usize;
        let queued_output_bytes: usize;

        let Ok(drain) = self.session_manager.drain_events_with_output_budget(
            SESSION_EVENT_DRAIN_BATCH,
            SESSION_EVENT_DRAIN_OUTPUT_BUDGET,
        ) else {
            self.terminal_status = "failed to drain session events".to_string();
            return true;
        };
        let events = drain.events;
        drained_output_bytes = drain.stats.drained_output_bytes;
        queued_events = drain.stats.queued_events;
        queued_output_bytes = drain.stats.queued_output_bytes;
        if drain.stats.dropped_output_bytes > 0 {
            self.terminal_runtime.session_event_dropped_output_bytes = self
                .terminal_runtime
                .session_event_dropped_output_bytes
                .saturating_add(drain.stats.dropped_output_bytes as u64);
        }
        if !events.is_empty() {
            dirty = true;
            drained_events = events.len();

            for event in events {
                match event {
                    SessionEvent::Output { session_id, data } => {
                        // Intercept ZMODEM protocol bytes before terminal paint (Tauri parity).
                        let data = self.process_zmodem_output(&session_id, &data, cx);
                        if data.is_empty() {
                            continue;
                        }
                        if self.active_session_id.as_deref() == Some(session_id.as_str()) {
                            let text = String::from_utf8_lossy(&data);
                            if self.ai_agent_capture.has_active() {
                                let result = self.ai_agent_capture.process(&text);
                                if !result.visible_text.is_empty() {
                                    self.recording_manager
                                        .write_output(&session_id, &result.visible_text);
                                    self.append_terminal_log(&result.visible_text);
                                    self.feed_credential_autofill_output(
                                        result.visible_text.as_bytes(),
                                        cx,
                                    );
                                }
                                for captured in result.completed {
                                    self.handle_ai_agent_captured_output(captured, cx);
                                }
                            } else {
                                self.recording_manager.write_output(&session_id, &text);
                                self.append_terminal_bytes(&data);
                                self.feed_credential_autofill_output(&data, cx);
                            }
                        } else {
                            let text = String::from_utf8_lossy(&data);
                            self.recording_manager.write_output(&session_id, &text);
                            self.append_terminal_bytes_for_session(Some(&session_id), &data, true);
                        }
                    }
                    SessionEvent::OutputDropped { session_id, bytes } => {
                        if let Some(view) = self.terminal_views.get_mut(&session_id) {
                            view.note_skipped_output(bytes);
                        }
                        if self.active_session_id.as_deref() == Some(session_id.as_str()) {
                            self.terminal_status = format!(
                                "terminal output overloaded; dropped {} queued byte(s)",
                                bytes
                            );
                        }
                    }
                    SessionEvent::Exited { session_id } => {
                        self.recording_manager.cleanup_session(&session_id);
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
                        if session_id.is_empty()
                            || self.active_session_id.as_deref() == Some(session_id.as_str())
                        {
                            self.terminal_status = format!("session error: {message}");
                            self.append_terminal_log(format!("\n# session error: {message}\n"));
                        } else {
                            self.append_terminal_log_for_session(
                                Some(&session_id),
                                &format!("\n# session error: {message}\n"),
                                true,
                            );
                        }
                    }
                }
            }
        }

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

        dirty |= self.drain_session_start_events(cx);
        // Continue sequential startup restore after async SSH connects complete.
        // Window handle is not available here; pump only when not waiting on pending.
        if !self.startup_restore_complete
            && self.pending_session_name.is_none()
            && !self.startup_restore_queue.is_empty()
        {
            // Defer to next render where Window is available.
            dirty = true;
        }
        dirty |= self.drain_tunnel_events();
        dirty |= self.drain_process_events();
        dirty |= self.drain_stats_events();
        dirty |= self.drain_translate_events();
        dirty |= self.drain_update_events();
        dirty |= self.drain_docker_events();
        dirty |= self.drain_transfer_events(cx);
        dirty |= self.drain_ai_discovery_events();
        dirty |= self.drain_ai_chat_events(cx);
        dirty |= self.drive_ai_agent_loop(cx);
        dirty |= self.drain_host_key_prompts();
        dirty |= self.drain_credential_prompts();
        dirty |= self.drain_duplicate_prompts();
        dirty
    }

    pub(in crate::ui::view) fn mark_user_activity(&mut self) {
        if !self.is_locked {
            self.last_user_activity_at = Instant::now();
        }
    }

    pub(in crate::ui::view) fn drive_idle_lock(&mut self) -> bool {
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

    pub(in crate::ui::view) fn ensure_event_pump(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.terminal_runtime.event_pump_started {
            return;
        }
        self.terminal_runtime.event_pump_started = true;

        window
            .spawn(cx, async move |cx| {
                loop {
                    Timer::after(Duration::from_millis(50)).await;
                    let keep_running = cx
                        .update_root(|root, window, cx| {
                            let Ok(view) = root.downcast::<NyaTermApp>() else {
                                return false;
                            };
                            view.update(cx, |this, cx| {
                                let mut dirty = this.drain_session_events(cx);
                                dirty |= this.drive_terminal_resize();
                                dirty |= this.poll_action_link_tooltip_delay(cx);
                                dirty |= this.drive_remote_auto_refresh(window, cx);
                                dirty |= this.drive_idle_lock();
                                // ~530ms blink half-period (50ms * 11 ticks) when enabled.
                                if this.settings.cursor_blink {
                                    this.terminal_runtime.cursor_blink_tick =
                                        this.terminal_runtime.cursor_blink_tick.wrapping_add(1);
                                    if this.terminal_runtime.cursor_blink_tick >= 11 {
                                        this.terminal_runtime.cursor_blink_tick = 0;
                                        this.terminal_runtime.cursor_blink_on =
                                            !this.terminal_runtime.cursor_blink_on;
                                        dirty = true;
                                    }
                                } else {
                                    if !this.terminal_runtime.cursor_blink_on
                                        || this.terminal_runtime.cursor_blink_tick != 0
                                    {
                                        dirty = true;
                                    }
                                    this.terminal_runtime.cursor_blink_on = true;
                                    this.terminal_runtime.cursor_blink_tick = 0;
                                }
                                // Visual BEL flash (~200ms at 50ms ticks).
                                if this.terminal_runtime.visual_bell_ticks > 0 {
                                    this.terminal_runtime.visual_bell_ticks =
                                        this.terminal_runtime.visual_bell_ticks.saturating_sub(1);
                                    dirty = true;
                                }
                                // Large-output protection recovery accounting.
                                for view in this.terminal_views.values_mut() {
                                    let before = view.performance_overlay;
                                    view.tick_performance_overlay();
                                    if view.performance_overlay != before {
                                        dirty = true;
                                    }
                                }
                                // Drop overlay only while a platform drag is active.
                                if this.terminal_file_drop_hover.is_some() && !cx.has_active_drag()
                                {
                                    this.terminal_file_drop_hover = None;
                                    dirty = true;
                                }
                                if dirty {
                                    cx.notify();
                                }
                                this.terminal_runtime.event_pump_started
                            })
                        })
                        .unwrap_or(false);
                    if !keep_running {
                        break;
                    }
                }
            })
            .detach();
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
}

const TRANSFER_AUTO_SYNC_CWD_INTERVAL_SECONDS: u32 = 3;
const SESSION_EVENT_DRAIN_BATCH: usize = 256;
const SESSION_EVENT_DRAIN_OUTPUT_BUDGET: usize = 32 * 1024;

fn remote_refresh_due(last_refresh_at: Option<Instant>, interval_seconds: u32) -> bool {
    last_refresh_at.is_none_or(|last_refresh_at| {
        last_refresh_at.elapsed() >= Duration::from_secs(u64::from(interval_seconds))
    })
}
