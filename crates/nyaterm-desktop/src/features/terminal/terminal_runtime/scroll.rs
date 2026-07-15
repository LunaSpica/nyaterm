use super::*;

pub(in crate::features) fn terminal_scroll_track_ratio(
    bounds: gpui::Bounds<gpui::Pixels>,
    pointer_y: gpui::Pixels,
) -> f32 {
    let height = f32::from(bounds.size.height).max(1.0);
    let local_y = f32::from(pointer_y - bounds.origin.y);
    (local_y / height).clamp(0.0, 1.0)
}

pub(in crate::features) fn terminal_resize_geometry_for_bounds(
    bounds: gpui::Bounds<gpui::Pixels>,
    cell_width: f32,
    cell_height: f32,
    padding: f32,
    gutter_width: f32,
) -> TerminalResizeGeometry {
    terminal_resize_geometry_for_size(
        f32::from(bounds.size.width),
        f32::from(bounds.size.height),
        cell_width,
        cell_height,
        padding,
        gutter_width,
    )
}

impl NyaTermApp {
    pub(in crate::features) fn active_terminal_scroll_offset(&self) -> usize {
        if let Some(session_id) = self.active_session_id.as_deref() {
            self.terminal_views
                .get(session_id)
                .map(|view| view.scroll_offset)
                .unwrap_or(0)
        } else {
            self.terminal_scroll_offset
        }
    }

    pub(in crate::features) fn scroll_terminal_by(
        &mut self,
        delta_lines: i32,
        cx: &mut Context<Self>,
    ) {
        let session_id = self.active_session_id.clone();
        self.scroll_terminal_by_for_session(session_id.as_deref(), delta_lines, cx);
    }

    pub(in crate::features) fn scroll_terminal_by_for_session(
        &mut self,
        session_id: Option<&str>,
        delta_lines: i32,
        cx: &mut Context<Self>,
    ) {
        if delta_lines == 0 {
            return;
        }
        let mut snapshot_request: Option<(String, usize)> = None;
        if let Some(session_id) = session_id.filter(|id| !id.is_empty()) {
            if let Some(view) = self.terminal_views.get_mut(session_id) {
                let max = view.scrollback_len_for_ui();
                let next = if delta_lines > 0 {
                    view.scroll_offset.saturating_add(delta_lines as usize)
                } else {
                    view.scroll_offset.saturating_sub((-delta_lines) as usize)
                };
                view.scroll_offset = next.min(max);
                if view.scroll_offset > 0 {
                    snapshot_request = Some((session_id.to_string(), view.scroll_offset));
                }
            }
        } else {
            let max = self.terminal_screen.scrollback_len();
            let next = if delta_lines > 0 {
                self.terminal_scroll_offset
                    .saturating_add(delta_lines as usize)
            } else {
                self.terminal_scroll_offset
                    .saturating_sub((-delta_lines) as usize)
            };
            self.terminal_scroll_offset = next.min(max);
        }
        if let Some(session_id) = session_id.filter(|id| !id.is_empty()) {
            if let Some(view) = self.terminal_views.get_mut(session_id) {
                if view.scroll_offset == 0 {
                    view.has_new_while_scrolled = false;
                }
            }
        } else if self.terminal_scroll_offset == 0 {
            if let Some(session_id) = self.active_session_id.clone()
                && let Some(view) = self.terminal_views.get_mut(&session_id)
            {
                view.has_new_while_scrolled = false;
            }
        }
        if let Some((session_id, offset)) = snapshot_request {
            self.request_terminal_frame_snapshot_when_idle(&session_id, offset);
        }
        cx.notify();
    }

    /// Insert quoted local file paths into the active session (Tauri Local drop).

