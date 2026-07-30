use std::time::Duration;

use gpui::{
    ClipboardItem, Context, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Timer,
};
use nyaterm_terminal::TerminalSnapshot;

use crate::features::NyaTermApp;
use crate::features::terminal::terminal_runtime::TerminalMouseReportRequest;
use crate::models::{TerminalCellPos, TerminalSelection};
use crate::terminal::{
    TerminalTextCell, terminal_is_zero_width_mark, terminal_text_cell_slice, terminal_text_cells,
};

use super::metrics::terminal_cell_for_visual_geometry;

const TERMINAL_SELECTION_DRAG_NOTIFY_DELAY: Duration = Duration::from_millis(8);

impl NyaTermApp {
    pub(in crate::features) fn clear_terminal_selection_state_for_session(
        &mut self,
        session_id: &str,
    ) {
        let selection_session_id = self
            .terminal
            .selection
            .session_id
            .as_deref()
            .or(self.session.active_id());
        if selection_session_id != Some(session_id) {
            return;
        }
        self.terminal.selection.selection = None;
        self.terminal.selection.session_id = None;
        self.terminal.selection.dragging = false;
    }

    fn notify_terminal_selection_owner_surface(&mut self, cx: &mut Context<Self>) {
        let session_id = self
            .terminal
            .selection
            .session_id
            .as_deref()
            .or(self.session.active_id())
            .map(str::to_string);
        self.notify_terminal_surface_only(session_id.as_deref(), cx);
    }

    pub(in crate::features) fn clear_terminal_selection(&mut self, cx: &mut Context<Self>) {
        let previous_session_id = self.terminal.selection.session_id.clone();
        if self.terminal.selection.selection.is_some() || self.terminal.selection.dragging {
            self.terminal.selection.selection = None;
            self.terminal.selection.session_id = None;
            self.terminal.selection.dragging = false;
            self.notify_terminal_surface_only(previous_session_id.as_deref(), cx);
        }
    }

    pub(in crate::features) fn select_all_terminal(&mut self, cx: &mut Context<Self>) {
        let (_, cols) = self.active_terminal_grid_size();
        if cols == 0 {
            self.terminal.selection.selection = None;
            self.terminal.selection.session_id = None;
            self.notify_terminal_selection_owner_surface(cx);
            return;
        }
        self.terminal.selection.selection = Some(TerminalSelection::all_buffer(cols));
        self.terminal.selection.session_id = self.session.active_id_owned();
        self.terminal.selection.dragging = false;
        self.shell
            .set_status("selected all terminal text".to_string());
        self.notify_terminal_selection_owner_surface(cx);
        cx.notify();
    }

    pub(in crate::features) fn selected_terminal_text(&self) -> Option<String> {
        let selection = self.terminal.selection.selection.as_ref()?;
        let session_id = self
            .terminal
            .selection
            .session_id
            .as_deref()
            .or(self.session.active_id());
        if selection.is_empty() {
            return None;
        }
        if selection.all_buffer {
            let lines = self
                .terminal
                .selection
                .session_id
                .as_deref()
                .or(self.session.active_id())
                .and_then(|session_id| self.terminal.view.views.get(session_id))
                .map(|view| view.screen.all_lines())
                .unwrap_or_else(|| self.terminal.view.screen.all_lines());
            return terminal_all_lines_text(lines);
        }
        let offset = selection.display_offset;
        let snapshot = self.terminal_snapshot_for_session(session_id, offset);
        let (start, end) = selection.ordered();
        let mut parts = Vec::new();
        for row in start.row..=end.row {
            let snapshot_row = selection
                .viewport_anchor_row
                .checked_add(row)
                .filter(|row| *row < snapshot.row_count());
            let line = snapshot_row
                .and_then(|row| snapshot.line(row))
                .unwrap_or("");
            let cells = terminal_text_cells(line);
            let (col_start, col_end_excl) = selection.cols_for_row(row)?;
            let col_end = col_end_excl.min(cells.len().max(col_start));
            let col_start = col_start.min(col_end);
            let slice = terminal_text_cell_slice(&cells, col_start, col_end);
            parts.push(slice.trim_end().to_string());
        }
        let text = parts.join("\n");
        if text.is_empty() { None } else { Some(text) }
    }

