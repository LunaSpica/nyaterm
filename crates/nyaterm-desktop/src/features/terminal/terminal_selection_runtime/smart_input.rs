use super::*;

impl NyaTermApp {
    pub(in crate::features) fn handle_smart_input_click(
        &mut self,
        event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if event.modifiers.control
            || event.modifiers.platform
            || event.modifiers.alt
            || event.modifiers.shift
        {
            return;
        }
        if !self.can_use_smart_cursor_selection() {
            return;
        }
        let Some(target) = self.input_index_at_mouse(event.position) else {
            return;
        };
        let _ = self.move_smart_input_cursor(target, cx);
    }

    pub(in crate::features) fn input_index_at_mouse(
        &self,
        position: Point<Pixels>,
    ) -> Option<usize> {
        if !self.can_use_smart_cursor_selection() {
            return None;
        }
        let state = &self.command_input_tracker;
        if state.value.is_empty() {
            return None;
        }
        let cell = self.point_to_terminal_cell(position)?;
        let offset = self.active_terminal_scroll_offset();
        if offset != 0 {
            return None;
        }
        let snapshot = self
            .active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.screen.viewport_snapshot(0))
            .unwrap_or_else(|| self.terminal_screen.viewport_snapshot(0));
        if cell.row != snapshot.cursor_row {
            return None;
        }
        let line = snapshot
            .lines
            .get(cell.row)
            .map(String::as_str)
            .unwrap_or("");
        let line_chars: Vec<char> = line.chars().collect();
        let value_chars: Vec<char> = state.value.chars().collect();
        if value_chars.is_empty() || value_chars.len() > line_chars.len() {
            return None;
        }
        let mut input_start_col: Option<usize> = None;
        if snapshot.cursor_col >= value_chars.len() {
            let candidate = snapshot.cursor_col - value_chars.len();
            if line_chars
                .get(candidate..snapshot.cursor_col)
                .map(|slice| slice == value_chars.as_slice())
                .unwrap_or(false)
            {
                input_start_col = Some(candidate);
            }
        }
        if input_start_col.is_none() {
            let max_start = line_chars.len().saturating_sub(value_chars.len());
            for start_col in (0..=max_start).rev() {
                if line_chars[start_col..start_col + value_chars.len()] == value_chars[..] {
                    input_start_col = Some(start_col);
                    break;
                }
            }
        }
        let input_start_col = input_start_col?;
        let input_end_col = input_start_col + value_chars.len();
        if cell.col < input_start_col {
            return None;
        }
        let char_index = if cell.col >= input_end_col {
            value_chars.len()
        } else {
            cell.col - input_start_col
        };
        Some(
            state
                .value
                .char_indices()
                .nth(char_index)
                .map(|(i, _)| i)
                .unwrap_or(state.value.len()),
        )
    }

    pub(in crate::features) fn move_smart_input_cursor(
        &mut self,
        target_cursor: usize,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.can_use_smart_cursor_selection() {
            return false;
        }
        let mut state = self.command_input_tracker.clone();
        let next_cursor = target_cursor.min(state.value.len());
        let payload = build_move_input_cursor_data(&state.value, state.cursor, next_cursor);
        if payload.is_empty() && next_cursor == state.cursor {
            return false;
        }
        state.cursor = next_cursor;
        self.command_input_tracker = state;
        if !payload.is_empty() {
            self.send_terminal_input_without_suggestion_track(payload.into_bytes(), cx);
        } else {
            cx.notify();
        }
        true
    }

    pub(in crate::features) fn can_use_smart_cursor_selection(&self) -> bool {
        if self.active_session_id.is_none() {
            return false;
        }
        if self.is_credential_prompt_input_mode() {
            return false;
        }
        let state = &self.command_input_tracker;
        if state.desynced || state.line_rewrite_required || state.paste_mode || state.multiline {
            return false;
        }
        if let Some(session_id) = self.active_session_id.as_deref() {
            if !self.sync_peer_session_ids(session_id).is_empty() {
                return false;
            }
        }
        true
    }

    /// Map the painted terminal selection onto the tracked input line when it is fully contained.

    pub(in crate::features) fn smart_cursor_selected_input_range(
        &self,
    ) -> Option<InputSelectionRange> {
        if !self.can_use_smart_cursor_selection() {
            return None;
        }
        let state = &self.command_input_tracker;
        if state.value.is_empty() {
            return None;
        }
        let selection = self.terminal_selection.as_ref()?;
        if selection.is_empty() {
            return None;
        }
        // Only single-row selections can map to a single-line tracked input.
        let (start, end) = selection.ordered();
        if start.row != end.row {
            return None;
        }

        let offset = self.active_terminal_scroll_offset();
        if offset != 0 {
            // Selection in history is not the live input line.
            return None;
        }
        let snapshot = self
            .active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.screen.viewport_snapshot(0))
            .unwrap_or_else(|| self.terminal_screen.viewport_snapshot(0));

        if start.row != snapshot.cursor_row {
            return None;
        }

        let line = snapshot
            .lines
            .get(start.row)
            .map(String::as_str)
            .unwrap_or("");
        let line_chars: Vec<char> = line.chars().collect();
        let (col_start, col_end_excl) = selection.cols_for_row(start.row)?;
        let col_end = col_end_excl.min(line_chars.len().max(col_start));
        let col_start = col_start.min(col_end);
        if col_end <= col_start {
            return None;
        }

        // Find tracked value as a suffix of the cursor line (prompt + input).
        let value = &state.value;
        if value.is_empty() {
            return None;
        }
        let value_chars: Vec<char> = value.chars().collect();
        if value_chars.len() > line_chars.len() {
            return None;
        }
        // Prefer alignment ending at cursor_col (input ends at cursor when typing at end).
        // Fall back to last occurrence of value as a contiguous span on the line.
        let mut input_start_col: Option<usize> = None;
        if snapshot.cursor_col >= value_chars.len() {
            let candidate = snapshot.cursor_col - value_chars.len();
            if line_chars
                .get(candidate..snapshot.cursor_col)
                .map(|slice| slice == value_chars.as_slice())
                .unwrap_or(false)
            {
                input_start_col = Some(candidate);
            }
        }
        if input_start_col.is_none() {
            // Scan for last match of value on the line.
            let max_start = line_chars.len().saturating_sub(value_chars.len());
            for start_col in (0..=max_start).rev() {
                if line_chars[start_col..start_col + value_chars.len()] == value_chars[..] {
                    input_start_col = Some(start_col);
                    break;
                }
            }
        }
        let input_start_col = input_start_col?;
        let input_end_col = input_start_col + value_chars.len();

        if col_start < input_start_col || col_end > input_end_col {
            return None;
        }

        let sel_start_char = col_start - input_start_col;
        let sel_end_char = col_end - input_start_col;
        if sel_end_char <= sel_start_char || sel_end_char > value_chars.len() {
            return None;
        }

        let byte_start = value
            .char_indices()
            .nth(sel_start_char)
            .map(|(i, _)| i)
            .unwrap_or(0);
        let byte_end = value
            .char_indices()
            .nth(sel_end_char)
            .map(|(i, _)| i)
            .unwrap_or(value.len());
        InputSelectionRange::new(byte_start, byte_end)
    }

    pub(in crate::features) fn send_smart_selection_payload(
        &mut self,
        next_state: TerminalInputState,
        payload: String,
        cx: &mut Context<Self>,
    ) {
        self.command_input_tracker = next_state;
        self.clear_terminal_selection(cx);
        // Don't double-track via note_command_suggestion_input; state is already updated.
        self.send_terminal_input_without_suggestion_track(payload.into_bytes(), cx);
        if can_suggest_from_tracker(&self.command_input_tracker) {
            self.schedule_command_suggestion_refresh(cx);
        } else if self.command_suggestions.take().is_some() {
            cx.notify();
        }
    }

    pub(in crate::features) fn delete_smart_input_selection(
        &mut self,
        selected: InputSelectionRange,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.can_use_smart_cursor_selection() {
            return false;
        }
        let state = self.command_input_tracker.clone();
        let move_to_end = build_move_input_cursor_data(&state.value, state.cursor, selected.end);
        let delete_count = selected.len_chars(&state.value);
        if delete_count == 0 {
            return false;
        }
        let delete_bytes = "\u{007f}".repeat(delete_count);
        let next = delete_terminal_input_range(&state, selected.start, selected.end);
        let payload = format!("{move_to_end}{delete_bytes}");
        self.send_smart_selection_payload(next, payload, cx);
        true
    }

    pub(in crate::features) fn replace_smart_input_selection(
        &mut self,
        selected: InputSelectionRange,
        data: &str,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.can_use_smart_cursor_selection() || data.is_empty() {
            return false;
        }
        let state = self.command_input_tracker.clone();
        let move_to_end = build_move_input_cursor_data(&state.value, state.cursor, selected.end);
        let delete_count = selected.len_chars(&state.value);
        let delete_bytes = "\u{007f}".repeat(delete_count);
        let after_delete = delete_terminal_input_range(&state, selected.start, selected.end);
        let next = apply_terminal_input_data(&after_delete, data);
        let payload = format!("{move_to_end}{delete_bytes}{data}");
        self.send_smart_selection_payload(next, payload, cx);
        true
    }

    pub(in crate::features) fn collapse_smart_input_selection(
        &mut self,
        selected: InputSelectionRange,
        edge: SmartSelectionEdge,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.can_use_smart_cursor_selection() {
            return false;
        }
        let state = self.command_input_tracker.clone();
        let target = match edge {
            SmartSelectionEdge::Start => selected.start,
            SmartSelectionEdge::End => selected.end,
        };
        let payload = build_move_input_cursor_data(&state.value, state.cursor, target);
        let mut next = state;
        next.cursor = target.min(next.value.len());
        self.command_input_tracker = next;
        self.clear_terminal_selection(cx);
        if !payload.is_empty() {
            self.send_terminal_input_without_suggestion_track(payload.into_bytes(), cx);
        } else {
            cx.notify();
        }
        true
    }

    /// Handle Backspace/Delete/arrows/plain text against a smart input selection.
    /// Returns true when the key was consumed.

    pub(in crate::features) fn handle_smart_input_selection_key(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(selected) = self.smart_cursor_selected_input_range() else {
            return false;
        };
        let keystroke = &event.keystroke;
        if keystroke.modifiers.control
            || keystroke.modifiers.platform
            || keystroke.modifiers.alt
            || keystroke.modifiers.function
        {
            return false;
        }

        match keystroke.key.as_str() {
            "left" if !keystroke.modifiers.shift => {
                return self.collapse_smart_input_selection(
                    selected,
                    SmartSelectionEdge::Start,
                    cx,
                );
            }
            "right" if !keystroke.modifiers.shift => {
                return self.collapse_smart_input_selection(selected, SmartSelectionEdge::End, cx);
            }
            "backspace" | "delete" => {
                return self.delete_smart_input_selection(selected, cx);
            }
            _ => {}
        }

        if let Some(ch) = keystroke.key_char.as_deref() {
            if ch.chars().count() == 1 && !ch.chars().any(|c| c.is_control()) {
                return self.replace_smart_input_selection(selected, ch, cx);
            }
        }
        false
    }
}
