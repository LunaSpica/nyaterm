use super::*;

impl NyaTermApp {
    pub(in crate::features) fn paste_from_clipboard(
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

    pub(in crate::features) fn paste_terminal_text(
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
        // Tauri pasteText: replace smart input selection when present.
        if let Some(selected) = self.smart_cursor_selected_input_range() {
            if self.replace_smart_input_selection(selected, &payload, cx) {
                return;
            }
        }
        self.send_terminal_paste_input(&payload, cx);
    }

    pub(in crate::features) fn session_bracketed_paste(&self, session_id: &str) -> bool {
        self.terminal_views
            .get(session_id)
            .map(|view| view.screen.bracketed_paste())
            .unwrap_or(false)
    }

    pub(in crate::features) fn active_terminal_bracketed_paste(&self) -> bool {
        if let Some(session_id) = self.active_session_id.as_deref() {
            self.session_bracketed_paste(session_id)
        } else {
            self.terminal_screen.bracketed_paste()
        }
    }

    pub(in crate::features) fn wrap_terminal_paste_bytes(&self, text: &str) -> Vec<u8> {
        self.wrap_terminal_paste_bytes_for_bracketed(text, self.active_terminal_bracketed_paste())
    }

    pub(in crate::features) fn wrap_terminal_paste_bytes_for_session(
        &self,
        session_id: &str,
        text: &str,
    ) -> Vec<u8> {
        let body = self.encode_session_outgoing(session_id, text.as_bytes());
        Self::wrap_terminal_paste_wire_bytes_for_bracketed(
            &body,
            self.session_bracketed_paste(session_id),
        )
    }

    pub(in crate::features) fn wrap_terminal_paste_bytes_for_bracketed(
        &self,
        text: &str,
        bracketed: bool,
    ) -> Vec<u8> {
        Self::wrap_terminal_paste_wire_bytes_for_bracketed(text.as_bytes(), bracketed)
    }

    pub(in crate::features) fn wrap_terminal_paste_wire_bytes_for_bracketed(
        body: &[u8],
        bracketed: bool,
    ) -> Vec<u8> {
        if bracketed {
            let mut out = Vec::with_capacity(body.len() + 12);
            out.extend_from_slice(b"\x1b[200~");
            out.extend_from_slice(body);
            out.extend_from_slice(b"\x1b[201~");
            out
        } else {
            body.to_vec()
        }
    }

    /// Paste fan-out wraps bracketed-paste mode per target session so sync peers
    /// with different DECBPM state receive correct framing.
    pub(in crate::features) fn send_terminal_paste_input(
        &mut self,
        text: &str,
        cx: &mut Context<Self>,
    ) {
        if text.is_empty() {
            return;
        }
        let Some(session_id) = self.active_session_id.clone() else {
            self.terminal_status = "no active session for paste".to_string();
            cx.notify();
            return;
        };
        if self.is_session_disconnected(&session_id) {
            self.terminal_status = "session disconnected — press Enter to reconnect".to_string();
            cx.notify();
            return;
        }
        if self.active_terminal_scroll_offset() > 0 {
            self.scroll_terminal_to_bottom(cx);
        }

        let peers = self.sync_peer_session_ids(&session_id);
        let mut ok_sessions = Vec::new();
        let recording_bytes = text.as_bytes();
        let primary_bytes = self.wrap_terminal_paste_bytes_for_session(&session_id, text);
        let byte_count = primary_bytes.len();
        match self.write_session_wire_input_recorded_as(
            &session_id,
            &primary_bytes,
            recording_bytes,
        ) {
            Ok(()) => ok_sessions.push(session_id),
            Err(error) => {
                self.terminal_status = format!("paste failed: {error}");
                cx.notify();
                return;
            }
        }

        let mut synced = 0usize;
        let mut failed = 0usize;
        for peer_id in peers {
            let peer_bytes = self.wrap_terminal_paste_bytes_for_session(&peer_id, text);
            match self.write_session_wire_input_recorded_as(&peer_id, &peer_bytes, recording_bytes)
            {
                Ok(()) => {
                    ok_sessions.push(peer_id);
                    synced += 1;
                }
                Err(_) => failed += 1,
            }
        }

        // History tracks the logical pasted text, not per-session framing bytes.
        let history_bytes = text.as_bytes();
        let session_refs: Vec<&str> = ok_sessions.iter().map(String::as_str).collect();
        self.record_command_history_for_sessions(&session_refs, history_bytes);

        self.terminal_status = terminal_input_fanout_status("pasted", byte_count, synced, failed);
        cx.notify();
    }

    pub(in crate::features) fn close_multi_line_paste(&mut self, cx: &mut Context<Self>) {
        self.multi_line_paste = None;
        self.terminal_status = "multi-line paste cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn direct_multi_line_paste(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.multi_line_paste.take() else {
            self.terminal_status = "no multi-line paste is active".to_string();
            cx.notify();
            return;
        };
        let text = draft.normalized_text();
        self.send_terminal_paste_input(&text, cx);
    }

    pub(in crate::features) fn send_multi_line_paste_by_line(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.multi_line_paste.take() else {
            self.terminal_status = "no multi-line paste is active".to_string();
            cx.notify();
            return;
        };
        let text = draft.normalized_text();
        let mut bytes = Vec::new();
        for line in text.split('\n') {
            bytes.extend_from_slice(line.as_bytes());
            bytes.push(b'\n');
        }
        // Line-by-line send intentionally skips bracketed paste framing.
        self.send_terminal_input(bytes, cx);
    }

    pub(in crate::features) fn handle_multi_line_paste_key_down(
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bracketed_paste_wraps_wire_bytes_without_reencoding_body() {
        let body = [0xb2, 0xe2, b'\n'];
        let wrapped = NyaTermApp::wrap_terminal_paste_wire_bytes_for_bracketed(&body, true);

        assert!(wrapped.starts_with(b"\x1b[200~"));
        assert!(wrapped.ends_with(b"\x1b[201~"));
        assert_eq!(
            &wrapped[b"\x1b[200~".len()..wrapped.len() - b"\x1b[201~".len()],
            &body
        );
    }

    #[test]
    fn plain_paste_wire_bytes_are_body_only() {
        let body = b"plain";
        assert_eq!(
            NyaTermApp::wrap_terminal_paste_wire_bytes_for_bracketed(body, false),
            body
        );
    }
}
