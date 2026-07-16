use super::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct TerminalBufferMatch {
    pub line_index: usize,
    /// Half-open character column range on the matched line.
    pub start_col: usize,
    pub end_col: usize,
}

#[derive(Debug, Clone)]
pub struct TerminalSearchFlags {
    pub case_sensitive: bool,
    pub regex: bool,
    pub whole_word: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TerminalLineDecorations {
    pub search_ranges: Vec<(usize, usize)>,
    pub active_search_ranges: Vec<(usize, usize)>,
    pub selection_cols: Option<(usize, usize)>,
    pub link_ranges: Vec<(usize, usize)>,
    /// OSC 133 shell-integration mark for this viewport row.
    pub command_mark: Option<nyaterm_terminal::ShellCommandMark>,
}

#[derive(Debug, Clone)]
struct CachedTerminalPaintRow {
    key: u64,
    line: ShapedLine,
}

#[derive(Debug, Default)]
pub struct NyaTerminalLayoutCache {
    rows: Vec<Option<CachedTerminalPaintRow>>,
    pub hits: u64,
    pub misses: u64,
}

impl NyaTerminalLayoutCache {
    pub fn clear(&mut self) {
        self.rows.clear();
        self.hits = 0;
        self.misses = 0;
    }

    fn shaped_line(
        &mut self,
        row: usize,
        key: u64,
        shape: impl FnOnce() -> ShapedLine,
    ) -> ShapedLine {
        if self.rows.len() <= row {
            self.rows.resize_with(row + 1, || None);
        }
        if let Some(cached) = self.rows[row].as_ref()
            && cached.key == key
        {
            self.hits = self.hits.saturating_add(1);
            return cached.line.clone();
        }
        self.misses = self.misses.saturating_add(1);
        let line = shape();
        self.rows[row] = Some(CachedTerminalPaintRow {
            key,
            line: line.clone(),
        });
        line
    }
}

#[cfg(test)]
mod layout_cache_tests {
    use super::*;

    #[test]
    fn shaped_line_cache_reuses_matching_row_key() {
        let mut cache = NyaTerminalLayoutCache::default();

        let _ = cache.shaped_line(0, 42, ShapedLine::default);
        let _ = cache.shaped_line(0, 42, || panic!("matching key should reuse cached row"));

        assert_eq!(cache.misses, 1);
        assert_eq!(cache.hits, 1);
    }

    #[test]
    fn clear_resets_shaped_line_cache() {
        let mut cache = NyaTerminalLayoutCache::default();
        let _ = cache.shaped_line(0, 42, ShapedLine::default);

        cache.clear();

        assert_eq!(cache.misses, 0);
        assert_eq!(cache.hits, 0);
        let _ = cache.shaped_line(0, 42, ShapedLine::default);
        assert_eq!(cache.misses, 1);
    }

    #[test]
    fn styled_span_hash_tracks_style_changes() {
        let plain = vec![nyaterm_terminal::StyledSpan {
            text: "same".to_string(),
            style: nyaterm_terminal::CellStyle::default(),
        }];
        let mut bold_style = nyaterm_terminal::CellStyle::default();
        bold_style.bold = true;
        let bold = vec![nyaterm_terminal::StyledSpan {
            text: "same".to_string(),
            style: bold_style,
        }];

        let mut plain_hasher = DefaultHasher::new();
        hash_styled_spans(Some(&plain), &mut plain_hasher);
        let mut bold_hasher = DefaultHasher::new();
        hash_styled_spans(Some(&bold), &mut bold_hasher);

        assert_ne!(plain_hasher.finish(), bold_hasher.finish());
    }

    #[test]
    fn row_layout_key_tracks_styled_spans() {
        let mut snapshot = TerminalScreen::default().snapshot();
        snapshot.lines[0] = "same".to_string();
        snapshot.line_signatures[0] = 7;
        let element = NyaTerminalElement::new(
            Arc::new(snapshot),
            Vec::new(),
            Vec::new(),
            false,
            "block",
            8.0,
            16.0,
            nyaterm_ui::theme_palette("github-dark"),
            "monospace".to_string(),
            14.0,
            400.0,
            700.0,
        );
        let decorations = TerminalLineDecorations::default();
        let plain = vec![nyaterm_terminal::StyledSpan {
            text: "same".to_string(),
            style: nyaterm_terminal::CellStyle::default(),
        }];
        let mut bold_style = nyaterm_terminal::CellStyle::default();
        bold_style.bold = true;
        let bold = vec![nyaterm_terminal::StyledSpan {
            text: "same".to_string(),
            style: bold_style,
        }];

        assert_ne!(
            element.row_layout_key(0, "same", Some(&plain), &decorations),
            element.row_layout_key(0, "same", Some(&bold), &decorations)
        );
    }

