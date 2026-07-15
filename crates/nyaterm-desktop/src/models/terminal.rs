use nyaterm_core::{TerminalBackendResize, terminal_backend_resize_changed};
use nyaterm_terminal::{TerminalOutputDecoder, TerminalScreen};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use crate::terminal::{
    NyaTerminalLayoutCache, terminal_cell_col_for_byte_index, terminal_screen_from_output,
    trim_terminal_output,
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

#[derive(Debug, Clone, Default)]
pub(crate) struct TerminalRenderedLine {
    pub(crate) text: String,
    pub(crate) action_link_ranges: Vec<(usize, usize)>,
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
    /// Last size sent to the PTY/backend for this session.
    pub(crate) last_backend_resize: Option<TerminalBackendResize>,
}

impl TerminalViewState {
    pub(crate) fn new() -> Self {
        Self {
            output: String::new(),
            screen: TerminalScreen::default(),
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
            last_backend_resize: None,
        }
    }

    pub(crate) fn from_output(output: String) -> Self {
        let screen = terminal_screen_from_output(&output);
        Self {
            output,
            screen,
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

    pub(crate) fn tick_performance_overlay(&mut self) {
        // End-of-tick calm accounting for recovery.
        self.maybe_exit_overloaded_mode();
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
    }

    pub(crate) fn clamp_scroll_offset(&mut self) {
        let max = self.screen.scrollback_len();
        if self.scroll_offset > max {
            self.scroll_offset = max;
        }
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
