use super::*;

impl NyaTermApp {
    pub(in crate::features) fn sync_terminal_scrollback_limits(&mut self) {
        let limit = self.settings.terminal_scrollback_lines.clamp(100, 100_000) as usize;
        self.terminal_screen.set_scrollback_limit(limit);
        for view in self.terminal_views.values_mut() {
            view.screen.set_scrollback_limit(limit);
            view.clamp_scroll_offset();
        }
        if self.terminal_scroll_offset > self.terminal_screen.scrollback_len() {
            self.terminal_scroll_offset = self.terminal_screen.scrollback_len();
        }
    }

    pub(in crate::features) fn terminal_scrollback_max_bytes(&self) -> usize {
        (self.settings.terminal_scrollback_lines.clamp(100, 100_000) as usize).saturating_mul(96)
    }

    pub(in crate::features) fn enforce_terminal_scrollback_limit(&mut self) {
        self.sync_terminal_scrollback_limits();
        let max_bytes = self.terminal_scrollback_max_bytes();
        trim_terminal_output_to(&mut self.terminal_output, max_bytes);
        for view in self.terminal_views.values_mut() {
            trim_terminal_output_to(&mut view.output, max_bytes);
        }
    }

    pub(in crate::features) fn append_terminal_bytes(&mut self, data: &[u8]) {
        let session_id = self.active_session_id.clone();
        self.append_terminal_bytes_for_session(session_id.as_deref(), data, false);
    }

    pub(in crate::features) fn append_terminal_log_for_session(
        &mut self,
        session_id: Option<&str>,
        text: &str,
        mark_unread: bool,
    ) {
        let mut shell_started = false;
        let mut shell_finished = false;
        let mut shell_running = false;
        let mut pending_cwd: Option<String> = None;

        if let Some(session_id) = session_id {
            let is_active = self.active_session_id.as_deref() == Some(session_id);
            let view = self
                .terminal_views
                .entry(session_id.to_string())
                .or_insert_with(TerminalViewState::new);
            let feed = view.protect_output_burst(text.as_bytes());
            view.append_bytes_unprotected(feed);
            if mark_unread && !is_active {
                view.has_unread = true;
            }
            if view.screen.take_visual_bell() {
                self.terminal_runtime.visual_bell_ticks = 4;
            }
            if let Some(title) = view.screen.take_window_title() {
                self.session_dynamic_titles
                    .insert(session_id.to_string(), title);
            }
            let (cmd_started, cmd_finished) = view.screen.take_shell_command_edges();
            let command_running = view.screen.command_running();
            shell_started |= cmd_started;
            shell_finished |= cmd_finished;
            shell_running = command_running;
            if let Some(cwd) = view.screen.take_cwd() {
                pending_cwd = Some(cwd);
            }
        } else {
            self.terminal_output.push_str(text);
            self.terminal_screen.advance(text.as_bytes());
            let max_bytes = self.terminal_scrollback_max_bytes();
            trim_terminal_output_to(&mut self.terminal_output, max_bytes);
            if self.terminal_screen.take_visual_bell() {
                self.terminal_runtime.visual_bell_ticks = 4;
            }
        }
        if shell_started || shell_finished {
            if let Some(session_id) = session_id {
                self.apply_shell_integration_edges(
                    session_id,
                    shell_started,
                    shell_finished,
                    shell_running,
                );
            }
        }
        if let (Some(session_id), Some(cwd)) = (session_id, pending_cwd) {
            self.apply_session_cwd(session_id, cwd);
        }
    }

    pub(in crate::features) fn append_terminal_bytes_for_session(
        &mut self,
        session_id: Option<&str>,
        data: &[u8],
        mark_unread: bool,
    ) {
        let mut shell_started = false;
        let mut shell_finished = false;
        let mut shell_running = false;
        let mut pending_cwd: Option<String> = None;

        if let Some(session_id) = session_id {
            let is_active = self.active_session_id.as_deref() == Some(session_id);
            let view = self
                .terminal_views
                .entry(session_id.to_string())
                .or_insert_with(TerminalViewState::new);
            let feed = view.protect_output_burst(data);
            view.append_bytes_unprotected(feed);
            if mark_unread && !is_active {
                view.has_unread = true;
            }
            if view.screen.take_visual_bell() {
                self.terminal_runtime.visual_bell_ticks = 4;
            }
            if let Some(title) = view.screen.take_window_title() {
                self.session_dynamic_titles
                    .insert(session_id.to_string(), title);
            }
            let (cmd_started, cmd_finished) = view.screen.take_shell_command_edges();
            let command_running = view.screen.command_running();
            shell_started |= cmd_started;
            shell_finished |= cmd_finished;
            shell_running = command_running;
            if let Some(cwd) = view.screen.take_cwd() {
                pending_cwd = Some(cwd);
            }
        } else {
            self.terminal_screen.advance(data);
            self.terminal_output
                .push_str(&String::from_utf8_lossy(data));
            let max_bytes = self.terminal_scrollback_max_bytes();
            trim_terminal_output_to(&mut self.terminal_output, max_bytes);
            if self.terminal_screen.take_visual_bell() {
                self.terminal_runtime.visual_bell_ticks = 4;
            }
        }
        if shell_started || shell_finished {
            if let Some(session_id) = session_id {
                self.apply_shell_integration_edges(
                    session_id,
                    shell_started,
                    shell_finished,
                    shell_running,
                );
            }
        }
        if let (Some(session_id), Some(cwd)) = (session_id, pending_cwd) {
            self.apply_session_cwd(session_id, cwd);
        }
    }

}
