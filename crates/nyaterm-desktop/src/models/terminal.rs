use nyaterm_core::{
    ActionLinksMatcherSettings, TerminalBackendResize, terminal_backend_resize_changed,
};
use nyaterm_terminal::{TerminalEffects, TerminalOutputDecoder, TerminalScreen, TerminalSnapshot};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::{
    action_links::{ActionLinkMatch, find_action_links},
    terminal::{
        NyaTerminalLayoutCache, terminal_cell_col_for_byte_index, terminal_screen_from_output,
        trim_terminal_output,
    },
};

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

/// Match Tauri `XTERM_PERFORMANCE_CONFIG.output` thresholds (bytes).
pub(crate) const TERMINAL_OUTPUT_WRITE_CHUNK: usize = 32 * 1024;
pub(crate) const TERMINAL_OUTPUT_VISIBLE_BACKLOG_CAP: usize = 1_000_000;
pub(crate) const TERMINAL_OUTPUT_VISIBLE_BURST_OVERLOAD: usize = 256 * 1024;
/// ~3s recovery notice at the 50ms event-pump cadence.
pub(crate) const TERMINAL_PERFORMANCE_RECOVERY_TICKS: u8 = 60;
/// Require a short calm window before re-enabling expensive render decorations.
pub(crate) const TERMINAL_RENDER_DEGRADATION_RECOVERY_TICKS: u8 = 8;

#[derive(Debug, Clone, Default)]
pub(crate) struct TerminalFrameActionLinks {
    pub(crate) matcher_key: u64,
    pub(crate) matches_by_line: Vec<Vec<ActionLinkMatch>>,
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
    action_link_ranges_by_line_hash: HashMap<u64, Vec<(usize, usize)>>,
    pub(crate) layout_cache: Arc<Mutex<NyaTerminalLayoutCache>>,
    pub(crate) hits: u64,
    pub(crate) misses: u64,
}

impl TerminalRenderCache {
    pub(crate) fn clear(&mut self) {
        self.action_link_ranges_by_line_hash.clear();
        if let Ok(mut cache) = self.layout_cache.lock() {
            cache.clear();
        }
        self.hits = 0;
        self.misses = 0;
    }

    pub(crate) fn action_link_ranges(
        &mut self,
        line: &str,
        matchers: &nyaterm_core::ActionLinksMatcherSettings,
    ) -> Vec<(usize, usize)> {
        let key = terminal_line_cache_key(line, matchers);
        if let Some(ranges) = self.action_link_ranges_by_line_hash.get(&key) {
            self.hits = self.hits.saturating_add(1);
            return ranges.clone();
        }
        self.misses = self.misses.saturating_add(1);
        let ranges = crate::action_links::find_action_links(line, matchers, true)
            .into_iter()
            .map(|item| {
                let start = terminal_cell_col_for_byte_index(line, item.start);
                let end = terminal_cell_col_for_byte_index(line, item.end);
                (start, end)
            })
            .collect::<Vec<_>>();
        self.action_link_ranges_by_line_hash
            .insert(key, ranges.clone());
        ranges
    }
}

