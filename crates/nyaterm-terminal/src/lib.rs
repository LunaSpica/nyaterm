use vte::{Params, Parser, Perform};

const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;

/// Cell style carried from SGR (16-color ANSI indices + intensity flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellStyle {
    /// Foreground ANSI index 0..=15, or default when None.
    pub fg: Option<u8>,
    /// Background ANSI index 0..=15, or default when None.
    pub bg: Option<u8>,
    pub bold: bool,
    pub reverse: bool,
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
}

pub struct TerminalScreen {
    parser: Parser,
    cols: usize,
    rows: usize,
    cursor_row: usize,
    cursor_col: usize,
    cells: Vec<Vec<Cell>>,
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
        self.cols = cols;
        self.rows = rows;
        self.cursor_row = self.cursor_row.min(self.rows - 1);
        self.cursor_col = self.cursor_col.min(self.cols - 1);
    }

    pub fn clear(&mut self) {
        for row in &mut self.cells {
            row.fill(Cell::default());
        }
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
        let mut lines = Vec::with_capacity(self.rows);
        let mut styled_lines = Vec::with_capacity(self.rows);
        for row in &self.cells {
            let text: String = row.iter().map(|cell| cell.ch).collect();
            lines.push(text.trim_end().to_string());
            styled_lines.push(compress_row(row));
        }
        TerminalSnapshot {
            lines,
            styled_lines,
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
        }
    }

    pub fn lines(&self) -> Vec<String> {
        self.snapshot().lines
    }

    pub fn styled_lines(&self) -> Vec<Vec<StyledSpan>> {
        self.snapshot().styled_lines
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
            self.cells.remove(0);
            self.cells.push(vec![Cell::default(); self.cols]);
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
                7 => self.pen.reverse = true,
                22 => self.pen.bold = false,
                27 => self.pen.reverse = false,
                30..=37 => self.pen.fg = Some((code - 30) as u8),
                39 => self.pen.fg = None,
                40..=47 => self.pen.bg = Some((code - 40) as u8),
                49 => self.pen.bg = None,
                90..=97 => self.pen.fg = Some((code - 90 + 8) as u8),
                100..=107 => self.pen.bg = Some((code - 100 + 8) as u8),
                38 | 48 => {
                    let is_fg = code == 38;
                    if i + 1 < values.len() {
                        match values[i + 1] {
                            5 if i + 2 < values.len() => {
                                let idx = values[i + 2];
                                let mapped = if idx < 16 {
                                    Some(idx as u8)
                                } else {
                                    None
                                };
                                if is_fg {
                                    self.pen.fg = mapped;
                                } else {
                                    self.pen.bg = mapped;
                                }
                                i += 2;
                            }
                            2 if i + 4 < values.len() => {
                                // Truecolor ignored for now; keep previous color.
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
}
