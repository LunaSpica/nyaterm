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
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: CellStyle::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledSpan {
    pub text: String,
    pub style: CellStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSnapshot {
    pub lines: Vec<String>,
    pub styled_lines: Vec<Vec<StyledSpan>>,
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
    /// Lines scrolled off the top of the live screen (oldest first).
    scrollback: Vec<Vec<Cell>>,
    scrollback_limit: usize,
    pen: CellStyle,
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
            scrollback: Vec::new(),
            scrollback_limit: 5_000,
            pen: CellStyle::default(),
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
        self.cols = cols;
        self.rows = rows;
        self.cursor_row = self.cursor_row.min(self.rows - 1);
        self.cursor_col = self.cursor_col.min(self.cols - 1);
    }

    pub fn set_scrollback_limit(&mut self, limit: usize) {
        self.scrollback_limit = limit.max(0);
        self.trim_scrollback();
    }

    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    pub fn total_rows(&self) -> usize {
        self.scrollback.len() + self.rows
    }

    pub fn clear(&mut self) {
        for row in &mut self.cells {
            row.fill(Cell::default());
        }
        self.scrollback.clear();
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.pen = CellStyle::default();
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
        for abs_row in start..end {
            let row = self.row_at(abs_row);
            let text: String = row.iter().map(|cell| cell.ch).collect();
            lines.push(text.trim_end().to_string());
            styled_lines.push(compress_row(row));
        }
        // Pad if history shorter than a full viewport (should be rare after clamp).
        while lines.len() < self.rows {
            lines.push(String::new());
            styled_lines.push(vec![StyledSpan {
                text: String::new(),
                style: CellStyle::default(),
            }]);
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

    fn row_at(&self, abs_row: usize) -> &[Cell] {
        if abs_row < self.scrollback.len() {
            &self.scrollback[abs_row]
        } else {
            let live = abs_row - self.scrollback.len();
            &self.cells[live.min(self.rows.saturating_sub(1))]
        }
    }

    fn trim_scrollback(&mut self) {
        if self.scrollback.len() > self.scrollback_limit {
            let excess = self.scrollback.len() - self.scrollback_limit;
            self.scrollback.drain(0..excess);
        }
    }


    fn put_char(&mut self, c: char) {
        if self.cursor_col >= self.cols {
            self.newline();
            self.carriage_return();
        }
        self.cells[self.cursor_row][self.cursor_col] = Cell {
            ch: c,
            style: self.pen,
        };
        self.cursor_col += 1;
    }

    fn carriage_return(&mut self) {
        self.cursor_col = 0;
    }

    fn newline(&mut self) {
        if self.cursor_row + 1 >= self.rows {
            self.scroll_up(1);
        } else {
            self.cursor_row += 1;
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
        let count = count.min(self.rows);
        for _ in 0..count {
            let row = self.cells.remove(0);
            // Only keep non-empty/styled rows to reduce noise? Keep all for fidelity.
            self.scrollback.push(row);
            self.cells.push(vec![Cell::default(); self.cols]);
        }
        self.trim_scrollback();
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
        self.cursor_row = row.min(self.rows - 1);
        self.cursor_col = col.min(self.cols - 1);
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
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], ignore: bool, action: char) {
        if ignore {
            return;
        }
        match action {
            'A' => {
                let count = Self::csi_param(params, 0, 1) as usize;
                self.cursor_row = self.cursor_row.saturating_sub(count);
            }
            'B' => {
                let count = Self::csi_param(params, 0, 1) as usize;
                self.cursor_row = (self.cursor_row + count).min(self.rows - 1);
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
            'G' => {
                let col = Self::csi_param(params, 0, 1).saturating_sub(1) as usize;
                self.cursor_col = col.min(self.cols - 1);
            }
            'm' => self.apply_sgr(params),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
