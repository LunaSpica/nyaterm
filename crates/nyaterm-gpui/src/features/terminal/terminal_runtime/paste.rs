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
        self.send_terminal_input(self.wrap_terminal_paste_bytes(&payload), cx);
    }

    pub(in crate::features) fn active_terminal_bracketed_paste(&self) -> bool {
        if let Some(session_id) = self.active_session_id.as_deref() {
            self.terminal_views
                .get(session_id)
                .map(|view| view.screen.bracketed_paste())
                .unwrap_or(false)
        } else {
            self.terminal_screen.bracketed_paste()
        }
    }

    pub(in crate::features) fn wrap_terminal_paste_bytes(&self, text: &str) -> Vec<u8> {
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
        let byte_count = text.len();
        self.send_terminal_input(self.wrap_terminal_paste_bytes(&text), cx);
        self.terminal_status = format!("direct pasted {byte_count} byte(s)");
        cx.notify();
    }

    pub(in crate::features) fn send_multi_line_paste_by_line(&mut self, cx: &mut Context<Self>) {
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
