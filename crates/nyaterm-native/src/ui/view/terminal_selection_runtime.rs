use super::*;
use gpui::{Bounds, ClickEvent, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point};

/// Approximate monospaced cell metrics used for hit-testing the painted terminal grid.
/// Keep in sync with `terminal_line_element` row height and surface font size.
const CELL_WIDTH_RATIO: f32 = 0.6;
const LINE_HEIGHT_RATIO: f32 = 1.25;

impl NyaTermApp {
    pub(in crate::ui::view) fn terminal_cell_size(&self) -> (f32, f32) {
        let font_size = self.settings.terminal_font_size.max(8) as f32;
        // Prefer painted fixed 18px when font is near default; scale with font otherwise.
        let cell_h = if (font_size - 14.).abs() < 0.5 {
            18.
        } else {
            (font_size * LINE_HEIGHT_RATIO).max(font_size + 2.)
        };
        let cell_w = (font_size * CELL_WIDTH_RATIO).max(4.);
        (cell_w, cell_h)
    }

    pub(in crate::ui::view) fn terminal_content_padding_px(&self) -> f32 {
        if self.settings.terminal_show_workspace_padding {
            16.
        } else {
            8.
        }
    }

    pub(in crate::ui::view) fn terminal_gutter_width_px(&self) -> f32 {
        // Keep in sync with terminal_surface gutter column widths.
        let mut width = 0.;
        if self.settings.terminal_show_timestamps {
            width += if self.settings.terminal_show_timestamp_milliseconds {
                96.
            } else {
                72.
            };
        }
        if self.settings.terminal_show_line_numbers {
            width += 40.;
        }
        if self.settings.terminal_show_timestamps && self.settings.terminal_show_line_numbers {
            width += 4.; // gap_1
        }
        if width > 0. {
            width += 4.; // pr_1 trailing gutter padding
        }
        width
    }

    pub(in crate::ui::view) fn active_terminal_grid_size(&self) -> (usize, usize) {
        let offset = self.active_terminal_scroll_offset();
        let snapshot = self
            .active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.screen.viewport_snapshot(offset))
            .unwrap_or_else(|| self.terminal_screen.viewport_snapshot(offset));
        let rows = snapshot.lines.len().max(1);
        let cols = snapshot
            .lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(80)
            .max(80);
        (rows, cols)
    }

    pub(in crate::ui::view) fn point_to_terminal_cell(
        &self,
        position: Point<Pixels>,
    ) -> Option<TerminalCellPos> {
        let bounds = self.terminal_surface_bounds?;
        let (cell_w, cell_h) = self.terminal_cell_size();
        let pad = self.terminal_content_padding_px();
        let gutter = self.terminal_gutter_width_px();
        let local_x = f32::from(position.x - bounds.origin.x) - pad - gutter;
        let local_y = f32::from(position.y - bounds.origin.y) - pad;
        if local_y < -cell_h || local_x < -cell_w {
            // Still allow clamping when slightly outside.
        }
        let (rows, cols) = self.active_terminal_grid_size();
        let row = (local_y / cell_h).floor().max(0.) as usize;
        let col = (local_x / cell_w).floor().max(0.) as usize;
        Some(TerminalCellPos::new(
            row.min(rows.saturating_sub(1)),
            col.min(cols.saturating_sub(1)),
        ))
    }

    pub(in crate::ui::view) fn clear_terminal_selection(&mut self, cx: &mut Context<Self>) {
        if self.terminal_selection.is_some() || self.terminal_selection_dragging {
            self.terminal_selection = None;
            self.terminal_selection_dragging = false;
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn select_all_terminal_visible(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn selected_terminal_text(&self) -> Option<String> {
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

    pub(in crate::ui::view) fn copy_terminal_selection(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(text) = self.selected_terminal_text() else {
            return false;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.terminal_status = "copied terminal selection".to_string();
        self.terminal_actions_open = false;
        cx.notify();
        true
    }

    pub(in crate::ui::view) fn copy_terminal_selection_or_visible(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.copy_terminal_selection(cx) {
            return;
        }
        self.copy_terminal_visible_text(cx);
    }

    pub(in crate::ui::view) fn start_terminal_selection(
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
        let (rows, cols) = self.active_terminal_grid_size();
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

    pub(in crate::ui::view) fn update_terminal_selection_drag(
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

    pub(in crate::ui::view) fn finish_terminal_selection(
        &mut self,
        event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        if !self.terminal_selection_dragging {
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
        } else if self.settings.interaction_copy_on_select {
            let _ = self.copy_terminal_selection(cx);
        }
        cx.notify();
    }

    fn word_bounds_at(&self, cell: TerminalCellPos) -> (usize, usize) {
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
        if !is_word_char(chars[idx]) {
            return (idx, idx.saturating_add(1));
        }
        let mut start = idx;
        while start > 0 && is_word_char(chars[start - 1]) {
            start -= 1;
        }
        let mut end = idx + 1;
        while end < chars.len() && is_word_char(chars[end]) {
            end += 1;
        }
        (start, end)
    }


    pub(in crate::ui::view) fn try_activate_action_link_at_click(
        &mut self,
        event: &ClickEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(cell) = self.point_to_terminal_cell(event.position()) else {
            return false;
        };
        let offset = self.active_terminal_scroll_offset();
        let snapshot = self
            .active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.screen.viewport_snapshot(offset))
            .unwrap_or_else(|| self.terminal_screen.viewport_snapshot(offset));
        let Some(line) = snapshot.lines.get(cell.row) else {
            return false;
        };
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            return false;
        }
        let char_offset = cell.col.min(chars.len().saturating_sub(1));
        let byte_offset: usize = chars.iter().take(char_offset).map(|ch| ch.len_utf8()).sum();
        let matchers = &self.settings.terminal_action_links_matchers;
        let Some(item) = match_at_offset(line, byte_offset, matchers) else {
            return false;
        };
        let actions = actions_for_match(&item);
        let Some(default) = actions
            .iter()
            .find(|action| action.is_default)
            .cloned()
            .or_else(|| actions.first().cloned())
        else {
            return false;
        };
        if let Some(url) = default.open_url {
            match open_external_url_for_action(&url) {
                Ok(()) => self.terminal_status = format!("opened {}: {url}", item.kind.label()),
                Err(error) => self.terminal_status = format!("open link failed: {error}"),
            }
            cx.notify();
            return true;
        }
        if let Some(command) = default.command {
            self.execute_action_link_command(command, cx);
            return true;
        }
        false
    }

    /// Capture painted bounds for hit-testing; called from a canvas prepaint under the output area.
    pub(in crate::ui::view) fn remember_terminal_surface_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.terminal_surface_bounds = Some(bounds);
    }
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' || ch == '/' || ch == ':'
}

/// Invisible canvas child that records the terminal output bounds for selection hit-testing.
pub(in crate::ui::view) fn terminal_bounds_tracker(
    entity: gpui::Entity<NyaTermApp>,
) -> impl IntoElement {
    gpui::canvas(
        move |bounds, _window, cx| {
            // Defer mutation so we never re-enter the entity while layout/prepaint is running.
            let entity = entity.clone();
            cx.defer(move |cx| {
                let _ = entity.update(cx, |this, _cx| {
                    this.remember_terminal_surface_bounds(bounds);
                });
            });
        },
        |_bounds, _state, _window, _cx| {},
    )
    .absolute()
    .size_full()
}

fn open_external_url_for_action(url: &str) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("empty url".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open url: {error}"))
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open url: {error}"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open url: {error}"))
    }
}

