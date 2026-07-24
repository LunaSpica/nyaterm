use super::*;
use std::collections::{HashMap, VecDeque, hash_map::DefaultHasher};
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
    background_ranges: Vec<TerminalRowBackgroundRange>,
    text_run_count: usize,
}

#[derive(Debug, Clone)]
struct TerminalRowBackgroundRange {
    bg: u32,
    start: usize,
    end: usize,
}

#[derive(Debug, Default)]
pub struct NyaTerminalLayoutCache {
    rows: HashMap<u64, Arc<CachedTerminalPaintRow>>,
    row_order: VecDeque<u64>,
    keyword_rules_source: Option<Arc<Vec<ResolvedKeywordHighlightRule>>>,
    keyword_rules_key: u64,
    compiled_keyword_key: Option<u64>,
    compiled_keyword_rules: Arc<CompiledKeywordRules>,
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
        self.row_order.clear();
        self.keyword_rules_source = None;
        self.keyword_rules_key = 0;
        self.compiled_keyword_key = None;
        self.compiled_keyword_rules = Arc::default();
        self.hits = 0;
        self.misses = 0;
        self.shape_calls = 0;
        self.shape_duration_us = 0;
    }

    fn keyword_rules_key(&mut self, rules: &Arc<Vec<ResolvedKeywordHighlightRule>>) -> u64 {
        if let Some(cached) = self.keyword_rules_source.as_ref() {
            if Arc::ptr_eq(cached, rules) {
                return self.keyword_rules_key;
            }
            if cached.as_ref() == rules.as_ref() {
                self.keyword_rules_source = Some(Arc::clone(rules));
                return self.keyword_rules_key;
            }
        }
        self.keyword_rules_key = terminal_keyword_rules_key(rules);
        self.keyword_rules_source = Some(Arc::clone(rules));
        self.keyword_rules_key
    }

    fn compiled_keyword_rules(
        &mut self,
        key: u64,
        rules: &[ResolvedKeywordHighlightRule],
    ) -> Arc<CompiledKeywordRules> {
        if self.compiled_keyword_key == Some(key) {
            return Arc::clone(&self.compiled_keyword_rules);
        }
        self.compiled_keyword_key = Some(key);
        self.compiled_keyword_rules = Arc::new(compile_keyword_rules(rules));
        Arc::clone(&self.compiled_keyword_rules)
    }

    #[cfg(test)]
    fn shaped_line(
        &mut self,
        _row: usize,
        key: u64,
        shape: impl FnOnce() -> (Arc<ShapedLine>, std::time::Duration),
    ) -> (Arc<ShapedLine>, bool, std::time::Duration) {
        let (row, did_shape, duration) = self.paint_row(_row, key, || {
            let (line, duration) = shape();
            (line, duration, 0, Vec::new())
        });
        (Arc::clone(&row.line), did_shape, duration)
    }

    fn paint_row(
        &mut self,
        _row: usize,
        key: u64,
        build: impl FnOnce() -> (
            Arc<ShapedLine>,
            std::time::Duration,
            usize,
            Vec<TerminalRowBackgroundRange>,
        ),
    ) -> (Arc<CachedTerminalPaintRow>, bool, std::time::Duration) {
        if let Some(cached) = self.rows.get(&key) {
            self.hits = self.hits.saturating_add(1);
            return (Arc::clone(cached), false, std::time::Duration::ZERO);
        }
        self.misses = self.misses.saturating_add(1);
        if self.rows.len() >= TERMINAL_LAYOUT_CACHE_ROW_CAP {
            self.evict_oldest_row();
        }
        let (line, duration, text_run_count, background_ranges) = build();
        self.shape_calls = self.shape_calls.saturating_add(1);
        self.shape_duration_us = self
            .shape_duration_us
            .saturating_add(duration.as_micros().min(u128::from(u64::MAX)) as u64);
        let row = Arc::new(CachedTerminalPaintRow {
            line: Arc::clone(&line),
            background_ranges,
            text_run_count,
        });
        self.rows.insert(key, Arc::clone(&row));
        self.row_order.push_back(key);
        (row, true, duration)
    }

    fn evict_oldest_row(&mut self) {
        while self.rows.len() >= TERMINAL_LAYOUT_CACHE_ROW_CAP {
            let Some(key) = self.row_order.pop_front() else {
                self.rows.clear();
                return;
            };
            if self.rows.remove(&key).is_some() {
                return;
            }
        }
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
        assert_eq!(cache.row_order.len(), 1);
    }

    #[test]
    fn keyword_rules_key_reuses_equal_rule_sets() {
        let mut cache = NyaTerminalLayoutCache::default();
        let first = Arc::new(vec![ResolvedKeywordHighlightRule {
            id: "error".to_string(),
            name: "Error".to_string(),
            patterns: vec!["error".to_string()],
            color: "#ff0000".to_string(),
            enabled: true,
        }]);
        let second = Arc::new(vec![ResolvedKeywordHighlightRule {
            id: "error".to_string(),
            name: "Error".to_string(),
            patterns: vec!["error".to_string()],
            color: "#ff0000".to_string(),
            enabled: true,
        }]);

        let key = cache.keyword_rules_key(&first);
        assert_eq!(cache.keyword_rules_key(&second), key);
        assert!(
            cache
                .keyword_rules_source
                .as_ref()
                .is_some_and(|cached| Arc::ptr_eq(cached, &second))
        );
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
    fn row_cache_evicts_incrementally_when_full() {
        let mut cache = NyaTerminalLayoutCache::default();
        for key in 0..=TERMINAL_LAYOUT_CACHE_ROW_CAP as u64 {
            let _ = cache.paint_row(0, key, || {
                (
                    Arc::new(ShapedLine::default()),
                    std::time::Duration::ZERO,
                    1,
                    Vec::new(),
                )
            });
        }

        assert_eq!(cache.rows.len(), TERMINAL_LAYOUT_CACHE_ROW_CAP);
        assert!(!cache.rows.contains_key(&0));
        assert!(cache.rows.contains_key(&1));
        assert!(
            cache
                .rows
                .contains_key(&(TERMINAL_LAYOUT_CACHE_ROW_CAP as u64))
        );

        let _ = cache.paint_row(0, 1, || {
            panic!("remaining rows should survive cache pressure");
        });
        assert_eq!(cache.hits, 1);
    }

    #[test]
    fn styled_span_hash_tracks_style_changes() {
        let plain = vec![nyaterm_terminal::StyledSpan {
            text: "same".to_string(),
            style: nyaterm_terminal::CellStyle::default(),
        }];
        let bold_style = nyaterm_terminal::CellStyle {
            bold: true,
            ..Default::default()
        };
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
    fn row_layout_key_falls_back_to_styled_spans_without_signature() {
        let mut snapshot = TerminalScreen::default().snapshot();
        snapshot.lines[0] = "same".to_string();
        snapshot.line_signatures.clear();
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
        let bold_style = nyaterm_terminal::CellStyle {
            bold: true,
            ..Default::default()
        };
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
    fn row_layout_key_uses_authoritative_line_signature() {
        let mut first_snapshot = TerminalScreen::default().snapshot();
        first_snapshot.line_signatures[0] = 7;
        let mut second_snapshot = first_snapshot.clone();
        second_snapshot.line_signatures[0] = 8;
        let make_element = |snapshot| {
            NyaTerminalElement::new(
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
            )
        };
        let first = make_element(first_snapshot);
        let second = make_element(second_snapshot);
        let decorations = TerminalLineDecorations::default();

        assert_ne!(
            first.row_layout_key(0, "same", None, &decorations),
            second.row_layout_key(0, "same", None, &decorations),
        );
    }

    #[test]
    fn row_layout_key_tracks_cell_width() {
        let mut snapshot = TerminalScreen::default().snapshot();
        snapshot.lines[0] = "same".to_string();
        snapshot.line_signatures[0] = 7;
        let make_element = |cell_width| {
            NyaTerminalElement::new(
                Arc::new(snapshot.clone()),
                Arc::new(Vec::new()),
                Vec::new(),
                false,
                "block",
                cell_width,
                16.0,
                nyaterm_ui::theme_palette("github-dark"),
                "monospace".to_string(),
                14.0,
                400.0,
                700.0,
            )
        };
        let narrow = make_element(8.0);
        let wide = make_element(12.0);
        let decorations = TerminalLineDecorations::default();

        assert_ne!(
            narrow.row_layout_key(0, "same", None, &decorations),
            wide.row_layout_key(0, "same", None, &decorations),
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
    fn paint_row_cache_reuses_full_row_payload() {
        let mut cache = NyaTerminalLayoutCache::default();
        let mut build_calls = 0usize;

        let (row, did_shape, duration) = cache.paint_row(0, 42, || {
            build_calls += 1;
            (
                Arc::new(ShapedLine::default()),
                std::time::Duration::ZERO,
                3,
                vec![TerminalRowBackgroundRange {
                    bg: 0xff00ff,
                    start: 2,
                    end: 4,
                }],
            )
        });

        assert!(did_shape);
        assert_eq!(duration, std::time::Duration::ZERO);
        assert_eq!(build_calls, 1);
        assert_eq!(row.text_run_count, 3);
        assert_eq!(row.background_ranges.len(), 1);

        let (cached, did_shape, duration) = cache.paint_row(0, 42, || {
            panic!("cached row should not rebuild");
        });

        assert!(!did_shape);
        assert_eq!(duration, std::time::Duration::ZERO);
        assert_eq!(build_calls, 1);
        assert_eq!(cached.text_run_count, 3);
        assert_eq!(cached.background_ranges[0].bg, 0xff00ff);
        assert_eq!(cached.background_ranges[0].start, 2);
        assert_eq!(cached.background_ranges[0].end, 4);
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
    fn row_layout_key_tracks_precomputed_keyword_presence() {
        let mut snapshot = TerminalScreen::default().snapshot();
        snapshot.lines[0] = "ERROR".to_string();
        snapshot.line_signatures[0] = 7;
        let keyword_rule = ResolvedKeywordHighlightRule {
            id: "errors".to_string(),
            name: "Errors".to_string(),
            patterns: vec!["ERROR".to_string()],
            color: "#ff0000".to_string(),
            enabled: true,
        };
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
        let keyword_rules_key = terminal_keyword_rules_key(&[keyword_rule]);

        assert_ne!(
            element.row_layout_key_with_keyword_key(
                0,
                "ERROR",
                None,
                &decorations,
                keyword_rules_key,
                false,
            ),
            element.row_layout_key_with_keyword_key(
                0,
                "ERROR",
                None,
                &decorations,
                keyword_rules_key,
                true,
            ),
        );
    }

    #[test]
    fn row_layout_key_ignores_keyword_rules_for_known_empty_keyword_rows() {
        let mut snapshot = TerminalScreen::default().snapshot();
        snapshot.lines[0] = "plain".to_string();
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

        assert_eq!(
            element.row_layout_key_with_keyword_state(
                0,
                "plain",
                None,
                &decorations,
                11,
                false,
                true,
            ),
            element.row_layout_key_with_keyword_state(
                0,
                "plain",
                None,
                &decorations,
                99,
                false,
                true,
            ),
        );
    }

    #[test]
    fn row_layout_key_keeps_keyword_rules_for_pending_keyword_rows() {
        let mut snapshot = TerminalScreen::default().snapshot();
        snapshot.lines[0] = "plain".to_string();
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

        assert_ne!(
            element.row_layout_key_with_keyword_state(
                0,
                "plain",
                None,
                &decorations,
                11,
                false,
                false,
            ),
            element.row_layout_key_with_keyword_state(
                0,
                "plain",
                None,
                &decorations,
                99,
                false,
                false,
            ),
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
    fn visible_rows_use_parent_content_mask_intersection() {
        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(100.), px(160.)));
        let clipped = Bounds::new(point(px(0.), px(64.)), size(px(100.), px(32.)));

        assert_eq!(
            terminal_visible_rows_for_clipped_bounds(bounds, clipped, 16., 10, 0.0),
            3..7
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
    keyword_highlights: Option<Arc<TerminalKeywordHighlightSnapshot>>,
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
    fill_height: bool,
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
            keyword_highlights: None,
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
            fill_height: false,
        }
    }

    pub fn with_layout_cache(mut self, cache: Arc<Mutex<NyaTerminalLayoutCache>>) -> Self {
        self.layout_cache = Some(cache);
        self
    }

    pub fn with_keyword_highlights(
        mut self,
        highlights: Arc<TerminalKeywordHighlightSnapshot>,
    ) -> Self {
        self.keyword_highlights = Some(highlights);
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

    /// Let the parent viewport own the element height, like an editor viewport.
    /// The snapshot row count still limits which rows are painted.
    pub fn with_fill_height(mut self, fill: bool) -> Self {
        self.fill_height = fill;
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
            false,
        )
    }

    #[cfg(test)]
    fn row_layout_key_with_keyword_key(
        &self,
        row: usize,
        display_line: &str,
        ansi_spans: Option<&[nyaterm_terminal::StyledSpan]>,
        decorations: &TerminalLineDecorations,
        keyword_rules_key: u64,
        keyword_spans_present: bool,
    ) -> u64 {
        self.row_layout_key_with_keyword_state(
            row,
            display_line,
            ansi_spans,
            decorations,
            keyword_rules_key,
            keyword_spans_present,
            false,
        )
    }

    #[cfg(test)]
    fn row_layout_key_with_keyword_state(
        &self,
        row: usize,
        display_line: &str,
        ansi_spans: Option<&[nyaterm_terminal::StyledSpan]>,
        decorations: &TerminalLineDecorations,
        keyword_rules_key: u64,
        keyword_spans_present: bool,
        keyword_result_known_empty: bool,
    ) -> u64 {
        let effective_keyword_rules_key =
            terminal_effective_keyword_rules_key(keyword_rules_key, keyword_result_known_empty);
        let paint_style_key = self.paint_style_key(effective_keyword_rules_key);
        let mut hasher = DefaultHasher::new();
        if let Some(signature) = self.snapshot.line_signatures.get(row) {
            signature.hash(&mut hasher);
        } else {
            // Synthetic callers without terminal row signatures retain the
            // defensive content/style fallback.
            display_line.hash(&mut hasher);
            hash_styled_spans(ansi_spans, &mut hasher);
        }
        hash_stable_glyph_decorations(decorations, &mut hasher);
        keyword_spans_present.hash(&mut hasher);
        paint_style_key.hash(&mut hasher);
        hasher.finish()
    }

    fn paint_style_key(&self, keyword_rules_key: u64) -> u64 {
        let mut hasher = DefaultHasher::new();
        keyword_rules_key.hash(&mut hasher);
        self.palette.bg.hash(&mut hasher);
        self.palette.surface.hash(&mut hasher);
        self.palette.accent.hash(&mut hasher);
        self.palette.warning.hash(&mut hasher);
        self.palette.terminal_fg.hash(&mut hasher);
        self.palette.terminal_bg.hash(&mut hasher);
        self.palette.terminal_ansi.hash(&mut hasher);
        self.font_family.hash(&mut hasher);
        self.font_size.to_bits().hash(&mut hasher);
        self.normal_weight.to_bits().hash(&mut hasher);
        self.bold_weight.to_bits().hash(&mut hasher);
        self.cell_width.max(1.0).to_bits().hash(&mut hasher);
        hasher.finish()
    }

    fn keyword_rules_key(&self) -> u64 {
        terminal_keyword_rules_key(&self.keyword_rules)
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

fn terminal_effective_keyword_rules_key(keyword_rules_key: u64, known_empty: bool) -> u64 {
    if known_empty { 0 } else { keyword_rules_key }
}

fn terminal_background_ranges_for_spans(
    spans: &[TerminalHighlightSpan],
    palette: nyaterm_ui::ThemePalette,
) -> Vec<TerminalRowBackgroundRange> {
    let mut out = Vec::new();
    let mut col = 0usize;
    let mut pending_bg: Option<TerminalRowBackgroundRange> = None;
    for span in spans {
        let bg = span.bg.or_else(|| span.keyword.then_some(palette.surface));
        let span_cols = terminal_cell_count(&span.text).max(1);
        if let Some(bg) = bg {
            match pending_bg.as_mut() {
                Some(current) if current.bg == bg && current.end == col => {
                    current.end = col + span_cols;
                }
                _ => {
                    if let Some(range) = pending_bg.take() {
                        out.push(range);
                    }
                    pending_bg = Some(TerminalRowBackgroundRange {
                        bg,
                        start: col,
                        end: col + span_cols,
                    });
                }
            }
        } else if let Some(range) = pending_bg.take() {
            out.push(range);
        }
        col += span_cols;
    }
    if let Some(range) = pending_bg.take() {
        out.push(range);
    }
    out
}

fn push_terminal_background_ranges(
    row: usize,
    ranges: &[TerminalRowBackgroundRange],
    bounds: Bounds<Pixels>,
    cell_w: f32,
    cell_h: f32,
    out: &mut Vec<PaintQuad>,
) {
    for range in ranges {
        flush_bg(
            Some((range.bg, range.start, range.end)),
            row,
            bounds,
            cell_w,
            cell_h,
            out,
        );
    }
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
        style.size.height = if self.fill_height {
            relative(1.).into()
        } else {
            px(terminal_layout_height_px(
                self.cell_height,
                self.snapshot.rows,
                self.layout_rows,
            ))
            .into()
        };
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
        let visible_bounds = window.content_mask().bounds.intersect(&bounds);
        if visible_bounds.size.width <= px(0.) || visible_bounds.size.height <= px(0.) {
            return NyaTerminalPaintPlan::default();
        }
        let layout_cache = self.layout_cache.clone();
        let mut layout_cache = layout_cache.as_ref().and_then(|cache| cache.lock().ok());
        let cache_stats_before = layout_cache
            .as_ref()
            .map(|cache| (cache.hits, cache.misses));
        let mut plan = NyaTerminalPaintPlan::default();
        let cell_w = self.cell_width.max(1.);
        let scale_factor = window.scale_factor();
        let cell_h = nyaterm_core::terminal_snapped_cell_height(self.cell_height, scale_factor);
        let font_size = px(self.font_size.max(8.));
        let base_font = font(SharedString::from(self.font_family.clone()));
        let keyword_rules_key = if let Some(highlights) = self.keyword_highlights.as_ref() {
            highlights.rules_key()
        } else if self.keyword_rules.is_empty() {
            0
        } else if let Some(cache) = layout_cache.as_deref_mut() {
            cache.keyword_rules_key(&self.keyword_rules)
        } else {
            self.keyword_rules_key()
        };
        let compiled_keyword_rules = if self.keyword_rules.is_empty() {
            Arc::default()
        } else if let Some(cache) = layout_cache.as_deref_mut() {
            cache.compiled_keyword_rules(keyword_rules_key, self.keyword_rules.as_slice())
        } else {
            Arc::new(compile_keyword_rules(self.keyword_rules.as_slice()))
        };

        let visual_y_offset = self.visual_y_offset;
        let visible_rows = terminal_visible_rows_for_clipped_bounds(
            bounds,
            visible_bounds,
            cell_h,
            self.snapshot.rows,
            visual_y_offset,
        );
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
            let line_signature = self.snapshot.line_signatures.get(row).copied();
            let keyword_lookup = self.keyword_highlights.as_ref().and_then(|highlights| {
                highlights.lookup(row, line_signature).or_else(|| {
                    highlights.stale_lookup(
                        row,
                        line_signature,
                        self.snapshot.display_offset,
                        self.snapshot.rows,
                    )
                })
            });
            let keyword_result_known_empty = keyword_lookup
                .as_ref()
                .is_some_and(|lookup| lookup.is_known_empty());
            let keyword_spans = keyword_lookup.as_ref().and_then(|lookup| lookup.spans());
            let keyword_spans_present = keyword_spans.is_some();
            let effective_keyword_rules_key =
                terminal_effective_keyword_rules_key(keyword_rules_key, keyword_result_known_empty);
            let row_paint_style_key = self.paint_style_key(effective_keyword_rules_key);
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

            let mut row_key_hasher = DefaultHasher::new();
            if let Some(signature) = self.snapshot.line_signatures.get(row) {
                signature.hash(&mut row_key_hasher);
            } else {
                display_line.hash(&mut row_key_hasher);
                hash_styled_spans(ansi, &mut row_key_hasher);
            }
            hash_stable_glyph_decorations(decorations, &mut row_key_hasher);
            keyword_spans_present.hash(&mut row_key_hasher);
            row_paint_style_key.hash(&mut row_key_hasher);
            let row_key = row_key_hasher.finish();
            let build_row = |window: &mut Window| {
                let row_keyword_rules: &[ResolvedKeywordHighlightRule] =
                    if keyword_result_known_empty {
                        &[]
                    } else {
                        self.keyword_rules.as_slice()
                    };
                if keyword_spans.is_none()
                    && terminal_plain_row_fast_path(ansi, row_keyword_rules, decorations)
                {
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
                    let line_started_at = Instant::now();
                    let line = Arc::new(window.text_system().shape_line(
                        SharedString::from(text),
                        font_size,
                        &text_runs,
                        Some(px(cell_w)),
                    ));
                    return (line, line_started_at.elapsed(), text_runs.len(), Vec::new());
                }

                // Base spans drive cell/keyword backgrounds only (under images).
                let background_spans = keyword_spans
                    .map(|spans| spans.as_ref().clone())
                    .unwrap_or_else(|| {
                        let row_compiled_keyword_rules: &[(regex::Regex, u32)] =
                            if keyword_result_known_empty {
                                &[]
                            } else {
                                compiled_keyword_rules.as_slice()
                            };
                        terminal_highlight_spans_compiled(
                            display_line,
                            ansi,
                            row_compiled_keyword_rules,
                            &[],
                            &[],
                            None,
                            &[],
                            self.palette,
                        )
                    });
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
                let background_ranges =
                    terminal_background_ranges_for_spans(&background_spans, self.palette);

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
                let line_started_at = Instant::now();
                let line = Arc::new(window.text_system().shape_line(
                    SharedString::from(text),
                    font_size,
                    &text_runs,
                    Some(px(cell_w)),
                ));
                (
                    line,
                    line_started_at.elapsed(),
                    text_runs.len(),
                    background_ranges,
                )
            };
            let (painted_row, did_shape, shape_duration) =
                if let Some(cache) = layout_cache.as_deref_mut() {
                    cache.paint_row(row, row_key, || build_row(window))
                } else {
                    let (line, duration, text_run_count, background_ranges) = build_row(window);
                    (
                        Arc::new(CachedTerminalPaintRow {
                            line,
                            background_ranges,
                            text_run_count,
                        }),
                        true,
                        duration,
                    )
                };
            push_terminal_background_ranges(
                row,
                &painted_row.background_ranges,
                bounds,
                cell_w,
                cell_h,
                &mut plan.backgrounds,
            );
            plan.text_run_count = plan
                .text_run_count
                .saturating_add(painted_row.text_run_count);
            if did_shape {
                plan.shape_line_count = plan.shape_line_count.saturating_add(1);
                plan.shape_line_duration += shape_duration;
            }
            plan.rows.push(TerminalPaintRow {
                y,
                line: Arc::clone(&painted_row.line),
            });
        }
        let cache_stats_after = layout_cache
            .as_ref()
            .map(|cache| (cache.hits, cache.misses));
        drop(layout_cache);

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
            && self.snapshot.cursor_row >= visible_row_start
            && self.snapshot.cursor_row < visible_row_end
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
                    Some(px(cell_w)),
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
            let (cache_hits, cache_misses) = cache_stats_after.unwrap_or((0, 0));
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
        let cell_height =
            nyaterm_core::terminal_snapped_cell_height(self.cell_height, window.scale_factor());
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
        let viewport_mask = ContentMask {
            bounds: window.content_mask().bounds.intersect(&bounds),
        };
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
                let _ = row
                    .line
                    .paint(point(bounds.left(), row.y), px(cell_height), window, cx);
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
                let _ = cursor_glyph
                    .line
                    .paint(cursor_glyph.origin, px(cell_height), window, cx);
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

#[cfg(test)]
fn terminal_visible_rows_for_bounds(
    bounds: Bounds<Pixels>,
    cell_h: f32,
    row_limit: usize,
    visual_y_offset: f32,
) -> std::ops::Range<usize> {
    terminal_visible_rows_for_clipped_bounds(bounds, bounds, cell_h, row_limit, visual_y_offset)
}

fn terminal_visible_rows_for_clipped_bounds(
    bounds: Bounds<Pixels>,
    visible_bounds: Bounds<Pixels>,
    cell_h: f32,
    row_limit: usize,
    visual_y_offset: f32,
) -> std::ops::Range<usize> {
    if row_limit == 0 {
        return 0..0;
    }
    let cell_h = cell_h.max(1.);
    let visible_top = f32::from(visible_bounds.top() - bounds.top());
    let visible_bottom = f32::from(visible_bounds.bottom() - bounds.top());
    let overscan_rows = 1usize;
    let visible_start = ((visible_top - visual_y_offset) / cell_h).floor().max(0.0) as usize;
    let visible_end = ((visible_bottom - visual_y_offset) / cell_h)
        .ceil()
        .max(0.0) as usize;
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
