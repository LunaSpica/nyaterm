use std::time::{SystemTime, UNIX_EPOCH};
use vte::{Params, Parser, Perform};

const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;

/// Cell style carried from SGR (ANSI / bright / truecolor + intensity flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellStyle {
    /// Foreground ANSI index 0..=15 when not using truecolor.
    pub fg: Option<u8>,
    /// Background ANSI index 0..=15 when not using truecolor.
    pub bg: Option<u8>,
    /// Truecolor foreground as 0xRRGGBB (CSI 38;2;r;g;b).
    pub fg_rgb: Option<u32>,
    /// Truecolor background as 0xRRGGBB (CSI 48;2;r;g;b).
    pub bg_rgb: Option<u32>,
    pub bold: bool,
    pub reverse: bool,
    pub underline: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cell {
    ch: char,
    style: CellStyle,
    /// Index into `TerminalScreen::hyperlinks` (None = no OSC 8 link).
    hyperlink: Option<u16>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: CellStyle::default(),
            hyperlink: None,
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSnapshot {
    pub lines: Vec<String>,
    pub styled_lines: Vec<Vec<StyledSpan>>,
    /// Wall-clock stamp (unix ms) for each viewport row, if the line was written.
    pub line_timestamps_ms: Vec<Option<u64>>,
    /// OSC 8 hyperlink spans per viewport line (char columns).
    pub hyperlink_lines: Vec<Vec<HyperlinkSpan>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    /// Rows available above the live screen (scrollback).
    pub scrollback_len: usize,
    /// Total rows in scrollback + live screen.
    pub total_rows: usize,
}

pub struct TerminalScreen {
    parser: Parser,
    cols: usize,
    rows: usize,
    cursor_row: usize,
    cursor_col: usize,
    cells: Vec<Vec<Cell>>,
    /// Parallel timestamps for live screen rows (unix ms).
    live_timestamps_ms: Vec<Option<u64>>,
    /// Lines scrolled off the top of the live screen (oldest first).
    scrollback: Vec<Vec<Cell>>,
    /// Parallel timestamps for scrollback rows (unix ms).
    scrollback_timestamps_ms: Vec<Option<u64>>,
    scrollback_limit: usize,
    pen: CellStyle,
    /// DECSET 2004 bracketed paste mode.
    bracketed_paste: bool,
    /// DECSET 1000/1002/1003: mouse reporting enabled (any of these).
    mouse_reporting: bool,
    /// DECSET 1006: SGR mouse reporting encoding.
    mouse_sgr: bool,
    /// DECSET 1049/47/1047: alternate screen buffer active.
    alternate_screen: bool,
    /// Saved main-screen cells when alternate buffer is entered.
    saved_main_cells: Option<Vec<Vec<Cell>>>,
    /// Saved main-screen timestamps when alternate buffer is entered.
    saved_main_timestamps_ms: Option<Vec<Option<u64>>>,
    /// Saved main-screen cursor when alternate buffer is entered.
    saved_main_cursor: Option<(usize, usize)>,
    /// DECSC/DECRC style saved cursor (CSI s / CSI u / DECSET 1048).
    saved_cursor: Option<(usize, usize)>,
    /// DECSTBM scroll region top row (0-based inclusive).
    scroll_top: usize,
    /// DECSTBM scroll region bottom row (0-based inclusive).
    scroll_bottom: usize,
    /// DECSET 6 origin mode: CUP is relative to scroll region.
    origin_mode: bool,
    /// Set when BEL (0x07) is received; UI should flash and clear.
    pending_visual_bell: bool,
    /// Latest OSC 0/2 window title (consumed by the UI layer).
    pending_window_title: Option<String>,
    /// Current window title last set by OSC 0/2.
    window_title: Option<String>,
    /// Hyperlink URI pool for OSC 8 (index stored on cells).
    hyperlinks: Vec<String>,
    /// Active OSC 8 hyperlink index while printing (None = closed).
    current_hyperlink: Option<u16>,
    /// OSC 133 shell integration: terminal has emitted marks.
    shell_integration_enabled: bool,
    /// OSC 133 C..D: a command is currently running.
    command_running: bool,
    /// Edge: command started (C) since last consume.
    pending_command_started: bool,
    /// Edge: command finished (D) since last consume.
    pending_command_finished: bool,
    /// Latest OSC 7 working directory path.
    cwd: Option<String>,
    /// Pending OSC 7 cwd update for UI consumption.
    pending_cwd: Option<String>,
}

impl Default for TerminalScreen {
    fn default() -> Self {
        Self::new(DEFAULT_COLS, DEFAULT_ROWS)
    }
}

impl TerminalScreen {
    pub fn new(cols: u16, rows: u16) -> Self {
        let cols = usize::from(cols).max(1);
        let rows = usize::from(rows).max(1);
        Self {
            parser: Parser::new(),
            cols,
            rows,
            cursor_row: 0,
            cursor_col: 0,
            cells: vec![vec![Cell::default(); cols]; rows],
            live_timestamps_ms: vec![None; rows],
            scrollback: Vec::new(),
            scrollback_timestamps_ms: Vec::new(),
            scrollback_limit: 5_000,
            pen: CellStyle::default(),
            bracketed_paste: false,
            mouse_reporting: false,
            mouse_sgr: false,
            alternate_screen: false,
            saved_main_cells: None,
            saved_main_timestamps_ms: None,
            saved_main_cursor: None,
            saved_cursor: None,
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            origin_mode: false,
            pending_visual_bell: false,
            pending_window_title: None,
            window_title: None,
            hyperlinks: Vec::new(),
            current_hyperlink: None,
            shell_integration_enabled: false,
            command_running: false,
            pending_command_started: false,
            pending_command_finished: false,
            cwd: None,
            pending_cwd: None,
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let cols = usize::from(cols).max(1);
        let rows = usize::from(rows).max(1);
        self.cells.resize_with(rows, || vec![Cell::default(); cols]);
        for row in &mut self.cells {
            row.resize(cols, Cell::default());
        }
        // Keep scrollback column width aligned when possible.
        for row in &mut self.scrollback {
            row.resize(cols, Cell::default());
        }
        self.live_timestamps_ms
            .resize(rows, None);
        if self.live_timestamps_ms.len() > rows {
            self.live_timestamps_ms.truncate(rows);
        }
        self.cols = cols;
        self.rows = rows;
        if self.scroll_bottom >= self.rows || self.scroll_top > self.scroll_bottom {
            self.scroll_top = 0;
            self.scroll_bottom = self.rows.saturating_sub(1);
        }
        self.cursor_row = self.cursor_row.min(self.rows - 1);
        self.cursor_col = self.cursor_col.min(self.cols - 1);
    }

    pub fn set_scrollback_limit(&mut self, limit: usize) {
        self.scrollback_limit = limit.max(0);
        self.trim_scrollback();
    }

    pub fn scrollback_len(&self) -> usize {
        if self.alternate_screen {
            0
        } else {
            self.scrollback.len()
        }
    }

    pub fn bracketed_paste(&self) -> bool {
        self.bracketed_paste
    }

    pub fn mouse_reporting(&self) -> bool {
        self.mouse_reporting
    }

    pub fn mouse_sgr(&self) -> bool {
        self.mouse_sgr
    }

    pub fn alternate_screen(&self) -> bool {
        self.alternate_screen
    }

    pub fn scroll_region(&self) -> (usize, usize) {
        (self.scroll_top, self.scroll_bottom)
    }

    pub fn origin_mode(&self) -> bool {
        self.origin_mode
    }

    /// Consume a pending visual bell flag (BEL / 0x07).
    pub fn take_visual_bell(&mut self) -> bool {
        let pending = self.pending_visual_bell;
        self.pending_visual_bell = false;
        pending
    }

    /// Latest OSC 0/2 title (does not clear).
    pub fn window_title(&self) -> Option<&str> {
        self.window_title.as_deref()
    }

    /// Consume a pending window-title update from OSC 0/2.
    pub fn take_window_title(&mut self) -> Option<String> {
        self.pending_window_title.take()
    }

    pub fn shell_integration_enabled(&self) -> bool {
        self.shell_integration_enabled
    }

    pub fn command_running(&self) -> bool {
        self.command_running
    }

    /// Consume OSC 133 C/D edges for the UI suggestion/history pipeline.
    pub fn take_shell_command_edges(&mut self) -> (bool, bool) {
        let started = self.pending_command_started;
        let finished = self.pending_command_finished;
        self.pending_command_started = false;
        self.pending_command_finished = false;
        (started, finished)
    }

    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    /// Consume a pending OSC 7 working-directory update.
    pub fn take_cwd(&mut self) -> Option<String> {
        self.pending_cwd.take()
    }

    pub fn total_rows(&self) -> usize {
        // Alternate buffer is isolated from primary scrollback history.
        self.scrollback_len() + self.rows
    }

    pub fn clear(&mut self) {
        for row in &mut self.cells {
            row.fill(Cell::default());
        }
        self.live_timestamps_ms.fill(None);
        self.scrollback.clear();
        self.scrollback_timestamps_ms.clear();
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.pen = CellStyle::default();
        self.bracketed_paste = false;
        self.mouse_reporting = false;
        self.mouse_sgr = false;
        self.alternate_screen = false;
        self.saved_main_cells = None;
        self.saved_main_timestamps_ms = None;
        self.saved_main_cursor = None;
        self.saved_cursor = None;
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
        self.origin_mode = false;
        self.pending_visual_bell = false;
        self.pending_window_title = None;
        self.window_title = None;
        self.hyperlinks.clear();
        self.current_hyperlink = None;
        self.shell_integration_enabled = false;
        self.command_running = false;
        self.pending_command_started = false;
        self.pending_command_finished = false;
        self.cwd = None;
        self.pending_cwd = None;
    }

    pub fn advance(&mut self, bytes: &[u8]) {
        let mut parser = std::mem::take(&mut self.parser);
        parser.advance(self, bytes);
        self.parser = parser;
    }

    pub fn snapshot(&self) -> TerminalSnapshot {
        self.viewport_snapshot(0)
    }

    /// Snapshot a viewport of the live screen height ending `offset` rows above the bottom.
    /// `offset == 0` is the live screen; larger offsets scroll into history.
    pub fn viewport_snapshot(&self, offset: usize) -> TerminalSnapshot {
        let total = self.total_rows();
        let max_offset = total.saturating_sub(self.rows);
        let offset = offset.min(max_offset);
        let end = total.saturating_sub(offset);
        let start = end.saturating_sub(self.rows);

        let mut lines = Vec::with_capacity(self.rows);
        let mut styled_lines = Vec::with_capacity(self.rows);
        let mut line_timestamps_ms = Vec::with_capacity(self.rows);
        let mut hyperlink_lines = Vec::with_capacity(self.rows);
        for abs_row in start..end {
            let row = self.row_at(abs_row);
            let text: String = row.iter().map(|cell| cell.ch).collect();
            lines.push(text.trim_end().to_string());
            styled_lines.push(compress_row(row));
            line_timestamps_ms.push(self.timestamp_at(abs_row));
            hyperlink_lines.push(compress_hyperlinks(row, &self.hyperlinks));
        }
        // Pad if history shorter than a full viewport (should be rare after clamp).
        while lines.len() < self.rows {
            lines.push(String::new());
            styled_lines.push(vec![StyledSpan {
                text: String::new(),
                style: CellStyle::default(),
            }]);
            line_timestamps_ms.push(None);
            hyperlink_lines.push(Vec::new());
        }

        let cursor_abs = self.scrollback.len() + self.cursor_row;
        let cursor_row = if offset == 0 && cursor_abs >= start && cursor_abs < end {
            cursor_abs - start
        } else if offset == 0 {
            self.cursor_row.min(self.rows.saturating_sub(1))
        } else {
            // Hide cursor when scrolled away from live bottom.
            usize::MAX
        };

        TerminalSnapshot {
            lines,
            styled_lines,
            line_timestamps_ms,
            hyperlink_lines,
            cursor_row,
            cursor_col: self.cursor_col,
            scrollback_len: self.scrollback.len(),
            total_rows: total,
        }
    }

    pub fn lines(&self) -> Vec<String> {
        self.snapshot().lines
    }

    pub fn styled_lines(&self) -> Vec<Vec<StyledSpan>> {
        self.snapshot().styled_lines
    }

    /// Full history + live screen plain text lines (absolute order, oldest first).
    pub fn all_lines(&self) -> Vec<String> {
        let total = self.total_rows();
        let mut lines = Vec::with_capacity(total);
        for abs in 0..total {
            let row = self.row_at(abs);
            let text: String = row.iter().map(|cell| cell.ch).collect();
            lines.push(text.trim_end().to_string());
        }
        lines
    }

    /// Absolute line range covered by a viewport offset (half-open).
    pub fn viewport_absolute_range(&self, offset: usize) -> (usize, usize) {
        let total = self.total_rows();
        let max_offset = total.saturating_sub(self.rows);
        let offset = offset.min(max_offset);
        let end = total.saturating_sub(offset);
        let start = end.saturating_sub(self.rows);
        (start, end)
    }

    fn row_at(&self, abs_row: usize) -> &[Cell] {
        let history = self.scrollback_len();
        if abs_row < history {
            &self.scrollback[abs_row]
        } else {
            let live = abs_row - history;
            &self.cells[live.min(self.rows.saturating_sub(1))]
        }
    }

    fn timestamp_at(&self, abs_row: usize) -> Option<u64> {
        let history = self.scrollback_len();
        if abs_row < history {
            self.scrollback_timestamps_ms.get(abs_row).copied().flatten()
        } else {
            let live = abs_row - history;
            self.live_timestamps_ms
                .get(live.min(self.rows.saturating_sub(1)))
                .copied()
                .flatten()
        }
    }

    fn stamp_current_line(&mut self) {
        if self.cursor_row >= self.live_timestamps_ms.len() {
            return;
        }
        if self.live_timestamps_ms[self.cursor_row].is_none() {
            self.live_timestamps_ms[self.cursor_row] = Some(now_unix_ms());
        }
    }

    fn trim_scrollback(&mut self) {
        if self.scrollback.len() > self.scrollback_limit {
            let excess = self.scrollback.len() - self.scrollback_limit;
            self.scrollback.drain(0..excess);
            if self.scrollback_timestamps_ms.len() > excess {
                self.scrollback_timestamps_ms.drain(0..excess);
            } else {
                self.scrollback_timestamps_ms.clear();
            }
        }
        // Keep parallel arrays aligned.
        if self.scrollback_timestamps_ms.len() > self.scrollback.len() {
            self.scrollback_timestamps_ms.truncate(self.scrollback.len());
        }
        while self.scrollback_timestamps_ms.len() < self.scrollback.len() {
            self.scrollback_timestamps_ms.push(None);
        }
    }


    fn put_char(&mut self, c: char) {
        if self.cursor_col >= self.cols {
            self.newline();
            self.carriage_return();
        }
        self.stamp_current_line();
        self.cells[self.cursor_row][self.cursor_col] = Cell {
            ch: c,
            style: self.pen,
            hyperlink: self.current_hyperlink,
        };
        self.cursor_col += 1;
    }

    fn carriage_return(&mut self) {
        self.cursor_col = 0;
    }

    fn newline(&mut self) {
        if self.cursor_row == self.scroll_bottom {
            self.scroll_region_up(1);
        } else if self.cursor_row + 1 < self.rows {
            self.cursor_row += 1;
        }
    }

    fn reverse_index(&mut self) {
        if self.cursor_row == self.scroll_top {
            self.scroll_region_down(1);
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
        }
    }

    fn backspace(&mut self) {
        self.cursor_col = self.cursor_col.saturating_sub(1);
    }

    fn tab(&mut self) {
        let next_stop = ((self.cursor_col / 8) + 1) * 8;
        self.cursor_col = next_stop.min(self.cols - 1);
    }

    fn scroll_up(&mut self, count: usize) {
        // Full-screen scroll (history-capable). Used when the scroll region is full.
        let count = count.min(self.rows);
        for _ in 0..count {
            let row = self.cells.remove(0);
            let ts = if !self.live_timestamps_ms.is_empty() {
                self.live_timestamps_ms.remove(0)
            } else {
                None
            };
            if self.alternate_screen {
                // Alternate buffer does not contribute to primary scrollback.
                let _ = (row, ts);
            } else {
                // Keep all rows for fidelity (including blanks).
                self.scrollback.push(row);
                self.scrollback_timestamps_ms.push(ts);
            }
            self.cells.push(vec![Cell::default(); self.cols]);
            self.live_timestamps_ms.push(None);
        }
        self.trim_scrollback();
    }

    fn scroll_region_up(&mut self, count: usize) {
        if self.scroll_top == 0 && self.scroll_bottom + 1 >= self.rows {
            self.scroll_up(count);
            return;
        }
        let top = self.scroll_top.min(self.rows.saturating_sub(1));
        let bottom = self.scroll_bottom.min(self.rows.saturating_sub(1)).max(top);
        let height = bottom - top + 1;
        let count = count.min(height);
        for _ in 0..count {
            // Drop the top of the region (no scrollback for partial regions).
            for row in top..bottom {
                self.cells[row] = std::mem::take(&mut self.cells[row + 1]);
                self.live_timestamps_ms[row] = self.live_timestamps_ms[row + 1];
            }
            self.cells[bottom] = vec![Cell::default(); self.cols];
            self.live_timestamps_ms[bottom] = None;
        }
    }

    fn scroll_region_down(&mut self, count: usize) {
        let top = self.scroll_top.min(self.rows.saturating_sub(1));
        let bottom = self.scroll_bottom.min(self.rows.saturating_sub(1)).max(top);
        let height = bottom - top + 1;
        let count = count.min(height);
        for _ in 0..count {
            for row in (top + 1..=bottom).rev() {
                self.cells[row] = std::mem::take(&mut self.cells[row - 1]);
                self.live_timestamps_ms[row] = self.live_timestamps_ms[row - 1];
            }
            self.cells[top] = vec![Cell::default(); self.cols];
            self.live_timestamps_ms[top] = None;
        }
    }

    fn set_scroll_region(&mut self, top: u16, bottom: u16) {
        let mut top = usize::from(top.saturating_sub(1));
        let mut bottom = if bottom == 0 {
            self.rows.saturating_sub(1)
        } else {
            usize::from(bottom.saturating_sub(1))
        };
        if top >= self.rows {
            top = 0;
        }
        if bottom >= self.rows {
            bottom = self.rows.saturating_sub(1);
        }
        if top > bottom {
            top = 0;
            bottom = self.rows.saturating_sub(1);
        }
        self.scroll_top = top;
        self.scroll_bottom = bottom;
        // xterm: setting margins moves the cursor to the home position.
        self.home_cursor();
    }

    fn home_cursor(&mut self) {
        if self.origin_mode {
            self.cursor_row = self.scroll_top;
        } else {
            self.cursor_row = 0;
        }
        self.cursor_col = 0;
    }

    fn move_cursor_clamped(&mut self, row: usize, col: usize) {
        let (min_row, max_row) = if self.origin_mode {
            (self.scroll_top, self.scroll_bottom.min(self.rows.saturating_sub(1)))
        } else {
            (0, self.rows.saturating_sub(1))
        };
        let row = row.clamp(min_row, max_row);
        let col = col.min(self.cols.saturating_sub(1));
        self.cursor_row = row;
        self.cursor_col = col;
    }

    fn enter_alternate_screen(&mut self, clear: bool) {
        if self.alternate_screen {
            if clear {
                for row in &mut self.cells {
                    row.fill(Cell::default());
                }
                self.live_timestamps_ms.fill(None);
                self.cursor_row = 0;
                self.cursor_col = 0;
            }
            return;
        }
        self.saved_main_cells = Some(self.cells.clone());
        self.saved_main_timestamps_ms = Some(self.live_timestamps_ms.clone());
        self.saved_main_cursor = Some((self.cursor_row, self.cursor_col));
        self.alternate_screen = true;
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
        if clear {
            for row in &mut self.cells {
                row.fill(Cell::default());
            }
            self.live_timestamps_ms.fill(None);
            self.cursor_row = 0;
            self.cursor_col = 0;
        }
    }

    fn leave_alternate_screen(&mut self, clear_before_restore: bool) {
        if !self.alternate_screen {
            return;
        }
        if clear_before_restore {
            for row in &mut self.cells {
                row.fill(Cell::default());
            }
            self.live_timestamps_ms.fill(None);
        }
        if let Some(cells) = self.saved_main_cells.take() {
            self.cells = cells;
        }
        if let Some(ts) = self.saved_main_timestamps_ms.take() {
            self.live_timestamps_ms = ts;
        }
        if let Some((row, col)) = self.saved_main_cursor.take() {
            self.cursor_row = row.min(self.rows.saturating_sub(1));
            self.cursor_col = col.min(self.cols.saturating_sub(1));
        }
        self.alternate_screen = false;
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
        // Ensure dimensions still match after restore.
        self.cells.resize_with(self.rows, || vec![Cell::default(); self.cols]);
        for row in &mut self.cells {
            row.resize(self.cols, Cell::default());
        }
        self.live_timestamps_ms.resize(self.rows, None);
    }

    fn insert_lines(&mut self, count: usize) {
        let top = self.cursor_row.max(self.scroll_top);
        let bottom = self.scroll_bottom.min(self.rows.saturating_sub(1));
        if top > bottom {
            return;
        }
        let count = count.min(bottom - top + 1);
        for _ in 0..count {
            for row in (top + 1..=bottom).rev() {
                self.cells[row] = std::mem::take(&mut self.cells[row - 1]);
                self.live_timestamps_ms[row] = self.live_timestamps_ms[row - 1];
            }
            self.cells[top] = vec![Cell::default(); self.cols];
            self.live_timestamps_ms[top] = None;
        }
    }

    fn delete_lines(&mut self, count: usize) {
        let top = self.cursor_row.max(self.scroll_top);
        let bottom = self.scroll_bottom.min(self.rows.saturating_sub(1));
        if top > bottom {
            return;
        }
        let count = count.min(bottom - top + 1);
        for _ in 0..count {
            for row in top..bottom {
                self.cells[row] = std::mem::take(&mut self.cells[row + 1]);
                self.live_timestamps_ms[row] = self.live_timestamps_ms[row + 1];
            }
            self.cells[bottom] = vec![Cell::default(); self.cols];
            self.live_timestamps_ms[bottom] = None;
        }
    }

    fn apply_private_mode(&mut self, mode: u16, enable: bool) {
        match mode {
            2004 => self.bracketed_paste = enable,
            6 => {
                self.origin_mode = enable;
                self.home_cursor();
            }
            1000 | 1002 | 1003 => {
                // Any of the classic mouse modes enables reporting; disable clears all.
                if enable {
                    self.mouse_reporting = true;
                } else {
                    self.mouse_reporting = false;
                }
            }
            1006 => self.mouse_sgr = enable,
            47 | 1047 => {
                if enable {
                    self.enter_alternate_screen(false);
                } else {
                    // 1047 clears alternate before restore in xterm; 47 restores as-is.
                    self.leave_alternate_screen(mode == 1047);
                }
            }
            1048 => {
                if enable {
                    self.saved_cursor = Some((self.cursor_row, self.cursor_col));
                } else if let Some((row, col)) = self.saved_cursor {
                    self.cursor_row = row.min(self.rows.saturating_sub(1));
                    self.cursor_col = col.min(self.cols.saturating_sub(1));
                }
            }
            1049 => {
                if enable {
                    self.saved_cursor = Some((self.cursor_row, self.cursor_col));
                    self.enter_alternate_screen(true);
                } else {
                    self.leave_alternate_screen(false);
                    if let Some((row, col)) = self.saved_cursor.take() {
                        self.cursor_row = row.min(self.rows.saturating_sub(1));
                        self.cursor_col = col.min(self.cols.saturating_sub(1));
                    }
                }
            }
            _ => {}
        }
    }

    fn erase_display(&mut self, mode: u16) {
        match mode {
            0 => {
                self.erase_line_right();
                for row in self.cursor_row + 1..self.rows {
                    self.cells[row].fill(Cell::default());
                }
            }
            1 => {
                for row in 0..self.cursor_row {
                    self.cells[row].fill(Cell::default());
                }
                for col in 0..=self.cursor_col.min(self.cols - 1) {
                    self.cells[self.cursor_row][col] = Cell::default();
                }
            }
            2 | 3 => {
                // CSI 2J/3J: clear display and home cursor (xterm common behavior for our UI).
                for row in &mut self.cells {
                    row.fill(Cell::default());
                }
                self.live_timestamps_ms.fill(None);
                if mode == 3 {
                    // CSI 3J also drops scrollback in many terminal emulators.
                    self.scrollback.clear();
                    self.scrollback_timestamps_ms.clear();
                }
                self.cursor_row = 0;
                self.cursor_col = 0;
            }
            _ => {}
        }
    }

    fn erase_line(&mut self, mode: u16) {
        match mode {
            0 => self.erase_line_right(),
            1 => {
                for col in 0..=self.cursor_col.min(self.cols - 1) {
                    self.cells[self.cursor_row][col] = Cell::default();
                }
            }
            2 => self.cells[self.cursor_row].fill(Cell::default()),
            _ => {}
        }
    }

    fn erase_line_right(&mut self) {
        for col in self.cursor_col..self.cols {
            self.cells[self.cursor_row][col] = Cell::default();
        }
    }

    fn move_cursor(&mut self, row: usize, col: usize) {
        // Absolute 0-based coordinates from CUP / HVP (origin mode adjusts base).
        let base_row = if self.origin_mode { self.scroll_top } else { 0 };
        self.move_cursor_clamped(base_row.saturating_add(row), col);
    }

    fn csi_param(params: &Params, index: usize, default: u16) -> u16 {
        params
            .iter()
            .nth(index)
            .and_then(|param| param.first().copied())
            .filter(|value| *value > 0)
            .unwrap_or(default)
    }

    fn apply_sgr(&mut self, params: &Params) {
        let mut values: Vec<u16> = params
            .iter()
            .flat_map(|param| param.iter().copied())
            .collect();
        if values.is_empty() {
            values.push(0);
        }
        let mut i = 0;
        while i < values.len() {
            let code = values[i];
            match code {
                0 => self.pen = CellStyle::default(),
                1 => self.pen.bold = true,
                2 => self.pen.bold = false, // dim ignored as intensity down
                4 => self.pen.underline = true,
                7 => self.pen.reverse = true,
                22 => self.pen.bold = false,
                24 => self.pen.underline = false,
                27 => self.pen.reverse = false,
                30..=37 => {
                    self.pen.fg = Some((code - 30) as u8);
                    self.pen.fg_rgb = None;
                }
                39 => {
                    self.pen.fg = None;
                    self.pen.fg_rgb = None;
                }
                40..=47 => {
                    self.pen.bg = Some((code - 40) as u8);
                    self.pen.bg_rgb = None;
                }
                49 => {
                    self.pen.bg = None;
                    self.pen.bg_rgb = None;
                }
                90..=97 => {
                    self.pen.fg = Some((code - 90 + 8) as u8);
                    self.pen.fg_rgb = None;
                }
                100..=107 => {
                    self.pen.bg = Some((code - 100 + 8) as u8);
                    self.pen.bg_rgb = None;
                }
                38 | 48 => {
                    let is_fg = code == 38;
                    if i + 1 < values.len() {
                        match values[i + 1] {
                            5 if i + 2 < values.len() => {
                                let idx = values[i + 2];
                                let mapped = if idx < 16 {
                                    Some(idx as u8)
                                } else {
                                    // Approximate 256-color cube/grayscale to nearest ANSI-ish gray/default.
                                    Some(map_256_to_ansi16(idx))
                                };
                                if is_fg {
                                    self.pen.fg = mapped;
                                    self.pen.fg_rgb = None;
                                } else {
                                    self.pen.bg = mapped;
                                    self.pen.bg_rgb = None;
                                }
                                i += 2;
                            }
                            2 if i + 4 < values.len() => {
                                let r = values[i + 2].min(255) as u32;
                                let g = values[i + 3].min(255) as u32;
                                let b = values[i + 4].min(255) as u32;
                                let rgb = (r << 16) | (g << 8) | b;
                                if is_fg {
                                    self.pen.fg_rgb = Some(rgb);
                                    self.pen.fg = None;
                                } else {
                                    self.pen.bg_rgb = Some(rgb);
                                    self.pen.bg = None;
                                }
                                i += 4;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn map_256_to_ansi16(idx: u16) -> u8 {
    match idx {
        0..=15 => idx as u8,
        232..=255 => {
            // Grayscale ramp -> dark/light grays.
            let level = idx - 232;
            if level < 12 { 8 } else { 7 }
        }
        _ => {
            // 16..231 color cube: pick dominant channel.
            let n = idx - 16;
            let r = n / 36;
            let g = (n % 36) / 6;
            let b = n % 6;
            let max = r.max(g).max(b);
            if max == 0 {
                0
            } else if r == max && g == max && b == max {
                if max >= 4 { 15 } else { 8 }
            } else if r == max && r > g && r > b {
                if r >= 4 { 9 } else { 1 }
            } else if g == max && g > r && g > b {
                if g >= 4 { 10 } else { 2 }
            } else if b == max && b > r && b > g {
                if b >= 4 { 12 } else { 4 }
            } else if r == max && g == max {
                if r >= 4 { 11 } else { 3 }
            } else if r == max && b == max {
                if r >= 4 { 13 } else { 5 }
            } else if g == max && b == max {
                if g >= 4 { 14 } else { 6 }
            } else {
                7
            }
        }
    }
}



fn parse_osc7_path(payload: &str) -> Option<String> {
    let after_scheme = payload.strip_prefix("file://")?;
    let path = if after_scheme.starts_with('/') {
        after_scheme.to_string()
    } else {
        let slash = after_scheme.find('/')?;
        after_scheme[slash..].to_string()
    };
    if path.is_empty() {
        None
    } else {
        // Percent-decode common %20 etc. lightly
        Some(path.replace("%20", " "))
    }
}

fn compress_hyperlinks(row: &[Cell], pool: &[String]) -> Vec<HyperlinkSpan> {
    let mut spans = Vec::new();
    let mut col = 0usize;
    while col < row.len() {
        let Some(idx) = row[col].hyperlink else {
            col += 1;
            continue;
        };
        let start = col;
        col += 1;
        while col < row.len() && row[col].hyperlink == Some(idx) {
            col += 1;
        }
        if let Some(uri) = pool.get(idx as usize) {
            if !uri.is_empty() {
                spans.push(HyperlinkSpan {
                    start_col: start,
                    end_col: col.saturating_sub(1),
                    uri: uri.clone(),
                });
            }
        }
    }
    spans
}

fn compress_row(row: &[Cell]) -> Vec<StyledSpan> {
    let mut spans = Vec::new();
    let mut end = row.len();
    while end > 0 && row[end - 1].ch == ' ' && row[end - 1].style == CellStyle::default() {
        end -= 1;
    }
    if end == 0 {
        return vec![StyledSpan {
            text: String::new(),
            style: CellStyle::default(),
        }];
    }
    let mut start = 0;
    while start < end {
        let style = row[start].style;
        let mut next = start + 1;
        while next < end && row[next].style == style {
            next += 1;
        }
        let text: String = row[start..next].iter().map(|cell| cell.ch).collect();
        spans.push(StyledSpan { text, style });
        start = next;
    }
    spans
}


/// Encode a mouse event for the active terminal mouse protocol.
/// `button`: 0 left, 1 middle, 2 right, 3 release, 64/65 wheel up/down.
/// `col`/`row` are 0-based cell coordinates.
/// Returns empty when mouse reporting is disabled.
pub fn encode_mouse_report(
    screen: &TerminalScreen,
    button: u8,
    col: u16,
    row: u16,
    press: bool,
) -> Vec<u8> {
    if !screen.mouse_reporting() {
        return Vec::new();
    }
    // Terminals report 1-based cells.
    let col = col.saturating_add(1);
    let row = row.saturating_add(1);
    if screen.mouse_sgr() {
        // SGR extended: CSI < Cb ; Cx ; Cy M/m
        let mut bytes = vec![0x1b, b'[', b'<'];
        bytes.extend(button.to_string().as_bytes());
        bytes.push(b';');
        bytes.extend(col.to_string().as_bytes());
        bytes.push(b';');
        bytes.extend(row.to_string().as_bytes());
        // Wheel events always use 'M'; release uses 'm'.
        bytes.push(if press || button >= 64 { b'M' } else { b'm' });
        return bytes;
    }
    // Legacy X10-style: CSI M Cb Cx Cy with 32+ offsets, clamped to 223.
    let cb = 32u8.saturating_add(button);
    let cx = 32u8.saturating_add(col.min(223) as u8);
    let cy = 32u8.saturating_add(row.min(223) as u8);
    vec![0x1b, b'[', b'M', cb, cx, cy]
}

impl Perform for TerminalScreen {
    fn print(&mut self, c: char) {
        self.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0b | 0x0c => {
                self.newline();
                self.carriage_return();
            }
            b'\r' => self.carriage_return(),
            0x08 => self.backspace(),
            b'\t' => self.tab(),
            0x07 => {
                // BEL — visual bell for the UI layer.
                self.pending_visual_bell = true;
            }
            // IND / NEL / RI (C1 or after ESC conversion).
            0x84 => {
                self.newline();
            }
            0x85 => {
                self.newline();
                self.carriage_return();
            }
            0x8d => {
                self.reverse_index();
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        if ignore || !intermediates.is_empty() {
            return;
        }
        match byte {
            // ESC D Index, ESC E Next Line, ESC M Reverse Index.
            b'D' => self.newline(),
            b'E' => {
                self.newline();
                self.carriage_return();
            }
            b'M' => self.reverse_index(),
            // ESC 7 / 8 save/restore cursor (DECSC/DECRC).
            b'7' => {
                self.saved_cursor = Some((self.cursor_row, self.cursor_col));
            }
            b'8' => {
                if let Some((row, col)) = self.saved_cursor {
                    self.cursor_row = row.min(self.rows.saturating_sub(1));
                    self.cursor_col = col.min(self.cols.saturating_sub(1));
                }
            }
            // ESC c RIS hard reset of modes we track.
            b'c' => {
                self.clear();
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        if ignore {
            return;
        }
        // DEC private modes: CSI ? <n> h/l (bracketed paste, mouse, alt screen).
        if intermediates == [b'?'] && matches!(action, 'h' | 'l') {
            let enable = action == 'h';
            for param in params.iter() {
                for value in param.iter() {
                    self.apply_private_mode(*value, enable);
                }
            }
            return;
        }
        match action {
            'A' => {
                let count = Self::csi_param(params, 0, 1) as usize;
                let min_row = if self.origin_mode { self.scroll_top } else { 0 };
                self.cursor_row = self.cursor_row.saturating_sub(count).max(min_row);
            }
            'B' => {
                let count = Self::csi_param(params, 0, 1) as usize;
                let max_row = if self.origin_mode {
                    self.scroll_bottom.min(self.rows.saturating_sub(1))
                } else {
                    self.rows.saturating_sub(1)
                };
                self.cursor_row = (self.cursor_row + count).min(max_row);
            }
            'C' => {
                let count = Self::csi_param(params, 0, 1) as usize;
                self.cursor_col = (self.cursor_col + count).min(self.cols - 1);
            }
            'D' => {
                let count = Self::csi_param(params, 0, 1) as usize;
                self.cursor_col = self.cursor_col.saturating_sub(count);
            }
            'H' | 'f' => {
                let row = Self::csi_param(params, 0, 1).saturating_sub(1) as usize;
                let col = Self::csi_param(params, 1, 1).saturating_sub(1) as usize;
                self.move_cursor(row, col);
            }
            'J' => self.erase_display(Self::csi_param(params, 0, 0)),
            'K' => self.erase_line(Self::csi_param(params, 0, 0)),
            // CSI S / T: scroll up/down inside the current region.
            'S' => {
                let count = Self::csi_param(params, 0, 1) as usize;
                self.scroll_region_up(count.max(1));
            }
            'T' => {
                let count = Self::csi_param(params, 0, 1) as usize;
                self.scroll_region_down(count.max(1));
            }
            // DECSTBM: CSI top ; bottom r
            'r' => {
                let top = Self::csi_param(params, 0, 1);
                let bottom = Self::csi_param(params, 1, 0);
                self.set_scroll_region(top, bottom);
            }
            // Insert / delete lines within the scroll region (vim-heavy).
            'L' => {
                let count = Self::csi_param(params, 0, 1) as usize;
                self.insert_lines(count.max(1));
            }
            'M' => {
                let count = Self::csi_param(params, 0, 1) as usize;
                self.delete_lines(count.max(1));
            }
            'G' => {
                let col = Self::csi_param(params, 0, 1).saturating_sub(1) as usize;
                self.cursor_col = col.min(self.cols - 1);
            }
            'm' => self.apply_sgr(params),
            // ANSI.SYS / xterm save/restore cursor (CSI s / CSI u).
            's' => {
                self.saved_cursor = Some((self.cursor_row, self.cursor_col));
            }
            'u' => {
                if let Some((row, col)) = self.saved_cursor {
                    self.cursor_row = row.min(self.rows.saturating_sub(1));
                    self.cursor_col = col.min(self.cols.saturating_sub(1));
                }
            }
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.is_empty() {
            return;
        }
        let code = std::str::from_utf8(params[0]).unwrap_or("").trim();
        // OSC 0 / 2: set window title (and icon name for 0).
        if matches!(code, "0" | "2") {
            let title = params
                .get(1..)
                .map(|parts| {
                    parts
                        .iter()
                        .map(|part| String::from_utf8_lossy(part))
                        .collect::<Vec<_>>()
                        .join(";")
                })
                .unwrap_or_default();
            let title = title.trim();
            if title.is_empty() {
                return;
            }
            // Keep titles compact for tab chrome.
            let clipped: String = title.chars().take(120).collect();
            self.window_title = Some(clipped.clone());
            self.pending_window_title = Some(clipped);
            return;
        }
        // OSC 8 ; params ; uri ST — hyperlink start/end (empty uri closes).
        if code == "8" {
            // params[0]=8, params[1]=id params, params[2]=uri (may be missing/empty)
            let uri = params
                .get(2)
                .map(|p| String::from_utf8_lossy(p).to_string())
                .unwrap_or_default();
            let uri = uri.trim();
            if uri.is_empty() {
                self.current_hyperlink = None;
            } else {
                let clipped: String = uri.chars().take(2048).collect();
                let idx = if let Some(pos) = self.hyperlinks.iter().position(|u| u == &clipped) {
                    pos
                } else {
                    if self.hyperlinks.len() >= u16::MAX as usize {
                        self.hyperlinks.clear();
                    }
                    self.hyperlinks.push(clipped);
                    self.hyperlinks.len() - 1
                };
                self.current_hyperlink = Some(idx as u16);
            }
            return;
        }
        // OSC 7 ; file://host/path — working directory.
        if code == "7" {
            let payload = params
                .get(1..)
                .map(|parts| {
                    parts
                        .iter()
                        .map(|p| String::from_utf8_lossy(p))
                        .collect::<Vec<_>>()
                        .join(";")
                })
                .unwrap_or_default();
            if let Some(path) = parse_osc7_path(payload.trim()) {
                self.cwd = Some(path.clone());
                self.pending_cwd = Some(path);
            }
            return;
        }
        // OSC 133 shell integration (FinalTerm / iTerm / VS Code).
        // Forms: OSC 133 ; A|B|C|D [; exit] ST  — mark letter is params[1].
        if code == "133" || code.starts_with("133") {
            let mark = if code == "133" {
                params
                    .get(1)
                    .and_then(|p| p.first().copied())
                    .map(|b| b as char)
                    .unwrap_or('\0')
            } else {
                code.chars().nth(3).unwrap_or('\0')
            };
            match mark {
                'A' | 'B' => {
                    self.shell_integration_enabled = true;
                    // Prompt / command-start of input: not running.
                    if mark == 'B' {
                        self.command_running = false;
                    }
                }
                'C' => {
                    self.shell_integration_enabled = true;
                    self.command_running = true;
                    self.pending_command_started = true;
                }
                'D' => {
                    self.shell_integration_enabled = true;
                    self.command_running = false;
                    self.pending_command_finished = true;
                }
                _ => {}
            }
        }
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
    fn osc8_hyperlink_spans() {
        let mut screen = TerminalScreen::new(40, 3);
        // OSC 8 ;;uri BEL text OSC 8 ;; BEL
        screen.advance(b"\x1b]8;;https://example.com\x07click\x1b]8;;\x07 plain");
        let snap = screen.viewport_snapshot(0);
        let spans = &snap.hyperlink_lines[0];
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].uri, "https://example.com");
        assert_eq!(spans[0].start_col, 0);
        assert_eq!(spans[0].end_col, 4); // "click"
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
    fn prints_and_wraps_lines() {
        let mut screen = TerminalScreen::new(5, 3);
        screen.advance(b"hello\nworld");
        assert_eq!(screen.lines()[0], "hello");
        assert_eq!(screen.lines()[1], "world");
    }

    #[test]
    fn carriage_return_overwrites_current_line() {
        let mut screen = TerminalScreen::new(10, 2);
        screen.advance(b"old\rnew");
        assert_eq!(screen.lines()[0], "new");
    }

    #[test]
    fn clear_screen_removes_previous_content() {
        let mut screen = TerminalScreen::new(10, 2);
        screen.advance(b"before\x1b[2Jafter");
        assert_eq!(screen.lines()[0], "after");
    }

    #[test]
    fn cursor_position_writes_at_requested_cell() {
        let mut screen = TerminalScreen::new(8, 4);
        screen.advance(b"\x1b[3;4HX");
        assert_eq!(screen.lines()[2], "   X");
    }

    #[test]
    fn sgr_sequences_do_not_pollute_text() {
        let mut screen = TerminalScreen::new(12, 2);
        screen.advance(b"\x1b[31mred\x1b[0m");
        assert_eq!(screen.lines()[0], "red");
    }

    #[test]
    fn sgr_preserves_ansi_color_index() {
        let mut screen = TerminalScreen::new(12, 2);
        screen.advance(b"\x1b[31mred\x1b[0m plain");
        let styled = screen.styled_lines();
        assert_eq!(styled[0][0].text, "red");
        assert_eq!(styled[0][0].style.fg, Some(1));
        assert!(styled[0].iter().any(|span| span.text.contains("plain") && span.style.fg.is_none()));
    }

    #[test]
    fn sgr_bright_and_bold() {
        let mut screen = TerminalScreen::new(20, 2);
        screen.advance(b"\x1b[1;92mok\x1b[0m");
        let styled = screen.styled_lines();
        assert_eq!(styled[0][0].style.fg, Some(10));
        assert!(styled[0][0].style.bold);
    }

    #[test]
    fn sgr_truecolor_and_underline() {
        let mut screen = TerminalScreen::new(20, 2);
        screen.advance(b"\x1b[4;38;2;255;128;0mhi\x1b[0m");
        let styled = screen.styled_lines();
        assert_eq!(styled[0][0].text, "hi");
        assert!(styled[0][0].style.underline);
        assert_eq!(styled[0][0].style.fg_rgb, Some(0xff8000));
        assert_eq!(styled[0][0].style.fg, None);
    }

    #[test]
    fn scrollback_preserves_scrolled_lines() {
        let mut screen = TerminalScreen::new(8, 2);
        screen.set_scrollback_limit(100);
        screen.advance(b"line1\nline2\nline3");
        assert!(screen.scrollback_len() >= 1);
        let live = screen.snapshot();
        assert!(live.lines.iter().any(|l| l.contains("line3") || l.contains("line2")));
        let history = screen.viewport_snapshot(1);
        assert!(
            history.lines.iter().any(|l| l.contains("line1") || l.contains("line2")),
            "expected history viewport to include earlier lines: {:?}",
            history.lines
        );
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
    fn alternate_screen_isolates_main_scrollback() {
        let mut screen = TerminalScreen::new(10, 3);
        screen.advance(b"main-line\r\n");
        screen.advance(b"main-2\r\n");
        screen.advance(b"main-3\r\n");
        screen.advance(b"main-4\r\n");
        let main_scroll = screen.scrollback_len();
        assert!(main_scroll > 0);
        // Enter alt screen (xterm 1049): clear alt buffer.
        screen.advance(b"\x1b[?1049h");
        assert!(screen.alternate_screen());
        assert_eq!(screen.scrollback_len(), 0);
        screen.advance(b"ALT");
        let snap = screen.viewport_snapshot(0);
        assert!(snap.lines.iter().any(|line| line.contains('A')));
        // Leave alt screen and restore main.
        screen.advance(b"\x1b[?1049l");
        assert!(!screen.alternate_screen());
        assert_eq!(screen.scrollback_len(), main_scroll);
    }

    #[test]
    fn mouse_reporting_modes_track_decset() {
        let mut screen = TerminalScreen::new(20, 2);
        assert!(!screen.mouse_reporting());
        assert!(!screen.mouse_sgr());
        screen.advance(b"\x1b[?1000h");
        screen.advance(b"\x1b[?1006h");
        assert!(screen.mouse_reporting());
        assert!(screen.mouse_sgr());
        screen.advance(b"\x1b[?1000l");
        screen.advance(b"\x1b[?1006l");
        assert!(!screen.mouse_reporting());
        assert!(!screen.mouse_sgr());
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
        let release = encode_mouse_report(&screen, 0, 1, 2, false);
        assert_eq!(release, b"\x1b[<0;2;3m".to_vec());
    }

    #[test]
    fn decstbm_scroll_region_keeps_status_line() {
        let mut screen = TerminalScreen::new(8, 4);
        // Fill rows 0..3 with markers.
        screen.advance(b"AAAAAAA\r\nBBBBBBB\r\nCCCCCCC\r\nDDDDDDD");
        // Scroll region rows 1-3 (1-based 2;4).
        screen.advance(b"\x1b[2;4r");
        assert_eq!(screen.scroll_region(), (1, 3));
        // Cursor home after DECSTBM.
        assert_eq!(screen.snapshot().cursor_row, 0);
        // Move to bottom of region and force scroll.
        screen.advance(b"\x1b[4;1H");
        screen.advance(b"\nEEEEEEE");
        let lines = screen.snapshot().lines;
        // Top status-like row outside region remains.
        assert!(lines[0].starts_with('A'), "expected preserved top row, got {:?}", lines[0]);
        // Bottom row received new content after region scroll.
        assert!(
            lines[3].contains('E'),
            "expected scrolled content on last row, got {:?}",
            lines[3]
        );
    }

    #[test]
    fn origin_mode_homes_into_scroll_region() {
        let mut screen = TerminalScreen::new(10, 5);
        screen.advance(b"\x1b[2;4r");
        screen.advance(b"\x1b[?6h");
        assert!(screen.origin_mode());
        let snap = screen.snapshot();
        assert_eq!(snap.cursor_row, 1);
        assert_eq!(snap.cursor_col, 0);
        // CUP 1;1 is relative to region top.
        screen.advance(b"\x1b[1;1H");
        let snap = screen.snapshot();
        assert_eq!(snap.cursor_row, 1);
        screen.advance(b"\x1b[?6l");
        assert!(!screen.origin_mode());
    }

    #[test]
    fn reverse_index_scrolls_region_down() {
        let mut screen = TerminalScreen::new(6, 3);
        screen.advance(b"111111\r\n222222\r\n333333");
        screen.advance(b"\x1b[1;3r");
        screen.advance(b"\x1b[1;1H");
        screen.advance(b"\x1bM"); // reverse index at top -> scroll down
        let lines = screen.snapshot().lines;
        assert!(
            lines[0].trim().is_empty() || lines[0].chars().all(|c| c == ' '),
            "top should be blank after reverse index scroll, got {:?}",
            lines[0]
        );
        assert!(lines[1].contains('1'), "row1 should hold previous top, got {:?}", lines[1]);
    }




    #[test]
    fn line_timestamps_are_stamped_on_write() {
        let mut screen = TerminalScreen::new(20, 3);
        screen.advance(b"hello\nworld");
        let snap = screen.snapshot();
        assert!(snap.line_timestamps_ms[0].is_some(), "first written line should be stamped");
        assert!(snap.line_timestamps_ms[1].is_some(), "second written line should be stamped");
    }

    #[test]
    fn scrollback_preserves_line_timestamps() {
        let mut screen = TerminalScreen::new(8, 2);
        screen.set_scrollback_limit(100);
        screen.advance(b"line1\nline2\nline3");
        assert!(screen.scrollback_len() >= 1);
        let history = screen.viewport_snapshot(screen.scrollback_len());
        assert!(
            history.line_timestamps_ms.iter().any(|ts| ts.is_some()),
            "history viewport should retain timestamps: {:?}",
            history.line_timestamps_ms
        );
    }
}