    pub(in crate::features) fn handle_terminal_external_file_drop(
        &mut self,
        session_id: String,
        paths: Vec<std::path::PathBuf>,
        cx: &mut Context<Self>,
    ) {
        self.terminal_file_drop_hover = None;
        if session_id.is_empty() || paths.is_empty() {
            cx.notify();
            return;
        }
        if self.is_session_disconnected(&session_id) {
            self.terminal_status =
                "session disconnected — reconnect before dropping files".to_string();
            cx.notify();
            return;
        }
        let kind = self
            .ordered_sessions()
            .into_iter()
            .find(|s| s.id == session_id)
            .map(|s| s.kind);
        let path_strings: Vec<String> = paths
            .iter()
            .filter_map(|p| {
                if p.is_dir() {
                    None
                } else {
                    Some(p.display().to_string())
                }
            })
            .collect();
        let has_dirs = paths.iter().any(|p| p.is_dir());
        match kind {
            Some(SessionKind::LocalPty) | None => {
                if path_strings.is_empty() {
                    self.terminal_status =
                        "folders cannot be dropped into a local terminal".to_string();
                    cx.notify();
                    return;
                }
                // Activate target session if needed.
                if self.active_session_id.as_deref() != Some(session_id.as_str()) {
                    self.activate_session_id(&session_id);
                }
                let text = nyaterm_core::format_local_terminal_drop_input(&path_strings);
                if self.send_terminal_input(text.into_bytes(), cx) {
                    self.terminal_status =
                        format!("inserted {} path(s) into terminal", path_strings.len());
                    cx.notify();
                }
            }
            Some(
                SessionKind::Ssh | SessionKind::Telnet | SessionKind::Serial | SessionKind::RawTcp,
            ) => {
                if has_dirs {
                    self.terminal_status =
                        "folders cannot be uploaded via ZMODEM — use the file explorer for SFTP"
                            .to_string();
                    cx.notify();
                    return;
                }
                if path_strings.is_empty() {
                    cx.notify();
                    return;
                }
                let files: Vec<std::path::PathBuf> = path_strings
                    .into_iter()
                    .map(std::path::PathBuf::from)
                    .collect();
                self.start_zmodem_upload(session_id, files, cx);
            }
        }
    }

    pub(in crate::features) fn set_terminal_file_drop_hover(
        &mut self,
        session_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self.terminal_file_drop_hover != session_id {
            self.terminal_file_drop_hover = session_id;
            cx.notify();
        }
    }

    pub(in crate::features) fn apply_session_cwd(&mut self, session_id: &str, cwd: String) {
        let changed = self
            .session_cwds
            .get(session_id)
            .map(|prev| prev != &cwd)
            .unwrap_or(true);
        self.session_cwds
            .insert(session_id.to_string(), cwd.clone());
        // Auto-sync the transfer browser path when enabled for the active SSH session.
        if changed
            && self.active_session_id.as_deref() == Some(session_id)
            && self.transfer_browser_auto_sync_cwd_enabled()
            && !cwd.trim().is_empty()
        {
            if self.transfer_browser_path != cwd {
                self.transfer_browser_path = cwd.clone();
                self.transfer_browser_path_draft = cwd.clone();
                self.transfer_browser_status = format!("cwd synced: {cwd}");
            }
        }
    }

    /// Apply OSC 133 command-start / command-finish edges (Tauri shell integration).

    pub(in crate::features) fn apply_shell_integration_edges(
        &mut self,
        session_id: &str,
        started: bool,
        finished: bool,
        command_running: bool,
    ) {
        // Only affect the active session suggestion pipeline.
        if self.active_session_id.as_deref() != Some(session_id) {
            return;
        }
        if started {
            // Command is running: clear tracker and suppress suggestions (Tauri C mark).
            self.command_input_tracker = TerminalInputState::new();
            self.command_suggestions = None;
            self.command_suggestions_suppressed = true;
            self.command_suggestion_search_gen =
                self.command_suggestion_search_gen.saturating_add(1);
        }
        if finished {
            // Command finished: re-enable suggestion tracking (Tauri D mark).
            self.command_suggestions_suppressed = false;
            self.command_input_tracker = TerminalInputState::new();
            self.command_suggestions = None;
            self.command_suggestion_search_gen =
                self.command_suggestion_search_gen.saturating_add(1);
        }
        let _ = command_running;
    }

