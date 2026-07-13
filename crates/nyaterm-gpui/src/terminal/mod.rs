use gpui::{
    App, Bounds, Element, ElementId, Font, FontStyle, FontWeight, GlobalElementId, Hsla,
    InspectorElementId, IntoElement, KeyDownEvent, LayoutId, PaintQuad, Pixels, ShapedLine,
    SharedString, StrikethroughStyle, Style, TextRun, UnderlineStyle, Window, div, fill, font,
    point, prelude::*, px, relative, rgb, size,
};
use nyaterm_core::ResolvedKeywordHighlightRule;
use nyaterm_terminal::{TerminalScreen, TerminalSnapshot};

use super::view::INITIAL_TERMINAL_BANNER;

#[derive(Debug, Clone)]
pub(super) struct TerminalBufferMatch {
    pub(super) line_index: usize,
    /// Half-open character column range on the matched line.
    pub(super) start_col: usize,
    pub(super) end_col: usize,
}

#[derive(Debug, Clone)]
pub(super) struct TerminalSearchFlags {
    pub(super) case_sensitive: bool,
    pub(super) regex: bool,
    pub(super) whole_word: bool,
}

struct TerminalHighlightSpan {
    text: String,
    color: Option<u32>,
    bg: Option<u32>,
    keyword: bool,
    underline: bool,
    strikeout: bool,
    bold: bool,
    italic: bool,
}

#[derive(Debug, Clone, Default)]
pub(super) struct TerminalLineDecorations {
    pub(super) search_ranges: Vec<(usize, usize)>,
    pub(super) active_search_ranges: Vec<(usize, usize)>,
    pub(super) selection_cols: Option<(usize, usize)>,
    pub(super) link_ranges: Vec<(usize, usize)>,
}

pub(super) struct NyaTerminalElement {
    snapshot: TerminalSnapshot,
    keyword_rules: Vec<ResolvedKeywordHighlightRule>,
    decorations: Vec<TerminalLineDecorations>,
    show_cursor: bool,
    cursor_style: String,
    cell_width: f32,
    cell_height: f32,
    palette: crate::ui::theme::ThemePalette,
    font_family: String,
    font_size: f32,
    normal_weight: f32,
    bold_weight: f32,
}

struct TerminalPaintRow {
    y: Pixels,
    line: ShapedLine,
}

#[derive(Default)]
pub(super) struct NyaTerminalPaintPlan {
    backgrounds: Vec<PaintQuad>,
    active_markers: Vec<PaintQuad>,
    rows: Vec<TerminalPaintRow>,
    cursor: Option<PaintQuad>,
}

impl NyaTerminalElement {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        snapshot: TerminalSnapshot,
        keyword_rules: Vec<ResolvedKeywordHighlightRule>,
        decorations: Vec<TerminalLineDecorations>,
        show_cursor: bool,
        cursor_style: impl Into<String>,
        cell_width: f32,
        cell_height: f32,
        palette: crate::ui::theme::ThemePalette,
        font_family: String,
        font_size: f32,
        normal_weight: f32,
        bold_weight: f32,
    ) -> Self {
        Self {
            snapshot,
            keyword_rules,
            decorations,
            show_cursor,
            cursor_style: cursor_style.into(),
            cell_width,
            cell_height,
            palette,
            font_family,
            font_size,
            normal_weight,
            bold_weight,
        }
    }
}

impl IntoElement for NyaTerminalElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for NyaTerminalElement {
    type RequestLayoutState = ();
    type PrepaintState = NyaTerminalPaintPlan;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = px(self.cell_height * self.snapshot.rows.max(1) as f32).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        let mut plan = NyaTerminalPaintPlan::default();
        let cell_w = self.cell_width.max(1.);
        let cell_h = self.cell_height.max(1.);
        let font_size = px(self.font_size.max(8.));
        let base_font = font(SharedString::from(self.font_family.clone()));

