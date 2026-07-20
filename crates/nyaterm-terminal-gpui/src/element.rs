use super::*;
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::Instant;

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
    line: Arc<ShapedLine>,
}

#[derive(Debug, Default)]
pub struct NyaTerminalLayoutCache {
    rows: HashMap<u64, CachedTerminalPaintRow>,
    compiled_keyword_key: Option<u64>,
    compiled_keyword_rules: CompiledKeywordRules,
    pub hits: u64,
    pub misses: u64,
    pub shape_calls: u64,
    pub shape_duration_us: u64,
}

const TERMINAL_LAYOUT_CACHE_ROW_CAP: usize = 4096;
const TERMINAL_ELEMENT_PREPAINT_SLOW_MS: u128 = 12;
const TERMINAL_ELEMENT_PAINT_SLOW_MS: u128 = 12;

impl NyaTerminalLayoutCache {
    pub fn clear(&mut self) {
        self.rows.clear();
        self.compiled_keyword_key = None;
        self.compiled_keyword_rules.clear();
        self.hits = 0;
        self.misses = 0;
        self.shape_calls = 0;
        self.shape_duration_us = 0;
    }

    fn compiled_keyword_rules(
        &mut self,
        key: u64,
        rules: &[ResolvedKeywordHighlightRule],
    ) -> CompiledKeywordRules {
        if self.compiled_keyword_key == Some(key) {
            return self.compiled_keyword_rules.clone();
        }
        self.compiled_keyword_key = Some(key);
        self.compiled_keyword_rules = compile_keyword_rules(rules);
        self.compiled_keyword_rules.clone()
    }

    fn shaped_line(
        &mut self,
        _row: usize,
        key: u64,
        shape: impl FnOnce() -> (Arc<ShapedLine>, std::time::Duration),
    ) -> (Arc<ShapedLine>, bool, std::time::Duration) {
        if let Some(cached) = self.rows.get(&key) {
            self.hits = self.hits.saturating_add(1);
            return (Arc::clone(&cached.line), false, std::time::Duration::ZERO);
        }
        self.misses = self.misses.saturating_add(1);
        if self.rows.len() >= TERMINAL_LAYOUT_CACHE_ROW_CAP {
            self.rows.clear();
        }
        let (line, duration) = shape();
        self.shape_calls = self.shape_calls.saturating_add(1);
        self.shape_duration_us = self
            .shape_duration_us
            .saturating_add(duration.as_micros().min(u128::from(u64::MAX)) as u64);
        self.rows.insert(
            key,
            CachedTerminalPaintRow {
                line: Arc::clone(&line),
            },
        );
        (line, true, duration)
    }
}

#[cfg(test)]
mod layout_cache_tests {
    use super::*;

    #[test]
    fn shaped_line_cache_reuses_matching_row_key() {
        let mut cache = NyaTerminalLayoutCache::default();

        let _ = cache.shaped_line(0, 42, || {
            (Arc::new(ShapedLine::default()), std::time::Duration::ZERO)
        });
        let _ = cache.shaped_line(7, 42, || {
            panic!("matching key should reuse cached row even at another viewport row")
        });

        assert_eq!(cache.misses, 1);
        assert_eq!(cache.hits, 1);
    }

    #[test]
    fn clear_resets_shaped_line_cache() {
        let mut cache = NyaTerminalLayoutCache::default();
        let _ = cache.shaped_line(0, 42, || {
            (Arc::new(ShapedLine::default()), std::time::Duration::ZERO)
        });

        cache.clear();

        assert_eq!(cache.misses, 0);
        assert_eq!(cache.hits, 0);
        let _ = cache.shaped_line(0, 42, || {
            (Arc::new(ShapedLine::default()), std::time::Duration::ZERO)
        });
        assert_eq!(cache.misses, 1);
    }

