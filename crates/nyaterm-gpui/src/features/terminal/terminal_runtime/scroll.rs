use super::*;

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
        if delta_lines == 0 {
            return;
        }
        if let Some(session_id) = self.active_session_id.clone() {
            if let Some(view) = self.terminal_views.get_mut(&session_id) {
                let max = view.screen.scrollback_len();
                let next = if delta_lines > 0 {
                    view.scroll_offset.saturating_add(delta_lines as usize)
                } else {
                    view.scroll_offset.saturating_sub((-delta_lines) as usize)
                };
                view.scroll_offset = next.min(max);
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
        if self.active_terminal_scroll_offset() == 0 {
            if let Some(session_id) = self.active_session_id.clone() {
                if let Some(view) = self.terminal_views.get_mut(&session_id) {
                    view.has_new_while_scrolled = false;
                }
            }
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
                    self.active_session_id = Some(session_id.clone());
                }
                let text = nyaterm_core::format_local_terminal_drop_input(&path_strings);
                self.send_terminal_input(text.into_bytes(), cx);
                self.terminal_status =
                    format!("inserted {} path(s) into terminal", path_strings.len());
                cx.notify();
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
        if let Some(session_id) = self.active_session_id.clone() {
            if let Some(view) = self.terminal_views.get_mut(&session_id) {
                view.scroll_offset = view.screen.scrollback_len();
            }
        } else {
            self.terminal_scroll_offset = self.terminal_screen.scrollback_len();
        }
        cx.notify();
    }

    pub(in crate::features) fn set_terminal_scroll_offset(
        &mut self,
        offset: usize,
        cx: &mut Context<Self>,
    ) {
        if let Some(session_id) = self.active_session_id.clone() {
            if let Some(view) = self.terminal_views.get_mut(&session_id) {
                let max = view.screen.scrollback_len();
                view.scroll_offset = offset.min(max);
                if view.scroll_offset == 0 {
                    view.has_new_while_scrolled = false;
                }
            }
        } else {
            let max = self.terminal_screen.scrollback_len();
            self.terminal_scroll_offset = offset.min(max);
        }
        cx.notify();
    }

    pub(in crate::features) fn active_terminal_scroll_max(&self) -> usize {
        if let Some(session_id) = self.active_session_id.as_deref() {
            self.terminal_views
                .get(session_id)
                .map(|view| view.screen.scrollback_len())
                .unwrap_or(0)
        } else {
            self.terminal_screen.scrollback_len()
        }
    }

    /// Map a vertical pointer position (0..=1 top→bottom of track) to scroll_offset.
    /// Top of track = oldest history (max offset); bottom = live (0).

    pub(in crate::features) fn set_terminal_scroll_from_track_ratio(
        &mut self,
        ratio: f32,
        cx: &mut Context<Self>,
    ) {
        let max = self.active_terminal_scroll_max();
        if max == 0 {
            self.set_terminal_scroll_offset(0, cx);
            return;
        }
        let ratio = ratio.clamp(0.0, 1.0);
        // ratio 0 (top) -> max, ratio 1 (bottom) -> 0
        let offset = ((1.0 - ratio) * max as f32).round() as usize;
        self.set_terminal_scroll_offset(offset.min(max), cx);
    }

    pub(in crate::features) fn begin_terminal_scrollbar_drag(&mut self, cx: &mut Context<Self>) {
        self.terminal_scrollbar_dragging = true;
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
        let Some(bounds) = self.terminal_surface_bounds else {
            return;
        };
        let height = f32::from(bounds.size.height).max(1.0);
        let local_y = f32::from(event.position.y - bounds.origin.y);
        let ratio = (local_y / height).clamp(0.0, 1.0);
        self.set_terminal_scroll_from_track_ratio(ratio, cx);
    }

    pub(in crate::features) fn finish_terminal_scrollbar_drag(&mut self, cx: &mut Context<Self>) {
        if self.terminal_scrollbar_dragging {
            self.terminal_scrollbar_dragging = false;
            cx.notify();
        }
    }

    pub(in crate::features) fn active_terminal_page_rows(&self) -> usize {
        // Prefer live screen rows when available; fall back to classic 24-row page.
        if let Some(session_id) = self.active_session_id.as_deref() {
            if let Some(view) = self.terminal_views.get(session_id) {
                let rows = view.screen.viewport_snapshot(0).lines.len();
                if rows > 0 {
                    return rows;
                }
            }
        }
        let rows = self.terminal_screen.viewport_snapshot(0).lines.len();
        if rows > 0 { rows } else { 24 }
    }

    pub(in crate::features) fn desired_terminal_grid_size(&self) -> Option<(u16, u16)> {
        let bounds = self.terminal_surface_bounds?;
        let (cell_w, cell_h) = self.terminal_cell_size();
        let pad = self.terminal_content_padding_px();
        let gutter = self.terminal_gutter_width_px();
        let width = (f32::from(bounds.size.width) - pad * 2. - gutter).max(cell_w);
        let height = (f32::from(bounds.size.height) - pad * 2.).max(cell_h);
        let cols = (width / cell_w.max(1.)).floor().clamp(20., 500.) as u16;
        let rows = (height / cell_h.max(1.)).floor().clamp(4., 200.) as u16;
        Some((cols, rows))
    }

    pub(in crate::features) fn drive_terminal_resize(&mut self) -> bool {
        let Some((cols, rows)) = self.desired_terminal_grid_size() else {
            return false;
        };
        if let Some(last) = self.terminal_runtime.last_terminal_resize_at
            && last.elapsed() < Duration::from_millis(100)
        {
            return false;
        }
        if let Some(session_id) = self.active_session_id.clone() {
            let Some(view) = self.terminal_views.get_mut(&session_id) else {
                return false;
            };
            let current_rows = view.screen.rows() as u16;
            let current_cols = view.screen.cols() as u16;
            if current_rows == rows && current_cols == cols {
                return false;
            }
            view.screen.resize(cols, rows);
            view.clamp_scroll_offset();
            let _ = self.session_manager.resize(&session_id, cols, rows);
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
