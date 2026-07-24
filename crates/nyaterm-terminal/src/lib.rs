use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::{Config, Term, TermDamage, TermMode};
use alacritty_terminal::vte::ansi;
use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb};

mod cells;
mod encoding;
mod graphics;
mod kitty_payload;
mod sixel;
pub use cells::{
    TerminalTextCell, terminal_byte_index_for_cell_col, terminal_cell_col_for_byte_index,
    terminal_cell_count, terminal_char_cell_width, terminal_is_zero_width_mark,
    terminal_text_cell_slice, terminal_text_cells,
};
pub use encoding::SessionEncoding;

/// OSC 133 shell-integration mark attached to a terminal line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShellCommandMark {
    /// Prompt / input region (`A` / `B`).
    Prompt,
    /// Command output start (`C`).
    Output,
    /// Command finished (`D`), with optional exit status from `D;code`.
    Finished {
        /// Exit status when the shell provided one (`OSC 133;D;code`).
        exit_code: Option<i32>,
    },
}

pub use graphics::{
    GraphicsEvent, GraphicsImageSnapshot, GraphicsIngress, GraphicsPlacement, GraphicsProtocol,
    GraphicsSegment, KittyDeleteMode, TerminalGraphicsState,
};

const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;

/// Cell style carried from Alacritty's terminal grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CellStyle {
    /// Foreground ANSI index 0..=15 when not using truecolor.
    pub fg: Option<u8>,
    /// Background ANSI index 0..=15 when not using truecolor.
    pub bg: Option<u8>,
    /// Truecolor foreground as 0xRRGGBB.
    pub fg_rgb: Option<u32>,
    /// Truecolor background as 0xRRGGBB.
    pub bg_rgb: Option<u32>,
    pub bold: bool,
    pub reverse: bool,
    pub underline: bool,
    pub strikeout: bool,
    pub italic: bool,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledSpan {
    pub text: String,
    pub style: CellStyle,
}