        for row in 0..self.snapshot.rows {
            let line = self
                .snapshot
                .lines
                .get(row)
                .map(String::as_str)
                .unwrap_or("");
            let display_line = if line.is_empty() { " " } else { line };
            let ansi = self.snapshot.styled_lines.get(row).map(Vec::as_slice);
            let decorations = self.decorations.get(row).cloned().unwrap_or_default();
            let spans = terminal_highlight_spans(
                display_line,
                ansi,
                &self.keyword_rules,
                &decorations.search_ranges,
                &decorations.active_search_ranges,
                decorations.selection_cols,
                &decorations.link_ranges,
                self.palette,
            );
            let y = px(f32::from(bounds.top()) + row as f32 * cell_h);

            if !decorations.active_search_ranges.is_empty() {
                plan.active_markers.push(fill(
                    Bounds::new(point(bounds.left(), y), size(px(2.), px(cell_h))),
                    rgb(self.palette.warning),
                ));
            }

            let mut text = String::new();
            let mut text_runs = Vec::new();
            let mut col = 0usize;
            let mut pending_bg: Option<(u32, usize, usize)> = None;
            for span in spans {
                let bg = span
                    .bg
                    .or_else(|| span.keyword.then_some(self.palette.surface));
                let span_cols = span.text.chars().count().max(1);
                if let Some(bg) = bg {
                    match pending_bg.as_mut() {
                        Some((current_bg, _start, end)) if *current_bg == bg && *end == col => {
                            *end = col + span_cols;
                        }
                        _ => {
                            flush_bg(
                                pending_bg.take(),
                                row,
                                bounds,
                                cell_w,
                                cell_h,
                                &mut plan.backgrounds,
                            );
                            pending_bg = Some((bg, col, col + span_cols));
                        }
                    }
                } else {
                    flush_bg(
                        pending_bg.take(),
                        row,
                        bounds,
                        cell_w,
                        cell_h,
                        &mut plan.backgrounds,
                    );
                }

                let run_len = span.text.len();
                text.push_str(&span.text);
                if run_len > 0 {
                    text_runs.push(TextRun {
                        len: run_len,
                        font: terminal_run_font(
                            base_font.clone(),
                            span.bold,
                            span.italic,
                            self.normal_weight,
                            self.bold_weight,
                        ),
                        color: span
                            .color
                            .map(rgb)
                            .unwrap_or_else(|| rgb(self.palette.terminal_fg))
                            .into(),
                        background_color: None,
                        underline: span.underline.then(|| {
                            line_underline_color(
                                span.color
                                    .map(rgb)
                                    .unwrap_or_else(|| rgb(self.palette.accent))
                                    .into(),
                            )
                        }),
                        strikethrough: span.strikeout.then(|| {
                            line_strike_color(
                                span.color
                                    .map(rgb)
                                    .unwrap_or_else(|| rgb(self.palette.terminal_fg))
                                    .into(),
                            )
                        }),
                    });
                }
                col += span_cols;
            }
            flush_bg(
                pending_bg.take(),
                row,
                bounds,
                cell_w,
                cell_h,
                &mut plan.backgrounds,
            );

            if text.is_empty() {
                text.push(' ');
                text_runs.push(TextRun {
                    len: 1,
                    font: terminal_run_font(
                        base_font.clone(),
                        false,
                        false,
                        self.normal_weight,
                        self.bold_weight,
                    ),
                    color: rgb(self.palette.terminal_fg).into(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                });
            }
            let shaped = window.text_system().shape_line(
                SharedString::from(text),
                font_size,
                &text_runs,
                None,
            );
            plan.rows.push(TerminalPaintRow { y, line: shaped });
        }

        if self.show_cursor
            && self.snapshot.cursor_row < self.snapshot.rows
            && self.snapshot.cursor_col < self.snapshot.cols.max(1)
        {
            let x = px(f32::from(bounds.left()) + self.snapshot.cursor_col as f32 * cell_w);
            let y = px(f32::from(bounds.top()) + self.snapshot.cursor_row as f32 * cell_h);
            let cursor_bounds = match self.cursor_style.as_str() {
                "bar" => Bounds::new(point(x, y), size(px(2.), px(cell_h))),
                "underline" => Bounds::new(
                    point(x, px(f32::from(y) + cell_h - 2.)),
                    size(px(cell_w), px(2.)),
                ),
                _ => Bounds::new(point(x, y), size(px(cell_w), px(cell_h))),
            };
            plan.cursor = Some(fill(cursor_bounds, rgb(self.palette.terminal_cursor)));
        }

        plan
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        for quad in prepaint.backgrounds.drain(..) {
            window.paint_quad(quad);
        }
        for row in prepaint.rows.drain(..) {
            let _ = row.line.paint(
                point(bounds.left(), row.y),
                px(self.cell_height.max(1.)),
                window,
                cx,
            );
        }
        for quad in prepaint.active_markers.drain(..) {
            window.paint_quad(quad);
        }
        if let Some(cursor) = prepaint.cursor.take() {
            window.paint_quad(cursor);
        }
    }
}

