use gpui::{Context, KeyDownEvent, Window};

use crate::features::{NyaTermApp, TextInputSetup};
use crate::models::{
    RecordingHistorySearchKey, RecordingWriteEvent, TerminalFrameSearchKey, TerminalSearchMode,
    terminal_frame_search_result_is_current,
};
use crate::terminal::TerminalBufferMatch;

impl NyaTermApp {
    pub(in crate::features) fn open_terminal_search(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal.menus.actions_open = false;
        self.terminal.search.open = true;
        self.terminal.search.active_index = 0;
        self.terminal.view.status = "terminal search opened".to_string();
        self.forget_text_inputs("terminal.search.");
        let query = self.terminal.search.query.clone();
        let field = self.text_input(
            "terminal.search.query",
            &query,
            TextInputSetup::placeholder("Find"),
            cx,
        );
        self.refresh_terminal_search_state(cx);
        window.focus(&field.read(cx).focus_handle());
    }

    pub(in crate::features) fn close_terminal_search(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal.search.open = false;
        self.terminal.search.active_index = 0;
        self.forget_text_inputs("terminal.search.");
        self.terminal.view.status = "terminal search closed".to_string();
        window.focus(&self.terminal.input.focus);
        self.notify_active_terminal_surface(cx);
        cx.notify();
    }

    pub(in crate::features) fn refresh_terminal_search_state(&mut self, cx: &mut Context<Self>) {
        self.request_active_terminal_search();
        self.notify_active_terminal_surface(cx);
        cx.notify();
    }

    pub(in crate::features) fn apply_terminal_search_query(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        self.terminal.search.query = text;
        self.terminal.search.active_index = 0;
        self.refresh_terminal_search_state(cx);
    }

    pub(in crate::features) fn terminal_search_key(&self) -> Option<TerminalFrameSearchKey> {
        let query = self.terminal.search.query.trim();
        if query.is_empty() {
            return None;
        }
        Some(TerminalFrameSearchKey {
            query: query.to_string(),
            case_sensitive: self.terminal.search.case_sensitive,
            regex: self.terminal.search.regex,
            whole_word: self.terminal.search.whole_word,
            limit: 1000,
        })
    }

    pub(in crate::features) fn request_active_terminal_buffer_search(&mut self) -> bool {
        if !self.terminal.search.open || self.terminal.search.mode != TerminalSearchMode::Buffer {
            return false;
        }
        let Some(session_id) = self.active_session_id.clone() else {
            return false;
        };
        let Some(key) = self.terminal_search_key() else {
            return false;
        };
        self.request_terminal_frame_search(&session_id, key)
    }

    pub(in crate::features) fn request_active_terminal_search(&mut self) {
        let _ = self.request_active_terminal_buffer_search();
        self.request_active_terminal_history_search();
    }

