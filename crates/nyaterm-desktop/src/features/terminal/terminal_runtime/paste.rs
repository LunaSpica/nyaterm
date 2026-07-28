use gpui::{Context, KeyDownEvent, Window};
use nyaterm_core::terminal_input_fanout_status;

use crate::features::NyaTermApp;
use crate::models::{MultiLinePasteDraft, is_multi_line_paste, normalize_paste_newlines};

impl NyaTermApp {
    pub(in crate::features) fn paste_from_clipboard(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            self.terminal.view.status = "clipboard does not contain text".to_string();
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
            self.terminal.view.status = "clipboard text is empty".to_string();
            cx.notify();
            return;
        }
        if self.settings.terminal_show_multi_line_paste_dialog && is_multi_line_paste(&text) {
            let text = normalize_paste_newlines(&text);
            self.multi_line_paste_cursor = text.len();
            self.multi_line_paste_anchor = None;
            self.multi_line_paste_marked_range = None;
            self.multi_line_paste = Some(MultiLinePasteDraft::new(text));
            self.multi_line_paste_marked_text.clear();
            self.terminal.view.status = "multi-line paste confirmation opened".to_string();
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
        self.terminal
            .view
            .views
            .get(session_id)
            .map(|view| view.protocol_state.bracketed_paste)
            .unwrap_or(false)
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
            if self.set_terminal_status_if_changed("no active session for paste") {
                cx.notify();
            }
            return;
        };
        if self.is_session_disconnected(&session_id) {
            if self
                .set_terminal_status_if_changed("session disconnected — press Enter to reconnect")
            {
                cx.notify();
            }
            return;
        }
        if self.active_terminal_visual_scroll_active() {
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
                if self.set_terminal_status_if_changed(format!("paste failed: {error}")) {
                    cx.notify();
                }
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

        if self.set_terminal_status_if_changed(terminal_input_fanout_status(
            "pasted", byte_count, synced, failed,
        )) {
            cx.notify();
        }
    }

