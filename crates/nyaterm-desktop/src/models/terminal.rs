use nyaterm_core::{
    ActionLinksMatcherSettings, TerminalBackendResize, terminal_backend_resize_changed,
};
use nyaterm_terminal::{
    TerminalEffects, TerminalOutputDecoder, TerminalScreen, TerminalSnapshot,
    terminal_cell_col_for_byte_index,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::{
    action_links::{ActionLinkMatch, find_action_links},
    terminal::{
        NyaTerminalLayoutCache, TerminalBufferMatch, TerminalLineDecorations, TerminalSearchFlags,
        terminal_buffer_matches, terminal_screen_from_output,
    },
};

use super::RecordingWriteHandle;

/// Large-output protection modes (Tauri XTerminal performanceMode).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TerminalPerformanceMode {
    #[default]
    Normal,
    Overloaded,
}

/// In-pane large-output protection banner (Tauri PerformanceOverlayState).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerminalPerformanceOverlay {
    Overloaded,
    Recovered,
}

pub(crate) const TERMINAL_OUTPUT_VISIBLE_BACKLOG_CAP: usize = 1_000_000;
pub(crate) const TERMINAL_OUTPUT_VISIBLE_BURST_OVERLOAD: usize = 256 * 1024;
/// UI-only text mirror cap. The authoritative terminal screen/scrollback lives
/// in the frame worker; the GPUI thread keeps only a recent tail for prompts,
/// AI context snippets, reconnect seed text, and compact tab actions.
pub(crate) const TERMINAL_UI_OUTPUT_TAIL_CAP: usize = 128 * 1024;
const TERMINAL_FRAME_VISIBLE_TEXT_TAIL_CAP: usize = 16 * 1024;
/// ~3s recovery notice at the 50ms event-pump cadence.
pub(crate) const TERMINAL_PERFORMANCE_RECOVERY_TICKS: u8 = 60;
/// Require a short calm window before re-enabling expensive render decorations.
pub(crate) const TERMINAL_RENDER_DEGRADATION_RECOVERY_TICKS: u8 = 8;