fn flush_bg(
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

fn terminal_run_font(
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

fn line_underline_color(color: Hsla) -> UnderlineStyle {
    UnderlineStyle {
        color: Some(color),
        thickness: px(1.0),
        wavy: false,
    }
}

fn line_strike_color(color: Hsla) -> StrikethroughStyle {
    StrikethroughStyle {
        color: Some(color),
        thickness: px(1.0),
    }
}

pub(super) fn terminal_line_element(
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
    palette: crate::ui::theme::ThemePalette,
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
fn terminal_highlight_spans(
    line: &str,
    ansi_spans: Option<&[nyaterm_terminal::StyledSpan]>,
    keyword_rules: &[ResolvedKeywordHighlightRule],
    search_ranges: &[(usize, usize)],
    active_search_ranges: &[(usize, usize)],
    selection_cols: Option<(usize, usize)>,
    link_ranges: &[(usize, usize)],
    palette: crate::ui::theme::ThemePalette,
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
fn apply_action_link_ranges(
    spans: Vec<TerminalHighlightSpan>,
    ranges: &[(usize, usize)],
    palette: crate::ui::theme::ThemePalette,
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
fn apply_selection_range(
    spans: Vec<TerminalHighlightSpan>,
    start: usize,
    end: usize,
    palette: crate::ui::theme::ThemePalette,
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

fn apply_search_ranges(
    spans: Vec<TerminalHighlightSpan>,
    ranges: &[(usize, usize)],
    active: bool,
    palette: crate::ui::theme::ThemePalette,
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
struct FlatTerminalCell {
    ch: char,
    color: Option<u32>,
    bg: Option<u32>,
    keyword: bool,
    underline: bool,
    strikeout: bool,
    bold: bool,
    italic: bool,
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

fn compress_flat_cells(flat: Vec<FlatTerminalCell>) -> Vec<TerminalHighlightSpan> {
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
fn apply_cursor_style(
    spans: Vec<TerminalHighlightSpan>,
    cursor_col: usize,
    cursor_style: &str,
    palette: crate::ui::theme::ThemePalette,
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

fn ansi_to_highlight_spans(
    ansi: &[nyaterm_terminal::StyledSpan],
    palette: crate::ui::theme::ThemePalette,
    keyword_rules: &[ResolvedKeywordHighlightRule],
) -> Vec<TerminalHighlightSpan> {
    // Build plain line for keyword overlay, then prefer keyword fg over default ANSI fg.
    let line: String = ansi.iter().map(|s| s.text.as_str()).collect();
    let keyword = keyword_highlight_spans(&line, keyword_rules);
    if keyword.iter().all(|s| !s.keyword) {
        return ansi
            .iter()
            .filter(|s| !s.text.is_empty())
            .map(|s| TerminalHighlightSpan {
                text: s.text.clone(),
                color: Some(palette.resolve_cell_fg(s.style)),
                bg: palette.resolve_cell_bg(s.style),
                keyword: false,
                underline: s.style.underline,
                strikeout: s.style.strikeout,
                bold: s.style.bold,
                italic: s.style.italic,
            })
            .collect();
    }

    // Flatten keyword color map by byte offset, then re-slice per ANSI span.
    let mut keyword_color_at = vec![None; line.len()];
    let mut offset = 0usize;
    for span in &keyword {
        let end = offset + span.text.len();
        if span.keyword {
            for idx in offset..end.min(keyword_color_at.len()) {
                keyword_color_at[idx] = span.color;
            }
        }
        offset = end;
    }

    let mut out = Vec::new();
    let mut cursor = 0usize;
    for s in ansi {
        if s.text.is_empty() {
            continue;
        }
        let start = cursor;
        let end = cursor + s.text.len();
        cursor = end;
        let bg = palette.resolve_cell_bg(s.style);
        let mut color = palette.resolve_cell_fg(s.style);
        let mut keyword_hit = false;
        if s.style.fg.is_none() {
            if let Some(kc) = keyword_color_at.get(start).copied().flatten() {
                color = kc;
                keyword_hit = true;
            }
        }
        out.push(TerminalHighlightSpan {
            text: s.text.clone(),
            color: Some(color),
            bg,
            keyword: keyword_hit,
            underline: s.style.underline,
            strikeout: s.style.strikeout,
            bold: s.style.bold,
            italic: s.style.italic,
        });
    }
    if out.is_empty() {
        out.push(TerminalHighlightSpan {
            text: " ".to_string(),
            color: None,
            bg: None,
            keyword: false,
            underline: false,
            strikeout: false,
            bold: false,
            italic: false,
        });
    }
    out
}

fn keyword_highlight_spans(
    line: &str,
    rules: &[ResolvedKeywordHighlightRule],
) -> Vec<TerminalHighlightSpan> {
    if rules.is_empty() || line.is_empty() {
        return vec![TerminalHighlightSpan {
            text: line.to_string(),
            color: None,
            bg: None,
            keyword: false,
            underline: false,
            strikeout: false,
            bold: false,
            italic: false,
        }];
    }

    let compiled = compile_keyword_rules(rules);
    if compiled.is_empty() {
        return vec![TerminalHighlightSpan {
            text: line.to_string(),
            color: None,
            bg: None,
            keyword: false,
            underline: false,
            strikeout: false,
            bold: false,
            italic: false,
        }];
    }

    let mut spans = Vec::new();
    let mut cursor = 0;
    while cursor < line.len() {
        let mut best: Option<(usize, usize, u32)> = None;
        for (regex, color) in &compiled {
            if let Some(found) = regex.find_at(line, cursor) {
                let start = found.start();
                let end = found.end();
                if end <= start {
                    continue;
                }
                let replace = best
                    .map(|(best_start, best_end, _)| {
                        start < best_start || (start == best_start && end > best_end)
                    })
                    .unwrap_or(true);
                if replace {
                    best = Some((start, end, *color));
                }
            }
        }

        let Some((start, end, color)) = best else {
            spans.push(TerminalHighlightSpan {
                text: line[cursor..].to_string(),
                color: None,
                bg: None,
                keyword: false,
                underline: false,
                strikeout: false,
                bold: false,
                italic: false,
            });
            break;
        };
        if start > cursor {
            spans.push(TerminalHighlightSpan {
                text: line[cursor..start].to_string(),
                color: None,
                bg: None,
                keyword: false,
                underline: false,
                strikeout: false,
                bold: false,
                italic: false,
            });
        }
        spans.push(TerminalHighlightSpan {
            text: line[start..end].to_string(),
            color: Some(color),
            bg: None,
            keyword: true,
            underline: false,
            strikeout: false,
            bold: false,
            italic: false,
        });
        cursor = end;
    }

    if spans.is_empty() {
        spans.push(TerminalHighlightSpan {
            text: " ".to_string(),
            color: None,
            bg: None,
            keyword: false,
            underline: false,
            strikeout: false,
            bold: false,
            italic: false,
        });
    }
    spans
}

fn compile_keyword_rules(rules: &[ResolvedKeywordHighlightRule]) -> Vec<(regex::Regex, u32)> {
    let mut compiled = Vec::new();
    for rule in rules.iter().filter(|rule| rule.enabled) {
        let color = parse_hex_rgb(&rule.color).unwrap_or(0x79c0ff);
        let mut alts = Vec::new();
        for pattern in rule
            .patterns
            .iter()
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
        {
            // Validate each alternative; skip invalid regex like Tauri.
            if regex::Regex::new(&format!("(?i)(?:{pattern})")).is_ok()
                || regex::Regex::new(pattern).is_ok()
            {
                alts.push(pattern.to_string());
            }
        }
        if alts.is_empty() {
            continue;
        }
        let combined = if alts.len() == 1 {
            alts[0].clone()
        } else {
            alts.iter()
                .map(|p| format!("(?:{p})"))
                .collect::<Vec<_>>()
                .join("|")
        };
        let pattern = if combined.contains("(?i)") || combined.contains("(?-i)") {
            combined
        } else {
            format!("(?i){combined}")
        };
        match regex::Regex::new(&pattern) {
            Ok(regex) => compiled.push((regex, color)),
            Err(_) => {
                for alt in alts {
                    let pat = if alt.contains("(?i)") {
                        alt
                    } else {
                        format!("(?i){alt}")
                    };
                    if let Ok(regex) = regex::Regex::new(&pat) {
                        compiled.push((regex, color));
                    }
                }
            }
        }
    }
    compiled
}

pub(super) fn terminal_buffer_matches(
    output: &str,
    query: &str,
    flags: &TerminalSearchFlags,
    limit: usize,
) -> Result<Vec<TerminalBufferMatch>, String> {
    if query.trim().is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let mut matches = Vec::new();
    if flags.regex {
        let pattern = if flags.case_sensitive {
            query.to_string()
        } else {
            format!("(?i){query}")
        };
        let regex = regex::Regex::new(&pattern).map_err(|error| error.to_string())?;
        for (line_index, line) in output.lines().enumerate() {
            for found in regex.find_iter(line) {
                if flags.whole_word && !is_whole_word_match(line, found.start(), found.end()) {
                    continue;
                }
                let start_col = line[..found.start()].chars().count();
                let end_col = line[..found.end()].chars().count();
                matches.push(TerminalBufferMatch {
                    line_index,
                    start_col,
                    end_col,
                });
                if matches.len() >= limit {
                    return Ok(matches);
                }
            }
        }
        return Ok(matches);
    }

    let needle = if flags.case_sensitive {
        query.to_string()
    } else {
        query.to_ascii_lowercase()
    };
    for (line_index, line) in output.lines().enumerate() {
        let haystack = if flags.case_sensitive {
            line.to_string()
        } else {
            line.to_ascii_lowercase()
        };
        let mut cursor = 0;
        while cursor <= haystack.len() {
            let Some(relative_start) = haystack[cursor..].find(&needle) else {
                break;
            };
            let start = cursor + relative_start;
            let end = start + needle.len();
            if !flags.whole_word || is_whole_word_match(line, start, end) {
                let start_col = line[..start.min(line.len())].chars().count();
                let end_col = line[..end.min(line.len())].chars().count();
                matches.push(TerminalBufferMatch {
                    line_index,
                    start_col,
                    end_col,
                });
                if matches.len() >= limit {
                    return Ok(matches);
                }
            }
            cursor = end.max(cursor + 1);
        }
    }
    Ok(matches)
}

fn is_whole_word_match(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char)
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn parse_hex_rgb(value: &str) -> Option<u32> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

pub(super) fn terminal_key_bytes(event: &KeyDownEvent) -> Option<Vec<u8>> {
    let keystroke = &event.keystroke;
    if keystroke.modifiers.function {
        return None;
    }
    // Super/Win key combos are reserved for the shell/OS.
    if keystroke.modifiers.platform && !keystroke.modifiers.control && !keystroke.modifiers.alt {
        return None;
    }

    let key = keystroke.key.as_str();
    let ctrl = keystroke.modifiers.control;
    let alt = keystroke.modifiers.alt;
    let shift = keystroke.modifiers.shift;

    // Ctrl+Arrow / Alt+Arrow CSI sequences (Tauri XTerminal word-nav parity).
    if matches!(key, "up" | "down" | "left" | "right") {
        if ctrl && !alt && !shift {
            let suffix = match key {
                "up" => b"\x1b[1;5A",
                "down" => b"\x1b[1;5B",
                "right" => b"\x1b[1;5C",
                "left" => b"\x1b[1;5D",
                _ => unreachable!(),
            };
            return Some(suffix.to_vec());
        }
        if alt && !ctrl && !shift {
            let suffix = match key {
                "up" => b"\x1b[1;3A",
                "down" => b"\x1b[1;3B",
                "right" => b"\x1b[1;3C",
                "left" => b"\x1b[1;3D",
                _ => unreachable!(),
            };
            return Some(suffix.to_vec());
        }
    }

    if ctrl && !alt {
        return control_key_bytes(key);
    }

    // Plain navigation / editing keys (no modifiers other than shift where irrelevant).
    if !ctrl && !alt && !keystroke.modifiers.platform {
        match key {
            "enter" => return Some(b"\r".to_vec()),
            "backspace" => return Some(vec![0x7f]),
            "tab" => return Some(b"\t".to_vec()),
            "escape" => return Some(vec![0x1b]),
            "up" => return Some(b"\x1b[A".to_vec()),
            "down" => return Some(b"\x1b[B".to_vec()),
            "right" => return Some(b"\x1b[C".to_vec()),
            "left" => return Some(b"\x1b[D".to_vec()),
            "home" => return Some(b"\x1b[H".to_vec()),
            "end" => return Some(b"\x1b[F".to_vec()),
            "delete" => return Some(b"\x1b[3~".to_vec()),
            "pageup" => return Some(b"\x1b[5~".to_vec()),
            "pagedown" => return Some(b"\x1b[6~".to_vec()),
            _ => {}
        }

        return keystroke
            .key_char
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(|value| value.as_bytes().to_vec());
    }

    None
}

fn control_key_bytes(key: &str) -> Option<Vec<u8>> {
    // Ctrl+Arrow handled above.
    if matches!(key, "up" | "down" | "left" | "right") {
        return None;
    }
    let byte = match key {
        "space" => 0x00,
        "left_bracket" | "[" => 0x1b,
        "backslash" | "\\" => 0x1c,
        "right_bracket" | "]" => 0x1d,
        "6" => 0x1e,
        "slash" | "/" => 0x1f,
        value if value.len() == 1 => {
            let byte = value.as_bytes()[0].to_ascii_lowercase();
            if byte.is_ascii_lowercase() {
                byte - b'a' + 1
            } else {
                return None;
            }
        }
        _ => return None,
    };
    Some(vec![byte])
}

pub(super) fn trim_terminal_output(output: &mut String) {
    const MAX_BYTES: usize = 64 * 1024;
    if output.len() <= MAX_BYTES {
        return;
    }
    let drain_to = output
        .char_indices()
        .find_map(|(index, _)| (index >= output.len() - MAX_BYTES).then_some(index))
        .unwrap_or(0);
    output.drain(..drain_to);
}

pub(super) fn initial_terminal_screen() -> TerminalScreen {
    terminal_screen_from_output(INITIAL_TERMINAL_BANNER)
}

pub(super) fn terminal_screen_from_output(output: &str) -> TerminalScreen {
    let mut screen = TerminalScreen::default();
    screen.advance(output.as_bytes());
    screen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_matches_report_column_ranges() {
        let output = "hello world\nfoo hello bar";
        let matches = terminal_buffer_matches(
            output,
            "hello",
            &TerminalSearchFlags {
                case_sensitive: false,
                regex: false,
                whole_word: false,
            },
            10,
        )
        .expect("matches");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line_index, 0);
        assert_eq!(matches[0].start_col, 0);
        assert_eq!(matches[0].end_col, 5);
        assert_eq!(matches[1].line_index, 1);
        assert_eq!(matches[1].start_col, 4);
        assert_eq!(matches[1].end_col, 9);
    }
}
