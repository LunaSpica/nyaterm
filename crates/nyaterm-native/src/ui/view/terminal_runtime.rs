use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn open_terminal_actions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal_actions_open = true;
        self.terminal_status = "terminal actions opened".to_string();
        window.focus(&self.terminal_actions_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn close_terminal_actions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal_actions_open = false;
        self.terminal_status = "terminal actions closed".to_string();
        window.focus(&self.terminal_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn active_terminal_visible_text(&self) -> String {
        self.active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.screen.lines().join("\n"))
            .unwrap_or_else(|| self.terminal_screen.lines().join("\n"))
    }

    pub(in crate::ui::view) fn active_terminal_buffer_text(&self) -> String {
        self.active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.output.clone())
            .unwrap_or_else(|| self.terminal_output.clone())
    }

    pub(in crate::ui::view) fn copy_terminal_visible_text(&mut self, cx: &mut Context<Self>) {
        let text = self.active_terminal_visible_text();
        if text.trim().is_empty() {
            self.terminal_status = "visible terminal text is empty".to_string();
        } else {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            self.terminal_status = "copied visible terminal text".to_string();
        }
        self.terminal_actions_open = false;
        cx.notify();
    }

    pub(in crate::ui::view) fn copy_terminal_buffer_text(&mut self, cx: &mut Context<Self>) {
        let text = self.active_terminal_buffer_text();
        if text.trim().is_empty() {
            self.terminal_status = "terminal buffer is empty".to_string();
        } else {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            self.terminal_status = "copied terminal buffer".to_string();
        }
        self.terminal_actions_open = false;
        cx.notify();
    }

    pub(in crate::ui::view) fn send_terminal_clear_screen(&mut self, cx: &mut Context<Self>) {
        self.terminal_actions_open = false;
        self.send_terminal_input(vec![0x0c], cx);
        self.terminal_status = "clear screen command sent".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn send_terminal_input(
        &mut self,
        bytes: Vec<u8>,
        cx: &mut Context<Self>,
    ) {
        if bytes.is_empty() {
            return;
        }
        let Some(session_id) = self.active_session_id.clone() else {
            self.terminal_status = "start a session before typing".to_string();
            cx.notify();
            return;
        };
        // Typing while scrolled in history returns to the live bottom (xterm-like).
        if self.active_terminal_scroll_offset() > 0 {
            self.scroll_terminal_to_bottom(cx);
        }
        let peers = self.sync_peer_session_ids(&session_id);
        let byte_count = bytes.len();
        self.send_terminal_input_to_session(session_id, bytes.clone(), cx);
        let mut synced = 0usize;
        let mut failed = 0usize;
        for peer_id in peers {
            match self.session_manager.write(&peer_id, &bytes) {
                Ok(()) => {
                    synced += 1;
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }
        if synced > 0 || failed > 0 {
            self.terminal_status = if failed == 0 {
                format!("sent {byte_count} byte(s) + synced {synced} peer(s)")
            } else {
                format!("sent {byte_count} byte(s), synced {synced} peer(s), {failed} failed")
            };
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn send_terminal_input_to_session(
        &mut self,
        session_id: String,
        bytes: Vec<u8>,
        cx: &mut Context<Self>,
    ) {
        if bytes.is_empty() {
            return;
        }
        match self.session_manager.write(&session_id, &bytes) {
            Ok(()) => {
                self.recording_manager.write_input(&session_id, &bytes);
                self.record_command_history_from_bytes(Some(&session_id), &bytes);
                self.terminal_status = format!("sent {} byte(s)", bytes.len());
            }
            Err(error) => {
                self.terminal_status = format!("input failed: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn terminal_key_bytes_for_event(
        &self,
        event: &KeyDownEvent,
    ) -> Option<Vec<u8>> {
        if self.settings.interaction_alt_as_meta
            && event.keystroke.modifiers.alt
            && !event.keystroke.modifiers.control
            && !event.keystroke.modifiers.platform
            && !event.keystroke.modifiers.function
            && let Some(input) = event.keystroke.key_char.as_deref()
            && !input.is_empty()
        {
            let mut bytes = Vec::with_capacity(input.len() + 1);
            bytes.push(0x1b);
            bytes.extend_from_slice(input.as_bytes());
            return Some(bytes);
        }
        terminal_key_bytes(event)
    }

    pub(in crate::ui::view) fn paste_from_clipboard(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            self.terminal_status = "clipboard does not contain text".to_string();
            cx.notify();
            return;
        };
        self.paste_terminal_text(text, window, cx);
    }

    pub(in crate::ui::view) fn paste_terminal_text(
        &mut self,
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if text.is_empty() {
            self.terminal_status = "clipboard text is empty".to_string();
            cx.notify();
            return;
        }
        if self.settings.terminal_show_multi_line_paste_dialog && is_multi_line_paste(&text) {
            self.multi_line_paste = Some(MultiLinePasteDraft::new(text));
            self.terminal_status = "multi-line paste confirmation opened".to_string();
            window.focus(&self.multi_line_paste_focus);
            cx.notify();
            return;
        }
        let payload = normalize_paste_newlines(&text);
        self.send_terminal_input(self.wrap_terminal_paste_bytes(&payload), cx);
    }


    fn active_terminal_bracketed_paste(&self) -> bool {
        if let Some(session_id) = self.active_session_id.as_deref() {
            self.terminal_views
                .get(session_id)
                .map(|view| view.screen.bracketed_paste())
                .unwrap_or(false)
        } else {
            self.terminal_screen.bracketed_paste()
        }
    }

    fn wrap_terminal_paste_bytes(&self, text: &str) -> Vec<u8> {
        let body = text.as_bytes();
        if self.active_terminal_bracketed_paste() {
            let mut out = Vec::with_capacity(body.len() + 12);
            out.extend_from_slice(b"\x1b[200~");
            out.extend_from_slice(body);
            out.extend_from_slice(b"\x1b[201~");
            out
        } else {
            body.to_vec()
        }
    }


    pub(in crate::ui::view) fn close_multi_line_paste(&mut self, cx: &mut Context<Self>) {
        self.multi_line_paste = None;
        self.terminal_status = "multi-line paste cancelled".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn direct_multi_line_paste(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.multi_line_paste.take() else {
            self.terminal_status = "no multi-line paste is active".to_string();
            cx.notify();
            return;
        };
        let text = draft.normalized_text();
        let byte_count = text.len();
        self.send_terminal_input(self.wrap_terminal_paste_bytes(&text), cx);
        self.terminal_status = format!("direct pasted {byte_count} byte(s)");
        cx.notify();
    }

    pub(in crate::ui::view) fn send_multi_line_paste_by_line(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.multi_line_paste.take() else {
            self.terminal_status = "no multi-line paste is active".to_string();
            cx.notify();
            return;
        };
        let text = draft.normalized_text();
        let mut bytes = Vec::new();
        let mut line_count = 0usize;
        for line in text.split('\n') {
            line_count += 1;
            bytes.extend_from_slice(line.as_bytes());
            bytes.push(b'\n');
        }
        // Line-by-line send intentionally skips bracketed paste framing.
        self.send_terminal_input(bytes, cx);
        self.terminal_status = format!("sent {line_count} pasted line(s)");
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_multi_line_paste_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        let primary = keystroke.modifiers.control || keystroke.modifiers.platform;
        if primary && !keystroke.modifiers.alt && !keystroke.modifiers.function {
            match keystroke.key.as_str() {
                "enter" => {
                    self.direct_multi_line_paste(cx);
                    return;
                }
                "l" | "L" => {
                    self.send_multi_line_paste_by_line(cx);
                    return;
                }
                _ => {}
            }
        }
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }
        let Some(draft) = self.multi_line_paste.as_mut() else {
            return;
        };
        match keystroke.key.as_str() {
            "escape" => self.close_multi_line_paste(cx),
            "backspace" => {
                draft.text.pop();
                cx.notify();
            }
            "enter" => {
                draft.text.push('\n');
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    draft.text.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::ui::view) fn schedule_startup_command(
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
                this.send_terminal_input_to_session(session_id, command.into_bytes(), cx);
                this.terminal_status = "startup command sent".to_string();
                cx.notify();
            });
        })
        .detach();
    }

    pub(in crate::ui::view) fn close_active_session(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.active_session_id.clone() else {
            self.terminal_status = "no active session".to_string();
            cx.notify();
            return;
        };
        self.close_session(session_id, cx);
    }

    pub(in crate::ui::view) fn close_session(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let was_active = self.active_session_id.as_deref() == Some(session_id.as_str());
        match self.session_manager.close(&session_id) {
            Ok(()) => {
                self.recording_manager.cleanup_session(&session_id);
                self.remove_session_state(&session_id);
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
                        self.terminal_screen = initial_terminal_screen();
                        self.terminal_status = "session closed".to_string();
                    }
                } else {
                    self.terminal_status = format!("closed {}", short_id(&session_id));
                }
            }
            Err(error) => {
                self.terminal_status = format!("close failed: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn close_session_batch(
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
                self.terminal_screen = initial_terminal_screen();
            }
        }

        self.terminal_status = if failed == 0 {
            format!("closed {closed} {label} session(s)")
        } else {
            format!("closed {closed} {label} session(s), {failed} failed")
        };
    }

    pub(in crate::ui::view) fn open_close_all_sessions_confirm(
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
        self.close_all_sessions_confirm_open = true;
        self.terminal_status = "close all sessions confirmation opened".to_string();
        window.focus(&self.close_all_sessions_confirm_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn cancel_close_all_sessions_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.close_all_sessions_confirm_open = false;
        self.terminal_status = "close all sessions cancelled".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn confirm_close_all_sessions(&mut self, cx: &mut Context<Self>) {
        self.close_all_sessions_confirm_open = false;
        self.close_all_sessions(cx);
    }

    pub(in crate::ui::view) fn close_all_sessions(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn close_inactive_sessions(
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

    pub(in crate::ui::view) fn close_sessions_to_right(
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

    pub(in crate::ui::view) fn clear_terminal(&mut self, cx: &mut Context<Self>) {
        if let Some(session_id) = self.active_session_id.as_deref()
            && let Some(view) = self.terminal_views.get_mut(session_id)
        {
            view.clear();
        }
        self.terminal_output.clear();
        self.terminal_screen.clear();
        self.terminal_status = "terminal cleared".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn append_terminal_log(&mut self, text: impl AsRef<str>) {
        let session_id = self.active_session_id.clone();
        self.append_terminal_log_for_session(session_id.as_deref(), text.as_ref(), false);
    }


    pub(in crate::ui::view) fn active_terminal_scroll_offset(&self) -> usize {
        if let Some(session_id) = self.active_session_id.as_deref() {
            self.terminal_views
                .get(session_id)
                .map(|view| view.scroll_offset)
                .unwrap_or(0)
        } else {
            self.terminal_scroll_offset
        }
    }

    pub(in crate::ui::view) fn scroll_terminal_by(
        &mut self,
        delta_lines: i32,
        cx: &mut Context<Self>,
    ) {
        if delta_lines == 0 {
            return;
        }
        if let Some(session_id) = self.active_session_id.clone() {
            if let Some(view) = self.terminal_views.get_mut(&session_id) {
                let max = view.screen.scrollback_len();
                let next = if delta_lines > 0 {
                    view.scroll_offset.saturating_add(delta_lines as usize)
                } else {
                    view.scroll_offset.saturating_sub((-delta_lines) as usize)
                };
                view.scroll_offset = next.min(max);
            }
        } else {
            let max = self.terminal_screen.scrollback_len();
            let next = if delta_lines > 0 {
                self.terminal_scroll_offset.saturating_add(delta_lines as usize)
            } else {
                self.terminal_scroll_offset.saturating_sub((-delta_lines) as usize)
            };
            self.terminal_scroll_offset = next.min(max);
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn scroll_terminal_to_bottom(&mut self, cx: &mut Context<Self>) {
        if let Some(session_id) = self.active_session_id.clone() {
            if let Some(view) = self.terminal_views.get_mut(&session_id) {
                view.scroll_offset = 0;
            }
        } else {
            self.terminal_scroll_offset = 0;
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn scroll_terminal_to_top(&mut self, cx: &mut Context<Self>) {
        if let Some(session_id) = self.active_session_id.clone() {
            if let Some(view) = self.terminal_views.get_mut(&session_id) {
                view.scroll_offset = view.screen.scrollback_len();
            }
        } else {
            self.terminal_scroll_offset = self.terminal_screen.scrollback_len();
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn set_terminal_scroll_offset(
        &mut self,
        offset: usize,
        cx: &mut Context<Self>,
    ) {
        if let Some(session_id) = self.active_session_id.clone() {
            if let Some(view) = self.terminal_views.get_mut(&session_id) {
                let max = view.screen.scrollback_len();
                view.scroll_offset = offset.min(max);
            }
        } else {
            let max = self.terminal_screen.scrollback_len();
            self.terminal_scroll_offset = offset.min(max);
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn active_terminal_scroll_max(&self) -> usize {
        if let Some(session_id) = self.active_session_id.as_deref() {
            self.terminal_views
                .get(session_id)
                .map(|view| view.screen.scrollback_len())
                .unwrap_or(0)
        } else {
            self.terminal_screen.scrollback_len()
        }
    }

    /// Map a vertical pointer position (0..=1 top→bottom of track) to scroll_offset.
    /// Top of track = oldest history (max offset); bottom = live (0).
    pub(in crate::ui::view) fn set_terminal_scroll_from_track_ratio(
        &mut self,
        ratio: f32,
        cx: &mut Context<Self>,
    ) {
        let max = self.active_terminal_scroll_max();
        if max == 0 {
            self.set_terminal_scroll_offset(0, cx);
            return;
        }
        let ratio = ratio.clamp(0.0, 1.0);
        // ratio 0 (top) -> max, ratio 1 (bottom) -> 0
        let offset = ((1.0 - ratio) * max as f32).round() as usize;
        self.set_terminal_scroll_offset(offset.min(max), cx);
    }

    pub(in crate::ui::view) fn begin_terminal_scrollbar_drag(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.terminal_scrollbar_dragging = true;
        cx.notify();
    }

    pub(in crate::ui::view) fn update_terminal_scrollbar_drag(
        &mut self,
        event: &gpui::MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        if !self.terminal_scrollbar_dragging {
            return;
        }
        let Some(bounds) = self.terminal_surface_bounds else {
            return;
        };
        let height = f32::from(bounds.size.height).max(1.0);
        let local_y = f32::from(event.position.y - bounds.origin.y);
        let ratio = (local_y / height).clamp(0.0, 1.0);
        self.set_terminal_scroll_from_track_ratio(ratio, cx);
    }

    pub(in crate::ui::view) fn finish_terminal_scrollbar_drag(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.terminal_scrollbar_dragging {
            self.terminal_scrollbar_dragging = false;
            cx.notify();
        }
    }


    pub(in crate::ui::view) fn active_terminal_page_rows(&self) -> usize {
        // Prefer live screen rows when available; fall back to classic 24-row page.
        if let Some(session_id) = self.active_session_id.as_deref() {
            if let Some(view) = self.terminal_views.get(session_id) {
                let rows = view.screen.viewport_snapshot(0).lines.len();
                if rows > 0 {
                    return rows;
                }
            }
        }
        let rows = self.terminal_screen.viewport_snapshot(0).lines.len();
        if rows > 0 { rows } else { 24 }
    }

    /// Shift+PageUp/PageDown/Home/End (and Ctrl+Shift+Up/Down) navigate local scrollback
    /// without sending CSI sequences to the remote PTY — common terminal emulator UX.
    pub(in crate::ui::view) fn handle_terminal_scroll_key(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let keystroke = &event.keystroke;
        let key = keystroke.key.as_str();
        let shift = keystroke.modifiers.shift;
        let control = keystroke.modifiers.control;
        let alt = keystroke.modifiers.alt;
        let platform = keystroke.modifiers.platform;
        let function = keystroke.modifiers.function;
        if alt || platform || function {
            return false;
        }

        let page = self.active_terminal_page_rows().max(1) as i32;
        if shift && !control {
            match key {
                "pageup" => {
                    self.scroll_terminal_by(page, cx);
                    return true;
                }
                "pagedown" => {
                    self.scroll_terminal_by(-page, cx);
                    return true;
                }
                "home" => {
                    self.scroll_terminal_to_top(cx);
                    return true;
                }
                "end" => {
                    self.scroll_terminal_to_bottom(cx);
                    return true;
                }
                _ => {}
            }
        }
        if shift && control {
            match key {
                "up" => {
                    self.scroll_terminal_by(1, cx);
                    return true;
                }
                "down" => {
                    self.scroll_terminal_by(-1, cx);
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    pub(in crate::ui::view) fn sync_terminal_scrollback_limits(&mut self) {
        let limit = self.settings.terminal_scrollback_lines.clamp(100, 100_000) as usize;
        self.terminal_screen.set_scrollback_limit(limit);
        for view in self.terminal_views.values_mut() {
            view.screen.set_scrollback_limit(limit);
            view.clamp_scroll_offset();
        }
        if self.terminal_scroll_offset > self.terminal_screen.scrollback_len() {
            self.terminal_scroll_offset = self.terminal_screen.scrollback_len();
        }
    }

    pub(in crate::ui::view) fn terminal_scrollback_max_bytes(&self) -> usize {
        (self.settings.terminal_scrollback_lines.clamp(100, 100_000) as usize).saturating_mul(96)
    }

    pub(in crate::ui::view) fn enforce_terminal_scrollback_limit(&mut self) {
        self.sync_terminal_scrollback_limits();
        let max_bytes = self.terminal_scrollback_max_bytes();
        trim_terminal_output_to(&mut self.terminal_output, max_bytes);
        for view in self.terminal_views.values_mut() {
            trim_terminal_output_to(&mut view.output, max_bytes);
        }
    }

    pub(in crate::ui::view) fn append_terminal_bytes(&mut self, data: &[u8]) {
        let session_id = self.active_session_id.clone();
        self.append_terminal_bytes_for_session(session_id.as_deref(), data, false);
    }

    pub(in crate::ui::view) fn append_terminal_log_for_session(
        &mut self,
        session_id: Option<&str>,
        text: &str,
        mark_unread: bool,
    ) {
        if let Some(session_id) = session_id {
            let is_active = self.active_session_id.as_deref() == Some(session_id);
            let view = self
                .terminal_views
                .entry(session_id.to_string())
                .or_insert_with(TerminalViewState::new);
            view.append_text(text);
            if mark_unread && !is_active {
                view.has_unread = true;
            }
            if is_active {
                self.terminal_output.push_str(text);
                self.terminal_screen.advance(text.as_bytes());
                let max_bytes = self.terminal_scrollback_max_bytes();
                trim_terminal_output_to(&mut self.terminal_output, max_bytes);
            }
        } else {
            self.terminal_output.push_str(text);
            self.terminal_screen.advance(text.as_bytes());
            let max_bytes = self.terminal_scrollback_max_bytes();
            trim_terminal_output_to(&mut self.terminal_output, max_bytes);
        }
    }

    pub(in crate::ui::view) fn append_terminal_bytes_for_session(
        &mut self,
        session_id: Option<&str>,
        data: &[u8],
        mark_unread: bool,
    ) {
        if let Some(session_id) = session_id {
            let is_active = self.active_session_id.as_deref() == Some(session_id);
            let view = self
                .terminal_views
                .entry(session_id.to_string())
                .or_insert_with(TerminalViewState::new);
            view.append_bytes(data);
            if mark_unread && !is_active {
                view.has_unread = true;
            }
            if is_active {
                self.terminal_screen.advance(data);
                self.terminal_output
                    .push_str(&String::from_utf8_lossy(data));
                let max_bytes = self.terminal_scrollback_max_bytes();
                trim_terminal_output_to(&mut self.terminal_output, max_bytes);
            }
        } else {
            self.terminal_screen.advance(data);
            self.terminal_output
                .push_str(&String::from_utf8_lossy(data));
            let max_bytes = self.terminal_scrollback_max_bytes();
            trim_terminal_output_to(&mut self.terminal_output, max_bytes);
        }
    }
}
