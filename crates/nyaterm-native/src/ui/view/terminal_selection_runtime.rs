use super::*;
use gpui::{Bounds, ClickEvent, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point};

/// Approximate monospaced cell metrics used for hit-testing the painted terminal grid.
/// Keep in sync with `terminal_line_element` row height and surface font size.
const CELL_WIDTH_RATIO: f32 = 0.62;
const LINE_HEIGHT_RATIO: f32 = 1.25;

impl NyaTermApp {
    pub(in crate::ui::view) fn terminal_cell_size(&self) -> (f32, f32) {
        if let Some(metrics) = self.terminal_cell_metrics {
            return metrics;
        }
        self.fallback_terminal_cell_size()
    }

    fn fallback_terminal_cell_size(&self) -> (f32, f32) {
        let font_size = self.settings.terminal_font_size.max(8) as f32;
        // Prefer painted fixed 18px when font is near default; scale with font otherwise.
        let cell_h = if (font_size - 14.).abs() < 0.5 {
            18.
        } else {
            (font_size * LINE_HEIGHT_RATIO).max(font_size + 2.)
        };
        // Tauri gutter fallback uses fontSize * 0.62 when measured cell is unavailable.
        let cell_w = (font_size * CELL_WIDTH_RATIO).max(4.);
        (cell_w, cell_h)
    }

    /// Refresh monospaced cell metrics from GPUI TextSystem for the configured terminal font.
    pub(in crate::ui::view) fn refresh_terminal_cell_metrics(&mut self, cx: &App) {
        let font_size = self.settings.terminal_font_size.max(8) as f32;
        let family = self.settings.terminal_font_family.trim();
        let family = if family.is_empty() {
            "JetBrains Mono"
        } else {
            family
        };
        let text_system = cx.text_system();
        let font_id = text_system.resolve_font(&gpui::font(SharedString::from(family.to_string())));
        let size = px(font_size);
        let measured_w = text_system
            .ch_advance(font_id, size)
            .or_else(|_| text_system.em_advance(font_id, size))
            .ok()
            .map(|w| f32::from(w))
            .filter(|w| w.is_finite() && *w > 1.0);
        let ascent = f32::from(text_system.ascent(font_id, size));
        let descent = f32::from(text_system.descent(font_id, size)).abs();
        let font_line = (ascent + descent).max(font_size + 2.);
        // Keep painter contract: default 14px font paints ~18px rows.
        let cell_h = if (font_size - 14.).abs() < 0.5 {
            18.
        } else {
            (font_size * LINE_HEIGHT_RATIO).max(font_line)
        };
        let cell_w = measured_w.unwrap_or_else(|| (font_size * CELL_WIDTH_RATIO).max(4.));
        let next = (cell_w, cell_h);
        if self.terminal_cell_metrics != Some(next) {
            self.terminal_cell_metrics = Some(next);
        }
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
        let (cell_w, _) = self.terminal_cell_size();
        let gutter_font = (self.settings.terminal_font_size.max(8) as f32 * 0.85).max(8.);
        // Gutter text uses 0.85x terminal font; approximate char width proportionally.
        let gutter_cell_w = (cell_w * (gutter_font / self.settings.terminal_font_size.max(8) as f32))
            .max(4.);
        let mut width = 0.;
        if self.settings.terminal_show_timestamps {
            // HH:MM:SS = 8 chars, HH:MM:SS.mmm = 12 chars (+ small pad like Tauri).
            let cols = if self.settings.terminal_show_timestamp_milliseconds {
                12.
            } else {
                8.
            };
            width += (gutter_cell_w * cols + 2.).max(if self.settings.terminal_show_timestamp_milliseconds {
                96.
            } else {
                72.
            });
        }
        if self.settings.terminal_show_line_numbers {
            // 5-digit absolute line numbers + pad.
            width += (gutter_cell_w * 5. + 2.).max(40.);
        }
        if self.settings.terminal_show_timestamps && self.settings.terminal_show_line_numbers {
            width += 4.; // gap_1
        }
        if width > 0. {
            width += 4.; // pr_1 trailing gutter padding
        }
        width
    }

    pub(in crate::ui::view) fn terminal_timestamp_gutter_width_px(&self) -> f32 {
        let (cell_w, _) = self.terminal_cell_size();
        let gutter_font = (self.settings.terminal_font_size.max(8) as f32 * 0.85).max(8.);
        let gutter_cell_w = (cell_w * (gutter_font / self.settings.terminal_font_size.max(8) as f32))
            .max(4.);
        let cols = if self.settings.terminal_show_timestamp_milliseconds {
            12.
        } else {
            8.
        };
        (gutter_cell_w * cols + 2.).max(if self.settings.terminal_show_timestamp_milliseconds {
            96.
        } else {
            72.
        })
    }