    pub(in crate::features) fn scroll_terminal_to_bottom(&mut self, cx: &mut Context<Self>) {
        if let Some(session_id) = self.active_session_id.clone() {
            if let Some(view) = self.terminal_views.get_mut(&session_id) {
                view.scroll_offset = 0;
                view.has_new_while_scrolled = false;
            }
        } else {
            self.terminal_scroll_offset = 0;
        }
        cx.notify();
    }

    pub(in crate::features) fn scroll_terminal_to_top(&mut self, cx: &mut Context<Self>) {
        let mut snapshot_request: Option<(String, usize)> = None;
        if let Some(session_id) = self.active_session_id.clone() {
            if let Some(view) = self.terminal_views.get_mut(&session_id) {
                view.scroll_offset = view.scrollback_len_for_ui();
                if view.scroll_offset > 0 {
                    snapshot_request = Some((session_id.clone(), view.scroll_offset));
                }
            }
        } else {
            self.terminal_scroll_offset = self.terminal_screen.scrollback_len();
        }
        if let Some((session_id, offset)) = snapshot_request {
            self.request_terminal_frame_snapshot_when_idle(&session_id, offset);
        }
        cx.notify();
    }

    pub(in crate::features) fn set_terminal_scroll_offset(
        &mut self,
        offset: usize,
        cx: &mut Context<Self>,
    ) {
        let mut snapshot_request: Option<(String, usize)> = None;
        if let Some(session_id) = self.active_session_id.clone() {
            if let Some(view) = self.terminal_views.get_mut(&session_id) {
                let max = view.scrollback_len_for_ui();
                view.scroll_offset = offset.min(max);
                if view.scroll_offset == 0 {
                    view.has_new_while_scrolled = false;
                } else {
                    snapshot_request = Some((session_id.clone(), view.scroll_offset));
                }
            }
        } else {
            let max = self.terminal_screen.scrollback_len();
            self.terminal_scroll_offset = offset.min(max);
        }
        if let Some((session_id, offset)) = snapshot_request {
            self.request_terminal_frame_snapshot_when_idle(&session_id, offset);
        }
        cx.notify();
    }

    pub(in crate::features) fn active_terminal_scroll_max(&self) -> usize {
        if let Some(session_id) = self.active_session_id.as_deref() {
            self.terminal_views
                .get(session_id)
                .map(|view| view.scrollback_len_for_ui())
                .unwrap_or(0)
        } else {
            self.terminal_screen.scrollback_len()
        }
    }

    pub(in crate::features) fn terminal_scroll_max_for_session(
        &self,
        session_id: Option<&str>,
    ) -> usize {
        if let Some(session_id) = session_id.filter(|id| !id.is_empty()) {
            self.terminal_views
                .get(session_id)
                .map(|view| view.scrollback_len_for_ui())
                .unwrap_or(0)
        } else {
            self.terminal_screen.scrollback_len()
        }
    }

    pub(in crate::features) fn set_terminal_scroll_offset_for_session(
        &mut self,
        session_id: Option<&str>,
        offset: usize,
        cx: &mut Context<Self>,
    ) {
        let mut snapshot_request: Option<(String, usize)> = None;
        if let Some(session_id) = session_id.filter(|id| !id.is_empty()) {
            if let Some(view) = self.terminal_views.get_mut(session_id) {
                let max = view.scrollback_len_for_ui();
                view.scroll_offset = offset.min(max);
                if view.scroll_offset == 0 {
                    view.has_new_while_scrolled = false;
                } else {
                    snapshot_request = Some((session_id.to_string(), view.scroll_offset));
                }
            }
        } else {
            let max = self.terminal_screen.scrollback_len();
            self.terminal_scroll_offset = offset.min(max);
        }
        if let Some((session_id, offset)) = snapshot_request {
            self.request_terminal_frame_snapshot_when_idle(&session_id, offset);
        }
        cx.notify();
    }

