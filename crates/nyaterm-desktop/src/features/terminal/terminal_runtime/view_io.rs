use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MouseReportWriteResult {
    NotHandled,
    Sent,
    Failed,
}

const TERMINAL_INPUT_SLOW_THRESHOLD: Duration = Duration::from_millis(12);
const TERMINAL_SCROLL_PAINT_SLOW_TOTAL: Duration = Duration::from_millis(16);
const TERMINAL_SCROLL_PAINT_SLOW_STAGE: Duration = Duration::from_millis(8);
const TERMINAL_SCROLL_POSITION_NOTIFY_SLOW: Duration = Duration::from_millis(8);
#[cfg(test)]
const TERMINAL_SCROLL_RETAINED_WINDOW_MIN_EXTRA_ROWS: usize = 32;
#[cfg(test)]
const TERMINAL_SCROLL_RETAINED_WINDOW_MAX_EXTRA_ROWS: usize = 192;

#[cfg(test)]
type TerminalSnapshotRowSlice = std::sync::Arc<nyaterm_terminal::TerminalSnapshotRow>;

#[cfg(test)]
fn terminal_paint_snapshot_for_view(
    view: Option<&TerminalViewState>,
    offset: usize,
    retained_surface_snapshot: Option<std::sync::Arc<TerminalSnapshot>>,
) -> Option<std::sync::Arc<TerminalSnapshot>> {
    let Some(view) = view else {
        return (offset == 0).then(|| retained_surface_snapshot).flatten();
    };
    if offset == 0 {
        return view.frame_snapshot.clone();
    }
    view.scrollback_snapshots
        .get(&offset)
        .cloned()
        .or(retained_surface_snapshot)
}

fn terminal_cached_scrollback_snapshot_covering_display_offset(
    view: &TerminalViewState,
    display_offset: usize,
    viewport_rows: usize,
) -> Option<std::sync::Arc<TerminalSnapshot>> {
    if display_offset == 0 {
        return None;
    }
    if let Some(snapshot) = view.scrollback_snapshots.get(&display_offset) {
        return Some(snapshot.clone());
    }
    let scrollback_len = view.scrollback_len_for_ui();
    view.scrollback_snapshots
        .values()
        .filter(|snapshot| {
            terminal_snapshot_covers_display_offset(
                snapshot.as_ref(),
                display_offset,
                viewport_rows,
                scrollback_len,
            )
        })
        .min_by_key(|snapshot| snapshot.display_offset.abs_diff(display_offset))
        .cloned()
}

fn terminal_retained_snapshot_matches_view(
    snapshot: &TerminalSnapshot,
    view: &TerminalViewState,
    display_offset: usize,
    viewport_rows: usize,
) -> bool {
    let scrollback_len = view.scrollback_len_for_ui();
    terminal_snapshot_matches_grid_geometry(snapshot, view.screen.cols(), view.screen.rows())
        && snapshot.scrollback_len == scrollback_len
        && terminal_snapshot_covers_display_offset(
            snapshot,
            display_offset,
            viewport_rows,
            scrollback_len,
        )
}

fn terminal_paint_window_snapshot_for_view(
    view: Option<&TerminalViewState>,
    display_offset: usize,
    viewport_rows: usize,
    retained_surface_snapshot: Option<std::sync::Arc<TerminalSnapshot>>,
) -> Option<std::sync::Arc<TerminalSnapshot>> {
    if view.is_some_and(|view| view.grid_resize_pending) {
        return retained_surface_snapshot;
    }
    if display_offset == 0 {
        let Some(view) = view else {
            return retained_surface_snapshot;
        };
        let scrollback_len = view.scrollback_len_for_ui();
        if let Some(snapshot) = view.frame_snapshot.as_ref()
            && terminal_snapshot_matches_grid_geometry(
                snapshot.as_ref(),
                view.screen.cols(),
                view.screen.rows(),
            )
            && terminal_snapshot_covers_display_offset(
                snapshot.as_ref(),
                display_offset,
                viewport_rows,
                scrollback_len,
            )
        {
            return Some(snapshot.clone());
        }
        return retained_surface_snapshot.filter(|snapshot| {
            terminal_snapshot_matches_grid_geometry(
                snapshot.as_ref(),
                view.screen.cols(),
                view.screen.rows(),
            ) && terminal_snapshot_covers_display_offset(
                snapshot.as_ref(),
                display_offset,
                viewport_rows,
                scrollback_len,
            )
        });
    }
    if let Some(snapshot) = retained_surface_snapshot {
        return Some(snapshot);
    }
    let view = view?;
    terminal_cached_scrollback_snapshot_covering_display_offset(view, display_offset, viewport_rows)
}

#[cfg(test)]
fn terminal_scroll_retained_window_extra_rows(viewport_rows: usize) -> usize {
    viewport_rows
        .saturating_mul(2)
        .max(TERMINAL_SCROLL_RETAINED_WINDOW_MIN_EXTRA_ROWS)
        .min(TERMINAL_SCROLL_RETAINED_WINDOW_MAX_EXTRA_ROWS)
}

pub(in crate::features) fn terminal_visual_display_offset(
    target_offset: usize,
    _residual_lines: f32,
    max_offset: usize,
) -> usize {
    // Keep fractional wheel/trackpad movement as a visual transform only.
    // Switching the text snapshot window at half-line boundaries causes the
    // surface to wait on a different scrollback snapshot and reads as flicker.
    if max_offset == 0 {
        return 0;
    }
    target_offset.min(max_offset)
}

fn terminal_scroll_snapshot_request_offset(
    target_offset: usize,
    residual_lines: f32,
    max_offset: usize,
) -> Option<usize> {
    let display_offset = terminal_visual_display_offset(target_offset, residual_lines, max_offset);
    (display_offset > 0).then_some(display_offset)
}

fn terminal_cursor_visible_for_display_offset(
    is_active: bool,
    is_disconnected: bool,
    display_offset: usize,
    remote_cursor_visible: bool,
    blink_enabled: bool,
    cursor_blink_on: bool,
) -> bool {
    is_active
        && !is_disconnected
        && display_offset == 0
        && remote_cursor_visible
        && (!blink_enabled || cursor_blink_on)
}

