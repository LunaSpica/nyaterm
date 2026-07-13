use super::*;

pub(super) fn flush_bg(
    pending: Option<(u32, usize, usize)>,
    row: usize,
    bounds: Bounds<Pixels>,
    cell_w: f32,
    cell_h: f32,
    out: &mut Vec<PaintQuad>,
) {
    let Some((bg, start, end)) = pending else {
        return;
    };
    if end <= start {
        return;
    }
    out.push(fill(
        Bounds::new(
            point(
                px(f32::from(bounds.left()) + start as f32 * cell_w),
                px(f32::from(bounds.top()) + row as f32 * cell_h),
            ),
            size(px((end - start) as f32 * cell_w), px(cell_h)),
        ),
        rgb(bg),
    ));
}
pub(super) fn terminal_run_font(
    mut font: Font,
    bold: bool,
    italic: bool,
    normal_weight: f32,
    bold_weight: f32,
) -> Font {
    font.weight = FontWeight(if bold { bold_weight } else { normal_weight });
    font.style = if italic {
        FontStyle::Italic
    } else {
        FontStyle::Normal
    };
    font
}
pub(super) fn line_underline_color(color: Hsla) -> UnderlineStyle {
    UnderlineStyle {
        color: Some(color),
        thickness: px(1.0),
        wavy: false,
    }
}
pub(super) fn line_strike_color(color: Hsla) -> StrikethroughStyle {
    StrikethroughStyle {
        color: Some(color),
        thickness: px(1.0),
    }
}
pub fn terminal_line_element(
    line: &str,
    ansi_spans: Option<&[nyaterm_terminal::StyledSpan]>,
    keyword_rules: &[ResolvedKeywordHighlightRule],
    search_ranges: &[(usize, usize)],
    active_search_ranges: &[(usize, usize)],
    cursor_col: Option<usize>,
    cursor_style: &str,
    // Half-open column range selected on this line, if any.
    selection_cols: Option<(usize, usize)>,
    // Half-open character ranges for action-link underlines.
    link_ranges: &[(usize, usize)],
    line_height: f32,
    palette: nyaterm_ui::ThemePalette,
    bold_weight: f32,
) -> impl IntoElement {
    let mut spans = terminal_highlight_spans(
        line,
        ansi_spans,
        keyword_rules,
        search_ranges,
        active_search_ranges,
        selection_cols,
        link_ranges,
        palette,
    );
    if let Some(col) = cursor_col {
        spans = apply_cursor_style(spans, col, cursor_style, palette);
    }
    let line_h = px(line_height.max(12.));

    let mut row = div()
        .flex()
        .flex_row()
        .items_center()
        .min_h(line_h)
        .line_height(line_h)
        .whitespace_nowrap();
    // Active find row gets a subtle left marker (xterm active decoration cue).
    if !active_search_ranges.is_empty() {
        row = row.border_l_2().border_color(rgb(palette.warning));
    }

    for span in spans {
        let mut child =
            div()
                .line_height(line_h)
                .whitespace_nowrap()
                .child(if span.text.is_empty() {
                    " ".to_string()
                } else {
                    span.text
                });
        if let Some(color) = span.color {
            child = child.text_color(rgb(color));
        }
        if let Some(bg) = span.bg {
            child = child.bg(rgb(bg));
        } else if span.keyword {
            child = child.bg(rgb(palette.surface));
        }
        if span.underline {
            child = child.underline();
        }
        if span.bold {
            child = child.font_weight(FontWeight(bold_weight));
        }
        row = row.child(child);
    }

    row
}
#[allow(clippy::too_many_arguments)]
pub(super) fn terminal_highlight_spans(
    line: &str,
    ansi_spans: Option<&[nyaterm_terminal::StyledSpan]>,
    keyword_rules: &[ResolvedKeywordHighlightRule],
    search_ranges: &[(usize, usize)],
    active_search_ranges: &[(usize, usize)],
    selection_cols: Option<(usize, usize)>,
    link_ranges: &[(usize, usize)],
    palette: nyaterm_ui::ThemePalette,
) -> Vec<TerminalHighlightSpan> {
    let mut spans = if let Some(ansi) = ansi_spans {
        if ansi.is_empty() || (ansi.len() == 1 && ansi[0].text.is_empty()) {
            keyword_highlight_spans(line, keyword_rules)
        } else {
            ansi_to_highlight_spans(ansi, palette, keyword_rules)
        }
    } else {
        keyword_highlight_spans(line, keyword_rules)
    };
    if !link_ranges.is_empty() {
        spans = apply_action_link_ranges(spans, link_ranges, palette);
    }
    if let Some((start, end)) = selection_cols {
        spans = apply_selection_range(spans, start, end, palette);
    }
    if !search_ranges.is_empty() {
        spans = apply_search_ranges(spans, search_ranges, false, palette);
    }
    if !active_search_ranges.is_empty() {
        spans = apply_search_ranges(spans, active_search_ranges, true, palette);
    }
    spans
}
/// Underline action-link ranges with the accent color (Tauri decoration look).
pub(super) fn apply_action_link_ranges(
    spans: Vec<TerminalHighlightSpan>,
    ranges: &[(usize, usize)],
    palette: nyaterm_ui::ThemePalette,
) -> Vec<TerminalHighlightSpan> {
    if ranges.is_empty() {
        return spans;
    }
    let mut flat: Vec<FlatTerminalCell> = Vec::new();
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        for ch in span.text.chars() {
            flat.push(FlatTerminalCell {
                ch,
                color: span.color,
                bg: span.bg,
                keyword: span.keyword,
                underline: span.underline,
                strikeout: span.strikeout,
                bold: span.bold,
                italic: span.italic,
            });
        }
    }
    for &(start, end) in ranges {
        if start >= end {
            continue;
        }
        let end = end.min(flat.len());
        let start = start.min(end);
        for idx in start..end {
            if let Some(cell) = flat.get_mut(idx) {
                cell.underline = true;
                if cell.color.is_none() {
                    cell.color = Some(palette.accent);
                }
            }
        }
    }
    compress_flat_cells(flat)
}
/// Highlight a half-open [start, end) column range with the theme selection background.
pub(super) fn apply_selection_range(
    spans: Vec<TerminalHighlightSpan>,
    start: usize,
    end: usize,
    palette: nyaterm_ui::ThemePalette,
) -> Vec<TerminalHighlightSpan> {
    if start >= end {
        return spans;
    }
    let mut flat: Vec<FlatTerminalCell> = Vec::new();
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        for ch in span.text.chars() {
            flat.push(FlatTerminalCell {
                ch,
                color: span.color,
                bg: span.bg,
                keyword: span.keyword,
                underline: span.underline,
                strikeout: span.strikeout,
                bold: span.bold,
                italic: span.italic,
            });
        }
    }
    while flat.len() < end {
        flat.push(FlatTerminalCell::blank());
    }
    let end = end.min(flat.len());
    for idx in start..end {
        if let Some(cell) = flat.get_mut(idx) {
            cell.bg = Some(palette.terminal_selection);
            cell.keyword = false;
        }
    }
    compress_flat_cells(flat)
}
pub(super) fn apply_search_ranges(
    spans: Vec<TerminalHighlightSpan>,
    ranges: &[(usize, usize)],
    active: bool,
    palette: nyaterm_ui::ThemePalette,
) -> Vec<TerminalHighlightSpan> {
    if ranges.is_empty() {
        return spans;
    }
    let mut flat: Vec<FlatTerminalCell> = Vec::new();
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        for ch in span.text.chars() {
            flat.push(FlatTerminalCell {
                ch,
                color: span.color,
                bg: span.bg,
                keyword: span.keyword,
                underline: span.underline,
                strikeout: span.strikeout,
                bold: span.bold,
                italic: span.italic,
            });
        }
    }
    let max_end = ranges.iter().map(|(_, end)| *end).max().unwrap_or(0);
    while flat.len() < max_end {
        flat.push(FlatTerminalCell::blank());
    }
    // Tauri xterm find decorations: inactive selection-ish, active stronger accent.
    let bg = if active {
        // Mix selection with warning accent by using warning-ish selection.
        palette.warning
    } else {
        palette.terminal_selection
    };
    let fg = if active {
        Some(palette.terminal_bg)
    } else {
        None
    };
    for &(start, end) in ranges {
        if start >= end {
            continue;
        }
        let end = end.min(flat.len());
        for idx in start..end {
            if let Some(cell) = flat.get_mut(idx) {
                cell.bg = Some(bg);
                if let Some(fg) = fg {
                    cell.color = Some(fg);
                }
                cell.keyword = false;
            }
        }
    }
    compress_flat_cells(flat)
}
#[derive(Debug, Clone, Copy)]
pub(super) struct FlatTerminalCell {
    pub(super) ch: char,
    pub(super) color: Option<u32>,
    pub(super) bg: Option<u32>,
    pub(super) keyword: bool,
    pub(super) underline: bool,
    pub(super) strikeout: bool,
    pub(super) bold: bool,
    pub(super) italic: bool,
}