    /// Map a vertical pointer position (0..=1 top→bottom of track) to scroll_offset.
    /// Top of track = oldest history (max offset); bottom = live (0).

    pub(in crate::features) fn set_terminal_scroll_from_track_ratio(
        &mut self,
        ratio: f32,
        cx: &mut Context<Self>,
    ) {
        self.set_terminal_scroll_from_track_ratio_for_session(
            self.active_session_id.clone().as_deref(),
            ratio,
            cx,
        );
    }

    pub(in crate::features) fn set_terminal_scroll_from_track_ratio_for_session(
        &mut self,
        session_id: Option<&str>,
        ratio: f32,
        cx: &mut Context<Self>,
    ) {
        let max = self.terminal_scroll_max_for_session(session_id);
        if max == 0 {
            self.set_terminal_scroll_offset_for_session(session_id, 0, cx);
            return;
        }
        let ratio = ratio.clamp(0.0, 1.0);
        // ratio 0 (top) -> max, ratio 1 (bottom) -> 0
        let offset = ((1.0 - ratio) * max as f32).round() as usize;
        self.set_terminal_scroll_offset_for_session(session_id, offset.min(max), cx);
    }

    pub(in crate::features) fn begin_terminal_scrollbar_drag(
        &mut self,
        session_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.terminal_scrollbar_dragging = true;
        self.terminal_scrollbar_drag_session_id = session_id;
        cx.notify();
    }

    pub(in crate::features) fn update_terminal_scrollbar_drag(
        &mut self,
        event: &gpui::MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        if !self.terminal_scrollbar_dragging {
            return;
        }
        let drag_session_id = self.terminal_scrollbar_drag_session_id.as_deref();
        let Some(bounds) = self.terminal_surface_bounds_for_session(drag_session_id) else {
            return;
        };
        let ratio = terminal_scroll_track_ratio(bounds, event.position.y);
        let drag_session_id = self.terminal_scrollbar_drag_session_id.clone();
        self.set_terminal_scroll_from_track_ratio_for_session(
            drag_session_id.as_deref(),
            ratio,
            cx,
        );
    }

    pub(in crate::features) fn finish_terminal_scrollbar_drag(&mut self, cx: &mut Context<Self>) {
        if self.terminal_scrollbar_dragging {
            self.terminal_scrollbar_dragging = false;
            self.terminal_scrollbar_drag_session_id = None;
            cx.notify();
        }
    }

    pub(in crate::features) fn active_terminal_surface_bounds(
        &self,
    ) -> Option<gpui::Bounds<gpui::Pixels>> {
        self.terminal_surface_bounds_for_session(self.active_session_id.as_deref())
    }

    pub(in crate::features) fn terminal_surface_bounds_for_session(
        &self,
        session_id: Option<&str>,
    ) -> Option<gpui::Bounds<gpui::Pixels>> {
        if let Some(session_id) = session_id.filter(|id| !id.is_empty()) {
            self.terminal_session_surface_bounds
                .get(session_id)
                .copied()
                .or(self.terminal_surface_bounds)
        } else {
            self.terminal_surface_bounds
        }
    }

    pub(in crate::features) fn active_terminal_page_rows(&self) -> usize {
        // Prefer live screen rows when available; fall back to classic 24-row page.
        if let Some(session_id) = self.active_session_id.as_deref() {
            if let Some(view) = self.terminal_views.get(session_id) {
                let rows = view.viewport_rows_for_ui();
                if rows > 0 {
                    return rows;
                }
            }
        }
        let rows = self.terminal_snapshot_for_session(None, 0).lines.len();
        if rows > 0 { rows } else { 24 }
    }

    pub(in crate::features) fn desired_terminal_grid_size(&self) -> Option<(u16, u16)> {
        self.desired_terminal_resize_geometry()
            .map(|geometry| (geometry.cols, geometry.rows))
    }