    pub(in crate::ui::view) fn terminal_line_number_gutter_width_px(&self) -> f32 {
        let (cell_w, _) = self.terminal_cell_size();
        let gutter_font = (self.settings.terminal_font_size.max(8) as f32 * 0.85).max(8.);
        let gutter_cell_w = (cell_w * (gutter_font / self.settings.terminal_font_size.max(8) as f32))
            .max(4.);
        (gutter_cell_w * 5. + 2.).max(40.)
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




    pub(in crate::ui::view) fn clear_action_link_tooltip(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        if self.action_link_tooltip.take().is_some() {
            changed = true;
        }
        if self.action_link_hover_pending.take().is_some() {
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn poll_action_link_tooltip_delay(&mut self, cx: &mut Context<Self>) {
        let Some((key, started, tip)) = self.action_link_hover_pending.clone() else {
            return;
        };
        if started.elapsed() < Duration::from_millis(250) {
            return;
        }
        self.action_link_hover_pending = None;
        // Only show if still matching the pending key (not superseded).
        if self
            .action_link_tooltip
            .as_ref()
            .is_some_and(|current| current.match_key == key)
        {
            return;
        }
        self.action_link_tooltip = Some(tip);
        cx.notify();
    }

    pub(in crate::ui::view) fn update_action_link_hover(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        if !self.settings.terminal_action_links_enabled {
            self.clear_action_link_tooltip(cx);
            return;
        }
        // Hide while menus are open or while selecting text.
        if self.action_link_menu.is_some()
            || self.terminal_context_menu.is_some()
            || self.terminal_selection_dragging
            || self.translation_dialog.is_some()
        {
            self.clear_action_link_tooltip(cx);
            return;
        }
        let Some((item, actions)) = self.action_link_at_point(event.position) else {
            self.clear_action_link_tooltip(cx);
            return;
        };
        if actions.is_empty() {
            self.clear_action_link_tooltip(cx);
            return;
        }
        let default = actions
            .iter()
            .find(|action| action.is_default)
            .cloned()
            .or_else(|| actions.first().cloned());
        let Some(default) = default else {
            self.clear_action_link_tooltip(cx);
            return;
        };
        let match_key = format!(
            "{}|{}|{}|{}",
            item.kind.label(),
            item.value,
            item.start,
            item.end
        );
        let preview = default
            .command
            .clone()
            .or_else(|| default.open_url.clone())
            .unwrap_or_else(|| default.label.clone());
        let next = ActionLinkTooltipState {
            x: event.position.x,
            y: event.position.y,
            kind_label: item.kind.label().to_string(),
            value: item.value.clone(),
            default_action_label: default.label.clone(),
            default_action_preview: preview,
            has_more_actions: actions.len() > 1,
            match_key: match_key.clone(),
        };
        // Already visible for this link: track position.
        if let Some(current) = self.action_link_tooltip.as_ref() {
            if current.match_key == match_key {
                if current.x != next.x || current.y != next.y {
                    self.action_link_tooltip = Some(next);
                    cx.notify();
                }
                return;
            }
        }
        // Pending same link: update position only.
        if let Some((key, started, _)) = self.action_link_hover_pending.clone() {
            if key == match_key {
                let ready = started.elapsed() >= Duration::from_millis(250);
                self.action_link_hover_pending = Some((match_key, started, next));
                if ready {
                    self.poll_action_link_tooltip_delay(cx);
                }
                return;
            }
        }
        // New link under cursor: start 250ms delay (Tauri ActionLinkTooltip).
        self.action_link_tooltip = None;
        self.action_link_hover_pending = Some((match_key, Instant::now(), next));
        cx.notify();
    }

    fn action_link_at_point(
        &self,
        position: Point<Pixels>,
    ) -> Option<(ActionLinkMatch, Vec<ActionLinkAction>)> {
        // Only hit-test when the pointer is over the painted terminal content area.
        let bounds = self.terminal_surface_bounds?;
        let (cell_w, cell_h) = self.terminal_cell_size();
        let pad = self.terminal_content_padding_px();
        let gutter = self.terminal_gutter_width_px();
        let local_x = f32::from(position.x - bounds.origin.x) - pad - gutter;
        let local_y = f32::from(position.y - bounds.origin.y) - pad;
        if local_x < 0. || local_y < 0. {
            return None;
        }
        let (rows, cols) = self.active_terminal_grid_size();
        if local_y >= cell_h * rows as f32 || local_x >= cell_w * cols as f32 {
            return None;
        }
        let cell = self.point_to_terminal_cell(position)?;
        let offset = self.active_terminal_scroll_offset();
        let snapshot = self
            .active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.screen.viewport_snapshot(offset))
            .unwrap_or_else(|| self.terminal_screen.viewport_snapshot(offset));
        let line = snapshot.lines.get(cell.row)?;
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            return None;
        }
        let char_offset = cell.col.min(chars.len().saturating_sub(1));
        let byte_offset: usize = chars.iter().take(char_offset).map(|ch| ch.len_utf8()).sum();
        let matchers = &self.settings.terminal_action_links_matchers;
        let item = match_at_offset(line, byte_offset, matchers)?;
        let actions = actions_for_match(&item);
        Some((item, actions))
    }

    pub(in crate::ui::view) fn close_action_link_menu(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        if self.action_link_menu.take().is_some() {
            self.terminal_status = "action link menu closed".to_string();
            changed = true;
        }
        if self.action_link_tooltip.take().is_some() {
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn try_open_action_link_menu_at_click(
        &mut self,
        event: &ClickEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((item, actions)) = self.action_link_at_click(event) else {
            return false;
        };
        if actions.is_empty() {
            return false;
        }
        let menu_actions = actions
            .into_iter()
            .map(|action| ActionLinkMenuAction {
                id: action.id,
                label: action.label,
                command: action.command,
                open_url: action.open_url,
                is_default: action.is_default,
            })
            .collect::<Vec<_>>();
        self.action_link_tooltip = None;
        self.action_link_menu = Some(ActionLinkMenuState {
            x: event.position().x,
            y: event.position().y,
            kind_label: item.kind.label().to_string(),
            value: item.value,
            actions: menu_actions,
        });
        self.terminal_context_menu = None;
        self.terminal_status = format!("action link menu: {}", item.kind.label());
        cx.notify();
        true
    }

    fn action_link_at_click(
        &self,
        event: &ClickEvent,
    ) -> Option<(ActionLinkMatch, Vec<ActionLinkAction>)> {
        self.action_link_at_point(event.position())
    }

    pub(in crate::ui::view) fn try_activate_action_link_at_click(
        &mut self,
        event: &ClickEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((item, actions)) = self.action_link_at_click(event) else {
            return false;
        };
        self.action_link_tooltip = None;
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