    #[test]
    fn shaped_line_cache_misses_on_style_key_change() {
        let mut cache = NyaTerminalLayoutCache::default();

        let _ = cache.shaped_line(0, 42, || {
            (Arc::new(ShapedLine::default()), std::time::Duration::ZERO)
        });
        let _ = cache.shaped_line(0, 43, || {
            (Arc::new(ShapedLine::default()), std::time::Duration::ZERO)
        });

        assert_eq!(cache.misses, 2);
        assert_eq!(cache.hits, 0);
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
            Arc::new(Vec::new()),
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
    fn row_layout_key_ignores_dynamic_overlay_decorations() {
        let mut snapshot = TerminalScreen::default().snapshot();
        snapshot.lines[0] = "same".to_string();
        snapshot.line_signatures[0] = 7;
        let element = NyaTerminalElement::new(
            Arc::new(snapshot),
            Arc::new(Vec::new()),
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
        let base = TerminalLineDecorations::default();
        let dynamic = TerminalLineDecorations {
            search_ranges: vec![(0, 2)],
            selection_cols: Some((1, 3)),
            command_mark: Some(nyaterm_terminal::ShellCommandMark::Prompt),
            ..TerminalLineDecorations::default()
        };

        assert_eq!(
            element.row_layout_key(0, "same", None, &base),
            element.row_layout_key(0, "same", None, &dynamic)
        );
    }

    #[test]
    fn row_layout_key_tracks_active_search_glyph_decorations() {
        let mut snapshot = TerminalScreen::default().snapshot();
        snapshot.lines[0] = "same".to_string();
        snapshot.line_signatures[0] = 7;
        let element = NyaTerminalElement::new(
            Arc::new(snapshot),
            Arc::new(Vec::new()),
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
        let base = TerminalLineDecorations::default();
        let active = TerminalLineDecorations {
            active_search_ranges: vec![(0, 2)],
            ..TerminalLineDecorations::default()
        };

        assert_ne!(
            element.row_layout_key(0, "same", None, &base),
            element.row_layout_key(0, "same", None, &active)
        );
    }

    #[test]
    fn row_layout_key_tracks_link_glyph_decorations() {
        let mut snapshot = TerminalScreen::default().snapshot();
        snapshot.lines[0] = "same".to_string();
        snapshot.line_signatures[0] = 7;
        let element = NyaTerminalElement::new(
            Arc::new(snapshot),
            Arc::new(Vec::new()),
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
        let base = TerminalLineDecorations::default();
        let linked = TerminalLineDecorations {
            link_ranges: vec![(0, 2)],
            ..TerminalLineDecorations::default()
        };

        assert_ne!(
            element.row_layout_key(0, "same", None, &base),
            element.row_layout_key(0, "same", None, &linked)
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
        assert!(!terminal_glyph_decorations_needed(&decorations));

        decorations.selection_cols = None;
        decorations.search_ranges.push((2, 4));
        assert!(!terminal_glyph_decorations_needed(&decorations));

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
        let active_search = TerminalLineDecorations {
            active_search_ranges: vec![(0, 2)],
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
        assert!(terminal_plain_row_fast_path(None, &[], &selection));
        assert!(terminal_plain_row_fast_path(None, &[], &command_mark));
        assert!(!terminal_plain_row_fast_path(None, &[], &active_search));
    }

    #[test]
    fn dynamic_decoration_backgrounds_include_plain_selection_and_search() {
        let palette = nyaterm_ui::theme_palette("github-dark");
        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(120.), px(40.)));
        let mut out = Vec::new();
        let decorations = TerminalLineDecorations {
            search_ranges: vec![(0, 2)],
            active_search_ranges: vec![(2, 4)],
            selection_cols: Some((4, 6)),
            ..TerminalLineDecorations::default()
        };

        push_dynamic_decoration_backgrounds(0, &decorations, palette, bounds, 8.0, 16.0, &mut out);

        assert_eq!(out.len(), 3);
    }

    #[test]
    fn visible_rows_expand_for_visual_scroll_offset() {
        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(100.), px(32.)));

        assert_eq!(terminal_visible_rows_for_bounds(bounds, 16., 10, 0.0), 0..3);
        assert_eq!(
            terminal_visible_rows_for_bounds(bounds, 16., 10, -8.0),
            0..4
        );
        assert_eq!(
            terminal_visible_rows_for_bounds(bounds, 16., 10, -20.0),
            0..5
        );
        assert_eq!(terminal_visible_rows_for_bounds(bounds, 16., 10, 8.0), 0..3);
        assert_eq!(
            terminal_visible_rows_for_bounds(bounds, 16., 10, 20.0),
            0..2
        );
    }

    #[test]
    fn layout_height_can_use_viewport_rows_instead_of_snapshot_window_rows() {
        assert_eq!(terminal_layout_height_px(16.0, 80, None), 1280.0);
        assert_eq!(terminal_layout_height_px(16.0, 80, Some(24)), 384.0);
        assert_eq!(terminal_layout_height_px(0.0, 0, Some(0)), 1.0);
    }

    #[test]
    fn concealed_cursor_cell_suppresses_cursor_glyph() {
        let mut snapshot = TerminalScreen::default().snapshot();
        snapshot.cursor_row = 0;
        snapshot.cursor_col = 0;
        snapshot.cells[0].style.hidden = true;

        assert!(terminal_cursor_cell_hidden(&snapshot));

        snapshot.cells[0].style.hidden = false;
        assert!(!terminal_cursor_cell_hidden(&snapshot));
    }
}

pub struct NyaTerminalElement {
    snapshot: Arc<TerminalSnapshot>,
    keyword_rules: Arc<Vec<ResolvedKeywordHighlightRule>>,
    decorations: Arc<[TerminalLineDecorations]>,
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
    visual_y_offset: f32,
    layout_rows: Option<usize>,
}

struct TerminalPaintRow {
    y: Pixels,
    line: Arc<ShapedLine>,
}

pub struct TerminalImagePaint {
    bounds: Bounds<Pixels>,
    image: std::sync::Arc<gpui::RenderImage>,
}

pub struct TerminalCursorGlyphPaint {
    origin: gpui::Point<Pixels>,
    line: Arc<ShapedLine>,
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
    cursor_background: Option<PaintQuad>,
    cursor_glyph: Option<TerminalCursorGlyphPaint>,
    shape_line_count: usize,
    shape_line_duration: std::time::Duration,
    text_run_count: usize,
}

impl NyaTerminalElement {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        snapshot: Arc<TerminalSnapshot>,
        keyword_rules: Arc<Vec<ResolvedKeywordHighlightRule>>,
        decorations: impl Into<Arc<[TerminalLineDecorations]>>,
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
            decorations: decorations.into(),
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
            visual_y_offset: 0.0,
            layout_rows: None,
        }
    }

