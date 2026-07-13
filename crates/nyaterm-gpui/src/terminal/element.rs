use super::*;

#[derive(Debug, Clone)]
pub(crate) struct TerminalBufferMatch {
    pub(crate) line_index: usize,
    /// Half-open character column range on the matched line.
    pub(crate) start_col: usize,
    pub(crate) end_col: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct TerminalSearchFlags {
    pub(crate) case_sensitive: bool,
    pub(crate) regex: bool,
    pub(crate) whole_word: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TerminalLineDecorations {
    pub(crate) search_ranges: Vec<(usize, usize)>,
    pub(crate) active_search_ranges: Vec<(usize, usize)>,
    pub(crate) selection_cols: Option<(usize, usize)>,
    pub(crate) link_ranges: Vec<(usize, usize)>,
}

pub(crate) struct NyaTerminalElement {
    snapshot: TerminalSnapshot,
    keyword_rules: Vec<ResolvedKeywordHighlightRule>,
    decorations: Vec<TerminalLineDecorations>,
    show_cursor: bool,
    cursor_style: String,
    cell_width: f32,
    cell_height: f32,
    palette: crate::theme::ThemePalette,
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
pub(crate) struct NyaTerminalPaintPlan {
    backgrounds: Vec<PaintQuad>,
    active_markers: Vec<PaintQuad>,
    rows: Vec<TerminalPaintRow>,
    cursor: Option<PaintQuad>,
}

impl NyaTerminalElement {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        snapshot: TerminalSnapshot,
        keyword_rules: Vec<ResolvedKeywordHighlightRule>,
        decorations: Vec<TerminalLineDecorations>,
        show_cursor: bool,
        cursor_style: impl Into<String>,
        cell_width: f32,
        cell_height: f32,
        palette: crate::theme::ThemePalette,
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
