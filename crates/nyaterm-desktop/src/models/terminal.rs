use nyaterm_terminal::TerminalScreen;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::terminal::{terminal_screen_from_output, trim_terminal_output};

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
    pub(crate) hits: u64,
    pub(crate) misses: u64,
}

impl TerminalRenderCache {
    pub(crate) fn clear(&mut self) {
        self.action_link_ranges_by_line_hash.clear();
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
                let start = line[..item.start.min(line.len())].chars().count();
                let end = line[..item.end.min(line.len())].chars().count();
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

pub(crate) struct TerminalViewState {
    pub(crate) output: String,
    pub(crate) screen: TerminalScreen,
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
}

impl TerminalViewState {
    pub(crate) fn new() -> Self {
        Self {
            output: String::new(),
            screen: TerminalScreen::default(),
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
        }
    }

    pub(crate) fn from_output(output: String) -> Self {
        let screen = terminal_screen_from_output(&output);
        Self {
            output,
            screen,
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
        }
    }

    pub(crate) fn append_text(&mut self, text: &str) {
        let feed = self.protect_output_burst(text.as_bytes());
        self.append_bytes_unprotected(feed);
    }

    pub(crate) fn append_bytes(&mut self, data: &[u8]) {
        let feed = self.protect_output_burst(data);
        self.append_bytes_unprotected(feed);
    }

    /// Feed already-protected bytes into the view (used when the caller applies
    /// the same feed to the mirrored active screen).
    pub(crate) fn append_bytes_unprotected(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        self.screen.advance(data);
        self.screen_revision = self.screen_revision.saturating_add(1);
        self.output.push_str(&String::from_utf8_lossy(data));
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
        let mut feed = data;
        if feed.len() > TERMINAL_OUTPUT_VISIBLE_BACKLOG_CAP {
            let skip = feed.len() - TERMINAL_OUTPUT_VISIBLE_BACKLOG_CAP;
            self.note_skipped_output(skip);
            feed = &feed[skip..];
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
