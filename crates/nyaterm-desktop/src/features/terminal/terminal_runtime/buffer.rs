use std::borrow::Cow;
use std::fmt::Write as _;

use super::*;

const MAX_OSC52_REPLY_CHARS: usize = 1_048_576;

impl NyaTermApp {
    pub(in crate::features) fn terminal_scrollback_line_limit(&self) -> usize {
        self.settings.terminal_scrollback_lines.clamp(100, 100_000) as usize
    }

    pub(in crate::features) fn sync_terminal_scrollback_limits(&mut self) {
        let limit = self.terminal_scrollback_line_limit();
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
        self.terminal_scrollback_line_limit().saturating_mul(96)
    }

    pub(in crate::features) fn submit_terminal_frame_output(
        &self,
        session_id: &str,
        data: Vec<u8>,
    ) {
        self.terminal_frame_pipeline.submit_output(
            session_id.to_string(),
            data,
            self.settings.interaction_default_encoding.clone(),
            self.terminal_scrollback_line_limit(),
        );
    }

    pub(in crate::features) fn seed_terminal_frame_session(
        &self,
        session_id: &str,
        output: String,
    ) {
        self.terminal_frame_pipeline.seed_session(
            session_id.to_string(),
            output,
            self.settings.interaction_default_encoding.clone(),
            self.terminal_scrollback_line_limit(),
        );
    }

    pub(in crate::features) fn drain_terminal_frame_events(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut dirty = false;
        for frame in self.terminal_frame_pipeline.drain_events(256) {
            dirty |= self.apply_terminal_frame_event(frame, cx);
        }
        dirty
    }

    fn apply_terminal_frame_event(
        &mut self,
        frame: TerminalFrameEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let session_id = frame.session_id.clone();
        let is_active = self.active_session_id.as_deref() == Some(session_id.as_str());
        if !frame.recording_text.is_empty() {
            self.recording_manager
                .write_output(&session_id, &frame.recording_text);
        }
        let view = self
            .terminal_views
            .entry(session_id.clone())
            .or_insert_with(TerminalViewState::new);
        view.apply_terminal_frame(&frame);
        if !is_active {
            view.has_unread = true;
        }
        self.apply_terminal_effects(&session_id, frame.effects, frame.command_running, cx);
        if is_active && !frame.visible_text.is_empty() {
            self.feed_credential_autofill_output(&frame.visible_text, cx);
        }
        if frame.process_duration >= Duration::from_millis(20)
            && self.should_log_slow_diagnostic("terminal_frame_processor", Instant::now())
        {
            tracing::warn!(
                diagnostic = "terminal_frame_processor",
                session_id = %session_id,
                accepted_bytes = frame.accepted_bytes,
                skipped_output_bytes = frame.skipped_output_bytes,
                visible_text_bytes = frame.visible_text.len(),
                recording_text_bytes = frame.recording_text.len(),
                process_ms = frame.process_duration.as_millis(),
                "slow terminal frame processing"
            );
        }
        true
    }

    pub(in crate::features) fn enforce_terminal_scrollback_limit(&mut self) {
        self.sync_terminal_scrollback_limits();
        let max_bytes = self.terminal_scrollback_max_bytes();
        trim_terminal_output_to(&mut self.terminal_output, max_bytes);
        for view in self.terminal_views.values_mut() {
            trim_terminal_output_to(&mut view.output, max_bytes);
        }
    }

    pub(in crate::features) fn decode_session_output_for_recording(
        &mut self,
        session_id: &str,
        data: &[u8],
    ) -> String {
        let encoding = self.settings.interaction_default_encoding.clone();
        let view = self
            .terminal_views
            .entry(session_id.to_string())
            .or_insert_with(TerminalViewState::new);
        view.recording_decoder.set_encoding(&encoding);
        view.recording_decoder.decode_output_text(data)
    }

    pub(in crate::features) fn encode_visible_terminal_text_for_output(
        &self,
        session_id: &str,
        text: &str,
    ) -> Vec<u8> {
        self.encode_session_outgoing(session_id, text.as_bytes())
    }

