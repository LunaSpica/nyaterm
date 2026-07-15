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

    pub(in crate::features) fn submit_terminal_frame_outputs(
        &self,
        outputs: Vec<(String, Vec<u8>)>,
    ) {
        if outputs.is_empty() {
            return;
        }
        let encoding = self.settings.interaction_default_encoding.clone();
        let scrollback_limit = self.terminal_scrollback_line_limit();
        let submissions = outputs
            .into_iter()
            .filter_map(|(session_id, data)| {
                (!data.is_empty()).then_some(TerminalFrameOutputSubmission {
                    session_id,
                    data,
                    encoding: encoding.clone(),
                    scrollback_limit,
                })
            })
            .collect::<Vec<_>>();
        self.terminal_frame_pipeline.submit_outputs(submissions);
    }

    pub(in crate::features) fn request_terminal_frame_snapshot(
        &mut self,
        session_id: &str,
        offset: usize,
    ) -> bool {
        if session_id.is_empty() || offset == 0 {
            return false;
        }
        let Some(view) = self.terminal_views.get_mut(session_id) else {
            return false;
        };
        if view.scrollback_snapshots.contains_key(&offset)
            || !view.pending_snapshot_offsets.insert(offset)
        {
            return false;
        }
        self.terminal_frame_pipeline.request_snapshot(
            session_id.to_string(),
            offset,
            self.settings.terminal_action_links_enabled,
            self.settings.terminal_action_links_matchers.clone(),
        );
        true
    }

    pub(in crate::features) fn request_terminal_frame_search(
        &mut self,
        session_id: &str,
        key: TerminalFrameSearchKey,
    ) -> bool {
        if session_id.is_empty() {
            return false;
        }
        let Some(view) = self.terminal_views.get_mut(session_id) else {
            return false;
        };
        if view.search_result.as_ref().is_some_and(|result| {
            terminal_frame_search_result_is_current(result, &key, view.screen_revision)
        }) || view.pending_search_key.as_ref() == Some(&key)
        {
            return false;
        }
        view.pending_search_key = Some(key.clone());
        self.terminal_frame_pipeline
            .request_search(session_id.to_string(), key);
        true
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

    pub(in crate::features) fn drive_terminal_render_requests(
        &mut self,
        allow_deferred_work: bool,
    ) -> bool {
        if !allow_deferred_work {
            return false;
        }
        let visible_session_ids = self.visible_terminal_session_ids();
        let snapshot_requests =
            terminal_frame_snapshot_request_candidates(&self.terminal_views, &visible_session_ids);
        let mut requested = false;
        for (session_id, offset) in snapshot_requests {
            requested |= self.request_terminal_frame_snapshot(&session_id, offset);
        }
        requested |= self.request_active_terminal_buffer_search();
        requested
    }

    pub(in crate::features) fn drain_terminal_frame_events(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let started_at = Instant::now();
        let mut dirty = false;
        let mut drained_events = 0usize;
        let mut output_events = 0usize;
        let mut coalesced_output_events = 0usize;
        let mut accepted_bytes = 0usize;
        let mut max_apply_duration = Duration::ZERO;

        self.fill_pending_terminal_frame_events();

        while drained_events < TERMINAL_FRAME_EVENT_DRAIN_BATCH {
            if self.pending_terminal_frame_events.is_empty() {
                if self.fill_pending_terminal_frame_events() == 0 {
                    break;
                }
            }

            let (frames, coalesced) =
                pop_terminal_frame_events_for_apply(&mut self.pending_terminal_frame_events);
            if frames.is_empty() {
                break;
            }
            coalesced_output_events = coalesced_output_events.saturating_add(coalesced);
            for frame in frames {
                if let TerminalFrameEvent::Output(output) = &frame {
                    output_events += 1;
                    accepted_bytes = accepted_bytes.saturating_add(output.accepted_bytes);
                }
                let apply_started_at = Instant::now();
                dirty |= self.apply_terminal_frame_event(frame, cx);
                max_apply_duration = max_apply_duration.max(apply_started_at.elapsed());
                drained_events += 1;

                if drained_events >= TERMINAL_FRAME_EVENT_DRAIN_BATCH
                    || started_at.elapsed() >= TERMINAL_FRAME_EVENT_DRAIN_WALL_BUDGET
                {
                    break;
                }
            }
            if drained_events >= TERMINAL_FRAME_EVENT_DRAIN_BATCH
                || started_at.elapsed() >= TERMINAL_FRAME_EVENT_DRAIN_WALL_BUDGET
            {
                break;
            }
        }

        if drained_events > 0 {
            self.terminal_runtime.last_terminal_frame_apply_at = Some(started_at);
        }
        let total_duration = started_at.elapsed();
        if (total_duration >= TERMINAL_FRAME_EVENT_DRAIN_SLOW_TOTAL
            || max_apply_duration >= TERMINAL_FRAME_EVENT_APPLY_SLOW)
            && self.should_log_slow_diagnostic("terminal_frame_event_drain", Instant::now())
        {
            tracing::warn!(
                diagnostic = "terminal_frame_event_drain",
                drained_events,
                output_events,
                coalesced_output_events,
                accepted_bytes,
                pending_events = self.pending_terminal_frame_events.len(),
                total_ms = total_duration.as_millis(),
                max_apply_ms = max_apply_duration.as_millis(),
                "slow terminal frame event drain"
            );
        }
        dirty
    }

    fn fill_pending_terminal_frame_events(&mut self) -> usize {
        let room = TERMINAL_FRAME_EVENT_DRAIN_BATCH
            .saturating_sub(self.pending_terminal_frame_events.len());
        self.terminal_frame_pipeline
            .drain_events_into(&mut self.pending_terminal_frame_events, room)
    }

    fn apply_terminal_frame_event(
        &mut self,
        event: TerminalFrameEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        match event {
            TerminalFrameEvent::Output(frame) => self.apply_terminal_output_frame(frame, cx),
            TerminalFrameEvent::Snapshot(snapshot) => self.apply_terminal_snapshot_frame(snapshot),
            TerminalFrameEvent::Search(search) => self.apply_terminal_search_frame(search),
            TerminalFrameEvent::BufferText(buffer) => {
                self.apply_terminal_buffer_text_frame(buffer, cx)
            }
        }
    }

    fn apply_terminal_output_frame(
        &mut self,
        frame: TerminalFrameOutputEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let TerminalFrameOutputEvent {
            session_id,
            visible_text,
            recording_text_bytes,
            snapshot,
            action_links,
            protocol_state,
            effects,
            command_running,
            accepted_bytes,
            skipped_output_bytes,
            revision,
            process_duration,
        } = frame;
        let is_active = self.active_session_id.as_deref() == Some(session_id.as_str());
        let is_visible = self.terminal_session_has_visible_surface(&session_id);
        let effects_need_ui_apply = terminal_effects_need_ui_apply(&effects);
        let view = self
            .terminal_views
            .entry(session_id.clone())
            .or_insert_with(TerminalViewState::new);
        let had_unread = view.has_unread;
        view.apply_terminal_frame_parts(
            &visible_text,
            snapshot,
            action_links,
            protocol_state,
            accepted_bytes,
            skipped_output_bytes,
            revision,
        );
        if !is_active {
            view.has_unread = true;
        }
        let unread_changed = !is_active && !had_unread;
        if effects_need_ui_apply {
            self.apply_terminal_effects(&session_id, effects, command_running, cx);
        }
        if process_duration >= Duration::from_millis(20)
            && self.should_log_slow_diagnostic("terminal_frame_processor", Instant::now())
        {
            tracing::warn!(
                diagnostic = "terminal_frame_processor",
                session_id = %session_id,
                accepted_bytes,
                skipped_output_bytes,
                visible_text_bytes = visible_text.len(),
                recording_text_bytes,
                process_ms = process_duration.as_millis(),
                "slow terminal frame processing"
            );
        }
        terminal_output_frame_needs_notify(is_visible, unread_changed, effects_need_ui_apply)
    }

    fn terminal_session_has_visible_surface(&self, session_id: &str) -> bool {
        if session_id.is_empty() || self.main_mode != MainMode::Workspace {
            return false;
        }
        self.visible_terminal_session_ids()
            .iter()
            .any(|id| *id == session_id)
    }

    fn visible_terminal_session_ids(&self) -> Vec<&str> {
        if self.main_mode != MainMode::Workspace {
            return Vec::new();
        }
        if let Some(root) = self.terminal_windows.as_ref()
            && matches!(root, TerminalWindowNode::Split { .. })
        {
            return terminal_window_node_visible_tab_ids(root);
        }
        if let Some(root) = self.workspace_split.as_ref() {
            return workspace_pane_node_visible_session_ids(root);
        }
        self.active_session_id.iter().map(String::as_str).collect()
    }

    fn apply_terminal_snapshot_frame(&mut self, frame: TerminalFrameSnapshotEvent) -> bool {
        let Some(view) = self.terminal_views.get_mut(&frame.session_id) else {
            return false;
        };
        view.pending_snapshot_offsets.remove(&frame.offset);
        view.scrollback_snapshots
            .insert(frame.offset, frame.snapshot.clone());
        if let Some(action_links) = frame.action_links {
            view.scrollback_action_links
                .insert(frame.offset, action_links);
        } else {
            view.scrollback_action_links.remove(&frame.offset);
        }
        while view.scrollback_snapshots.len() > 16 {
            let Some(drop_offset) = view
                .scrollback_snapshots
                .keys()
                .copied()
                .find(|offset| *offset != frame.offset)
            else {
                break;
            };
            view.scrollback_snapshots.remove(&drop_offset);
            view.scrollback_action_links.remove(&drop_offset);
        }
        if frame.process_duration >= Duration::from_millis(20)
            && self.should_log_slow_diagnostic("terminal_frame_snapshot", Instant::now())
        {
            tracing::warn!(
                diagnostic = "terminal_frame_snapshot",
                session_id = %frame.session_id,
                offset = frame.offset,
                revision = frame.revision,
                process_ms = frame.process_duration.as_millis(),
                "slow terminal frame snapshot"
            );
        }
        true
    }

    fn apply_terminal_search_frame(&mut self, frame: TerminalFrameSearchEvent) -> bool {
        let Some((current_revision, is_current_revision)) =
            self.terminal_views.get_mut(&frame.session_id).map(|view| {
                if view.pending_search_key.as_ref() == Some(&frame.result.key) {
                    view.pending_search_key = None;
                }
                let current_revision = view.screen_revision;
                let is_current_revision = frame.result.revision == current_revision;
                if is_current_revision {
                    view.search_result = Some(frame.result.clone());
                }
                (current_revision, is_current_revision)
            })
        else {
            return false;
        };
        if frame.process_duration >= Duration::from_millis(20)
            && self.should_log_slow_diagnostic("terminal_frame_search", Instant::now())
        {
            let match_count = frame
                .result
                .matches
                .as_ref()
                .map(|matches| matches.len())
                .unwrap_or(0);
            tracing::warn!(
                diagnostic = "terminal_frame_search",
                session_id = %frame.session_id,
                query_len = frame.result.key.query.len(),
                revision = frame.result.revision,
                current_revision,
                stale = !is_current_revision,
                match_count,
                process_ms = frame.process_duration.as_millis(),
                "slow terminal frame search"
            );
        }
        is_current_revision
    }

    fn apply_terminal_buffer_text_frame(
        &mut self,
        frame: TerminalFrameBufferTextEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let text_bytes = frame.text.len();
        if frame.text.trim().is_empty() {
            self.terminal_status = "terminal buffer is empty".to_string();
        } else {
            let char_count = frame.text.chars().count();
            cx.write_to_clipboard(ClipboardItem::new_string(frame.text));
            self.terminal_status = if frame.truncated {
                format!("copied terminal buffer tail ({char_count} chars)")
            } else {
                format!("copied terminal buffer ({char_count} chars)")
            };
        }
        if frame.process_duration >= Duration::from_millis(20)
            && self.should_log_slow_diagnostic("terminal_frame_buffer_text", Instant::now())
        {
            tracing::warn!(
                diagnostic = "terminal_frame_buffer_text",
                session_id = %frame.session_id,
                request_id = %frame.request_id,
                text_bytes,
                truncated = frame.truncated,
                process_ms = frame.process_duration.as_millis(),
                "slow terminal frame buffer text"
            );
        }
        true
    }

    pub(in crate::features) fn enforce_terminal_scrollback_limit(&mut self) {
        self.sync_terminal_scrollback_limits();
        let max_bytes = self.terminal_scrollback_max_bytes();
        trim_terminal_output_to(&mut self.terminal_output, max_bytes);
        let ui_output_tail_cap = max_bytes.min(TERMINAL_UI_OUTPUT_TAIL_CAP);
        for view in self.terminal_views.values_mut() {
            trim_terminal_output_to(&mut view.output, ui_output_tail_cap);
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

fn pop_terminal_frame_events_for_apply(
    events: &mut VecDeque<TerminalFrameEvent>,
) -> (Vec<TerminalFrameEvent>, usize) {
    let Some(first) = events.pop_front() else {
        return (Vec::new(), 0);
    };
    if !matches!(first, TerminalFrameEvent::Output(_)) {
        return (vec![first], 0);
    }

    let mut output_run = vec![first];
    while matches!(events.front(), Some(TerminalFrameEvent::Output(_))) {
        let Some(event) = events.pop_front() else {
            break;
        };
        output_run.push(event);
    }
    let original_count = output_run.len();
    let mut latest_by_session: HashMap<String, (usize, TerminalFrameEvent)> = HashMap::new();
    for (index, event) in output_run.into_iter().enumerate() {
        let TerminalFrameEvent::Output(frame) = event else {
            continue;
        };
        latest_by_session.insert(
            frame.session_id.clone(),
            (index, TerminalFrameEvent::Output(frame)),
        );
    }
    let mut latest = latest_by_session.into_values().collect::<Vec<_>>();
    latest.sort_by_key(|(index, _)| *index);
    let events = latest
        .into_iter()
        .map(|(_, event)| event)
        .collect::<Vec<_>>();
    let coalesced = original_count.saturating_sub(events.len());
    (events, coalesced)
}

fn terminal_frame_snapshot_request_candidates(
    terminal_views: &HashMap<String, TerminalViewState>,
    visible_session_ids: &[&str],
) -> Vec<(String, usize)> {
    terminal_views
        .iter()
        .filter_map(|(session_id, view)| {
            (view.scroll_offset > 0
                && visible_session_ids
                    .iter()
                    .any(|visible_id| *visible_id == session_id))
            .then(|| (session_id.clone(), view.scroll_offset))
        })
        .collect()
}

fn workspace_pane_node_visible_session_ids(root: &WorkspacePaneNode) -> Vec<&str> {
    let mut ids = Vec::new();
    collect_workspace_pane_node_visible_session_ids(root, &mut ids);
    ids
}

fn collect_workspace_pane_node_visible_session_ids<'a>(
    node: &'a WorkspacePaneNode,
    ids: &mut Vec<&'a str>,
) {
    match node {
        WorkspacePaneNode::Leaf { session_id, .. } => {
            ids.push(session_id.as_str());
        }
        WorkspacePaneNode::Split { first, second, .. } => {
            collect_workspace_pane_node_visible_session_ids(first, ids);
            collect_workspace_pane_node_visible_session_ids(second, ids);
        }
    }
}

#[cfg(test)]
mod frame_event_queue_tests {
    use super::*;

    fn output_frame(session_id: &str, revision: u64) -> TerminalFrameEvent {
        TerminalFrameEvent::Output(TerminalFrameOutputEvent {
            session_id: session_id.to_string(),
            visible_text: format!("rev-{revision}"),
            recording_text_bytes: 0,
            snapshot: nyaterm_terminal::TerminalScreen::default().viewport_snapshot(0),
            action_links: None,
            protocol_state: TerminalProtocolState::default(),
            effects: TerminalEffects::default(),
            command_running: false,
            accepted_bytes: revision as usize,
            skipped_output_bytes: 0,
            revision,
            process_duration: Duration::ZERO,
        })
    }

    fn buffer_text_frame(session_id: &str) -> TerminalFrameEvent {
        TerminalFrameEvent::BufferText(TerminalFrameBufferTextEvent {
            session_id: session_id.to_string(),
            request_id: "request".to_string(),
            text: "buffer".to_string(),
            truncated: false,
            process_duration: Duration::ZERO,
        })
    }

    #[test]
    fn terminal_frame_snapshot_requests_only_include_visible_scrolled_sessions() {
        let mut views = HashMap::new();
        let mut visible_scrolled = TerminalViewState::new();
        visible_scrolled.scroll_offset = 5;
        views.insert("visible-scrolled".to_string(), visible_scrolled);

        let mut hidden_scrolled = TerminalViewState::new();
        hidden_scrolled.scroll_offset = 7;
        views.insert("hidden-scrolled".to_string(), hidden_scrolled);

        let mut visible_at_bottom = TerminalViewState::new();
        visible_at_bottom.scroll_offset = 0;
        views.insert("visible-at-bottom".to_string(), visible_at_bottom);

        assert_eq!(
            terminal_frame_snapshot_request_candidates(
                &views,
                &["visible-scrolled", "visible-at-bottom"]
            ),
            vec![("visible-scrolled".to_string(), 5)]
        );
    }

    #[test]
    fn terminal_frame_apply_coalesces_consecutive_output_to_latest_per_session() {
        let mut events = VecDeque::from([
            output_frame("a", 1),
            output_frame("a", 2),
            output_frame("b", 3),
        ]);

        let (frames, coalesced) = pop_terminal_frame_events_for_apply(&mut events);

        assert_eq!(coalesced, 1);
        assert!(events.is_empty());
        assert_eq!(frames.len(), 2);
        assert!(matches!(
            &frames[0],
            TerminalFrameEvent::Output(frame) if frame.session_id == "a" && frame.revision == 2
        ));
        assert!(matches!(
            &frames[1],
            TerminalFrameEvent::Output(frame) if frame.session_id == "b" && frame.revision == 3
        ));
    }

    #[test]
    fn terminal_frame_apply_does_not_coalesce_across_non_output_events() {
        let mut events = VecDeque::from([
            output_frame("a", 1),
            buffer_text_frame("a"),
            output_frame("a", 2),
        ]);

        let (first, first_coalesced) = pop_terminal_frame_events_for_apply(&mut events);
        let (second, second_coalesced) = pop_terminal_frame_events_for_apply(&mut events);
        let (third, third_coalesced) = pop_terminal_frame_events_for_apply(&mut events);

        assert_eq!(first_coalesced, 0);
        assert!(matches!(
            &first[0],
            TerminalFrameEvent::Output(frame) if frame.revision == 1
        ));
        assert_eq!(second_coalesced, 0);
        assert!(matches!(second[0], TerminalFrameEvent::BufferText(_)));
        assert_eq!(third_coalesced, 0);
        assert!(matches!(
            &third[0],
            TerminalFrameEvent::Output(frame) if frame.revision == 2
        ));
        assert!(events.is_empty());
    }
}

const TERMINAL_FRAME_EVENT_DRAIN_BATCH: usize = 64;
const TERMINAL_FRAME_EVENT_DRAIN_WALL_BUDGET: Duration = Duration::from_millis(4);
const TERMINAL_FRAME_EVENT_DRAIN_SLOW_TOTAL: Duration = Duration::from_millis(12);
const TERMINAL_FRAME_EVENT_APPLY_SLOW: Duration = Duration::from_millis(8);

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

fn terminal_effects_need_ui_apply(effects: &TerminalEffects) -> bool {
    effects.bell
        || effects.title.is_some()
        || effects.reset_title
        || effects.cwd.is_some()
        || effects.shell_command_started
        || effects.shell_command_finished
        || !effects.pty_write.is_empty()
        || effects.clipboard_store.is_some()
        || !effects.clipboard_loads.is_empty()
}

fn terminal_output_frame_needs_notify(
    is_visible: bool,
    unread_changed: bool,
    effects_need_ui_apply: bool,
) -> bool {
    is_visible || unread_changed || effects_need_ui_apply
}

fn terminal_window_node_visible_tab_ids(root: &TerminalWindowNode) -> Vec<&str> {
    let mut ids = Vec::new();
    collect_terminal_window_node_visible_tab_ids(root, &mut ids);
    ids
}

fn collect_terminal_window_node_visible_tab_ids<'a>(
    node: &'a TerminalWindowNode,
    ids: &mut Vec<&'a str>,
) {
    match node {
        TerminalWindowNode::Leaf { active_tab_id, .. } => {
            if let Some(id) = active_tab_id.as_deref() {
                ids.push(id);
            }
        }
        TerminalWindowNode::Split { first, second, .. } => {
            collect_terminal_window_node_visible_tab_ids(first, ids);
            collect_terminal_window_node_visible_tab_ids(second, ids);
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
    fn terminal_effects_skip_ui_apply_for_plain_output() {
        assert!(!terminal_effects_need_ui_apply(&TerminalEffects::default()));

        let mut effects = TerminalEffects::default();
        effects.bell = true;
        assert!(terminal_effects_need_ui_apply(&effects));

        let mut effects = TerminalEffects::default();
        effects.pty_write.push(b"\x1b[6n".to_vec());
        assert!(terminal_effects_need_ui_apply(&effects));

        let mut effects = TerminalEffects::default();
        effects.shell_command_finished = true;
        assert!(terminal_effects_need_ui_apply(&effects));
    }

    #[test]
    fn terminal_output_frame_notify_tracks_visible_unread_or_effects() {
        assert!(!terminal_output_frame_needs_notify(false, false, false));
        assert!(terminal_output_frame_needs_notify(true, false, false));
        assert!(terminal_output_frame_needs_notify(false, true, false));
        assert!(terminal_output_frame_needs_notify(false, false, true));
    }

    #[test]
    fn terminal_window_visible_tab_ids_returns_leaf_active_tabs() {
        let root = TerminalWindowNode::Split {
            id: "split".to_string(),
            direction: WorkspaceSplitDirection::Vertical,
            ratio_percent: 50,
            first: Box::new(TerminalWindowNode::Leaf {
                id: "left".to_string(),
                tab_ids: vec!["a".to_string(), "b".to_string()],
                active_tab_id: Some("b".to_string()),
            }),
            second: Box::new(TerminalWindowNode::Leaf {
                id: "right".to_string(),
                tab_ids: vec!["c".to_string()],
                active_tab_id: Some("c".to_string()),
            }),
        };

        assert_eq!(terminal_window_node_visible_tab_ids(&root), vec!["b", "c"]);
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
