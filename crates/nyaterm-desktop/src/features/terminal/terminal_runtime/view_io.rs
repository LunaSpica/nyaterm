use super::*;

impl NyaTermApp {
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
        self.active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.screen.lines().join("\n"))
            .unwrap_or_else(|| self.terminal_screen.lines().join("\n"))
    }

    pub(in crate::features) fn active_terminal_buffer_text(&self) -> String {
        self.active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.output.clone())
            .unwrap_or_else(|| self.terminal_output.clone())
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

    pub(in crate::features) fn send_terminal_clear_screen(&mut self, cx: &mut Context<Self>) {
        self.terminal_actions_open = false;
        self.send_terminal_input(vec![0x0c], cx);
        self.terminal_status = "clear screen command sent".to_string();
        cx.notify();
    }

    pub(in crate::features) fn send_terminal_input(
        &mut self,
        bytes: Vec<u8>,
        cx: &mut Context<Self>,
    ) {
        self.send_terminal_input_with_options(bytes, true, cx);
    }

    pub(in crate::features) fn send_terminal_input_without_suggestion_track(
        &mut self,
        bytes: Vec<u8>,
        cx: &mut Context<Self>,
    ) {
        self.send_terminal_input_with_options(bytes, false, cx);
    }

    pub(in crate::features) fn send_terminal_input_with_options(
        &mut self,
        bytes: Vec<u8>,
        track_suggestions: bool,
        cx: &mut Context<Self>,
    ) {
        if bytes.is_empty() {
            return;
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
            return;
        };
        if self.is_session_disconnected(&session_id) {
            // Key handler owns Enter-to-reconnect (needs Window). Block writes here.
            self.terminal_status = "session disconnected — press Enter to reconnect".to_string();
            cx.notify();
            return;
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

    /// When the active session's screen has mouse reporting enabled, encode and
    /// send a mouse report instead of performing local selection/scroll.
    /// Returns true if a report was sent (caller should skip local handling).

    pub(in crate::features) fn maybe_send_mouse_report(
        &mut self,
        button: u8,
        col: u16,
        row: u16,
        press: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(session_id) = self.active_session_id.clone() else {
            return false;
        };
        if self.is_session_disconnected(&session_id) {
            return false;
        }
        let reporting = self
            .terminal_views
            .get(&session_id)
            .map(|view| view.screen.mouse_reporting())
            .unwrap_or_else(|| self.terminal_screen.mouse_reporting());
        if !reporting {
            return false;
        }
        let screen = self
            .terminal_views
            .get(&session_id)
            .map(|view| &view.screen)
            .unwrap_or(&self.terminal_screen);
        let bytes = nyaterm_terminal::encode_mouse_report(screen, button, col, row, press);
        if bytes.is_empty() {
            return false;
        }
        self.send_terminal_input_to_session(session_id, bytes, cx);
        true
    }

    pub(in crate::features) fn send_terminal_input_to_session(
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

    pub(in crate::features) fn terminal_key_bytes_for_event(
        &self,
        event: &KeyDownEvent,
    ) -> Option<Vec<u8>> {
        // Prefer structured CSI for modified arrows (Ctrl/Alt) from terminal_key_bytes.
        if let Some(bytes) = terminal_key_bytes(event) {
            return Some(bytes);
        }
        // Alt-as-meta: ESC + character for shell word ops (Alt+b/f/d, etc.).
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
}
