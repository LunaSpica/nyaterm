use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Boundary, Column, Direction, Line, Point};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::search::{RegexIter, RegexSearch};
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
    GraphicsSegment, GraphicsSegmentRef, KittyDeleteMode, TerminalGraphicsState,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalSearchDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TerminalSearchQuery {
    pub pattern: String,
    pub regex: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub direction: TerminalSearchDirection,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalGridMatch {
    /// Absolute row in scrollback + live screen coordinates.
    pub line_index: usize,
    /// Half-open terminal cell column range for this row.
    pub start_col: usize,
    pub end_col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalSearchError {
    InvalidRegex(String),
}

impl std::fmt::Display for TerminalSearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRegex(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for TerminalSearchError {}

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
pub struct TerminalSnapshotRow {
    pub cells: Box<[RenderCell]>,
    pub text: String,
    pub styled_spans: Box<[StyledSpan]>,
    pub signature: u64,
    pub timestamp_ms: Option<u64>,
    pub wrapped: bool,
    pub hyperlinks: Box<[HyperlinkSpan]>,
    pub command_mark: Option<ShellCommandMark>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSnapshotMeta {
    pub cols: usize,
    pub viewport_rows: usize,
    pub cursor: CursorSnapshot,
    pub selection: Option<SelectionSnapshot>,
    pub scrollback_len: usize,
    pub total_rows: usize,
    pub display_offset: usize,
    pub images: Vec<GraphicsImageSnapshot>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TerminalSnapshotBuildStats {
    pub reused_rows: usize,
    pub rebuilt_rows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSnapshot {
    pub cols: usize,
    pub viewport_rows: usize,
    pub row_data: Arc<[Arc<TerminalSnapshotRow>]>,
    pub cursor: CursorSnapshot,
    pub selection: Option<SelectionSnapshot>,
    /// Rows available above the live screen (scrollback).
    pub scrollback_len: usize,
    /// Total rows in scrollback + live screen.
    pub total_rows: usize,
    /// Alacritty display offset represented by this snapshot.
    pub display_offset: usize,
    /// Inline graphics placements visible in this viewport (Kitty / iTerm2 / Sixel).
    pub images: Vec<GraphicsImageSnapshot>,
}

impl TerminalSnapshot {
    pub fn from_rows(
        meta: TerminalSnapshotMeta,
        rows: impl Into<Arc<[Arc<TerminalSnapshotRow>]>>,
    ) -> Self {
        let row_data = rows.into();
        Self {
            cols: meta.cols,
            viewport_rows: meta.viewport_rows,
            row_data,
            cursor: meta.cursor,
            selection: meta.selection,
            scrollback_len: meta.scrollback_len,
            total_rows: meta.total_rows,
            display_offset: meta.display_offset,
            images: meta.images,
        }
    }

    pub fn row_count(&self) -> usize {
        self.row_data.len()
    }

    pub fn rows(&self) -> &[Arc<TerminalSnapshotRow>] {
        &self.row_data
    }

    pub fn row(&self, row: usize) -> Option<&TerminalSnapshotRow> {
        self.row_data.get(row).map(Arc::as_ref)
    }

    pub fn cell(&self, row: usize, col: usize) -> Option<&RenderCell> {
        self.row(row)?.cells.get(col)
    }

    pub fn line(&self, row: usize) -> Option<&str> {
        self.row(row).map(|row| row.text.as_str())
    }
}

/// Formats a host clipboard value into an OSC 52 protocol response.
pub type TerminalClipboardLoad = std::sync::Arc<dyn Fn(&str) -> String + Sync + Send + 'static>;

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
    pub clipboard_loads: Vec<TerminalClipboardLoad>,
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

const TERMINAL_SNAPSHOT_ROW_CACHE_LIMIT: usize = 8_192;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TerminalSnapshotRowCacheKey {
    cols: usize,
    signature: u64,
    timestamp_ms: Option<u64>,
    wrapped: bool,
    command_mark: Option<ShellCommandMark>,
}

#[derive(Debug)]
struct TerminalSnapshotRowCacheEntry {
    row: Weak<TerminalSnapshotRow>,
    last_used: u64,
}

#[derive(Debug, Default)]
struct TerminalSnapshotRowCache {
    entries: HashMap<TerminalSnapshotRowCacheKey, TerminalSnapshotRowCacheEntry>,
    generation: u64,
}

impl TerminalSnapshotRowCache {
    fn next_generation(&mut self) -> u64 {
        self.generation = self.generation.saturating_add(1);
        self.generation
    }

    fn prune(&mut self) {
        if self.entries.len() <= TERMINAL_SNAPSHOT_ROW_CACHE_LIMIT {
            return;
        }
        self.entries.retain(|_, entry| entry.row.strong_count() > 0);
        if self.entries.len() <= TERMINAL_SNAPSHOT_ROW_CACHE_LIMIT {
            return;
        }
        let mut oldest = self
            .entries
            .iter()
            .map(|(key, entry)| (*key, entry.last_used))
            .collect::<Vec<_>>();
        oldest.sort_unstable_by_key(|(_, last_used)| *last_used);
        let remove = oldest
            .len()
            .saturating_sub(TERMINAL_SNAPSHOT_ROW_CACHE_LIMIT);
        for (key, _) in oldest.into_iter().take(remove) {
            self.entries.remove(&key);
        }
    }
}

const TERMINAL_SEARCH_CACHE_LIMIT: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TerminalSearchCacheKey {
    pattern: String,
    regex: bool,
    case_sensitive: bool,
    whole_word: bool,
}

#[derive(Debug, Default)]
struct TerminalSearchCache {
    entries: HashMap<TerminalSearchCacheKey, RegexSearch>,
}

impl TerminalSearchCache {
    fn regex_for(
        &mut self,
        key: TerminalSearchCacheKey,
    ) -> Result<&mut RegexSearch, TerminalSearchError> {
        if self.entries.len() >= TERMINAL_SEARCH_CACHE_LIMIT && !self.entries.contains_key(&key) {
            if let Some(old_key) = self.entries.keys().next().cloned() {
                self.entries.remove(&old_key);
            }
        }
        if !self.entries.contains_key(&key) {
            let pattern = terminal_search_regex_pattern(&key);
            let regex = RegexSearch::new(&pattern)
                .map_err(|error| TerminalSearchError::InvalidRegex(error.to_string()))?;
            self.entries.insert(key.clone(), regex);
        }
        Ok(self
            .entries
            .get_mut(&key)
            .expect("search cache entry inserted or already present"))
    }
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
    snapshot_row_cache: Mutex<TerminalSnapshotRowCache>,
    search_cache: Mutex<TerminalSearchCache>,
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
        self.graphics_ingress.advance_with(
            bytes,
            |data| out.push_str(&self.session_encoding.decode_output_text(data)),
            |_| {},
        );
        out
    }

    /// Decode terminal-text segments, keeping only the last `max_bytes` of text.
    ///
    /// The whole input still flows through the graphics ingress and the charset
    /// decoder, so their streaming state stays exact — only the *result* is
    /// capped. Callers that want a visible-output tail get to skip
    /// materialising a whole multi-hundred-kilobyte burst just to drain 7/8 of
    /// it back off again.
    pub fn decode_output_text_tail(&mut self, bytes: &[u8], max_bytes: usize) -> String {
        let mut out = String::new();
        self.graphics_ingress.advance_with(
            bytes,
            |data| {
                let decoded = self.session_encoding.decode_output_bytes(data);
                out.push_str(&String::from_utf8_lossy(utf8_tail_bytes(
                    &decoded, max_bytes,
                )));
                truncate_text_to_tail(&mut out, max_bytes);
            },
            |_| {},
        );
        out
    }

    /// Decode terminal-text segments to UTF-8 bytes, skipping graphics payloads.
    pub fn decode_output_chunk(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.decode_output_text(bytes).into_bytes()
    }
}

/// The last `max_bytes` of `text`, snapped forward to a character boundary.
fn utf8_tail_bytes(text: &[u8], max_bytes: usize) -> &[u8] {
    if text.len() <= max_bytes {
        return text;
    }
    let mut start = text.len() - max_bytes;
    // Continuation bytes are 0b10xx_xxxx: walk off the middle of a character.
    while start < text.len() && text[start] & 0xc0 == 0x80 {
        start += 1;
    }
    &text[start..]
}

/// Drop leading characters until `text` fits in `max_bytes`.
fn truncate_text_to_tail(text: &mut String, max_bytes: usize) {
    if text.len() <= max_bytes {
        return;
    }
    let min_start = text.len() - max_bytes;
    let cut = text
        .char_indices()
        .find_map(|(index, _)| (index >= min_start).then_some(index))
        .unwrap_or(text.len());
    text.drain(..cut);
}

impl Default for TerminalCore {
    fn default() -> Self {
        Self::new(DEFAULT_COLS, DEFAULT_ROWS)
    }
}

impl TerminalCore {
    pub fn new(cols: u16, rows: u16) -> Self {
        let config = Config {
            scrolling_history: 5_000,
            kitty_keyboard: true,
            // Allow remote OSC 52 copy and paste; host applies size limits on store.
            osc52: alacritty_terminal::term::Osc52::CopyPaste,
            ..Config::default()
        };
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
            snapshot_row_cache: Mutex::new(TerminalSnapshotRowCache::default()),
            search_cache: Mutex::new(TerminalSearchCache::default()),
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
        let mut graphics_ingress = std::mem::take(&mut self.graphics_ingress);
        graphics_ingress.advance_segments(bytes, |segment| {
            match segment {
                GraphicsSegmentRef::Terminal(data) => {
                    if data.is_empty() {
                        return;
                    }
                    // Charset conversion after graphics split, before ANSI/grid.
                    // Borrows straight through for valid UTF-8, which is the
                    // default charset and so the overwhelming majority.
                    let data = self.session_encoding.decode_output_bytes(data);
                    if data.is_empty() {
                        return;
                    }
                    self.sidecar.advance(&data);
                    self.parser.advance(&mut self.term, &data);
                    self.drain_alacritty_events();
                    self.record_shell_command_marks();
                    self.shift_graphics_for_history_delta(&mut graphics_history_marker);
                }
                GraphicsSegmentRef::Event(event) => {
                    if event == GraphicsEvent::ClearScrollback {
                        self.clear_scrollback();
                        let history_size = self.term.grid().history_size();
                        line_history_marker = history_size;
                        graphics_history_marker = history_size;
                        return;
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
        });
        self.graphics_ingress = graphics_ingress;
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
        self.viewport_snapshot_with_stats(offset).0
    }

    pub fn viewport_snapshot_with_stats(
        &self,
        offset: usize,
    ) -> (TerminalSnapshot, TerminalSnapshotBuildStats) {
        let max_offset = self.scrollback_len();
        let offset = offset.min(max_offset);
        let (mut snapshot, stats) = snapshot_from_term(
            &self.term,
            offset,
            &self.line_signatures,
            &self.line_timestamps_ms,
            &self.command_marks,
            &self.snapshot_row_cache,
        );
        snapshot.images =
            self.graphics
                .viewport_images(offset, snapshot.row_count(), snapshot.cols);
        (snapshot, stats)
    }

    pub fn viewport_snapshot_with_window(
        &self,
        offset: usize,
        older_rows: usize,
        newer_rows: usize,
    ) -> TerminalSnapshot {
        self.viewport_snapshot_with_window_and_stats(offset, older_rows, newer_rows)
            .0
    }

    pub fn viewport_snapshot_with_window_and_stats(
        &self,
        offset: usize,
        older_rows: usize,
        newer_rows: usize,
    ) -> (TerminalSnapshot, TerminalSnapshotBuildStats) {
        let scrollback_len = self.scrollback_len();
        let offset = offset.min(scrollback_len);
        let older_rows = older_rows.min(scrollback_len.saturating_sub(offset));
        let newer_rows = newer_rows.min(offset);
        let (mut snapshot, stats) = snapshot_window_from_term(
            &self.term,
            TerminalSnapshotWindow {
                display_offset: offset,
                older_rows,
                newer_rows,
            },
            &self.line_signatures,
            &self.line_timestamps_ms,
            &self.command_marks,
            &self.snapshot_row_cache,
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
        (snapshot, stats)
    }

    pub fn lines(&self) -> Vec<String> {
        self.snapshot()
            .rows()
            .iter()
            .map(|row| row.text.clone())
            .collect()
    }

    pub fn styled_lines(&self) -> Vec<Vec<StyledSpan>> {
        self.snapshot()
            .rows()
            .iter()
            .map(|row| row.styled_spans.to_vec())
            .collect()
    }

    pub fn all_lines(&self) -> Vec<String> {
        let max = self.scrollback_len();
        if max == 0 {
            return self.lines();
        }
        let mut out = Vec::new();
        for offset in (0..=max).rev() {
            let snap = self.viewport_snapshot(offset);
            out.extend(snap.rows().iter().map(|row| row.text.clone()));
        }
        dedup_overlapping_viewports(out, self.rows)
    }

    pub fn search_grid(
        &self,
        query: &TerminalSearchQuery,
    ) -> Result<Vec<TerminalGridMatch>, TerminalSearchError> {
        if query.pattern.trim().is_empty() || query.limit == 0 {
            return Ok(Vec::new());
        }

        let key = TerminalSearchCacheKey {
            pattern: query.pattern.clone(),
            regex: query.regex,
            case_sensitive: query.case_sensitive,
            whole_word: query.whole_word,
        };
        let mut cache = self
            .search_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let regex = cache.regex_for(key)?;
        let topmost = self.term.topmost_line();
        let bottommost = self.term.bottommost_line();
        if bottommost < topmost {
            return Ok(Vec::new());
        }

        let start = Point::new(topmost, Column(0));
        let end = Point::new(bottommost, self.term.grid().last_column());
        let mut out = Vec::new();
        for found in RegexIter::new(start, end, Direction::Right, &self.term, regex) {
            if query.whole_word && !terminal_grid_match_is_whole_word(&self.term, &found) {
                continue;
            }
            push_terminal_grid_match_segments(
                &self.term,
                found,
                query.limit,
                query.direction,
                &mut out,
            );
            if out.len() >= query.limit {
                break;
            }
        }
        out.sort_unstable_by_key(|m| (m.line_index, m.start_col, m.end_col));
        out.dedup_by_key(|m| (m.line_index, m.start_col, m.end_col));
        if query.direction == TerminalSearchDirection::Backward {
            out.reverse();
        }
        out.truncate(query.limit);
        Ok(out)
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
    row_cache: &Mutex<TerminalSnapshotRowCache>,
) -> (TerminalSnapshot, TerminalSnapshotBuildStats) {
    snapshot_window_from_term(
        term,
        TerminalSnapshotWindow {
            display_offset: requested_offset,
            older_rows: 0,
            newer_rows: 0,
        },
        line_signatures_by_line,
        line_timestamps_by_line,
        command_marks_by_line,
        row_cache,
    )
}

#[derive(Clone, Copy)]
struct TerminalSnapshotWindow {
    display_offset: usize,
    older_rows: usize,
    newer_rows: usize,
}

fn snapshot_window_from_term(
    term: &Term<NyaTermEventProxy>,
    window: TerminalSnapshotWindow,
    line_signatures_by_line: &HashMap<i32, u64>,
    line_timestamps_by_line: &HashMap<i32, u64>,
    command_marks_by_line: &HashMap<i32, ShellCommandMark>,
    row_cache: &Mutex<TerminalSnapshotRowCache>,
) -> (TerminalSnapshot, TerminalSnapshotBuildStats) {
    let content = term.renderable_content();
    let cols = term.columns();
    let viewport_rows = term.screen_lines();
    let rows = window
        .older_rows
        .saturating_add(viewport_rows)
        .saturating_add(window.newer_rows);
    let display_offset = window.display_offset;
    let mut row_data = Vec::with_capacity(rows);
    let mut stats = TerminalSnapshotBuildStats::default();
    let mut row_cache = row_cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let topmost = term.topmost_line();
    let bottommost = term.bottommost_line();
    for row in 0..rows {
        let line = Line(row as i32 - window.display_offset as i32 - window.older_rows as i32);
        let line_in_grid = (line >= topmost && line <= bottommost).then_some(line);
        let signature = line_in_grid
            .map(|line| {
                line_signatures_by_line
                    .get(&line.0)
                    .copied()
                    .unwrap_or_else(|| line_signature(term, line, cols))
            })
            .unwrap_or(0);
        let timestamp_ms =
            line_in_grid.and_then(|line| line_timestamps_by_line.get(&line.0).copied());
        let command_mark =
            line_in_grid.and_then(|line| command_marks_by_line.get(&line.0).copied());
        let wrapped = line_in_grid.is_some_and(|line| {
            let previous_line = Line(line.0 - 1);
            cols > 0
                && previous_line >= topmost
                && term.grid()[previous_line][Column(cols - 1)]
                    .flags
                    .contains(Flags::WRAPLINE)
        });
        let key = TerminalSnapshotRowCacheKey {
            cols,
            signature,
            timestamp_ms,
            wrapped,
            command_mark,
        };
        let generation = row_cache.next_generation();
        let cached = row_cache.entries.get_mut(&key).and_then(|entry| {
            let row = entry.row.upgrade()?;
            snapshot_row_matches_term(row.as_ref(), term, line_in_grid, cols).then(|| {
                entry.last_used = generation;
                row
            })
        });
        let snapshot_row = if let Some(cached) = cached {
            stats.reused_rows = stats.reused_rows.saturating_add(1);
            cached
        } else {
            stats.rebuilt_rows = stats.rebuilt_rows.saturating_add(1);
            let rebuilt = Arc::new(snapshot_row_from_term(
                term,
                line_in_grid,
                cols,
                signature,
                timestamp_ms,
                wrapped,
                command_mark,
            ));
            row_cache.entries.insert(
                key,
                TerminalSnapshotRowCacheEntry {
                    row: Arc::downgrade(&rebuilt),
                    last_used: generation,
                },
            );
            rebuilt
        };
        row_data.push(snapshot_row);
    }
    row_cache.prune();
    drop(row_cache);

    let cursor_point = content.cursor.point;
    let cursor_row = if window.display_offset == 0 {
        usize::try_from(cursor_point.line.0 + window.older_rows as i32).unwrap_or(usize::MAX)
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
    let scrollback_len = if content.mode.contains(TermMode::ALT_SCREEN) {
        0
    } else {
        term.grid().history_size()
    };

    let cursor = CursorSnapshot {
        row: cursor_row,
        col: cursor_col,
        shape: cursor_shape,
        visible: cursor_visible,
        blinking: cursor_blinking,
    };
    let snapshot = TerminalSnapshot::from_rows(
        TerminalSnapshotMeta {
            cols,
            viewport_rows,
            cursor,
            selection,
            scrollback_len,
            total_rows: scrollback_len
                .saturating_add(viewport_rows)
                .saturating_add(window.newer_rows),
            display_offset,
            images: Vec::new(),
        },
        row_data,
    );
    (snapshot, stats)
}

fn snapshot_row_from_term(
    term: &Term<NyaTermEventProxy>,
    line: Option<Line>,
    cols: usize,
    signature: u64,
    timestamp_ms: Option<u64>,
    wrapped: bool,
    command_mark: Option<ShellCommandMark>,
) -> TerminalSnapshotRow {
    let cells = if let Some(line) = line {
        (0..cols)
            .map(|col| {
                let cell = &term.grid()[line][Column(col)];
                RenderCell {
                    text: cell_text(cell),
                    style: cell_style(cell),
                    width: render_cell_width(cell),
                    hyperlink: cell.hyperlink().map(|link| link.uri().to_string()),
                }
            })
            .collect::<Vec<_>>()
    } else {
        (0..cols)
            .map(|_| RenderCell {
                text: String::new(),
                style: CellStyle::default(),
                width: 1,
                hyperlink: None,
            })
            .collect()
    };
    let mut text = String::with_capacity(cols);
    for cell in &cells {
        push_render_cell_text(&mut text, cell);
    }
    text.truncate(text.trim_end().len());
    TerminalSnapshotRow {
        styled_spans: compress_render_row(&cells).into_boxed_slice(),
        hyperlinks: compress_render_hyperlinks(&cells).into_boxed_slice(),
        cells: cells.into_boxed_slice(),
        text,
        signature,
        timestamp_ms,
        wrapped,
        command_mark,
    }
}

fn snapshot_row_matches_term(
    row: &TerminalSnapshotRow,
    term: &Term<NyaTermEventProxy>,
    line: Option<Line>,
    cols: usize,
) -> bool {
    if row.cells.len() != cols {
        return false;
    }
    let Some(line) = line else {
        return row.cells.iter().all(|cell| {
            cell.text.is_empty()
                && cell.style == CellStyle::default()
                && cell.width == 1
                && cell.hyperlink.is_none()
        });
    };
    row.cells.iter().enumerate().all(|(col, snapshot_cell)| {
        let cell = &term.grid()[line][Column(col)];
        snapshot_cell.style == cell_style(cell)
            && snapshot_cell.width == render_cell_width(cell)
            && cell_hyperlink_matches(cell, snapshot_cell.hyperlink.as_deref())
            && cell_text_matches(cell, snapshot_cell.text.as_str())
    })
}

fn cell_hyperlink_matches(cell: &Cell, expected: Option<&str>) -> bool {
    match (cell.hyperlink(), expected) {
        (Some(link), Some(expected)) => link.uri() == expected,
        (None, None) => true,
        _ => false,
    }
}

fn cell_text_matches(cell: &Cell, text: &str) -> bool {
    if cell_text_is_blank(cell) {
        return text.is_empty();
    }
    text.chars()
        .eq(std::iter::once(cell.c).chain(cell.zerowidth().into_iter().flatten().copied()))
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

fn terminal_search_regex_pattern(key: &TerminalSearchCacheKey) -> String {
    let pattern = if key.regex {
        key.pattern.clone()
    } else {
        regex::escape(&key.pattern)
    };
    if key.case_sensitive {
        format!("(?-i:{pattern})")
    } else {
        format!("(?i:{pattern})")
    }
}

fn push_terminal_grid_match_segments(
    term: &Term<NyaTermEventProxy>,
    found: alacritty_terminal::term::search::Match,
    limit: usize,
    direction: TerminalSearchDirection,
    out: &mut Vec<TerminalGridMatch>,
) {
    let (start, end) = terminal_ordered_match_points(found);
    let cols = term.columns();
    let history = term.grid().history_size();
    let lines: Box<dyn Iterator<Item = i32>> = match direction {
        TerminalSearchDirection::Forward => Box::new(start.line.0..=end.line.0),
        TerminalSearchDirection::Backward => Box::new((start.line.0..=end.line.0).rev()),
    };
    for line_index in lines {
        if out.len() >= limit {
            break;
        }
        let line = Line(line_index);
        let start_col = if line == start.line {
            start.column.0
        } else {
            0
        };
        let end_col = if line == end.line {
            end.column.0.saturating_add(1).min(cols)
        } else {
            cols
        };
        if start_col >= end_col {
            continue;
        }
        let absolute = i64::try_from(history)
            .unwrap_or(i64::MAX)
            .saturating_add(i64::from(line.0));
        let Ok(line_index) = usize::try_from(absolute) else {
            continue;
        };
        out.push(TerminalGridMatch {
            line_index,
            start_col,
            end_col,
        });
    }
}

fn terminal_ordered_match_points(found: alacritty_terminal::term::search::Match) -> (Point, Point) {
    let start = *found.start();
    let end = *found.end();
    if start.line < end.line || (start.line == end.line && start.column <= end.column) {
        (start, end)
    } else {
        (end, start)
    }
}

fn terminal_grid_match_is_whole_word(
    term: &Term<NyaTermEventProxy>,
    found: &alacritty_terminal::term::search::Match,
) -> bool {
    let (start, end) = terminal_ordered_match_points(found.clone());
    let before = start.sub(term, Boundary::Grid, 1);
    let after = end.add(term, Boundary::Grid, 1);
    let before = (before != start)
        .then(|| terminal_word_char_at(term, before))
        .flatten();
    let after = (after != end)
        .then(|| terminal_word_char_at(term, after))
        .flatten();
    !before.is_some_and(terminal_search_is_word_char)
        && !after.is_some_and(terminal_search_is_word_char)
}

fn terminal_word_char_at(term: &Term<NyaTermEventProxy>, point: Point) -> Option<char> {
    if point.line < term.topmost_line() || point.line > term.bottommost_line() {
        return None;
    }
    let cell = &term.grid()[point.line][point.column];
    (!cell_text_is_blank(cell)).then_some(cell.c)
}

fn terminal_search_is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
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
        } else if let Some(line) = chunk.first()
            && out.last() != Some(line)
        {
            out.push(line.clone());
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
mod tests;