    pub(in crate::features) fn desired_terminal_resize_geometry(
        &self,
    ) -> Option<TerminalResizeGeometry> {
        self.desired_terminal_resize_geometry_for_session_hint(self.active_session_id.as_deref())
    }

    pub(in crate::features) fn desired_terminal_resize_geometry_for_session_hint(
        &self,
        session_id: Option<&str>,
    ) -> Option<TerminalResizeGeometry> {
        let bounds = session_id
            .filter(|id| !id.is_empty())
            .and_then(|session_id| {
                self.terminal_session_surface_bounds
                    .get(session_id)
                    .copied()
            })
            .or_else(|| {
                self.active_session_id.as_deref().and_then(|session_id| {
                    self.terminal_session_surface_bounds
                        .get(session_id)
                        .copied()
                })
            })
            .or(self.terminal_surface_bounds)?;
        Some(self.terminal_resize_geometry_for_bounds(bounds))
    }

    pub(in crate::features) fn desired_terminal_grid_size_for_bounds(
        &self,
        bounds: gpui::Bounds<gpui::Pixels>,
    ) -> Option<(u16, u16)> {
        let geometry = self.terminal_resize_geometry_for_bounds(bounds);
        Some((geometry.cols, geometry.rows))
    }

    pub(in crate::features) fn terminal_resize_geometry_for_bounds(
        &self,
        bounds: gpui::Bounds<gpui::Pixels>,
    ) -> TerminalResizeGeometry {
        let (cell_w, cell_h) = self.terminal_cell_size();
        let pad = self.terminal_content_padding_px();
        let gutter = self.terminal_gutter_width_px();
        terminal_resize_geometry_for_bounds(bounds, cell_w, cell_h, pad, gutter)
    }

    pub(in crate::features) fn drive_terminal_resize(&mut self) -> bool {
        if let Some(last) = self.terminal_runtime.last_terminal_resize_at
            && last.elapsed() < Duration::from_millis(100)
        {
            return false;
        }
        let bounds = if let Some(session_id) = self.active_session_id.as_deref() {
            self.terminal_session_surface_bounds
                .get(session_id)
                .copied()
                .or(self.terminal_surface_bounds)
        } else {
            self.terminal_surface_bounds
        };
        let Some(bounds) = bounds else {
            return false;
        };
        self.resize_terminal_to_bounds_for_session(
            self.active_session_id.clone().as_deref(),
            bounds,
        )
    }

    pub(in crate::features) fn resize_all_known_terminal_surfaces(&mut self) -> bool {
        let mut dirty = false;
        if let Some(bounds) = self.terminal_surface_bounds {
            dirty |= self.resize_terminal_to_bounds_for_session(None, bounds);
        }
        let bounds_by_session = self
            .terminal_session_surface_bounds
            .iter()
            .map(|(session_id, bounds)| (session_id.clone(), *bounds))
            .collect::<Vec<_>>();
        for (session_id, bounds) in bounds_by_session {
            dirty |= self.resize_terminal_to_bounds_for_session(Some(&session_id), bounds);
        }
        dirty
    }