#[derive(Debug, Clone, Default)]
pub(crate) struct TerminalFrameActionLinks {
    pub(crate) matcher_key: u64,
    pub(crate) matches_by_line: Vec<Vec<ActionLinkMatch>>,
    pub(crate) cell_ranges_by_line: Vec<Vec<(usize, usize)>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TerminalFrameSearchKey {
    pub(crate) query: String,
    pub(crate) case_sensitive: bool,
    pub(crate) regex: bool,
    pub(crate) whole_word: bool,
    pub(crate) limit: usize,
}

impl TerminalFrameSearchKey {
    pub(crate) fn flags(&self) -> TerminalSearchFlags {
        TerminalSearchFlags {
            case_sensitive: self.case_sensitive,
            regex: self.regex,
            whole_word: self.whole_word,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TerminalFrameSearchResult {
    pub(crate) key: TerminalFrameSearchKey,
    pub(crate) revision: u64,
    pub(crate) matches: Result<Vec<TerminalBufferMatch>, String>,
}

pub(crate) fn terminal_frame_search_result_is_current(
    result: &TerminalFrameSearchResult,
    key: &TerminalFrameSearchKey,
    revision: u64,
) -> bool {
    result.key == *key && result.revision == revision
}

pub(crate) fn terminal_expensive_interactions_enabled(
    action_links_enabled: bool,
    is_active: bool,
    render_degraded: bool,
    runtime_output_pressure: bool,
    output_burst_bytes: usize,
    performance_mode: TerminalPerformanceMode,
) -> bool {
    action_links_enabled
        && is_active
        && !render_degraded
        && !runtime_output_pressure
        && output_burst_bytes == 0
        && performance_mode != TerminalPerformanceMode::Overloaded
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TerminalProtocolState {
    pub(crate) focus_reporting: bool,
    pub(crate) bracketed_paste: bool,
    pub(crate) mouse_reporting: bool,
    pub(crate) mouse_sgr: bool,
    pub(crate) mouse_drag_reporting: bool,
    pub(crate) mouse_motion_reporting: bool,
    pub(crate) application_cursor_keys: bool,
    pub(crate) application_keypad: bool,
    pub(crate) kitty_keyboard_disambiguate: bool,
    pub(crate) kitty_keyboard_report_event_types: bool,
    pub(crate) kitty_keyboard_report_alternate_keys: bool,
    pub(crate) kitty_keyboard_report_all_keys_as_esc: bool,
    pub(crate) kitty_keyboard_report_associated_text: bool,
    pub(crate) alternate_scroll: bool,
    pub(crate) alternate_screen: bool,
}

impl TerminalProtocolState {
    pub(crate) fn from_screen(screen: &TerminalScreen) -> Self {
        Self {
            focus_reporting: screen.focus_reporting(),
            bracketed_paste: screen.bracketed_paste(),
            mouse_reporting: screen.mouse_reporting(),
            mouse_sgr: screen.mouse_sgr(),
            mouse_drag_reporting: screen.mouse_drag_reporting(),
            mouse_motion_reporting: screen.mouse_motion_reporting(),
            application_cursor_keys: screen.application_cursor_keys(),
            application_keypad: screen.application_keypad(),
            kitty_keyboard_disambiguate: screen.kitty_keyboard_disambiguate(),
            kitty_keyboard_report_event_types: screen.kitty_keyboard_report_event_types(),
            kitty_keyboard_report_alternate_keys: screen.kitty_keyboard_report_alternate_keys(),
            kitty_keyboard_report_all_keys_as_esc: screen.kitty_keyboard_report_all_keys_as_esc(),
            kitty_keyboard_report_associated_text: screen.kitty_keyboard_report_associated_text(),
            alternate_scroll: screen.alternate_scroll(),
            alternate_screen: screen.alternate_screen(),
        }
    }

    pub(crate) fn alternate_scroll_payload(self, delta_lines: i32) -> Option<Vec<u8>> {
        if delta_lines == 0
            || !self.alternate_screen
            || !self.alternate_scroll
            || self.mouse_reporting
        {
            return None;
        }
        let up = delta_lines > 0;
        let unit = nyaterm_terminal::alternate_scroll_key_bytes(up, self.application_cursor_keys);
        let steps = delta_lines.unsigned_abs().min(8) as usize;
        let mut payload = Vec::with_capacity(unit.len() * steps);
        for _ in 0..steps {
            payload.extend_from_slice(&unit);
        }
        Some(payload)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn encode_mouse_report(
        self,
        button: u8,
        col: u16,
        row: u16,
        press: bool,
        motion: bool,
        shift: bool,
        alt: bool,
        ctrl: bool,
    ) -> Vec<u8> {
        if !self.mouse_reporting {
            return Vec::new();
        }
        let x = col.saturating_add(1);
        let y = row.saturating_add(1);
        let mut code = if press || self.mouse_sgr { button } else { 3 };
        if motion {
            code = code.saturating_add(32);
        }
        if shift {
            code = code.saturating_add(4);
        }
        if alt {
            code = code.saturating_add(8);
        }
        if ctrl {
            code = code.saturating_add(16);
        }
        if self.mouse_sgr {
            let suffix = if press { 'M' } else { 'm' };
            format!("\x1b[<{code};{x};{y}{suffix}").into_bytes()
        } else {
            let cb = 32u16.saturating_add(u16::from(code)).min(255) as u8;
            let cx = 32u16.saturating_add(x).min(255) as u8;
            let cy = 32u16.saturating_add(y).min(255) as u8;
            vec![0x1b, b'[', b'M', cb, cx, cy]
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct TerminalRenderCache {
    pub(crate) layout_cache: Arc<Mutex<NyaTerminalLayoutCache>>,
    decoration_cache: Arc<Mutex<TerminalDecorationCache>>,
}

#[derive(Debug, Default)]
struct TerminalDecorationCache {
    decoration_key: Option<u64>,
    decoration_lines: Vec<TerminalLineDecorations>,
    hits: u64,
    misses: u64,
}

impl TerminalRenderCache {
    pub(crate) fn clear(&mut self) {
        if let Ok(mut cache) = self.layout_cache.lock() {
            cache.clear();
        }
        if let Ok(mut cache) = self.decoration_cache.lock() {
            cache.clear();
        }
    }

    pub(crate) fn line_decorations(
        &self,
        key: u64,
        build: impl FnOnce() -> Vec<TerminalLineDecorations>,
    ) -> Vec<TerminalLineDecorations> {
        let Ok(mut cache) = self.decoration_cache.lock() else {
            return build();
        };
        cache.line_decorations(key, build)
    }

    pub(crate) fn decoration_stats(&self) -> (u64, u64) {
        self.decoration_cache
            .lock()
            .map(|cache| (cache.hits, cache.misses))
            .unwrap_or((0, 0))
    }
}

impl TerminalDecorationCache {
    fn clear(&mut self) {
        self.decoration_key = None;
        self.decoration_lines.clear();
        self.hits = 0;
        self.misses = 0;
    }

    fn line_decorations(
        &mut self,
        key: u64,
        build: impl FnOnce() -> Vec<TerminalLineDecorations>,
    ) -> Vec<TerminalLineDecorations> {
        if self.decoration_key == Some(key) {
            self.hits = self.hits.saturating_add(1);
            return self.decoration_lines.clone();
        }
        self.misses = self.misses.saturating_add(1);
        let decorations = build();
        self.decoration_key = Some(key);
        self.decoration_lines = decorations.clone();
        decorations
    }
}

pub(crate) fn terminal_action_link_matcher_key(
    enabled: bool,
    matchers: &ActionLinksMatcherSettings,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    enabled.hash(&mut hasher);
    matchers.ipv4.hash(&mut hasher);
    matchers.archive.hash(&mut hasher);
    matchers.host_port.hash(&mut hasher);
    hasher.finish()
}

fn prepare_terminal_frame_action_links(
    snapshot: &TerminalSnapshot,
    enabled: bool,
    matchers: &ActionLinksMatcherSettings,
) -> Option<TerminalFrameActionLinks> {
    if !enabled {
        return Some(TerminalFrameActionLinks {
            matcher_key: terminal_action_link_matcher_key(false, matchers),
            matches_by_line: vec![Vec::new(); snapshot.lines.len()],
            cell_ranges_by_line: vec![Vec::new(); snapshot.lines.len()],
        });
    }
    let matches_by_line = snapshot
        .lines
        .iter()
        .map(|line| {
            if line.is_empty() {
                Vec::new()
            } else {
                find_action_links(line, matchers, true)
            }
        })
        .collect::<Vec<_>>();
    let cell_ranges_by_line = snapshot
        .lines
        .iter()
        .zip(matches_by_line.iter())
        .map(|(line, matches)| {
            matches
                .iter()
                .map(|item| {
                    (
                        terminal_cell_col_for_byte_index(line, item.start),
                        terminal_cell_col_for_byte_index(line, item.end),
                    )
                })
                .collect()
        })
        .collect();
    Some(TerminalFrameActionLinks {
        matcher_key: terminal_action_link_matcher_key(true, matchers),
        matches_by_line,
        cell_ranges_by_line,
    })
}

pub(crate) fn protect_terminal_output_burst<'a>(
    screen: &mut TerminalScreen,
    output_decoder: &mut TerminalOutputDecoder,
    data: &'a [u8],
) -> (&'a [u8], usize) {
    if data.len() <= TERMINAL_OUTPUT_VISIBLE_BACKLOG_CAP {
        return (data, 0);
    }
    let skip = data.len() - TERMINAL_OUTPUT_VISIBLE_BACKLOG_CAP;
    screen.reset_stream_state();
    output_decoder.reset_decoder();
    (&data[skip..], skip)
}

fn trim_string_to_tail(output: &mut String, max_bytes: usize) {
    if max_bytes == 0 {
        output.clear();
        return;
    }
    if output.len() <= max_bytes {
        return;
    }
    let min_start = output.len() - max_bytes;
    let drain_to = output
        .char_indices()
        .find_map(|(index, _)| (index >= min_start).then_some(index))
        .unwrap_or(output.len());
    output.drain(..drain_to);
}

pub(crate) fn append_terminal_ui_output_tail(output: &mut String, text: &str) {
    if text.is_empty() {
        return;
    }
    output.push_str(text);
    trim_string_to_tail(output, TERMINAL_UI_OUTPUT_TAIL_CAP);
}

fn terminal_text_tail(mut text: String, max_bytes: usize) -> String {
    trim_string_to_tail(&mut text, max_bytes);
    text
}

fn append_terminal_frame_visible_tail(output: &mut String, text: &str) {
    if text.is_empty() {
        return;
    }
    output.push_str(text);
    trim_string_to_tail(output, TERMINAL_FRAME_VISIBLE_TEXT_TAIL_CAP);
}

fn merge_terminal_effects(target: &mut TerminalEffects, mut incoming: TerminalEffects) {
    if incoming.title.is_some() {
        target.title = incoming.title.take();
    }
    target.reset_title |= incoming.reset_title;
    target.bell |= incoming.bell;
    if incoming.cwd.is_some() {
        target.cwd = incoming.cwd.take();
    }
    target.shell_command_started |= incoming.shell_command_started;
    target.shell_command_finished |= incoming.shell_command_finished;
    target.pty_write.append(&mut incoming.pty_write);
    if incoming.clipboard_store.is_some() {
        target.clipboard_store = incoming.clipboard_store.take();
    }
    target.clipboard_loads.append(&mut incoming.clipboard_loads);
}

pub(crate) struct TerminalViewState {
    pub(crate) output: String,
    pub(crate) screen: TerminalScreen,
    /// Latest live viewport prepared by the background terminal frame processor.
    pub(crate) frame_snapshot: Option<Arc<TerminalSnapshot>>,
    pub(crate) frame_action_links: Option<TerminalFrameActionLinks>,
    pub(crate) scrollback_snapshots: HashMap<usize, Arc<TerminalSnapshot>>,
    pub(crate) scrollback_action_links: HashMap<usize, TerminalFrameActionLinks>,
    pub(crate) pending_snapshot_offsets: HashSet<usize>,
    pub(crate) search_result: Option<TerminalFrameSearchResult>,
    pub(crate) pending_search_key: Option<TerminalFrameSearchKey>,
    pub(crate) protocol_state: TerminalProtocolState,
    pub(crate) output_decoder: TerminalOutputDecoder,
    pub(crate) recording_decoder: TerminalOutputDecoder,
    pub(crate) screen_revision: u64,
    pub(crate) render_cache: TerminalRenderCache,
    pub(crate) has_unread: bool,
    /// Viewport offset from the live bottom (0 = follow output).
    pub(crate) scroll_offset: usize,
    /// True when output arrived while scrolled into history (FAB "New" affordance).
    pub(crate) has_new_while_scrolled: bool,
    pub(crate) performance_mode: TerminalPerformanceMode,
    pub(crate) performance_overlay: Option<TerminalPerformanceOverlay>,
    /// Remaining pump ticks for recovered banner auto-dismiss (0 = none).
    pub(crate) performance_overlay_ticks: u8,
    /// Characters dropped while protecting responsiveness (Tauri skippedOutputChars).
    pub(crate) skipped_output_chars: u64,
    /// Bytes accepted in the current calm window (reset each event-pump tick).
    pub(crate) output_burst_bytes: usize,
    /// True while expensive render decorations are temporarily skipped.
    pub(crate) render_degraded: bool,
    /// Consecutive low-pressure ticks before re-enabling render decorations.
    pub(crate) render_degraded_calm_ticks: u8,
    /// Last size sent to the PTY/backend for this session.
    pub(crate) last_backend_resize: Option<TerminalBackendResize>,
}

impl TerminalViewState {
    pub(crate) fn new() -> Self {
        Self {
            output: String::new(),
            screen: TerminalScreen::default(),
            frame_snapshot: None,
            frame_action_links: None,
            scrollback_snapshots: HashMap::new(),
            scrollback_action_links: HashMap::new(),
            pending_snapshot_offsets: HashSet::new(),
            search_result: None,
            pending_search_key: None,
            protocol_state: TerminalProtocolState::default(),
            output_decoder: TerminalOutputDecoder::default(),
            recording_decoder: TerminalOutputDecoder::default(),
            screen_revision: 0,
            render_cache: TerminalRenderCache::default(),
            has_unread: false,
            scroll_offset: 0,
            has_new_while_scrolled: false,
            performance_mode: TerminalPerformanceMode::Normal,
            performance_overlay: None,
            performance_overlay_ticks: 0,
            skipped_output_chars: 0,
            output_burst_bytes: 0,
            render_degraded: true,
            render_degraded_calm_ticks: 0,
            last_backend_resize: None,
        }
    }

    pub(crate) fn from_output(output: String) -> Self {
        let screen = terminal_screen_from_output(&output);
        let protocol_state = TerminalProtocolState::from_screen(&screen);
        Self {
            output,
            screen,
            frame_snapshot: None,
            frame_action_links: None,
            scrollback_snapshots: HashMap::new(),
            scrollback_action_links: HashMap::new(),
            pending_snapshot_offsets: HashSet::new(),
            search_result: None,
            pending_search_key: None,
            protocol_state,
            output_decoder: TerminalOutputDecoder::default(),
            recording_decoder: TerminalOutputDecoder::default(),
            screen_revision: 0,
            render_cache: TerminalRenderCache::default(),
            has_unread: false,
            scroll_offset: 0,
            has_new_while_scrolled: false,
            performance_mode: TerminalPerformanceMode::Normal,
            performance_overlay: None,
            performance_overlay_ticks: 0,
            skipped_output_chars: 0,
            output_burst_bytes: 0,
            render_degraded: true,
            render_degraded_calm_ticks: 0,
            last_backend_resize: None,
        }
    }

    pub(crate) fn from_output_with_encoding(output: String, encoding: &str) -> Self {
        let mut view = Self::from_output(output);
        view.set_encoding(encoding);
        view
    }

    pub(crate) fn set_encoding(&mut self, encoding: &str) {
        self.screen.set_encoding(encoding);
        self.output_decoder.set_encoding(encoding);
        self.recording_decoder.set_encoding(encoding);
    }

    pub(crate) fn append_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.screen.advance_decoded_text(text);
        self.screen_revision = self.screen_revision.saturating_add(1);
        self.frame_snapshot = Some(Arc::new(self.screen.viewport_snapshot(0)));
        self.frame_action_links = None;
        self.enter_render_degraded_mode();
        self.clear_scrollback_query_caches();
        self.protocol_state = TerminalProtocolState::from_screen(&self.screen);
        append_terminal_ui_output_tail(&mut self.output, text);
        if self.scroll_offset > 0 {
            self.has_new_while_scrolled = true;
        }
        self.clamp_scroll_offset();
    }

    /// Feed already-protected bytes into the view (used when the caller applies
    /// the same feed to the mirrored active screen).
    #[cfg(test)]
    pub(crate) fn append_bytes_unprotected(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        self.screen.advance(data);
        self.screen_revision = self.screen_revision.saturating_add(1);
        self.frame_snapshot = Some(Arc::new(self.screen.viewport_snapshot(0)));
        self.frame_action_links = None;
        self.enter_render_degraded_mode();
        self.clear_scrollback_query_caches();
        self.protocol_state = TerminalProtocolState::from_screen(&self.screen);
        append_terminal_ui_output_tail(
            &mut self.output,
            &self.output_decoder.decode_output_text(data),
        );
        if self.scroll_offset > 0 {
            self.has_new_while_scrolled = true;
        }
        self.clamp_scroll_offset();
    }

    /// Drop the oldest part of an oversized burst so the latest screen state wins
    /// (Tauri backlog trim + large-output protection).
    #[cfg(test)]
    pub(crate) fn protect_output_burst<'a>(&mut self, data: &'a [u8]) -> &'a [u8] {
        if data.is_empty() {
            return data;
        }
        let (feed, skip) =
            protect_terminal_output_burst(&mut self.screen, &mut self.output_decoder, data);
        if skip > 0 {
            self.note_skipped_output(skip);
        }
        self.output_burst_bytes = self.output_burst_bytes.saturating_add(feed.len());
        if self.output_burst_bytes > TERMINAL_OUTPUT_VISIBLE_BURST_OVERLOAD
            || feed.len() > 32 * 1024
        {
            self.enter_overloaded_mode();
        } else if !feed.is_empty() {
            self.enter_render_degraded_mode();
        }
        feed
    }

    pub(crate) fn note_skipped_output(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.skipped_output_chars = self.skipped_output_chars.saturating_add(count as u64);
        self.enter_overloaded_mode();
    }

    pub(crate) fn note_output_discontinuity(&mut self, count: usize) {
        self.note_skipped_output(count);
        self.screen.reset_stream_state();
        self.output_decoder.reset_decoder();
        self.recording_decoder.reset_decoder();
    }

    pub(crate) fn enter_overloaded_mode(&mut self) {
        self.performance_mode = TerminalPerformanceMode::Overloaded;
        self.performance_overlay = Some(TerminalPerformanceOverlay::Overloaded);
        self.performance_overlay_ticks = 0;
        self.enter_render_degraded_mode();
    }

    pub(crate) fn maybe_exit_overloaded_mode(&mut self) {
        if self.performance_mode != TerminalPerformanceMode::Overloaded {
            return;
        }
        // Calm window: no large burst this tick.
        if self.output_burst_bytes > TERMINAL_OUTPUT_VISIBLE_BURST_OVERLOAD / 4 {
            return;
        }
        self.performance_mode = TerminalPerformanceMode::Normal;
        self.performance_overlay = Some(TerminalPerformanceOverlay::Recovered);
        self.performance_overlay_ticks = TERMINAL_PERFORMANCE_RECOVERY_TICKS;
    }

    pub(crate) fn enter_render_degraded_mode(&mut self) {
        self.render_degraded = true;
        self.render_degraded_calm_ticks = 0;
    }

    fn tick_render_degradation(&mut self, output_pressure: bool) {
        if output_pressure || self.output_burst_bytes > 0 {
            self.enter_render_degraded_mode();
            return;
        }
        if !self.render_degraded {
            return;
        }
        self.render_degraded_calm_ticks = self.render_degraded_calm_ticks.saturating_add(1);
        if self.render_degraded_calm_ticks >= TERMINAL_RENDER_DEGRADATION_RECOVERY_TICKS {
            self.render_degraded = false;
            self.render_degraded_calm_ticks = 0;
        }
    }

    pub(crate) fn tick_performance_overlay(&mut self, output_pressure: bool) {
        // End-of-tick calm accounting for recovery.
        self.maybe_exit_overloaded_mode();
        self.tick_render_degradation(output_pressure);
        self.output_burst_bytes = 0;
        if self.performance_overlay_ticks > 0 {
            self.performance_overlay_ticks = self.performance_overlay_ticks.saturating_sub(1);
            if self.performance_overlay_ticks == 0
                && self.performance_overlay == Some(TerminalPerformanceOverlay::Recovered)
            {
                self.performance_overlay = None;
            }
        }
    }

    pub(crate) fn clear(&mut self) {
        self.output.clear();
        self.screen.clear();
        self.frame_snapshot = None;
        self.frame_action_links = None;
        self.clear_terminal_query_caches();
        self.protocol_state = TerminalProtocolState::default();
        self.output_decoder.reset_decoder();
        self.recording_decoder.reset_decoder();
        self.screen_revision = self.screen_revision.saturating_add(1);
        self.render_cache.clear();
        self.has_unread = false;
        self.scroll_offset = 0;
        self.has_new_while_scrolled = false;
        self.performance_mode = TerminalPerformanceMode::Normal;
        self.performance_overlay = None;
        self.performance_overlay_ticks = 0;
        self.skipped_output_chars = 0;
        self.output_burst_bytes = 0;
        self.render_degraded = true;
        self.render_degraded_calm_ticks = 0;
    }

    pub(crate) fn clamp_scroll_offset(&mut self) {
        let max = self
            .frame_snapshot
            .as_ref()
            .map(|snapshot| snapshot.scrollback_len)
            .unwrap_or_else(|| self.screen.scrollback_len());
        if self.scroll_offset > max {
            self.scroll_offset = max;
        }
    }

    pub(crate) fn apply_terminal_frame_parts(
        &mut self,
        visible_text: &str,
        snapshot: Arc<TerminalSnapshot>,
        action_links: Option<TerminalFrameActionLinks>,
        protocol_state: TerminalProtocolState,
        accepted_bytes: usize,
        skipped_output_bytes: usize,
        revision: u64,
    ) {
        // UI output tail is only used for copy/export helpers. Skip rebuilding a
        // large String during output pressure / degraded paint so frame apply stays cheap.
        if !visible_text.is_empty() && !self.render_degraded {
            append_terminal_ui_output_tail(&mut self.output, visible_text);
        }
        self.frame_snapshot = Some(snapshot);
        self.frame_action_links = action_links;
        self.clear_scrollback_query_caches();
        self.protocol_state = protocol_state;
        self.screen_revision = revision;
        self.output_burst_bytes = self.output_burst_bytes.saturating_add(accepted_bytes);
        if accepted_bytes > 0 {
            self.enter_render_degraded_mode();
        }
        if skipped_output_bytes > 0 {
            self.note_skipped_output(skipped_output_bytes);
        }
        if self.scroll_offset > 0 {
            self.has_new_while_scrolled = true;
        }
        self.clamp_scroll_offset();
    }

    pub(crate) fn apply_terminal_background_frame_parts(
        &mut self,
        snapshot: Arc<TerminalSnapshot>,
        action_links: Option<TerminalFrameActionLinks>,
        protocol_state: TerminalProtocolState,
        skipped_output_bytes: usize,
        revision: u64,
    ) {
        self.frame_snapshot = Some(snapshot);
        self.frame_action_links = action_links;
        self.protocol_state = protocol_state;
        self.screen_revision = revision;
        if skipped_output_bytes > 0 {
            self.note_skipped_output(skipped_output_bytes);
        }
        self.clamp_scroll_offset();
    }

    pub(crate) fn backend_resize_changed(
        &self,
        cols: u16,
        rows: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> bool {
        terminal_backend_resize_changed(
            self.last_backend_resize,
            TerminalBackendResize::new(cols, rows, pixel_width, pixel_height),
        )
    }

    pub(crate) fn remember_backend_resize(
        &mut self,
        cols: u16,
        rows: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) {
        self.last_backend_resize = Some(TerminalBackendResize::new(
            cols,
            rows,
            pixel_width,
            pixel_height,
        ));
    }

    fn clear_scrollback_query_caches(&mut self) {
        self.scrollback_snapshots.clear();
        self.scrollback_action_links.clear();
        self.pending_snapshot_offsets.clear();
    }

    fn clear_terminal_query_caches(&mut self) {
        self.clear_scrollback_query_caches();
        self.search_result = None;
        self.pending_search_key = None;
    }

    pub(crate) fn scrollback_len_for_ui(&self) -> usize {
        self.frame_snapshot
            .as_ref()
            .map(|snapshot| snapshot.scrollback_len)
            .unwrap_or_else(|| self.screen.scrollback_len())
    }

    pub(crate) fn viewport_rows_for_ui(&self) -> usize {
        self.frame_snapshot
            .as_ref()
            .map(|snapshot| snapshot.lines.len().max(1))
            .unwrap_or_else(|| self.screen.viewport_snapshot(0).lines.len().max(1))
    }

    pub(crate) fn total_rows_for_ui(&self) -> usize {
        self.frame_snapshot
            .as_ref()
            .map(|snapshot| snapshot.total_rows.max(1))
            .unwrap_or_else(|| self.screen.total_rows().max(1))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchEngineEditorField {
    Name,
    Url,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeywordHighlightEditorField {
    Name,
    Patterns,
    ColorDark,
    ColorLight,
}

impl KeywordHighlightEditorField {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Name => Self::Patterns,
            Self::Patterns => Self::ColorDark,
            Self::ColorDark => Self::ColorLight,
            Self::ColorLight => Self::Name,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TerminalFramePipeline {
    command_tx: TerminalFrameCommandSender,
    event_queue: TerminalFrameEventQueue,
}

pub(crate) struct TerminalFrameOutputSubmission {
    pub(crate) session_id: String,
    pub(crate) data: Vec<u8>,
    pub(crate) encoding: String,
    pub(crate) scrollback_limit: usize,
}

impl TerminalFramePipeline {
    pub(crate) fn spawn(recording_writer: RecordingWriteHandle) -> Self {
        let (command_tx, command_rx) = terminal_frame_command_channel();
        let event_queue = TerminalFrameEventQueue::new(TERMINAL_FRAME_EVENT_QUEUE_CAP);
        let event_queue_for_worker = event_queue.clone();
        thread::Builder::new()
            .name("nyaterm-terminal-frame-processor".to_string())
            .spawn(move || {
                run_terminal_frame_processor(command_rx, event_queue_for_worker, recording_writer)
            })
            .expect("failed to spawn terminal frame processor");
        Self {
            command_tx,
            event_queue,
        }
    }

    pub(crate) fn ensure_session(
        &self,
        session_id: impl Into<String>,
        encoding: impl Into<String>,
        scrollback_limit: usize,
    ) {
        let _ = self.command_tx.send(TerminalFrameCommand::EnsureSession {
            session_id: session_id.into(),
            encoding: encoding.into(),
            scrollback_limit,
        });
    }

    pub(crate) fn seed_session(
        &self,
        session_id: impl Into<String>,
        output: impl Into<String>,
        encoding: impl Into<String>,
        scrollback_limit: usize,
    ) {
        let _ = self.command_tx.send(TerminalFrameCommand::SeedSession {
            session_id: session_id.into(),
            output: output.into(),
            encoding: encoding.into(),
            scrollback_limit,
        });
    }

    pub(crate) fn remove_session(&self, session_id: impl Into<String>) {
        let _ = self.command_tx.send(TerminalFrameCommand::RemoveSession {
            session_id: session_id.into(),
        });
    }

    pub(crate) fn resize_session(&self, session_id: impl Into<String>, cols: u16, rows: u16) {
        let _ = self.command_tx.send(TerminalFrameCommand::ResizeSession {
            session_id: session_id.into(),
            cols,
            rows,
        });
    }

    pub(crate) fn submit_output(
        &self,
        session_id: impl Into<String>,
        data: Vec<u8>,
        encoding: impl Into<String>,
        scrollback_limit: usize,
    ) {
        if data.is_empty() {
            return;
        }
        let _ = self.command_tx.send(TerminalFrameCommand::Output {
            session_id: session_id.into(),
            data,
            encoding: encoding.into(),
            scrollback_limit,
        });
    }

    pub(crate) fn submit_outputs(&self, outputs: Vec<TerminalFrameOutputSubmission>) {
        if outputs.is_empty() {
            return;
        }
        let commands = outputs.into_iter().filter_map(|output| {
            (!output.data.is_empty()).then_some(TerminalFrameCommand::Output {
                session_id: output.session_id,
                data: output.data,
                encoding: output.encoding,
                scrollback_limit: output.scrollback_limit,
            })
        });
        let _ = self.command_tx.send_many(commands);
    }

    pub(crate) fn request_snapshot(
        &self,
        session_id: impl Into<String>,
        offset: usize,
        action_links_enabled: bool,
        action_link_matchers: ActionLinksMatcherSettings,
    ) {
        let _ = self.command_tx.send(TerminalFrameCommand::RequestSnapshot {
            session_id: session_id.into(),
            offset,
            action_links_enabled,
            action_link_matchers,
        });
    }

    pub(crate) fn request_search(
        &self,
        session_id: impl Into<String>,
        key: TerminalFrameSearchKey,
    ) {
        if key.query.trim().is_empty() || key.limit == 0 {
            return;
        }
        let _ = self.command_tx.send(TerminalFrameCommand::RequestSearch {
            session_id: session_id.into(),
            key,
        });
    }

    pub(crate) fn request_buffer_text(
        &self,
        session_id: impl Into<String>,
        max_bytes: usize,
        request_id: impl Into<String>,
    ) {
        let _ = self
            .command_tx
            .send(TerminalFrameCommand::RequestBufferText {
                session_id: session_id.into(),
                max_bytes,
                request_id: request_id.into(),
            });
    }

    pub(crate) fn drain_events_into(
        &self,
        events: &mut VecDeque<TerminalFrameEvent>,
        limit: usize,
    ) -> usize {
        self.event_queue.drain_into(events, limit)
    }

    pub(crate) fn queued_event_count(&self) -> usize {
        self.event_queue.len()
    }

    pub(crate) fn queued_command_count(&self) -> usize {
        self.command_tx.len()
    }

    pub(crate) fn queued_output_bytes(&self) -> usize {
        self.command_tx.queued_output_bytes()
    }
}

impl Default for TerminalFramePipeline {
    fn default() -> Self {
        let recording_manager = Arc::new(nyaterm_transport::RecordingManager::new());
        let recording_writer = super::RecordingWritePipeline::spawn(recording_manager).writer();
        Self::spawn(recording_writer)
    }
}

#[derive(Debug)]
enum TerminalFrameCommand {
    EnsureSession {
        session_id: String,
        encoding: String,
        scrollback_limit: usize,
    },
    SeedSession {
        session_id: String,
        output: String,
        encoding: String,
        scrollback_limit: usize,
    },
    RemoveSession {
        session_id: String,
    },
    ResizeSession {
        session_id: String,
        cols: u16,
        rows: u16,
    },
    Output {
        session_id: String,
        data: Vec<u8>,
        encoding: String,
        scrollback_limit: usize,
    },
    RequestSnapshot {
        session_id: String,
        offset: usize,
        action_links_enabled: bool,
        action_link_matchers: ActionLinksMatcherSettings,
    },
    RequestSearch {
        session_id: String,
        key: TerminalFrameSearchKey,
    },
    RequestBufferText {
        session_id: String,
        max_bytes: usize,
        request_id: String,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum TerminalFrameEvent {
    Output(TerminalFrameOutputEvent),
    Snapshot(TerminalFrameSnapshotEvent),
    Search(TerminalFrameSearchEvent),
    BufferText(TerminalFrameBufferTextEvent),
}

#[derive(Clone, Debug)]
struct TerminalFrameEventQueue {
    inner: Arc<Mutex<VecDeque<TerminalFrameEvent>>>,
    cap: usize,
}

impl TerminalFrameEventQueue {
    fn new(cap: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(cap.min(1024)))),
            cap,
        }
    }

    fn push(&self, event: TerminalFrameEvent) {
        let Ok(mut queue) = self.inner.lock() else {
            return;
        };
        compact_terminal_frame_event_queue(&mut queue, &event);
        while queue.len() >= self.cap.max(1) {
            let drop_index = queue
                .iter()
                .position(terminal_frame_event_can_drop_under_pressure)
                .unwrap_or(0);
            queue.remove(drop_index);
        }
        queue.push_back(event);
    }

    #[cfg(test)]
    fn try_recv(&self) -> Option<TerminalFrameEvent> {
        self.inner.lock().ok()?.pop_front()
    }

    fn drain_into(&self, events: &mut VecDeque<TerminalFrameEvent>, limit: usize) -> usize {
        if limit == 0 {
            return 0;
        }
        let Ok(mut queue) = self.inner.lock() else {
            return 0;
        };
        let mut drained = 0usize;
        while drained < limit {
            let Some(event) = queue.pop_front() else {
                break;
            };
            events.push_back(event);
            drained += 1;
        }
        drained
    }

    fn len(&self) -> usize {
        self.inner.lock().map(|queue| queue.len()).unwrap_or(0)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TerminalFrameOutputEvent {
    pub(crate) session_id: String,
    pub(crate) visible_text: String,
    pub(crate) recording_text_bytes: usize,
    pub(crate) snapshot: Arc<TerminalSnapshot>,
    pub(crate) action_links: Option<TerminalFrameActionLinks>,
    pub(crate) protocol_state: TerminalProtocolState,
    pub(crate) effects: TerminalEffects,
    pub(crate) command_running: bool,
    pub(crate) accepted_bytes: usize,
    pub(crate) skipped_output_bytes: usize,
    pub(crate) revision: u64,
    pub(crate) process_duration: Duration,
}

#[derive(Clone, Debug)]
pub(crate) struct TerminalFrameSnapshotEvent {
    pub(crate) session_id: String,
    pub(crate) offset: usize,
    pub(crate) snapshot: Arc<TerminalSnapshot>,
    pub(crate) action_links: Option<TerminalFrameActionLinks>,
    pub(crate) revision: u64,
    pub(crate) process_duration: Duration,
}

#[derive(Clone, Debug)]
pub(crate) struct TerminalFrameSearchEvent {
    pub(crate) session_id: String,
    pub(crate) result: TerminalFrameSearchResult,
    pub(crate) process_duration: Duration,
}

#[derive(Clone, Debug)]
pub(crate) struct TerminalFrameBufferTextEvent {
    pub(crate) session_id: String,
    pub(crate) request_id: String,
    pub(crate) text: String,
    pub(crate) truncated: bool,
    pub(crate) process_duration: Duration,
}

fn compact_terminal_frame_event_queue(
    queue: &mut VecDeque<TerminalFrameEvent>,
    incoming: &TerminalFrameEvent,
) {
    let TerminalFrameEvent::Output(incoming) = incoming else {
        return;
    };
    if !terminal_frame_output_event_can_drop_under_pressure(incoming) {
        return;
    }
    queue.retain(|event| {
        let TerminalFrameEvent::Output(queued) = event else {
            return true;
        };
        queued.session_id != incoming.session_id
            || !terminal_frame_output_event_can_drop_under_pressure(queued)
    });
}

fn terminal_frame_event_can_drop_under_pressure(event: &TerminalFrameEvent) -> bool {
    match event {
        TerminalFrameEvent::Output(frame) => {
            terminal_frame_output_event_can_drop_under_pressure(frame)
        }
        TerminalFrameEvent::Snapshot(_)
        | TerminalFrameEvent::Search(_)
        | TerminalFrameEvent::BufferText(_) => false,
    }
}

fn terminal_frame_output_event_can_drop_under_pressure(frame: &TerminalFrameOutputEvent) -> bool {
    !frame.effects.bell
        && frame.effects.title.is_none()
        && !frame.effects.reset_title
        && frame.effects.cwd.is_none()
        && !frame.effects.shell_command_started
        && !frame.effects.shell_command_finished
        && frame.effects.pty_write.is_empty()
        && frame.effects.clipboard_store.is_none()
        && frame.effects.clipboard_loads.is_empty()
}

struct TerminalFrameSession {
    screen: TerminalScreen,
    output_decoder: TerminalOutputDecoder,
    recording_decoder: TerminalOutputDecoder,
    revision: u64,
}

impl TerminalFrameSession {
    fn new(encoding: &str, scrollback_limit: usize) -> Self {
        let mut screen = TerminalScreen::default();
        screen.set_encoding(encoding);
        screen.set_scrollback_limit(scrollback_limit);
        let mut output_decoder = TerminalOutputDecoder::default();
        output_decoder.set_encoding(encoding);
        let mut recording_decoder = TerminalOutputDecoder::default();
        recording_decoder.set_encoding(encoding);
        Self {
            screen,
            output_decoder,
            recording_decoder,
            revision: 0,
        }
    }

    fn set_encoding_and_limit(&mut self, encoding: &str, scrollback_limit: usize) {
        self.screen.set_encoding(encoding);
        self.screen.set_scrollback_limit(scrollback_limit);
        self.output_decoder.set_encoding(encoding);
        self.recording_decoder.set_encoding(encoding);
    }

    fn seed(&mut self, output: String, encoding: &str, scrollback_limit: usize) {
        self.screen = terminal_screen_from_output(&output);
        self.screen.set_encoding(encoding);
        self.screen.set_scrollback_limit(scrollback_limit);
        self.output_decoder = TerminalOutputDecoder::default();
        self.output_decoder.set_encoding(encoding);
        self.recording_decoder = TerminalOutputDecoder::default();
        self.recording_decoder.set_encoding(encoding);
        self.revision = self.revision.saturating_add(1);
    }

    fn resize(&mut self, cols: u16, rows: u16) {
        if self.screen.cols() as u16 != cols || self.screen.rows() as u16 != rows {
            self.screen.resize(cols, rows);
            self.revision = self.revision.saturating_add(1);
        }
    }

    #[cfg(test)]
    fn process_output(
        &mut self,
        session_id: String,
        data: Vec<u8>,
        encoding: String,
        scrollback_limit: usize,
        recording_writer: &RecordingWriteHandle,
    ) -> TerminalFrameOutputEvent {
        let started_at = Instant::now();
        let mut batch = TerminalFrameOutputBatch::default();
        batch.absorb(self.process_output_chunk(
            &session_id,
            &data,
            &encoding,
            scrollback_limit,
            recording_writer,
        ));
        self.output_event_from_batch(session_id, batch, started_at)
    }

    fn process_output_chunk(
        &mut self,
        session_id: &str,
        data: &[u8],
        encoding: &str,
        scrollback_limit: usize,
        recording_writer: &RecordingWriteHandle,
    ) -> TerminalFrameOutputChunk {
        self.set_encoding_and_limit(encoding, scrollback_limit);
        let recording_text = self.recording_decoder.decode_output_text(&data);
        let recording_text_bytes = recording_text.len();
        recording_writer.write_output(session_id.to_string(), recording_text);
        let (feed, skipped_output_bytes) =
            protect_terminal_output_burst(&mut self.screen, &mut self.output_decoder, &data);
        self.screen.advance(feed);
        let visible_text = terminal_text_tail(
            self.output_decoder.decode_output_text(feed),
            TERMINAL_FRAME_VISIBLE_TEXT_TAIL_CAP,
        );
        self.revision = self.revision.saturating_add(1);
        let effects = self.screen.take_effects();
        TerminalFrameOutputChunk {
            visible_text,
            recording_text_bytes,
            effects,
            accepted_bytes: feed.len(),
            skipped_output_bytes,
        }
    }

    fn output_event_from_batch(
        &self,
        session_id: String,
        batch: TerminalFrameOutputBatch,
        started_at: Instant,
    ) -> TerminalFrameOutputEvent {
        let command_running = self.screen.command_running();
        let protocol_state = TerminalProtocolState::from_screen(&self.screen);
        let snapshot = Arc::new(self.screen.viewport_snapshot(0));
        TerminalFrameOutputEvent {
            session_id,
            visible_text: batch.visible_text,
            recording_text_bytes: batch.recording_text_bytes,
            snapshot,
            action_links: None,
            protocol_state,
            effects: batch.effects,
            command_running,
            accepted_bytes: batch.accepted_bytes,
            skipped_output_bytes: batch.skipped_output_bytes,
            revision: self.revision,
            process_duration: started_at.elapsed(),
        }
    }

    fn snapshot_event(
        &self,
        session_id: String,
        offset: usize,
        action_links_enabled: bool,
        action_link_matchers: ActionLinksMatcherSettings,
    ) -> TerminalFrameSnapshotEvent {
        let started_at = Instant::now();
        let snapshot = Arc::new(self.screen.viewport_snapshot(offset));
        let action_links = prepare_terminal_frame_action_links(
            &snapshot,
            action_links_enabled,
            &action_link_matchers,
        );
        TerminalFrameSnapshotEvent {
            session_id,
            offset: snapshot.display_offset,
            snapshot,
            action_links,
            revision: self.revision,
            process_duration: started_at.elapsed(),
        }
    }

    fn search_event(
        &self,
        session_id: String,
        key: TerminalFrameSearchKey,
    ) -> TerminalFrameSearchEvent {
        let started_at = Instant::now();
        let flags = key.flags();
        let buffer_text = self.screen.all_lines().join("\n");
        let matches = terminal_buffer_matches(&buffer_text, &key.query, &flags, key.limit);
        TerminalFrameSearchEvent {
            session_id,
            result: TerminalFrameSearchResult {
                key,
                revision: self.revision,
                matches,
            },
            process_duration: started_at.elapsed(),
        }
    }

    fn buffer_text_event(
        &self,
        session_id: String,
        request_id: String,
        max_bytes: usize,
    ) -> TerminalFrameBufferTextEvent {
        let started_at = Instant::now();
        let mut text = self.screen.all_lines().join("\n");
        let truncated = text.len() > max_bytes;
        if truncated {
            text = terminal_text_tail(text, max_bytes);
        }
        TerminalFrameBufferTextEvent {
            session_id,
            request_id,
            text,
            truncated,
            process_duration: started_at.elapsed(),
        }
    }
}

#[derive(Debug)]
struct TerminalFrameOutputChunk {
    visible_text: String,
    recording_text_bytes: usize,
    effects: TerminalEffects,
    accepted_bytes: usize,
    skipped_output_bytes: usize,
}

#[derive(Debug, Default)]
struct TerminalFrameOutputBatch {
    visible_text: String,
    recording_text_bytes: usize,
    effects: TerminalEffects,
    accepted_bytes: usize,
    skipped_output_bytes: usize,
}

impl TerminalFrameOutputBatch {
    fn absorb(&mut self, chunk: TerminalFrameOutputChunk) {
        append_terminal_frame_visible_tail(&mut self.visible_text, &chunk.visible_text);
        self.recording_text_bytes = self
            .recording_text_bytes
            .saturating_add(chunk.recording_text_bytes);
        self.accepted_bytes = self.accepted_bytes.saturating_add(chunk.accepted_bytes);
        self.skipped_output_bytes = self
            .skipped_output_bytes
            .saturating_add(chunk.skipped_output_bytes);
        merge_terminal_effects(&mut self.effects, chunk.effects);
    }
}

#[derive(Debug)]
struct TerminalFrameCommandSender {
    shared: Arc<TerminalFrameCommandQueueShared>,
}

#[derive(Debug)]
struct TerminalFrameCommandReceiver {
    shared: Arc<TerminalFrameCommandQueueShared>,
}

#[derive(Debug)]
struct TerminalFrameCommandQueueShared {
    inner: Mutex<TerminalFrameCommandQueueInner>,
    ready: Condvar,
}

#[derive(Debug)]
struct TerminalFrameCommandQueueInner {
    commands: VecDeque<TerminalFrameCommand>,
    sender_count: usize,
}

fn terminal_frame_command_channel() -> (TerminalFrameCommandSender, TerminalFrameCommandReceiver) {
    let shared = Arc::new(TerminalFrameCommandQueueShared {
        inner: Mutex::new(TerminalFrameCommandQueueInner {
            commands: VecDeque::new(),
            sender_count: 1,
        }),
        ready: Condvar::new(),
    });
    (
        TerminalFrameCommandSender {
            shared: shared.clone(),
        },
        TerminalFrameCommandReceiver { shared },
    )
}

impl TerminalFrameCommandSender {
    fn send(&self, command: TerminalFrameCommand) -> bool {
        let Ok(mut inner) = self.shared.inner.lock() else {
            return false;
        };
        push_terminal_frame_command(&mut inner.commands, command);
        self.shared.ready.notify_one();
        true
    }

    fn send_many<I>(&self, commands: I) -> bool
    where
        I: IntoIterator<Item = TerminalFrameCommand>,
    {
        let Ok(mut inner) = self.shared.inner.lock() else {
            return false;
        };
        let mut sent = false;
        for command in commands {
            push_terminal_frame_command(&mut inner.commands, command);
            sent = true;
        }
        if sent {
            self.shared.ready.notify_one();
        }
        sent
    }

    fn len(&self) -> usize {
        self.shared
            .inner
            .lock()
            .map(|inner| inner.commands.len())
            .unwrap_or(0)
    }

    fn queued_output_bytes(&self) -> usize {
        self.shared
            .inner
            .lock()
            .map(|inner| terminal_frame_command_queue_output_bytes(&inner.commands))
            .unwrap_or(0)
    }
}

impl Clone for TerminalFrameCommandSender {
    fn clone(&self) -> Self {
        if let Ok(mut inner) = self.shared.inner.lock() {
            inner.sender_count = inner.sender_count.saturating_add(1);
        }
        Self {
            shared: self.shared.clone(),
        }
    }
}

impl Drop for TerminalFrameCommandSender {
    fn drop(&mut self) {
        let Ok(mut inner) = self.shared.inner.lock() else {
            return;
        };
        inner.sender_count = inner.sender_count.saturating_sub(1);
        self.shared.ready.notify_all();
    }
}

impl TerminalFrameCommandReceiver {
    fn recv(&self) -> Option<TerminalFrameCommand> {
        let mut inner = self.shared.inner.lock().ok()?;
        loop {
            if let Some(command) = inner.commands.pop_front() {
                return Some(command);
            }
            if inner.sender_count == 0 {
                return None;
            }
            inner = self.shared.ready.wait(inner).ok()?;
        }
    }

    fn try_recv(&self) -> Option<TerminalFrameCommand> {
        self.shared.inner.lock().ok()?.commands.pop_front()
    }
}

fn push_terminal_frame_command(
    commands: &mut VecDeque<TerminalFrameCommand>,
    command: TerminalFrameCommand,
) {
    match command {
        TerminalFrameCommand::Output {
            session_id,
            data,
            encoding,
            scrollback_limit,
        } => {
            commands.push_back(TerminalFrameCommand::Output {
                session_id,
                data,
                encoding,
                scrollback_limit,
            });
        }
        TerminalFrameCommand::ResizeSession {
            session_id,
            cols,
            rows,
        } => {
            if let Some(TerminalFrameCommand::ResizeSession {
                session_id: last_session_id,
                cols: last_cols,
                rows: last_rows,
            }) = commands.back_mut()
                && *last_session_id == session_id
            {
                *last_cols = cols;
                *last_rows = rows;
                return;
            }
            commands.push_back(TerminalFrameCommand::ResizeSession {
                session_id,
                cols,
                rows,
            });
        }
        other => commands.push_back(other),
    }
    compact_terminal_frame_command_queue(commands, TERMINAL_FRAME_COMMAND_QUEUE_CAP);
}

fn compact_terminal_frame_command_queue(commands: &mut VecDeque<TerminalFrameCommand>, cap: usize) {
    compact_stale_terminal_frame_commands(commands);
    while commands.len() > cap {
        let Some(drop_index) = commands
            .iter()
            .position(terminal_frame_command_can_drop_under_pressure)
        else {
            break;
        };
        commands.remove(drop_index);
    }
}

fn compact_stale_terminal_frame_commands(commands: &mut VecDeque<TerminalFrameCommand>) {
    if commands.len() <= 1 {
        return;
    }
    let mut seen_snapshots: HashSet<(String, usize)> = HashSet::new();
    let mut seen_searches: HashSet<String> = HashSet::new();
    let mut compacted = VecDeque::with_capacity(commands.len());

    for command in commands.drain(..).rev() {
        let keep = match &command {
            TerminalFrameCommand::RequestSnapshot {
                session_id, offset, ..
            } => seen_snapshots.insert((session_id.clone(), *offset)),
            TerminalFrameCommand::RequestSearch { session_id, .. } => {
                seen_searches.insert(session_id.clone())
            }
            _ => true,
        };
        if keep {
            compacted.push_front(command);
        }
    }

    *commands = compacted;
}

fn terminal_frame_command_can_drop_under_pressure(command: &TerminalFrameCommand) -> bool {
    matches!(
        command,
        TerminalFrameCommand::RequestSnapshot { .. } | TerminalFrameCommand::RequestSearch { .. }
    )
}

fn terminal_frame_command_queue_output_bytes(commands: &VecDeque<TerminalFrameCommand>) -> usize {
    commands
        .iter()
        .map(|command| match command {
            TerminalFrameCommand::Output { data, .. } => data.len(),
            _ => 0,
        })
        .sum()
}

fn run_terminal_frame_processor(
    command_rx: TerminalFrameCommandReceiver,
    event_queue: TerminalFrameEventQueue,
    recording_writer: RecordingWriteHandle,
) {
    let mut sessions: HashMap<String, TerminalFrameSession> = HashMap::new();
    let mut pending_commands = VecDeque::new();
    while let Some(command) = next_terminal_frame_command(&command_rx, &mut pending_commands) {
        match command {
            TerminalFrameCommand::EnsureSession {
                session_id,
                encoding,
                scrollback_limit,
            } => {
                sessions
                    .entry(session_id)
                    .or_insert_with(|| TerminalFrameSession::new(&encoding, scrollback_limit))
                    .set_encoding_and_limit(&encoding, scrollback_limit);
            }
            TerminalFrameCommand::SeedSession {
                session_id,
                output,
                encoding,
                scrollback_limit,
            } => {
                let session = sessions
                    .entry(session_id)
                    .or_insert_with(|| TerminalFrameSession::new(&encoding, scrollback_limit));
                session.seed(output, &encoding, scrollback_limit);
            }
            TerminalFrameCommand::RemoveSession { session_id } => {
                sessions.remove(&session_id);
            }
            TerminalFrameCommand::ResizeSession {
                session_id,
                cols,
                rows,
            } => {
                if let Some(session) = sessions.get_mut(&session_id) {
                    session.resize(cols, rows);
                }
            }
            TerminalFrameCommand::Output {
                session_id,
                data,
                encoding,
                scrollback_limit,
            } => {
                let event = process_terminal_frame_output_burst(
                    &command_rx,
                    &mut pending_commands,
                    &mut sessions,
                    &recording_writer,
                    session_id,
                    data,
                    encoding,
                    scrollback_limit,
                );
                event_queue.push(TerminalFrameEvent::Output(event));
            }
            TerminalFrameCommand::RequestSnapshot {
                session_id,
                offset,
                action_links_enabled,
                action_link_matchers,
            } => {
                if let Some(session) = sessions.get(&session_id) {
                    let event = session.snapshot_event(
                        session_id,
                        offset,
                        action_links_enabled,
                        action_link_matchers,
                    );
                    event_queue.push(TerminalFrameEvent::Snapshot(event));
                }
            }
            TerminalFrameCommand::RequestSearch { session_id, key } => {
                if let Some(session) = sessions.get(&session_id) {
                    let event = session.search_event(session_id, key);
                    event_queue.push(TerminalFrameEvent::Search(event));
                }
            }
            TerminalFrameCommand::RequestBufferText {
                session_id,
                max_bytes,
                request_id,
            } => {
                if let Some(session) = sessions.get(&session_id) {
                    let event = session.buffer_text_event(session_id, request_id, max_bytes);
                    event_queue.push(TerminalFrameEvent::BufferText(event));
                }
            }
        }
    }
}

fn process_terminal_frame_output_burst(
    command_rx: &TerminalFrameCommandReceiver,
    pending_commands: &mut VecDeque<TerminalFrameCommand>,
    sessions: &mut HashMap<String, TerminalFrameSession>,
    recording_writer: &RecordingWriteHandle,
    session_id: String,
    data: Vec<u8>,
    encoding: String,
    scrollback_limit: usize,
) -> TerminalFrameOutputEvent {
    let started_at = Instant::now();
    let mut batch = TerminalFrameOutputBatch::default();
    let mut processed_bytes = 0usize;
    {
        let session = sessions
            .entry(session_id.clone())
            .or_insert_with(|| TerminalFrameSession::new(&encoding, scrollback_limit));
        processed_bytes = processed_bytes.saturating_add(data.len());
        batch.absorb(session.process_output_chunk(
            &session_id,
            &data,
            &encoding,
            scrollback_limit,
            recording_writer,
        ));
    }

    loop {
        if !terminal_frame_output_batch_should_continue(
            processed_bytes,
            started_at.elapsed(),
            TERMINAL_FRAME_OUTPUT_BURST_BYTE_LIMIT,
            TERMINAL_FRAME_OUTPUT_BURST_WALL_BUDGET,
        ) {
            break;
        }
        let next = pending_commands
            .pop_front()
            .or_else(|| command_rx.try_recv());
        let Some(next) = next else {
            break;
        };
        match next {
            TerminalFrameCommand::Output {
                session_id: next_session_id,
                data: next_data,
                encoding: next_encoding,
                scrollback_limit: next_scrollback_limit,
            } if terminal_frame_output_commands_can_merge(
                &session_id,
                &encoding,
                scrollback_limit,
                &next_session_id,
                &next_encoding,
                next_scrollback_limit,
                processed_bytes,
                next_data.len(),
                TERMINAL_FRAME_OUTPUT_BURST_BYTE_LIMIT,
            ) =>
            {
                processed_bytes = processed_bytes.saturating_add(next_data.len());
                let session = sessions
                    .entry(session_id.clone())
                    .or_insert_with(|| TerminalFrameSession::new(&encoding, scrollback_limit));
                batch.absorb(session.process_output_chunk(
                    &session_id,
                    &next_data,
                    &encoding,
                    scrollback_limit,
                    recording_writer,
                ));
            }
            other => {
                pending_commands.push_front(other);
                break;
            }
        }
    }

    let session = sessions
        .entry(session_id.clone())
        .or_insert_with(|| TerminalFrameSession::new(&encoding, scrollback_limit));
    session.output_event_from_batch(session_id, batch, started_at)
}

fn next_terminal_frame_command(
    command_rx: &TerminalFrameCommandReceiver,
    pending_commands: &mut VecDeque<TerminalFrameCommand>,
) -> Option<TerminalFrameCommand> {
    pending_commands.pop_front().or_else(|| command_rx.recv())
}

#[cfg(test)]
fn coalesce_terminal_frame_output_command(
    command_rx: &TerminalFrameCommandReceiver,
    pending_commands: &mut VecDeque<TerminalFrameCommand>,
    session_id: String,
    mut data: Vec<u8>,
    encoding: String,
    scrollback_limit: usize,
) -> (String, Vec<u8>, String, usize) {
    loop {
        let next = pending_commands
            .pop_front()
            .or_else(|| command_rx.try_recv());
        let Some(next) = next else {
            break;
        };
        match next {
            TerminalFrameCommand::Output {
                session_id: next_session_id,
                data: next_data,
                encoding: next_encoding,
                scrollback_limit: next_scrollback_limit,
            } if terminal_frame_output_commands_can_merge(
                &session_id,
                &encoding,
                scrollback_limit,
                &next_session_id,
                &next_encoding,
                next_scrollback_limit,
                data.len(),
                next_data.len(),
                TERMINAL_FRAME_OUTPUT_COALESCE_BYTE_LIMIT,
            ) =>
            {
                data.extend(next_data);
            }
            other => {
                pending_commands.push_front(other);
                break;
            }
        }
    }

    (session_id, data, encoding, scrollback_limit)
}

fn terminal_frame_output_commands_can_merge(
    session_id: &str,
    encoding: &str,
    scrollback_limit: usize,
    next_session_id: &str,
    next_encoding: &str,
    next_scrollback_limit: usize,
    current_bytes: usize,
    next_bytes: usize,
    byte_limit: usize,
) -> bool {
    session_id == next_session_id
        && encoding == next_encoding
        && scrollback_limit == next_scrollback_limit
        && current_bytes.saturating_add(next_bytes) <= byte_limit
}

const TERMINAL_FRAME_EVENT_QUEUE_CAP: usize = 1024;
const TERMINAL_FRAME_COMMAND_QUEUE_CAP: usize = 512;
#[cfg(test)]
const TERMINAL_FRAME_OUTPUT_COALESCE_BYTE_LIMIT: usize = 32 * 1024;
const TERMINAL_FRAME_OUTPUT_BURST_BYTE_LIMIT: usize = 64 * 1024;
const TERMINAL_FRAME_OUTPUT_BURST_WALL_BUDGET: Duration = Duration::from_millis(4);

fn terminal_frame_output_batch_should_continue(
    processed_bytes: usize,
    elapsed: Duration,
    byte_limit: usize,
    wall_budget: Duration,
) -> bool {
    processed_bytes < byte_limit && elapsed < wall_budget
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output_frame_with_sizes(
        accepted_bytes: usize,
        skipped_output_bytes: usize,
    ) -> TerminalFrameOutputEvent {
        TerminalFrameOutputEvent {
            session_id: "s1".to_string(),
            visible_text: "x".to_string(),
            recording_text_bytes: 1,
            snapshot: Arc::new(TerminalScreen::default().viewport_snapshot(0)),
            action_links: None,
            protocol_state: TerminalProtocolState::default(),
            effects: TerminalEffects::default(),
            command_running: false,
            accepted_bytes,
            skipped_output_bytes,
            revision: 1,
            process_duration: Duration::ZERO,
        }
    }

    fn apply_output_frame_to_view(view: &mut TerminalViewState, frame: TerminalFrameOutputEvent) {
        let TerminalFrameOutputEvent {
            visible_text,
            snapshot,
            action_links,
            protocol_state,
            accepted_bytes,
            skipped_output_bytes,
            revision,
            ..
        } = frame;
        view.apply_terminal_frame_parts(
            &visible_text,
            snapshot,
            action_links,
            protocol_state,
            accepted_bytes,
            skipped_output_bytes,
            revision,
        );
    }

    #[test]
    fn terminal_view_output_decodes_session_charset() {
        let mut view = TerminalViewState::new();
        view.set_encoding("GBK");

        view.append_bytes_unprotected(&[0xb2]);
        assert!(view.output.is_empty());

        view.append_bytes_unprotected(&[0xe2, 0xca, 0xd4]);
        assert_eq!(view.output, "测试");
        assert!(view.screen.lines().join("").contains("测试"));
    }

    #[test]
    fn terminal_view_local_text_bypasses_session_charset() {
        let mut view = TerminalViewState::new();
        view.set_encoding("GBK");

        view.append_text("本地提示");

        assert_eq!(view.output, "本地提示");
        let joined = view.screen.lines().join("");
        let compact = joined.replace(' ', "");
        assert!(compact.contains("本地提示"), "grid={joined:?}");
        assert!(!joined.contains('\u{fffd}'), "grid={joined:?}");
    }

    #[test]
    fn terminal_view_seed_output_applies_session_encoding() {
        let view = TerminalViewState::from_output_with_encoding("seed".to_string(), "GBK");

        assert_eq!(view.screen.encoding_label(), "GBK");
        assert_eq!(view.output_decoder.encoding_label(), "GBK");
        assert_eq!(view.recording_decoder.encoding_label(), "GBK");
        assert_eq!(
            view.screen.encode_outgoing("测试".as_bytes()),
            [0xb2, 0xe2, 0xca, 0xd4]
        );
    }

    #[test]
    fn terminal_view_output_decodes_split_utf8() {
        let mut view = TerminalViewState::new();
        let bytes = "测".as_bytes();

        view.append_bytes_unprotected(&bytes[..1]);
        assert!(view.output.is_empty());

        view.append_bytes_unprotected(&bytes[1..]);
        assert_eq!(view.output, "测");
        assert!(view.screen.lines().join("").contains('测'));
    }

    #[test]
    fn terminal_view_output_burst_drop_resets_stream_decoders() {
        let mut view = TerminalViewState::new();
        view.set_encoding("GBK");

        // First byte of "测" in GBK, intentionally left incomplete.
        view.append_bytes_unprotected(&[0xb2]);
        assert!(view.output.is_empty());

        let mut burst = vec![b'a'; TERMINAL_OUTPUT_VISIBLE_BACKLOG_CAP + 2];
        // The first byte retained after the forced drop is the second byte of
        // "测". It must not combine with the skipped/pending 0xb2 above.
        burst[2] = 0xe2;

        let feed = view.protect_output_burst(&burst);
        view.append_bytes_unprotected(feed);

        assert!(!view.output.contains('测'), "output={:?}", view.output);
        let grid = view.screen.lines().join("");
        assert!(!grid.contains('测'), "grid={grid:?}");
        assert_eq!(view.skipped_output_chars, 2);
    }

    #[test]
    fn terminal_output_burst_helper_resets_screen_and_decoder() {
        let mut screen = TerminalScreen::default();
        let mut decoder = TerminalOutputDecoder::default();
        screen.set_encoding("GBK");
        decoder.set_encoding("GBK");

        // First byte of "测" in GBK, intentionally left incomplete in both
        // streaming consumers.
        screen.advance(&[0xb2]);
        assert!(decoder.decode_output_text(&[0xb2]).is_empty());

        let mut burst = vec![b'a'; TERMINAL_OUTPUT_VISIBLE_BACKLOG_CAP + 2];
        // The first retained byte is the second half of "测". It must not pair
        // with the skipped/pending first byte after protection resets state.
        burst[2] = 0xe2;

        let (feed, skipped) = protect_terminal_output_burst(&mut screen, &mut decoder, &burst);
        screen.advance(feed);
        let output = decoder.decode_output_text(feed);

        assert_eq!(skipped, 2);
        assert!(!output.contains('测'), "output={output:?}");
        let grid = screen.lines().join("");
        assert!(!grid.contains('测'), "grid={grid:?}");
    }

    #[test]
    fn terminal_frame_large_accepted_output_does_not_show_protection_overlay() {
        let mut view = TerminalViewState::new();
        let frame = output_frame_with_sizes((32 * 1024) + 1, 0);

        apply_output_frame_to_view(&mut view, frame);

        assert_eq!(view.performance_mode, TerminalPerformanceMode::Normal);
        assert_eq!(view.performance_overlay, None);
        assert_eq!(view.skipped_output_chars, 0);
    }

    #[test]
    fn terminal_background_frame_apply_skips_render_work() {
        let mut view = TerminalViewState::new();
        view.render_degraded = false;
        let frame = output_frame_with_sizes((32 * 1024) + 1, 0);

        view.apply_terminal_background_frame_parts(
            frame.snapshot.clone(),
            frame.action_links.clone(),
            frame.protocol_state,
            frame.skipped_output_bytes,
            frame.revision,
        );

        assert_eq!(view.output, "");
        assert_eq!(view.screen_revision, frame.revision);
        assert!(view.frame_snapshot.is_some());
        assert_eq!(view.output_burst_bytes, 0);
        assert!(!view.render_degraded);
        assert_eq!(view.performance_mode, TerminalPerformanceMode::Normal);
        assert_eq!(view.performance_overlay, None);
    }

    #[test]
    fn terminal_frame_skipped_output_shows_protection_overlay() {
        let mut view = TerminalViewState::new();
        let frame = output_frame_with_sizes(1, 7);

        apply_output_frame_to_view(&mut view, frame);

        assert_eq!(view.performance_mode, TerminalPerformanceMode::Overloaded);
        assert_eq!(
            view.performance_overlay,
            Some(TerminalPerformanceOverlay::Overloaded)
        );
        assert_eq!(view.skipped_output_chars, 7);
    }

    #[test]
    fn terminal_view_output_discontinuity_resets_all_stream_decoders() {
        let mut view = TerminalViewState::new();
        view.set_encoding("GBK");

        // First byte of "测" in GBK, intentionally left incomplete in all
        // three streaming consumers: screen, visible output, and recording.
        view.append_bytes_unprotected(&[0xb2]);
        assert!(view.output.is_empty());
        assert!(
            view.recording_decoder
                .decode_output_text(&[0xb2])
                .is_empty()
        );

        view.note_output_discontinuity(7);

        view.append_bytes_unprotected(&[0xe2]);
        let recorded = view.recording_decoder.decode_output_text(&[0xe2]);

        assert!(!view.output.contains('测'), "output={:?}", view.output);
        assert!(!recorded.contains('测'), "recorded={recorded:?}");
        let grid = view.screen.lines().join("");
        assert!(!grid.contains('测'), "grid={grid:?}");
        assert_eq!(view.skipped_output_chars, 7);
    }

    #[test]
    fn terminal_view_output_skips_graphics_payload() {
        let mut view = TerminalViewState::new();

        view.append_bytes_unprotected(b"pre\x1b_Ga=T,i=1,c=1,r=1;QUI=\x1b\\post");

        assert_eq!(view.output, "prepost");
        assert!(view.screen.lines().join("").contains("prepost"));
    }

    #[test]
    fn terminal_view_filtered_visible_text_can_reenter_byte_parser() {
        let mut view = TerminalViewState::new();
        let visible_text = "plain \x1b[31mred\x1b[0m";
        let visible_bytes = view.screen.encode_outgoing_str(visible_text);

        view.append_bytes_unprotected(&visible_bytes);

        let snapshot = view.screen.snapshot();
        let red_cell = snapshot
            .cells
            .iter()
            .find(|cell| cell.text == "r")
            .expect("styled red cell");
        assert_eq!(red_cell.style.fg, Some(1));
    }

    #[test]
    fn terminal_ui_output_tail_is_bounded_and_utf8_safe() {
        let mut output = format!("{}界", "好".repeat(TERMINAL_UI_OUTPUT_TAIL_CAP));

        append_terminal_ui_output_tail(&mut output, "done");

        assert!(output.len() <= TERMINAL_UI_OUTPUT_TAIL_CAP);
        assert!(output.ends_with("done"));
        assert!(std::str::from_utf8(output.as_bytes()).is_ok());
    }

    #[test]
    fn terminal_frame_visible_text_event_keeps_only_tail_for_ui() {
        let mut session = TerminalFrameSession::new("UTF-8", 1000);
        let recording_manager = Arc::new(nyaterm_transport::RecordingManager::new());
        let recording_pipeline =
            super::super::RecordingWritePipeline::spawn(Arc::clone(&recording_manager));
        let input = format!(
            "{}tail",
            "x".repeat(TERMINAL_FRAME_VISIBLE_TEXT_TAIL_CAP + 1024)
        );

        let event = session.process_output(
            "s1".to_string(),
            input.into_bytes(),
            "UTF-8".to_string(),
            1000,
            &recording_pipeline.writer(),
        );

        assert!(event.visible_text.len() <= TERMINAL_FRAME_VISIBLE_TEXT_TAIL_CAP);
        assert!(event.visible_text.ends_with("tail"));
        assert_eq!(
            event.recording_text_bytes,
            TERMINAL_FRAME_VISIBLE_TEXT_TAIL_CAP + 1028
        );
        recording_pipeline.flush();
        let recorded = recording_manager
            .search_history(nyaterm_transport::TerminalHistorySearchRequest {
                session_id: "s1".to_string(),
                query: "tail".to_string(),
                case_sensitive: false,
                regex: false,
                whole_word: false,
                limit: Some(10),
                context_before: Some(0),
                context_after: Some(0),
                max_lines: None,
            })
            .expect("recording history search should succeed");
        assert_eq!(recorded.total, 1);
    }

    #[test]
    fn terminal_frame_buffer_text_event_is_prepared_off_ui_state() {
        let mut session = TerminalFrameSession::new("UTF-8", 1000);
        session.screen.advance_decoded_text("alpha\n");
        session.screen.advance_decoded_text(&"x".repeat(512));

        let event = session.buffer_text_event("s1".to_string(), "r1".to_string(), 64);

        assert_eq!(event.session_id, "s1");
        assert_eq!(event.request_id, "r1");
        assert!(event.truncated);
        assert!(event.text.len() <= 64);
        assert!(event.text.ends_with('x'));
    }

    #[test]
    fn terminal_frame_search_result_current_requires_matching_revision() {
        let key = TerminalFrameSearchKey {
            query: "alpha".to_string(),
            case_sensitive: false,
            regex: false,
            whole_word: false,
            limit: 100,
        };
        let result = TerminalFrameSearchResult {
            key: key.clone(),
            revision: 7,
            matches: Ok(Vec::new()),
        };
        let other_key = TerminalFrameSearchKey {
            query: "beta".to_string(),
            ..key.clone()
        };

        assert!(terminal_frame_search_result_is_current(&result, &key, 7));
        assert!(!terminal_frame_search_result_is_current(&result, &key, 8));
        assert!(!terminal_frame_search_result_is_current(
            &result, &other_key, 7
        ));
    }

    #[test]
    fn terminal_frame_search_event_carries_current_session_revision() {
        let mut session = TerminalFrameSession::new("UTF-8", 1000);
        session.screen.advance_decoded_text("alpha\nbeta");
        session.revision = 3;
        let key = TerminalFrameSearchKey {
            query: "alpha".to_string(),
            case_sensitive: false,
            regex: false,
            whole_word: false,
            limit: 100,
        };

        let event = session.search_event("s1".to_string(), key.clone());

        assert_eq!(event.session_id, "s1");
        assert_eq!(event.result.key, key);
        assert_eq!(event.result.revision, 3);
        assert_eq!(event.result.matches.unwrap().len(), 1);
    }

    #[test]
    fn terminal_view_backend_resize_detects_pixel_only_changes() {
        let mut view = TerminalViewState::new();

        assert!(view.backend_resize_changed(80, 24, 800, 432));
        view.remember_backend_resize(80, 24, 800, 432);

        assert!(!view.backend_resize_changed(80, 24, 800, 432));
        assert!(view.backend_resize_changed(80, 24, 960, 432));
        assert!(view.backend_resize_changed(80, 25, 800, 450));
    }

    #[test]
    fn terminal_protocol_state_encodes_sgr_mouse_report() {
        let protocol = TerminalProtocolState {
            mouse_reporting: true,
            mouse_sgr: true,
            ..TerminalProtocolState::default()
        };

        assert_eq!(
            protocol.encode_mouse_report(0, 1, 2, true, false, false, false, false),
            b"\x1b[<0;2;3M".to_vec()
        );
        assert_eq!(
            protocol.encode_mouse_report(0, 1, 2, false, false, false, false, false),
            b"\x1b[<0;2;3m".to_vec()
        );
    }

    #[test]
    fn terminal_protocol_state_blocks_alternate_scroll_when_mouse_reporting() {
        let protocol = TerminalProtocolState {
            alternate_screen: true,
            alternate_scroll: true,
            mouse_reporting: true,
            application_cursor_keys: true,
            ..TerminalProtocolState::default()
        };

        assert_eq!(protocol.alternate_scroll_payload(1), None);
    }

    #[test]
    fn terminal_protocol_state_emits_alternate_scroll_payload() {
        let protocol = TerminalProtocolState {
            alternate_screen: true,
            alternate_scroll: true,
            application_cursor_keys: true,
            ..TerminalProtocolState::default()
        };

        assert_eq!(
            protocol.alternate_scroll_payload(1),
            Some(b"\x1bOA".to_vec())
        );
    }

    #[test]
    fn terminal_frame_event_queue_coalesces_pure_output_to_latest() {
        let queue = TerminalFrameEventQueue::new(8);
        let mut first = output_frame_with_sizes(1, 0);
        first.revision = 1;
        let mut second = output_frame_with_sizes(2, 0);
        second.revision = 2;

        queue.push(TerminalFrameEvent::Output(first));
        queue.push(TerminalFrameEvent::Output(second));

        assert!(matches!(
            queue.try_recv(),
            Some(TerminalFrameEvent::Output(frame)) if frame.revision == 2
        ));
        assert!(queue.try_recv().is_none());
    }

    #[test]
    fn terminal_frame_event_queue_preserves_output_effects() {
        let queue = TerminalFrameEventQueue::new(8);
        let mut effect_frame = output_frame_with_sizes(1, 0);
        effect_frame.revision = 1;
        effect_frame.effects.bell = true;
        let mut latest = output_frame_with_sizes(2, 0);
        latest.revision = 2;

        queue.push(TerminalFrameEvent::Output(effect_frame));
        queue.push(TerminalFrameEvent::Output(latest));

        assert!(matches!(
            queue.try_recv(),
            Some(TerminalFrameEvent::Output(frame)) if frame.revision == 1 && frame.effects.bell
        ));
        assert!(matches!(
            queue.try_recv(),
            Some(TerminalFrameEvent::Output(frame)) if frame.revision == 2
        ));
        assert!(queue.try_recv().is_none());
    }

    #[test]
    fn expensive_interactions_require_active_calm_terminal() {
        assert!(terminal_expensive_interactions_enabled(
            true,
            true,
            false,
            false,
            0,
            TerminalPerformanceMode::Normal,
        ));
        assert!(!terminal_expensive_interactions_enabled(
            false,
            true,
            false,
            false,
            0,
            TerminalPerformanceMode::Normal,
        ));
        assert!(!terminal_expensive_interactions_enabled(
            true,
            false,
            false,
            false,
            0,
            TerminalPerformanceMode::Normal,
        ));
    }

    #[test]
    fn expensive_interactions_yield_under_render_pressure() {
        assert!(!terminal_expensive_interactions_enabled(
            true,
            true,
            true,
            false,
            0,
            TerminalPerformanceMode::Normal,
        ));
        assert!(!terminal_expensive_interactions_enabled(
            true,
            true,
            false,
            true,
            0,
            TerminalPerformanceMode::Normal,
        ));
        assert!(!terminal_expensive_interactions_enabled(
            true,
            true,
            false,
            false,
            1,
            TerminalPerformanceMode::Normal,
        ));
        assert!(!terminal_expensive_interactions_enabled(
            true,
            true,
            false,
            false,
            0,
            TerminalPerformanceMode::Overloaded,
        ));
    }

    #[test]
    fn terminal_frame_event_queue_drains_batch_with_limit() {
        let queue = TerminalFrameEventQueue::new(8);
        let mut first = output_frame_with_sizes(1, 0);
        first.revision = 1;
        first.effects.bell = true;
        let mut second = output_frame_with_sizes(2, 0);
        second.revision = 2;
        second.effects.bell = true;
        let mut third = output_frame_with_sizes(3, 0);
        third.revision = 3;
        third.effects.bell = true;

        queue.push(TerminalFrameEvent::Output(first));
        queue.push(TerminalFrameEvent::Output(second));
        queue.push(TerminalFrameEvent::Output(third));

        let mut drained = VecDeque::new();
        assert_eq!(queue.drain_into(&mut drained, 2), 2);
        assert_eq!(drained.len(), 2);
        assert!(matches!(
            drained.pop_front(),
            Some(TerminalFrameEvent::Output(frame)) if frame.revision == 1
        ));
        assert!(matches!(
            drained.pop_front(),
            Some(TerminalFrameEvent::Output(frame)) if frame.revision == 2
        ));
        assert!(matches!(
            queue.try_recv(),
            Some(TerminalFrameEvent::Output(frame)) if frame.revision == 3
        ));
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn terminal_frame_action_links_align_with_snapshot_lines() {
        let mut screen = TerminalScreen::default();
        screen.advance_decoded_text("visit http://example.com\nping 10.0.0.1");
        let snapshot = screen.viewport_snapshot(0);
        let matchers = ActionLinksMatcherSettings::default();

        let links = prepare_terminal_frame_action_links(&snapshot, true, &matchers).unwrap();

        assert_eq!(links.matches_by_line.len(), snapshot.lines.len());
        assert_eq!(links.cell_ranges_by_line.len(), snapshot.lines.len());
        assert!(
            links
                .matches_by_line
                .iter()
                .flatten()
                .any(|item| item.value == "http://example.com")
        );
        assert!(
            links
                .cell_ranges_by_line
                .iter()
                .flatten()
                .any(|range| *range == (6, 24))
        );
        assert!(
            links
                .matches_by_line
                .iter()
                .flatten()
                .any(|item| item.value == "10.0.0.1")
        );

        let disabled = prepare_terminal_frame_action_links(&snapshot, false, &matchers).unwrap();
        assert!(disabled.matches_by_line.iter().all(Vec::is_empty));
        assert!(disabled.cell_ranges_by_line.iter().all(Vec::is_empty));
        assert_ne!(links.matcher_key, disabled.matcher_key);
    }

    #[test]
    fn terminal_frame_worker_coalesces_adjacent_matching_output() {
        let (tx, rx) = terminal_frame_command_channel();
        assert!(tx.send(TerminalFrameCommand::Output {
            session_id: "s1".to_string(),
            data: b"bc".to_vec(),
            encoding: "UTF-8".to_string(),
            scrollback_limit: 1000,
        }));
        drop(tx);

        let mut pending = VecDeque::new();
        let (_, data, _, _) = coalesce_terminal_frame_output_command(
            &rx,
            &mut pending,
            "s1".to_string(),
            b"a".to_vec(),
            "UTF-8".to_string(),
            1000,
        );

        assert_eq!(data, b"abc");
        assert!(pending.is_empty());
    }

    #[test]
    fn terminal_frame_worker_caps_coalesced_output_batch() {
        let (tx, rx) = terminal_frame_command_channel();
        assert!(tx.send(TerminalFrameCommand::Output {
            session_id: "s1".to_string(),
            data: vec![b'b'; 2],
            encoding: "UTF-8".to_string(),
            scrollback_limit: 1000,
        }));
        drop(tx);

        let mut pending = VecDeque::new();
        let (_, data, _, _) = coalesce_terminal_frame_output_command(
            &rx,
            &mut pending,
            "s1".to_string(),
            vec![b'a'; TERMINAL_FRAME_OUTPUT_COALESCE_BYTE_LIMIT - 1],
            "UTF-8".to_string(),
            1000,
        );

        assert_eq!(data.len(), TERMINAL_FRAME_OUTPUT_COALESCE_BYTE_LIMIT - 1);
        assert!(matches!(
            next_terminal_frame_command(&rx, &mut pending),
            Some(TerminalFrameCommand::Output { data, .. }) if data == vec![b'b'; 2]
        ));
    }

    #[test]
    fn terminal_frame_worker_does_not_coalesce_across_resize() {
        let (tx, rx) = terminal_frame_command_channel();
        assert!(tx.send(TerminalFrameCommand::ResizeSession {
            session_id: "s1".to_string(),
            cols: 100,
            rows: 30,
        }));
        assert!(tx.send(TerminalFrameCommand::Output {
            session_id: "s1".to_string(),
            data: b"bc".to_vec(),
            encoding: "UTF-8".to_string(),
            scrollback_limit: 1000,
        }));
        drop(tx);

        let mut pending = VecDeque::new();
        let (_, data, _, _) = coalesce_terminal_frame_output_command(
            &rx,
            &mut pending,
            "s1".to_string(),
            b"a".to_vec(),
            "UTF-8".to_string(),
            1000,
        );

        assert_eq!(data, b"a");
        assert!(matches!(
            next_terminal_frame_command(&rx, &mut pending),
            Some(TerminalFrameCommand::ResizeSession { .. })
        ));
        assert!(matches!(
            next_terminal_frame_command(&rx, &mut pending),
            Some(TerminalFrameCommand::Output { .. })
        ));
    }

    #[test]
    fn terminal_frame_worker_batches_output_burst_into_single_frame() {
        let (tx, rx) = terminal_frame_command_channel();
        assert!(tx.send(TerminalFrameCommand::Output {
            session_id: "s1".to_string(),
            data: b"bc".to_vec(),
            encoding: "UTF-8".to_string(),
            scrollback_limit: 1000,
        }));
        drop(tx);

        let recording_manager = Arc::new(nyaterm_transport::RecordingManager::new());
        let recording_pipeline =
            super::super::RecordingWritePipeline::spawn(Arc::clone(&recording_manager));
        let mut pending = VecDeque::new();
        let mut sessions = HashMap::new();
        let event = process_terminal_frame_output_burst(
            &rx,
            &mut pending,
            &mut sessions,
            &recording_pipeline.writer(),
            "s1".to_string(),
            b"a".to_vec(),
            "UTF-8".to_string(),
            1000,
        );

        assert_eq!(event.visible_text, "abc");
        assert_eq!(event.recording_text_bytes, 3);
        assert_eq!(event.accepted_bytes, 3);
        assert_eq!(event.revision, 2);
        assert!(event.snapshot.lines.join("").contains("abc"));
        assert!(pending.is_empty());
    }

    #[test]
    fn terminal_frame_worker_batch_stops_at_resize_boundary() {
        let (tx, rx) = terminal_frame_command_channel();
        assert!(tx.send(TerminalFrameCommand::ResizeSession {
            session_id: "s1".to_string(),
            cols: 100,
            rows: 30,
        }));
        assert!(tx.send(TerminalFrameCommand::Output {
            session_id: "s1".to_string(),
            data: b"bc".to_vec(),
            encoding: "UTF-8".to_string(),
            scrollback_limit: 1000,
        }));
        drop(tx);

        let recording_manager = Arc::new(nyaterm_transport::RecordingManager::new());
        let recording_pipeline =
            super::super::RecordingWritePipeline::spawn(Arc::clone(&recording_manager));
        let mut pending = VecDeque::new();
        let mut sessions = HashMap::new();
        let event = process_terminal_frame_output_burst(
            &rx,
            &mut pending,
            &mut sessions,
            &recording_pipeline.writer(),
            "s1".to_string(),
            b"a".to_vec(),
            "UTF-8".to_string(),
            1000,
        );

        assert_eq!(event.visible_text, "a");
        assert!(matches!(
            next_terminal_frame_command(&rx, &mut pending),
            Some(TerminalFrameCommand::ResizeSession { .. })
        ));
        assert!(matches!(
            next_terminal_frame_command(&rx, &mut pending),
            Some(TerminalFrameCommand::Output { .. })
        ));
    }

    #[test]
    fn terminal_frame_output_batch_policy_stops_at_latency_budget() {
        assert!(terminal_frame_output_batch_should_continue(
            1024,
            Duration::from_millis(1),
            TERMINAL_FRAME_OUTPUT_BURST_BYTE_LIMIT,
            TERMINAL_FRAME_OUTPUT_BURST_WALL_BUDGET,
        ));
        assert!(!terminal_frame_output_batch_should_continue(
            1024,
            TERMINAL_FRAME_OUTPUT_BURST_WALL_BUDGET,
            TERMINAL_FRAME_OUTPUT_BURST_BYTE_LIMIT,
            TERMINAL_FRAME_OUTPUT_BURST_WALL_BUDGET,
        ));
    }

    #[test]
    fn terminal_frame_output_batch_policy_stops_at_byte_budget() {
        assert!(!terminal_frame_output_batch_should_continue(
            TERMINAL_FRAME_OUTPUT_BURST_BYTE_LIMIT,
            Duration::from_millis(1),
            TERMINAL_FRAME_OUTPUT_BURST_BYTE_LIMIT,
            TERMINAL_FRAME_OUTPUT_BURST_WALL_BUDGET,
        ));
    }

    #[test]
    fn terminal_frame_command_queue_keeps_output_chunks_separate_for_worker() {
        let (tx, rx) = terminal_frame_command_channel();
        assert!(tx.send(TerminalFrameCommand::Output {
            session_id: "s1".to_string(),
            data: b"a".to_vec(),
            encoding: "UTF-8".to_string(),
            scrollback_limit: 1000,
        }));
        assert!(tx.send(TerminalFrameCommand::Output {
            session_id: "s1".to_string(),
            data: b"bc".to_vec(),
            encoding: "UTF-8".to_string(),
            scrollback_limit: 1000,
        }));

        assert!(matches!(
            rx.try_recv(),
            Some(TerminalFrameCommand::Output { data, .. }) if data == b"a"
        ));
        assert!(matches!(
            rx.try_recv(),
            Some(TerminalFrameCommand::Output { data, .. }) if data == b"bc"
        ));
        assert!(rx.try_recv().is_none());
    }

    #[test]
    fn terminal_frame_command_queue_does_not_coalesce_across_resize() {
        let (tx, rx) = terminal_frame_command_channel();
        assert!(tx.send(TerminalFrameCommand::Output {
            session_id: "s1".to_string(),
            data: b"a".to_vec(),
            encoding: "UTF-8".to_string(),
            scrollback_limit: 1000,
        }));
        assert!(tx.send(TerminalFrameCommand::ResizeSession {
            session_id: "s1".to_string(),
            cols: 100,
            rows: 30,
        }));
        assert!(tx.send(TerminalFrameCommand::Output {
            session_id: "s1".to_string(),
            data: b"bc".to_vec(),
            encoding: "UTF-8".to_string(),
            scrollback_limit: 1000,
        }));

        assert!(matches!(
            rx.try_recv(),
            Some(TerminalFrameCommand::Output { data, .. }) if data == b"a"
        ));
        assert!(matches!(
            rx.try_recv(),
            Some(TerminalFrameCommand::ResizeSession { .. })
        ));
        assert!(matches!(
            rx.try_recv(),
            Some(TerminalFrameCommand::Output { data, .. }) if data == b"bc"
        ));
    }

    #[test]
    fn terminal_frame_command_queue_keeps_latest_search_per_session() {
        let (tx, rx) = terminal_frame_command_channel();
        assert!(tx.send(TerminalFrameCommand::RequestSearch {
            session_id: "s1".to_string(),
            key: TerminalFrameSearchKey {
                query: "old".to_string(),
                case_sensitive: false,
                regex: false,
                whole_word: false,
                limit: 100,
            },
        }));
        assert!(tx.send(TerminalFrameCommand::RequestSearch {
            session_id: "s1".to_string(),
            key: TerminalFrameSearchKey {
                query: "new".to_string(),
                case_sensitive: false,
                regex: false,
                whole_word: false,
                limit: 100,
            },
        }));

        assert!(matches!(
            rx.try_recv(),
            Some(TerminalFrameCommand::RequestSearch { key, .. }) if key.query == "new"
        ));
        assert!(rx.try_recv().is_none());
    }

    #[test]
    fn terminal_frame_command_queue_keeps_latest_resize_per_session() {
        let (tx, rx) = terminal_frame_command_channel();
        assert!(tx.send(TerminalFrameCommand::ResizeSession {
            session_id: "s1".to_string(),
            cols: 80,
            rows: 24,
        }));
        assert!(tx.send(TerminalFrameCommand::ResizeSession {
            session_id: "s1".to_string(),
            cols: 120,
            rows: 40,
        }));

        assert!(matches!(
            rx.try_recv(),
            Some(TerminalFrameCommand::ResizeSession {
                cols: 120,
                rows: 40,
                ..
            })
        ));
        assert!(rx.try_recv().is_none());
    }

    #[test]
    fn terminal_frame_command_queue_keeps_resize_when_output_intervenes() {
        let (tx, rx) = terminal_frame_command_channel();
        assert!(tx.send(TerminalFrameCommand::ResizeSession {
            session_id: "s1".to_string(),
            cols: 80,
            rows: 24,
        }));
        assert!(tx.send(TerminalFrameCommand::Output {
            session_id: "s1".to_string(),
            data: b"a".to_vec(),
            encoding: "UTF-8".to_string(),
            scrollback_limit: 1000,
        }));
        assert!(tx.send(TerminalFrameCommand::ResizeSession {
            session_id: "s1".to_string(),
            cols: 120,
            rows: 40,
        }));

        assert!(matches!(
            rx.try_recv(),
            Some(TerminalFrameCommand::ResizeSession {
                cols: 80,
                rows: 24,
                ..
            })
        ));
        assert!(matches!(
            rx.try_recv(),
            Some(TerminalFrameCommand::Output { data, .. }) if data == b"a"
        ));
        assert!(matches!(
            rx.try_recv(),
            Some(TerminalFrameCommand::ResizeSession {
                cols: 120,
                rows: 40,
                ..
            })
        ));
        assert!(rx.try_recv().is_none());
    }

    #[test]
    fn terminal_frame_command_queue_caps_rebuildable_render_requests() {
        let (tx, rx) = terminal_frame_command_channel();
        for offset in 0..TERMINAL_FRAME_COMMAND_QUEUE_CAP + 32 {
            assert!(tx.send(TerminalFrameCommand::RequestSnapshot {
                session_id: format!("s{offset}"),
                offset,
                action_links_enabled: false,
                action_link_matchers: ActionLinksMatcherSettings::default(),
            }));
        }

        assert_eq!(tx.len(), TERMINAL_FRAME_COMMAND_QUEUE_CAP);
        let mut drained = 0usize;
        while let Some(command) = rx.try_recv() {
            assert!(matches!(
                command,
                TerminalFrameCommand::RequestSnapshot { .. }
            ));
            drained += 1;
        }
        assert_eq!(drained, TERMINAL_FRAME_COMMAND_QUEUE_CAP);
    }

    #[test]
    fn terminal_frame_command_queue_reports_queued_output_bytes() {
        let (tx, _rx) = terminal_frame_command_channel();
        assert!(tx.send(TerminalFrameCommand::Output {
            session_id: "s1".to_string(),
            data: b"abc".to_vec(),
            encoding: "UTF-8".to_string(),
            scrollback_limit: 1000,
        }));
        assert!(tx.send(TerminalFrameCommand::ResizeSession {
            session_id: "s1".to_string(),
            cols: 100,
            rows: 30,
        }));
        assert!(tx.send(TerminalFrameCommand::Output {
            session_id: "s1".to_string(),
            data: b"de".to_vec(),
            encoding: "UTF-8".to_string(),
            scrollback_limit: 1000,
        }));

        assert_eq!(tx.queued_output_bytes(), 5);
    }

    #[test]
    fn terminal_frame_command_queue_sends_many_in_order() {
        let (tx, rx) = terminal_frame_command_channel();
        assert!(!tx.send_many(Vec::<TerminalFrameCommand>::new()));
        assert!(tx.send_many(vec![
            TerminalFrameCommand::Output {
                session_id: "s1".to_string(),
                data: b"abc".to_vec(),
                encoding: "UTF-8".to_string(),
                scrollback_limit: 1000,
            },
            TerminalFrameCommand::ResizeSession {
                session_id: "s1".to_string(),
                cols: 100,
                rows: 30,
            },
            TerminalFrameCommand::Output {
                session_id: "s1".to_string(),
                data: b"de".to_vec(),
                encoding: "UTF-8".to_string(),
                scrollback_limit: 1000,
            },
        ]));

        assert_eq!(tx.queued_output_bytes(), 5);
        assert!(matches!(
            rx.try_recv(),
            Some(TerminalFrameCommand::Output { data, .. }) if data == b"abc"
        ));
        assert!(matches!(
            rx.try_recv(),
            Some(TerminalFrameCommand::ResizeSession {
                cols: 100,
                rows: 30,
                ..
            })
        ));
        assert!(matches!(
            rx.try_recv(),
            Some(TerminalFrameCommand::Output { data, .. }) if data == b"de"
        ));
        assert!(rx.try_recv().is_none());
    }

    #[test]
    fn terminal_frame_command_queue_stops_after_sender_drop() {
        let (tx, rx) = terminal_frame_command_channel();
        drop(tx);

        assert!(rx.recv().is_none());
    }

    #[test]
    fn render_degradation_stays_active_while_output_pressure_is_present() {
        let mut view = TerminalViewState::new();

        assert!(view.render_degraded);

        view.tick_performance_overlay(true);

        assert!(view.render_degraded);
        assert_eq!(view.render_degraded_calm_ticks, 0);
        for _ in 0..TERMINAL_RENDER_DEGRADATION_RECOVERY_TICKS {
            view.tick_performance_overlay(true);
        }
        assert!(view.render_degraded);
        assert_eq!(view.render_degraded_calm_ticks, 0);
    }

    #[test]
    fn render_degradation_is_initial_view_profile() {
        let mut view = TerminalViewState::new();

        assert!(view.render_degraded);
        for _ in 0..TERMINAL_RENDER_DEGRADATION_RECOVERY_TICKS {
            view.tick_performance_overlay(false);
        }

        assert!(!view.render_degraded);
    }

    #[test]
    fn render_degradation_starts_after_output_frame_applies() {
        let mut view = TerminalViewState::new();
        for _ in 0..TERMINAL_RENDER_DEGRADATION_RECOVERY_TICKS {
            view.tick_performance_overlay(false);
        }
        assert!(!view.render_degraded);
        let frame = output_frame_with_sizes(1, 0);

        apply_output_frame_to_view(&mut view, frame);
        view.tick_performance_overlay(false);

        assert!(view.render_degraded);
        assert_eq!(view.render_degraded_calm_ticks, 0);
    }

    #[test]
    fn render_degradation_recovers_after_consecutive_calm_ticks() {
        let mut view = TerminalViewState::new();
        view.enter_render_degraded_mode();

        for _ in 0..TERMINAL_RENDER_DEGRADATION_RECOVERY_TICKS.saturating_sub(1) {
            view.tick_performance_overlay(false);
            assert!(view.render_degraded);
        }
        view.tick_performance_overlay(false);

        assert!(!view.render_degraded);
        assert_eq!(view.render_degraded_calm_ticks, 0);
    }
}

impl SearchEngineEditorField {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Name => Self::Url,
            Self::Url => Self::Name,
        }
    }
}

/// Inclusive cell coordinate inside the visible terminal grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TerminalCellPos {
    pub(crate) row: usize,
    pub(crate) col: usize,
}

impl TerminalCellPos {
    pub(crate) fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

/// Visible-grid text selection (start/end are inclusive cell positions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TerminalSelection {
    pub(crate) anchor: TerminalCellPos,
    pub(crate) head: TerminalCellPos,
}

impl TerminalSelection {
    pub(crate) fn new(anchor: TerminalCellPos) -> Self {
        Self {
            anchor,
            head: anchor,
        }
    }

    pub(crate) fn ordered(&self) -> (TerminalCellPos, TerminalCellPos) {
        let a = self.anchor;
        let b = self.head;
        if (a.row, a.col) <= (b.row, b.col) {
            (a, b)
        } else {
            (b, a)
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Column range [start, end) for a painted line, if any cells are selected.
    /// Endpoints are inclusive cell positions; returned range is half-open for slicing.
    pub(crate) fn cols_for_row(&self, row: usize) -> Option<(usize, usize)> {
        if self.is_empty() {
            return None;
        }
        let (start, end) = self.ordered();
        if row < start.row || row > end.row {
            return None;
        }
        if start.row == end.row {
            return Some((start.col, end.col.saturating_add(1)));
        }
        if row == start.row {
            return Some((start.col, usize::MAX));
        }
        if row == end.row {
            return Some((0, end.col.saturating_add(1)));
        }
        Some((0, usize::MAX))
    }
}