    #[test]
    fn terminal_glyph_decorations_detects_glyph_only_work() {
        assert!(!terminal_glyph_decorations_needed(
            &TerminalLineDecorations::default()
        ));

        let mut decorations = TerminalLineDecorations {
            command_mark: Some(nyaterm_terminal::ShellCommandMark::Prompt),
            ..TerminalLineDecorations::default()
        };
        assert!(!terminal_glyph_decorations_needed(&decorations));

        decorations.link_ranges.push((1, 3));
        assert!(terminal_glyph_decorations_needed(&decorations));

        decorations.link_ranges.clear();
        decorations.selection_cols = Some((0, 2));
        assert!(terminal_glyph_decorations_needed(&decorations));

        decorations.selection_cols = None;
        decorations.search_ranges.push((2, 4));
        assert!(terminal_glyph_decorations_needed(&decorations));

        decorations.search_ranges.clear();
        decorations.active_search_ranges.push((2, 4));
        assert!(terminal_glyph_decorations_needed(&decorations));
    }

    #[test]
    fn plain_row_fast_path_accepts_unstyled_rows() {
        let default_spans = [nyaterm_terminal::StyledSpan {
            text: "plain".to_string(),
            style: nyaterm_terminal::CellStyle::default(),
        }];

        assert!(terminal_plain_row_fast_path(
            None,
            &[],
            &TerminalLineDecorations::default()
        ));
        assert!(terminal_plain_row_fast_path(
            Some(&default_spans),
            &[],
            &TerminalLineDecorations::default()
        ));
    }