    pub(in crate::features) fn append_terminal_log_for_session(
        &mut self,
        session_id: Option<&str>,
        text: &str,
        mark_unread: bool,
    ) {
        self.append_terminal_log_for_session_with_context(session_id, text, mark_unread, None);
    }

    pub(in crate::features) fn append_terminal_log_for_session_with_context(
        &mut self,
        session_id: Option<&str>,
        text: &str,
        mark_unread: bool,
        mut cx: Option<&mut Context<Self>>,
    ) {
        if text.is_empty() {
            return;
        }
        let text = terminal_local_log_text(text);
        let mut shell_started = false;
        let mut shell_finished = false;
        let mut shell_running = false;
        let mut pending_cwd: Option<String> = None;
        let mut pending_pty_writes: Vec<Vec<u8>>;
        let mut clipboard_store: Option<String>;
        let mut clipboard_loads;

        if let Some(session_id) = session_id {
            let is_active = self.active_session_id.as_deref() == Some(session_id);
            let encoding = self.settings.interaction_default_encoding.clone();
            let view = self
                .terminal_views
                .entry(session_id.to_string())
                .or_insert_with(TerminalViewState::new);
            view.set_encoding(&encoding);
            view.append_text(text.as_ref());
            if mark_unread && !is_active {
                view.has_unread = true;
            }
            let effects = view.screen.take_effects();
            pending_pty_writes = effects.pty_write;
            clipboard_store = effects.clipboard_store;
            clipboard_loads = effects.clipboard_loads;
            if effects.bell {
                self.terminal_runtime.visual_bell_ticks = 4;
            }
            if let Some(title) = effects.title {
                self.session_dynamic_titles
                    .insert(session_id.to_string(), title);
            }
            if effects.reset_title {
                self.session_dynamic_titles.remove(session_id);
            }
            let command_running = view.screen.command_running();
            shell_started |= effects.shell_command_started;
            shell_finished |= effects.shell_command_finished;
            shell_running = command_running;
            if let Some(cwd) = effects.cwd {
                pending_cwd = Some(cwd);
            }
        } else {
            self.terminal_screen.advance_decoded_text(text.as_ref());
            self.terminal_output.push_str(text.as_ref());
            let max_bytes = self.terminal_scrollback_max_bytes();
            trim_terminal_output_to(&mut self.terminal_output, max_bytes);
            let effects = self.terminal_screen.take_effects();
            pending_pty_writes = effects.pty_write;
            clipboard_store = effects.clipboard_store;
            clipboard_loads = effects.clipboard_loads;
            if effects.bell {
                self.terminal_runtime.visual_bell_ticks = 4;
            }
        }

        self.handle_terminal_clipboard_effects(
            &mut clipboard_store,
            &mut clipboard_loads,
            &mut pending_pty_writes,
            cx.as_deref_mut(),
        );

        if let Some(session_id) = session_id {
            self.write_terminal_pty_responses(session_id, pending_pty_writes);
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
        mut cx: Option<&mut Context<Self>>,
    ) {
        let mut shell_started = false;
        let mut shell_finished = false;
        let mut shell_running = false;
        let mut pending_cwd: Option<String> = None;
        let mut pending_pty_writes: Vec<Vec<u8>>;
        let mut clipboard_store: Option<String>;
        let mut clipboard_loads;

        if let Some(session_id) = session_id {
            let is_active = self.active_session_id.as_deref() == Some(session_id);
            let encoding = self.settings.interaction_default_encoding.clone();
            let view = self
                .terminal_views
                .entry(session_id.to_string())
                .or_insert_with(TerminalViewState::new);
            view.screen.set_encoding(&encoding);
            view.output_decoder.set_encoding(&encoding);
            let feed = view.protect_output_burst(data);
            view.append_bytes_unprotected(feed);
            if mark_unread && !is_active {
                view.has_unread = true;
            }
            let effects = view.screen.take_effects();
            pending_pty_writes = effects.pty_write;
            clipboard_store = effects.clipboard_store;
            clipboard_loads = effects.clipboard_loads;
            if effects.bell {
                self.terminal_runtime.visual_bell_ticks = 4;
            }
            if let Some(title) = effects.title {
                self.session_dynamic_titles
                    .insert(session_id.to_string(), title);
            }
            if effects.reset_title {
                self.session_dynamic_titles.remove(session_id);
            }
            let command_running = view.screen.command_running();
            shell_started |= effects.shell_command_started;
            shell_finished |= effects.shell_command_finished;
            shell_running = command_running;
            if let Some(cwd) = effects.cwd {
                pending_cwd = Some(cwd);
            }
        } else {
            let encoding = self.settings.interaction_default_encoding.clone();
            self.terminal_screen.set_encoding(&encoding);
            self.terminal_output_decoder.set_encoding(&encoding);
            let (feed, _skipped) = protect_terminal_output_burst(
                &mut self.terminal_screen,
                &mut self.terminal_output_decoder,
                data,
            );
            self.terminal_screen.advance(feed);
            self.terminal_output
                .push_str(&self.terminal_output_decoder.decode_output_text(feed));
            let max_bytes = self.terminal_scrollback_max_bytes();
            trim_terminal_output_to(&mut self.terminal_output, max_bytes);
            let effects = self.terminal_screen.take_effects();
            pending_pty_writes = effects.pty_write;
            clipboard_store = effects.clipboard_store;
            clipboard_loads = effects.clipboard_loads;
            if effects.bell {
                self.terminal_runtime.visual_bell_ticks = 4;
            }
        }

        self.handle_terminal_clipboard_effects(
            &mut clipboard_store,
            &mut clipboard_loads,
            &mut pending_pty_writes,
            cx.as_deref_mut(),
        );

        if let Some(session_id) = session_id {
            self.write_terminal_pty_responses(session_id, pending_pty_writes);
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

    fn write_terminal_pty_responses(&mut self, session_id: &str, responses: Vec<Vec<u8>>) {
        for response in responses {
            if response.is_empty() {
                continue;
            }
            if let Err(error) = self.write_session_protocol_response(session_id, &response) {
                self.terminal_status = format!("terminal response failed: {error}");
                break;
            }
        }
    }

    fn apply_terminal_effects(
        &mut self,
        session_id: &str,
        effects: TerminalEffects,
        command_running: bool,
        cx: &mut Context<Self>,
    ) {
        let mut pending_pty_writes = effects.pty_write;
        let mut clipboard_store = effects.clipboard_store;
        let mut clipboard_loads = effects.clipboard_loads;
        if effects.bell {
            self.terminal_runtime.visual_bell_ticks = 4;
        }
        if let Some(title) = effects.title {
            self.session_dynamic_titles
                .insert(session_id.to_string(), title);
        }
        if effects.reset_title {
            self.session_dynamic_titles.remove(session_id);
        }
        self.handle_terminal_clipboard_effects(
            &mut clipboard_store,
            &mut clipboard_loads,
            &mut pending_pty_writes,
            Some(cx),
        );
        self.write_terminal_pty_responses(session_id, pending_pty_writes);
        if effects.shell_command_started || effects.shell_command_finished {
            self.apply_shell_integration_edges(
                session_id,
                effects.shell_command_started,
                effects.shell_command_finished,
                command_running,
            );
        }
        if let Some(cwd) = effects.cwd {
            self.apply_session_cwd(session_id, cwd);
        }
    }

    fn handle_terminal_clipboard_effects(
        &mut self,
        clipboard_store: &mut Option<String>,
        clipboard_loads: &mut Vec<std::sync::Arc<dyn Fn(&str) -> String + Sync + Send + 'static>>,
        pending_pty_writes: &mut Vec<Vec<u8>>,
        cx: Option<&mut Context<Self>>,
    ) {
        if let Some(cx) = cx {
            if let Some(text) = clipboard_store.take() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                self.terminal_status = "OSC 52 clipboard updated".to_string();
            }
            if !clipboard_loads.is_empty() {
                let clipboard_text = cx
                    .read_from_clipboard()
                    .and_then(|item| item.text())
                    .unwrap_or_default();
                queue_osc52_clipboard_load_replies(
                    clipboard_loads,
                    &clipboard_text,
                    pending_pty_writes,
                );
            }
        } else {
            if clipboard_store.take().is_some() {
                self.terminal_status =
                    "OSC 52 clipboard update skipped: UI unavailable".to_string();
            }
            if !clipboard_loads.is_empty() {
                queue_osc52_clipboard_load_replies(clipboard_loads, "", pending_pty_writes);
            }
        }
    }
}

fn terminal_local_log_text(text: &str) -> Cow<'_, str> {
    if !text.chars().any(terminal_local_log_control_needs_escape) {
        return Cow::Borrowed(text);
    }

    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\n' | '\r' | '\t' => out.push(ch),
            '\x1b' => out.push_str("\\x1b"),
            ch if ch.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    Cow::Owned(out)
}

fn terminal_local_log_control_needs_escape(ch: char) -> bool {
    ch.is_control() && !matches!(ch, '\n' | '\r' | '\t')
}

fn limit_osc52_clipboard_reply_text(text: &str) -> std::borrow::Cow<'_, str> {
    match text.char_indices().nth(MAX_OSC52_REPLY_CHARS) {
        Some((boundary, _)) => std::borrow::Cow::Owned(text[..boundary].to_string()),
        None => std::borrow::Cow::Borrowed(text),
    }
}