    pub(in crate::features) fn close_multi_line_paste(&mut self, cx: &mut Context<Self>) {
        self.multi_line_paste = None;
        self.multi_line_paste_marked_text.clear();
        self.multi_line_paste_marked_range = None;
        self.multi_line_paste_cursor = 0;
        self.multi_line_paste_anchor = None;
        self.terminal.view.status = "multi-line paste cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn direct_multi_line_paste(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.multi_line_paste.take() else {
            self.terminal.view.status = "no multi-line paste is active".to_string();
            cx.notify();
            return;
        };
        self.multi_line_paste_marked_text.clear();
        self.multi_line_paste_marked_range = None;
        self.multi_line_paste_cursor = 0;
        self.multi_line_paste_anchor = None;
        let text = draft.normalized_text();
        self.send_terminal_paste_input(&text, cx);
    }

    pub(in crate::features) fn send_multi_line_paste_by_line(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.multi_line_paste.take() else {
            self.terminal.view.status = "no multi-line paste is active".to_string();
            cx.notify();
            return;
        };
        self.multi_line_paste_marked_text.clear();
        self.multi_line_paste_marked_range = None;
        self.multi_line_paste_cursor = 0;
        self.multi_line_paste_anchor = None;
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
                "a" | "A" => {
                    self.multi_line_paste_anchor = Some(0);
                    self.multi_line_paste_cursor = self
                        .multi_line_paste
                        .as_ref()
                        .map(|draft| draft.text.len())
                        .unwrap_or_default();
                    self.multi_line_paste_marked_range = None;
                    self.multi_line_paste_marked_text.clear();
                    cx.notify();
                    return;
                }
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
        match keystroke.key.as_str() {
            "escape" => self.close_multi_line_paste(cx),
            "backspace" => {
                let range = self.multi_line_paste_selected_byte_range();
                let range = if range.is_empty() {
                    let start = previous_char_boundary(
                        self.multi_line_paste_text(),
                        self.multi_line_paste_cursor,
                    );
                    start..self.multi_line_paste_cursor
                } else {
                    range
                };
                self.replace_multi_line_paste_range(range, "", cx);
            }
            "enter" => {
                self.replace_multi_line_paste_selection("\n", cx);
            }
            "delete" => {
                let range = self.multi_line_paste_selected_byte_range();
                let range = if range.is_empty() {
                    let end = next_char_boundary(
                        self.multi_line_paste_text(),
                        self.multi_line_paste_cursor,
                    );
                    self.multi_line_paste_cursor..end
                } else {
                    range
                };
                self.replace_multi_line_paste_range(range, "", cx);
            }
            "left" => {
                if !keystroke.modifiers.shift {
                    if let Some(anchor) = self.multi_line_paste_anchor {
                        let target = anchor.min(self.multi_line_paste_cursor);
                        self.move_multi_line_paste_cursor(target, false, cx);
                        return;
                    }
                }
                self.move_multi_line_paste_cursor(
                    previous_char_boundary(
                        self.multi_line_paste_text(),
                        self.multi_line_paste_cursor,
                    ),
                    keystroke.modifiers.shift,
                    cx,
                );
            }
            "right" => {
                if !keystroke.modifiers.shift {
                    if let Some(anchor) = self.multi_line_paste_anchor {
                        let target = anchor.max(self.multi_line_paste_cursor);
                        self.move_multi_line_paste_cursor(target, false, cx);
                        return;
                    }
                }
                self.move_multi_line_paste_cursor(
                    next_char_boundary(self.multi_line_paste_text(), self.multi_line_paste_cursor),
                    keystroke.modifiers.shift,
                    cx,
                );
            }
            "home" => {
                self.move_multi_line_paste_cursor(
                    line_start(self.multi_line_paste_text(), self.multi_line_paste_cursor),
                    keystroke.modifiers.shift,
                    cx,
                );
            }
            "end" => {
                self.move_multi_line_paste_cursor(
                    line_end(self.multi_line_paste_text(), self.multi_line_paste_cursor),
                    keystroke.modifiers.shift,
                    cx,
                );
            }
            "up" => {
                self.move_multi_line_paste_vertical(-1, keystroke.modifiers.shift, cx);
            }
            "down" => {
                self.move_multi_line_paste_vertical(1, keystroke.modifiers.shift, cx);
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.replace_multi_line_paste_selection(input, cx);
                }
            }
        }
    }

    pub(in crate::features) fn multi_line_paste_text(&self) -> &str {
        self.multi_line_paste
            .as_ref()
            .map(|draft| draft.text.as_str())
            .unwrap_or_default()
    }

    pub(in crate::features) fn multi_line_paste_selected_byte_range(
        &self,
    ) -> std::ops::Range<usize> {
        let cursor = self
            .multi_line_paste_cursor
            .min(self.multi_line_paste_text().len());
        let anchor = self.multi_line_paste_anchor.unwrap_or(cursor);
        if anchor <= cursor {
            anchor..cursor
        } else {
            cursor..anchor
        }
    }

    pub(in crate::features) fn move_multi_line_paste_cursor(
        &mut self,
        cursor: usize,
        extend: bool,
        cx: &mut Context<Self>,
    ) {
        let cursor = cursor.min(self.multi_line_paste_text().len());
        if extend {
            self.multi_line_paste_anchor
                .get_or_insert(self.multi_line_paste_cursor);
        } else {
            self.multi_line_paste_anchor = None;
        }
        self.multi_line_paste_cursor = cursor;
        self.multi_line_paste_marked_range = None;
        self.multi_line_paste_marked_text.clear();
        cx.notify();
    }

    pub(in crate::features) fn move_multi_line_paste_vertical(
        &mut self,
        delta: isize,
        extend: bool,
        cx: &mut Context<Self>,
    ) {
        let text = self.multi_line_paste_text();
        let cursor = self.multi_line_paste_cursor.min(text.len());
        let current_start = line_start(text, cursor);
        let column = text[current_start..cursor].chars().count();
        let target_start = if delta < 0 {
            if current_start == 0 {
                0
            } else {
                line_start(text, current_start - 1)
            }
        } else {
            let current_end = line_end(text, cursor);
            if current_end >= text.len() {
                current_start
            } else {
                current_end + 1
            }
        };
        let target_end = line_end(text, target_start);
        let target = text[target_start..target_end]
            .char_indices()
            .nth(column)
            .map(|(offset, _)| target_start + offset)
            .unwrap_or(target_end);
        self.move_multi_line_paste_cursor(target, extend, cx);
    }

    pub(in crate::features) fn replace_multi_line_paste_selection(
        &mut self,
        text: &str,
        cx: &mut Context<Self>,
    ) {
        let range = self.multi_line_paste_selected_byte_range();
        self.replace_multi_line_paste_range(range, text, cx);
    }

    pub(in crate::features) fn replace_multi_line_paste_range(
        &mut self,
        range: std::ops::Range<usize>,
        text: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(draft) = self.multi_line_paste.as_mut() else {
            return;
        };
        let start = range.start.min(draft.text.len());
        let end = range.end.min(draft.text.len()).max(start);
        draft.text.replace_range(start..end, text);
        self.multi_line_paste_cursor = start + text.len();
        self.multi_line_paste_anchor = None;
        self.multi_line_paste_marked_range = None;
        self.multi_line_paste_marked_text.clear();
        cx.notify();
    }
}

fn previous_char_boundary(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())]
        .char_indices()
        .next_back()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn next_char_boundary(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    text[offset..]
        .chars()
        .next()
        .map(|ch| offset + ch.len_utf8())
        .unwrap_or(offset)
}

fn line_start(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0)
}

fn line_end(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    text[offset..]
        .find('\n')
        .map(|index| offset + index)
        .unwrap_or(text.len())
}

#[cfg(test)]
mod tests {
    use crate::features::NyaTermApp;

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
