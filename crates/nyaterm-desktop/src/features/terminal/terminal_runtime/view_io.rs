use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MouseReportWriteResult {
    NotHandled,
    Sent,
    Failed,
}

impl NyaTermApp {
    fn terminal_protocol_state_for_session(&self, session_id: &str) -> TerminalProtocolState {
        self.terminal_views
            .get(session_id)
            .map(|view| view.protocol_state)
            .unwrap_or_else(|| TerminalProtocolState::from_screen(&self.terminal_screen))
    }

    pub(in crate::features) fn open_terminal_actions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal_actions_open = true;
        self.terminal_status = "terminal actions opened".to_string();
        window.focus(&self.terminal_actions_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_terminal_actions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal_actions_open = false;
        self.terminal_status = "terminal actions closed".to_string();
        window.focus(&self.terminal_focus);
        cx.notify();
    }

    pub(in crate::features) fn active_terminal_visible_text(&self) -> String {
        self.active_terminal_snapshot().lines.join("\n")
    }

    pub(in crate::features) fn active_terminal_buffer_text(&self) -> String {
        self.active_session_id
            .as_deref()
            .map(|session_id| self.terminal_buffer_text_for_session(session_id))
            .unwrap_or_else(|| self.terminal_output.clone())
    }

    pub(in crate::features) fn terminal_buffer_text_for_session(&self, session_id: &str) -> String {
        self.terminal_views
            .get(session_id)
            .map(|view| view.output.clone())
            .unwrap_or_else(|| self.terminal_output.clone())
    }