fn terminal_session_write_failure_safe_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x1b' => out.push_str("\\x1b"),
            ch if ch.is_control() => out.push_str(&format!("\\u{{{:x}}}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

fn terminal_session_write_failure_log(context: &str, error: &str) -> String {
    let safe_error = terminal_session_write_failure_safe_text(error);
    format!("\n# session write failed ({context}): {safe_error}\n")
}

#[cfg(test)]
fn terminal_snapshot_with_newer_edge_row(
    base: std::sync::Arc<TerminalSnapshot>,
    newer: std::sync::Arc<TerminalSnapshot>,
) -> std::sync::Arc<TerminalSnapshot> {
    if base.cols == 0 || base.row_count() == 0 || base.cols != newer.cols || newer.row_count() == 0
    {
        return base;
    }
    let mut rows = base.rows().to_vec();
    rows.push(newer.rows().last().unwrap().clone());
    std::sync::Arc::new(TerminalSnapshot::from_rows(
        nyaterm_terminal::TerminalSnapshotMeta {
            cols: base.cols,
            viewport_rows: base.viewport_rows,
            cursor: base.cursor.clone(),
            selection: base.selection.clone(),
            scrollback_len: base.scrollback_len,
            total_rows: base.total_rows.saturating_add(1),
            display_offset: base.display_offset,
            images: base.images.clone(),
        },
        rows,
    ))
}

#[cfg(test)]
fn terminal_snapshot_with_retained_scroll_window(
    view: &TerminalViewState,
    base: std::sync::Arc<TerminalSnapshot>,
    display_offset: usize,
    viewport_rows: usize,
    scrollback_len: usize,
) -> std::sync::Arc<TerminalSnapshot> {
    if base.cols == 0 || base.row_count() == 0 {
        return base;
    }
    let extra = terminal_scroll_retained_window_extra_rows(viewport_rows);
    let older_count = scrollback_len.saturating_sub(display_offset).min(extra);
    let newer_count = display_offset.min(extra);
    if older_count == 0 && newer_count == 0 {
        return base;
    }
    let older_rows = if older_count == 0 {
        Vec::new()
    } else {
        terminal_snapshot_older_row_slices(&view.screen, display_offset, older_count)
    };
    let older_count = older_rows.len();
    let mut rows = Vec::with_capacity(older_count + base.row_count() + newer_count);
    rows.extend(older_rows);
    rows.extend(base.rows().iter().cloned());
    if newer_count > 0 {
        rows.extend(terminal_snapshot_newer_row_slices(
            &view.screen,
            display_offset,
            newer_count,
        ));
    }
    let images = view
        .screen
        .viewport_snapshot(display_offset)
        .images
        .into_iter()
        .filter(|image| image.row < viewport_rows)
        .map(|mut image| {
            image.row = image.row.saturating_add(older_count);
            image
        })
        .collect();
    std::sync::Arc::new(TerminalSnapshot::from_rows(
        nyaterm_terminal::TerminalSnapshotMeta {
            cols: base.cols,
            viewport_rows: base.viewport_rows,
            cursor: base.cursor.clone(),
            selection: base.selection.clone(),
            scrollback_len: base.scrollback_len,
            total_rows: base.total_rows.saturating_add(newer_count),
            display_offset: base.display_offset,
            images,
        },
        rows,
    ))
}

#[cfg(test)]
fn terminal_snapshot_older_row_slices(
    screen: &TerminalScreen,
    display_offset: usize,
    row_count: usize,
) -> Vec<TerminalSnapshotRowSlice> {
    let mut rows = Vec::new();
    let mut remaining = row_count;
    while remaining > 0 {
        let snapshot = screen.viewport_snapshot(display_offset.saturating_add(remaining));
        if snapshot.row_count() == 0 {
            break;
        }
        let take = remaining.min(snapshot.row_count());
        rows.extend(terminal_snapshot_row_slices(&snapshot, 0, take));
        remaining = remaining.saturating_sub(take);
    }
    rows
}

#[cfg(test)]
fn terminal_snapshot_newer_row_slices(
    screen: &TerminalScreen,
    display_offset: usize,
    row_count: usize,
) -> Vec<TerminalSnapshotRowSlice> {
    let mut rows = Vec::new();
    let mut consumed = 0usize;
    let viewport_rows = screen.viewport_snapshot(display_offset).row_count().max(1);
    while consumed < row_count {
        let remaining = row_count - consumed;
        let take = remaining.min(viewport_rows);
        let offset_delta = consumed.saturating_add(take);
        let snapshot = screen.viewport_snapshot(display_offset.saturating_sub(offset_delta));
        if snapshot.row_count() == 0 {
            break;
        }
        let take = take.min(snapshot.row_count());
        rows.extend(terminal_snapshot_row_slices(
            &snapshot,
            snapshot.row_count().saturating_sub(take),
            take,
        ));
        consumed = consumed.saturating_add(take);
    }
    rows
}

#[cfg(test)]
fn terminal_snapshot_row_slices(
    snapshot: &TerminalSnapshot,
    start_row: usize,
    row_count: usize,
) -> Vec<TerminalSnapshotRowSlice> {
    if row_count == 0 || start_row >= snapshot.row_count() {
        return Vec::new();
    }
    let end_row = start_row
        .saturating_add(row_count)
        .min(snapshot.row_count());
    (start_row..end_row)
        .filter_map(|row| terminal_snapshot_row_slice(snapshot, row))
        .collect()
}

#[cfg(test)]
fn terminal_snapshot_row_slice(
    snapshot: &TerminalSnapshot,
    row: usize,
) -> Option<TerminalSnapshotRowSlice> {
    snapshot.rows().get(row).cloned()
}

fn terminal_scroll_text_first_decorations(
    snapshot: &TerminalSnapshot,
    search_matches: Option<&[TerminalBufferMatch]>,
    frame_action_links: Option<&TerminalFrameActionLinks>,
    include_action_links: bool,
    include_hyperlinks: bool,
    include_command_marks: bool,
) -> Vec<TerminalLineDecorations> {
    let has_command_marks =
        include_command_marks && snapshot.rows().iter().any(|row| row.command_mark.is_some());
    let mut search_ranges_by_line: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
    if let Some(search_matches) = search_matches {
        let (abs_start, abs_end) =
            crate::features::terminal_surface::terminal_snapshot_absolute_range(snapshot);
        for search_match in search_matches {
            let abs = search_match.line_index;
            if abs < abs_start || abs >= abs_end {
                continue;
            }
            search_ranges_by_line
                .entry(abs - abs_start)
                .or_default()
                .push((search_match.start_col, search_match.end_col));
        }
    }
    let has_search_decorations = !search_ranges_by_line.is_empty();
    let has_frame_action_links = include_action_links
        && frame_action_links.is_some_and(|links| {
            links
                .cell_ranges_by_line
                .iter()
                .any(|ranges| !ranges.is_empty())
        });
    let has_hyperlinks =
        include_hyperlinks && snapshot.rows().iter().any(|row| !row.hyperlinks.is_empty());
    if !crate::features::terminal_surface::terminal_line_decorations_needed(
        false,
        has_search_decorations,
        has_frame_action_links,
        has_hyperlinks,
        has_command_marks,
    ) {
        return Vec::new();
    }

    crate::features::terminal_surface::build_terminal_line_decorations(
        snapshot,
        None,
        0,
        &search_ranges_by_line,
        &HashMap::new(),
        frame_action_links,
        include_action_links,
        include_hyperlinks,
        include_command_marks,
    )
}

fn terminal_keyword_highlight_updates_allowed(is_active: bool) -> bool {
    // TerminalSurface paints only precomputed keyword spans. Keeping the rule
    // source available under transient pressure lets its background worker
    // catch up without putting regex work back on the paint path.
    is_active
}

fn terminal_user_scroll_active(
    display_offset: usize,
    session_has_recent_user_scroll: bool,
    last_user_scroll_at: Option<Instant>,
    now: Instant,
) -> bool {
    display_offset > 0
        && session_has_recent_user_scroll
        && last_user_scroll_at.is_some_and(|last| {
            now.saturating_duration_since(last) < TERMINAL_USER_SCROLL_ACTIVE_WINDOW
        })
}

fn terminal_input_latency_active(last_input_at: Option<Instant>, now: Instant) -> bool {
    last_input_at
        .is_some_and(|last| now.saturating_duration_since(last) < TERMINAL_INPUT_LATENCY_WINDOW)
}

fn terminal_should_track_command_suggestion_input(
    track_suggestions: bool,
    low_latency_mode: bool,
    command_suggestions_enabled: bool,
) -> bool {
    track_suggestions && !low_latency_mode && command_suggestions_enabled
}

impl NyaTermApp {
    pub(in crate::features) fn terminal_protocol_state_for_session(
        &self,
        session_id: &str,
    ) -> TerminalProtocolState {
        self.terminal_views
            .get(session_id)
            .map(|view| view.protocol_state)
            .unwrap_or_else(|| TerminalProtocolState::from_screen(&self.terminal_screen))
    }

    pub(in crate::features) fn open_terminal_actions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal_actions_open = true;
        self.terminal_status = "terminal actions opened".to_string();
        window.focus(&self.terminal_actions_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_terminal_actions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal_actions_open = false;
        self.terminal_status = "terminal actions closed".to_string();
        window.focus(&self.terminal_focus);
        cx.notify();
    }

    pub(in crate::features) fn active_terminal_visible_text(&self) -> String {
        self.active_terminal_snapshot()
            .rows()
            .iter()
            .map(|row| row.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(in crate::features) fn active_terminal_buffer_text(&self) -> String {
        self.active_session_id
            .as_deref()
            .map(|session_id| self.terminal_buffer_text_for_session(session_id))
            .unwrap_or_else(|| self.terminal_output.clone())
    }

    pub(in crate::features) fn terminal_buffer_text_for_session(&self, session_id: &str) -> String {
        self.terminal_views
            .get(session_id)
            .map(|view| view.output.clone())
            .unwrap_or_else(|| self.terminal_output.clone())
    }

    pub(in crate::features) fn active_terminal_buffer_tail(&self) -> &str {
        self.active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.output.as_str())
            .unwrap_or(self.terminal_output.as_str())
    }

    pub(in crate::features) fn terminal_buffer_tail_for_session(&self, session_id: &str) -> &str {
        self.terminal_views
            .get(session_id)
            .map(|view| view.output.as_str())
            .unwrap_or(self.terminal_output.as_str())
    }

    pub(in crate::features) fn active_terminal_view(&self) -> Option<&TerminalViewState> {
        self.active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
    }

    pub(in crate::features) fn active_terminal_view_mut(
        &mut self,
    ) -> Option<&mut TerminalViewState> {
        let session_id = self.active_session_id.clone()?;
        self.terminal_views.get_mut(&session_id)
    }

    pub(in crate::features) fn terminal_view_for(
        &self,
        session_id: &str,
    ) -> Option<&TerminalViewState> {
        self.terminal_views.get(session_id)
    }

    pub(in crate::features) fn terminal_snapshot_for_session(
        &self,
        session_id: Option<&str>,
        offset: usize,
    ) -> std::sync::Arc<TerminalSnapshot> {
        if let Some(session_id) = session_id.filter(|id| !id.is_empty()) {
            if let Some(view) = self.terminal_views.get(session_id) {
                if offset == 0 {
                    return view
                        .frame_snapshot
                        .clone()
                        .unwrap_or_else(|| view.live_snapshot_with_scroll_window());
                }
                if let Some(snapshot) = view.scrollback_snapshots.get(&offset).cloned() {
                    return snapshot;
                }
                if let Some(snapshot) = view.frame_snapshot.as_ref()
                    && terminal_snapshot_covers_display_offset(
                        snapshot.as_ref(),
                        offset,
                        view.viewport_rows_for_ui(),
                        view.scrollback_len_for_ui(),
                    )
                {
                    return snapshot.clone();
                }
                // A live session's UI screen is only a geometry/encoding mirror;
                // it may not contain the worker's latest output. Use the nearest
                // authoritative frame when the requested history window is still
                // being fetched instead of painting or hit-testing stale content.
                return view
                    .frame_snapshot
                    .clone()
                    .unwrap_or_else(|| std::sync::Arc::new(view.screen.viewport_snapshot(offset)));
            }
        }
        std::sync::Arc::new(self.terminal_screen.viewport_snapshot(offset))
    }

    pub(in crate::features) fn active_terminal_snapshot(&self) -> std::sync::Arc<TerminalSnapshot> {
        self.terminal_snapshot_for_session(
            self.active_session_id.as_deref(),
            self.active_terminal_display_offset(),
        )
    }

    pub(in crate::features) fn copy_terminal_visible_text(&mut self, cx: &mut Context<Self>) {
        let text = self.active_terminal_visible_text();
        if text.trim().is_empty() {
            self.terminal_status = "visible terminal text is empty".to_string();
        } else {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            self.terminal_status = "copied visible terminal text".to_string();
        }
        self.terminal_actions_open = false;
        cx.notify();
    }

    pub(in crate::features) fn copy_terminal_buffer_text(&mut self, cx: &mut Context<Self>) {
        self.terminal_actions_open = false;
        let Some(session_id) = self.active_session_id.clone() else {
            let text = self.terminal_output.clone();
            if text.trim().is_empty() {
                self.terminal_status = "terminal buffer is empty".to_string();
            } else {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                self.terminal_status = "copied terminal buffer".to_string();
            }
            cx.notify();
            return;
        };
        let request_id = uuid();
        self.terminal_frame_pipeline.request_buffer_text(
            session_id,
            self.terminal_scrollback_max_bytes(),
            request_id,
        );
        self.terminal_status = "preparing terminal buffer copy".to_string();
        cx.notify();
    }

    pub(in crate::features) fn send_terminal_clear_screen(&mut self, cx: &mut Context<Self>) {
        self.terminal_actions_open = false;
        self.clear_terminal_selection(cx);
        if self.send_terminal_input(vec![0x0c], cx) {
            self.terminal_status = "clear screen command sent".to_string();
            cx.notify();
        }
    }

    pub(in crate::features) fn send_terminal_input(
        &mut self,
        bytes: Vec<u8>,
        cx: &mut Context<Self>,
    ) -> bool {
        self.send_terminal_input_with_options(bytes, true, cx)
    }

    pub(in crate::features) fn send_terminal_input_without_suggestion_track(
        &mut self,
        bytes: Vec<u8>,
        cx: &mut Context<Self>,
    ) -> bool {
        self.send_terminal_input_with_options(bytes, false, cx)
    }

    pub(in crate::features) fn send_terminal_input_with_options(
        &mut self,
        bytes: Vec<u8>,
        track_suggestions: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let input_started_at = Instant::now();
        if bytes.is_empty() {
            return false;
        }
        // Tauri/xterm custom key path: non-smart buffer selections stay painted while
        // typing. Smart input selections are handled earlier and clear themselves.
        // Only drop an in-progress drag so a stuck drag cannot block further input.
        if self.terminal_selection_dragging {
            self.terminal_selection_dragging = false;
        }
        let Some(session_id) = self.active_session_id.clone() else {
            if self.set_terminal_status_if_changed("start a session before typing") {
                cx.notify();
            }
            return false;
        };
        if self.is_session_disconnected(&session_id) {
            // Key handler owns Enter-to-reconnect (needs Window). Block writes here.
            if self
                .set_terminal_status_if_changed("session disconnected — press Enter to reconnect")
            {
                cx.notify();
            }
            return false;
        }
        // Typing while scrolled in history returns to the live bottom (xterm-like).
        if self.active_terminal_visual_scroll_active() {
            self.scroll_terminal_to_bottom(cx);
        }
        let peers = self.sync_peer_session_ids(&session_id);
        let byte_count = bytes.len();

        debug_assert!(
            terminal_wire_write_disposition(TerminalWireWriteKind::LogicalInput)
                .allow_command_history
        );
        self.terminal_frame_pipeline.arm_output_event_wake();
        // Primary + sync peers share write/record/history so recording and per-session
        // command history stay consistent. Resolve history once after all writes so a
        // pending Enter submission is applied to every successful peer.
        let mut ok_sessions = Vec::new();
        let write_started_at = Instant::now();
        match self.write_session_input_recorded(&session_id, &bytes) {
            Ok(()) => ok_sessions.push(session_id),
            Err(error) => {
                if self.set_terminal_status_if_changed(format!("input failed: {error}")) {
                    cx.notify();
                }
                return false;
            }
        }

        let mut synced = 0usize;
        let mut failed = 0usize;
        for peer_id in peers {
            match self.write_session_input_recorded(&peer_id, &bytes) {
                Ok(()) => {
                    ok_sessions.push(peer_id);
                    synced += 1;
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }
        let write_duration = write_started_at.elapsed();

        let input_wake_started_at = Instant::now();
        if !ok_sessions.is_empty() {
            self.arm_terminal_input_wake(cx);
        }
        let input_wake_duration = input_wake_started_at.elapsed();

        let suggestion_started_at = Instant::now();
        if track_suggestions {
            if terminal_should_track_command_suggestion_input(
                track_suggestions,
                self.settings.terminal_low_latency_mode,
                self.settings.interaction_command_suggestions_enabled,
            ) {
                self.note_command_suggestion_input(&bytes, cx);
            } else {
                self.note_command_history_input(&bytes);
            }
        }
        let suggestion_duration = suggestion_started_at.elapsed();

        let session_refs: Vec<&str> = ok_sessions.iter().map(String::as_str).collect();
        let history_started_at = Instant::now();
        self.record_command_history_for_sessions(&session_refs, &bytes);
        let history_duration = history_started_at.elapsed();

        let should_notify = synced > 0 || failed > 0;
        let notify_started_at = Instant::now();
        if should_notify {
            if self.set_terminal_status_if_changed(terminal_input_fanout_status(
                "sent", byte_count, synced, failed,
            )) {
                cx.notify();
            }
        }
        let notify_duration = input_wake_duration + notify_started_at.elapsed();
        log_slow_terminal_input_diagnostic(
            "input_bytes",
            byte_count,
            synced,
            failed,
            input_started_at.elapsed(),
            Duration::ZERO,
            write_duration,
            suggestion_duration,
            history_duration,
            notify_duration,
        );
        failed == 0
    }

    pub(in crate::features) fn send_terminal_key_event(
        &mut self,
        event: &KeyDownEvent,
        track_suggestions: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let input_started_at = Instant::now();
        // Key protocol modes are session-local (application cursor/keypad,
        // Kitty keyboard). Re-encode for each sync peer instead of broadcasting
        // the active session's wire bytes.
        if self.terminal_selection_dragging {
            self.terminal_selection_dragging = false;
        }
        let Some(session_id) = self.active_session_id.clone() else {
            if self.set_terminal_status_if_changed("start a session before typing") {
                cx.notify();
            }
            return false;
        };
        if self.is_session_disconnected(&session_id) {
            if self
                .set_terminal_status_if_changed("session disconnected — press Enter to reconnect")
            {
                cx.notify();
            }
            return false;
        }
        let encode_started_at = Instant::now();
        let Some(primary_bytes) =
            self.terminal_key_bytes_for_event_for_session(Some(&session_id), event)
        else {
            return false;
        };
        if primary_bytes.is_empty() {
            return false;
        }
        let mut encode_duration = encode_started_at.elapsed();
        if self.active_terminal_visual_scroll_active() {
            self.scroll_terminal_to_bottom(cx);
        }

        debug_assert!(
            terminal_wire_write_disposition(TerminalWireWriteKind::LogicalInput)
                .allow_command_history
        );
        self.terminal_frame_pipeline.arm_output_event_wake();
        let byte_count = primary_bytes.len();
        let peers = self.sync_peer_session_ids(&session_id);
        let mut ok_sessions = Vec::new();
        let write_started_at = Instant::now();
        match self.write_session_input_recorded(&session_id, &primary_bytes) {
            Ok(()) => ok_sessions.push(session_id),
            Err(error) => {
                if self.set_terminal_status_if_changed(format!("input failed: {error}")) {
                    cx.notify();
                }
                return false;
            }
        }

        let mut synced = 0usize;
        let mut failed = 0usize;
        for peer_id in peers {
            let peer_encode_started_at = Instant::now();
            let Some(peer_bytes) =
                self.terminal_key_bytes_for_event_for_session(Some(&peer_id), event)
            else {
                encode_duration += peer_encode_started_at.elapsed();
                continue;
            };
            encode_duration += peer_encode_started_at.elapsed();
            if peer_bytes.is_empty() {
                continue;
            }
            match self.write_session_input_recorded(&peer_id, &peer_bytes) {
                Ok(()) => {
                    ok_sessions.push(peer_id);
                    synced += 1;
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }
        let write_duration = write_started_at.elapsed();

        let input_wake_started_at = Instant::now();
        if !ok_sessions.is_empty() {
            self.arm_terminal_input_wake(cx);
        }
        let input_wake_duration = input_wake_started_at.elapsed();

        let suggestion_started_at = Instant::now();
        if track_suggestions {
            if terminal_should_track_command_suggestion_input(
                track_suggestions,
                self.settings.terminal_low_latency_mode,
                self.settings.interaction_command_suggestions_enabled,
            ) {
                self.note_command_suggestion_input(&primary_bytes, cx);
            } else {
                self.note_command_history_input(&primary_bytes);
            }
        }
        let suggestion_duration = suggestion_started_at.elapsed();

        let session_refs: Vec<&str> = ok_sessions.iter().map(String::as_str).collect();
        let history_started_at = Instant::now();
        self.record_command_history_for_sessions(&session_refs, &primary_bytes);
        let history_duration = history_started_at.elapsed();

        let should_notify = synced > 0 || failed > 0;
        let notify_started_at = Instant::now();
        if should_notify {
            if self.set_terminal_status_if_changed(terminal_input_fanout_status(
                "sent", byte_count, synced, failed,
            )) {
                cx.notify();
            }
        }
        let notify_duration = input_wake_duration + notify_started_at.elapsed();
        log_slow_terminal_input_diagnostic(
            "key_down",
            byte_count,
            synced,
            failed,
            input_started_at.elapsed(),
            encode_duration,
            write_duration,
            suggestion_duration,
            history_duration,
            notify_duration,
        );
        failed == 0
    }

    pub(in crate::features) fn send_terminal_key_release_event(
        &mut self,
        event: &KeyUpEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(session_id) = self.active_session_id.clone() else {
            return false;
        };
        if self.is_session_disconnected(&session_id) {
            return false;
        }
        let Some(primary_bytes) =
            self.terminal_key_release_bytes_for_event_for_session(Some(&session_id), event)
        else {
            return false;
        };
        if primary_bytes.is_empty() {
            return false;
        }

        let byte_count = primary_bytes.len();
        let peers = self.sync_peer_session_ids(&session_id);
        let mut synced = 0usize;
        let mut failed = 0usize;
        match self.write_session_input_recorded(&session_id, &primary_bytes) {
            Ok(()) => {}
            Err(error) => {
                if self.set_terminal_status_if_changed(format!("input failed: {error}")) {
                    cx.notify();
                }
                return false;
            }
        }
        for peer_id in peers {
            let Some(peer_bytes) =
                self.terminal_key_release_bytes_for_event_for_session(Some(&peer_id), event)
            else {
                continue;
            };
            if peer_bytes.is_empty() {
                continue;
            }
            match self.write_session_input_recorded(&peer_id, &peer_bytes) {
                Ok(()) => synced += 1,
                Err(_) => failed += 1,
            }
        }
        if synced > 0 || failed > 0 {
            if self.set_terminal_status_if_changed(terminal_input_fanout_status(
                "sent", byte_count, synced, failed,
            )) {
                cx.notify();
            }
        }
        failed == 0
    }

    /// On the alternate screen with alternate-scroll enabled and no mouse
    /// tracking, emulate xterm: wheel becomes Up/Down cursor sequences.
    pub(in crate::features) fn maybe_send_alternate_scroll_for_session(
        &mut self,
        session_id: &str,
        delta_lines: i32,
        cx: &mut Context<Self>,
    ) -> bool {
        if delta_lines == 0 || session_id.is_empty() {
            return false;
        }
        if self.is_session_disconnected(session_id) {
            return false;
        }
        let Some(payload) = self.alternate_scroll_payload_for_session(session_id, delta_lines)
        else {
            return false;
        };
        if let Err(error) = self.write_session_input_recorded(session_id, &payload) {
            if self.set_terminal_status_if_changed(format!("alternate scroll failed: {error}")) {
                cx.notify();
            }
            return true;
        }

        let peers = self.sync_peer_session_ids(session_id);
        let mut synced = 0usize;
        let mut failed = 0usize;
        for peer_id in peers {
            let Some(peer_payload) =
                self.alternate_scroll_payload_for_session(&peer_id, delta_lines)
            else {
                continue;
            };
            match self.write_session_input_recorded(&peer_id, &peer_payload) {
                Ok(()) => synced += 1,
                Err(_) => failed += 1,
            }
        }
        if failed > 0 {
            self.terminal_status =
                format!("alternate scroll synced {synced} peer(s), {failed} failed");
            cx.notify();
        }
        true
    }

    fn alternate_scroll_payload_for_session(
        &self,
        session_id: &str,
        delta_lines: i32,
    ) -> Option<Vec<u8>> {
        if delta_lines == 0 || session_id.is_empty() || self.is_session_disconnected(session_id) {
            return None;
        }
        self.terminal_protocol_state_for_session(session_id)
            .alternate_scroll_payload(delta_lines)
    }

    /// When the active session's screen has mouse reporting enabled, encode and
    /// send a mouse report instead of performing local selection/scroll.
    /// Returns true when the terminal app handled the event (caller should skip
    /// local handling). Protocol traffic is recorded but not command history.

    pub(in crate::features) fn maybe_send_mouse_report(
        &mut self,
        button: u8,
        col: u16,
        row: u16,
        press: bool,
        motion: bool,
        modifiers: gpui::Modifiers,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(session_id) = self.active_session_id.clone() else {
            return false;
        };
        self.maybe_send_mouse_report_for_session(
            &session_id,
            button,
            col,
            row,
            press,
            motion,
            modifiers,
            cx,
        )
    }

    pub(in crate::features) fn maybe_send_mouse_report_for_session(
        &mut self,
        session_id: &str,
        button: u8,
        col: u16,
        row: u16,
        press: bool,
        motion: bool,
        modifiers: gpui::Modifiers,
        cx: &mut Context<Self>,
    ) -> bool {
        let result = self.write_mouse_report_to_session(
            session_id, button, col, row, press, motion, modifiers, cx,
        );
        match result {
            MouseReportWriteResult::NotHandled => return false,
            MouseReportWriteResult::Failed => return true,
            MouseReportWriteResult::Sent => {}
        }

        self.terminal_mouse_report_position = Some((col, row));
        if motion {
            let peers = self.terminal_mouse_report_peer_session_ids.clone();
            for peer_id in peers {
                let _ = self.write_mouse_report_to_session(
                    &peer_id, button, col, row, press, true, modifiers, cx,
                );
            }
            return true;
        }
        if press && button < 3 {
            let peers = self.sync_peer_session_ids(session_id);
            let mut captured_peers = Vec::new();
            for peer_id in peers {
                if self.write_mouse_report_to_session(
                    &peer_id, button, col, row, true, false, modifiers, cx,
                ) == MouseReportWriteResult::Sent
                {
                    captured_peers.push(peer_id);
                }
            }
            self.terminal_mouse_report_button = Some(button);
            self.terminal_mouse_report_session_id = Some(session_id.to_string());
            self.terminal_mouse_report_peer_session_ids = captured_peers;
        } else if !press {
            let peers = std::mem::take(&mut self.terminal_mouse_report_peer_session_ids);
            for peer_id in peers {
                let _ = self.write_mouse_report_to_session(
                    &peer_id, button, col, row, false, false, modifiers, cx,
                );
            }
            self.clear_terminal_mouse_report_capture();
        }
        true
    }

    pub(in crate::features) fn maybe_send_terminal_any_motion_report(
        &mut self,
        event: &gpui::MouseMoveEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.terminal_mouse_report_button.is_some() {
            return false;
        }
        let Some(session_id) = self.terminal_session_at_point(event.position) else {
            return false;
        };
        let session_id = session_id
            .or_else(|| self.active_session_id.clone())
            .unwrap_or_default();
        if session_id.is_empty() {
            return false;
        }
        let protocol = self.terminal_protocol_state_for_session(&session_id);
        if !protocol.mouse_motion_reporting {
            return false;
        }
        let Some(cell) =
            self.point_to_terminal_cell_for_session(Some(session_id.as_str()), event.position, cx)
        else {
            return false;
        };
        let col = cell.col as u16;
        let row = cell.row as u16;
        match self.write_mouse_report_to_session(
            &session_id,
            3,
            col,
            row,
            true,
            true,
            event.modifiers,
            cx,
        ) {
            MouseReportWriteResult::Sent => {
                for peer_id in self.sync_peer_session_ids(&session_id) {
                    let _ = self.write_mouse_report_to_session(
                        &peer_id,
                        3,
                        col,
                        row,
                        true,
                        true,
                        event.modifiers,
                        cx,
                    );
                }
                true
            }
            MouseReportWriteResult::Failed => true,
            MouseReportWriteResult::NotHandled => false,
        }
    }

    fn write_mouse_report_to_session(
        &mut self,
        session_id: &str,
        button: u8,
        col: u16,
        row: u16,
        press: bool,
        motion: bool,
        modifiers: gpui::Modifiers,
        cx: &mut Context<Self>,
    ) -> MouseReportWriteResult {
        if session_id.is_empty() {
            return MouseReportWriteResult::NotHandled;
        }
        let disconnected = self.is_session_disconnected(session_id);
        let protocol = self.terminal_protocol_state_for_session(session_id);
        if !protocol.mouse_reporting {
            return MouseReportWriteResult::NotHandled;
        }
        if !terminal_mouse_report_should_send(TerminalMouseReportEligibility {
            session_id_empty: false,
            disconnected,
            mouse_reporting: protocol.mouse_reporting,
            motion,
            mouse_drag_reporting: protocol.mouse_drag_reporting,
        }) {
            return MouseReportWriteResult::NotHandled;
        }
        let bytes = protocol.encode_mouse_report(
            button,
            col,
            row,
            press,
            motion,
            modifiers.shift,
            modifiers.alt,
            modifiers.control || modifiers.platform,
        );
        if bytes.is_empty() {
            return MouseReportWriteResult::NotHandled;
        }
        if let Err(error) = self.write_session_input_recorded(session_id, &bytes) {
            if self.set_terminal_status_if_changed(format!("mouse report failed: {error}")) {
                cx.notify();
            }
            return MouseReportWriteResult::Failed;
        }
        MouseReportWriteResult::Sent
    }

    pub(in crate::features) fn finish_terminal_mouse_report(
        &mut self,
        event: &gpui::MouseUpEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(button) = self.terminal_mouse_report_button else {
            return false;
        };
        let Some(session_id) = self
            .terminal_mouse_report_session_id
            .clone()
            .or_else(|| self.active_session_id.clone())
        else {
            self.clear_terminal_mouse_report_capture();
            return false;
        };
        let (col, row) = if let Some(cell) =
            self.point_to_terminal_cell_for_session(Some(session_id.as_str()), event.position, cx)
        {
            (cell.col as u16, cell.row as u16)
        } else if let Some((col, row)) = self.terminal_mouse_report_position {
            (col, row)
        } else {
            self.clear_terminal_mouse_report_capture();
            return false;
        };
        self.maybe_send_mouse_report_for_session(
            &session_id,
            button,
            col,
            row,
            false,
            false,
            event.modifiers,
            cx,
        )
    }

    pub(in crate::features) fn clear_terminal_mouse_report_for_session(
        &mut self,
        session_id: &str,
    ) {
        if self.terminal_mouse_report_session_id.as_deref() == Some(session_id)
            || self
                .terminal_mouse_report_peer_session_ids
                .iter()
                .any(|peer_id| peer_id == session_id)
        {
            self.clear_terminal_mouse_report_capture();
        }
    }

    fn clear_terminal_mouse_report_capture(&mut self) {
        self.terminal_mouse_report_button = None;
        self.terminal_mouse_report_session_id = None;
        self.terminal_mouse_report_peer_session_ids.clear();
        self.terminal_mouse_report_position = None;
    }

    /// Write UTF-8/ASCII input to a live session and mirror the logical input
    /// into the session recording buffer.
    /// Does not touch command history, status text, or UI notify.
    pub(in crate::features) fn write_session_input_recorded(
        &mut self,
        session_id: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        // Charset-encode paste/typed text; pure ASCII CSI/mouse reports pass through.
        let disposition = terminal_wire_write_disposition(TerminalWireWriteKind::LogicalInput);
        let encoded = if disposition.encode_session_charset {
            self.encode_session_outgoing(session_id, bytes)
        } else {
            bytes.to_vec()
        };
        if let Err(error) = self
            .session_manager
            .write(session_id, &encoded)
            .map_err(|error| error.to_string())
        {
            self.record_terminal_session_write_failure(session_id, "input", &error);
            return Err(error);
        }
        if disposition.record_logical_input {
            self.recording_write_pipeline
                .write_input(session_id.to_string(), bytes.to_vec());
        }
        Ok(())
    }

    /// Write already-encoded/raw bytes to a live session and mirror the exact
    /// bytes into the session recording buffer.
    pub(in crate::features) fn write_session_raw_input_recorded(
        &mut self,
        session_id: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        let disposition = terminal_wire_write_disposition(TerminalWireWriteKind::RawInput);
        debug_assert!(!disposition.encode_session_charset);
        if let Err(error) = self
            .session_manager
            .write(session_id, bytes)
            .map_err(|error| error.to_string())
        {
            self.record_terminal_session_write_failure(session_id, "raw input", &error);
            return Err(error);
        }
        if disposition.record_raw_input {
            self.recording_write_pipeline
                .write_raw_input(session_id.to_string(), bytes.to_vec());
        }
        Ok(())
    }

    /// Write bytes already prepared for the PTY while recording a separate
    /// logical input stream. Used when protocol framing (for example bracketed
    /// paste) should reach the session but not pollute input recordings.
    pub(in crate::features) fn write_session_wire_input_recorded_as(
        &mut self,
        session_id: &str,
        wire_bytes: &[u8],
        recording_bytes: &[u8],
    ) -> Result<(), String> {
        let disposition = terminal_wire_write_disposition(TerminalWireWriteKind::FramedInput);
        debug_assert!(!disposition.encode_session_charset);
        if let Err(error) = self
            .session_manager
            .write(session_id, wire_bytes)
            .map_err(|error| error.to_string())
        {
            self.record_terminal_session_write_failure(session_id, "framed input", &error);
            return Err(error);
        }
        if disposition.record_logical_input {
            self.recording_write_pipeline
                .write_input(session_id.to_string(), recording_bytes.to_vec());
        }
        Ok(())
    }

    /// Write terminal-emulator protocol bytes (DSR/OSC/Kitty replies, focus
    /// reports) without marking them as user input in recordings.
    pub(in crate::features) fn write_session_protocol_response(
        &mut self,
        session_id: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        let disposition = terminal_wire_write_disposition(TerminalWireWriteKind::ProtocolResponse);
        debug_assert!(!disposition.encode_session_charset);
        debug_assert!(!disposition.record_logical_input);
        debug_assert!(!disposition.record_raw_input);
        debug_assert!(!disposition.allow_command_history);
        if let Err(error) = self
            .session_manager
            .write(session_id, bytes)
            .map_err(|error| error.to_string())
        {
            self.record_terminal_session_write_failure(session_id, "protocol response", &error);
            return Err(error);
        }
        Ok(())
    }

    fn record_terminal_session_write_failure(
        &mut self,
        session_id: &str,
        context: &str,
        error: &str,
    ) {
        let safe_error = terminal_session_write_failure_safe_text(error);
        tracing::warn!(
            diagnostic = "session_write_failed",
            session_id = %session_id,
            context,
            error = %safe_error,
            "terminal session write failed"
        );
        if session_id.is_empty() {
            return;
        }
        let log = terminal_session_write_failure_log(context, error);
        self.recording_write_pipeline
            .write_output(session_id.to_string(), log.clone());
        self.append_terminal_log_for_session(Some(session_id), &log, true);
    }

    /// Encode UTF-8 host input for the session wire charset (UTF-8/GBK/…).
    pub(in crate::features) fn encode_session_outgoing(
        &self,
        session_id: &str,
        bytes: &[u8],
    ) -> Vec<u8> {
        if let Some(view) = self.terminal_views.get(session_id) {
            return view.screen.encode_outgoing(bytes);
        }
        self.terminal_screen.encode_outgoing(bytes)
    }

    /// Apply interaction default encoding to a terminal screen.
    pub(in crate::features) fn apply_terminal_encoding_to_screen(
        &self,
        screen: &mut nyaterm_terminal::TerminalScreen,
    ) {
        screen.set_encoding(&self.settings.interaction_default_encoding);
    }

    /// Keep all live terminal screens on the current interaction encoding.
    pub(in crate::features) fn sync_terminal_encodings_from_settings(&mut self) {
        let label = self.settings.interaction_default_encoding.clone();
        self.terminal_screen.set_encoding(&label);
        self.terminal_output_decoder.set_encoding(&label);
        for view in self.terminal_views.values_mut() {
            view.set_encoding(&label);
        }
        self.sync_session_event_bridge_config();
    }

    pub(in crate::features) fn ensure_terminal_focus_reporting(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.terminal_focus_subscriptions.is_empty() {
            return;
        }
        let focus_in = cx.on_focus_in(&self.terminal_focus, window, |this, _window, cx| {
            this.report_terminal_focus(true, cx);
        });
        let focus_out =
            cx.on_focus_out(&self.terminal_focus, window, |this, _event, _window, cx| {
                this.report_terminal_focus(false, cx);
            });
        self.terminal_focus_subscriptions = vec![focus_in, focus_out];
    }

    pub(in crate::features) fn report_terminal_focus(
        &mut self,
        focused: bool,
        cx: &mut Context<Self>,
    ) {
        self.terminal_focus_active = focused;
        let Some(session_id) = self.active_session_id.clone() else {
            return;
        };
        if self.write_terminal_focus_report_to_session(&session_id, focused) {
            cx.notify();
        }
    }

    /// Send a DECSET 1004 focus report to a specific session when that session
    /// has enabled focus reporting. Protocol traffic is not command history.
    pub(in crate::features) fn write_terminal_focus_report_to_session(
        &mut self,
        session_id: &str,
        focused: bool,
    ) -> bool {
        if self.is_session_disconnected(session_id) {
            return false;
        }
        if !self
            .terminal_protocol_state_for_session(session_id)
            .focus_reporting
        {
            return false;
        }
        let bytes = nyaterm_terminal::TerminalScreen::encode_focus_report(focused);
        self.write_session_protocol_response(session_id, &bytes)
            .is_ok()
    }

    pub(in crate::features) fn sync_terminal_cell_metrics_to_screens(&mut self) {
        let (width, height) = self.terminal_cell_size();
        let width = width.round().clamp(1.0, 512.0) as u16;
        let height = height.round().clamp(1.0, 512.0) as u16;
        self.terminal_screen.set_cell_metrics(width, height);
        for view in self.terminal_views.values_mut() {
            view.screen.set_cell_metrics(width, height);
        }
    }

    pub(in crate::features) fn send_terminal_input_to_session(
        &mut self,
        session_id: String,
        bytes: Vec<u8>,
        cx: &mut Context<Self>,
    ) -> bool {
        if bytes.is_empty() {
            return false;
        }
        if self.is_session_disconnected(&session_id) {
            if self.set_terminal_status_if_changed(
                "session disconnected — reconnect before sending".to_string(),
            ) {
                cx.notify();
            }
            return false;
        }
        self.terminal_frame_pipeline.arm_output_event_wake();
        let sent = match self.write_session_input_recorded(&session_id, &bytes) {
            Ok(()) => {
                self.record_command_history_from_bytes(Some(&session_id), &bytes);
                let terminal_status_changed =
                    self.set_terminal_status_if_changed(format!("sent {} byte(s)", bytes.len()));
                self.arm_terminal_input_wake(cx);
                if terminal_status_changed {
                    cx.notify();
                }
                true
            }
            Err(error) => {
                if self.set_terminal_status_if_changed(format!("input failed: {error}")) {
                    cx.notify();
                }
                false
            }
        };
        sent
    }

    pub(in crate::features) fn send_terminal_raw_input_to_session(
        &mut self,
        session_id: String,
        bytes: Vec<u8>,
        cx: &mut Context<Self>,
    ) -> bool {
        if bytes.is_empty() {
            return false;
        }
        if self.is_session_disconnected(&session_id) {
            if self.set_terminal_status_if_changed(
                "session disconnected — reconnect before sending".to_string(),
            ) {
                cx.notify();
            }
            return false;
        }
        self.terminal_frame_pipeline.arm_output_event_wake();
        let sent = match self.write_session_raw_input_recorded(&session_id, &bytes) {
            Ok(()) => {
                let terminal_status_changed =
                    self.set_terminal_status_if_changed(format!("sent {} byte(s)", bytes.len()));
                self.arm_terminal_input_wake(cx);
                if terminal_status_changed {
                    cx.notify();
                }
                true
            }
            Err(error) => {
                if self.set_terminal_status_if_changed(format!("input failed: {error}")) {
                    cx.notify();
                }
                false
            }
        };
        sent
    }

    pub(in crate::features) fn active_terminal_key_mode(&self) -> TerminalKeyMode {
        self.terminal_key_mode_for_session(self.active_session_id.as_deref())
    }

    pub(in crate::features) fn set_terminal_status_if_changed(
        &mut self,
        status: impl Into<String>,
    ) -> bool {
        let status = status.into();
        if !terminal_status_changed(self.terminal_status.as_str(), status.as_str()) {
            return false;
        }
        self.terminal_status = status;
        true
    }

    pub(in crate::features) fn terminal_key_mode_for_session(
        &self,
        session_id: Option<&str>,
    ) -> TerminalKeyMode {
        let (
            application_cursor,
            application_keypad,
            kitty_keyboard_disambiguate,
            kitty_keyboard_report_event_types,
            kitty_keyboard_report_alternate_keys,
            kitty_keyboard_report_all_keys_as_esc,
            kitty_keyboard_report_associated_text,
        ) = if let Some(session_id) = session_id {
            self.terminal_views
                .get(session_id)
                .map(|view| {
                    let protocol = view.protocol_state;
                    (
                        protocol.application_cursor_keys,
                        protocol.application_keypad,
                        protocol.kitty_keyboard_disambiguate,
                        protocol.kitty_keyboard_report_event_types,
                        protocol.kitty_keyboard_report_alternate_keys,
                        protocol.kitty_keyboard_report_all_keys_as_esc,
                        protocol.kitty_keyboard_report_associated_text,
                    )
                })
                .unwrap_or((false, false, false, false, false, false, false))
        } else {
            (
                self.terminal_screen.application_cursor_keys(),
                self.terminal_screen.application_keypad(),
                self.terminal_screen.kitty_keyboard_disambiguate(),
                self.terminal_screen.kitty_keyboard_report_event_types(),
                self.terminal_screen.kitty_keyboard_report_alternate_keys(),
                self.terminal_screen.kitty_keyboard_report_all_keys_as_esc(),
                self.terminal_screen.kitty_keyboard_report_associated_text(),
            )
        };
        TerminalKeyMode {
            application_cursor,
            application_keypad,
            kitty_keyboard_disambiguate,
            kitty_keyboard_report_event_types,
            kitty_keyboard_report_alternate_keys,
            kitty_keyboard_report_all_keys_as_esc,
            kitty_keyboard_report_associated_text,
        }
    }

    pub(in crate::features) fn terminal_key_bytes_for_event(
        &self,
        event: &KeyDownEvent,
    ) -> Option<Vec<u8>> {
        self.terminal_key_bytes_for_event_for_session(self.active_session_id.as_deref(), event)
    }

    pub(in crate::features) fn terminal_key_bytes_for_event_for_session(
        &self,
        session_id: Option<&str>,
        event: &KeyDownEvent,
    ) -> Option<Vec<u8>> {
        terminal_key_bytes_for_mode_and_settings(
            event,
            self.terminal_key_mode_for_session(session_id),
            self.settings.interaction_alt_as_meta,
        )
    }

    pub(in crate::features) fn terminal_should_defer_key_text_to_input_handler(
        &self,
        event: &KeyDownEvent,
    ) -> bool {
        terminal_should_defer_key_text_to_input_handler_for_state(
            self.settings.interaction_mac_ime_compatibility,
            &self.terminal_ime_marked_text,
            event,
        )
    }

    pub(in crate::features) fn terminal_key_release_bytes_for_event(
        &self,
        event: &KeyUpEvent,
    ) -> Option<Vec<u8>> {
        self.terminal_key_release_bytes_for_event_for_session(
            self.active_session_id.as_deref(),
            event,
        )
    }

    pub(in crate::features) fn terminal_key_release_bytes_for_event_for_session(
        &self,
        session_id: Option<&str>,
        event: &KeyUpEvent,
    ) -> Option<Vec<u8>> {
        terminal_key_release_bytes_with_mode(event, self.terminal_key_mode_for_session(session_id))
    }

    pub(in crate::features) fn ensure_terminal_surface(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
    ) -> Entity<TerminalSurface> {
        if let Some(surface) = self.terminal_surfaces.get(session_id) {
            return surface.clone();
        }
        let layout_cache = self
            .terminal_views
            .get(session_id)
            .map(|view| view.render_cache.layout_cache.clone())
            .unwrap_or_else(|| {
                std::sync::Arc::new(std::sync::Mutex::new(NyaTerminalLayoutCache::default()))
            });
        let session_id_owned = session_id.to_string();
        let app = cx.entity();
        let surface = cx.new(|_| {
            let mut surface = TerminalSurface::new(session_id_owned);
            surface.set_layout_cache(layout_cache);
            surface.set_app(app);
            surface
        });
        self.terminal_surfaces
            .insert(session_id.to_string(), surface.clone());
        surface
    }

    pub(in crate::features) fn remove_terminal_surface(&mut self, session_id: &str) {
        self.terminal_surfaces.remove(session_id);
    }

    fn remember_terminal_scroll_window_snapshot(
        &mut self,
        session_id: &str,
        display_offset: usize,
        snapshot: &std::sync::Arc<TerminalSnapshot>,
    ) {
        if display_offset == 0 {
            return;
        }
        if let Some(view) = self.terminal_views.get_mut(session_id) {
            view.remember_scrollback_snapshot(display_offset, snapshot.clone());
        }
    }

    fn sync_terminal_scroll_text_first_surface_paint(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
    ) -> bool {
        if session_id.is_empty() {
            return false;
        }
        let surface = self.ensure_terminal_surface(session_id, cx);
        let Some(view) = self.terminal_views.get(session_id) else {
            return false;
        };
        let scroll_offset = view.scroll_offset;
        let scroll_residual_lines = self.terminal_scroll_residual_for_session(Some(session_id));
        let scrollback_len = view.scrollback_len_for_ui();
        let viewport_rows = view.viewport_rows_for_ui();
        let display_offset =
            terminal_visual_display_offset(scroll_offset, scroll_residual_lines, scrollback_len);
        let now = Instant::now();
        let user_scroll_active = terminal_user_scroll_active(
            display_offset,
            self.terminal_runtime
                .pending_terminal_user_scroll_idle_sessions
                .contains(session_id),
            self.terminal_runtime.last_terminal_user_scroll_at,
            now,
        );
        let input_latency_active =
            terminal_input_latency_active(self.terminal_runtime.last_terminal_input_at, now);
        let Some(snapshot) = terminal_paint_window_snapshot_for_view(
            Some(view),
            display_offset,
            viewport_rows,
            None,
        ) else {
            return false;
        };
        let palette = self.terminal_theme_palette();
        let transparent_background = self.wallpaper_enabled();
        let font_family = self.gpui_terminal_font_family();
        let font_size = self.settings.terminal_font_size as f32;
        let normal_weight = self.settings.terminal_font_weight as f32;
        let bold_weight = self.settings.terminal_font_weight_bold as f32;
        let show_line_numbers = self.settings.terminal_show_line_numbers;
        let show_timestamps = self.settings.terminal_show_timestamps;
        let show_timestamp_ms = self.settings.terminal_show_timestamp_milliseconds;
        let (cell_w, cell_h) = self
            .terminal_cell_metrics
            .unwrap_or(((font_size * 0.6).max(6.0), (font_size * 1.35).max(12.0)));
        let is_active = self.active_session_id.as_deref() == Some(session_id);
        let visual_bell = is_active && self.terminal_runtime.visual_bell_ticks > 0;
        let layout_cache = view.render_cache.layout_cache.clone();
        let render_degraded = view.render_degraded || self.settings.terminal_low_latency_mode;
        let has_new = view.has_new_while_scrolled;
        let performance_overlay = view.performance_overlay;
        let skipped = view.skipped_output_chars;
        let protocol_state = view.protocol_state;
        let search_matches = if is_active
            && !input_latency_active
            && !self.settings.terminal_low_latency_mode
            && self.terminal_search_open
            && self.terminal_search_mode == TerminalSearchMode::Buffer
        {
            self.terminal_buffer_matches().unwrap_or_default()
        } else {
            Vec::new()
        };
        let action_links_enabled =
            self.settings.terminal_action_links_enabled && !self.settings.terminal_low_latency_mode;
        let action_link_matcher_key = terminal_action_link_matcher_key(
            action_links_enabled,
            &self.settings.terminal_action_links_matchers,
        );
        let frame_action_links = action_links_enabled
            .then(|| {
                if display_offset == 0 {
                    view.frame_action_links.as_ref()
                } else {
                    view.scrollback_action_links.get(&display_offset)
                }
            })
            .flatten()
            .filter(|links| links.matcher_key == action_link_matcher_key);
        let needs_scroll_enrichment = super::buffer::terminal_scroll_enrichment_should_request(
            display_offset > 0,
            display_offset,
            action_links_enabled.then_some(action_link_matcher_key),
            Some(view),
        );
        let decorations = terminal_scroll_text_first_decorations(
            snapshot.as_ref(),
            (!search_matches.is_empty()).then_some(search_matches.as_slice()),
            frame_action_links,
            action_links_enabled,
            action_links_enabled,
            is_active && !input_latency_active && !self.settings.terminal_low_latency_mode,
        );
        let has_action_link_decorations = frame_action_links.is_some_and(|links| {
            links
                .cell_ranges_by_line
                .iter()
                .any(|ranges| !ranges.is_empty())
        });
        let configured_keyword_rules = self.resolved_keyword_highlight_rules();
        let clear_keyword_highlights = configured_keyword_rules.is_empty();
        let keyword_rules = if terminal_keyword_highlight_updates_allowed(is_active) {
            configured_keyword_rules.clone()
        } else {
            std::sync::Arc::new(Vec::new())
        };
        if needs_scroll_enrichment {
            let _ = self
                .request_terminal_frame_snapshot_for_scroll_enrichment(session_id, display_offset);
        }
        self.remember_terminal_scroll_window_snapshot(session_id, display_offset, &snapshot);
        surface.update(cx, |surface, cx| {
            let mut changed = false;
            changed |= surface.set_layout_cache(layout_cache);
            changed |= surface.set_background_transparent(transparent_background);
            changed |= surface.set_paint_chrome(
                palette,
                font_family,
                font_size,
                normal_weight,
                bold_weight,
                cell_w,
                cell_h,
                show_line_numbers,
                show_timestamps,
                show_timestamp_ms,
                is_active,
                visual_bell,
            );
            changed |= surface.set_protocol_state(protocol_state);
            let had_pending_local_scroll_sync = surface.has_pending_local_scroll_sync();
            let frame_applied = surface.apply_frame_snapshot(
                snapshot,
                scroll_offset,
                scroll_residual_lines,
                display_offset,
                scrollback_len,
                viewport_rows,
                has_new,
                performance_overlay,
                skipped,
                has_action_link_decorations,
                false,
                "block",
            );
            let paint_details_changed = if frame_applied {
                surface.set_decorations_and_keywords(decorations, keyword_rules, false, "block")
            } else if !render_degraded && !user_scroll_active && !input_latency_active {
                surface.set_decorations_and_keywords(decorations, keyword_rules, false, "block")
            } else {
                surface.set_decorations_and_keywords_preserving_stale(
                    decorations,
                    keyword_rules,
                    false,
                    "block",
                    true,
                )
            };
            if frame_applied || paint_details_changed {
                surface.schedule_keyword_highlights(clear_keyword_highlights, cx);
            }
            if changed || frame_applied || paint_details_changed || had_pending_local_scroll_sync {
                cx.notify();
            }
        });
        true
    }

    /// Push the current view/frame paint state into the session surface and notify it.
    pub(in crate::features) fn sync_terminal_surface_paint(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        if session_id.is_empty() {
            return;
        }
        let paint_started_at = Instant::now();
        self.ensure_paint_theme_caches();
        let surface = self.ensure_terminal_surface(session_id, cx);
        let is_active = self.active_session_id.as_deref() == Some(session_id);
        let is_disconnected = self.is_session_disconnected(session_id);
        let render_output_pressure = self.runtime_output_pressure_active();
        let view = self.terminal_views.get(session_id);
        let scroll_offset = view.map(|v| v.scroll_offset).unwrap_or(0);
        let scroll_residual_lines = self.terminal_scroll_residual_for_session(Some(session_id));
        let has_new = view.map(|v| v.has_new_while_scrolled).unwrap_or(false);
        let performance_overlay = view.and_then(|v| v.performance_overlay);
        let skipped = view.map(|v| v.skipped_output_chars).unwrap_or(0);
        let protocol_state = view.map(|v| v.protocol_state).unwrap_or_default();
        let layout_cache = view
            .map(|v| v.render_cache.layout_cache.clone())
            .unwrap_or_else(|| {
                std::sync::Arc::new(std::sync::Mutex::new(NyaTerminalLayoutCache::default()))
            });
        let render_degraded_view = view.map(|v| v.render_degraded).unwrap_or(false);
        let burst = view.map(|v| v.output_burst_bytes).unwrap_or(0);
        let mode = view
            .map(|v| v.performance_mode)
            .unwrap_or(TerminalPerformanceMode::Normal);
        let scrollback_len = self
            .terminal_views
            .get(session_id)
            .map(|view| view.scrollback_len_for_ui())
            .unwrap_or(0);
        let viewport_rows = self
            .terminal_views
            .get(session_id)
            .map(|view| view.viewport_rows_for_ui())
            .unwrap_or(1);
        let display_offset =
            terminal_visual_display_offset(scroll_offset, scroll_residual_lines, scrollback_len);
        let user_scroll_active = terminal_user_scroll_active(
            display_offset,
            self.terminal_runtime
                .pending_terminal_user_scroll_idle_sessions
                .contains(session_id),
            self.terminal_runtime.last_terminal_user_scroll_at,
            paint_started_at,
        );
        let input_latency_active = terminal_input_latency_active(
            self.terminal_runtime.last_terminal_input_at,
            paint_started_at,
        );
        let render_pressure = render_output_pressure
            || burst > 0
            || mode == TerminalPerformanceMode::Overloaded
            || user_scroll_active
            || input_latency_active
            || self.settings.terminal_low_latency_mode;
        let render_degraded = render_degraded_view || render_pressure;
        let configured_keyword_rules = self.resolved_keyword_highlight_rules();
        let clear_keyword_highlights = configured_keyword_rules.is_empty();
        let keyword_rules = if !terminal_keyword_highlight_updates_allowed(is_active) {
            std::sync::Arc::new(Vec::new())
        } else {
            configured_keyword_rules.clone()
        };
        let retained_lookup_started_at = Instant::now();
        let retained_surface_snapshot = if display_offset > 0 {
            surface
                .read(cx)
                .snapshot_covering_display_offset(display_offset, viewport_rows, scrollback_len)
                .filter(|snapshot| {
                    terminal_snapshot_covers_display_offset(
                        snapshot.as_ref(),
                        display_offset,
                        viewport_rows,
                        scrollback_len,
                    )
                })
                .filter(|snapshot| {
                    view.is_some_and(|view| {
                        terminal_retained_snapshot_matches_view(
                            snapshot.as_ref(),
                            view,
                            display_offset,
                            viewport_rows,
                        )
                    })
                })
        } else {
            None
        };
        let retained_lookup_duration = retained_lookup_started_at.elapsed();
        let retained_snapshot_reused = retained_surface_snapshot.is_some();
        let snapshot_started_at = Instant::now();
        let snapshot = terminal_paint_window_snapshot_for_view(
            view,
            display_offset,
            viewport_rows,
            retained_surface_snapshot,
        );
        let snapshot_duration = snapshot_started_at.elapsed();
        let frame_action_links = view.and_then(|v| {
            if display_offset == 0 {
                v.frame_action_links.clone()
            } else {
                v.scrollback_action_links.get(&display_offset).cloned()
            }
        });
        let palette = self.terminal_theme_palette();
        let transparent_background = self.wallpaper_enabled();
        let font_family = self.gpui_terminal_font_family();
        let font_size = self.settings.terminal_font_size as f32;
        let normal_weight = self.settings.terminal_font_weight as f32;
        let bold_weight = self.settings.terminal_font_weight_bold as f32;
        let show_line_numbers = self.settings.terminal_show_line_numbers;
        let show_timestamps = self.settings.terminal_show_timestamps;
        let show_timestamp_ms = self.settings.terminal_show_timestamp_milliseconds;
        let (cell_w, cell_h) = self
            .terminal_cell_metrics
            .unwrap_or(((font_size * 0.6).max(6.0), (font_size * 1.35).max(12.0)));
        let visual_bell = is_active && self.terminal_runtime.visual_bell_ticks > 0;
        let Some(snapshot) = snapshot else {
            if let Some(request_offset) = terminal_scroll_snapshot_request_offset(
                scroll_offset,
                scroll_residual_lines,
                scrollback_len,
            ) {
                self.request_terminal_frame_snapshot_for_user_scroll(session_id, request_offset);
                if user_scroll_active {
                    if self.sync_terminal_scroll_text_first_surface_paint(session_id, cx) {
                        return;
                    }
                } else if self
                    .terminal_views
                    .get(session_id)
                    .is_some_and(|view| view.scrollback_snapshots.contains_key(&request_offset))
                    && self.sync_terminal_scroll_text_first_surface_paint(session_id, cx)
                {
                    return;
                }
                if self
                    .should_log_slow_diagnostic("terminal_scroll_snapshot_missing", Instant::now())
                {
                    tracing::warn!(
                        diagnostic = "terminal_scroll_snapshot_missing",
                        session_id = %session_id,
                        offset = request_offset,
                        "terminal scrolled paint retained current surface while waiting for snapshot"
                    );
                }
            }
            surface.update(cx, |surface, cx| {
                let mut changed = false;
                changed |= surface.set_layout_cache(layout_cache);
                changed |= surface.set_background_transparent(transparent_background);
                changed |= surface.set_paint_chrome(
                    palette,
                    font_family,
                    font_size,
                    normal_weight,
                    bold_weight,
                    cell_w,
                    cell_h,
                    show_line_numbers,
                    show_timestamps,
                    show_timestamp_ms,
                    is_active,
                    visual_bell,
                );
                changed |= surface.set_protocol_state(protocol_state);
                changed |= surface.update_scroll_chrome_without_snapshot(
                    scroll_offset,
                    scroll_residual_lines,
                    display_offset,
                    scrollback_len,
                    viewport_rows,
                    has_new,
                    performance_overlay,
                    skipped,
                );
                if changed {
                    cx.notify();
                }
            });
            return;
        };
        let cursor_row = snapshot.cursor.row;
        let remote_cursor_visible = snapshot.cursor.visible
            && snapshot.cursor.shape != nyaterm_terminal::CursorShape::Hidden
            && cursor_row != usize::MAX;
        let blink_enabled = self.settings.cursor_blink || snapshot.cursor.blinking;
        let show_cursor = terminal_cursor_visible_for_display_offset(
            is_active,
            is_disconnected,
            display_offset,
            remote_cursor_visible,
            blink_enabled,
            self.terminal_runtime.cursor_blink_on,
        );
        let cursor_style = match snapshot.cursor.shape {
            nyaterm_terminal::CursorShape::Underline => "underline".to_string(),
            nyaterm_terminal::CursorShape::Beam => "bar".to_string(),
            nyaterm_terminal::CursorShape::Hidden => self.settings.cursor_style.clone(),
            nyaterm_terminal::CursorShape::Block => self.settings.cursor_style.clone(),
        };

        let search_mapping_started_at = Instant::now();
        let action_links_enabled =
            self.settings.terminal_action_links_enabled && !self.settings.terminal_low_latency_mode;
        let paint_policy = EffectiveTerminalPaintPolicy::resolve(
            is_active,
            render_degraded,
            render_output_pressure,
            burst,
            mode,
            action_links_enabled,
        );
        let enhanced = paint_policy.enhanced_decorations;
        let action_link_matcher_key = terminal_action_link_matcher_key(
            action_links_enabled,
            &self.settings.terminal_action_links_matchers,
        );
        let frame_action_links = frame_action_links
            .as_ref()
            .filter(|links| links.matcher_key == action_link_matcher_key);
        let needs_scroll_enrichment = super::buffer::terminal_scroll_enrichment_should_request(
            display_offset > 0,
            display_offset,
            action_links_enabled.then_some(action_link_matcher_key),
            view,
        );

        let mut search_ranges_by_line: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
        let mut active_search_ranges_by_line: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
        if enhanced
            && is_active
            && self.terminal_search_open
            && self.terminal_search_mode == TerminalSearchMode::Buffer
        {
            let search_matches = self.terminal_buffer_matches().unwrap_or_default();
            let (abs_start, abs_end) =
                crate::features::terminal_surface::terminal_snapshot_absolute_range(&snapshot);
            let active_match_abs = search_matches
                .get(
                    self.terminal_search_active_index
                        .min(search_matches.len().saturating_sub(1)),
                )
                .map(|search_match| search_match.line_index);
            for (match_index, search_match) in search_matches.iter().enumerate() {
                let abs = search_match.line_index;
                if abs < abs_start || abs >= abs_end {
                    continue;
                }
                let view_row = abs - abs_start;
                let range = (search_match.start_col, search_match.end_col);
                search_ranges_by_line
                    .entry(view_row)
                    .or_default()
                    .push(range);
                if Some(abs) == active_match_abs
                    && match_index
                        == self
                            .terminal_search_active_index
                            .min(search_matches.len().saturating_sub(1))
                {
                    active_search_ranges_by_line
                        .entry(view_row)
                        .or_default()
                        .push(range);
                }
            }
        }
        let search_mapping_duration = search_mapping_started_at.elapsed();

        let terminal_selection = if enhanced {
            is_active.then_some(self.terminal_selection).flatten()
        } else {
            None
        };
        let selection_viewport_anchor_row = terminal_selection
            .map(|selection| selection.viewport_anchor_row)
            .unwrap_or(0);
        let has_selection = terminal_selection.is_some();
        let has_search_decorations =
            !search_ranges_by_line.is_empty() || !active_search_ranges_by_line.is_empty();
        let has_frame_action_links = action_links_enabled
            && frame_action_links.is_some_and(|links| {
                links
                    .cell_ranges_by_line
                    .iter()
                    .any(|ranges| !ranges.is_empty())
            });
        let has_hyperlinks =
            action_links_enabled && snapshot.rows().iter().any(|row| !row.hyperlinks.is_empty());
        let include_command_marks = paint_policy.include_command_marks;
        let has_command_marks =
            include_command_marks && snapshot.rows().iter().any(|row| row.command_mark.is_some());
        let decorations_started_at = Instant::now();
        let decorations = if crate::features::terminal_surface::terminal_line_decorations_needed(
            has_selection,
            has_search_decorations,
            has_frame_action_links,
            has_hyperlinks,
            has_command_marks,
        ) {
            let include_action_links = action_links_enabled;
            let include_hyperlinks = action_links_enabled;
            let decoration_cache_key =
                crate::features::terminal_surface::terminal_line_decorations_cache_key(
                    &snapshot,
                    terminal_selection,
                    selection_viewport_anchor_row,
                    &search_ranges_by_line,
                    &active_search_ranges_by_line,
                    frame_action_links,
                    include_action_links,
                    include_hyperlinks,
                    include_command_marks,
                );
            let build = || {
                crate::features::terminal_surface::build_terminal_line_decorations(
                    &snapshot,
                    terminal_selection,
                    selection_viewport_anchor_row,
                    &search_ranges_by_line,
                    &active_search_ranges_by_line,
                    frame_action_links,
                    include_action_links,
                    include_hyperlinks,
                    include_command_marks,
                )
            };
            if let Some(view) = self.terminal_views.get(session_id) {
                view.render_cache
                    .line_decorations(decoration_cache_key, build)
            } else {
                build().into()
            }
        } else {
            std::sync::Arc::from(Vec::<TerminalLineDecorations>::new())
        };
        let decorations_duration = decorations_started_at.elapsed();

        if needs_scroll_enrichment {
            let _ = self
                .request_terminal_frame_snapshot_for_scroll_enrichment(session_id, display_offset);
        }

        let prep_duration = paint_started_at.elapsed();
        if display_offset > 0
            && (prep_duration >= TERMINAL_SCROLL_PAINT_SLOW_TOTAL
                || snapshot_duration >= TERMINAL_SCROLL_PAINT_SLOW_STAGE
                || search_mapping_duration >= TERMINAL_SCROLL_PAINT_SLOW_STAGE
                || decorations_duration >= TERMINAL_SCROLL_PAINT_SLOW_STAGE)
            && self.should_log_slow_diagnostic("terminal_scroll_paint_prepare", Instant::now())
        {
            tracing::warn!(
                diagnostic = "terminal_scroll_paint_prepare",
                session_id = %session_id,
                scroll_offset,
                display_offset,
                residual_lines = scroll_residual_lines,
                viewport_rows,
                scrollback_len,
                snapshot_rows = snapshot.row_count(),
                retained_snapshot_reused,
                retained_lookup_us = retained_lookup_duration.as_micros(),
                snapshot_us = snapshot_duration.as_micros(),
                search_mapping_us = search_mapping_duration.as_micros(),
                decorations_us = decorations_duration.as_micros(),
                total_us = prep_duration.as_micros(),
                "slow terminal scroll paint preparation"
            );
        }

        self.remember_terminal_scroll_window_snapshot(session_id, display_offset, &snapshot);
        surface.update(cx, |surface, cx| {
            let mut changed = false;
            changed |= surface.set_layout_cache(layout_cache);
            changed |= surface.set_background_transparent(transparent_background);
            changed |= surface.set_paint_chrome(
                palette,
                font_family,
                font_size,
                normal_weight,
                bold_weight,
                cell_w,
                cell_h,
                show_line_numbers,
                show_timestamps,
                show_timestamp_ms,
                is_active,
                visual_bell,
            );
            changed |= surface.set_protocol_state(protocol_state);
            let had_pending_local_scroll_sync = surface.has_pending_local_scroll_sync();
            let frame_applied = surface.apply_frame_snapshot(
                snapshot,
                scroll_offset,
                scroll_residual_lines,
                display_offset,
                scrollback_len,
                viewport_rows,
                has_new,
                performance_overlay,
                skipped,
                has_frame_action_links,
                show_cursor,
                cursor_style.clone(),
            );
            let paint_details_changed = if frame_applied {
                surface.set_decorations_and_keywords(
                    decorations,
                    keyword_rules,
                    show_cursor,
                    cursor_style,
                )
            } else if !render_degraded {
                surface.set_decorations_and_keywords(
                    decorations,
                    keyword_rules,
                    show_cursor,
                    cursor_style,
                )
            } else {
                surface.set_decorations_and_keywords_preserving_stale(
                    decorations,
                    keyword_rules,
                    show_cursor,
                    cursor_style,
                    true,
                )
            };
            if frame_applied || paint_details_changed {
                surface.schedule_keyword_highlights(clear_keyword_highlights, cx);
            }
            if changed || frame_applied || paint_details_changed || had_pending_local_scroll_sync {
                cx.notify();
            }
        });
    }

    pub(in crate::features) fn notify_terminal_scroll_position_only(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        if session_id.is_empty() {
            return;
        }
        let notify_started_at = Instant::now();
        let surface = self.ensure_terminal_surface(session_id, cx);
        let Some(view) = self.terminal_views.get(session_id) else {
            return;
        };
        let scroll_offset = view.scroll_offset;
        let scroll_residual_lines = self.terminal_scroll_residual_for_session(Some(session_id));
        let scrollback_len = view.scrollback_len_for_ui();
        let viewport_rows = view.viewport_rows_for_ui();
        let display_offset =
            terminal_visual_display_offset(scroll_offset, scroll_residual_lines, scrollback_len);
        let has_new = view.has_new_while_scrolled;
        let performance_overlay = view.performance_overlay;
        let skipped = view.skipped_output_chars;
        let desired_scroll_state = TerminalScrollVisualState {
            session_id: session_id.to_string(),
            scroll_offset,
            scroll_residual_lines,
            display_offset,
            scrollback_len,
            viewport_rows,
            has_new_while_scrolled: has_new,
            performance_overlay,
            skipped_output_chars: skipped,
        };
        let (can_reuse_snapshot, scroll_state_current) = {
            let surface = surface.read(cx);
            (
                surface.has_snapshot_covering_display_offset(
                    display_offset,
                    viewport_rows,
                    scrollback_len,
                ),
                surface.scroll_visual_state_matches(&desired_scroll_state),
            )
        };
        if can_reuse_snapshot && scroll_state_current {
            return;
        }
        if !can_reuse_snapshot {
            if display_offset > 0 {
                self.request_terminal_frame_snapshot_for_user_scroll(session_id, display_offset);
                if self.sync_terminal_scroll_text_first_surface_paint(session_id, cx) {
                    let elapsed = notify_started_at.elapsed();
                    if elapsed >= TERMINAL_SCROLL_POSITION_NOTIFY_SLOW
                        && self.should_log_slow_diagnostic(
                            "terminal_scroll_position_notify",
                            Instant::now(),
                        )
                    {
                        tracing::warn!(
                            diagnostic = "terminal_scroll_position_notify",
                            session_id = %session_id,
                            scroll_offset,
                            display_offset,
                            residual_lines = scroll_residual_lines,
                            viewport_rows,
                            scrollback_len,
                            can_reuse_snapshot = false,
                            text_first = true,
                            elapsed_us = elapsed.as_micros(),
                            "slow terminal scroll position notify"
                        );
                    }
                    return;
                }
                if self.should_log_slow_diagnostic("terminal_scroll_snapshot_wait", Instant::now())
                {
                    tracing::warn!(
                        diagnostic = "terminal_scroll_snapshot_wait",
                        session_id = %session_id,
                        offset = display_offset,
                        "terminal scroll retained current surface while waiting for target snapshot"
                    );
                }
            }
            surface.update(cx, |surface, cx| {
                let changed = surface.update_scroll_chrome_without_snapshot(
                    scroll_offset,
                    scroll_residual_lines,
                    display_offset,
                    scrollback_len,
                    viewport_rows,
                    has_new,
                    performance_overlay,
                    skipped,
                );
                if changed {
                    cx.notify();
                }
            });
            let elapsed = notify_started_at.elapsed();
            if elapsed >= TERMINAL_SCROLL_POSITION_NOTIFY_SLOW
                && self
                    .should_log_slow_diagnostic("terminal_scroll_position_notify", Instant::now())
            {
                tracing::warn!(
                    diagnostic = "terminal_scroll_position_notify",
                    session_id = %session_id,
                    scroll_offset,
                    display_offset,
                    residual_lines = scroll_residual_lines,
                    viewport_rows,
                    scrollback_len,
                    can_reuse_snapshot = false,
                    elapsed_us = elapsed.as_micros(),
                    "slow terminal scroll position notify"
                );
            }
            return;
        }
        surface.update(cx, |surface, cx| {
            let changed = surface.update_scroll_position_without_snapshot(
                scroll_offset,
                scroll_residual_lines,
                display_offset,
                scrollback_len,
                viewport_rows,
                has_new,
                performance_overlay,
                skipped,
            );
            if changed {
                cx.notify();
            }
        });
        let elapsed = notify_started_at.elapsed();
        if elapsed >= TERMINAL_SCROLL_POSITION_NOTIFY_SLOW
            && self.should_log_slow_diagnostic("terminal_scroll_position_notify", Instant::now())
        {
            tracing::warn!(
                diagnostic = "terminal_scroll_position_notify",
                session_id = %session_id,
                scroll_offset,
                display_offset,
                residual_lines = scroll_residual_lines,
                viewport_rows,
                scrollback_len,
                can_reuse_snapshot = true,
                elapsed_us = elapsed.as_micros(),
                "slow terminal scroll position notify"
            );
        }
    }

    pub(in crate::features) fn notify_terminal_scroll_visual_only(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        if session_id.is_empty() {
            return;
        }
        let notify_started_at = Instant::now();
        let surface = self.ensure_terminal_surface(session_id, cx);
        let Some(view) = self.terminal_views.get(session_id) else {
            return;
        };
        let scroll_offset = view.scroll_offset;
        let scroll_residual_lines = self.terminal_scroll_residual_for_session(Some(session_id));
        let scrollback_len = view.scrollback_len_for_ui();
        let viewport_rows = view.viewport_rows_for_ui();
        let display_offset =
            terminal_visual_display_offset(scroll_offset, scroll_residual_lines, scrollback_len);
        let has_new = view.has_new_while_scrolled;
        let performance_overlay = view.performance_overlay;
        let skipped = view.skipped_output_chars;
        let desired_scroll_state = TerminalScrollVisualState {
            session_id: session_id.to_string(),
            scroll_offset,
            scroll_residual_lines,
            display_offset,
            scrollback_len,
            viewport_rows,
            has_new_while_scrolled: has_new,
            performance_overlay,
            skipped_output_chars: skipped,
        };
        let (can_reuse_snapshot, scroll_state_current) = {
            let surface = surface.read(cx);
            (
                surface.has_snapshot_covering_display_offset(
                    display_offset,
                    viewport_rows,
                    scrollback_len,
                ),
                surface.scroll_visual_state_matches(&desired_scroll_state),
            )
        };
        if can_reuse_snapshot && scroll_state_current {
            return;
        }
        if !can_reuse_snapshot && display_offset > 0 {
            self.request_terminal_frame_snapshot_for_user_scroll(session_id, display_offset);
        }
        surface.update(cx, |surface, cx| {
            let changed = if can_reuse_snapshot {
                surface.update_scroll_position_without_snapshot(
                    scroll_offset,
                    scroll_residual_lines,
                    display_offset,
                    scrollback_len,
                    viewport_rows,
                    has_new,
                    performance_overlay,
                    skipped,
                )
            } else {
                surface.update_scroll_chrome_without_snapshot(
                    scroll_offset,
                    scroll_residual_lines,
                    display_offset,
                    scrollback_len,
                    viewport_rows,
                    has_new,
                    performance_overlay,
                    skipped,
                )
            };
            if changed {
                cx.notify();
            }
        });
        let elapsed = notify_started_at.elapsed();
        if elapsed >= TERMINAL_SCROLL_POSITION_NOTIFY_SLOW
            && self.should_log_slow_diagnostic("terminal_scroll_visual_notify", Instant::now())
        {
            tracing::warn!(
                diagnostic = "terminal_scroll_visual_notify",
                session_id = %session_id,
                scroll_offset,
                display_offset,
                residual_lines = scroll_residual_lines,
                viewport_rows,
                scrollback_len,
                can_reuse_snapshot,
                elapsed_us = elapsed.as_micros(),
                "slow terminal scroll visual notify"
            );
        }
    }

    /// Notify surface only (no full shell). Used for cursor blink / visual bell.
    pub(in crate::features) fn notify_active_terminal_surface(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.active_session_id.clone() else {
            return;
        };
        self.sync_terminal_surface_paint(&session_id, cx);
    }

    /// Surface-only repaint for the given session (scroll / selection / frame).
    pub(in crate::features) fn notify_terminal_surface_only(
        &mut self,
        session_id: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        let session_id = session_id
            .map(str::to_string)
            .or_else(|| self.active_session_id.clone());
        let Some(session_id) = session_id else {
            return;
        };
        if session_id.is_empty() {
            return;
        }
        self.sync_terminal_surface_paint(&session_id, cx);
    }

    pub(in crate::features) fn notify_terminal_selection_visual_only(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        if session_id.is_empty() {
            return;
        }
        let Some(surface) = self.terminal_surfaces.get(session_id).cloned() else {
            self.sync_terminal_surface_paint(session_id, cx);
            return;
        };
        let selection = self.terminal_selection;
        let visual_state_ready = surface.update(cx, |surface, cx| {
            if surface.set_selection_visual(selection) {
                cx.notify();
                true
            } else {
                surface.has_snapshot()
            }
        });
        if !visual_state_ready {
            self.sync_terminal_surface_paint(session_id, cx);
        }
    }
}

fn terminal_key_bytes_for_mode_and_settings(
    event: &KeyDownEvent,
    mode: TerminalKeyMode,
    alt_as_meta: bool,
) -> Option<Vec<u8>> {
    // Prefer structured CSI for modified arrows (Ctrl/Alt) from terminal_key_bytes.
    if let Some(bytes) = terminal_key_bytes_with_mode(event, mode) {
        return Some(bytes);
    }
    // Alt-as-meta: ESC + character for shell word ops (Alt+b/f/d, etc.).
    if alt_as_meta
        && event.keystroke.modifiers.alt
        && !event.keystroke.modifiers.control
        && !event.keystroke.modifiers.platform
        && !event.keystroke.modifiers.function
        && let Some(input) = event.keystroke.key_char.as_deref()
        && !input.is_empty()
    {
        let mut bytes = Vec::with_capacity(input.len() + 1);
        bytes.push(0x1b);
        bytes.extend_from_slice(input.as_bytes());
        return Some(bytes);
    }
    // Even when alt-as-meta is off, still emit ESC+letter for Alt+b/f/d word ops
    // that shells commonly expect (Tauri XTerminal parity).
    if event.keystroke.modifiers.alt
        && !event.keystroke.modifiers.control
        && !event.keystroke.modifiers.platform
        && !event.keystroke.modifiers.function
        && !event.keystroke.modifiers.shift
    {
        let key = event.keystroke.key.as_str();
        if matches!(key, "b" | "B" | "f" | "F" | "d" | "D") {
            return Some(vec![0x1b, key.as_bytes()[0].to_ascii_lowercase()]);
        }
        if let Some(input) = event.keystroke.key_char.as_deref() {
            if input.len() == 1 {
                let ch = input.chars().next().unwrap();
                if matches!(ch, 'b' | 'B' | 'f' | 'F' | 'd' | 'D') {
                    return Some(vec![0x1b, ch.to_ascii_lowercase() as u8]);
                }
            }
        }
    }
    None
}

fn terminal_should_defer_key_text_to_input_handler_for_state(
    ime_compatibility: bool,
    marked_text: &str,
    event: &KeyDownEvent,
) -> bool {
    if !ime_compatibility
        || !event
            .keystroke
            .modifiers
            .is_subset_of(&gpui::Modifiers::shift())
    {
        return false;
    }
    if !marked_text.is_empty() {
        return true;
    }
    event.keystroke.key_char.as_deref().is_some_and(|input| {
        !input.is_empty()
            && input.chars().all(|ch| !ch.is_control())
            && input.chars().any(|ch| !ch.is_ascii())
    })
}

fn terminal_status_changed(current: &str, next: &str) -> bool {
    current != next
}

#[allow(clippy::too_many_arguments)]
fn log_slow_terminal_input_diagnostic(
    kind: &'static str,
    byte_count: usize,
    synced: usize,
    failed: usize,
    total_duration: Duration,
    encode_duration: Duration,
    write_duration: Duration,
    suggestion_duration: Duration,
    history_duration: Duration,
    notify_duration: Duration,
) {
    if total_duration < TERMINAL_INPUT_SLOW_THRESHOLD {
        return;
    }
    tracing::warn!(
        diagnostic = "terminal_input_slow",
        kind,
        byte_count,
        synced,
        failed,
        total_us = total_duration.as_micros(),
        encode_us = encode_duration.as_micros(),
        write_us = write_duration.as_micros(),
        suggestion_us = suggestion_duration.as_micros(),
        history_us = history_duration.as_micros(),
        notify_us = notify_duration.as_micros(),
        "slow terminal input path"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_event(key: &str, key_char: Option<&str>, modifiers: gpui::Modifiers) -> KeyDownEvent {
        KeyDownEvent {
            keystroke: gpui::Keystroke {
                modifiers,
                key: key.to_string(),
                key_char: key_char.map(str::to_string),
            },
            is_held: false,
        }
    }

    fn terminal_output_lines(count: usize) -> String {
        (0..count)
            .map(|index| format!("line {index:03}\n"))
            .collect::<String>()
    }

    #[test]
    fn terminal_paint_snapshot_waits_without_authoritative_scrollback_snapshot() {
        let view = TerminalViewState::from_output(terminal_output_lines(40));

        assert!(terminal_paint_snapshot_for_view(Some(&view), 4, None).is_none());
    }

    #[test]
    fn terminal_paint_snapshot_can_retain_matching_surface_snapshot() {
        let view = TerminalViewState::from_output(terminal_output_lines(40));
        let retained = std::sync::Arc::new(view.screen.viewport_snapshot(4));

        let snapshot = terminal_paint_snapshot_for_view(Some(&view), 4, Some(retained))
            .expect("matching retained surface snapshot should be usable");

        assert_eq!(snapshot.display_offset, 4);
    }

    #[test]
    fn terminal_paint_snapshot_does_not_use_ui_screen_fallback() {
        let view = TerminalViewState::from_output(terminal_output_lines(40));

        assert!(terminal_paint_snapshot_for_view(Some(&view), 4, None).is_none());
    }

    #[test]
    fn terminal_paint_window_snapshot_prefers_latest_live_frame_over_retained_surface() {
        let mut old_view = TerminalViewState::from_output(terminal_output_lines(40));
        old_view.frame_snapshot = Some(std::sync::Arc::new(old_view.screen.viewport_snapshot(0)));
        let mut view = TerminalViewState::from_output(terminal_output_lines(42));
        view.frame_snapshot = Some(std::sync::Arc::new(view.screen.viewport_snapshot(0)));
        let retained = old_view.frame_snapshot.clone();
        let latest = view
            .frame_snapshot
            .clone()
            .expect("view should have live frame snapshot");

        let snapshot = terminal_paint_window_snapshot_for_view(
            Some(&view),
            0,
            view.viewport_rows_for_ui(),
            retained,
        )
        .expect("live frame snapshot should be available");

        assert!(std::sync::Arc::ptr_eq(&snapshot, &latest));
    }

    #[test]
    fn terminal_paint_window_keeps_worker_snapshot_when_ui_screen_is_stale() {
        let view = TerminalViewState::from_output(terminal_output_lines(8));
        let mut worker_screen = TerminalScreen::default();
        worker_screen.advance_decoded_text(&terminal_output_lines(160));
        let worker_snapshot = std::sync::Arc::new(worker_screen.snapshot());
        let mut view = view;
        view.frame_snapshot = Some(worker_snapshot.clone());

        let snapshot = terminal_paint_window_snapshot_for_view(
            Some(&view),
            0,
            view.viewport_rows_for_ui(),
            None,
        )
        .expect("authoritative worker snapshot should be paintable");

        assert!(std::sync::Arc::ptr_eq(&snapshot, &worker_snapshot));
        assert!(
            snapshot
                .rows()
                .iter()
                .any(|row| row.text.contains("line 159"))
        );
    }

    #[test]
    fn terminal_retained_snapshot_uses_worker_scrollback_length() {
        let view = TerminalViewState::from_output(terminal_output_lines(8));
        let mut worker_screen = TerminalScreen::default();
        worker_screen.advance_decoded_text(&terminal_output_lines(160));
        let worker_snapshot = std::sync::Arc::new(worker_screen.snapshot());
        let mut view = view;
        view.frame_snapshot = Some(worker_snapshot.clone());

        assert!(terminal_retained_snapshot_matches_view(
            worker_snapshot.as_ref(),
            &view,
            0,
            view.viewport_rows_for_ui(),
        ));
    }

    #[test]
    fn terminal_paint_window_waits_for_worker_after_height_resize() {
        let mut view = TerminalViewState::from_output(terminal_output_lines(80));
        let old_snapshot = std::sync::Arc::new(view.screen.viewport_snapshot(0));
        let old_rows = old_snapshot.row_count();
        view.frame_snapshot = Some(old_snapshot.clone());

        view.screen
            .resize(view.screen.cols() as u16, (old_rows + 16) as u16);
        view.grid_resize_pending = true;
        let viewport_rows = view.viewport_rows_for_ui();
        assert!(viewport_rows > old_rows);

        let snapshot = terminal_paint_window_snapshot_for_view(Some(&view), 0, viewport_rows, None);

        assert!(snapshot.is_none());
        assert!(std::sync::Arc::ptr_eq(
            view.frame_snapshot.as_ref().unwrap(),
            &old_snapshot,
        ));
    }

    #[test]
    fn terminal_paint_window_waits_for_worker_after_width_resize() {
        let mut view = TerminalViewState::from_output(terminal_output_lines(80));
        let old_snapshot = std::sync::Arc::new(view.screen.viewport_snapshot(0));
        let old_cols = old_snapshot.cols;
        view.frame_snapshot = Some(old_snapshot.clone());

        view.screen
            .resize((old_cols + 24) as u16, view.screen.rows() as u16);
        view.grid_resize_pending = true;
        let viewport_rows = view.viewport_rows_for_ui();

        let snapshot = terminal_paint_window_snapshot_for_view(Some(&view), 0, viewport_rows, None);

        assert!(snapshot.is_none());
        assert_eq!(view.frame_snapshot.as_ref().unwrap().cols, old_cols);
    }

    #[test]
    fn terminal_retained_snapshot_accepts_current_scroll_window_geometry() {
        let view = TerminalViewState::from_output(terminal_output_lines(80));
        let display_offset = 4;
        let snapshot = view.snapshot_with_scroll_window(display_offset);
        let viewport_rows = view.viewport_rows_for_ui();

        assert!(terminal_retained_snapshot_matches_view(
            snapshot.as_ref(),
            &view,
            display_offset,
            viewport_rows,
        ));
    }

    #[test]
    fn terminal_retained_snapshot_rejects_pre_resize_geometry() {
        let mut view = TerminalViewState::from_output(terminal_output_lines(80));
        let display_offset = 4;
        let snapshot = view.snapshot_with_scroll_window(display_offset);
        let viewport_rows = view.viewport_rows_for_ui();
        let resized_cols = view.screen.cols().saturating_add(8) as u16;
        let resized_rows = view.screen.rows().saturating_add(6) as u16;

        view.screen.resize(resized_cols, resized_rows);

        assert!(!terminal_retained_snapshot_matches_view(
            snapshot.as_ref(),
            &view,
            display_offset,
            viewport_rows,
        ));
    }

    #[test]
    fn terminal_visual_display_offset_keeps_text_window_stable_for_fractional_scroll() {
        assert_eq!(terminal_visual_display_offset(0, 0.0, 10), 0);
        assert_eq!(terminal_visual_display_offset(0, 0.25, 10), 0);
        assert_eq!(terminal_visual_display_offset(0, 0.5, 10), 0);
        assert_eq!(terminal_visual_display_offset(0, 0.95, 10), 0);
        assert_eq!(terminal_visual_display_offset(4, -0.25, 10), 4);
        assert_eq!(terminal_visual_display_offset(4, -0.6, 10), 4);
        assert_eq!(terminal_visual_display_offset(10, 0.5, 10), 10);
    }

    #[test]
    fn terminal_scroll_snapshot_request_offset_waits_for_stable_text_offset() {
        assert_eq!(terminal_scroll_snapshot_request_offset(0, 0.0, 10), None);
        assert_eq!(terminal_scroll_snapshot_request_offset(0, 0.49, 10), None);
        assert_eq!(terminal_scroll_snapshot_request_offset(0, 0.5, 10), None);
        assert_eq!(
            terminal_scroll_snapshot_request_offset(4, -0.25, 10),
            Some(4)
        );
        assert_eq!(
            terminal_scroll_snapshot_request_offset(10, 0.5, 10),
            Some(10)
        );
    }

    #[test]
    fn terminal_cursor_visibility_uses_display_offset_not_raw_scroll_offset() {
        assert!(terminal_cursor_visible_for_display_offset(
            true, false, 0, true, false, false
        ));
        assert!(!terminal_cursor_visible_for_display_offset(
            true, false, 1, true, false, false
        ));
        assert!(!terminal_cursor_visible_for_display_offset(
            true, false, 0, true, true, false
        ));
    }

    #[test]
    fn terminal_snapshot_edge_row_extends_fractional_scroll_window() {
        let view = TerminalViewState::from_output(terminal_output_lines(40));
        let base = std::sync::Arc::new(view.screen.viewport_snapshot(1));
        let newer = std::sync::Arc::new(view.screen.viewport_snapshot(0));
        let base_rows = base.row_count();
        let newer_tail = newer.rows().last().map(|row| row.text.clone());

        let snapshot = terminal_snapshot_with_newer_edge_row(base, newer);

        assert_eq!(snapshot.row_count(), base_rows + 1);
        assert_eq!(
            snapshot.rows().last().map(|row| row.text.clone()),
            newer_tail
        );
        assert!(
            snapshot
                .rows()
                .iter()
                .all(|row| row.cells.len() == snapshot.cols)
        );
    }

    #[test]
    fn terminal_snapshot_edge_row_preserves_absolute_range_start() {
        let view = TerminalViewState::from_output(terminal_output_lines(40));
        let base = std::sync::Arc::new(view.screen.viewport_snapshot(1));
        let newer = std::sync::Arc::new(view.screen.viewport_snapshot(0));
        let (base_start, _) =
            crate::features::terminal_surface::terminal_snapshot_absolute_range(base.as_ref());

        let snapshot = terminal_snapshot_with_newer_edge_row(base, newer);
        let (start, end) =
            crate::features::terminal_surface::terminal_snapshot_absolute_range(snapshot.as_ref());

        assert_eq!(start, base_start);
        assert_eq!(
            end,
            snapshot.total_rows.saturating_sub(snapshot.display_offset)
        );
    }

    #[test]
    fn terminal_paint_window_snapshot_reuses_cached_retained_window_for_neighbor_offsets() {
        let mut view = TerminalViewState::from_output(terminal_output_lines(80));
        let display_offset = 6;
        let viewport_rows = view.viewport_rows_for_ui();
        let scrollback_len = view.scrollback_len_for_ui();
        let retained = terminal_snapshot_with_retained_scroll_window(
            &view,
            std::sync::Arc::new(view.screen.viewport_snapshot(display_offset)),
            display_offset,
            viewport_rows,
            scrollback_len,
        );
        view.scrollback_snapshots.insert(display_offset, retained);

        let snapshot = terminal_paint_window_snapshot_for_view(
            Some(&view),
            display_offset,
            viewport_rows,
            None,
        )
        .expect("window snapshot should be available");

        assert!(snapshot.row_count() > viewport_rows);
        assert!(terminal_snapshot_covers_display_offset(
            snapshot.as_ref(),
            display_offset,
            viewport_rows,
            scrollback_len
        ));
        assert!(terminal_snapshot_covers_display_offset(
            snapshot.as_ref(),
            display_offset - 1,
            viewport_rows,
            scrollback_len
        ));
        assert!(terminal_snapshot_covers_display_offset(
            snapshot.as_ref(),
            display_offset + 1,
            viewport_rows,
            scrollback_len
        ));
        assert!(
            terminal_snapshot_anchor_row_for_display_offset(
                snapshot.as_ref(),
                display_offset,
                viewport_rows,
                scrollback_len
            ) > 0
        );
    }

    #[test]
    fn terminal_paint_window_snapshot_reuses_covering_cached_retained_window() {
        let mut view = TerminalViewState::from_output(terminal_output_lines(160));
        let cached_offset = 40;
        let target_offset = cached_offset + 2;
        let viewport_rows = view.viewport_rows_for_ui();
        let scrollback_len = view.scrollback_len_for_ui();
        let retained = terminal_snapshot_with_retained_scroll_window(
            &view,
            std::sync::Arc::new(view.screen.viewport_snapshot(cached_offset)),
            cached_offset,
            viewport_rows,
            scrollback_len,
        );
        assert!(terminal_snapshot_covers_display_offset(
            retained.as_ref(),
            target_offset,
            viewport_rows,
            scrollback_len
        ));
        view.scrollback_snapshots
            .insert(cached_offset, retained.clone());

        let snapshot = terminal_paint_window_snapshot_for_view(
            Some(&view),
            target_offset,
            viewport_rows,
            None,
        )
        .expect("covering retained window should be reused");

        assert!(std::sync::Arc::ptr_eq(&snapshot, &retained));
        let anchor = terminal_snapshot_anchor_row_for_display_offset(
            snapshot.as_ref(),
            target_offset,
            viewport_rows,
            scrollback_len,
        );
        assert_eq!(
            snapshot.line(anchor),
            view.screen.viewport_snapshot(target_offset).line(0)
        );
    }

    #[test]
    fn terminal_paint_window_snapshot_reuses_cached_retained_window_without_rewrapping() {
        let mut view = TerminalViewState::from_output(terminal_output_lines(120));
        let display_offset = 12;
        let viewport_rows = view.viewport_rows_for_ui();
        let scrollback_len = view.scrollback_len_for_ui();
        let base = std::sync::Arc::new(view.screen.viewport_snapshot(display_offset));
        let retained = terminal_snapshot_with_retained_scroll_window(
            &view,
            base,
            display_offset,
            viewport_rows,
            scrollback_len,
        );
        let retained_rows = retained.row_count();
        view.scrollback_snapshots
            .insert(display_offset, retained.clone());

        let snapshot = terminal_paint_window_snapshot_for_view(
            Some(&view),
            display_offset,
            viewport_rows,
            None,
        )
        .expect("cached retained window should be reusable");

        assert_eq!(snapshot.row_count(), retained_rows);
        assert!(std::sync::Arc::ptr_eq(&snapshot, &retained));
        assert!(terminal_snapshot_covers_display_offset(
            snapshot.as_ref(),
            display_offset,
            viewport_rows,
            scrollback_len
        ));
    }

    #[test]
    fn terminal_paint_window_snapshot_covers_viewport_sized_scroll_runs() {
        let mut view = TerminalViewState::from_output(terminal_output_lines(160));
        let display_offset = 40;
        let viewport_rows = view.viewport_rows_for_ui();
        let scrollback_len = view.scrollback_len_for_ui();
        let base = std::sync::Arc::new(view.screen.viewport_snapshot(display_offset));
        let retained = terminal_snapshot_with_retained_scroll_window(
            &view,
            base,
            display_offset,
            viewport_rows,
            scrollback_len,
        );
        view.scrollback_snapshots.insert(display_offset, retained);

        let snapshot = terminal_paint_window_snapshot_for_view(
            Some(&view),
            display_offset,
            viewport_rows,
            None,
        )
        .expect("cached window snapshot should cover direct scroll runs");

        assert!(viewport_rows >= 16);
        assert!(terminal_snapshot_covers_display_offset(
            snapshot.as_ref(),
            display_offset.saturating_sub(viewport_rows),
            viewport_rows,
            scrollback_len
        ));
        assert!(terminal_snapshot_covers_display_offset(
            snapshot.as_ref(),
            display_offset,
            viewport_rows,
            scrollback_len
        ));
        assert!(terminal_snapshot_covers_display_offset(
            snapshot.as_ref(),
            display_offset + viewport_rows,
            viewport_rows,
            scrollback_len
        ));
    }

    #[test]
    fn terminal_paint_window_snapshot_covers_multi_viewport_fast_scroll_runs() {
        let mut view = TerminalViewState::from_output(terminal_output_lines(240));
        let display_offset = 80;
        let viewport_rows = view.viewport_rows_for_ui();
        let scrollback_len = view.scrollback_len_for_ui();
        let fast_delta = viewport_rows.saturating_mul(2);
        let base = std::sync::Arc::new(view.screen.viewport_snapshot(display_offset));
        let retained = terminal_snapshot_with_retained_scroll_window(
            &view,
            base,
            display_offset,
            viewport_rows,
            scrollback_len,
        );
        view.scrollback_snapshots.insert(display_offset, retained);

        let snapshot = terminal_paint_window_snapshot_for_view(
            Some(&view),
            display_offset,
            viewport_rows,
            None,
        )
        .expect("cached window snapshot should cover multi-viewport fast scroll runs");

        assert!(
            snapshot
                .rows()
                .iter()
                .all(|row| row.cells.len() == snapshot.cols)
        );
        assert!(terminal_snapshot_covers_display_offset(
            snapshot.as_ref(),
            display_offset.saturating_sub(fast_delta),
            viewport_rows,
            scrollback_len
        ));
        assert!(terminal_snapshot_covers_display_offset(
            snapshot.as_ref(),
            display_offset,
            viewport_rows,
            scrollback_len
        ));
        assert!(terminal_snapshot_covers_display_offset(
            snapshot.as_ref(),
            display_offset + fast_delta,
            viewport_rows,
            scrollback_len
        ));

        for offset in [
            display_offset.saturating_sub(fast_delta),
            display_offset,
            display_offset + fast_delta,
        ] {
            let anchor = terminal_snapshot_anchor_row_for_display_offset(
                snapshot.as_ref(),
                offset,
                viewport_rows,
                scrollback_len,
            );
            assert_eq!(
                snapshot.line(anchor),
                view.screen.viewport_snapshot(offset).line(0)
            );
        }
    }

    #[test]
    fn terminal_scroll_retained_window_extra_rows_covers_fast_scroll_runs() {
        assert_eq!(terminal_scroll_retained_window_extra_rows(12), 32);
        assert_eq!(terminal_scroll_retained_window_extra_rows(40), 80);
        assert_eq!(terminal_scroll_retained_window_extra_rows(120), 192);
    }

    #[test]
    fn terminal_paint_window_snapshot_waits_without_cached_snapshot() {
        let view = TerminalViewState::from_output(terminal_output_lines(80));
        let display_offset = 6;
        let viewport_rows = view.viewport_rows_for_ui();

        assert!(
            terminal_paint_window_snapshot_for_view(
                Some(&view),
                display_offset,
                viewport_rows,
                None,
            )
            .is_none()
        );
    }

    #[test]
    fn terminal_paint_window_snapshot_reuses_cached_authoritative_snapshot() {
        let mut view = TerminalViewState::from_output(terminal_output_lines(80));
        let display_offset = 6;
        let viewport_rows = view.viewport_rows_for_ui();
        let scrollback_len = view.scrollback_len_for_ui();
        let retained = terminal_snapshot_with_retained_scroll_window(
            &view,
            std::sync::Arc::new(view.screen.viewport_snapshot(display_offset)),
            display_offset,
            viewport_rows,
            scrollback_len,
        );
        view.scrollback_snapshots.insert(display_offset, retained);

        let snapshot = terminal_paint_window_snapshot_for_view(
            Some(&view),
            display_offset,
            viewport_rows,
            None,
        )
        .expect("cached scrolled paint window should be used");

        assert!(terminal_snapshot_covers_display_offset(
            snapshot.as_ref(),
            display_offset,
            viewport_rows,
            scrollback_len
        ));
        assert_eq!(
            snapshot.line(terminal_snapshot_anchor_row_for_display_offset(
                snapshot.as_ref(),
                display_offset,
                viewport_rows,
                scrollback_len
            )),
            view.screen.viewport_snapshot(display_offset).line(0)
        );
    }

    #[test]
    fn terminal_paint_window_snapshot_preserves_view_absolute_start() {
        let mut view = TerminalViewState::from_output(terminal_output_lines(80));
        let display_offset = 4;
        let viewport_rows = view.viewport_rows_for_ui();
        let base = std::sync::Arc::new(view.screen.viewport_snapshot(display_offset));
        view.scrollback_snapshots
            .insert(display_offset, base.clone());
        let (base_start, _) =
            crate::features::terminal_surface::terminal_snapshot_absolute_range(base.as_ref());

        let snapshot = terminal_paint_window_snapshot_for_view(
            Some(&view),
            display_offset,
            viewport_rows,
            None,
        )
        .expect("window snapshot should be available");
        let anchor = terminal_snapshot_anchor_row_for_display_offset(
            snapshot.as_ref(),
            display_offset,
            viewport_rows,
            view.scrollback_len_for_ui(),
        );
        let (window_start, _) =
            crate::features::terminal_surface::terminal_snapshot_absolute_range(snapshot.as_ref());

        assert_eq!(window_start + anchor, base_start);
    }

    #[test]
    fn terminal_scroll_text_first_decorations_keep_search_overlay_without_links() {
        let view = TerminalViewState::from_output(terminal_output_lines(80));
        let snapshot = view.screen.viewport_snapshot(6);
        let (abs_start, _) =
            crate::features::terminal_surface::terminal_snapshot_absolute_range(&snapshot);
        let matches = vec![TerminalBufferMatch {
            line_index: abs_start + 1,
            start_col: 2,
            end_col: 6,
        }];

        let decorations = terminal_scroll_text_first_decorations(
            &snapshot,
            Some(matches.as_slice()),
            None,
            true,
            true,
            false,
        );

        assert_eq!(decorations.len(), snapshot.row_count());
        assert_eq!(decorations[1].search_ranges, vec![(2, 6)]);
        assert!(decorations[1].active_search_ranges.is_empty());
        assert!(decorations[1].link_ranges.is_empty());
        assert!(decorations.iter().all(|line| line.selection_cols.is_none()));
    }

    #[test]
    fn terminal_scroll_text_first_decorations_include_links_for_current_snapshot() {
        let mut screen = TerminalScreen::new(40, 3);
        screen.advance(b"\x1b]8;;https://example.com\x07click\x1b]8;;\x07 plain");
        let snapshot = screen.viewport_snapshot(0);
        let mut links = TerminalFrameActionLinks {
            matcher_key: 1,
            matches_by_line: vec![Vec::new(); snapshot.row_count()],
            cell_ranges_by_line: vec![Vec::new(); snapshot.row_count()],
        };
        links.cell_ranges_by_line[0].push((6, 11));

        let decorations = terminal_scroll_text_first_decorations(
            &snapshot,
            None,
            Some(&links),
            true,
            true,
            false,
        );

        assert_eq!(decorations.len(), snapshot.row_count());
        assert_eq!(decorations[0].link_ranges, vec![(6, 11), (0, 5)]);
        assert!(decorations[0].search_ranges.is_empty());
        assert!(decorations.iter().all(|line| line.selection_cols.is_none()));
    }

    #[test]
    fn terminal_keyword_highlight_updates_stay_enabled_for_active_surface() {
        assert!(terminal_keyword_highlight_updates_allowed(true));
    }

    #[test]
    fn terminal_keyword_highlight_updates_pause_for_inactive_surface() {
        assert!(!terminal_keyword_highlight_updates_allowed(false));
    }

    #[test]
    fn terminal_user_scroll_active_requires_scrolled_surface_and_recent_input() {
        let now = Instant::now();

        assert!(!terminal_user_scroll_active(0, true, Some(now), now));
        assert!(terminal_user_scroll_active(4, true, Some(now), now));
        assert!(!terminal_user_scroll_active(4, false, Some(now), now));
        assert!(!terminal_user_scroll_active(
            4,
            true,
            Some(now - TERMINAL_USER_SCROLL_ACTIVE_WINDOW - Duration::from_millis(1)),
            now,
        ));
        assert!(!terminal_user_scroll_active(4, true, None, now));
    }

    #[test]
    fn terminal_input_latency_active_uses_short_idle_window() {
        let now = Instant::now();

        assert!(terminal_input_latency_active(Some(now), now));
        assert!(!terminal_input_latency_active(
            Some(now - TERMINAL_INPUT_LATENCY_WINDOW - Duration::from_millis(1)),
            now,
        ));
        assert!(!terminal_input_latency_active(None, now));
    }

    #[test]
    fn terminal_command_suggestion_input_tracking_skips_low_latency_mode() {
        assert!(terminal_should_track_command_suggestion_input(
            true, false, true
        ));
        assert!(!terminal_should_track_command_suggestion_input(
            true, true, true
        ));
        assert!(!terminal_should_track_command_suggestion_input(
            false, false, true
        ));
    }

    #[test]
    fn terminal_command_suggestion_input_tracking_skips_disabled_suggestions() {
        assert!(!terminal_should_track_command_suggestion_input(
            true, false, false
        ));
    }

    #[test]
    fn terminal_session_write_failure_log_escapes_control_text() {
        let log = terminal_session_write_failure_log("input", "closed\r\n\x1b[31m");

        assert_eq!(
            log,
            "\n# session write failed (input): closed\\r\\n\\x1b[31m\n"
        );
    }

    #[test]
    fn terminal_key_encoding_uses_target_session_mode() {
        let event = key_event("up", None, gpui::Modifiers::default());
        let normal =
            terminal_key_bytes_for_mode_and_settings(&event, TerminalKeyMode::default(), false)
                .unwrap();
        let application = terminal_key_bytes_for_mode_and_settings(
            &event,
            TerminalKeyMode {
                application_cursor: true,
                ..TerminalKeyMode::default()
            },
            false,
        )
        .unwrap();

        assert_eq!(normal, b"\x1b[A".to_vec());
        assert_eq!(application, b"\x1bOA".to_vec());
    }

    #[test]
    fn terminal_key_encoding_keeps_alt_meta_setting_outside_mode() {
        let event = key_event(
            "x",
            Some("x"),
            gpui::Modifiers {
                alt: true,
                ..gpui::Modifiers::default()
            },
        );

        assert_eq!(
            terminal_key_bytes_for_mode_and_settings(&event, TerminalKeyMode::default(), true,)
                .unwrap(),
            b"\x1bx".to_vec()
        );
        assert!(
            terminal_key_bytes_for_mode_and_settings(&event, TerminalKeyMode::default(), false,)
                .is_none()
        );
    }

    #[test]
    fn ime_defer_does_not_swallow_plain_space_without_marked_text() {
        let event = key_event("space", None, gpui::Modifiers::default());

        assert!(!terminal_should_defer_key_text_to_input_handler_for_state(
            true, "", &event
        ));
    }

    #[test]
    fn ime_defer_keeps_space_for_active_marked_text() {
        let event = key_event("space", None, gpui::Modifiers::default());

        assert!(terminal_should_defer_key_text_to_input_handler_for_state(
            true, "ni", &event
        ));
    }

    #[test]
    fn ime_defer_keeps_non_ascii_text_for_input_handler() {
        let event = key_event("あ", Some("あ"), gpui::Modifiers::default());

        assert!(terminal_should_defer_key_text_to_input_handler_for_state(
            true, "", &event
        ));
    }

    #[test]
    fn terminal_status_changed_detects_identical_text() {
        assert!(!terminal_status_changed("sent 1 byte(s)", "sent 1 byte(s)"));
        assert!(terminal_status_changed("idle", "sent 1 byte(s)"));
    }
}