fn queue_osc52_clipboard_load_replies(
    clipboard_loads: &mut Vec<std::sync::Arc<dyn Fn(&str) -> String + Sync + Send + 'static>>,
    clipboard_text: &str,
    pending_pty_writes: &mut Vec<Vec<u8>>,
) {
    let clipboard_text = limit_osc52_clipboard_reply_text(clipboard_text);
    for formatter in clipboard_loads.drain(..) {
        let reply = formatter(clipboard_text.as_ref());
        if !reply.is_empty() {
            pending_pty_writes.push(reply.into_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn osc52_clipboard_reply_limit_borrows_small_text() {
        let text = "small clipboard";
        let limited = limit_osc52_clipboard_reply_text(text);
        assert!(matches!(limited, std::borrow::Cow::Borrowed(_)));
        assert_eq!(limited.as_ref(), text);
    }

    #[test]
    fn osc52_clipboard_reply_limit_preserves_utf8_boundary() {
        let text = format!("{}界", "好".repeat(MAX_OSC52_REPLY_CHARS));
        let limited = limit_osc52_clipboard_reply_text(&text);
        assert!(matches!(limited, std::borrow::Cow::Owned(_)));
        assert_eq!(limited.chars().count(), MAX_OSC52_REPLY_CHARS);
        assert!(limited.chars().all(|ch| ch == '好'));
    }

    #[test]
    fn osc52_clipboard_load_reply_uses_empty_text_when_clipboard_unavailable() {
        let mut formatters: Vec<Arc<dyn Fn(&str) -> String + Sync + Send + 'static>> =
            vec![Arc::new(|text| format!("reply:{text}"))];
        let mut replies = Vec::new();

        queue_osc52_clipboard_load_replies(&mut formatters, "", &mut replies);

        assert!(formatters.is_empty());
        assert_eq!(replies, vec![b"reply:".to_vec()]);
    }

    #[test]
    fn terminal_local_log_text_preserves_framing_but_escapes_controls() {
        let text = "\n# started evil\x1b]52;c;AAAA\x07\tpath\r\n";
        let escaped = terminal_local_log_text(text);

        assert_eq!(
            escaped.as_ref(),
            "\n# started evil\\x1b]52;c;AAAA\\u{7}\tpath\r\n"
        );
        assert!(!escaped.contains('\x1b'));
        assert!(!escaped.contains('\x07'));
        assert!(escaped.starts_with('\n'));
        assert!(escaped.ends_with("\r\n"));
    }
}
