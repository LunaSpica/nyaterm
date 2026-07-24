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
    let left = (f32::from(bounds.left()) + start as f32 * cell_w).floor();
    let top = (f32::from(bounds.top()) + row as f32 * cell_h).floor();
    let right = (f32::from(bounds.left()) + end as f32 * cell_w).ceil();
    let bottom = (f32::from(bounds.top()) + (row + 1) as f32 * cell_h).ceil();
    out.push(fill(
        Bounds::new(
            point(px(left), px(top)),
            size(px((right - left).max(0.)), px((bottom - top).max(0.))),
        ),
        rgb(bg),
    ));
}
/// Solid background over a half-open [start, end) column range on a viewport row.
pub(super) fn push_col_range_bg(
    row: usize,
    start: usize,
    end: usize,
    color: u32,
    bounds: Bounds<Pixels>,
    cell_w: f32,
    cell_h: f32,
    out: &mut Vec<PaintQuad>,
) {
    if end <= start {
        return;
    }
    flush_bg(Some((color, start, end)), row, bounds, cell_w, cell_h, out);
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
    let compiled = compile_keyword_rules(keyword_rules);
    terminal_highlight_spans_compiled(
        line,
        ansi_spans,
        &compiled,
        search_ranges,
        active_search_ranges,
        selection_cols,
        link_ranges,
        palette,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn terminal_highlight_spans_compiled(
    line: &str,
    ansi_spans: Option<&[nyaterm_terminal::StyledSpan]>,
    compiled_keyword_rules: &[(regex::Regex, u32)],
    search_ranges: &[(usize, usize)],
    active_search_ranges: &[(usize, usize)],
    selection_cols: Option<(usize, usize)>,
    link_ranges: &[(usize, usize)],
    palette: nyaterm_ui::ThemePalette,
) -> Vec<TerminalHighlightSpan> {
    let mut spans = if let Some(ansi) = ansi_spans {
        if ansi.is_empty() || (ansi.len() == 1 && ansi[0].text.is_empty()) {
            keyword_highlight_spans_compiled(line, compiled_keyword_rules)
        } else {
            ansi_to_highlight_spans_compiled(ansi, palette, compiled_keyword_rules)
        }
    } else {
        keyword_highlight_spans_compiled(line, compiled_keyword_rules)
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
    let mut flat = flatten_highlight_spans(spans);
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
    let mut flat = flatten_highlight_spans(spans);
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
    let mut flat = flatten_highlight_spans(spans);
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

pub(super) fn terminal_highlight_spans_with_keyword_ranges(
    line: &str,
    ansi_spans: Option<&[nyaterm_terminal::StyledSpan]>,
    keyword_ranges: Option<&[TerminalKeywordRange]>,
    palette: nyaterm_ui::ThemePalette,
) -> Vec<TerminalHighlightSpan> {
    let mut flat = flatten_base_cells_with_keyword_permissions(line, ansi_spans, palette);
    if let Some(ranges) = keyword_ranges {
        for range in ranges {
            if range.start_col >= range.end_col {
                continue;
            }
            let end = range.end_col.min(flat.len());
            let start = range.start_col.min(end);
            for cell in &mut flat[start..end] {
                if !cell.allow_keyword {
                    continue;
                }
                cell.cell.color = Some(range.color);
                cell.cell.keyword = true;
            }
        }
    }
    compress_flat_cells(flat.into_iter().map(|cell| cell.cell).collect())
}

struct KeywordPermissionCell {
    cell: FlatTerminalCell,
    allow_keyword: bool,
}

fn flatten_base_cells_with_keyword_permissions(
    line: &str,
    ansi_spans: Option<&[nyaterm_terminal::StyledSpan]>,
    palette: nyaterm_ui::ThemePalette,
) -> Vec<KeywordPermissionCell> {
    let Some(ansi) = ansi_spans else {
        return flatten_plain_cells_with_keyword_permissions(line);
    };
    if ansi.is_empty() || (ansi.len() == 1 && ansi[0].text.is_empty()) {
        return flatten_plain_cells_with_keyword_permissions(line);
    }

    let mut flat = Vec::new();
    for span in ansi {
        if span.text.is_empty() {
            continue;
        }
        let bg = palette.resolve_cell_bg(span.style);
        let color = if span.style.hidden {
            bg.unwrap_or(palette.terminal_bg)
        } else {
            palette.resolve_cell_fg(span.style)
        };
        let allow_keyword =
            !span.style.hidden && span.style.fg.is_none() && span.style.fg_rgb.is_none();
        let highlight = TerminalHighlightSpan {
            text: String::new(),
            color: Some(color),
            bg,
            keyword: false,
            underline: span.style.underline,
            strikeout: span.style.strikeout,
            bold: span.style.bold,
            italic: span.style.italic,
        };
        push_permission_cells_for_text(&mut flat, &highlight, &span.text, allow_keyword);
    }
    if flat.is_empty() {
        return flatten_plain_cells_with_keyword_permissions(line);
    }
    flat
}

fn flatten_plain_cells_with_keyword_permissions(line: &str) -> Vec<KeywordPermissionCell> {
    let highlight = TerminalHighlightSpan {
        text: String::new(),
        color: None,
        bg: None,
        keyword: false,
        underline: false,
        strikeout: false,
        bold: false,
        italic: false,
    };
    let text = if line.is_empty() { " " } else { line };
    let mut flat = Vec::new();
    push_permission_cells_for_text(&mut flat, &highlight, text, true);
    flat
}

fn push_permission_cells_for_text(
    flat: &mut Vec<KeywordPermissionCell>,
    span: &TerminalHighlightSpan,
    text: &str,
    allow_keyword: bool,
) {
    for ch in text.chars() {
        if terminal_is_zero_width_mark(ch)
            && let Some(previous) = flat.last_mut()
        {
            previous.cell.text.push(ch);
            continue;
        }
        flat.push(KeywordPermissionCell {
            cell: FlatTerminalCell::from_span_char(span, ch),
            allow_keyword,
        });
        for _ in 1..terminal_char_cell_width(ch) {
            let mut spacer = FlatTerminalCell::from_span_char(span, ' ');
            spacer.text.clear();
            flat.push(KeywordPermissionCell {
                cell: spacer,
                allow_keyword,
            });
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct FlatTerminalCell {
    pub(super) text: String,
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
            text: " ".to_string(),
            color: None,
            bg: None,
            keyword: false,
            underline: false,
            strikeout: false,
            bold: false,
            italic: false,
        }
    }

    fn from_span_char(span: &TerminalHighlightSpan, ch: char) -> Self {
        Self {
            text: ch.to_string(),
            color: span.color,
            bg: span.bg,
            keyword: span.keyword,
            underline: span.underline,
            strikeout: span.strikeout,
            bold: span.bold,
            italic: span.italic,
        }
    }
}

fn flatten_highlight_spans(spans: Vec<TerminalHighlightSpan>) -> Vec<FlatTerminalCell> {
    let mut flat: Vec<FlatTerminalCell> = Vec::new();
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        for ch in span.text.chars() {
            if terminal_is_zero_width_mark(ch)
                && let Some(previous) = flat.last_mut()
            {
                previous.text.push(ch);
                continue;
            }
            flat.push(FlatTerminalCell::from_span_char(&span, ch));
            for _ in 1..terminal_char_cell_width(ch) {
                let mut spacer = FlatTerminalCell::from_span_char(&span, ' ');
                spacer.text.clear();
                flat.push(spacer);
            }
        }
    }
    flat
}

pub(super) fn compress_flat_cells(flat: Vec<FlatTerminalCell>) -> Vec<TerminalHighlightSpan> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < flat.len() {
        let cell = flat[i].clone();
        let mut text = cell.text.clone();
        let mut j = i + 1;
        while j < flat.len() {
            let next = &flat[j];
            if next.color == cell.color
                && next.bg == cell.bg
                && next.keyword == cell.keyword
                && next.underline == cell.underline
                && next.strikeout == cell.strikeout
                && next.bold == cell.bold
                && next.italic == cell.italic
            {
                text.push_str(&next.text);
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

pub(super) fn terminal_cell_text_at_col(line: &str, col: usize) -> String {
    let mut cell_col = 0usize;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if terminal_is_zero_width_mark(ch) {
            if col == 0 {
                let mut text = String::new();
                text.push(ch);
                return text;
            }
            continue;
        }

        let width = terminal_char_cell_width(ch).max(1);
        let mut text = String::new();
        text.push(ch);
        while let Some(next) = chars.peek().copied() {
            if !terminal_is_zero_width_mark(next) {
                break;
            }
            text.push(next);
            chars.next();
        }

        if col >= cell_col && col < cell_col + width {
            return text;
        }
        cell_col += width;
    }

    " ".to_string()
}

/// Paint a caret at `cursor_col` (char index) using Tauri cursor styles.
pub(super) fn apply_cursor_style(
    spans: Vec<TerminalHighlightSpan>,
    cursor_col: usize,
    cursor_style: &str,
    palette: nyaterm_ui::ThemePalette,
) -> Vec<TerminalHighlightSpan> {
    let mut flat = flatten_highlight_spans(spans);
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
                cell.text = "|".to_string();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn plain_span(text: &str) -> TerminalHighlightSpan {
        TerminalHighlightSpan {
            text: text.to_string(),
            color: None,
            bg: None,
            keyword: false,
            underline: false,
            strikeout: false,
            bold: false,
            italic: false,
        }
    }

    fn styled_span(
        text: &str,
        style: nyaterm_terminal::CellStyle,
    ) -> Vec<nyaterm_terminal::StyledSpan> {
        vec![nyaterm_terminal::StyledSpan {
            text: text.to_string(),
            style,
        }]
    }

    #[test]
    fn keyword_ranges_apply_to_columns_after_wide_chars() {
        let palette = nyaterm_ui::theme_palette("github-dark");
        let spans = terminal_highlight_spans_with_keyword_ranges(
            "界ERROR",
            None,
            Some(&[TerminalKeywordRange {
                start_col: 2,
                end_col: 7,
                color: 0xff2244,
            }]),
            palette,
        );

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "界");
        assert!(!spans[0].keyword);
        assert_eq!(spans[1].text, "ERROR");
        assert!(spans[1].keyword);
        assert_eq!(spans[1].color, Some(0xff2244));
    }

    #[test]
    fn keyword_ranges_preserve_ansi_underline() {
        let palette = nyaterm_ui::theme_palette("github-dark");
        let style = nyaterm_terminal::CellStyle {
            underline: true,
            ..nyaterm_terminal::CellStyle::default()
        };
        let spans = terminal_highlight_spans_with_keyword_ranges(
            "ERROR",
            Some(&styled_span("ERROR", style)),
            Some(&[TerminalKeywordRange {
                start_col: 0,
                end_col: 5,
                color: 0xff2244,
            }]),
            palette,
        );

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "ERROR");
        assert!(spans[0].keyword);
        assert!(spans[0].underline);
        assert_eq!(spans[0].color, Some(0xff2244));
    }

    #[test]
    fn keyword_ranges_do_not_override_explicit_or_hidden_ansi_text() {
        let palette = nyaterm_ui::theme_palette("github-dark");
        let explicit = nyaterm_terminal::CellStyle {
            fg_rgb: Some(0x112233),
            ..nyaterm_terminal::CellStyle::default()
        };
        let explicit_spans = terminal_highlight_spans_with_keyword_ranges(
            "ERROR",
            Some(&styled_span("ERROR", explicit)),
            Some(&[TerminalKeywordRange {
                start_col: 0,
                end_col: 5,
                color: 0xff2244,
            }]),
            palette,
        );
        assert_eq!(explicit_spans[0].color, Some(0x112233));
        assert!(!explicit_spans[0].keyword);

        let hidden = nyaterm_terminal::CellStyle {
            bg_rgb: Some(0x445566),
            hidden: true,
            ..nyaterm_terminal::CellStyle::default()
        };
        let hidden_spans = terminal_highlight_spans_with_keyword_ranges(
            "secret",
            Some(&styled_span("secret", hidden)),
            Some(&[TerminalKeywordRange {
                start_col: 0,
                end_col: 6,
                color: 0xff2244,
            }]),
            palette,
        );
        assert_eq!(hidden_spans[0].color, Some(0x445566));
        assert_eq!(hidden_spans[0].bg, Some(0x445566));
        assert!(!hidden_spans[0].keyword);
    }

    #[test]
    fn selection_columns_treat_combining_mark_as_same_cell() {
        let palette = nyaterm_ui::theme_palette("github-dark");
        let spans = apply_selection_range(vec![plain_span("e\u{301}x")], 1, 2, palette);

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "e\u{301}");
        assert_eq!(spans[0].bg, None);
        assert_eq!(spans[1].text, "x");
        assert_eq!(spans[1].bg, Some(palette.terminal_selection));
    }

    #[test]
    fn cursor_columns_treat_combining_mark_as_same_cell() {
        let palette = nyaterm_ui::theme_palette("github-dark");
        let spans = apply_cursor_style(vec![plain_span("e\u{301}x")], 1, "block", palette);

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "e\u{301}");
        assert_eq!(spans[0].bg, None);
        assert_eq!(spans[1].text, "x");
        assert_eq!(spans[1].bg, Some(palette.terminal_cursor));
    }

    #[test]
    fn action_link_columns_treat_combining_mark_as_same_cell() {
        let palette = nyaterm_ui::theme_palette("github-dark");
        let spans = apply_action_link_ranges(vec![plain_span("e\u{301}x")], &[(1, 2)], palette);

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "e\u{301}");
        assert!(!spans[0].underline);
        assert_eq!(spans[1].text, "x");
        assert!(spans[1].underline);
    }

    #[test]
    fn action_link_columns_count_wide_chars_as_two_cells() {
        let palette = nyaterm_ui::theme_palette("github-dark");
        let spans = apply_action_link_ranges(vec![plain_span("界x")], &[(2, 3)], palette);

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "界");
        assert!(!spans[0].underline);
        assert_eq!(spans[1].text, "x");
        assert!(spans[1].underline);
    }

    #[test]
    fn terminal_cell_count_keeps_combining_mark_with_previous_cell() {
        assert_eq!(terminal_cell_count("e\u{301}x"), 2);
        assert_eq!(terminal_cell_count("\u{301}x"), 2);
        assert_eq!(terminal_cell_count(""), 0);
    }

    #[test]
    fn terminal_cell_count_treats_wide_char_as_two_cells() {
        assert_eq!(terminal_cell_count("界x"), 3);
        assert_eq!(terminal_cell_count("界\u{301}x"), 3);
    }

    #[test]
    fn terminal_cell_count_keeps_variation_selector_with_previous_cell() {
        assert!(terminal_is_zero_width_mark('\u{fe0f}'));
        assert_eq!(terminal_cell_count("a\u{fe0f}x"), 2);
    }

    #[test]
    fn terminal_cell_col_for_byte_index_keeps_combining_mark_with_previous_cell() {
        let text = "e\u{301}x";

        assert_eq!(terminal_cell_col_for_byte_index(text, 0), 0);
        assert_eq!(terminal_cell_col_for_byte_index(text, "e".len()), 1);
        assert_eq!(terminal_cell_col_for_byte_index(text, "e\u{301}".len()), 1);
        assert_eq!(terminal_cell_col_for_byte_index(text, text.len()), 2);
    }

    #[test]
    fn terminal_cell_col_for_byte_index_counts_wide_char_columns() {
        let text = "界x";

        assert_eq!(terminal_cell_col_for_byte_index(text, 0), 0);
        assert_eq!(terminal_cell_col_for_byte_index(text, "界".len()), 2);
        assert_eq!(terminal_cell_col_for_byte_index(text, text.len()), 3);
    }

    #[test]
    fn terminal_cell_col_for_byte_index_keeps_variation_selector_with_previous_cell() {
        let text = "a\u{fe0f}x";

        assert_eq!(terminal_cell_col_for_byte_index(text, 0), 0);
        assert_eq!(terminal_cell_col_for_byte_index(text, "a".len()), 1);
        assert_eq!(terminal_cell_col_for_byte_index(text, "a\u{fe0f}".len()), 1);
        assert_eq!(terminal_cell_col_for_byte_index(text, text.len()), 2);
    }

    #[test]
    fn terminal_byte_index_for_cell_col_skips_attached_combining_marks() {
        let text = "e\u{301}x";

        assert_eq!(terminal_byte_index_for_cell_col(text, 0), 0);
        assert_eq!(terminal_byte_index_for_cell_col(text, 1), "e\u{301}".len());
        assert_eq!(terminal_byte_index_for_cell_col(text, 2), text.len());
        assert_eq!(terminal_byte_index_for_cell_col(text, 99), text.len());
    }

    #[test]
    fn terminal_byte_index_for_cell_col_maps_wide_char_spacer_to_base() {
        let text = "界x";

        assert_eq!(terminal_byte_index_for_cell_col(text, 0), 0);
        assert_eq!(terminal_byte_index_for_cell_col(text, 1), 0);
        assert_eq!(terminal_byte_index_for_cell_col(text, 2), "界".len());
        assert_eq!(terminal_byte_index_for_cell_col(text, 3), text.len());
    }

    #[test]
    fn terminal_byte_index_for_cell_col_skips_attached_variation_selector() {
        let text = "a\u{fe0f}x";

        assert_eq!(terminal_byte_index_for_cell_col(text, 0), 0);
        assert_eq!(terminal_byte_index_for_cell_col(text, 1), "a\u{fe0f}".len());
        assert_eq!(terminal_byte_index_for_cell_col(text, 2), text.len());
    }

    #[test]
    fn terminal_cell_text_at_col_keeps_combining_marks() {
        assert_eq!(terminal_cell_text_at_col("e\u{301}x", 0), "e\u{301}");
        assert_eq!(terminal_cell_text_at_col("e\u{301}x", 1), "x");
    }

    #[test]
    fn terminal_cell_text_at_col_maps_wide_spacer_to_base_glyph() {
        assert_eq!(terminal_cell_text_at_col("界x", 0), "界");
        assert_eq!(terminal_cell_text_at_col("界x", 1), "界");
        assert_eq!(terminal_cell_text_at_col("界x", 2), "x");
        assert_eq!(terminal_cell_text_at_col("界x", 3), " ");
    }
}
