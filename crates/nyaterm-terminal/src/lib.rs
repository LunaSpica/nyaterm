use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi;
use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb};

const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;

/// Cell style carried from Alacritty's terminal grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
    pub rows: usize,
    pub cells: Vec<RenderCell>,
    pub cursor: CursorSnapshot,
    pub selection: Option<SelectionSnapshot>,
    pub lines: Vec<String>,
    pub styled_lines: Vec<Vec<StyledSpan>>,
    /// Wall-clock stamp (unix ms) for each viewport row, if known.
    pub line_timestamps_ms: Vec<Option<u64>>,
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
}

#[derive(Debug, Clone, Default)]
pub struct TerminalEffects {
    pub title: Option<String>,
    pub reset_title: bool,
    pub bell: bool,
    pub cwd: Option<String>,
    pub shell_command_started: bool,
    pub shell_command_finished: bool,
    pub pty_write: Vec<Vec<u8>>,
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
    proxy: NyaTermEventProxy,
    sidecar: NyaTermSidecar,
    pending_effects: TerminalEffects,
    rows: usize,
    cols: usize,
    scrollback_limit: usize,
}

pub type TerminalScreen = TerminalCore;

impl Default for TerminalCore {
    fn default() -> Self {
        Self::new(DEFAULT_COLS, DEFAULT_ROWS)
    }
}

impl TerminalCore {
    pub fn new(cols: u16, rows: u16) -> Self {
        let cols = usize::from(cols).max(1);
        let rows = usize::from(rows).max(1);
        let proxy = NyaTermEventProxy::default();
        let mut config = Config::default();
        config.scrolling_history = 5_000;
        let size = TermSize { cols, rows };
        Self {
            parser: ansi::Processor::new(),
            term: Term::new(config, &size, proxy.clone()),
            proxy,
            sidecar: NyaTermSidecar::default(),
            pending_effects: TerminalEffects::default(),
            rows,
            cols,
            scrollback_limit: 5_000,
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
    }

    pub fn set_scrollback_limit(&mut self, limit: usize) {
        // Alacritty applies the history size through Config at construction time.
        // Keep this value for future rebuilds and report clamped history to callers.
        self.scrollback_limit = limit;
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
        std::mem::take(&mut self.pending_effects)
    }

    pub fn total_rows(&self) -> usize {
        self.scrollback_len() + self.rows
    }

    pub fn clear(&mut self) {
        let cols = self.cols as u16;
        let rows = self.rows as u16;
        *self = Self::new(cols, rows);
        self.set_scrollback_limit(self.scrollback_limit);
    }

    pub fn advance(&mut self, bytes: &[u8]) {
        self.sidecar.advance(bytes);
        self.parser.advance(&mut self.term, bytes);
        self.drain_alacritty_events();
    }

    pub fn snapshot(&self) -> TerminalSnapshot {
        self.viewport_snapshot(0)
    }

    pub fn viewport_snapshot(&self, offset: usize) -> TerminalSnapshot {
        let max_offset = self.scrollback_len();
        let offset = offset.min(max_offset);
        snapshot_from_term(&self.term, offset)
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
                Event::ClipboardStore(_, _)
                | Event::ClipboardLoad(_, _)
                | Event::ColorRequest(_, _)
                | Event::TextAreaSizeRequest(_)
                | Event::CursorBlinkingChange
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
}

fn snapshot_from_term(term: &Term<NyaTermEventProxy>, requested_offset: usize) -> TerminalSnapshot {
    let content = term.renderable_content();
    let cols = term.columns();
    let rows = term.screen_lines();
    let display_offset = requested_offset;
    let mut row_cells = vec![Vec::<RenderCell>::with_capacity(cols); rows];
    let mut line_timestamps_ms = vec![None; rows];

    let topmost = term.topmost_line();
    let bottommost = term.bottommost_line();
    for row in 0..rows {
        let line = Line(row as i32 - requested_offset as i32);
        if line < topmost || line > bottommost {
            continue;
        }
        for col in 0..cols {
            let cell = &term.grid()[line][Column(col)];
            let text = cell_text(cell);
            row_cells[row].push(RenderCell {
                text,
                style: cell_style(cell),
                width: if cell.flags.contains(Flags::WIDE_CHAR) {
                    2
                } else {
                    1
                },
                hyperlink: cell.hyperlink().map(|link| link.uri().to_string()),
            });
        }
    }

    for row in &mut row_cells {
        while row.len() < cols {
            row.push(RenderCell {
                text: " ".to_string(),
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
        let line = row
            .iter()
            .map(|cell| cell.text.as_str())
            .collect::<String>();
        lines.push(line.trim_end().to_string());
        styled_lines.push(compress_render_row(row));
        hyperlink_lines.push(compress_render_hyperlinks(row));
    }

    let cursor_point = content.cursor.point;
    let cursor_row = if requested_offset == 0 {
        usize::try_from(cursor_point.line.0 + requested_offset as i32).unwrap_or(usize::MAX)
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
        rows,
        cells,
        cursor: CursorSnapshot {
            row: cursor_row,
            col: cursor_col,
            shape: cursor_shape,
            visible: cursor_visible,
        },
        selection,
        lines,
        styled_lines,
        line_timestamps_ms: {
            line_timestamps_ms.resize(rows, None);
            line_timestamps_ms
        },
        hyperlink_lines,
        cursor_row,
        cursor_col,
        scrollback_len,
        total_rows: scrollback_len + rows,
        display_offset,
    }
}

fn cell_text(cell: &Cell) -> String {
    if cell.flags.contains(Flags::WIDE_CHAR_SPACER)
        || cell.flags.contains(Flags::LEADING_WIDE_CHAR_SPACER)
    {
        return " ".to_string();
    }
    let mut text = String::new();
    text.push(cell.c);
    if let Some(zerowidth) = cell.zerowidth() {
        text.extend(zerowidth.iter().copied());
    }
    text
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
            text.push_str(&row[j].text);
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
                self.handle_osc133_mark(mark);
            }
            _ if code.starts_with("133") => {
                let mark = code.chars().nth(3).unwrap_or('\0');
                self.handle_osc133_mark(mark);
            }
            _ => {}
        }
    }

    fn handle_osc133_mark(&mut self, mark: char) {
        match mark {
            'A' | 'B' => {
                self.shell_integration_enabled = true;
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
    if !screen.mouse_reporting() {
        return Vec::new();
    }
    let x = col.saturating_add(1);
    let y = row.saturating_add(1);
    if screen.mouse_sgr() {
        let suffix = if press { 'M' } else { 'm' };
        format!("\x1b[<{button};{x};{y}{suffix}").into_bytes()
    } else {
        let cb = 32u16.saturating_add(u16::from(button)).min(255) as u8;
        let cx = 32u16.saturating_add(x).min(255) as u8;
        let cy = 32u16.saturating_add(y).min(255) as u8;
        vec![0x1b, b'[', b'M', cb, cx, cy]
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
    fn prints_and_wraps_lines() {
        let mut screen = TerminalScreen::new(5, 3);
        screen.advance(b"hello\nworld");
        assert_eq!(screen.lines()[0], "hello");
        assert!(screen.lines().iter().any(|line| line.contains("world")));
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
    fn mouse_reporting_modes_track_decset() {
        let mut screen = TerminalScreen::new(20, 2);
        assert!(!screen.mouse_reporting());
        assert!(!screen.mouse_sgr());
        screen.advance(b"\x1b[?1000h");
        screen.advance(b"\x1b[?1006h");
        assert!(screen.mouse_reporting());
        assert!(screen.mouse_sgr());
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
}