    pub fn with_layout_cache(mut self, cache: Arc<Mutex<NyaTerminalLayoutCache>>) -> Self {
        self.layout_cache = Some(cache);
        self
    }

    pub fn with_visual_y_offset(mut self, offset: f32) -> Self {
        self.visual_y_offset = offset;
        self
    }

    pub fn with_layout_rows(mut self, rows: usize) -> Self {
        self.layout_rows = Some(rows.max(1));
        self
    }

    #[cfg(test)]
    fn row_layout_key(
        &self,
        row: usize,
        display_line: &str,
        ansi_spans: Option<&[nyaterm_terminal::StyledSpan]>,
        decorations: &TerminalLineDecorations,
    ) -> u64 {
        self.row_layout_key_with_keyword_key(
            row,
            display_line,
            ansi_spans,
            decorations,
            self.keyword_rules_key(),
        )
    }

    fn row_layout_key_with_keyword_key(
        &self,
        row: usize,
        display_line: &str,
        ansi_spans: Option<&[nyaterm_terminal::StyledSpan]>,
        decorations: &TerminalLineDecorations,
        keyword_rules_key: u64,
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
        hash_stable_glyph_decorations(decorations, &mut hasher);
        keyword_rules_key.hash(&mut hasher);
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

    fn keyword_rules_key(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        for rule in self.keyword_rules.iter() {
            rule.id.hash(&mut hasher);
            rule.name.hash(&mut hasher);
            rule.patterns.hash(&mut hasher);
            rule.color.hash(&mut hasher);
            rule.enabled.hash(&mut hasher);
        }
        hasher.finish()
    }

    fn compiled_keyword_rules_for_key(&self, key: u64) -> CompiledKeywordRules {
        if let Some(cache) = self.layout_cache.as_ref()
            && let Ok(mut cache) = cache.lock()
        {
            return cache.compiled_keyword_rules(key, self.keyword_rules.as_slice());
        }
        compile_keyword_rules(self.keyword_rules.as_slice())
    }

    fn shape_row(
        &self,
        row: usize,
        row_key: u64,
        text: String,
        text_runs: Vec<TextRun>,
        font_size: Pixels,
        window: &mut Window,
    ) -> (Arc<ShapedLine>, bool, std::time::Duration) {
        if let Some(cache) = self.layout_cache.as_ref() {
            if let Ok(mut cache) = cache.lock() {
                return cache.shaped_line(row, row_key, || {
                    let started_at = Instant::now();
                    let line = Arc::new(window.text_system().shape_line(
                        SharedString::from(text),
                        font_size,
                        &text_runs,
                        None,
                    ));
                    (line, started_at.elapsed())
                });
            }
        }
        let started_at = Instant::now();
        let line = Arc::new(window.text_system().shape_line(
            SharedString::from(text),
            font_size,
            &text_runs,
            None,
        ));
        (line, true, started_at.elapsed())
    }
}

fn hash_stable_glyph_decorations<H: Hasher>(decorations: &TerminalLineDecorations, hasher: &mut H) {
    decorations.active_search_ranges.hash(hasher);
    decorations.link_ranges.hash(hasher);
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

fn terminal_cursor_cell_hidden(snapshot: &TerminalSnapshot) -> bool {
    snapshot
        .cursor_row
        .checked_mul(snapshot.cols)
        .and_then(|row_start| row_start.checked_add(snapshot.cursor_col))
        .and_then(|index| snapshot.cells.get(index))
        .is_some_and(|cell| cell.style.hidden)
}

fn terminal_glyph_decorations_needed(decorations: &TerminalLineDecorations) -> bool {
    !decorations.active_search_ranges.is_empty() || !decorations.link_ranges.is_empty()
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

fn push_dynamic_decoration_backgrounds(
    row: usize,
    decorations: &TerminalLineDecorations,
    palette: nyaterm_ui::ThemePalette,
    bounds: Bounds<Pixels>,
    cell_w: f32,
    cell_h: f32,
    out: &mut Vec<PaintQuad>,
) {
    for &(start, end) in &decorations.search_ranges {
        push_col_range_bg(
            row,
            start,
            end,
            palette.terminal_selection,
            bounds,
            cell_w,
            cell_h,
            out,
        );
    }
    for &(start, end) in &decorations.active_search_ranges {
        push_col_range_bg(
            row,
            start,
            end,
            palette.warning,
            bounds,
            cell_w,
            cell_h,
            out,
        );
    }
    if let Some((start, end)) = decorations.selection_cols {
        push_col_range_bg(
            row,
            start,
            end,
            palette.terminal_selection,
            bounds,
            cell_w,
            cell_h,
            out,
        );
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
        style.size.height = px(terminal_layout_height_px(
            self.cell_height,
            self.snapshot.rows,
            self.layout_rows,
        ))
        .into();
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
        let started_at = Instant::now();
        let cache_stats_before = self
            .layout_cache
            .as_ref()
            .and_then(|cache| cache.lock().ok().map(|cache| (cache.hits, cache.misses)));
        let mut plan = NyaTerminalPaintPlan::default();
        let cell_w = self.cell_width.max(1.);
        let cell_h = self.cell_height.max(1.);
        let font_size = px(self.font_size.max(8.));
        let base_font = font(SharedString::from(self.font_family.clone()));
        let keyword_rules_key = self.keyword_rules_key();
        let compiled_keyword_rules = self.compiled_keyword_rules_for_key(keyword_rules_key);

        let visual_y_offset = self.visual_y_offset;
        let visible_rows =
            terminal_visible_rows_for_bounds(bounds, cell_h, self.snapshot.rows, visual_y_offset);
        let visible_row_start = visible_rows.start;
        let visible_row_end = visible_rows.end;
        let visible_row_count = visible_rows.len();
        for row in visible_rows {
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
            let y = px(f32::from(bounds.top()) + visual_y_offset + row as f32 * cell_h);

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

            push_dynamic_decoration_backgrounds(
                row,
                decorations,
                self.palette,
                bounds,
                cell_w,
                cell_h,
                &mut plan.decoration_backgrounds,
            );

            // Plain/degraded rows skip full highlight span work entirely.
            if terminal_plain_row_fast_path(ansi, self.keyword_rules.as_slice(), decorations) {
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
                let row_key = self.row_layout_key_with_keyword_key(
                    row,
                    display_line,
                    ansi,
                    decorations,
                    keyword_rules_key,
                );
                plan.text_run_count = plan.text_run_count.saturating_add(text_runs.len());
                let (shaped, did_shape, shape_duration) =
                    self.shape_row(row, row_key, text, text_runs, font_size, window);
                if did_shape {
                    plan.shape_line_count = plan.shape_line_count.saturating_add(1);
                    plan.shape_line_duration += shape_duration;
                }
                plan.rows.push(TerminalPaintRow { y, line: shaped });
                continue;
            }

            // Base spans drive cell/keyword backgrounds only (under images).
            let background_spans = terminal_highlight_spans_compiled(
                display_line,
                ansi,
                &compiled_keyword_rules,
                &[],
                &[],
                None,
                &[],
                self.palette,
            );
            // Glyph spans intentionally exclude search/selection/cursor state so
            // dynamic overlays do not invalidate shaped base rows.
            let mut spans = background_spans.clone();
            if !decorations.link_ranges.is_empty() {
                spans = apply_action_link_ranges(spans, &decorations.link_ranges, self.palette);
            }
            if !decorations.active_search_ranges.is_empty() {
                spans = apply_search_ranges(
                    spans,
                    &decorations.active_search_ranges,
                    true,
                    self.palette,
                );
            }

            // Cell / keyword backgrounds.
            let mut col = 0usize;
            let mut pending_bg: Option<(u32, usize, usize)> = None;
            for span in &background_spans {
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
            let row_key = self.row_layout_key_with_keyword_key(
                row,
                display_line,
                ansi,
                decorations,
                keyword_rules_key,
            );
            plan.text_run_count = plan.text_run_count.saturating_add(text_runs.len());
            let (shaped, did_shape, shape_duration) =
                self.shape_row(row, row_key, text, text_runs, font_size, window);
            if did_shape {
                plan.shape_line_count = plan.shape_line_count.saturating_add(1);
                plan.shape_line_duration += shape_duration;
            }
            plan.rows.push(TerminalPaintRow { y, line: shaped });
        }

        // Graphics protocol placements (Kitty / iTerm2 / Sixel).
        // Kitty z>0 places above text; everything else stays under the glyph layer.
        for image in &self.snapshot.images {
            if image.width_cells == 0 || image.height_cells == 0 {
                continue;
            }
            let image_row_end = image.row.saturating_add(image.height_cells);
            if image_row_end <= visible_row_start || image.row >= visible_row_end {
                continue;
            }
            let x = px(f32::from(bounds.left())
                + (image.col as f32 - image.source_col_cells as f32) * cell_w);
            let y = px(f32::from(bounds.top())
                + visual_y_offset
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
            let left =
                (f32::from(bounds.left()) + self.snapshot.cursor_col as f32 * cell_w).floor();
            let top = (f32::from(bounds.top())
                + visual_y_offset
                + self.snapshot.cursor_row as f32 * cell_h)
                .floor();
            let right =
                (f32::from(bounds.left()) + (self.snapshot.cursor_col + 1) as f32 * cell_w).ceil();
            let bottom = (f32::from(bounds.top())
                + visual_y_offset
                + (self.snapshot.cursor_row + 1) as f32 * cell_h)
                .ceil();
            let x = px(left);
            let y = px(top);
            let width = (right - left).max(1.);
            let height = (bottom - top).max(1.);
            let cursor_bounds = match self.cursor_style.as_str() {
                "bar" => Bounds::new(point(x, y), size(px(2.), px(height))),
                "underline" => {
                    Bounds::new(point(x, px((bottom - 2.).floor())), size(px(width), px(2.)))
                }
                _ => Bounds::new(point(x, y), size(px(width), px(height))),
            };
            plan.cursor_background = Some(fill(cursor_bounds, rgb(self.palette.terminal_cursor)));
            if self.cursor_style.as_str() != "bar"
                && self.cursor_style.as_str() != "underline"
                && !terminal_cursor_cell_hidden(&self.snapshot)
            {
                let cursor_line = self
                    .snapshot
                    .lines
                    .get(self.snapshot.cursor_row)
                    .map(String::as_str)
                    .unwrap_or("");
                let cursor_text = terminal_cell_text_at_col(cursor_line, self.snapshot.cursor_col);
                let cursor_runs = vec![TextRun {
                    len: cursor_text.len().max(1),
                    font: terminal_run_font(
                        base_font,
                        false,
                        false,
                        self.normal_weight,
                        self.bold_weight,
                    ),
                    color: rgb(self.palette.terminal_bg).into(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                }];
                let started_at = Instant::now();
                let line = Arc::new(window.text_system().shape_line(
                    SharedString::from(cursor_text),
                    font_size,
                    &cursor_runs,
                    None,
                ));
                plan.shape_line_count = plan.shape_line_count.saturating_add(1);
                plan.shape_line_duration += started_at.elapsed();
                plan.text_run_count = plan.text_run_count.saturating_add(cursor_runs.len());
                plan.cursor_glyph = Some(TerminalCursorGlyphPaint {
                    origin: point(x, y),
                    line,
                });
            }
        }

        let elapsed = started_at.elapsed();
        if elapsed.as_millis() >= TERMINAL_ELEMENT_PREPAINT_SLOW_MS {
            let (cache_hits, cache_misses) = self
                .layout_cache
                .as_ref()
                .and_then(|cache| cache.lock().ok().map(|cache| (cache.hits, cache.misses)))
                .unwrap_or((0, 0));
            let (cache_hit_delta, cache_miss_delta) =
                cache_stats_before.map_or((0, 0), |(before_hits, before_misses)| {
                    (
                        cache_hits.saturating_sub(before_hits),
                        cache_misses.saturating_sub(before_misses),
                    )
                });
            tracing::warn!(
                diagnostic = "terminal_element_prepaint",
                total_ms = elapsed.as_millis(),
                visible_row_start,
                visible_row_end,
                visible_row_count,
                snapshot_rows = self.snapshot.rows,
                snapshot_cols = self.snapshot.cols,
                styled_lines = self.snapshot.styled_lines.len(),
                decorations = self.decorations.len(),
                keyword_rules = self.keyword_rules.len(),
                images = self.snapshot.images.len(),
                backgrounds = plan.backgrounds.len(),
                decoration_backgrounds = plan.decoration_backgrounds.len(),
                active_markers = plan.active_markers.len(),
                shaped_rows = plan.rows.len(),
                shape_line_count = plan.shape_line_count,
                shape_line_ms = plan.shape_line_duration.as_millis(),
                text_run_count = plan.text_run_count,
                images_under = plan.images_under.len(),
                images_above = plan.images_above.len(),
                placeholders_under = plan.placeholders_under.len(),
                placeholders_above = plan.placeholders_above.len(),
                cache_hit_delta,
                cache_miss_delta,
                cache_hits,
                cache_misses,
                "slow terminal element prepaint"
            );
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
        let started_at = Instant::now();
        let backgrounds = prepaint.backgrounds.len();
        let images_under = prepaint.images_under.len();
        let placeholders_under = prepaint.placeholders_under.len();
        let decoration_backgrounds = prepaint.decoration_backgrounds.len();
        let active_markers = prepaint.active_markers.len();
        let shaped_rows = prepaint.rows.len();
        let images_above = prepaint.images_above.len();
        let placeholders_above = prepaint.placeholders_above.len();
        let cursor = prepaint.cursor_background.is_some();
        let cursor_glyph = prepaint.cursor_glyph.is_some();
        let shape_line_count = prepaint.shape_line_count;
        let shape_line_ms = prepaint.shape_line_duration.as_millis();
        let text_run_count = prepaint.text_run_count;
        let viewport_mask = ContentMask { bounds };
        window.with_content_mask(Some(viewport_mask), |window| {
            // cell/keyword bg → under images → search/selection → marks → text → above images → cursor
            for quad in prepaint.backgrounds.drain(..) {
                window.paint_quad(quad);
            }
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
            if let Some(cursor) = prepaint.cursor_background.take() {
                window.paint_quad(cursor);
            }
            if let Some(cursor_glyph) = prepaint.cursor_glyph.take() {
                let _ = cursor_glyph.line.paint(
                    cursor_glyph.origin,
                    px(self.cell_height.max(1.)),
                    window,
                    cx,
                );
            }
        });
        let elapsed = started_at.elapsed();
        if elapsed.as_millis() >= TERMINAL_ELEMENT_PAINT_SLOW_MS {
            tracing::warn!(
                diagnostic = "terminal_element_paint",
                total_ms = elapsed.as_millis(),
                snapshot_rows = self.snapshot.rows,
                snapshot_cols = self.snapshot.cols,
                backgrounds,
                decoration_backgrounds,
                active_markers,
                shaped_rows,
                images_under,
                images_above,
                placeholders_under,
                placeholders_above,
                cursor,
                cursor_glyph,
                shape_line_count,
                shape_line_ms,
                text_run_count,
                "slow terminal element paint"
            );
        }
    }
}

fn terminal_visible_rows_for_bounds(
    bounds: Bounds<Pixels>,
    cell_h: f32,
    row_limit: usize,
    visual_y_offset: f32,
) -> std::ops::Range<usize> {
    if row_limit == 0 {
        return 0..0;
    }
    let cell_h = cell_h.max(1.);
    let height = f32::from(bounds.size.height).max(0.);
    let overscan_rows = 1usize;
    let visible_start = ((-visual_y_offset) / cell_h).floor().max(0.0) as usize;
    let visible_end = ((height - visual_y_offset) / cell_h).ceil().max(0.0) as usize;
    let start = visible_start.saturating_sub(overscan_rows).min(row_limit);
    let end = visible_end.saturating_add(overscan_rows).min(row_limit);
    if end < start {
        return start..start;
    }
    start..end
}

fn terminal_layout_rows(snapshot_rows: usize, override_rows: Option<usize>) -> usize {
    override_rows.unwrap_or(snapshot_rows).max(1)
}

fn terminal_layout_height_px(
    cell_height: f32,
    snapshot_rows: usize,
    override_rows: Option<usize>,
) -> f32 {
    cell_height.max(1.0) * terminal_layout_rows(snapshot_rows, override_rows) as f32
}