    pub(in crate::features) fn copy_terminal_selection(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(text) = self.selected_terminal_text() else {
            return false;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.shell
            .set_status("copied terminal selection".to_string());
        self.notify_terminal_selection_owner_surface(cx);
        self.terminal.menus.actions_open = false;
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

    pub(in crate::features) fn start_terminal_selection_for_session(
        &mut self,
        session_id: Option<&str>,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        let selection_session_id = session_id
            .filter(|session_id| !session_id.is_empty())
            .map(str::to_string)
            .or_else(|| {
                self.session
                    .active_id_owned()
                    .filter(|session_id| !session_id.is_empty())
            });
        let Some(geometry) =
            self.terminal_hit_test_geometry_for_session(selection_session_id.as_deref(), cx)
        else {
            return;
        };
        let cell = terminal_cell_for_visual_geometry(event.position, &geometry);
        // Applications with mouse tracking (vim/less/tmux) consume left presses.
        if let Some(session_id) = selection_session_id.as_deref()
            && self.maybe_send_mouse_report_for_session(
                TerminalMouseReportRequest {
                    session_id,
                    button: 0,
                    col: cell.col as u16,
                    row: cell.row as u16,
                    press: true,
                    motion: false,
                    modifiers: event.modifiers,
                },
                cx,
            )
        {
            self.clear_terminal_selection(cx);
            return;
        }
        let (rows, cols) = self.terminal_grid_size_for_session(selection_session_id.as_deref());
        // Shift+click extends the existing selection from its anchor (xterm-style).
        if event.modifiers.shift
            && event.click_count <= 1
            && let Some(selection) = self.terminal.selection.selection.as_mut()
        {
            selection.head = cell;
            if self.terminal.selection.session_id.is_none() {
                self.terminal.selection.session_id = selection_session_id;
            }
            self.terminal.selection.dragging = true;
            // Defer status-bar shell notify until selection finishes.
            self.notify_terminal_selection_owner_surface(cx);
            return;
        }
        if event.click_count >= 3 {
            self.terminal.selection.selection = Some(TerminalSelection::from_range(
                TerminalCellPos::new(cell.row, 0),
                TerminalCellPos::new(cell.row, cols.saturating_sub(1)),
                geometry.display_offset,
                geometry.viewport_anchor_row,
            ));
            self.terminal.selection.session_id = selection_session_id;
            self.terminal.selection.dragging = false;
            self.shell
                .set_status(format!("selected line {}", cell.row + 1));
            self.notify_terminal_selection_owner_surface(cx);
            // Discrete click: status bar update is fine (not a high-frequency path).
            cx.notify();
            return;
        }
        if event.click_count == 2 {
            let word = self.word_bounds_at_for_viewport(
                selection_session_id.as_deref(),
                cell,
                geometry.display_offset,
                geometry.viewport_anchor_row,
            );
            self.terminal.selection.selection = Some(TerminalSelection::from_range(
                TerminalCellPos::new(cell.row, word.0),
                TerminalCellPos::new(cell.row, word.1.saturating_sub(1).max(word.0)),
                geometry.display_offset,
                geometry.viewport_anchor_row,
            ));
            self.terminal.selection.session_id = selection_session_id;
            self.terminal.selection.dragging = false;
            self.shell.set_status("selected word".to_string());
            self.notify_terminal_selection_owner_surface(cx);
            cx.notify();
            return;
        }
        self.terminal.selection.selection = Some(TerminalSelection::with_viewport(
            cell,
            geometry.display_offset,
            geometry.viewport_anchor_row,
        ));
        self.terminal.selection.session_id = selection_session_id;
        self.terminal.selection.dragging = true;
        let _ = rows;
        self.notify_terminal_selection_owner_surface(cx);
    }

    pub(in crate::features) fn update_terminal_selection_drag(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        if let Some(button) = self.terminal.selection.mouse_report_button {
            let captured_session_id = self
                .terminal
                .selection
                .mouse_report_session_id
                .clone()
                .or_else(|| self.session.active_id_owned());
            if let Some(session_id) = captured_session_id
                && let Some(cell) = self.point_to_terminal_cell_for_session(
                    Some(session_id.as_str()),
                    event.position,
                    cx,
                )
                && self.maybe_send_mouse_report_for_session(
                    TerminalMouseReportRequest {
                        session_id: &session_id,
                        button,
                        col: cell.col as u16,
                        row: cell.row as u16,
                        press: true,
                        motion: true,
                        modifiers: event.modifiers,
                    },
                    cx,
                )
            {
                return;
            }
        }
        if !self.terminal.selection.dragging {
            return;
        }
        let selection_session_id = self
            .terminal
            .selection
            .session_id
            .as_deref()
            .or(self.session.active_id())
            .filter(|session_id| !session_id.is_empty());
        let Some(geometry) = self.terminal_hit_test_geometry_for_session(selection_session_id, cx)
        else {
            return;
        };
        let cell = terminal_cell_for_visual_geometry(event.position, &geometry);
        if let Some(selection) = self.terminal.selection.selection.as_mut()
            && selection.head != cell
        {
            selection.head = cell;
            if self.terminal.selection.session_id.is_none() {
                self.terminal.selection.session_id = selection_session_id.map(str::to_string);
            }
            self.queue_terminal_selection_drag_visual_notify(cx);
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
        if self.finish_terminal_mouse_report(event, cx) {
            self.clear_terminal_selection(cx);
            return;
        }
        if !self.terminal.selection.dragging {
            // Stationary click without an active drag can still reposition the
            // tracked input cursor (Tauri handleTerminalMouseUp smart cursor).
            if self.terminal.selection.selection.is_none() {
                self.handle_smart_input_click(event, cx);
            }
            return;
        }
        let selection_session_id = self
            .terminal
            .selection
            .session_id
            .as_deref()
            .or(self.session.active_id())
            .filter(|session_id| !session_id.is_empty());
        if let Some(geometry) =
            self.terminal_hit_test_geometry_for_session(selection_session_id, cx)
        {
            let cell = terminal_cell_for_visual_geometry(event.position, &geometry);
            if let Some(selection) = self.terminal.selection.selection.as_mut() {
                selection.head = cell;
            }
        }
        self.terminal.selection.dragging = false;
        if self
            .terminal
            .selection
            .selection
            .as_ref()
            .is_some_and(|selection| selection.is_empty())
        {
            self.terminal.selection.selection = None;
            // Empty selection after click: try smart input cursor move.
            self.handle_smart_input_click(event, cx);
        } else if let Some(selected) = self.smart_cursor_selected_input_range() {
            // Collapse caret toward click/edge, then clear selection (Tauri path).
            let target = if event.click_count >= 2 {
                selected.end
            } else if let Some(index) = self.input_index_at_mouse(event.position, cx) {
                index.clamp(selected.start, selected.end)
            } else {
                selected.end
            };
            if self.settings.summary().interaction_copy_on_select {
                let _ = self.copy_terminal_selection(cx);
            }
            let _ = self.move_smart_input_cursor(target, cx);
            self.clear_terminal_selection(cx);
        } else if self.settings.summary().interaction_copy_on_select {
            let _ = self.copy_terminal_selection(cx);
        } else if self.terminal.selection.selection.is_some() {
            // One shell notify for status after drag ends (not per mouse move).
            self.shell.set_status("selection ready".to_string());
            cx.notify();
        }
        self.notify_terminal_selection_owner_surface(cx);
    }

    fn queue_terminal_selection_drag_visual_notify(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self
            .terminal
            .selection
            .session_id
            .clone()
            .or_else(|| self.session.active_id_owned())
        else {
            return;
        };
        if session_id.is_empty() {
            return;
        }
        if !self.shell.queue_terminal_selection_drag(session_id) {
            return;
        }
        cx.spawn(async move |this, cx| {
            Timer::after(TERMINAL_SELECTION_DRAG_NOTIFY_DELAY).await;
            let _ = this.update(cx, |this, cx| {
                this.flush_terminal_selection_drag_visual_notify(cx);
            });
        })
        .detach();
    }

    fn flush_terminal_selection_drag_visual_notify(&mut self, cx: &mut Context<Self>) {
        let session_ids = self.shell.drain_terminal_selection_drag_sessions();
        for session_id in session_ids {
            self.notify_terminal_selection_visual_only(session_id.as_str(), cx);
        }
    }

    fn word_bounds_at_for_viewport(
        &self,
        session_id: Option<&str>,
        cell: TerminalCellPos,
        display_offset: usize,
        viewport_anchor_row: usize,
    ) -> (usize, usize) {
        let snapshot = self.terminal_snapshot_for_session(session_id, display_offset);
        self.word_bounds_at_for_snapshot(cell, snapshot.as_ref(), viewport_anchor_row)
    }

    fn word_bounds_at_for_snapshot(
        &self,
        cell: TerminalCellPos,
        snapshot: &TerminalSnapshot,
        viewport_anchor_row: usize,
    ) -> (usize, usize) {
        let snapshot_row = viewport_anchor_row
            .checked_add(cell.row)
            .filter(|row| *row < snapshot.row_count());
        let line = snapshot_row
            .and_then(|row| snapshot.line(row))
            .unwrap_or("");
        let cells = terminal_text_cells(line);
        if cells.is_empty() {
            return (cell.col, cell.col.saturating_add(1));
        }
        let idx = cell.col.min(cells.len().saturating_sub(1));
        // xterm wordSeparator semantics: characters listed are separators, not word body.
        let separators = self.settings.summary().interaction_word_separators.as_str();
        let is_word = |cell: &TerminalTextCell| terminal_text_cell_is_word(cell, separators);
        if !is_word(&cells[idx]) {
            return (idx, idx.saturating_add(1));
        }
        let mut start = idx;
        while start > 0 && is_word(&cells[start - 1]) {
            start -= 1;
        }
        let mut end = idx + 1;
        while end < cells.len() && is_word(&cells[end]) {
            end += 1;
        }
        (start, end)
    }
}

fn terminal_text_cell_is_word(cell: &TerminalTextCell, separators: &str) -> bool {
    cell.text
        .chars()
        .find(|ch| !terminal_is_zero_width_mark(*ch))
        .is_some_and(|ch| !separators.contains(ch))
}

fn terminal_all_lines_text(lines: Vec<String>) -> Option<String> {
    let text = lines
        .into_iter()
        .map(|line| line.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let text = text.trim_end_matches('\n').to_string();
    if text.is_empty() { None } else { Some(text) }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::terminal::{TerminalTextCell, terminal_text_cell_slice, terminal_text_cells};

    use super::{
        TERMINAL_SELECTION_DRAG_NOTIFY_DELAY, terminal_all_lines_text, terminal_text_cell_is_word,
    };

    #[test]
    fn terminal_selection_drag_notify_delay_is_frame_coalesced() {
        assert_eq!(
            TERMINAL_SELECTION_DRAG_NOTIFY_DELAY,
            Duration::from_millis(8)
        );
    }

    #[test]
    fn terminal_text_cells_keep_combining_mark_with_previous_cell() {
        let cells = terminal_text_cells("e\u{301}x");

        assert_eq!(
            cells,
            vec![
                TerminalTextCell {
                    text: "e\u{301}".to_string(),
                    byte_start: 0,
                    byte_end: "e\u{301}".len(),
                },
                TerminalTextCell {
                    text: "x".to_string(),
                    byte_start: "e\u{301}".len(),
                    byte_end: "e\u{301}x".len(),
                },
            ]
        );
        assert_eq!(terminal_text_cell_slice(&cells, 0, 1), "e\u{301}");
        assert_eq!(terminal_text_cell_slice(&cells, 1, 2), "x");
    }

    #[test]
    fn terminal_text_word_cells_use_base_character_for_separators() {
        let cells = terminal_text_cells("e\u{301}/x");

        assert!(terminal_text_cell_is_word(&cells[0], "/"));
        assert!(!terminal_text_cell_is_word(&cells[1], "/"));
        assert!(terminal_text_cell_is_word(&cells[2], "/"));
    }

    #[test]
    fn terminal_text_cells_count_wide_char_as_two_terminal_cells() {
        let cells = terminal_text_cells("界x");

        assert_eq!(cells.len(), 3);
        assert_eq!(cells[0].text, "界");
        assert_eq!(cells[1].text, "界");
        assert_eq!(cells[0].byte_start, cells[1].byte_start);
        assert_eq!(cells[0].byte_end, cells[1].byte_end);
        assert_eq!(terminal_text_cell_slice(&cells, 0, 1), "界");
        assert_eq!(terminal_text_cell_slice(&cells, 1, 2), "界");
        assert_eq!(terminal_text_cell_slice(&cells, 0, 2), "界");
        assert_eq!(terminal_text_cell_slice(&cells, 2, 3), "x");
    }

    #[test]
    fn terminal_text_cells_attach_combining_mark_to_all_wide_halves() {
        let text = "界\u{301}x";
        let cells = terminal_text_cells(text);

        assert_eq!(cells.len(), 3);
        assert_eq!(cells[0].text, "界\u{301}");
        assert_eq!(cells[1].text, "界\u{301}");
        assert_eq!(cells[0].byte_end, "界\u{301}".len());
        assert_eq!(cells[1].byte_end, "界\u{301}".len());
        assert_eq!(terminal_text_cell_slice(&cells, 0, 2), "界\u{301}");
        assert_eq!(terminal_text_cell_slice(&cells, 1, 2), "界\u{301}");
    }

    #[test]
    fn terminal_text_cells_attach_variation_selector_to_previous_cell() {
        let text = "a\u{fe0f}x";
        let cells = terminal_text_cells(text);

        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].text, "a\u{fe0f}");
        assert_eq!(cells[0].byte_end, "a\u{fe0f}".len());
        assert_eq!(terminal_text_cell_slice(&cells, 0, 1), "a\u{fe0f}");
        assert_eq!(terminal_text_cell_slice(&cells, 1, 2), "x");
    }

    #[test]
    fn terminal_all_lines_text_preserves_internal_blank_lines() {
        assert_eq!(
            terminal_all_lines_text(vec![
                "first  ".to_string(),
                String::new(),
                "last".to_string(),
                String::new(),
            ]),
            Some("first\n\nlast".to_string())
        );
    }
}
