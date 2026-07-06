use vte::{Params, Parser, Perform};

const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSnapshot {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
}

pub struct TerminalScreen {
    parser: Parser,
    cols: usize,
    rows: usize,
    cursor_row: usize,
    cursor_col: usize,
    cells: Vec<Vec<char>>,
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
            cells: vec![vec![' '; cols]; rows],
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let cols = usize::from(cols).max(1);
        let rows = usize::from(rows).max(1);
        self.cells.resize_with(rows, || vec![' '; cols]);
        for row in &mut self.cells {
            row.resize(cols, ' ');
        }
        self.cols = cols;
        self.rows = rows;
        self.cursor_row = self.cursor_row.min(self.rows - 1);
        self.cursor_col = self.cursor_col.min(self.cols - 1);
    }

    pub fn clear(&mut self) {
        for row in &mut self.cells {
            row.fill(' ');
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    pub fn advance(&mut self, bytes: &[u8]) {
        let mut parser = std::mem::take(&mut self.parser);
        parser.advance(self, bytes);
        self.parser = parser;
    }

    pub fn snapshot(&self) -> TerminalSnapshot {
        TerminalSnapshot {
            lines: self
                .cells
                .iter()
                .map(|row| row.iter().collect::<String>().trim_end().to_string())
                .collect(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
        }
    }

    pub fn lines(&self) -> Vec<String> {
        self.snapshot().lines
    }

    fn put_char(&mut self, c: char) {
        if self.cursor_col >= self.cols {
            self.newline();
            self.carriage_return();
        }
        self.cells[self.cursor_row][self.cursor_col] = c;
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
            self.cells.push(vec![' '; self.cols]);
        }
    }

    fn erase_display(&mut self, mode: u16) {
        match mode {
            0 => {
                self.erase_line_right();
                for row in self.cursor_row + 1..self.rows {
                    self.cells[row].fill(' ');
                }
            }
            1 => {
                for row in 0..self.cursor_row {
                    self.cells[row].fill(' ');
                }
                for col in 0..=self.cursor_col.min(self.cols - 1) {
                    self.cells[self.cursor_row][col] = ' ';
                }
            }
            2 | 3 => self.clear(),
            _ => {}
        }
    }

    fn erase_line(&mut self, mode: u16) {
        match mode {
            0 => self.erase_line_right(),
            1 => {
                for col in 0..=self.cursor_col.min(self.cols - 1) {
                    self.cells[self.cursor_row][col] = ' ';
                }
            }
            2 => self.cells[self.cursor_row].fill(' '),
            _ => {}
        }
    }

    fn erase_line_right(&mut self) {
        for col in self.cursor_col..self.cols {
            self.cells[self.cursor_row][col] = ' ';
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
            'm' => {}
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
}
