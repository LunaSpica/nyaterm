use super::*;

impl NyaTermApp {
    pub(in crate::features) fn clear_terminal_selection(&mut self, cx: &mut Context<Self>) {
        if self.terminal_selection.is_some() || self.terminal_selection_dragging {
            self.terminal_selection = None;
            self.terminal_selection_dragging = false;
            cx.notify();
        }
    }

    pub(in crate::features) fn select_all_terminal_visible(&mut self, cx: &mut Context<Self>) {
        let (rows, cols) = self.active_terminal_grid_size();
        if rows == 0 || cols == 0 {
            self.terminal_selection = None;
            cx.notify();
            return;
        }
        self.terminal_selection = Some(TerminalSelection {
            anchor: TerminalCellPos::new(0, 0),
            head: TerminalCellPos::new(rows.saturating_sub(1), cols.saturating_sub(1)),
        });
        self.terminal_selection_dragging = false;
        self.terminal_status = "selected all visible terminal text".to_string();
        cx.notify();
    }

    pub(in crate::features) fn selected_terminal_text(&self) -> Option<String> {
        let selection = self.terminal_selection.as_ref()?;
        if selection.is_empty() {
            return None;
        }
        let offset = self.active_terminal_scroll_offset();
        let snapshot = self
            .active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.screen.viewport_snapshot(offset))
            .unwrap_or_else(|| self.terminal_screen.viewport_snapshot(offset));
        let (start, end) = selection.ordered();
        let mut parts = Vec::new();
        for row in start.row..=end.row {
            let line = snapshot.lines.get(row).map(String::as_str).unwrap_or("");
            let chars: Vec<char> = line.chars().collect();
            let (col_start, col_end_excl) = selection.cols_for_row(row)?;
            let col_end = col_end_excl.min(chars.len().max(col_start));
            let col_start = col_start.min(col_end);
            let slice: String = chars
                .get(col_start..col_end)
                .map(|slice| slice.iter().collect())
                .unwrap_or_default();
            parts.push(slice.trim_end().to_string());
        }
        let text = parts.join("\n");
        if text.trim().is_empty() {
            None
        } else {
            Some(text)
        }
    }

    pub(in crate::features) fn copy_terminal_selection(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(text) = self.selected_terminal_text() else {
            return false;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.terminal_status = "copied terminal selection".to_string();
        self.terminal_actions_open = false;
        cx.notify();
        true
    }

    pub(in crate::features) fn copy_terminal_selection_or_visible(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.copy_terminal_selection(cx) {
            return;
        }
        self.copy_terminal_visible_text(cx);
    }

    pub(in crate::features) fn start_terminal_selection(
        &mut self,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        let Some(cell) = self.point_to_terminal_cell(event.position) else {
            return;
        };
        // Applications with mouse tracking (vim/less/tmux) consume left presses.
        if self.maybe_send_mouse_report(0, cell.col as u16, cell.row as u16, true, cx) {
            self.clear_terminal_selection(cx);
            return;
        }
        let (rows, cols) = self.active_terminal_grid_size();
        // Shift+click extends the existing selection from its anchor (xterm-style).
        if event.modifiers.shift && event.click_count <= 1 {
            if let Some(selection) = self.terminal_selection.as_mut() {
                selection.head = cell;
                self.terminal_selection_dragging = true;
                self.terminal_status = "selection extended".to_string();
                cx.notify();
                return;
            }
        }
        if event.click_count >= 3 {
            self.terminal_selection = Some(TerminalSelection {
                anchor: TerminalCellPos::new(cell.row, 0),
                head: TerminalCellPos::new(cell.row, cols.saturating_sub(1).max(0)),
            });
            self.terminal_selection_dragging = false;
            self.terminal_status = format!("selected line {}", cell.row + 1);
            cx.notify();
            return;
        }
        if event.click_count == 2 {
            let word = self.word_bounds_at(cell);
            self.terminal_selection = Some(TerminalSelection {
                anchor: TerminalCellPos::new(cell.row, word.0),
                head: TerminalCellPos::new(cell.row, word.1.saturating_sub(1).max(word.0)),
            });
            self.terminal_selection_dragging = false;
            self.terminal_status = "selected word".to_string();
            cx.notify();
            return;
        }
        self.terminal_selection = Some(TerminalSelection::new(cell));
        self.terminal_selection_dragging = true;
        let _ = rows;
        cx.notify();
    }

    pub(in crate::features) fn update_terminal_selection_drag(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        if !self.terminal_selection_dragging {
            return;
        }
        let Some(cell) = self.point_to_terminal_cell(event.position) else {
            return;
        };
        if let Some(selection) = self.terminal_selection.as_mut() {
            if selection.head != cell {
                selection.head = cell;
                cx.notify();
            }
        }
    }

    pub(in crate::features) fn finish_terminal_selection(
        &mut self,
        event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        if let Some(cell) = self.point_to_terminal_cell(event.position) {
            // Mouse tracking release (button 0 / SGR m).
            if self.maybe_send_mouse_report(0, cell.col as u16, cell.row as u16, false, cx) {
                self.clear_terminal_selection(cx);
                return;
            }
        }
        if !self.terminal_selection_dragging {
            // Stationary click without an active drag can still reposition the
            // tracked input cursor (Tauri handleTerminalMouseUp smart cursor).
            if self.terminal_selection.is_none() {
                self.handle_smart_input_click(event, cx);
            }
            return;
        }
        if let Some(cell) = self.point_to_terminal_cell(event.position) {
            if let Some(selection) = self.terminal_selection.as_mut() {
                selection.head = cell;
            }
        }
        self.terminal_selection_dragging = false;
        if self
            .terminal_selection
            .as_ref()
            .is_some_and(|selection| selection.is_empty())
        {
            self.terminal_selection = None;
            // Empty selection after click: try smart input cursor move.
            self.handle_smart_input_click(event, cx);
        } else if let Some(selected) = self.smart_cursor_selected_input_range() {
            // Collapse caret toward click/edge, then clear selection (Tauri path).
            let target = if event.click_count >= 2 {
                selected.end
            } else if let Some(index) = self.input_index_at_mouse(event.position) {
                index.clamp(selected.start, selected.end)
            } else {
                selected.end
            };
            if self.settings.interaction_copy_on_select {
                let _ = self.copy_terminal_selection(cx);
            }
            let _ = self.move_smart_input_cursor(target, cx);
            self.clear_terminal_selection(cx);
        } else if self.settings.interaction_copy_on_select {
            let _ = self.copy_terminal_selection(cx);
        }
        cx.notify();
    }

    pub(in crate::features) fn word_bounds_at(&self, cell: TerminalCellPos) -> (usize, usize) {
        let offset = self.active_terminal_scroll_offset();
        let snapshot = self
            .active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.screen.viewport_snapshot(offset))
            .unwrap_or_else(|| self.terminal_screen.viewport_snapshot(offset));
        let line = snapshot
            .lines
            .get(cell.row)
            .map(String::as_str)
            .unwrap_or("");
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            return (cell.col, cell.col.saturating_add(1));
        }
        let idx = cell.col.min(chars.len().saturating_sub(1));
        // xterm wordSeparator semantics: characters listed are separators, not word body.
        let separators = self.settings.interaction_word_separators.as_str();
        let is_word = |ch: char| !separators.contains(ch);
        if !is_word(chars[idx]) {
            return (idx, idx.saturating_add(1));
        }
        let mut start = idx;
        while start > 0 && is_word(chars[start - 1]) {
            start -= 1;
        }
        let mut end = idx + 1;
        while end < chars.len() && is_word(chars[end]) {
            end += 1;
        }
        (start, end)
    }

}