fn terminal_line_cache_key(line: &str, matchers: &nyaterm_core::ActionLinksMatcherSettings) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    line.hash(&mut hasher);
    matchers.ipv4.hash(&mut hasher);
    matchers.archive.hash(&mut hasher);
    matchers.host_port.hash(&mut hasher);
    hasher.finish()
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
    Some(TerminalFrameActionLinks {
        matcher_key: terminal_action_link_matcher_key(true, matchers),
        matches_by_line,
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

pub(crate) struct TerminalViewState {
    pub(crate) output: String,
    pub(crate) screen: TerminalScreen,
    /// Latest live viewport prepared by the background terminal frame processor.
    pub(crate) frame_snapshot: Option<TerminalSnapshot>,
    pub(crate) frame_action_links: Option<TerminalFrameActionLinks>,
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
            render_degraded: false,
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
            render_degraded: false,
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
        self.frame_snapshot = Some(self.screen.viewport_snapshot(0));
        self.frame_action_links = None;
        self.protocol_state = TerminalProtocolState::from_screen(&self.screen);
        self.output.push_str(text);
        trim_terminal_output(&mut self.output);
        if self.scroll_offset > 0 {
            self.has_new_while_scrolled = true;
        }
        self.clamp_scroll_offset();
    }

    /// Feed already-protected bytes into the view (used when the caller applies
    /// the same feed to the mirrored active screen).
    pub(crate) fn append_bytes_unprotected(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        self.screen.advance(data);
        self.screen_revision = self.screen_revision.saturating_add(1);
        self.frame_snapshot = Some(self.screen.viewport_snapshot(0));
        self.frame_action_links = None;
        self.protocol_state = TerminalProtocolState::from_screen(&self.screen);
        self.output
            .push_str(&self.output_decoder.decode_output_text(data));
        trim_terminal_output(&mut self.output);
        if self.scroll_offset > 0 {
            self.has_new_while_scrolled = true;
        }
        self.clamp_scroll_offset();
    }

    /// Drop the oldest part of an oversized burst so the latest screen state wins
    /// (Tauri backlog trim + large-output protection).
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
            || feed.len() > TERMINAL_OUTPUT_WRITE_CHUNK
        {
            self.enter_overloaded_mode();
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

    fn tick_render_degradation(&mut self) {
        if !self.render_degraded {
            return;
        }
        if self.output_burst_bytes > 0 {
            self.render_degraded_calm_ticks = 0;
            return;
        }
        self.render_degraded_calm_ticks = self.render_degraded_calm_ticks.saturating_add(1);
        if self.render_degraded_calm_ticks >= TERMINAL_RENDER_DEGRADATION_RECOVERY_TICKS {
            self.render_degraded = false;
            self.render_degraded_calm_ticks = 0;
        }
    }

    pub(crate) fn tick_performance_overlay(&mut self) {
        // End-of-tick calm accounting for recovery.
        self.maybe_exit_overloaded_mode();
        self.tick_render_degradation();
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
        self.render_degraded = false;
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

    pub(crate) fn apply_terminal_frame(&mut self, frame: &TerminalFrameEvent) {
        if !frame.visible_text.is_empty() {
            self.output.push_str(&frame.visible_text);
            trim_terminal_output(&mut self.output);
        }
        self.frame_snapshot = Some(frame.snapshot.clone());
        self.frame_action_links = frame.action_links.clone();
        self.protocol_state = frame.protocol_state;
        self.screen_revision = frame.revision;
        self.output_burst_bytes = self.output_burst_bytes.saturating_add(frame.accepted_bytes);
        if frame.skipped_output_bytes > 0 {
            self.note_skipped_output(frame.skipped_output_bytes);
        } else if self.output_burst_bytes > TERMINAL_OUTPUT_VISIBLE_BURST_OVERLOAD
            || frame.accepted_bytes > TERMINAL_OUTPUT_WRITE_CHUNK
        {
            self.enter_overloaded_mode();
        }
        if self.scroll_offset > 0 {
            self.has_new_while_scrolled = true;
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

#[derive(Debug)]
pub(crate) struct TerminalFramePipeline {
    command_tx: mpsc::Sender<TerminalFrameCommand>,
    event_rx: mpsc::Receiver<TerminalFrameEvent>,
}

impl TerminalFramePipeline {
    pub(crate) fn spawn() -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        thread::Builder::new()
            .name("nyaterm-terminal-frame-processor".to_string())
            .spawn(move || run_terminal_frame_processor(command_rx, event_tx))
            .expect("failed to spawn terminal frame processor");
        Self {
            command_tx,
            event_rx,
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
        action_links_enabled: bool,
        action_link_matchers: ActionLinksMatcherSettings,
    ) {
        if data.is_empty() {
            return;
        }
        let _ = self.command_tx.send(TerminalFrameCommand::Output {
            session_id: session_id.into(),
            data,
            encoding: encoding.into(),
            scrollback_limit,
            action_links_enabled,
            action_link_matchers,
        });
    }

    pub(crate) fn drain_events(&self, max_events: usize) -> Vec<TerminalFrameEvent> {
        let mut events = Vec::new();
        for _ in 0..max_events {
            match self.event_rx.try_recv() {
                Ok(event) => events.push(event),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        }
        events
    }
}

impl Default for TerminalFramePipeline {
    fn default() -> Self {
        Self::spawn()
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
        action_links_enabled: bool,
        action_link_matchers: ActionLinksMatcherSettings,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct TerminalFrameEvent {
    pub(crate) session_id: String,
    pub(crate) visible_text: String,
    pub(crate) recording_text: String,
    pub(crate) snapshot: TerminalSnapshot,
    pub(crate) action_links: Option<TerminalFrameActionLinks>,
    pub(crate) protocol_state: TerminalProtocolState,
    pub(crate) effects: TerminalEffects,
    pub(crate) command_running: bool,
    pub(crate) accepted_bytes: usize,
    pub(crate) skipped_output_bytes: usize,
    pub(crate) revision: u64,
    pub(crate) process_duration: Duration,
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

    fn process_output(
        &mut self,
        session_id: String,
        data: Vec<u8>,
        encoding: String,
        scrollback_limit: usize,
        action_links_enabled: bool,
        action_link_matchers: ActionLinksMatcherSettings,
    ) -> TerminalFrameEvent {
        let started_at = Instant::now();
        self.set_encoding_and_limit(&encoding, scrollback_limit);
        let recording_text = self.recording_decoder.decode_output_text(&data);
        let (feed, skipped_output_bytes) =
            protect_terminal_output_burst(&mut self.screen, &mut self.output_decoder, &data);
        self.screen.advance(feed);
        let visible_text = self.output_decoder.decode_output_text(feed);
        self.revision = self.revision.saturating_add(1);
        let effects = self.screen.take_effects();
        let command_running = self.screen.command_running();
        let protocol_state = TerminalProtocolState::from_screen(&self.screen);
        let snapshot = self.screen.viewport_snapshot(0);
        let action_links = prepare_terminal_frame_action_links(
            &snapshot,
            action_links_enabled,
            &action_link_matchers,
        );
        TerminalFrameEvent {
            session_id,
            visible_text,
            recording_text,
            snapshot,
            action_links,
            protocol_state,
            effects,
            command_running,
            accepted_bytes: feed.len(),
            skipped_output_bytes,
            revision: self.revision,
            process_duration: started_at.elapsed(),
        }
    }
}

fn run_terminal_frame_processor(
    command_rx: mpsc::Receiver<TerminalFrameCommand>,
    event_tx: mpsc::Sender<TerminalFrameEvent>,
) {
    let mut sessions: HashMap<String, TerminalFrameSession> = HashMap::new();
    while let Ok(command) = command_rx.recv() {
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
                action_links_enabled,
                action_link_matchers,
            } => {
                let session = sessions
                    .entry(session_id.clone())
                    .or_insert_with(|| TerminalFrameSession::new(&encoding, scrollback_limit));
                let event = session.process_output(
                    session_id,
                    data,
                    encoding,
                    scrollback_limit,
                    action_links_enabled,
                    action_link_matchers,
                );
                let _ = event_tx.send(event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn terminal_frame_action_links_align_with_snapshot_lines() {
        let mut screen = TerminalScreen::default();
        screen.advance_decoded_text("visit http://example.com\nping 10.0.0.1");
        let snapshot = screen.viewport_snapshot(0);
        let matchers = ActionLinksMatcherSettings::default();

        let links = prepare_terminal_frame_action_links(&snapshot, true, &matchers).unwrap();

        assert_eq!(links.matches_by_line.len(), snapshot.lines.len());
        assert!(
            links
                .matches_by_line
                .iter()
                .flatten()
                .any(|item| item.value == "http://example.com")
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
        assert_ne!(links.matcher_key, disabled.matcher_key);
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