    pub(in crate::features) fn terminal_buffer_matches(
        &self,
    ) -> Result<Vec<TerminalBufferMatch>, String> {
        let Some(key) = self.terminal_search_key() else {
            return Ok(Vec::new());
        };
        let Some(view) = self
            .active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal.view.views.get(session_id))
        else {
            return Ok(Vec::new());
        };
        view.search_result
            .as_ref()
            .filter(|result| {
                terminal_frame_search_result_is_current(result, &key, view.screen_revision)
            })
            .map(|result| result.matches.clone())
            .unwrap_or_else(|| Ok(Vec::new()))
    }

    /// Ensure the absolute buffer line is visible by adjusting scroll_offset.
    pub(in crate::features) fn reveal_terminal_absolute_line(
        &mut self,
        abs_line: usize,
        cx: &mut Context<Self>,
    ) {
        if let Some(session_id) = self.active_session_id.clone() {
            if let Some(view) = self.terminal.view.views.get_mut(&session_id) {
                let total = view.total_rows_for_ui();
                let rows = view.viewport_rows_for_ui();
                let max_start = total.saturating_sub(rows);
                let start = abs_line.min(max_start);
                let offset = total.saturating_sub(start + rows);
                view.scroll_offset = offset.min(view.scrollback_len_for_ui());
                self.clear_terminal_scroll_residual_for_session(Some(&session_id));
            }
            self.notify_terminal_scroll_after_state_change(Some(session_id.as_str()), cx);
        } else {
            let total = self.terminal.view.screen.total_rows().max(1);
            let rows = self
                .terminal_snapshot_for_session(None, 0)
                .row_count()
                .max(1);
            let max_start = total.saturating_sub(rows);
            let start = abs_line.min(max_start);
            let offset = total.saturating_sub(start + rows);
            self.terminal.view.scroll_offset =
                offset.min(self.terminal.view.screen.scrollback_len());
            self.clear_terminal_scroll_residual_for_session(None);
            cx.notify();
        }
    }

    pub(in crate::features) fn terminal_history_search_key(
        &self,
    ) -> Option<RecordingHistorySearchKey> {
        let session_id = self.active_session_id.clone()?;
        let query = self.terminal.search.query.trim();
        if query.is_empty() {
            return None;
        }
        Some(RecordingHistorySearchKey {
            session_id,
            query: query.to_string(),
            case_sensitive: self.terminal.search.case_sensitive,
            regex: self.terminal.search.regex,
            whole_word: self.terminal.search.whole_word,
            limit: Some(8),
            context_before: Some(1),
            context_after: Some(1),
            max_lines: Some(30_000),
        })
    }

    pub(in crate::features) fn request_active_terminal_history_search(&mut self) {
        if !self.terminal.search.open || self.terminal.search.mode != TerminalSearchMode::History {
            return;
        }
        let Some(key) = self.terminal_history_search_key() else {
            self.terminal.search.history_pending_key = None;
            self.terminal.search.history_result = None;
            return;
        };
        if self.terminal.search.history_pending_key.as_ref() == Some(&key)
            || self
                .terminal
                .search
                .history_result
                .as_ref()
                .is_some_and(|result| result.key == key)
        {
            return;
        }
        self.terminal.search.history_pending_key = Some(key.clone());
        self.recording_write_pipeline.request_history_search(key);
    }

    pub(in crate::features) fn drain_recording_pipeline_events(&mut self) -> bool {
        if self.terminal.search.history_pending_key.is_none() {
            return false;
        }
        let mut dirty = false;
        while let Some(event) = self.recording_write_pipeline.try_recv_event() {
            match event {
                RecordingWriteEvent::HistorySearch(event) => {
                    if self.terminal.search.history_pending_key.as_ref() == Some(&event.key) {
                        self.terminal.search.history_pending_key = None;
                        self.terminal.search.history_result = Some(event);
                        dirty = true;
                    }
                }
            }
        }
        dirty
    }

    pub(in crate::features) fn terminal_history_search_pending_for_current_query(&self) -> bool {
        let Some(key) = self.terminal_history_search_key() else {
            return false;
        };
        self.terminal.search.history_pending_key.as_ref() == Some(&key)
    }

    pub(in crate::features) fn terminal_history_search_results(
        &self,
    ) -> Result<nyaterm_transport::TerminalHistorySearchResponse, String> {
        let Some(key) = self.terminal_history_search_key() else {
            return Ok(empty_terminal_history_search_response());
        };
        if let Some(result) = self
            .terminal
            .search
            .history_result
            .as_ref()
            .filter(|result| result.key == key)
        {
            return result.result.clone();
        }
        Ok(empty_terminal_history_search_response())
    }

    pub(in crate::features) fn navigate_terminal_search(
        &mut self,
        direction: isize,
        cx: &mut Context<Self>,
    ) {
        let count = match self.terminal.search.mode {
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
            self.terminal.search.active_index = 0;
            self.terminal.view.status = "terminal search has no matches".to_string();
            self.notify_active_terminal_surface(cx);
            cx.notify();
            return;
        }
        self.terminal.search.active_index = (self.terminal.search.active_index as isize + direction)
            .rem_euclid(count as isize) as usize;
        if self.terminal.search.mode == TerminalSearchMode::Buffer {
            if let Ok(matches) = self.terminal_buffer_matches() {
                if let Some(m) = matches.get(self.terminal.search.active_index) {
                    self.reveal_terminal_absolute_line(m.line_index, cx);
                }
            }
        }
        self.terminal.view.status = format!(
            "terminal search match {}/{}",
            self.terminal.search.active_index + 1,
            count
        );
        self.notify_active_terminal_surface(cx);
        cx.notify();
    }

    pub(in crate::features) fn handle_terminal_search_key_down(
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
            "tab" => {
                self.terminal.search.mode =
                    if self.terminal.search.mode == TerminalSearchMode::Buffer {
                        TerminalSearchMode::History
                    } else {
                        TerminalSearchMode::Buffer
                    };
                self.terminal.search.active_index = 0;
                self.refresh_terminal_search_state(cx);
            }
            _ => {}
        }
    }

    /// Apply an edit from the active sessions filter box.
    pub(in crate::features) fn apply_active_sessions_search(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.active_sessions_search_draft = text;
        cx.notify();
    }
}

fn empty_terminal_history_search_response() -> nyaterm_transport::TerminalHistorySearchResponse {
    nyaterm_transport::TerminalHistorySearchResponse {
        total: 0,
        elapsed_ms: 0,
        truncated: false,
        results: Vec::new(),
    }
}