/// Inclusive character range on a snapshot line with an OSC 8 URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HyperlinkSpan {
    pub start_col: usize,
    pub end_col: usize,
    pub uri: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    Block,
    Underline,
    Beam,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorSnapshot {
    pub row: usize,
    pub col: usize,
    pub shape: CursorShape,
    pub visible: bool,
    /// Whether the active cursor style requests blinking (DECSCUSR / DECSET 12).
    pub blinking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderCell {
    pub text: String,
    pub style: CellStyle,
    pub width: u8,
    pub hyperlink: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionSnapshot {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSnapshot {
    pub cols: usize,
    /// Rows in the underlying terminal viewport. `rows` may be larger when a
    /// retained scroll window is attached for smooth painting.
    pub viewport_rows: usize,
    pub rows: usize,
    pub cells: Vec<RenderCell>,
    pub cursor: CursorSnapshot,
    pub selection: Option<SelectionSnapshot>,
    pub lines: Vec<String>,
    pub styled_lines: Vec<Vec<StyledSpan>>,
    /// Stable content/style signature for each viewport row.
    pub line_signatures: Vec<u64>,
    /// Wall-clock stamp (unix ms) for each viewport row, if known.
    pub line_timestamps_ms: Vec<Option<u64>>,
    /// Whether each viewport row continues a wrapped logical line.
    pub line_wrapped: Vec<bool>,
    /// OSC 8 hyperlink spans per viewport line (char columns).
    pub hyperlink_lines: Vec<Vec<HyperlinkSpan>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    /// Rows available above the live screen (scrollback).
    pub scrollback_len: usize,
    /// Total rows in scrollback + live screen.
    pub total_rows: usize,
    /// Alacritty display offset represented by this snapshot.
    pub display_offset: usize,
    /// Inline graphics placements visible in this viewport (Kitty / iTerm2 / Sixel).
    pub images: Vec<GraphicsImageSnapshot>,
    /// OSC 133 command marks for each viewport row (prompt / output / finished).
    pub command_marks: Vec<Option<ShellCommandMark>>,
}

#[derive(Clone, Default)]
pub struct TerminalEffects {
    pub title: Option<String>,
    pub reset_title: bool,
    pub bell: bool,
    pub cwd: Option<String>,
    pub shell_command_started: bool,
    pub shell_command_finished: bool,
    pub pty_write: Vec<Vec<u8>>,
    /// OSC 52 clipboard store requests from the remote (decoded text).
    pub clipboard_store: Option<String>,
    /// OSC 52 clipboard load formatters; host supplies clipboard text.
    pub clipboard_loads: Vec<std::sync::Arc<dyn Fn(&str) -> String + Sync + Send + 'static>>,
}

impl std::fmt::Debug for TerminalEffects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalEffects")
            .field("title", &self.title)
            .field("reset_title", &self.reset_title)
            .field("bell", &self.bell)
            .field("cwd", &self.cwd)
            .field("shell_command_started", &self.shell_command_started)
            .field("shell_command_finished", &self.shell_command_finished)
            .field("pty_write_count", &self.pty_write.len())
            .field("clipboard_store", &self.clipboard_store)
            .field("clipboard_load_count", &self.clipboard_loads.len())
            .finish()
    }
}

#[derive(Clone, Default)]
pub struct NyaTermEventProxy {
    events: Arc<Mutex<Vec<Event>>>,
}

impl NyaTermEventProxy {
    fn drain(&self) -> Vec<Event> {
        self.events
            .lock()
            .map(|mut events| events.drain(..).collect())
            .unwrap_or_default()
    }
}

impl EventListener for NyaTermEventProxy {
    fn send_event(&self, event: Event) {
        if let Ok(mut events) = self.events.lock() {
            events.push(event);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TermSize {
    cols: usize,
    rows: usize,
}

impl Dimensions for TermSize {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

pub struct TerminalCore {
    parser: ansi::Processor,
    term: Term<NyaTermEventProxy>,
    term_config: Config,
    proxy: NyaTermEventProxy,
    sidecar: NyaTermSidecar,
    pending_effects: TerminalEffects,
    line_timestamps_ms: HashMap<i32, u64>,
    line_signatures: HashMap<i32, u64>,
    signature_damage_lines: Vec<i32>,
    /// Absolute Alacritty line → OSC 133 shell mark.
    command_marks: HashMap<i32, ShellCommandMark>,
    rows: usize,
    cols: usize,
    scrollback_limit: usize,
    /// Host cell pixel size used for CSI 14/16 size replies.
    cell_width_px: u16,
    cell_height_px: u16,
    graphics_ingress: GraphicsIngress,
    graphics: TerminalGraphicsState,
    /// Charset for session I/O (UTF-8 / GBK / …). Graphics stay on raw bytes.
    session_encoding: SessionEncoding,
    #[cfg(test)]
    last_signature_scan_count: usize,
}

pub type TerminalScreen = TerminalCore;

/// Stateful text decoder for session output consumers that need the same
/// charset/graphics boundary behavior as [`TerminalCore`] without mutating it.
#[derive(Debug)]
pub struct TerminalOutputDecoder {
    graphics_ingress: GraphicsIngress,
    session_encoding: SessionEncoding,
}

impl Default for TerminalOutputDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalOutputDecoder {
    pub fn new() -> Self {
        Self {
            graphics_ingress: GraphicsIngress::new(),
            session_encoding: SessionEncoding::default(),
        }
    }

    pub fn set_encoding(&mut self, label: &str) {
        let next = SessionEncoding::from_label(label);
        if self.session_encoding.label() == next.label() {
            return;
        }
        self.session_encoding = next;
        self.graphics_ingress = GraphicsIngress::new();
    }

    pub fn encoding_label(&self) -> &str {
        self.session_encoding.label()
    }

    pub fn reset_decoder(&mut self) {
        self.graphics_ingress = GraphicsIngress::new();
        self.session_encoding.reset_decoder();
    }

    /// Decode terminal-text segments to text, skipping graphics payloads.
    pub fn decode_output_text(&mut self, bytes: &[u8]) -> String {
        let mut out = String::new();
        for segment in self.graphics_ingress.advance(bytes) {
            if let GraphicsSegment::Terminal(data) = segment {
                out.push_str(&self.session_encoding.decode_output_text(&data));
            }
        }
        out
    }

    /// Decode terminal-text segments to UTF-8 bytes, skipping graphics payloads.
    pub fn decode_output_chunk(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.decode_output_text(bytes).into_bytes()
    }
}

impl Default for TerminalCore {
    fn default() -> Self {
        Self::new(DEFAULT_COLS, DEFAULT_ROWS)
    }
}

impl TerminalCore {
    pub fn new(cols: u16, rows: u16) -> Self {
        let mut config = Config::default();
        config.scrolling_history = 5_000;
        config.kitty_keyboard = true;
        // Allow remote OSC 52 copy and paste; host applies size limits on store.
        config.osc52 = alacritty_terminal::term::Osc52::CopyPaste;
        Self::new_with_config(cols, rows, config)
    }

    fn new_with_config(cols: u16, rows: u16, config: Config) -> Self {
        let cols = usize::from(cols).max(1);
        let rows = usize::from(rows).max(1);
        let proxy = NyaTermEventProxy::default();
        let size = TermSize { cols, rows };
        let scrollback_limit = config.scrolling_history;
        Self {
            parser: ansi::Processor::new(),
            term: Term::new(config.clone(), &size, proxy.clone()),
            term_config: config,
            proxy,
            sidecar: NyaTermSidecar::default(),
            pending_effects: TerminalEffects::default(),
            line_timestamps_ms: HashMap::new(),
            line_signatures: HashMap::new(),
            signature_damage_lines: Vec::with_capacity(rows),
            command_marks: HashMap::new(),
            rows,
            cols,
            scrollback_limit,
            cell_width_px: 9,
            cell_height_px: 18,
            graphics_ingress: GraphicsIngress::new(),
            graphics: TerminalGraphicsState::default(),
            session_encoding: SessionEncoding::default(),
            #[cfg(test)]
            last_signature_scan_count: 0,
        }
    }

    pub fn set_cell_metrics(&mut self, cell_width_px: u16, cell_height_px: u16) {
        self.cell_width_px = cell_width_px.max(1);
        self.cell_height_px = cell_height_px.max(1);
    }

    pub fn cell_metrics(&self) -> (u16, u16) {
        (self.cell_width_px, self.cell_height_px)
    }

    pub fn focus_reporting(&self) -> bool {
        self.term.mode().contains(TermMode::FOCUS_IN_OUT)
    }

    /// CSI sequences for DECSET 1004 focus reporting.
    pub fn encode_focus_report(focused: bool) -> Vec<u8> {
        if focused {
            b"\x1b[I".to_vec()
        } else {
            b"\x1b[O".to_vec()
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = usize::from(cols).max(1);
        self.rows = usize::from(rows).max(1);
        let size = TermSize {
            cols: self.cols,
            rows: self.rows,
        };
        self.term.resize(size);
        self.refresh_line_signatures();
    }

    pub fn set_scrollback_limit(&mut self, limit: usize) {
        if self.scrollback_limit == limit {
            return;
        }
        self.drain_alacritty_events();
        self.scrollback_limit = limit;
        self.term_config.scrolling_history = limit;
        self.term.set_options(self.term_config.clone());
        // `set_options` re-emits the current title/reset-title as a config update
        // side effect; do not surface that as terminal output state.
        let _ = self.proxy.drain();
        self.retain_line_metadata_range(self.term.topmost_line(), self.term.bottommost_line());
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn scrollback_len(&self) -> usize {
        if self.alternate_screen() {
            0
        } else {
            self.term.grid().history_size().min(self.scrollback_limit)
        }
    }

    pub fn bracketed_paste(&self) -> bool {
        self.term.mode().contains(TermMode::BRACKETED_PASTE)
    }

    pub fn mouse_reporting(&self) -> bool {
        self.term.mode().intersects(TermMode::MOUSE_MODE)
    }

    pub fn mouse_sgr(&self) -> bool {
        self.term.mode().contains(TermMode::SGR_MOUSE)
    }

    pub fn mouse_drag_reporting(&self) -> bool {
        self.term
            .mode()
            .intersects(TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION)
    }

    pub fn mouse_motion_reporting(&self) -> bool {
        self.term.mode().contains(TermMode::MOUSE_MOTION)
    }

    /// DECCKM application cursor keys (DECSET 1).
    pub fn application_cursor_keys(&self) -> bool {
        self.term.mode().contains(TermMode::APP_CURSOR)
    }

    /// DECKPAM application keypad (ESC =).
    pub fn application_keypad(&self) -> bool {
        self.term.mode().contains(TermMode::APP_KEYPAD)
    }

    /// Kitty keyboard protocol disambiguate mode (`CSI = 1 u` / stack modes).
    pub fn kitty_keyboard_disambiguate(&self) -> bool {
        self.term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES)
    }

    /// Kitty keyboard protocol event-type reporting (`CSI = 2 u` bit).
    pub fn kitty_keyboard_report_event_types(&self) -> bool {
        self.term.mode().contains(TermMode::REPORT_EVENT_TYPES)
    }

    /// Kitty keyboard protocol alternate-key reporting (`CSI = 4 u` bit).
    pub fn kitty_keyboard_report_alternate_keys(&self) -> bool {
        self.term.mode().contains(TermMode::REPORT_ALTERNATE_KEYS)
    }

    /// Kitty keyboard protocol "all keys as escape codes" mode (`CSI = 8 u` bit).
    pub fn kitty_keyboard_report_all_keys_as_esc(&self) -> bool {
        self.term.mode().contains(TermMode::REPORT_ALL_KEYS_AS_ESC)
    }

    /// Kitty keyboard protocol associated-text reporting (`CSI = 16 u` bit).
    pub fn kitty_keyboard_report_associated_text(&self) -> bool {
        self.term.mode().contains(TermMode::REPORT_ASSOCIATED_TEXT)
    }

    /// Alternate scroll: wheel sends cursor keys on the alt screen (DECSET 1007).
    pub fn alternate_scroll(&self) -> bool {
        self.term.mode().contains(TermMode::ALTERNATE_SCROLL)
    }

    /// Encode xterm alternate-screen wheel emulation for this terminal state.
    ///
    /// The payload is emitted only when the terminal is in the alternate screen,
    /// DECSET 1007 alternate-scroll is enabled, mouse reporting is off, and the
    /// wheel delta is nonzero. The caller may send this to the PTY/SSH as input.
    pub fn alternate_scroll_payload(&self, delta_lines: i32) -> Option<Vec<u8>> {
        if delta_lines == 0
            || !self.alternate_screen()
            || !self.alternate_scroll()
            || self.mouse_reporting()
        {
            return None;
        }
        let up = delta_lines > 0;
        let unit = alternate_scroll_key_bytes(up, self.application_cursor_keys());
        let steps = delta_lines.unsigned_abs().min(8) as usize;
        let mut payload = Vec::with_capacity(unit.len() * steps);
        for _ in 0..steps {
            payload.extend_from_slice(&unit);
        }
        Some(payload)
    }

    pub fn alternate_screen(&self) -> bool {
        self.term.mode().contains(TermMode::ALT_SCREEN)
    }

    pub fn scroll_region(&self) -> (usize, usize) {
        // Alacritty does not expose the scroll region publicly. Keep a conservative
        // full-screen report for legacy UI diagnostics.
        (0, self.rows.saturating_sub(1))
    }

    pub fn origin_mode(&self) -> bool {
        self.term.mode().contains(TermMode::ORIGIN)
    }

    pub fn take_visual_bell(&mut self) -> bool {
        self.drain_alacritty_events();
        let pending = self.pending_effects.bell;
        self.pending_effects.bell = false;
        pending
    }

    pub fn window_title(&self) -> Option<&str> {
        self.sidecar.window_title.as_deref()
    }

    pub fn take_window_title(&mut self) -> Option<String> {
        self.drain_alacritty_events();
        self.pending_effects.title.take()
    }

    pub fn shell_integration_enabled(&self) -> bool {
        self.sidecar.shell_integration_enabled
    }

    pub fn command_running(&self) -> bool {
        self.sidecar.command_running
    }

    pub fn take_shell_command_edges(&mut self) -> (bool, bool) {
        let started = self.sidecar.pending_command_started;
        let finished = self.sidecar.pending_command_finished;
        self.sidecar.pending_command_started = false;
        self.sidecar.pending_command_finished = false;
        (started, finished)
    }

    pub fn cwd(&self) -> Option<&str> {
        self.sidecar.cwd.as_deref()
    }

    pub fn take_cwd(&mut self) -> Option<String> {
        self.sidecar.pending_cwd.take()
    }

    pub fn take_effects(&mut self) -> TerminalEffects {
        self.drain_alacritty_events();
        let effects = std::mem::take(&mut self.pending_effects);
        self.sidecar.pending_cwd = None;
        self.sidecar.pending_command_started = false;
        self.sidecar.pending_command_finished = false;
        effects
    }

    /// Reset stream parser state after an upstream byte discontinuity.
    ///
    /// This keeps a newly fed tail from being interpreted as the continuation of
    /// a skipped UTF-8/charset, ANSI, OSC, or graphics payload.
    pub fn reset_stream_state(&mut self) {
        self.parser = ansi::Processor::new();
        self.graphics_ingress = GraphicsIngress::new();
        self.session_encoding.reset_decoder();
        self.sidecar.osc = None;
    }

    pub fn total_rows(&self) -> usize {
        self.scrollback_len() + self.rows
    }

    pub fn clear(&mut self) {
        let cols = self.cols as u16;
        let rows = self.rows as u16;
        let mut config = self.term_config.clone();
        config.scrolling_history = self.scrollback_limit;
        let encoding_label = self.session_encoding.label().to_string();
        let cell_metrics = (self.cell_width_px, self.cell_height_px);
        *self = Self::new_with_config(cols, rows, config);
        self.set_encoding(&encoding_label);
        self.set_cell_metrics(cell_metrics.0, cell_metrics.1);
    }

    /// Set session charset used for output decode and input encode.
    /// No-op when the resolved label is unchanged so multi-byte decoder state
    /// survives across output chunks.
    pub fn set_encoding(&mut self, label: &str) {
        let next = SessionEncoding::from_label(label);
        if self.session_encoding.label() == next.label() {
            return;
        }
        self.session_encoding = next;
        self.graphics_ingress = GraphicsIngress::new();
    }

    pub fn encoding_label(&self) -> &str {
        self.session_encoding.label()
    }

    /// Encode UTF-8 / ASCII input bytes for the session wire charset.
    pub fn encode_outgoing(&self, utf8_or_ascii: &[u8]) -> Vec<u8> {
        self.session_encoding.encode_outgoing(utf8_or_ascii)
    }

    pub fn encode_outgoing_str(&self, text: &str) -> Vec<u8> {
        self.session_encoding.encode_str(text)
    }

    pub fn advance(&mut self, bytes: &[u8]) {
        let mut line_history_marker = self.term.grid().history_size();
        let mut graphics_history_marker = line_history_marker;
        let segments = self.graphics_ingress.advance(bytes);
        for segment in segments {
            match segment {
                GraphicsSegment::Terminal(data) => {
                    if data.is_empty() {
                        continue;
                    }
                    // Charset conversion after graphics split, before ANSI/grid.
                    let data = self.session_encoding.decode_output_chunk(&data);
                    if data.is_empty() {
                        continue;
                    }
                    self.sidecar.advance(&data);
                    self.parser.advance(&mut self.term, &data);
                    self.drain_alacritty_events();
                    self.record_shell_command_marks();
                    self.shift_graphics_for_history_delta(&mut graphics_history_marker);
                }
                GraphicsSegment::Event(event) => {
                    if event == GraphicsEvent::ClearScrollback {
                        self.clear_scrollback();
                        let history_size = self.term.grid().history_size();
                        line_history_marker = history_size;
                        graphics_history_marker = history_size;
                        continue;
                    }
                    let point = self.term.renderable_content().cursor.point;
                    let result = self.graphics.handle(
                        event,
                        point.line.0,
                        point.column.0,
                        self.cols,
                        self.cell_width_px,
                        self.cell_height_px,
                    );
                    if let Some(motion) = result.cursor_motion {
                        // Kitty C=1: move past image via relative CSI (CUD + CHA).
                        let ansi = motion.to_ansi();
                        self.sidecar.advance(&ansi);
                        self.parser.advance(&mut self.term, &ansi);
                        self.drain_alacritty_events();
                        self.shift_graphics_for_history_delta(&mut graphics_history_marker);
                    }
                    for reply in result.pty_writes {
                        self.pending_effects.pty_write.push(reply);
                    }
                }
            }
        }
        self.stamp_changed_lines(line_history_marker);
    }

    /// Advance already-decoded UTF-8 terminal text.
    ///
    /// This is for local status/log lines generated by nyaterm itself. Remote
    /// session bytes must use [`Self::advance`] so graphics protocols and the
    /// configured session charset are honored.
    pub fn advance_decoded_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let line_history_marker = self.term.grid().history_size();
        let mut graphics_history_marker = line_history_marker;
        let data = text.as_bytes();
        self.sidecar.advance(data);
        self.parser.advance(&mut self.term, data);
        self.drain_alacritty_events();
        self.record_shell_command_marks();
        self.shift_graphics_for_history_delta(&mut graphics_history_marker);
        self.stamp_changed_lines(line_history_marker);
    }

    fn clear_scrollback(&mut self) {
        const CLEAR_SAVED_LINES: &[u8] = b"\x1b[3J";
        self.sidecar.advance(CLEAR_SAVED_LINES);
        self.parser.advance(&mut self.term, CLEAR_SAVED_LINES);
        self.drain_alacritty_events();
        self.retain_line_metadata_range(self.term.topmost_line(), self.term.bottommost_line());
    }

    fn shift_graphics_for_history_delta(&mut self, history_marker: &mut usize) {
        let current_history = self.term.grid().history_size();
        if current_history > *history_marker {
            self.graphics
                .shift_lines(current_history.saturating_sub(*history_marker));
        }
        *history_marker = current_history;
    }

    pub fn snapshot(&self) -> TerminalSnapshot {
        self.viewport_snapshot(0)
    }

    pub fn viewport_snapshot(&self, offset: usize) -> TerminalSnapshot {
        let max_offset = self.scrollback_len();
        let offset = offset.min(max_offset);
        let mut snapshot = snapshot_from_term(
            &self.term,
            offset,
            &self.line_signatures,
            &self.line_timestamps_ms,
            &self.command_marks,
        );
        snapshot.images = self
            .graphics
            .viewport_images(offset, snapshot.rows, snapshot.cols);
        snapshot
    }

    pub fn viewport_snapshot_with_window(
        &self,
        offset: usize,
        older_rows: usize,
        newer_rows: usize,
    ) -> TerminalSnapshot {
        let scrollback_len = self.scrollback_len();
        let offset = offset.min(scrollback_len);
        let older_rows = older_rows.min(scrollback_len.saturating_sub(offset));
        let newer_rows = newer_rows.min(offset);
        let mut snapshot = snapshot_window_from_term(
            &self.term,
            offset,
            older_rows,
            newer_rows,
            &self.line_signatures,
            &self.line_timestamps_ms,
            &self.command_marks,
        );
        snapshot.images = self
            .graphics
            .viewport_images(offset, self.rows, snapshot.cols)
            .into_iter()
            .map(|mut image| {
                image.row = image.row.saturating_add(older_rows);
                image
            })
            .collect();
        snapshot
    }

    pub fn lines(&self) -> Vec<String> {
        self.snapshot().lines
    }

    pub fn styled_lines(&self) -> Vec<Vec<StyledSpan>> {
        self.snapshot().styled_lines
    }

    pub fn all_lines(&self) -> Vec<String> {
        let max = self.scrollback_len();
        if max == 0 {
            return self.lines();
        }
        let mut out = Vec::new();
        for offset in (0..=max).rev() {
            let snap = self.viewport_snapshot(offset);
            out.extend(snap.lines);
        }
        dedup_overlapping_viewports(out, self.rows)
    }

    pub fn viewport_absolute_range(&self, offset: usize) -> (usize, usize) {
        let total = self.total_rows();
        let max_offset = total.saturating_sub(self.rows);
        let offset = offset.min(max_offset);
        let end = total.saturating_sub(offset);
        let start = end.saturating_sub(self.rows);
        (start, end)
    }

    fn drain_alacritty_events(&mut self) {
        for event in self.proxy.drain() {
            match event {
                Event::Title(title) => {
                    let clipped: String = title.chars().take(120).collect();
                    self.sidecar.window_title = Some(clipped.clone());
                    self.pending_effects.title = Some(clipped);
                }
                Event::ResetTitle => {
                    self.sidecar.window_title = None;
                    self.pending_effects.reset_title = true;
                }
                Event::Bell => {
                    self.pending_effects.bell = true;
                }
                Event::PtyWrite(text) => {
                    self.pending_effects.pty_write.push(text.into_bytes());
                }
                Event::ClipboardStore(_kind, text) => {
                    // Cap remote clipboard payloads to avoid pathological OSC 52 stores.
                    const MAX_OSC52_CHARS: usize = 1_048_576;
                    let clipped: String = text.chars().take(MAX_OSC52_CHARS).collect();
                    self.pending_effects.clipboard_store = Some(clipped);
                }
                Event::ClipboardLoad(_kind, formatter) => {
                    self.pending_effects.clipboard_loads.push(formatter);
                }
                Event::ColorRequest(index, formatter) => {
                    let color = self.resolve_color_rgb(index);
                    let reply = formatter(color);
                    if !reply.is_empty() {
                        self.pending_effects.pty_write.push(reply.into_bytes());
                    }
                }
                Event::TextAreaSizeRequest(formatter) => {
                    let window_size = alacritty_terminal::event::WindowSize {
                        num_lines: self.rows as u16,
                        num_cols: self.cols as u16,
                        cell_width: self.cell_width_px,
                        cell_height: self.cell_height_px,
                    };
                    let reply = formatter(window_size);
                    if !reply.is_empty() {
                        self.pending_effects.pty_write.push(reply.into_bytes());
                    }
                }
                Event::CursorBlinkingChange
                | Event::MouseCursorDirty
                | Event::Wakeup
                | Event::Exit
                | Event::ChildExit(_) => {}
            }
        }
        if let Some(cwd) = self.sidecar.pending_cwd.clone() {
            self.pending_effects.cwd = Some(cwd);
        }
        if self.sidecar.pending_command_started {
            self.pending_effects.shell_command_started = true;
        }
        if self.sidecar.pending_command_finished {
            self.pending_effects.shell_command_finished = true;
        }
    }

    fn stamp_changed_lines(&mut self, before_history: usize) {
        let after_history = self.term.grid().history_size();
        if after_history > before_history {
            let delta = after_history - before_history;
            self.shift_line_metadata(delta);
        }

        let now_ms = unix_time_ms();
        let cols = self.term.columns();
        let topmost = self.term.topmost_line();
        let bottommost = self.term.bottommost_line();
        let history_changed = after_history != before_history;
        let mut damaged_lines = std::mem::take(&mut self.signature_damage_lines);
        damaged_lines.clear();
        let full_damage = match self.term.damage() {
            TermDamage::Full => true,
            TermDamage::Partial(lines) => {
                damaged_lines.extend(lines.filter_map(|damage| i32::try_from(damage.line).ok()));
                false
            }
        };
        self.term.reset_damage();
        let scan_start = if after_history > before_history {
            let scrolled = i32::try_from(after_history - before_history).unwrap_or(i32::MAX);
            topmost.0.max(scrolled.saturating_neg())
        } else {
            0
        };
        let scan_end = i32::try_from(self.term.screen_lines())
            .unwrap_or(i32::MAX)
            .saturating_sub(1);
        let full_scan = history_changed || full_damage;
        #[cfg(test)]
        let scanned = if full_scan {
            (scan_start..=scan_end)
                .filter(|line| *line >= topmost.0 && *line <= bottommost.0)
                .count()
        } else {
            damaged_lines
                .iter()
                .filter(|line| **line >= topmost.0 && **line <= bottommost.0)
                .count()
        };
        if full_scan {
            for line_index in scan_start..=scan_end {
                self.stamp_line_signature(line_index, cols, now_ms, topmost, bottommost);
            }
        } else {
            for &line_index in &damaged_lines {
                self.stamp_line_signature(line_index, cols, now_ms, topmost, bottommost);
            }
        }
        #[cfg(test)]
        {
            self.last_signature_scan_count = scanned;
        }
        self.signature_damage_lines = damaged_lines;
        self.retain_line_metadata_range(topmost, bottommost);
    }

    fn stamp_line_signature(
        &mut self,
        line_index: i32,
        cols: usize,
        now_ms: u64,
        topmost: Line,
        bottommost: Line,
    ) {
        let line = Line(line_index);
        if line < topmost || line > bottommost {
            return;
        }
        let signature = line_signature(&self.term, line, cols);
        if self.line_signatures.get(&line_index).copied() != Some(signature)
            || !self.line_timestamps_ms.contains_key(&line_index)
        {
            self.line_timestamps_ms.insert(line_index, now_ms);
        }
        self.line_signatures.insert(line_index, signature);
    }

    fn refresh_line_signatures(&mut self) {
        let cols = self.term.columns();
        let topmost = self.term.topmost_line();
        let bottommost = self.term.bottommost_line();
        for row in 0..self.term.screen_lines() {
            let line = Line(row as i32);
            if line < topmost || line > bottommost {
                continue;
            }
            let signature = line_signature(&self.term, line, cols);
            self.line_signatures.insert(line.0, signature);
        }
        self.retain_line_metadata_range(topmost, bottommost);
    }

    fn shift_line_metadata(&mut self, delta: usize) {
        if delta == 0 {
            return;
        }
        let delta = i32::try_from(delta).unwrap_or(i32::MAX);
        self.line_timestamps_ms = self
            .line_timestamps_ms
            .drain()
            .map(|(line, timestamp)| (line.saturating_sub(delta), timestamp))
            .collect();
        self.line_signatures = self
            .line_signatures
            .drain()
            .map(|(line, signature)| (line.saturating_sub(delta), signature))
            .collect();
        self.command_marks = self
            .command_marks
            .drain()
            .map(|(line, mark)| (line.saturating_sub(delta), mark))
            .collect();
    }

    fn retain_line_metadata_range(&mut self, topmost: Line, bottommost: Line) {
        self.line_timestamps_ms
            .retain(|line, _| *line >= topmost.0 && *line <= bottommost.0);
        self.line_signatures
            .retain(|line, _| *line >= topmost.0 && *line <= bottommost.0);
        self.command_marks
            .retain(|line, _| *line >= topmost.0 && *line <= bottommost.0);
        self.graphics.retain_line_range(topmost.0, bottommost.0);
    }

    fn record_shell_command_marks(&mut self) {
        let marks = self.sidecar.take_fired_shell_marks();
        if marks.is_empty() {
            return;
        }
        let line = self.term.renderable_content().cursor.point.line.0;
        for mark in marks {
            self.command_marks.insert(line, mark);
        }
    }
}

fn snapshot_from_term(
    term: &Term<NyaTermEventProxy>,
    requested_offset: usize,
    line_signatures_by_line: &HashMap<i32, u64>,
    line_timestamps_by_line: &HashMap<i32, u64>,
    command_marks_by_line: &HashMap<i32, ShellCommandMark>,
) -> TerminalSnapshot {
    snapshot_window_from_term(
        term,
        requested_offset,
        0,
        0,
        line_signatures_by_line,
        line_timestamps_by_line,
        command_marks_by_line,
    )
}

fn snapshot_window_from_term(
    term: &Term<NyaTermEventProxy>,
    requested_offset: usize,
    older_rows: usize,
    newer_rows: usize,
    line_signatures_by_line: &HashMap<i32, u64>,
    line_timestamps_by_line: &HashMap<i32, u64>,
    command_marks_by_line: &HashMap<i32, ShellCommandMark>,
) -> TerminalSnapshot {
    let content = term.renderable_content();
    let cols = term.columns();
    let viewport_rows = term.screen_lines();
    let rows = older_rows
        .saturating_add(viewport_rows)
        .saturating_add(newer_rows);
    let display_offset = requested_offset;
    let mut row_cells = vec![Vec::<RenderCell>::with_capacity(cols); rows];
    let mut line_signatures = vec![0; rows];
    let mut line_timestamps_ms = vec![None; rows];
    let mut line_wrapped = vec![false; rows];
    let mut command_marks = vec![None; rows];

    let topmost = term.topmost_line();
    let bottommost = term.bottommost_line();
    for row in 0..rows {
        let line = Line(row as i32 - requested_offset as i32 - older_rows as i32);
        if line < topmost || line > bottommost {
            continue;
        }
        line_signatures[row] = line_signatures_by_line
            .get(&line.0)
            .copied()
            .unwrap_or_else(|| line_signature(term, line, cols));
        line_timestamps_ms[row] = line_timestamps_by_line.get(&line.0).copied();
        command_marks[row] = command_marks_by_line.get(&line.0).copied();
        let previous_line = Line(line.0 - 1);
        line_wrapped[row] = cols > 0
            && previous_line >= topmost
            && term.grid()[previous_line][Column(cols - 1)]
                .flags
                .contains(Flags::WRAPLINE);
        for col in 0..cols {
            let cell = &term.grid()[line][Column(col)];
            let text = cell_text(cell);
            row_cells[row].push(RenderCell {
                text,
                style: cell_style(cell),
                width: render_cell_width(cell),
                hyperlink: cell.hyperlink().map(|link| link.uri().to_string()),
            });
        }
    }

    for row in &mut row_cells {
        while row.len() < cols {
            row.push(RenderCell {
                text: String::new(),
                style: CellStyle::default(),
                width: 1,
                hyperlink: None,
            });
        }
        if row.len() > cols {
            row.truncate(cols);
        }
    }

    let mut lines = Vec::with_capacity(rows);
    let mut styled_lines = Vec::with_capacity(rows);
    let mut hyperlink_lines = Vec::with_capacity(rows);
    for row in &row_cells {
        let mut line = String::with_capacity(cols);
        for cell in row {
            push_render_cell_text(&mut line, cell);
        }
        lines.push(line.trim_end().to_string());
        styled_lines.push(compress_render_row(row));
        hyperlink_lines.push(compress_render_hyperlinks(row));
    }

    let cursor_point = content.cursor.point;
    let cursor_row = if requested_offset == 0 {
        usize::try_from(cursor_point.line.0 + older_rows as i32).unwrap_or(usize::MAX)
    } else {
        usize::MAX
    };
    let cursor_col = cursor_point.column.0;
    let cursor_shape = match content.cursor.shape {
        alacritty_terminal::vte::ansi::CursorShape::Hidden => CursorShape::Hidden,
        alacritty_terminal::vte::ansi::CursorShape::Underline => CursorShape::Underline,
        alacritty_terminal::vte::ansi::CursorShape::Beam => CursorShape::Beam,
        _ => CursorShape::Block,
    };
    let cursor_visible = cursor_shape != CursorShape::Hidden && cursor_row != usize::MAX;
    let cursor_blinking = term.cursor_style().blinking;
    let selection = term
        .selection_to_string()
        .filter(|text| !text.is_empty())
        .map(|text| SelectionSnapshot { text });
    let cells = row_cells.into_iter().flatten().collect::<Vec<_>>();
    let scrollback_len = if content.mode.contains(TermMode::ALT_SCREEN) {
        0
    } else {
        term.grid().history_size()
    };

    TerminalSnapshot {
        cols,
        viewport_rows,
        rows,
        cells,
        cursor: CursorSnapshot {
            row: cursor_row,
            col: cursor_col,
            shape: cursor_shape,
            visible: cursor_visible,
            blinking: cursor_blinking,
        },
        selection,
        lines,
        styled_lines,
        line_signatures,
        line_timestamps_ms: {
            line_timestamps_ms.resize(rows, None);
            line_timestamps_ms
        },
        line_wrapped,
        hyperlink_lines,
        cursor_row,
        cursor_col,
        scrollback_len,
        total_rows: scrollback_len
            .saturating_add(viewport_rows)
            .saturating_add(newer_rows),
        display_offset,
        images: Vec::new(),
        command_marks: {
            command_marks.resize(rows, None);
            command_marks
        },
    }
}

fn line_signature(term: &Term<NyaTermEventProxy>, line: Line, cols: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    for col in 0..cols {
        let cell = &term.grid()[line][Column(col)];
        hash_cell_text(cell, &mut hasher);
        cell_style(cell).hash(&mut hasher);
        render_cell_width(cell).hash(&mut hasher);
        if let Some(link) = cell.hyperlink() {
            Some(link.uri()).hash(&mut hasher);
        } else {
            Option::<&str>::None.hash(&mut hasher);
        }
    }
    hasher.finish()
}

#[cfg(test)]
fn render_row_signature(row: &[RenderCell]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for cell in row {
        cell.text.hash(&mut hasher);
        cell.style.hash(&mut hasher);
        cell.width.hash(&mut hasher);
        cell.hyperlink.hash(&mut hasher);
    }
    hasher.finish()
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn cell_text(cell: &Cell) -> String {
    if cell_text_is_blank(cell) {
        return String::new();
    }
    let mut text = String::new();
    text.push(cell.c);
    if let Some(zerowidth) = cell.zerowidth() {
        text.extend(zerowidth.iter().copied());
    }
    text
}

fn cell_text_is_blank(cell: &Cell) -> bool {
    cell.flags.contains(Flags::WIDE_CHAR_SPACER)
        || cell.flags.contains(Flags::LEADING_WIDE_CHAR_SPACER)
        || (cell.c == ' ' && cell.zerowidth().is_none_or(|chars| chars.is_empty()))
}

fn render_cell_width(cell: &Cell) -> u8 {
    if cell.flags.contains(Flags::WIDE_CHAR_SPACER)
        || cell.flags.contains(Flags::LEADING_WIDE_CHAR_SPACER)
    {
        0
    } else if cell.flags.contains(Flags::WIDE_CHAR) {
        2
    } else {
        1
    }
}

fn hash_cell_text<H: Hasher>(cell: &Cell, hasher: &mut H) {
    if cell_text_is_blank(cell) {
        "".hash(hasher);
    } else if cell.zerowidth().is_none_or(|chars| chars.is_empty()) {
        let mut encoded = [0; 4];
        cell.c.encode_utf8(&mut encoded).hash(hasher);
    } else {
        cell_text(cell).hash(hasher);
    }
}

fn push_render_cell_text(output: &mut String, cell: &RenderCell) {
    if cell.text.is_empty() {
        if cell.width != 0 {
            output.push(' ');
        }
    } else {
        output.push_str(&cell.text);
    }
}

fn cell_style(cell: &Cell) -> CellStyle {
    let flags = cell.flags;
    let mut fg = color_to_style_fg(cell.fg);
    let mut bg = color_to_style_bg(cell.bg);
    if flags.contains(Flags::INVERSE) {
        std::mem::swap(&mut fg, &mut bg);
    }
    CellStyle {
        fg: fg.0,
        fg_rgb: fg.1,
        bg: bg.0,
        bg_rgb: bg.1,
        bold: flags.contains(Flags::BOLD),
        reverse: flags.contains(Flags::INVERSE),
        underline: flags.intersects(Flags::ALL_UNDERLINES),
        strikeout: flags.contains(Flags::STRIKEOUT),
        italic: flags.contains(Flags::ITALIC),
        hidden: flags.contains(Flags::HIDDEN),
    }
}

fn color_to_style_fg(color: Color) -> (Option<u8>, Option<u32>) {
    color_to_style(color)
}

fn color_to_style_bg(color: Color) -> (Option<u8>, Option<u32>) {
    color_to_style(color)
}

fn color_to_style(color: Color) -> (Option<u8>, Option<u32>) {
    match color {
        Color::Named(named) => (named_color_index(named), None),
        Color::Indexed(index) if index <= 15 => (Some(index), None),
        Color::Indexed(index) => (None, Some(indexed_color_rgb(index))),
        Color::Spec(Rgb { r, g, b }) => (
            None,
            Some(((r as u32) << 16) | ((g as u32) << 8) | b as u32),
        ),
    }
}

fn named_color_index(color: NamedColor) -> Option<u8> {
    match color {
        NamedColor::Black => Some(0),
        NamedColor::Red => Some(1),
        NamedColor::Green => Some(2),
        NamedColor::Yellow => Some(3),
        NamedColor::Blue => Some(4),
        NamedColor::Magenta => Some(5),
        NamedColor::Cyan => Some(6),
        NamedColor::White => Some(7),
        NamedColor::BrightBlack => Some(8),
        NamedColor::BrightRed => Some(9),
        NamedColor::BrightGreen => Some(10),
        NamedColor::BrightYellow => Some(11),
        NamedColor::BrightBlue => Some(12),
        NamedColor::BrightMagenta => Some(13),
        NamedColor::BrightCyan => Some(14),
        NamedColor::BrightWhite => Some(15),
        _ => None,
    }
}

impl TerminalCore {
    fn resolve_color_rgb(&self, index: usize) -> Rgb {
        if let Some(color) = self.term.colors()[index] {
            return color;
        }
        default_color_rgb(index)
    }
}

fn default_color_rgb(index: usize) -> Rgb {
    match index {
        0..=15 => ansi16_rgb(index as u8),
        16..=255 => {
            let value = indexed_color_rgb(index as u8);
            Rgb {
                r: ((value >> 16) & 0xff) as u8,
                g: ((value >> 8) & 0xff) as u8,
                b: (value & 0xff) as u8,
            }
        }
        // Foreground / Cursor / BrightForeground
        256 | 258 | 267 => Rgb {
            r: 0xcc,
            g: 0xcc,
            b: 0xcc,
        },
        // Background
        257 => Rgb {
            r: 0x12,
            g: 0x12,
            b: 0x12,
        },
        // DimBlack..DimWhite
        259..=266 => dim_rgb(ansi16_rgb((index - 259) as u8)),
        // DimForeground / DimBackground-ish
        268 => dim_rgb(Rgb {
            r: 0xcc,
            g: 0xcc,
            b: 0xcc,
        }),
        _ => Rgb { r: 0, g: 0, b: 0 },
    }
}

fn ansi16_rgb(index: u8) -> Rgb {
    const TABLE: [(u8, u8, u8); 16] = [
        (0x00, 0x00, 0x00),
        (0xcd, 0x00, 0x00),
        (0x00, 0xcd, 0x00),
        (0xcd, 0xcd, 0x00),
        (0x00, 0x00, 0xee),
        (0xcd, 0x00, 0xcd),
        (0x00, 0xcd, 0xcd),
        (0xe5, 0xe5, 0xe5),
        (0x7f, 0x7f, 0x7f),
        (0xff, 0x00, 0x00),
        (0x00, 0xff, 0x00),
        (0xff, 0xff, 0x00),
        (0x5c, 0x5c, 0xff),
        (0xff, 0x00, 0xff),
        (0x00, 0xff, 0xff),
        (0xff, 0xff, 0xff),
    ];
    let (r, g, b) = TABLE[index as usize % 16];
    Rgb { r, g, b }
}

fn dim_rgb(color: Rgb) -> Rgb {
    Rgb {
        r: color.r / 2,
        g: color.g / 2,
        b: color.b / 2,
    }
}

fn indexed_color_rgb(index: u8) -> u32 {
    if index < 16 {
        return u32::from(index);
    }
    if index >= 232 {
        let gray = 8 + (u32::from(index) - 232) * 10;
        return (gray << 16) | (gray << 8) | gray;
    }
    let idx = u32::from(index) - 16;
    let r = idx / 36;
    let g = (idx / 6) % 6;
    let b = idx % 6;
    let expand = |v: u32| if v == 0 { 0 } else { 55 + v * 40 };
    (expand(r) << 16) | (expand(g) << 8) | expand(b)
}

fn compress_render_row(row: &[RenderCell]) -> Vec<StyledSpan> {
    let mut spans = Vec::new();
    let mut i = 0;
    while i < row.len() {
        let style = row[i].style;
        let mut text = String::new();
        let mut j = i;
        while j < row.len() && row[j].style == style {
            push_render_cell_text(&mut text, &row[j]);
            j += 1;
        }
        spans.push(StyledSpan { text, style });
        i = j;
    }
    if spans.is_empty() {
        spans.push(StyledSpan {
            text: String::new(),
            style: CellStyle::default(),
        });
    }
    while spans
        .last()
        .is_some_and(|span| span.text.trim_end().is_empty() && spans.len() > 1)
    {
        spans.pop();
    }
    spans
}

fn compress_render_hyperlinks(row: &[RenderCell]) -> Vec<HyperlinkSpan> {
    let mut spans = Vec::new();
    let mut i = 0;
    while i < row.len() {
        let Some(uri) = row[i].hyperlink.clone() else {
            i += 1;
            continue;
        };
        let start = i;
        let mut end = i;
        while end + 1 < row.len() && row[end + 1].hyperlink.as_deref() == Some(uri.as_str()) {
            end += 1;
        }
        spans.push(HyperlinkSpan {
            start_col: start,
            end_col: end,
            uri,
        });
        i = end + 1;
    }
    spans
}

fn dedup_overlapping_viewports(lines: Vec<String>, rows: usize) -> Vec<String> {
    if rows == 0 || lines.len() <= rows {
        return lines;
    }
    let mut out = Vec::new();
    for chunk in lines.chunks(rows) {
        if out.is_empty() {
            out.extend_from_slice(chunk);
        } else if let Some(line) = chunk.first() {
            if out.last() != Some(line) {
                out.push(line.clone());
            }
        }
    }
    out
}

#[derive(Default)]
struct NyaTermSidecar {
    osc: Option<Vec<u8>>,
    window_title: Option<String>,
    cwd: Option<String>,
    pending_cwd: Option<String>,
    shell_integration_enabled: bool,
    command_running: bool,
    pending_command_started: bool,
    pending_command_finished: bool,
    /// OSC 133 marks observed in the current advance chunk (in order).
    fired_shell_marks: Vec<ShellCommandMark>,
}

impl NyaTermSidecar {
    fn advance(&mut self, bytes: &[u8]) {
        let mut i = 0;
        while i < bytes.len() {
            let byte = bytes[i];
            if let Some(buffer) = self.osc.as_mut() {
                if byte == 0x07 {
                    let payload = std::mem::take(buffer);
                    self.osc = None;
                    self.handle_osc(&payload);
                } else if byte == 0x1b && bytes.get(i + 1) == Some(&b'\\') {
                    let payload = std::mem::take(buffer);
                    self.osc = None;
                    self.handle_osc(&payload);
                    i += 1;
                } else {
                    buffer.push(byte);
                    if buffer.len() > 4096 {
                        self.osc = None;
                    }
                }
            } else if byte == 0x1b && bytes.get(i + 1) == Some(&b']') {
                self.osc = Some(Vec::new());
                i += 1;
            }
            i += 1;
        }
    }

    fn handle_osc(&mut self, payload: &[u8]) {
        let text = String::from_utf8_lossy(payload);
        let mut parts = text.split(';');
        let code = parts.next().unwrap_or("").trim();
        match code {
            "0" | "2" => {
                let title = parts.collect::<Vec<_>>().join(";").trim().to_string();
                if !title.is_empty() {
                    let clipped: String = title.chars().take(120).collect();
                    self.window_title = Some(clipped);
                }
            }
            "7" => {
                let payload = parts.collect::<Vec<_>>().join(";");
                if let Some(path) = parse_osc7_path(payload.trim()) {
                    self.cwd = Some(path.clone());
                    self.pending_cwd = Some(path);
                }
            }
            "133" => {
                let mark = parts
                    .next()
                    .and_then(|part| part.chars().next())
                    .unwrap_or('\0');
                let status = parts.next().and_then(|s| s.trim().parse::<i32>().ok());
                self.handle_osc133_mark(mark, status);
            }
            _ if code.starts_with("133") => {
                let mark = code.chars().nth(3).unwrap_or('\0');
                let status = parts.next().and_then(|s| s.trim().parse::<i32>().ok());
                self.handle_osc133_mark(mark, status);
            }
            _ => {}
        }
    }

    fn handle_osc133_mark(&mut self, mark: char, exit_code: Option<i32>) {
        match mark {
            'A' | 'B' => {
                self.shell_integration_enabled = true;
                if mark == 'B' {
                    self.command_running = false;
                }
                self.fired_shell_marks.push(ShellCommandMark::Prompt);
            }
            'C' => {
                self.shell_integration_enabled = true;
                self.command_running = true;
                self.pending_command_started = true;
                self.fired_shell_marks.push(ShellCommandMark::Output);
            }
            'D' => {
                self.shell_integration_enabled = true;
                self.command_running = false;
                self.pending_command_finished = true;
                self.fired_shell_marks
                    .push(ShellCommandMark::Finished { exit_code });
            }
            _ => {}
        }
    }

    fn take_fired_shell_marks(&mut self) -> Vec<ShellCommandMark> {
        std::mem::take(&mut self.fired_shell_marks)
    }
}

fn parse_osc7_path(payload: &str) -> Option<String> {
    let rest = payload.strip_prefix("file://")?;
    let slash = rest.find('/')?;
    let path = &rest[slash..];
    if path.is_empty() {
        None
    } else {
        Some(percent_decode_path(path))
    }
}

fn percent_decode_path(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
        {
            out.push((hi << 4) | lo);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn hex_val(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub fn encode_mouse_report(
    screen: &TerminalScreen,
    button: u8,
    col: u16,
    row: u16,
    press: bool,
) -> Vec<u8> {
    encode_mouse_report_with_modifiers(screen, button, col, row, press, false, false, false, false)
}

#[allow(clippy::too_many_arguments)]
pub fn encode_mouse_report_with_modifiers(
    screen: &TerminalScreen,
    button: u8,
    col: u16,
    row: u16,
    press: bool,
    motion: bool,
    shift: bool,
    alt: bool,
    ctrl: bool,
) -> Vec<u8> {
    if !screen.mouse_reporting() {
        return Vec::new();
    }
    let x = col.saturating_add(1);
    let y = row.saturating_add(1);
    // SGR (1006) keeps the real button on release and uses M/m as the press bit.
    // Legacy X10-style encodings always report button 3 for release.
    let mut code = if press || screen.mouse_sgr() {
        button
    } else {
        3
    };
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
    if screen.mouse_sgr() {
        let suffix = if press { 'M' } else { 'm' };
        format!("\x1b[<{code};{x};{y}{suffix}").into_bytes()
    } else {
        let cb = 32u16.saturating_add(u16::from(code)).min(255) as u8;
        let cx = 32u16.saturating_add(x).min(255) as u8;
        let cy = 32u16.saturating_add(y).min(255) as u8;
        vec![0x1b, b'[', b'M', cb, cx, cy]
    }
}

/// Encode plain Up/Down for alternate-screen mouse wheel emulation.
pub fn alternate_scroll_key_bytes(up: bool, application_cursor: bool) -> Vec<u8> {
    match (up, application_cursor) {
        (true, true) => b"\x1bOA".to_vec(),
        (true, false) => b"\x1b[A".to_vec(),
        (false, true) => b"\x1bOB".to_vec(),
        (false, false) => b"\x1b[B".to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc7_sets_cwd() {
        let mut screen = TerminalScreen::new(40, 3);
        screen.advance(b"\x1b]7;file://host/home/user/proj\x07");
        assert_eq!(screen.take_cwd().as_deref(), Some("/home/user/proj"));
        assert_eq!(screen.cwd(), Some("/home/user/proj"));
        assert!(screen.take_cwd().is_none());
    }

    #[test]
    fn take_effects_consumes_cwd_edge() {
        let mut screen = TerminalScreen::new(40, 3);
        screen.advance(b"\x1b]7;file://host/home/user/proj\x07");

        let effects = screen.take_effects();
        assert_eq!(effects.cwd.as_deref(), Some("/home/user/proj"));
        assert!(screen.take_effects().cwd.is_none());
        assert!(screen.take_cwd().is_none());
        assert_eq!(screen.cwd(), Some("/home/user/proj"));
    }

    #[test]
    fn osc133_shell_integration_marks() {
        let mut screen = TerminalScreen::new(40, 3);
        assert!(!screen.shell_integration_enabled());
        screen.advance(b"\x1b]133;A\x07");
        assert!(screen.shell_integration_enabled());
        screen.advance(b"\x1b]133;C\x07");
        assert!(screen.command_running());
        let (started, finished) = screen.take_shell_command_edges();
        assert!(started);
        assert!(!finished);
        screen.advance(b"\x1b]133;D;0\x07");
        assert!(!screen.command_running());
        let (started, finished) = screen.take_shell_command_edges();
        assert!(!started);
        assert!(finished);
    }

    #[test]
    fn take_effects_consumes_shell_command_edges() {
        let mut screen = TerminalScreen::new(40, 3);
        screen.advance(b"\x1b]133;C\x07");

        let effects = screen.take_effects();
        assert!(effects.shell_command_started);
        assert!(!effects.shell_command_finished);
        assert_eq!(screen.take_shell_command_edges(), (false, false));

        let effects = screen.take_effects();
        assert!(!effects.shell_command_started);
        assert!(!effects.shell_command_finished);

        screen.advance(b"\x1b]133;D;0\x07");
        let effects = screen.take_effects();
        assert!(!effects.shell_command_started);
        assert!(effects.shell_command_finished);
        assert_eq!(screen.take_shell_command_edges(), (false, false));
    }

    #[test]
    fn command_marks_appear_in_snapshot() {
        let mut screen = TerminalScreen::new(40, 8);
        screen.advance(b"prompt\x1b]133;A\x07");
        screen.advance(b"\x1b]133;C\x07out\n");
        screen.advance(b"\x1b]133;D;0\x07");
        let snap = screen.snapshot();
        assert!(
            snap.command_marks.iter().any(|m| {
                matches!(
                    m,
                    Some(
                        ShellCommandMark::Prompt
                            | ShellCommandMark::Output
                            | ShellCommandMark::Finished { .. }
                    )
                )
            }),
            "marks={:?}",
            snap.command_marks
        );
        assert!(
            snap.command_marks
                .iter()
                .any(|m| { matches!(m, Some(ShellCommandMark::Finished { exit_code: Some(0) })) }),
            "expected Finished with exit 0, marks={:?}",
            snap.command_marks
        );
    }

    #[test]
    fn command_mark_finished_carries_exit_code() {
        let mut screen = TerminalScreen::new(40, 6);
        screen.advance(b"\x1b]133;D;1\x07");
        let snap = screen.snapshot();
        assert!(
            snap.command_marks
                .iter()
                .any(|m| { matches!(m, Some(ShellCommandMark::Finished { exit_code: Some(1) })) }),
            "marks={:?}",
            snap.command_marks
        );
        screen.advance(b"\x1b]133;D;0\x07");
        let snap = screen.snapshot();
        assert!(
            snap.command_marks
                .iter()
                .any(|m| { matches!(m, Some(ShellCommandMark::Finished { exit_code: Some(0) })) }),
            "marks={:?}",
            snap.command_marks
        );
    }

    #[test]
    fn osc8_hyperlink_spans() {
        let mut screen = TerminalScreen::new(40, 3);
        screen.advance(b"\x1b]8;;https://example.com\x07click\x1b]8;;\x07 plain");
        let snap = screen.viewport_snapshot(0);
        let spans = &snap.hyperlink_lines[0];
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].uri, "https://example.com");
        assert_eq!(spans[0].start_col, 0);
        assert_eq!(spans[0].end_col, 4);
    }

    #[test]
    fn osc_sets_window_title() {
        let mut screen = TerminalScreen::new(20, 5);
        screen.advance(b"\x1b]2;hello-host\x07");
        assert_eq!(screen.take_window_title().as_deref(), Some("hello-host"));
        assert_eq!(screen.window_title(), Some("hello-host"));
        assert!(screen.take_window_title().is_none());
    }

    #[test]
    fn visual_bell_on_bel() {
        let mut screen = TerminalScreen::new(20, 5);
        assert!(!screen.take_visual_bell());
        screen.advance(b"hi\x07");
        assert!(screen.take_visual_bell());
        assert!(!screen.take_visual_bell());
    }

    #[test]
    fn device_status_query_emits_pty_write_response() {
        let mut screen = TerminalScreen::new(20, 5);
        screen.advance(b"\x1b[5n");
        let effects = screen.take_effects();
        assert_eq!(effects.pty_write, vec![b"\x1b[0n".to_vec()]);
    }

    #[test]
    fn prints_and_wraps_lines() {
        let mut screen = TerminalScreen::new(5, 3);
        screen.advance(b"hello\nworld");
        assert_eq!(screen.lines()[0], "hello");
        assert!(screen.lines().iter().any(|line| line.contains("world")));
    }

    #[test]
    fn snapshots_mark_wrapped_continuation_rows() {
        let mut screen = TerminalScreen::new(5, 3);
        screen.advance(b"abcdef");
        let snapshot = screen.viewport_snapshot(0);

        assert_eq!(snapshot.line_wrapped.first().copied(), Some(false));
        assert_eq!(snapshot.line_wrapped.get(1).copied(), Some(true));
    }

    #[test]
    fn changed_visible_lines_receive_timestamps() {
        let mut screen = TerminalScreen::new(20, 3);
        screen.advance(b"alpha\nbeta");
        let snap = screen.viewport_snapshot(0);

        assert!(
            snap.line_timestamps_ms
                .iter()
                .zip(snap.lines.iter())
                .any(|(timestamp, line)| timestamp.is_some() && line.contains("alpha"))
        );
        assert!(
            snap.line_timestamps_ms
                .iter()
                .zip(snap.lines.iter())
                .any(|(timestamp, line)| timestamp.is_some() && line.contains("beta"))
        );
    }

    #[test]
    fn snapshot_includes_row_signatures() {
        let mut screen = TerminalScreen::new(20, 3);
        screen.advance(b"alpha\nbeta");
        let snap = screen.viewport_snapshot(0);

        assert_eq!(snap.line_signatures.len(), snap.rows);
        assert!(snap.line_signatures.iter().any(|signature| *signature != 0));
        for (signature, cells) in snap
            .line_signatures
            .iter()
            .zip(snap.cells.chunks_exact(snap.cols))
        {
            assert_eq!(*signature, render_row_signature(cells));
        }
    }

    #[test]
    fn snapshot_keeps_blank_cell_storage_allocation_free() {
        let screen = TerminalScreen::new(80, 24);
        let snapshot = screen.viewport_snapshot(0);

        assert!(snapshot.cells.iter().all(|cell| cell.text.is_empty()));
        assert!(snapshot.lines.iter().all(String::is_empty));
        assert!(snapshot.styled_lines.iter().all(|spans| {
            spans
                .iter()
                .map(|span| span.text.as_str())
                .collect::<String>()
                == " ".repeat(snapshot.cols)
        }));
    }

    #[test]
    fn snapshot_blank_cell_storage_preserves_wide_text_and_signatures() {
        let mut screen = TerminalScreen::new(8, 2);
        screen.advance("界 a".as_bytes());
        let snapshot = screen.viewport_snapshot(0);

        assert_eq!(snapshot.lines[0], "界 a");
        assert!(snapshot.cells[1].text.is_empty());
        for (signature, cells) in snapshot
            .line_signatures
            .iter()
            .zip(snapshot.cells.chunks_exact(snapshot.cols))
        {
            assert_eq!(*signature, render_row_signature(cells));
        }
    }

    #[test]
    fn row_signatures_change_with_content_and_style() {
        let mut screen = TerminalScreen::new(20, 2);
        let initial = screen.viewport_snapshot(0).line_signatures[0];

        screen.advance(b"alpha");
        let text_signature = screen.viewport_snapshot(0).line_signatures[0];
        assert_ne!(text_signature, initial);

        screen.clear();
        screen.advance(b"\x1b[31malpha");
        let styled_signature = screen.viewport_snapshot(0).line_signatures[0];
        assert_ne!(styled_signature, text_signature);
    }

    #[test]
    fn consecutive_input_scans_only_alacritty_damaged_lines() {
        let mut screen = TerminalScreen::new(80, 120);
        screen.advance(b"a");

        // Reapplying the worker's unchanged output configuration must not turn
        // the next single-cell update into full terminal damage.
        screen.set_scrollback_limit(5_000);
        screen.advance(b"b");

        assert!(screen.last_signature_scan_count > 0);
        assert!(
            screen.last_signature_scan_count < screen.rows(),
            "single-line input scanned {} of {} rows",
            screen.last_signature_scan_count,
            screen.rows()
        );
    }

    #[test]
    fn scrolled_lines_keep_timestamps_in_history_viewport() {
        let mut screen = TerminalScreen::new(20, 2);
        screen.advance(b"one\r\ntwo\r\nthree");
        assert!(screen.scrollback_len() > 0);

        let snap = screen.viewport_snapshot(1);
        assert!(
            snap.line_timestamps_ms
                .iter()
                .zip(snap.lines.iter())
                .any(|(timestamp, line)| timestamp.is_some() && line.contains("one"))
        );
    }

    #[test]
    fn window_snapshot_matches_adjacent_viewports() {
        let mut screen = TerminalScreen::new(20, 3);
        for line in 0..12 {
            screen.advance(format!("line-{line:02}\r\n").as_bytes());
        }
        let offset = 3;
        let older_rows = 2;
        let newer_rows = 2;
        let window = screen.viewport_snapshot_with_window(offset, older_rows, newer_rows);
        let base = screen.viewport_snapshot(offset);
        let older = screen.viewport_snapshot(offset + older_rows);
        let newer = screen.viewport_snapshot(offset - newer_rows);

        assert_eq!(window.viewport_rows, base.rows);
        assert_eq!(window.rows, base.rows + older_rows + newer_rows);
        assert_eq!(
            &window.lines[older_rows..older_rows + base.rows],
            base.lines.as_slice()
        );
        assert_eq!(&window.lines[..older_rows], &older.lines[..older_rows]);
        assert_eq!(
            &window.lines[older_rows + base.rows..],
            &newer.lines[base.rows - newer_rows..]
        );
    }

    #[test]
    fn live_window_snapshot_offsets_cursor_by_prepended_rows() {
        let mut screen = TerminalScreen::new(20, 3);
        for line in 0..8 {
            screen.advance(format!("line-{line:02}\r\n").as_bytes());
        }
        let base = screen.viewport_snapshot(0);
        let window = screen.viewport_snapshot_with_window(0, 4, 4);

        assert_eq!(window.rows, base.rows + 4);
        assert_eq!(window.cursor_row, base.cursor_row + 4);
        assert_eq!(window.cursor.row, base.cursor.row + 4);
        assert_eq!(window.total_rows, base.total_rows);
    }

    #[test]
    fn scrollback_limit_updates_terminal_history() {
        let mut screen = TerminalScreen::new(20, 2);
        screen.advance(b"one\r\ntwo\r\nthree\r\nfour");
        assert!(screen.scrollback_len() > 1);

        screen.set_scrollback_limit(1);
        assert_eq!(screen.scrollback_len(), 1);
        assert_eq!(screen.total_rows(), 3);
    }

    #[test]
    fn iterm2_clear_scrollback_clears_history() {
        let mut screen = TerminalScreen::new(20, 2);
        screen.advance(b"one\r\ntwo\r\nthree");
        assert!(screen.scrollback_len() > 0);

        screen.advance(b"\x1b]1337;ClearScrollback\x07");

        assert_eq!(screen.scrollback_len(), 0);
        assert!(screen.lines().iter().any(|line| line.contains("three")));
    }

    #[test]
    fn clear_scrollback_then_scroll_same_chunk_stamps_history() {
        let mut screen = TerminalScreen::new(20, 2);
        screen.advance(b"one\r\ntwo\r\nthree");
        assert!(screen.scrollback_len() > 0);

        screen.advance(b"\x1b]1337;ClearScrollback\x07\r\nfour");

        assert_eq!(screen.scrollback_len(), 1);
        let snap = screen.viewport_snapshot(1);
        assert!(
            snap.line_timestamps_ms
                .iter()
                .zip(snap.lines.iter())
                .any(|(timestamp, line)| timestamp.is_some() && line.contains("two")),
            "{:?}",
            snap.lines
        );
    }

    #[test]
    fn scrollback_limit_applies_before_output() {
        let mut screen = TerminalScreen::new(20, 2);
        screen.set_scrollback_limit(1);
        screen.advance(b"one\r\ntwo\r\nthree\r\nfour");

        assert_eq!(screen.scrollback_len(), 1);
    }

    #[test]
    fn clear_preserves_scrollback_limit() {
        let mut screen = TerminalScreen::new(20, 2);
        screen.set_scrollback_limit(1);
        screen.advance(b"one\r\ntwo\r\nthree\r\nfour");
        assert_eq!(screen.scrollback_len(), 1);

        screen.clear();
        screen.advance(b"five\r\nsix\r\nseven\r\neight");

        assert_eq!(screen.scrollback_len(), 1);
    }

    #[test]
    fn sgr_truecolor_and_underline() {
        let mut screen = TerminalScreen::new(20, 2);
        screen.advance(b"\x1b[4;38;2;255;128;0mhi\x1b[0m");
        let styled = screen.styled_lines();
        assert_eq!(styled[0][0].text, "hi");
        assert!(styled[0][0].style.underline);
        assert_eq!(styled[0][0].style.fg_rgb, Some(0xff8000));
    }

    #[test]
    fn bracketed_paste_mode_tracks_decset() {
        let mut screen = TerminalScreen::new(20, 2);
        assert!(!screen.bracketed_paste());
        screen.advance(b"\x1b[?2004h");
        assert!(screen.bracketed_paste());
        screen.advance(b"\x1b[?2004l");
        assert!(!screen.bracketed_paste());
    }

    #[test]
    fn osc52_clipboard_store_emits_effect() {
        let mut screen = TerminalScreen::new(20, 2);
        // base64("hello-osc52") == aGVsbG8tb3NjNTI=
        screen.advance(b"\x1b]52;c;aGVsbG8tb3NjNTI=\x07");
        let effects = screen.take_effects();
        assert_eq!(effects.clipboard_store.as_deref(), Some("hello-osc52"));
    }

    #[test]
    fn osc52_clipboard_load_emits_formatter() {
        let mut screen = TerminalScreen::new(20, 2);
        // Query clipboard contents via OSC 52.
        screen.advance(b"\x1b]52;c;?\x07");
        let effects = screen.take_effects();
        assert_eq!(effects.clipboard_loads.len(), 1);
        let reply = (effects.clipboard_loads[0])("payload");
        assert!(reply.starts_with("\x1b]52;c;"));
        // base64("payload") == cGF5bG9hZA==
        assert!(reply.contains("cGF5bG9hZA=="));
    }

    #[test]
    fn text_area_size_request_uses_cell_metrics() {
        let mut screen = TerminalScreen::new(80, 24);
        screen.set_cell_metrics(10, 20);
        // CSI 14 t -> text area size in pixels
        screen.advance(b"\x1b[14t");
        let effects = screen.take_effects();
        assert_eq!(effects.pty_write.len(), 1);
        // height = 24*20 = 480, width = 80*10 = 800
        assert_eq!(effects.pty_write[0], b"\x1b[4;480;800t".to_vec());
    }

    #[test]
    fn color_request_emits_rgb_reply() {
        let mut screen = TerminalScreen::new(20, 2);
        // OSC 10 ? -> query foreground
        screen.advance(b"\x1b]10;?\x07");
        let effects = screen.take_effects();
        assert_eq!(effects.pty_write.len(), 1);
        let reply = String::from_utf8(effects.pty_write[0].clone()).unwrap();
        assert!(reply.starts_with("\x1b]10;rgb:"));
        assert!(reply.contains("cccc"));
    }

    #[test]
    fn focus_reporting_mode_tracks_decset() {
        let mut screen = TerminalScreen::new(20, 2);
        assert!(!screen.focus_reporting());
        screen.advance(b"\x1b[?1004h");
        assert!(screen.focus_reporting());
        assert_eq!(
            TerminalScreen::encode_focus_report(true),
            b"\x1b[I".to_vec()
        );
        assert_eq!(
            TerminalScreen::encode_focus_report(false),
            b"\x1b[O".to_vec()
        );
        screen.advance(b"\x1b[?1004l");
        assert!(!screen.focus_reporting());
    }

    #[test]
    fn mouse_reporting_modes_track_decset() {
        let mut screen = TerminalScreen::new(20, 2);
        assert!(!screen.mouse_reporting());
        assert!(!screen.mouse_sgr());
        assert!(!screen.mouse_drag_reporting());
        assert!(!screen.mouse_motion_reporting());
        screen.advance(b"\x1b[?1000h");
        screen.advance(b"\x1b[?1006h");
        assert!(screen.mouse_reporting());
        assert!(screen.mouse_sgr());
        assert!(!screen.mouse_drag_reporting());
        screen.advance(b"\x1b[?1002h");
        assert!(screen.mouse_drag_reporting());
        assert!(!screen.mouse_motion_reporting());
        screen.advance(b"\x1b[?1003h");
        assert!(screen.mouse_motion_reporting());
    }

    #[test]
    fn application_cursor_keys_track_decset() {
        let mut screen = TerminalScreen::new(20, 5);
        assert!(!screen.application_cursor_keys());
        screen.advance(b"\x1b[?1h");
        assert!(screen.application_cursor_keys());
        screen.advance(b"\x1b[?1l");
        assert!(!screen.application_cursor_keys());
    }

    #[test]
    fn application_keypad_tracks_deckpam() {
        let mut screen = TerminalScreen::new(40, 3);
        assert!(!screen.application_keypad());
        screen.advance(b"\x1b=");
        assert!(screen.application_keypad());
        screen.advance(b"\x1b>");
        assert!(!screen.application_keypad());
    }

    #[test]
    fn kitty_keyboard_disambiguate_mode_tracks_csi_u() {
        let mut screen = TerminalScreen::new(40, 3);
        assert!(!screen.kitty_keyboard_disambiguate());
        assert!(!screen.kitty_keyboard_report_event_types());
        assert!(!screen.kitty_keyboard_report_alternate_keys());
        assert!(!screen.kitty_keyboard_report_all_keys_as_esc());
        assert!(!screen.kitty_keyboard_report_associated_text());
        screen.advance(b"\x1b[=1u");
        assert!(screen.kitty_keyboard_disambiguate());
        screen.advance(b"\x1b[=31u");
        assert!(screen.kitty_keyboard_disambiguate());
        assert!(screen.kitty_keyboard_report_event_types());
        assert!(screen.kitty_keyboard_report_alternate_keys());
        assert!(screen.kitty_keyboard_report_all_keys_as_esc());
        assert!(screen.kitty_keyboard_report_associated_text());
        screen.advance(b"\x1b[=0u");
        assert!(!screen.kitty_keyboard_disambiguate());
        assert!(!screen.kitty_keyboard_report_event_types());
        assert!(!screen.kitty_keyboard_report_alternate_keys());
        assert!(!screen.kitty_keyboard_report_all_keys_as_esc());
        assert!(!screen.kitty_keyboard_report_associated_text());
    }

    #[test]
    fn cursor_shape_and_visibility_follow_decscusr() {
        let mut screen = TerminalScreen::new(40, 3);
        screen.advance(b"hi");
        let snap = screen.snapshot();
        assert!(snap.cursor.visible);
        assert_eq!(snap.cursor.shape, CursorShape::Block);

        // DECSCUSR 3 = blinking underline; 4 = steady underline; 6 = steady bar; 0/1 = block.
        screen.advance(b"\x1b[3 q");
        let snap = screen.snapshot();
        assert_eq!(snap.cursor.shape, CursorShape::Underline);
        assert!(snap.cursor.blinking);
        assert!(snap.cursor.visible);

        screen.advance(b"\x1b[6 q");
        let snap = screen.snapshot();
        assert_eq!(snap.cursor.shape, CursorShape::Beam);
        assert!(!snap.cursor.blinking);

        // DECTCEM hide cursor (CSI ?25l).
        screen.advance(b"\x1b[?25l");
        let snap = screen.snapshot();
        assert!(!snap.cursor.visible);
        assert_eq!(snap.cursor.shape, CursorShape::Hidden);

        screen.advance(b"\x1b[?25h");
        let snap = screen.snapshot();
        assert!(snap.cursor.visible);
    }

    #[test]
    fn alternate_scroll_defaults_on_and_tracks_decset() {
        let mut screen = TerminalScreen::new(20, 5);
        // Alacritty enables ALTERNATE_SCROLL by default.
        assert!(screen.alternate_scroll());
        screen.advance(b"\x1b[?1007l");
        assert!(!screen.alternate_scroll());
        screen.advance(b"\x1b[?1007h");
        assert!(screen.alternate_scroll());
    }

    #[test]
    fn alternate_scroll_payload_requires_qualified_terminal_state() {
        let mut screen = TerminalScreen::new(20, 5);
        assert!(screen.alternate_scroll());
        assert_eq!(screen.alternate_scroll_payload(1), None);

        screen.advance(b"\x1b[?1049h");
        assert_eq!(screen.alternate_scroll_payload(0), None);
        assert_eq!(
            screen.alternate_scroll_payload(2),
            Some(b"\x1b[A\x1b[A".to_vec())
        );
        assert_eq!(
            screen.alternate_scroll_payload(-1),
            Some(b"\x1b[B".to_vec())
        );

        screen.advance(b"\x1b[?1h");
        assert_eq!(screen.alternate_scroll_payload(1), Some(b"\x1bOA".to_vec()));
        assert_eq!(
            screen.alternate_scroll_payload(-1),
            Some(b"\x1bOB".to_vec())
        );

        let capped = screen.alternate_scroll_payload(20).unwrap();
        assert_eq!(capped, b"\x1bOA".repeat(8));

        screen.advance(b"\x1b[?1000h");
        assert_eq!(screen.alternate_scroll_payload(1), None);
        screen.advance(b"\x1b[?1000l");
        screen.advance(b"\x1b[?1007l");
        assert_eq!(screen.alternate_scroll_payload(1), None);
    }

    #[test]
    fn alternate_scroll_key_bytes_respect_cursor_mode() {
        assert_eq!(alternate_scroll_key_bytes(true, false), b"\x1b[A".to_vec());
        assert_eq!(alternate_scroll_key_bytes(false, true), b"\x1bOB".to_vec());
    }

    #[test]
    fn encode_mouse_report_sgr_and_legacy() {
        let mut screen = TerminalScreen::new(80, 24);
        assert!(encode_mouse_report(&screen, 0, 0, 0, true).is_empty());
        screen.advance(b"\x1b[?1000h");
        let legacy = encode_mouse_report(&screen, 0, 0, 0, true);
        assert_eq!(legacy, vec![0x1b, b'[', b'M', 32, 33, 33]);
        screen.advance(b"\x1b[?1006h");
        let sgr = encode_mouse_report(&screen, 0, 1, 2, true);
        assert_eq!(sgr, b"\x1b[<0;2;3M".to_vec());
    }

    #[test]
    fn encode_mouse_report_release_motion_and_modifiers() {
        let mut screen = TerminalScreen::new(80, 24);
        screen.advance(b"\x1b[?1000h");
        let legacy_release = encode_mouse_report(&screen, 0, 0, 0, false);
        assert_eq!(legacy_release, vec![0x1b, b'[', b'M', 35, 33, 33]);

        screen.advance(b"\x1b[?1006h");
        // SGR release reports the button that was released (0), not legacy code 3.
        let sgr_release = encode_mouse_report(&screen, 0, 3, 4, false);
        assert_eq!(sgr_release, b"\x1b[<0;4;5m".to_vec());
        let sgr_right_release = encode_mouse_report(&screen, 2, 1, 1, false);
        assert_eq!(sgr_right_release, b"\x1b[<2;2;2m".to_vec());

        let modified_motion =
            encode_mouse_report_with_modifiers(&screen, 0, 1, 2, true, true, true, true, true);
        assert_eq!(modified_motion, b"\x1b[<60;2;3M".to_vec());

        let any_motion =
            encode_mouse_report_with_modifiers(&screen, 3, 4, 5, true, true, false, false, false);
        assert_eq!(any_motion, b"\x1b[<35;5;6M".to_vec());
    }

    #[test]
    fn graphics_iterm2_does_not_pollute_grid() {
        let mut screen = TerminalScreen::new(40, 8);
        // Minimal "PNG" base64 payload via iTerm2 inline.
        screen.advance(b"pre\x1b]1337;File=name=x.png;width=3;height=2;inline=1:UE5H\x07post");
        let snap = screen.snapshot();
        let joined = snap.lines.join("");
        assert!(joined.contains("pre"), "{joined:?}");
        assert!(joined.contains("post"), "{joined:?}");
        assert!(
            !joined.contains("1337") && !joined.contains("File="),
            "graphics payload leaked into grid: {joined:?}"
        );
        assert_eq!(snap.images.len(), 1);
        assert_eq!(snap.images[0].width_cells, 3);
        assert_eq!(snap.images[0].height_cells, 2);
        assert_eq!(snap.images[0].protocol, GraphicsProtocol::ITerm2);
        assert_eq!(snap.images[0].data, b"PNG");
    }

    #[test]
    fn graphics_iterm2_file_without_inline_does_not_place_image() {
        let mut screen = TerminalScreen::new(40, 8);
        screen.advance(b"pre\x1b]1337;File=name=x.png;width=3;height=2:UE5H\x07post");
        let snap = screen.snapshot();
        let joined = snap.lines.join("");
        assert!(joined.contains("pre"), "{joined:?}");
        assert!(joined.contains("post"), "{joined:?}");
        assert!(
            !joined.contains("1337") && !joined.contains("File="),
            "download-only OSC 1337 leaked into grid: {joined:?}"
        );
        assert!(snap.images.is_empty());
    }

    #[test]
    fn graphics_kitty_placement_appears_in_snapshot() {
        let mut screen = TerminalScreen::new(40, 8);
        screen.advance(b"\x1b_Ga=T,i=7,c=5,r=3;QUJD\x1b\\");
        let snap = screen.snapshot();
        assert_eq!(snap.images.len(), 1);
        assert_eq!(snap.images[0].protocol, GraphicsProtocol::Kitty);
        assert_eq!(snap.images[0].width_cells, 5);
        assert_eq!(snap.images[0].height_cells, 3);
        assert_eq!(snap.images[0].data, b"ABC");
    }

    #[test]
    fn graphics_delete_clears_kitty_image() {
        let mut screen = TerminalScreen::new(40, 8);
        screen.advance(b"\x1b_Ga=T,i=3,c=2,r=2;QUI=\x1b\\");
        assert_eq!(screen.snapshot().images.len(), 1);
        screen.advance(b"\x1b_Ga=d,i=3\x1b\\");
        assert!(screen.snapshot().images.is_empty());
    }

    #[test]
    fn graphics_kitty_multi_chunk_via_advance() {
        let mut screen = TerminalScreen::new(40, 8);
        // m=1 then m=0 with base64 "AB" + "CD"; a=T places after final chunk.
        // a=t would be store-only (see graphics store/place unit tests).
        screen.advance(b"\x1b_Ga=T,i=11,c=3,r=2,m=1;QUI=\x1b\\");
        assert!(screen.snapshot().images.is_empty());
        screen.advance(b"\x1b_Ga=T,i=11,m=0;Q0Q=\x1b\\");
        let snap = screen.snapshot();
        assert_eq!(snap.images.len(), 1);
        assert_eq!(snap.images[0].data, b"ABCD");
        assert_eq!(snap.images[0].width_cells, 3);
        assert_eq!(snap.images[0].height_cells, 2);
    }

    #[test]
    fn graphics_sixel_via_advance() {
        let mut screen = TerminalScreen::new(40, 8);
        // Solid red sixel column.
        screen.advance(b"\x1bP0;0;0q#0;2;100;0;0#0~\x1b\\");
        let snap = screen.snapshot();
        assert_eq!(snap.images.len(), 1);
        assert_eq!(snap.images[0].protocol, GraphicsProtocol::Sixel);
        assert!(snap.images[0].data.starts_with(b"NYAR"));
        assert!(snap.images[0].width_cells >= 1);
        assert!(snap.images[0].height_cells >= 1);
    }

    #[test]
    fn graphics_kitty_cursor_motion_via_advance() {
        let mut screen = TerminalScreen::new(40, 8);
        // Place 3x2 at origin with C=1; cursor should leave top-left.
        screen.advance(b"\x1b_Ga=T,i=1,c=3,r=2,C=1;QUI=\x1b\\");
        let snap = screen.snapshot();
        assert_eq!(snap.images.len(), 1);
        // After CUD1 + CHA4: row=1, col=3 (0-based).
        assert_eq!(snap.cursor_row, 1);
        assert_eq!(snap.cursor_col, 3);
    }

    #[test]
    fn graphics_after_scroll_in_same_chunk_stays_on_live_screen() {
        let mut screen = TerminalScreen::new(40, 3);
        screen.advance(b"one\r\ntwo\r\nthree\r\n\x1b_Ga=T,i=1,c=1,r=1;QUI=\x1b\\");
        let snap = screen.snapshot();
        assert_eq!(snap.images.len(), 1);
        assert_eq!(
            snap.images[0].row, snap.cursor_row,
            "image placed after scroll should not be shifted into history"
        );
    }

    #[test]
    fn graphics_kitty_rgb24_via_advance() {
        let mut screen = TerminalScreen::new(40, 8);
        // f=24,s=1,v=1 single red RGB pixel (base64 of FF 00 00 = /wAA)
        screen.advance(b"\x1b_Ga=T,i=1,f=24,s=1,v=1,c=1,r=1;/wAA\x1b\\");
        let snap = screen.snapshot();
        assert_eq!(snap.images.len(), 1);
        assert!(snap.images[0].data.starts_with(b"NYAR"));
        assert_eq!(&snap.images[0].data[12..16], &[255, 0, 0, 255]);
    }

    #[test]
    fn graphics_kitty_query_via_advance() {
        let mut screen = TerminalScreen::new(40, 8);
        screen.advance(b"\x1b_Ga=t,i=4,c=1,r=1,q=2;QUI=\x1b\\");
        let effects = screen.take_effects();
        assert_eq!(effects.pty_write.len(), 1);
        assert!(
            String::from_utf8_lossy(&effects.pty_write[0]).contains("OK"),
            "{:?}",
            effects.pty_write
        );
        screen.advance(b"\x1b_Ga=q,i=4\x1b\\");
        let effects = screen.take_effects();
        assert!(
            String::from_utf8_lossy(&effects.pty_write[0]).contains("OK"),
            "{:?}",
            effects.pty_write
        );
        screen.advance(b"\x1b_Ga=q,i=99\x1b\\");
        let effects = screen.take_effects();
        assert!(
            String::from_utf8_lossy(&effects.pty_write[0]).contains("ENOENT"),
            "{:?}",
            effects.pty_write
        );
    }

    #[test]
    fn encoding_gbk_output_decodes_to_grid() {
        let mut screen = TerminalScreen::new(40, 8);
        screen.set_encoding("GBK");
        // GBK "测"
        screen.advance(&[0xb2, 0xe2]);
        let snap = screen.snapshot();
        let joined = snap.lines.join("");
        assert!(joined.contains('测'), "grid={joined:?}");
    }

    #[test]
    fn decoded_local_text_bypasses_session_charset() {
        let mut screen = TerminalScreen::new(40, 8);
        screen.set_encoding("GBK");

        screen.advance_decoded_text("本地提示");

        let joined = screen.snapshot().lines.join("");
        let compact = joined.replace(' ', "");
        assert!(compact.contains("本地提示"), "grid={joined:?}");
        assert!(!joined.contains('\u{fffd}'), "grid={joined:?}");
    }

    #[test]
    fn encoding_gbk_output_decodes_split_multibyte_to_grid() {
        let mut screen = TerminalScreen::new(40, 8);
        screen.set_encoding("GBK");
        // GBK "测试" split in the middle of the first character.
        screen.advance(&[0xb2]);
        assert!(
            !screen.snapshot().lines.join("").contains('\u{fffd}'),
            "incomplete byte should not render as replacement"
        );
        screen.advance(&[0xe2, 0xca, 0xd4]);
        let joined = screen.snapshot().lines.join("");
        assert!(
            joined.contains('测') && joined.contains('试') && !joined.contains('\u{fffd}'),
            "grid={joined:?}"
        );
    }

    #[test]
    fn output_decoder_gbk_decodes_split_multibyte_text() {
        let mut decoder = TerminalOutputDecoder::new();
        decoder.set_encoding("GBK");
        assert!(decoder.decode_output_text(&[0xb2]).is_empty());
        let text = decoder.decode_output_text(&[0xe2, 0xca, 0xd4]);
        assert_eq!(text, "测试");
    }

    #[test]
    fn output_decoder_utf8_decodes_split_multibyte_text() {
        let mut decoder = TerminalOutputDecoder::new();
        let bytes = "测".as_bytes();
        assert!(decoder.decode_output_text(&bytes[..1]).is_empty());
        assert_eq!(decoder.decode_output_text(&bytes[1..]), "测");
    }

    #[test]
    fn output_decoder_skips_graphics_payload() {
        let mut decoder = TerminalOutputDecoder::new();
        decoder.set_encoding("GBK");
        let text = decoder.decode_output_text(b"pre\x1b_Ga=T,i=1,c=1,r=1;QUI=\x1b\\post");
        assert_eq!(text, "prepost");
    }

    #[test]
    fn output_decoder_skips_iterm2_graphics_payload() {
        let mut decoder = TerminalOutputDecoder::new();
        let text = decoder.decode_output_text(
            b"pre\x1b]1337;File=name=x.png;width=4;height=2;inline=1:UE5H\x07post",
        );
        assert_eq!(text, "prepost");
    }

    #[test]
    fn output_decoder_skips_sixel_graphics_payload() {
        let mut decoder = TerminalOutputDecoder::new();
        let text = decoder.decode_output_text(b"pre\x1bP0;0;0q#0;2;100;0;0#0~\x1b\\post");
        assert_eq!(text, "prepost");
    }

    #[test]
    fn output_decoder_encoding_change_drops_pending_multibyte_state() {
        let mut decoder = TerminalOutputDecoder::new();
        decoder.set_encoding("GBK");

        assert!(decoder.decode_output_text(&[0xb2]).is_empty());
        decoder.set_encoding("UTF-8");

        assert_eq!(decoder.decode_output_text(b"ok"), "ok");
    }

    #[test]
    fn terminal_screen_encoding_change_drops_pending_graphics_state() {
        let mut screen = TerminalScreen::new(40, 8);

        screen.advance(b"\x1b_Ga=T,i=1,c=1,r=1;QUI=");
        screen.set_encoding("GBK");
        screen.advance(b"\x1b\\");

        assert!(
            screen.snapshot().images.is_empty(),
            "incomplete graphics should not survive an encoding switch"
        );
    }

    #[test]
    fn encoding_outgoing_reencodes_utf8_text() {
        let mut screen = TerminalScreen::new(40, 8);
        screen.set_encoding("GBK");
        assert_eq!(screen.encode_outgoing_str("测试"), [0xb2, 0xe2, 0xca, 0xd4]);
        assert_eq!(screen.encode_outgoing(b"\x1b[A"), b"\x1b[A");
    }
}
