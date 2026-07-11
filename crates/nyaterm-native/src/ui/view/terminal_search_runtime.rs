use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn open_terminal_search(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal_actions_open = false;
        self.terminal_search_open = true;
        self.terminal_search_active_index = 0;
        self.terminal_status = "terminal search opened".to_string();
        window.focus(&self.terminal_search_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn close_terminal_search(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal_search_open = false;
        self.terminal_search_active_index = 0;
        self.terminal_status = "terminal search closed".to_string();
        window.focus(&self.terminal_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn terminal_search_flags(&self) -> TerminalSearchFlags {
        TerminalSearchFlags {
            case_sensitive: self.terminal_search_case_sensitive,
            regex: self.terminal_search_regex,
            whole_word: self.terminal_search_whole_word,
        }
    }

    pub(in crate::ui::view) fn terminal_buffer_matches(
        &self,
    ) -> Result<Vec<TerminalBufferMatch>, String> {
        let query = self.terminal_search_query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let flags = self.terminal_search_flags();
        // Search absolute scrollback + live screen (Tauri xterm buffer find).
        let buffer_text = self
            .active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.screen.all_lines().join("
"))
            .unwrap_or_else(|| self.terminal_screen.all_lines().join("
"));
        terminal_buffer_matches(&buffer_text, query, &flags, 1000)
    }

    /// Ensure the absolute buffer line is visible by adjusting scroll_offset.
    pub(in crate::ui::view) fn reveal_terminal_absolute_line(
        &mut self,
        abs_line: usize,
        cx: &mut Context<Self>,
    ) {
        if let Some(session_id) = self.active_session_id.clone() {
            if let Some(view) = self.terminal_views.get_mut(&session_id) {
                let total = view.screen.total_rows().max(1);
                let rows = view.screen.viewport_snapshot(0).lines.len().max(1);
                let max_start = total.saturating_sub(rows);
                let start = abs_line.min(max_start);
                let offset = total.saturating_sub(start + rows);
                view.scroll_offset = offset.min(view.screen.scrollback_len());
            }
        } else {
            let total = self.terminal_screen.total_rows().max(1);
            let rows = self.terminal_screen.viewport_snapshot(0).lines.len().max(1);
            let max_start = total.saturating_sub(rows);
            let start = abs_line.min(max_start);
            let offset = total.saturating_sub(start + rows);
            self.terminal_scroll_offset = offset.min(self.terminal_screen.scrollback_len());
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn terminal_history_search_results(
        &self,
    ) -> Result<nyaterm_session::TerminalHistorySearchResponse, String> {
        let Some(session_id) = self.active_session_id.clone() else {
            return Ok(nyaterm_session::TerminalHistorySearchResponse {
                total: 0,
                elapsed_ms: 0,
                truncated: false,
                results: Vec::new(),
            });
        };
        let query = self.terminal_search_query.trim();
        if query.is_empty() {
            return Ok(nyaterm_session::TerminalHistorySearchResponse {
                total: 0,
                elapsed_ms: 0,
                truncated: false,
                results: Vec::new(),
            });
        }
        self.recording_manager
            .search_history(TerminalHistorySearchRequest {
                session_id,
                query: query.to_string(),
                case_sensitive: self.terminal_search_case_sensitive,
                regex: self.terminal_search_regex,
                whole_word: self.terminal_search_whole_word,
                limit: Some(8),
                context_before: Some(1),
                context_after: Some(1),
                max_lines: Some(30_000),
            })
            .map_err(|error| error.to_string())
    }

    pub(in crate::ui::view) fn navigate_terminal_search(
        &mut self,
        direction: isize,
        cx: &mut Context<Self>,
    ) {
        let count = match self.terminal_search_mode {
            TerminalSearchMode::Buffer => self
                .terminal_buffer_matches()
                .map(|matches| matches.len())
                .unwrap_or(0),
            TerminalSearchMode::History => self
                .terminal_history_search_results()
                .map(|response| response.results.len())
                .unwrap_or(0),
        };
        if count == 0 {
            self.terminal_search_active_index = 0;
            self.terminal_status = "terminal search has no matches".to_string();
            cx.notify();
            return;
        }
        self.terminal_search_active_index = (self.terminal_search_active_index as isize + direction)
            .rem_euclid(count as isize) as usize;
        if self.terminal_search_mode == TerminalSearchMode::Buffer {
            if let Ok(matches) = self.terminal_buffer_matches() {
                if let Some(m) = matches.get(self.terminal_search_active_index) {
                    self.reveal_terminal_absolute_line(m.line_index, cx);
                }
            }
        }
        self.terminal_status = format!(
            "terminal search match {}/{}",
            self.terminal_search_active_index + 1,
            count
        );
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_terminal_search_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "escape" => self.close_terminal_search(window, cx),
            "enter" => {
                if keystroke.modifiers.shift {
                    self.navigate_terminal_search(-1, cx);
                } else {
                    self.navigate_terminal_search(1, cx);
                }
            }
            "backspace" => {
                self.terminal_search_query.pop();
                self.terminal_search_active_index = 0;
                cx.notify();
            }
            "tab" => {
                self.terminal_search_mode =
                    if self.terminal_search_mode == TerminalSearchMode::Buffer {
                        TerminalSearchMode::History
                    } else {
                        TerminalSearchMode::Buffer
                    };
                self.terminal_search_active_index = 0;
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.terminal_search_query.push_str(input);
                    self.terminal_search_active_index = 0;
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::ui::view) fn handle_active_sessions_search_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "escape" => {
                self.active_sessions_search_draft.clear();
                self.terminal_status = "active sessions search cleared".to_string();
                cx.notify();
            }
            "backspace" => {
                self.active_sessions_search_draft.pop();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.active_sessions_search_draft.push_str(input);
                    cx.notify();
                }
            }
        }
    }
}