    pub(in crate::features) fn active_terminal_buffer_tail(&self) -> &str {
        self.active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.output.as_str())
            .unwrap_or(self.terminal_output.as_str())
    }

    pub(in crate::features) fn terminal_buffer_tail_for_session(&self, session_id: &str) -> &str {
        self.terminal_views
            .get(session_id)
            .map(|view| view.output.as_str())
            .unwrap_or(self.terminal_output.as_str())
    }

    pub(in crate::features) fn active_terminal_view(&self) -> Option<&TerminalViewState> {
        self.active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
    }

    pub(in crate::features) fn active_terminal_view_mut(
        &mut self,
    ) -> Option<&mut TerminalViewState> {
        let session_id = self.active_session_id.clone()?;
        self.terminal_views.get_mut(&session_id)
    }

    pub(in crate::features) fn terminal_view_for(
        &self,
        session_id: &str,
    ) -> Option<&TerminalViewState> {
        self.terminal_views.get(session_id)
    }

    pub(in crate::features) fn terminal_snapshot_for_session(
        &self,
        session_id: Option<&str>,
        offset: usize,
    ) -> TerminalSnapshot {
        if let Some(session_id) = session_id.filter(|id| !id.is_empty()) {
            if let Some(view) = self.terminal_views.get(session_id) {
                if offset == 0 {
                    return view
                        .frame_snapshot
                        .clone()
                        .unwrap_or_else(|| view.screen.viewport_snapshot(0));
                }
                return view
                    .scrollback_snapshots
                    .get(&offset)
                    .cloned()
                    .unwrap_or_else(|| view.screen.viewport_snapshot(offset));
            }
        }
        self.terminal_screen.viewport_snapshot(offset)
    }

    pub(in crate::features) fn active_terminal_snapshot(&self) -> TerminalSnapshot {
        self.terminal_snapshot_for_session(
            self.active_session_id.as_deref(),
            self.active_terminal_scroll_offset(),
        )
    }

    pub(in crate::features) fn copy_terminal_visible_text(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn copy_terminal_buffer_text(&mut self, cx: &mut Context<Self>) {
        self.terminal_actions_open = false;
        let Some(session_id) = self.active_session_id.clone() else {
            let text = self.terminal_output.clone();
            if text.trim().is_empty() {
                self.terminal_status = "terminal buffer is empty".to_string();
            } else {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                self.terminal_status = "copied terminal buffer".to_string();
            }
            cx.notify();
            return;
        };
        let request_id = uuid();
        self.terminal_frame_pipeline.request_buffer_text(
            session_id,
            self.terminal_scrollback_max_bytes(),
            request_id,
        );
        self.terminal_status = "preparing terminal buffer copy".to_string();
        cx.notify();
    }

    pub(in crate::features) fn send_terminal_clear_screen(&mut self, cx: &mut Context<Self>) {
        self.terminal_actions_open = false;
        if self.send_terminal_input(vec![0x0c], cx) {
            self.terminal_status = "clear screen command sent".to_string();
            cx.notify();
        }
    }

    pub(in crate::features) fn send_terminal_input(
        &mut self,
        bytes: Vec<u8>,
        cx: &mut Context<Self>,
    ) -> bool {
        self.send_terminal_input_with_options(bytes, true, cx)
    }

    pub(in crate::features) fn send_terminal_input_without_suggestion_track(
        &mut self,
        bytes: Vec<u8>,
        cx: &mut Context<Self>,
    ) -> bool {
        self.send_terminal_input_with_options(bytes, false, cx)
    }

    pub(in crate::features) fn send_terminal_input_with_options(
        &mut self,
        bytes: Vec<u8>,
        track_suggestions: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        if bytes.is_empty() {
            return false;
        }
        // Tauri/xterm custom key path: non-smart buffer selections stay painted while
        // typing. Smart input selections are handled earlier and clear themselves.
        // Only drop an in-progress drag so a stuck drag cannot block further input.
        if self.terminal_selection_dragging {
            self.terminal_selection_dragging = false;
        }
        let Some(session_id) = self.active_session_id.clone() else {
            self.terminal_status = "start a session before typing".to_string();
            cx.notify();
            return false;
        };
        if self.is_session_disconnected(&session_id) {
            // Key handler owns Enter-to-reconnect (needs Window). Block writes here.
            self.terminal_status = "session disconnected — press Enter to reconnect".to_string();
            cx.notify();
            return false;
        }
        // Typing while scrolled in history returns to the live bottom (xterm-like).
        if self.active_terminal_scroll_offset() > 0 {
            self.scroll_terminal_to_bottom(cx);
        }
        let peers = self.sync_peer_session_ids(&session_id);
        let byte_count = bytes.len();
        if track_suggestions {
            self.note_command_suggestion_input(&bytes, cx);
        }

        debug_assert!(
            terminal_wire_write_disposition(TerminalWireWriteKind::LogicalInput)
                .allow_command_history
        );
        // Primary + sync peers share write/record/history so recording and per-session
        // command history stay consistent. Resolve history once after all writes so a
        // pending Enter submission is applied to every successful peer.
        let mut ok_sessions = Vec::new();
        match self.write_session_input_recorded(&session_id, &bytes) {
            Ok(()) => ok_sessions.push(session_id),
            Err(error) => {
                self.terminal_status = format!("input failed: {error}");
                cx.notify();
                return false;
            }
        }

        let mut synced = 0usize;
        let mut failed = 0usize;
        for peer_id in peers {
            match self.write_session_input_recorded(&peer_id, &bytes) {
                Ok(()) => {
                    ok_sessions.push(peer_id);
                    synced += 1;
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }

        let session_refs: Vec<&str> = ok_sessions.iter().map(String::as_str).collect();
        self.record_command_history_for_sessions(&session_refs, &bytes);

        self.terminal_status = terminal_input_fanout_status("sent", byte_count, synced, failed);
        cx.notify();
        failed == 0
    }

    pub(in crate::features) fn send_terminal_key_event(
        &mut self,
        event: &KeyDownEvent,
        track_suggestions: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        // Key protocol modes are session-local (application cursor/keypad,
        // Kitty keyboard). Re-encode for each sync peer instead of broadcasting
        // the active session's wire bytes.
        if self.terminal_selection_dragging {
            self.terminal_selection_dragging = false;
        }
        let Some(session_id) = self.active_session_id.clone() else {
            self.terminal_status = "start a session before typing".to_string();
            cx.notify();
            return false;
        };
        if self.is_session_disconnected(&session_id) {
            self.terminal_status = "session disconnected — press Enter to reconnect".to_string();
            cx.notify();
            return false;
        }
        let Some(primary_bytes) =
            self.terminal_key_bytes_for_event_for_session(Some(&session_id), event)
        else {
            return false;
        };
        if primary_bytes.is_empty() {
            return false;
        }
        if self.active_terminal_scroll_offset() > 0 {
            self.scroll_terminal_to_bottom(cx);
        }
        if track_suggestions {
            self.note_command_suggestion_input(&primary_bytes, cx);
        }

        debug_assert!(
            terminal_wire_write_disposition(TerminalWireWriteKind::LogicalInput)
                .allow_command_history
        );
        let byte_count = primary_bytes.len();
        let peers = self.sync_peer_session_ids(&session_id);
        let mut ok_sessions = Vec::new();
        match self.write_session_input_recorded(&session_id, &primary_bytes) {
            Ok(()) => ok_sessions.push(session_id),
            Err(error) => {
                self.terminal_status = format!("input failed: {error}");
                cx.notify();
                return false;
            }
        }

        let mut synced = 0usize;
        let mut failed = 0usize;
        for peer_id in peers {
            let Some(peer_bytes) =
                self.terminal_key_bytes_for_event_for_session(Some(&peer_id), event)
            else {
                continue;
            };
            if peer_bytes.is_empty() {
                continue;
            }
            match self.write_session_input_recorded(&peer_id, &peer_bytes) {
                Ok(()) => {
                    ok_sessions.push(peer_id);
                    synced += 1;
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }

        let session_refs: Vec<&str> = ok_sessions.iter().map(String::as_str).collect();
        self.record_command_history_for_sessions(&session_refs, &primary_bytes);
        self.terminal_status = terminal_input_fanout_status("sent", byte_count, synced, failed);
        cx.notify();
        failed == 0
    }

    pub(in crate::features) fn send_terminal_key_release_event(
        &mut self,
        event: &KeyUpEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(session_id) = self.active_session_id.clone() else {
            return false;
        };
        if self.is_session_disconnected(&session_id) {
            return false;
        }
        let Some(primary_bytes) =
            self.terminal_key_release_bytes_for_event_for_session(Some(&session_id), event)
        else {
            return false;
        };
        if primary_bytes.is_empty() {
            return false;
        }

        let byte_count = primary_bytes.len();
        let peers = self.sync_peer_session_ids(&session_id);
        let mut synced = 0usize;
        let mut failed = 0usize;
        match self.write_session_input_recorded(&session_id, &primary_bytes) {
            Ok(()) => {}
            Err(error) => {
                self.terminal_status = format!("input failed: {error}");
                cx.notify();
                return false;
            }
        }
        for peer_id in peers {
            let Some(peer_bytes) =
                self.terminal_key_release_bytes_for_event_for_session(Some(&peer_id), event)
            else {
                continue;
            };
            if peer_bytes.is_empty() {
                continue;
            }
            match self.write_session_input_recorded(&peer_id, &peer_bytes) {
                Ok(()) => synced += 1,
                Err(_) => failed += 1,
            }
        }
        self.terminal_status = terminal_input_fanout_status("sent", byte_count, synced, failed);
        cx.notify();
        failed == 0
    }

    /// On the alternate screen with alternate-scroll enabled and no mouse
    /// tracking, emulate xterm: wheel becomes Up/Down cursor sequences.
    pub(in crate::features) fn maybe_send_alternate_scroll_for_session(
        &mut self,
        session_id: &str,
        delta_lines: i32,
        cx: &mut Context<Self>,
    ) -> bool {
        if delta_lines == 0 || session_id.is_empty() {
            return false;
        }
        if self.is_session_disconnected(session_id) {
            return false;
        }
        let Some(payload) = self.alternate_scroll_payload_for_session(session_id, delta_lines)
        else {
            return false;
        };
        if let Err(error) = self.write_session_input_recorded(session_id, &payload) {
            self.terminal_status = format!("alternate scroll failed: {error}");
            cx.notify();
            return true;
        }

        let peers = self.sync_peer_session_ids(session_id);
        let mut synced = 0usize;
        let mut failed = 0usize;
        for peer_id in peers {
            let Some(peer_payload) =
                self.alternate_scroll_payload_for_session(&peer_id, delta_lines)
            else {
                continue;
            };
            match self.write_session_input_recorded(&peer_id, &peer_payload) {
                Ok(()) => synced += 1,
                Err(_) => failed += 1,
            }
        }
        if failed > 0 {
            self.terminal_status =
                format!("alternate scroll synced {synced} peer(s), {failed} failed");
            cx.notify();
        }
        true
    }

    fn alternate_scroll_payload_for_session(
        &self,
        session_id: &str,
        delta_lines: i32,
    ) -> Option<Vec<u8>> {
        if delta_lines == 0 || session_id.is_empty() || self.is_session_disconnected(session_id) {
            return None;
        }
        self.terminal_protocol_state_for_session(session_id)
            .alternate_scroll_payload(delta_lines)
    }

    /// When the active session's screen has mouse reporting enabled, encode and
    /// send a mouse report instead of performing local selection/scroll.
    /// Returns true when the terminal app handled the event (caller should skip
    /// local handling). Protocol traffic is recorded but not command history.

    pub(in crate::features) fn maybe_send_mouse_report(
        &mut self,
        button: u8,
        col: u16,
        row: u16,
        press: bool,
        motion: bool,
        modifiers: gpui::Modifiers,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(session_id) = self.active_session_id.clone() else {
            return false;
        };
        self.maybe_send_mouse_report_for_session(
            &session_id,
            button,
            col,
            row,
            press,
            motion,
            modifiers,
            cx,
        )
    }

    pub(in crate::features) fn maybe_send_mouse_report_for_session(
        &mut self,
        session_id: &str,
        button: u8,
        col: u16,
        row: u16,
        press: bool,
        motion: bool,
        modifiers: gpui::Modifiers,
        cx: &mut Context<Self>,
    ) -> bool {
        let result = self.write_mouse_report_to_session(
            session_id, button, col, row, press, motion, modifiers, cx,
        );
        match result {
            MouseReportWriteResult::NotHandled => return false,
            MouseReportWriteResult::Failed => return true,
            MouseReportWriteResult::Sent => {}
        }

        self.terminal_mouse_report_position = Some((col, row));
        if motion {
            let peers = self.terminal_mouse_report_peer_session_ids.clone();
            for peer_id in peers {
                let _ = self.write_mouse_report_to_session(
                    &peer_id, button, col, row, press, true, modifiers, cx,
                );
            }
            return true;
        }
        if press && button < 3 {
            let peers = self.sync_peer_session_ids(session_id);
            let mut captured_peers = Vec::new();
            for peer_id in peers {
                if self.write_mouse_report_to_session(
                    &peer_id, button, col, row, true, false, modifiers, cx,
                ) == MouseReportWriteResult::Sent
                {
                    captured_peers.push(peer_id);
                }
            }
            self.terminal_mouse_report_button = Some(button);
            self.terminal_mouse_report_session_id = Some(session_id.to_string());
            self.terminal_mouse_report_peer_session_ids = captured_peers;
        } else if !press {
            let peers = std::mem::take(&mut self.terminal_mouse_report_peer_session_ids);
            for peer_id in peers {
                let _ = self.write_mouse_report_to_session(
                    &peer_id, button, col, row, false, false, modifiers, cx,
                );
            }
            self.clear_terminal_mouse_report_capture();
        }
        true
    }

    pub(in crate::features) fn maybe_send_terminal_any_motion_report(
        &mut self,
        event: &gpui::MouseMoveEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.terminal_mouse_report_button.is_some() {
            return false;
        }
        let Some(session_id) = self.terminal_session_at_point(event.position) else {
            return false;
        };
        let session_id = session_id
            .or_else(|| self.active_session_id.clone())
            .unwrap_or_default();
        if session_id.is_empty() {
            return false;
        }
        let protocol = self.terminal_protocol_state_for_session(&session_id);
        if !protocol.mouse_motion_reporting {
            return false;
        }
        let Some(cell) =
            self.point_to_terminal_cell_for_session(Some(session_id.as_str()), event.position)
        else {
            return false;
        };
        let col = cell.col as u16;
        let row = cell.row as u16;
        match self.write_mouse_report_to_session(
            &session_id,
            3,
            col,
            row,
            true,
            true,
            event.modifiers,
            cx,
        ) {
            MouseReportWriteResult::Sent => {
                for peer_id in self.sync_peer_session_ids(&session_id) {
                    let _ = self.write_mouse_report_to_session(
                        &peer_id,
                        3,
                        col,
                        row,
                        true,
                        true,
                        event.modifiers,
                        cx,
                    );
                }
                true
            }
            MouseReportWriteResult::Failed => true,
            MouseReportWriteResult::NotHandled => false,
        }
    }

    fn write_mouse_report_to_session(
        &mut self,
        session_id: &str,
        button: u8,
        col: u16,
        row: u16,
        press: bool,
        motion: bool,
        modifiers: gpui::Modifiers,
        cx: &mut Context<Self>,
    ) -> MouseReportWriteResult {
        if session_id.is_empty() {
            return MouseReportWriteResult::NotHandled;
        }
        let disconnected = self.is_session_disconnected(session_id);
        let protocol = self.terminal_protocol_state_for_session(session_id);
        if !protocol.mouse_reporting {
            return MouseReportWriteResult::NotHandled;
        }
        if !terminal_mouse_report_should_send(TerminalMouseReportEligibility {
            session_id_empty: false,
            disconnected,
            mouse_reporting: protocol.mouse_reporting,
            motion,
            mouse_drag_reporting: protocol.mouse_drag_reporting,
        }) {
            return MouseReportWriteResult::NotHandled;
        }
        let bytes = protocol.encode_mouse_report(
            button,
            col,
            row,
            press,
            motion,
            modifiers.shift,
            modifiers.alt,
            modifiers.control || modifiers.platform,
        );
        if bytes.is_empty() {
            return MouseReportWriteResult::NotHandled;
        }
        if let Err(error) = self.write_session_input_recorded(session_id, &bytes) {
            self.terminal_status = format!("mouse report failed: {error}");
            cx.notify();
            return MouseReportWriteResult::Failed;
        }
        MouseReportWriteResult::Sent
    }

    pub(in crate::features) fn finish_terminal_mouse_report(
        &mut self,
        event: &gpui::MouseUpEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(button) = self.terminal_mouse_report_button else {
            return false;
        };
        let Some(session_id) = self
            .terminal_mouse_report_session_id
            .clone()
            .or_else(|| self.active_session_id.clone())
        else {
            self.clear_terminal_mouse_report_capture();
            return false;
        };
        let (col, row) = if let Some(cell) =
            self.point_to_terminal_cell_for_session(Some(session_id.as_str()), event.position)
        {
            (cell.col as u16, cell.row as u16)
        } else if let Some((col, row)) = self.terminal_mouse_report_position {
            (col, row)
        } else {
            self.clear_terminal_mouse_report_capture();
            return false;
        };
        self.maybe_send_mouse_report_for_session(
            &session_id,
            button,
            col,
            row,
            false,
            false,
            event.modifiers,
            cx,
        )
    }

    pub(in crate::features) fn clear_terminal_mouse_report_for_session(
        &mut self,
        session_id: &str,
    ) {
        if self.terminal_mouse_report_session_id.as_deref() == Some(session_id)
            || self
                .terminal_mouse_report_peer_session_ids
                .iter()
                .any(|peer_id| peer_id == session_id)
        {
            self.clear_terminal_mouse_report_capture();
        }
    }

    fn clear_terminal_mouse_report_capture(&mut self) {
        self.terminal_mouse_report_button = None;
        self.terminal_mouse_report_session_id = None;
        self.terminal_mouse_report_peer_session_ids.clear();
        self.terminal_mouse_report_position = None;
    }

    /// Write UTF-8/ASCII input to a live session and mirror the logical input
    /// into the session recording buffer.
    /// Does not touch command history, status text, or UI notify.
    pub(in crate::features) fn write_session_input_recorded(
        &mut self,
        session_id: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        // Charset-encode paste/typed text; pure ASCII CSI/mouse reports pass through.
        let disposition = terminal_wire_write_disposition(TerminalWireWriteKind::LogicalInput);
        let encoded = if disposition.encode_session_charset {
            self.encode_session_outgoing(session_id, bytes)
        } else {
            bytes.to_vec()
        };
        self.session_manager
            .write(session_id, &encoded)
            .map_err(|error| error.to_string())?;
        if disposition.record_logical_input {
            self.recording_write_pipeline
                .write_input(session_id.to_string(), bytes.to_vec());
        }
        Ok(())
    }

    /// Write already-encoded/raw bytes to a live session and mirror the exact
    /// bytes into the session recording buffer.
    pub(in crate::features) fn write_session_raw_input_recorded(
        &mut self,
        session_id: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        let disposition = terminal_wire_write_disposition(TerminalWireWriteKind::RawInput);
        debug_assert!(!disposition.encode_session_charset);
        self.session_manager
            .write(session_id, bytes)
            .map_err(|error| error.to_string())?;
        if disposition.record_raw_input {
            self.recording_write_pipeline
                .write_raw_input(session_id.to_string(), bytes.to_vec());
        }
        Ok(())
    }

    /// Write bytes already prepared for the PTY while recording a separate
    /// logical input stream. Used when protocol framing (for example bracketed
    /// paste) should reach the session but not pollute input recordings.
    pub(in crate::features) fn write_session_wire_input_recorded_as(
        &mut self,
        session_id: &str,
        wire_bytes: &[u8],
        recording_bytes: &[u8],
    ) -> Result<(), String> {
        let disposition = terminal_wire_write_disposition(TerminalWireWriteKind::FramedInput);
        debug_assert!(!disposition.encode_session_charset);
        self.session_manager
            .write(session_id, wire_bytes)
            .map_err(|error| error.to_string())?;
        if disposition.record_logical_input {
            self.recording_write_pipeline
                .write_input(session_id.to_string(), recording_bytes.to_vec());
        }
        Ok(())
    }

    /// Write terminal-emulator protocol bytes (DSR/OSC/Kitty replies, focus
    /// reports) without marking them as user input in recordings.
    pub(in crate::features) fn write_session_protocol_response(
        &mut self,
        session_id: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        let disposition = terminal_wire_write_disposition(TerminalWireWriteKind::ProtocolResponse);
        debug_assert!(!disposition.encode_session_charset);
        debug_assert!(!disposition.record_logical_input);
        debug_assert!(!disposition.record_raw_input);
        debug_assert!(!disposition.allow_command_history);
        self.session_manager
            .write(session_id, bytes)
            .map_err(|error| error.to_string())
    }

    /// Encode UTF-8 host input for the session wire charset (UTF-8/GBK/…).
    pub(in crate::features) fn encode_session_outgoing(
        &self,
        session_id: &str,
        bytes: &[u8],
    ) -> Vec<u8> {
        if let Some(view) = self.terminal_views.get(session_id) {
            return view.screen.encode_outgoing(bytes);
        }
        self.terminal_screen.encode_outgoing(bytes)
    }

    /// Apply interaction default encoding to a terminal screen.
    pub(in crate::features) fn apply_terminal_encoding_to_screen(
        &self,
        screen: &mut nyaterm_terminal::TerminalScreen,
    ) {
        screen.set_encoding(&self.settings.interaction_default_encoding);
    }

    /// Keep all live terminal screens on the current interaction encoding.
    pub(in crate::features) fn sync_terminal_encodings_from_settings(&mut self) {
        let label = self.settings.interaction_default_encoding.clone();
        self.terminal_screen.set_encoding(&label);
        self.terminal_output_decoder.set_encoding(&label);
        for view in self.terminal_views.values_mut() {
            view.set_encoding(&label);
        }
        self.sync_session_event_bridge_config();
    }

    pub(in crate::features) fn ensure_terminal_focus_reporting(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.terminal_focus_subscriptions.is_empty() {
            return;
        }
        let focus_in = cx.on_focus_in(&self.terminal_focus, window, |this, _window, cx| {
            this.report_terminal_focus(true, cx);
        });
        let focus_out =
            cx.on_focus_out(&self.terminal_focus, window, |this, _event, _window, cx| {
                this.report_terminal_focus(false, cx);
            });
        self.terminal_focus_subscriptions = vec![focus_in, focus_out];
    }

    pub(in crate::features) fn report_terminal_focus(
        &mut self,
        focused: bool,
        cx: &mut Context<Self>,
    ) {
        self.terminal_focus_active = focused;
        let Some(session_id) = self.active_session_id.clone() else {
            return;
        };
        if self.write_terminal_focus_report_to_session(&session_id, focused) {
            cx.notify();
        }
    }

    /// Send a DECSET 1004 focus report to a specific session when that session
    /// has enabled focus reporting. Protocol traffic is not command history.
    pub(in crate::features) fn write_terminal_focus_report_to_session(
        &mut self,
        session_id: &str,
        focused: bool,
    ) -> bool {
        if self.is_session_disconnected(session_id) {
            return false;
        }
        if !self
            .terminal_protocol_state_for_session(session_id)
            .focus_reporting
        {
            return false;
        }
        let bytes = nyaterm_terminal::TerminalScreen::encode_focus_report(focused);
        self.write_session_protocol_response(session_id, &bytes)
            .is_ok()
    }

    pub(in crate::features) fn sync_terminal_cell_metrics_to_screens(&mut self) {
        let (width, height) = self.terminal_cell_size();
        let width = width.round().clamp(1.0, 512.0) as u16;
        let height = height.round().clamp(1.0, 512.0) as u16;
        self.terminal_screen.set_cell_metrics(width, height);
        for view in self.terminal_views.values_mut() {
            view.screen.set_cell_metrics(width, height);
        }
    }

    pub(in crate::features) fn send_terminal_input_to_session(
        &mut self,
        session_id: String,
        bytes: Vec<u8>,
        cx: &mut Context<Self>,
    ) -> bool {
        if bytes.is_empty() {
            return false;
        }
        if self.is_session_disconnected(&session_id) {
            self.terminal_status = "session disconnected — reconnect before sending".to_string();
            cx.notify();
            return false;
        }
        let sent = match self.write_session_input_recorded(&session_id, &bytes) {
            Ok(()) => {
                self.record_command_history_from_bytes(Some(&session_id), &bytes);
                self.terminal_status = format!("sent {} byte(s)", bytes.len());
                true
            }
            Err(error) => {
                self.terminal_status = format!("input failed: {error}");
                false
            }
        };
        cx.notify();
        sent
    }

    pub(in crate::features) fn send_terminal_raw_input_to_session(
        &mut self,
        session_id: String,
        bytes: Vec<u8>,
        cx: &mut Context<Self>,
    ) -> bool {
        if bytes.is_empty() {
            return false;
        }
        if self.is_session_disconnected(&session_id) {
            self.terminal_status = "session disconnected — reconnect before sending".to_string();
            cx.notify();
            return false;
        }
        let sent = match self.write_session_raw_input_recorded(&session_id, &bytes) {
            Ok(()) => {
                self.terminal_status = format!("sent {} byte(s)", bytes.len());
                true
            }
            Err(error) => {
                self.terminal_status = format!("input failed: {error}");
                false
            }
        };
        cx.notify();
        sent
    }

    pub(in crate::features) fn active_terminal_key_mode(&self) -> TerminalKeyMode {
        self.terminal_key_mode_for_session(self.active_session_id.as_deref())
    }

    pub(in crate::features) fn terminal_key_mode_for_session(
        &self,
        session_id: Option<&str>,
    ) -> TerminalKeyMode {
        let (
            application_cursor,
            application_keypad,
            kitty_keyboard_disambiguate,
            kitty_keyboard_report_event_types,
            kitty_keyboard_report_alternate_keys,
            kitty_keyboard_report_all_keys_as_esc,
            kitty_keyboard_report_associated_text,
        ) = if let Some(session_id) = session_id {
            self.terminal_views
                .get(session_id)
                .map(|view| {
                    let protocol = view.protocol_state;
                    (
                        protocol.application_cursor_keys,
                        protocol.application_keypad,
                        protocol.kitty_keyboard_disambiguate,
                        protocol.kitty_keyboard_report_event_types,
                        protocol.kitty_keyboard_report_alternate_keys,
                        protocol.kitty_keyboard_report_all_keys_as_esc,
                        protocol.kitty_keyboard_report_associated_text,
                    )
                })
                .unwrap_or((false, false, false, false, false, false, false))
        } else {
            (
                self.terminal_screen.application_cursor_keys(),
                self.terminal_screen.application_keypad(),
                self.terminal_screen.kitty_keyboard_disambiguate(),
                self.terminal_screen.kitty_keyboard_report_event_types(),
                self.terminal_screen.kitty_keyboard_report_alternate_keys(),
                self.terminal_screen.kitty_keyboard_report_all_keys_as_esc(),
                self.terminal_screen.kitty_keyboard_report_associated_text(),
            )
        };
        TerminalKeyMode {
            application_cursor,
            application_keypad,
            kitty_keyboard_disambiguate,
            kitty_keyboard_report_event_types,
            kitty_keyboard_report_alternate_keys,
            kitty_keyboard_report_all_keys_as_esc,
            kitty_keyboard_report_associated_text,
        }
    }

    pub(in crate::features) fn terminal_key_bytes_for_event(
        &self,
        event: &KeyDownEvent,
    ) -> Option<Vec<u8>> {
        self.terminal_key_bytes_for_event_for_session(self.active_session_id.as_deref(), event)
    }

    pub(in crate::features) fn terminal_key_bytes_for_event_for_session(
        &self,
        session_id: Option<&str>,
        event: &KeyDownEvent,
    ) -> Option<Vec<u8>> {
        terminal_key_bytes_for_mode_and_settings(
            event,
            self.terminal_key_mode_for_session(session_id),
            self.settings.interaction_alt_as_meta,
        )
    }

    pub(in crate::features) fn terminal_should_defer_key_text_to_input_handler(
        &self,
        event: &KeyDownEvent,
    ) -> bool {
        self.settings.interaction_mac_ime_compatibility
            && event
                .keystroke
                .modifiers
                .is_subset_of(&gpui::Modifiers::shift())
            && event
                .keystroke
                .key_char
                .as_deref()
                .is_some_and(|input| !input.is_empty() && input.chars().all(|ch| !ch.is_control()))
    }

    pub(in crate::features) fn terminal_key_release_bytes_for_event(
        &self,
        event: &KeyUpEvent,
    ) -> Option<Vec<u8>> {
        self.terminal_key_release_bytes_for_event_for_session(
            self.active_session_id.as_deref(),
            event,
        )
    }

    pub(in crate::features) fn terminal_key_release_bytes_for_event_for_session(
        &self,
        session_id: Option<&str>,
        event: &KeyUpEvent,
    ) -> Option<Vec<u8>> {
        terminal_key_release_bytes_with_mode(event, self.terminal_key_mode_for_session(session_id))
    }
}

fn terminal_key_bytes_for_mode_and_settings(
    event: &KeyDownEvent,
    mode: TerminalKeyMode,
    alt_as_meta: bool,
) -> Option<Vec<u8>> {
    // Prefer structured CSI for modified arrows (Ctrl/Alt) from terminal_key_bytes.
    if let Some(bytes) = terminal_key_bytes_with_mode(event, mode) {
        return Some(bytes);
    }
    // Alt-as-meta: ESC + character for shell word ops (Alt+b/f/d, etc.).
    if alt_as_meta
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
    // Even when alt-as-meta is off, still emit ESC+letter for Alt+b/f/d word ops
    // that shells commonly expect (Tauri XTerminal parity).
    if event.keystroke.modifiers.alt
        && !event.keystroke.modifiers.control
        && !event.keystroke.modifiers.platform
        && !event.keystroke.modifiers.function
        && !event.keystroke.modifiers.shift
    {
        let key = event.keystroke.key.as_str();
        if matches!(key, "b" | "B" | "f" | "F" | "d" | "D") {
            return Some(vec![0x1b, key.as_bytes()[0].to_ascii_lowercase()]);
        }
        if let Some(input) = event.keystroke.key_char.as_deref() {
            if input.len() == 1 {
                let ch = input.chars().next().unwrap();
                if matches!(ch, 'b' | 'B' | 'f' | 'F' | 'd' | 'D') {
                    return Some(vec![0x1b, ch.to_ascii_lowercase() as u8]);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_event(key: &str, key_char: Option<&str>, modifiers: gpui::Modifiers) -> KeyDownEvent {
        KeyDownEvent {
            keystroke: gpui::Keystroke {
                modifiers,
                key: key.to_string(),
                key_char: key_char.map(str::to_string),
            },
            is_held: false,
        }
    }

    #[test]
    fn terminal_key_encoding_uses_target_session_mode() {
        let event = key_event("up", None, gpui::Modifiers::default());
        let normal =
            terminal_key_bytes_for_mode_and_settings(&event, TerminalKeyMode::default(), false)
                .unwrap();
        let application = terminal_key_bytes_for_mode_and_settings(
            &event,
            TerminalKeyMode {
                application_cursor: true,
                ..TerminalKeyMode::default()
            },
            false,
        )
        .unwrap();

        assert_eq!(normal, b"\x1b[A".to_vec());
        assert_eq!(application, b"\x1bOA".to_vec());
    }

    #[test]
    fn terminal_key_encoding_keeps_alt_meta_setting_outside_mode() {
        let event = key_event(
            "x",
            Some("x"),
            gpui::Modifiers {
                alt: true,
                ..gpui::Modifiers::default()
            },
        );

        assert_eq!(
            terminal_key_bytes_for_mode_and_settings(&event, TerminalKeyMode::default(), true,)
                .unwrap(),
            b"\x1bx".to_vec()
        );
        assert!(
            terminal_key_bytes_for_mode_and_settings(&event, TerminalKeyMode::default(), false,)
                .is_none()
        );
    }
}