    pub(in crate::features) fn resize_terminal_to_bounds_for_session(
        &mut self,
        session_id: Option<&str>,
        bounds: gpui::Bounds<gpui::Pixels>,
    ) -> bool {
        let TerminalResizeGeometry {
            cols,
            rows,
            pixel_width,
            pixel_height,
        } = self.terminal_resize_geometry_for_bounds(bounds);
        if let Some(session_id) = session_id.filter(|id| !id.is_empty()) {
            let Some(view) = self.terminal_views.get_mut(session_id) else {
                return false;
            };
            let current_rows = view.screen.rows() as u16;
            let current_cols = view.screen.cols() as u16;
            let grid_changed = current_rows != rows || current_cols != cols;
            let backend_changed =
                view.backend_resize_changed(cols, rows, pixel_width, pixel_height);
            if !grid_changed && !backend_changed {
                return false;
            }
            if grid_changed {
                view.screen.resize(cols, rows);
                view.clamp_scroll_offset();
                self.terminal_frame_pipeline
                    .resize_session(session_id.to_string(), cols, rows);
            }
            if backend_changed {
                view.remember_backend_resize(cols, rows, pixel_width, pixel_height);
                let _ = self.session_manager.resize_with_pixels(
                    session_id,
                    cols,
                    rows,
                    pixel_width,
                    pixel_height,
                );
            }
        } else {
            let current_rows = self.terminal_screen.rows() as u16;
            let current_cols = self.terminal_screen.cols() as u16;
            if current_rows == rows && current_cols == cols {
                return false;
            }
            self.terminal_screen.resize(cols, rows);
        }
        self.terminal_runtime.last_terminal_resize_at = Some(Instant::now());
        true
    }

    /// Shift+PageUp/PageDown/Home/End (and Ctrl+Shift+Up/Down) navigate local scrollback
    /// without sending CSI sequences to the remote PTY — common terminal emulator UX.

    pub(in crate::features) fn handle_terminal_scroll_key(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let keystroke = &event.keystroke;
        let key = keystroke.key.as_str();
        let shift = keystroke.modifiers.shift;
        let control = keystroke.modifiers.control;
        let alt = keystroke.modifiers.alt;
        let platform = keystroke.modifiers.platform;
        let function = keystroke.modifiers.function;
        if alt || platform || function {
            return false;
        }

        let page = self.active_terminal_page_rows().max(1) as i32;
        if shift && !control {
            match key {
                "pageup" => {
                    self.scroll_terminal_by(page, cx);
                    return true;
                }
                "pagedown" => {
                    self.scroll_terminal_by(-page, cx);
                    return true;
                }
                "home" => {
                    self.scroll_terminal_to_top(cx);
                    return true;
                }
                "end" => {
                    self.scroll_terminal_to_bottom(cx);
                    return true;
                }
                _ => {}
            }
        }
        if shift && control {
            match key {
                "up" => {
                    self.scroll_terminal_by(1, cx);
                    return true;
                }
                "down" => {
                    self.scroll_terminal_by(-1, cx);
                    return true;
                }
                _ => {}
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Bounds, point, px, size};

    #[test]
    fn terminal_scroll_track_ratio_uses_bounds_origin_and_clamps() {
        let bounds = Bounds::new(point(px(10.), px(100.)), size(px(12.), px(200.)));

        assert_eq!(terminal_scroll_track_ratio(bounds, px(100.)), 0.0);
        assert_eq!(terminal_scroll_track_ratio(bounds, px(200.)), 0.5);
        assert_eq!(terminal_scroll_track_ratio(bounds, px(300.)), 1.0);
        assert_eq!(terminal_scroll_track_ratio(bounds, px(50.)), 0.0);
        assert_eq!(terminal_scroll_track_ratio(bounds, px(350.)), 1.0);
    }

    #[test]
    fn terminal_resize_geometry_keeps_usable_pixel_remainder() {
        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(812.), px(612.)));

        let geometry = terminal_resize_geometry_for_bounds(bounds, 10., 20., 8., 72.);

        assert_eq!(
            geometry,
            TerminalResizeGeometry {
                cols: 72,
                rows: 29,
                pixel_width: 724,
                pixel_height: 596,
            }
        );
    }

    #[test]
    fn terminal_resize_geometry_clamps_grid_but_keeps_nonzero_pixels() {
        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(10.), px(10.)));

        let geometry = terminal_resize_geometry_for_bounds(bounds, 10., 20., 8., 72.);

        assert_eq!(geometry.cols, 20);
        assert_eq!(geometry.rows, 4);
        assert_eq!(geometry.pixel_width, 200);
        assert_eq!(geometry.pixel_height, 80);
    }
}