impl FlatTerminalCell {
    fn blank() -> Self {
        Self {
            ch: ' ',
            color: None,
            bg: None,
            keyword: false,
            underline: false,
            strikeout: false,
            bold: false,
            italic: false,
        }
    }
}
pub(super) fn compress_flat_cells(flat: Vec<FlatTerminalCell>) -> Vec<TerminalHighlightSpan> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < flat.len() {
        let cell = flat[i];
        let mut text = String::new();
        text.push(cell.ch);
        let mut j = i + 1;
        while j < flat.len() {
            let next = flat[j];
            if next.color == cell.color
                && next.bg == cell.bg
                && next.keyword == cell.keyword
                && next.underline == cell.underline
                && next.strikeout == cell.strikeout
                && next.bold == cell.bold
                && next.italic == cell.italic
            {
                text.push(next.ch);
                j += 1;
            } else {
                break;
            }
        }
        out.push(TerminalHighlightSpan {
            text,
            color: cell.color,
            bg: cell.bg,
            keyword: cell.keyword,
            underline: cell.underline,
            strikeout: cell.strikeout,
            bold: cell.bold,
            italic: cell.italic,
        });
        i = j;
    }
    out
}
/// Paint a caret at `cursor_col` (char index) using Tauri cursor styles.
pub(super) fn apply_cursor_style(
    spans: Vec<TerminalHighlightSpan>,
    cursor_col: usize,
    cursor_style: &str,
    palette: nyaterm_ui::ThemePalette,
) -> Vec<TerminalHighlightSpan> {
    let mut flat: Vec<FlatTerminalCell> = Vec::new();
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        for ch in span.text.chars() {
            flat.push(FlatTerminalCell {
                ch,
                color: span.color,
                bg: span.bg,
                keyword: span.keyword,
                underline: span.underline,
                strikeout: span.strikeout,
                bold: span.bold,
                italic: span.italic,
            });
        }
    }
    // Ensure the cursor column exists even on a short/empty line.
    while flat.len() <= cursor_col {
        flat.push(FlatTerminalCell::blank());
    }
    if let Some(cell) = flat.get_mut(cursor_col) {
        match cursor_style {
            "underline" => {
                // Approximate underline caret: keep glyph, tint with cursor color and dim cell bg.
                if cell.color.is_none() {
                    cell.color = Some(palette.terminal_cursor);
                }
                cell.bg = Some(palette.terminal_selection);
                cell.keyword = false;
            }
            "bar" => {
                // Approximate bar caret: thin visual via inverted narrow space marker.
                cell.ch = '|';
                cell.color = Some(palette.terminal_cursor);
                cell.bg = None;
                cell.keyword = false;
            }
            _ => {
                // Block cursor: invert with theme cursor color (Tauri xterm cursor).
                cell.color = Some(palette.terminal_bg);
                cell.bg = Some(palette.terminal_cursor);
                cell.keyword = false;
            }
        }
    }

    compress_flat_cells(flat)
}