    #[test]
    fn plain_row_fast_path_rejects_enhanced_rows() {
        let mut styled = nyaterm_terminal::CellStyle::default();
        styled.bold = true;
        let styled_spans = [nyaterm_terminal::StyledSpan {
            text: "bold".to_string(),
            style: styled,
        }];
        let keyword_rule = ResolvedKeywordHighlightRule {
            id: "errors".to_string(),
            name: "Errors".to_string(),
            patterns: vec!["error".to_string()],
            color: "#ff0000".to_string(),
            enabled: true,
        };
        let selection = TerminalLineDecorations {
            selection_cols: Some((0, 2)),
            ..TerminalLineDecorations::default()
        };
        let command_mark = TerminalLineDecorations {
            command_mark: Some(nyaterm_terminal::ShellCommandMark::Prompt),
            ..TerminalLineDecorations::default()
        };

        assert!(!terminal_plain_row_fast_path(
            Some(&styled_spans),
            &[],
            &TerminalLineDecorations::default()
        ));
        assert!(!terminal_plain_row_fast_path(
            None,
            &[keyword_rule],
            &TerminalLineDecorations::default()
        ));
        assert!(!terminal_plain_row_fast_path(None, &[], &selection));
        assert!(terminal_plain_row_fast_path(None, &[], &command_mark));
    }
}

pub struct NyaTerminalElement {
    snapshot: Arc<TerminalSnapshot>,
    keyword_rules: Vec<ResolvedKeywordHighlightRule>,
    decorations: Vec<TerminalLineDecorations>,
    layout_cache: Option<Arc<Mutex<NyaTerminalLayoutCache>>>,
    show_cursor: bool,
    cursor_style: String,
    cell_width: f32,
    cell_height: f32,
    palette: nyaterm_ui::ThemePalette,
    font_family: String,
    font_size: f32,
    normal_weight: f32,
    bold_weight: f32,
}

struct TerminalPaintRow {
    y: Pixels,
    line: ShapedLine,
}

pub struct TerminalImagePaint {
    bounds: Bounds<Pixels>,
    image: std::sync::Arc<gpui::RenderImage>,
}

#[derive(Default)]
pub struct NyaTerminalPaintPlan {
    /// Cell / keyword backgrounds (under protocol images).
    backgrounds: Vec<PaintQuad>,
    /// Decoded graphics protocol images painted under terminal text.
    images_under: Vec<TerminalImagePaint>,
    /// Accent placeholders for undecodable under-text images.
    placeholders_under: Vec<PaintQuad>,
    /// Search match + selection washes (over under-text images, under glyphs).
    decoration_backgrounds: Vec<PaintQuad>,
    /// Active-search gutter + OSC 133 command marks (under glyphs).
    active_markers: Vec<PaintQuad>,
    rows: Vec<TerminalPaintRow>,
    /// Decoded graphics with Kitty z>0, painted above terminal text.
    images_above: Vec<TerminalImagePaint>,
    /// Accent placeholders for undecodable above-text images.
    placeholders_above: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
}

fn image_mask(
    bounds: Bounds<Pixels>,
    cols: usize,
    rows: usize,
    cell_w: f32,
    cell_h: f32,
) -> ContentMask<Pixels> {
    ContentMask {
        bounds: Bounds::new(
            bounds.origin,
            size(px(cols as f32 * cell_w), px(rows as f32 * cell_h)),
        ),
    }
}

impl NyaTerminalElement {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        snapshot: Arc<TerminalSnapshot>,
        keyword_rules: Vec<ResolvedKeywordHighlightRule>,
        decorations: Vec<TerminalLineDecorations>,
        show_cursor: bool,
        cursor_style: impl Into<String>,
        cell_width: f32,
        cell_height: f32,
        palette: nyaterm_ui::ThemePalette,
        font_family: String,
        font_size: f32,
        normal_weight: f32,
        bold_weight: f32,
    ) -> Self {
        Self {
            snapshot,
            keyword_rules,
            decorations,
            layout_cache: None,
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

    pub fn with_layout_cache(mut self, cache: Arc<Mutex<NyaTerminalLayoutCache>>) -> Self {
        self.layout_cache = Some(cache);
        self
    }

    fn row_layout_key(
        &self,
        row: usize,
        display_line: &str,
        ansi_spans: Option<&[nyaterm_terminal::StyledSpan]>,
        decorations: &TerminalLineDecorations,
    ) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.snapshot
            .line_signatures
            .get(row)
            .copied()
            .unwrap_or_default()
            .hash(&mut hasher);
        display_line.hash(&mut hasher);
        hash_styled_spans(ansi_spans, &mut hasher);
        decorations.hash(&mut hasher);
        for rule in &self.keyword_rules {
            rule.id.hash(&mut hasher);
            rule.name.hash(&mut hasher);
            rule.patterns.hash(&mut hasher);
            rule.color.hash(&mut hasher);
            rule.enabled.hash(&mut hasher);
        }
        self.palette.bg.hash(&mut hasher);
        self.palette.surface.hash(&mut hasher);
        self.palette.accent.hash(&mut hasher);
        self.palette.terminal_fg.hash(&mut hasher);
        self.palette.terminal_bg.hash(&mut hasher);
        self.palette.terminal_ansi.hash(&mut hasher);
        self.font_family.hash(&mut hasher);
        self.font_size.to_bits().hash(&mut hasher);
        self.normal_weight.to_bits().hash(&mut hasher);
        self.bold_weight.to_bits().hash(&mut hasher);
        hasher.finish()
    }

    fn shape_row(
        &self,
        row: usize,
        row_key: u64,
        text: String,
        text_runs: Vec<TextRun>,
        font_size: Pixels,
        window: &mut Window,
    ) -> ShapedLine {
        let shape_row = |window: &mut Window| {
            window.text_system().shape_line(
                SharedString::from(text.clone()),
                font_size,
                &text_runs,
                None,
            )
        };
        if let Some(cache) = self.layout_cache.as_ref() {
            match cache.lock() {
                Ok(mut cache) => cache.shaped_line(row, row_key, || shape_row(window)),
                Err(_) => shape_row(window),
            }
        } else {
            shape_row(window)
        }
    }
}

fn hash_styled_spans<H: Hasher>(
    ansi_spans: Option<&[nyaterm_terminal::StyledSpan]>,
    hasher: &mut H,
) {
    if let Some(spans) = ansi_spans {
        spans.len().hash(hasher);
        for span in spans {
            span.text.hash(hasher);
            span.style.hash(hasher);
        }
    } else {
        0usize.hash(hasher);
    }
}

fn terminal_glyph_decorations_needed(decorations: &TerminalLineDecorations) -> bool {
    !decorations.search_ranges.is_empty()
        || !decorations.active_search_ranges.is_empty()
        || decorations.selection_cols.is_some()
        || !decorations.link_ranges.is_empty()
}

fn terminal_plain_row_fast_path(
    ansi_spans: Option<&[nyaterm_terminal::StyledSpan]>,
    keyword_rules: &[ResolvedKeywordHighlightRule],
    decorations: &TerminalLineDecorations,
) -> bool {
    keyword_rules.is_empty()
        && !terminal_glyph_decorations_needed(decorations)
        && terminal_ansi_spans_are_plain(ansi_spans)
}

fn terminal_ansi_spans_are_plain(ansi_spans: Option<&[nyaterm_terminal::StyledSpan]>) -> bool {
    let Some(spans) = ansi_spans else {
        return true;
    };
    let default_style = nyaterm_terminal::CellStyle::default();
    spans
        .iter()
        .all(|span| span.text.is_empty() || span.style == default_style)
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
            let default_decorations;
            let decorations = if let Some(decorations) = self.decorations.get(row) {
                decorations
            } else {
                default_decorations = TerminalLineDecorations::default();
                &default_decorations
            };
            let y = px(f32::from(bounds.top()) + row as f32 * cell_h);

            if !decorations.active_search_ranges.is_empty() {
                plan.active_markers.push(fill(
                    Bounds::new(point(bounds.left(), y), size(px(2.), px(cell_h))),
                    rgb(self.palette.warning),
                ));
            }
            // OSC 133 command marks: left gutter bar (under glyphs, with other marks).
            if let Some(mark) = decorations.command_mark {
                use nyaterm_terminal::ShellCommandMark;
                let color = match mark {
                    ShellCommandMark::Prompt => self.palette.accent,
                    ShellCommandMark::Output => self.palette.text_muted,
                    ShellCommandMark::Finished {
                        exit_code: Some(code),
                    } if code != 0 => self.palette.danger,
                    ShellCommandMark::Finished { .. } => self.palette.success,
                };
                // Offset 1px when active-search mark is also present so both remain visible.
                let x = if decorations.active_search_ranges.is_empty() {
                    bounds.left()
                } else {
                    px(f32::from(bounds.left()) + 2.)
                };
                plan.active_markers.push(fill(
                    Bounds::new(point(x, y), size(px(2.), px(cell_h))),
                    rgb(color),
                ));
            }

            // Plain/degraded rows skip full highlight span work entirely.
            if terminal_plain_row_fast_path(ansi, &self.keyword_rules, decorations) {
                let text = display_line.to_string();
                let text_runs = vec![TextRun {
                    len: text.len().max(1),
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
                }];
                let row_key = self.row_layout_key(row, display_line, ansi, decorations);
                let shaped = self.shape_row(row, row_key, text, text_runs, font_size, window);
                plan.rows.push(TerminalPaintRow { y, line: shaped });
                continue;
            }

            // Base spans drive cell/keyword backgrounds only (under images).
            let base_spans = terminal_highlight_spans(
                display_line,
                ansi,
                &self.keyword_rules,
                &[],
                &[],
                None,
                &decorations.link_ranges,
                self.palette,
            );
            // Full spans include search/selection fg tweaks for the glyph layer.
            let spans = if terminal_glyph_decorations_needed(decorations) {
                terminal_highlight_spans(
                    display_line,
                    ansi,
                    &self.keyword_rules,
                    &decorations.search_ranges,
                    &decorations.active_search_ranges,
                    decorations.selection_cols,
                    &decorations.link_ranges,
                    self.palette,
                )
            } else {
                base_spans.clone()
            };

            // Cell / keyword backgrounds (OxideTerm layers 2–3).
            let mut col = 0usize;
            let mut pending_bg: Option<(u32, usize, usize)> = None;
            for span in &base_spans {
                let bg = span
                    .bg
                    .or_else(|| span.keyword.then_some(self.palette.surface));
                let span_cols = terminal_cell_count(&span.text).max(1);
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

            // Search + selection washes (OxideTerm layers 5/7) — filled after under-images.
            for &(start, end) in &decorations.search_ranges {
                push_col_range_bg(
                    row,
                    start,
                    end,
                    self.palette.terminal_selection,
                    bounds,
                    cell_w,
                    cell_h,
                    &mut plan.decoration_backgrounds,
                );
            }
            for &(start, end) in &decorations.active_search_ranges {
                push_col_range_bg(
                    row,
                    start,
                    end,
                    self.palette.warning,
                    bounds,
                    cell_w,
                    cell_h,
                    &mut plan.decoration_backgrounds,
                );
            }
            if let Some((start, end)) = decorations.selection_cols {
                push_col_range_bg(
                    row,
                    start,
                    end,
                    self.palette.terminal_selection,
                    bounds,
                    cell_w,
                    cell_h,
                    &mut plan.decoration_backgrounds,
                );
            }

            let mut text = String::new();
            let mut text_runs = Vec::new();
            for span in spans {
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
            }

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
            let row_key = self.row_layout_key(row, display_line, ansi, decorations);
            let shaped = self.shape_row(row, row_key, text, text_runs, font_size, window);
            plan.rows.push(TerminalPaintRow { y, line: shaped });
        }

        // Graphics protocol placements (Kitty / iTerm2 / Sixel).
        // Kitty z>0 places above text; everything else stays under the glyph layer.
        for image in &self.snapshot.images {
            if image.width_cells == 0 || image.height_cells == 0 {
                continue;
            }
            let x = px(f32::from(bounds.left())
                + (image.col as f32 - image.source_col_cells as f32) * cell_w);
            let y = px(f32::from(bounds.top())
                + (image.row as f32 - image.source_row_cells as f32) * cell_h);
            let w = px(image.image_width_cells as f32 * cell_w);
            let h = px(image.image_height_cells as f32 * cell_h);
            let rect = Bounds::new(point(x, y), size(w, h));
            if let Some(decoded) = crate::cached_render_image(image.id, &image.data) {
                let paint = TerminalImagePaint {
                    bounds: rect,
                    image: decoded,
                };
                if image.above_text {
                    plan.images_above.push(paint);
                } else {
                    plan.images_under.push(paint);
                }
            } else {
                // Dim accent wash when payload is missing/undecodable (e.g. raw Sixel).
                let mut wash = rgb(self.palette.accent);
                wash.a = 0.18;
                let bar = Bounds::new(point(x, y), size(w, px(2.)));
                let mut bar_color = rgb(self.palette.accent);
                bar_color.a = 0.55;
                if image.above_text {
                    plan.placeholders_above.push(fill(rect, wash));
                    plan.placeholders_above.push(fill(bar, bar_color));
                } else {
                    plan.placeholders_under.push(fill(rect, wash));
                    plan.placeholders_under.push(fill(bar, bar_color));
                }
            }
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
        // OxideTerm-aligned layers:
        // cell/keyword bg → under images → search/selection → marks → text → above images → cursor
        for quad in prepaint.backgrounds.drain(..) {
            window.paint_quad(quad);
        }
        let image_mask = image_mask(
            bounds,
            self.snapshot.cols,
            self.snapshot.rows,
            self.cell_width.max(1.),
            self.cell_height.max(1.),
        );
        window.with_content_mask(Some(image_mask.clone()), |window| {
            for image in prepaint.images_under.drain(..) {
                let _ = window.paint_image(
                    image.bounds,
                    gpui::Corners::default(),
                    image.image,
                    0,
                    false,
                );
            }
            for quad in prepaint.placeholders_under.drain(..) {
                window.paint_quad(quad);
            }
        });
        for quad in prepaint.decoration_backgrounds.drain(..) {
            window.paint_quad(quad);
        }
        for quad in prepaint.active_markers.drain(..) {
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
        window.with_content_mask(Some(image_mask), |window| {
            for image in prepaint.images_above.drain(..) {
                let _ = window.paint_image(
                    image.bounds,
                    gpui::Corners::default(),
                    image.image,
                    0,
                    false,
                );
            }
            for quad in prepaint.placeholders_above.drain(..) {
                window.paint_quad(quad);
            }
        });
        if let Some(cursor) = prepaint.cursor.take() {
            window.paint_quad(cursor);
        }
    }
}
