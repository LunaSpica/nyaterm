use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::models::TerminalBufferCellPos;
use crate::models::{TerminalProtocolState, TerminalSelection};
use crate::terminal::{
    TerminalLineDecorations, compile_terminal_keyword_highlighter, terminal_keyword_rules_key,
};
use gpui::prelude::*;
use gpui::{
    AppContext, Context, Entity, Render, TestAppContext, Window, bounds, div, point, px, size,
};
use nyaterm_terminal::{TerminalScreen, TerminalSnapshot};
use nyaterm_terminal_gpui::precompute_terminal_keyword_highlights;

use super::{
    TERMINAL_KEYWORD_HIGHLIGHT_PREFETCH_VIEWPORTS,
    TERMINAL_KEYWORD_HIGHLIGHT_PRESSURE_PREFETCH_VIEWPORTS,
    TERMINAL_KEYWORD_HIGHLIGHT_PRESSURE_THROTTLE, TerminalKeywordHighlightRequestKey,
    TerminalPaintedHitTestGeometry, TerminalScrollVisualState, TerminalSurface,
    TerminalSurfaceFrameSnapshot, TerminalSurfaceLocalScrollResult, TerminalVisualScrollGeometry,
    empty_terminal_keyword_rules, terminal_effective_visual_scroll_offset_px,
    terminal_gutter_metrics, terminal_keyword_highlight_prefetch_rows,
    terminal_keyword_highlight_prefetch_viewports, terminal_keyword_highlight_pressure_delay,
    terminal_keyword_highlight_request_key, terminal_keyword_highlight_visible_rows,
    terminal_line_number_digits, terminal_selection_visual_row_range,
    terminal_selection_visual_row_union, terminal_snapshot_anchor_row_for_display_offset,
    terminal_snapshot_covers_display_offset, terminal_surface_fractional_prefetch_offset,
    terminal_surface_synthesized_window_extra_rows, terminal_surface_text_first_repaint_ready,
    terminal_surface_visible_rows_for_viewport, terminal_visual_scroll_offset_px,
};

struct TerminalSurfaceLayoutTestView {
    surface: Entity<TerminalSurface>,
}

impl Render for TerminalSurfaceLayoutTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .min_w_0()
            .min_h_0()
            .child(self.surface.clone())
    }
}

fn rendered_terminal_grid_bounds(show_line_numbers: bool) -> gpui::Bounds<gpui::Pixels> {
    let mut cx = TestAppContext::single();
    let surface = cx.new(|_| TerminalSurface::new("session"));
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    cx.update_entity(&surface, |surface, _cx| {
        surface.show_line_numbers = show_line_numbers;
        surface.apply_frame_snapshot(
            TerminalSurfaceFrameSnapshot::new(
                snapshot,
                TerminalScrollVisualState {
                    session_id: "session".to_string(),
                    scroll_offset: 0,
                    scroll_residual_lines: 0.0,
                    display_offset: 0,
                    scrollback_len: 0,
                    viewport_rows: rows,
                    has_new_while_scrolled: false,
                    performance_overlay: None,
                    skipped_output_chars: 0,
                },
            )
            .with_presentation(false, true, "block"),
        );
    });
    let window = cx.open_window(size(px(400.0), px(240.0)), {
        let surface = surface.clone();
        move |_window, _cx| TerminalSurfaceLayoutTestView { surface }
    });

    cx.update_window(window.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();

    cx.read_entity(&surface, |surface, _cx| {
        let geometry = surface.painted_hit_test_geometry.expect("painted geometry");
        geometry.grid_bounds.expect("painted grid bounds")
    })
}

#[test]
fn painted_hit_test_grid_bounds_match_the_rendered_grid_layout() {
    let grid_bounds = rendered_terminal_grid_bounds(false);

    assert_eq!(grid_bounds.origin, point(px(0.0), px(0.0)));
    assert_eq!(grid_bounds.size.height, px(240.0));
}

#[test]
fn painted_hit_test_grid_bounds_start_after_the_rendered_gutter() {
    let snapshot = TerminalScreen::default().viewport_snapshot(0);
    let expected_gutter = terminal_gutter_metrics(
        8.0,
        false,
        false,
        true,
        terminal_line_number_digits(&snapshot),
    )
    .total_width();
    let grid_bounds = rendered_terminal_grid_bounds(true);

    assert_eq!(grid_bounds.origin, point(px(expected_gutter), px(0.0)));
    assert_eq!(grid_bounds.size.height, px(240.0));
}

#[test]
fn painted_hit_test_state_uses_the_latest_grid_bounds() {
    let mut surface = TerminalSurface::new("session");
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    surface.painted_hit_test_snapshot = Some(snapshot);
    surface.painted_hit_test_geometry = Some(TerminalPaintedHitTestGeometry {
        grid_bounds: None,
        display_offset: 0,
        viewport_anchor_row: 0,
        snapshot_rows: 24,
        viewport_rows: 24,
        visual_y_offset: 0.0,
        cell_width: 8.0,
        cell_height: 16.0,
        revision: 1,
    });
    let grid_bounds = bounds(point(px(87.0), px(31.0)), size(px(640.0), px(384.0)));

    assert!(surface.set_painted_hit_test_grid_bounds(grid_bounds));
    assert!(!surface.set_painted_hit_test_grid_bounds(grid_bounds));
    assert_eq!(
        surface
            .painted_hit_test_state()
            .map(|(geometry, _)| geometry.grid_bounds),
        Some(Some(grid_bounds))
    );
}

macro_rules! apply_test_frame_snapshot {
    (
        $surface:expr,
        $snapshot:expr,
        $scroll_offset:expr,
        $scroll_residual_lines:expr,
        $display_offset:expr,
        $scrollback_len:expr,
        $viewport_rows:expr,
        $has_new_while_scrolled:expr,
        $performance_overlay:expr,
        $skipped_output_chars:expr,
        $has_action_link_decorations:expr,
        $show_cursor:expr,
        $cursor_style:expr $(,)?
    ) => {{
        $surface.apply_frame_snapshot(
            TerminalSurfaceFrameSnapshot::new(
                $snapshot,
                TerminalScrollVisualState {
                    session_id: "session".to_string(),
                    scroll_offset: $scroll_offset,
                    scroll_residual_lines: $scroll_residual_lines,
                    display_offset: $display_offset,
                    scrollback_len: $scrollback_len,
                    viewport_rows: $viewport_rows,
                    has_new_while_scrolled: false,
                    performance_overlay: None,
                    skipped_output_chars: 0,
                },
            )
            .with_output_state(
                $has_new_while_scrolled,
                $performance_overlay,
                $skipped_output_chars,
            )
            .with_presentation(
                $has_action_link_decorations,
                $show_cursor,
                $cursor_style,
            ),
        )
    }};
}

fn terminal_test_output_lines(count: usize) -> String {
    (0..count)
        .map(|index| format!("line {index:03}\n"))
        .collect::<String>()
}

#[test]
fn empty_terminal_keyword_rules_reuses_arc() {
    let first = empty_terminal_keyword_rules();
    let second = empty_terminal_keyword_rules();

    assert!(first.is_empty());
    assert!(Arc::ptr_eq(&first, &second));
}

fn terminal_test_retained_live_snapshot(count: usize) -> (TerminalSnapshot, usize) {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text(&terminal_test_output_lines(count));
    let base = screen.viewport_snapshot(0);
    let viewport_rows = base.row_count().max(1);
    let older = screen.viewport_snapshot(viewport_rows);
    let retained_older_rows = older.row_count().min(viewport_rows);
    let mut snapshot = base;
    if retained_older_rows == 0 || snapshot.cols == 0 {
        return (snapshot, viewport_rows);
    }

    let older_start = older.row_count().saturating_sub(retained_older_rows);
    let mut rows = older.rows()[older_start..].to_vec();
    rows.extend(snapshot.rows().iter().cloned());
    snapshot.row_data = rows.into();
    (snapshot, viewport_rows)
}

#[test]
fn retained_row_cache_refreshes_only_changed_snapshot_rows() {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text(&terminal_test_output_lines(80));
    let snapshot = screen.viewport_snapshot(0);
    let mut surface = TerminalSurface::new("session");

    assert_eq!(
        surface.remember_retained_snapshot_rows(&snapshot),
        snapshot.row_count()
    );
    assert_eq!(surface.remember_retained_snapshot_rows(&snapshot), 0);

    let mut metadata_changed = snapshot.clone();
    let rows = Arc::make_mut(&mut metadata_changed.row_data);
    Arc::make_mut(&mut rows[0]).timestamp_ms = Some(42);
    assert_eq!(
        surface.remember_retained_snapshot_rows(&metadata_changed),
        1
    );
}

#[test]
fn synthesized_scroll_window_uses_bounded_overscan() {
    assert_eq!(terminal_surface_synthesized_window_extra_rows(8), 32);
    assert_eq!(terminal_surface_synthesized_window_extra_rows(40), 80);
    assert_eq!(terminal_surface_synthesized_window_extra_rows(200), 192);
}

#[test]
fn visual_scroll_offset_tracks_target_display_and_residual() {
    assert_eq!(terminal_visual_scroll_offset_px(0, 0, 0.0, 16.0), 0.0);
    assert_eq!(terminal_visual_scroll_offset_px(1, 0, 0.25, 16.0), 20.0);
    assert_eq!(terminal_visual_scroll_offset_px(0, 1, -0.25, 16.0), -20.0);
    assert_eq!(terminal_visual_scroll_offset_px(4, 4, 0.5, 20.0), 10.0);
    assert_eq!(terminal_visual_scroll_offset_px(20, 0, 0.0, 16.0), 320.0);
    assert_eq!(
        terminal_effective_visual_scroll_offset_px(TerminalVisualScrollGeometry {
            snapshot_pending: true,
            target_offset: 1,
            displayed_offset: 0,
            residual_lines: 0.25,
            viewport_anchor_row: 1,
            snapshot_rows: 4,
            viewport_rows: 2,
            cell_height: 16.0,
        }),
        16.0
    );
    assert_eq!(
        terminal_effective_visual_scroll_offset_px(TerminalVisualScrollGeometry {
            snapshot_pending: false,
            target_offset: 1,
            displayed_offset: 0,
            residual_lines: 0.25,
            viewport_anchor_row: 0,
            snapshot_rows: 1,
            viewport_rows: 1,
            cell_height: 16.0,
        }),
        20.0
    );
}

#[test]
fn text_first_repaint_waits_for_cached_target_text() {
    let state = TerminalScrollVisualState {
        session_id: "session".to_string(),
        scroll_offset: 4,
        scroll_residual_lines: 0.0,
        display_offset: 4,
        scrollback_len: 20,
        viewport_rows: 8,
        has_new_while_scrolled: false,
        performance_overlay: None,
        skipped_output_chars: 0,
    };

    assert!(!terminal_surface_text_first_repaint_ready(
        &state, false, false
    ));
    assert!(terminal_surface_text_first_repaint_ready(
        &state, false, true
    ));
    assert!(!terminal_surface_text_first_repaint_ready(
        &state, true, true
    ));
}

#[test]
fn keyword_highlight_request_key_tracks_snapshot_and_rules() {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text("alpha\nbeta\n");
    let snapshot = screen.viewport_snapshot(0);
    let rules = vec![nyaterm_core::ResolvedKeywordHighlightRule {
        id: "alpha".to_string(),
        name: "Alpha".to_string(),
        patterns: vec!["alpha".to_string()],
        color: "#ff0000".to_string(),
        enabled: true,
    }];
    let rules_key = terminal_keyword_rules_key(&rules);
    let rows = 0..snapshot.row_count();
    let key = terminal_keyword_highlight_request_key(&snapshot, rules_key, rows.clone());

    assert_eq!(
        key,
        terminal_keyword_highlight_request_key(&snapshot, rules_key, rows.clone())
    );
    assert_ne!(
        key,
        terminal_keyword_highlight_request_key(
            &snapshot,
            rules_key.saturating_add(1),
            rows.clone(),
        )
    );
    assert_ne!(
        key,
        terminal_keyword_highlight_request_key(&snapshot, rules_key, 1..snapshot.row_count())
    );

    let mut scrolled_snapshot = snapshot.clone();
    scrolled_snapshot.display_offset = scrolled_snapshot.display_offset.saturating_add(1);
    assert_ne!(
        key,
        terminal_keyword_highlight_request_key(&scrolled_snapshot, rules_key, rows.clone())
    );

    let mut edited_snapshot = snapshot.clone();
    let row_data = Arc::make_mut(&mut edited_snapshot.row_data);
    let row = Arc::make_mut(&mut row_data[0]);
    row.signature = row.signature.wrapping_add(1);
    assert_ne!(
        key,
        terminal_keyword_highlight_request_key(&edited_snapshot, rules_key, rows)
    );

    let visible_rows = 0..1;
    let visible_key =
        terminal_keyword_highlight_request_key(&snapshot, rules_key, visible_rows.clone());
    let mut edited_outside_visible_rows = snapshot.clone();
    let rows = Arc::make_mut(&mut edited_outside_visible_rows.row_data);
    let row = Arc::make_mut(&mut rows[1]);
    row.signature = row.signature.wrapping_add(1);
    assert_eq!(
        visible_key,
        terminal_keyword_highlight_request_key(
            &edited_outside_visible_rows,
            rules_key,
            visible_rows,
        )
    );
}

#[test]
fn keyword_highlight_request_key_tracks_wrapped_row_context() {
    let mut snapshot = TerminalScreen::new(3, 4).snapshot();
    {
        let rows = Arc::make_mut(&mut snapshot.row_data);
        Arc::make_mut(&mut rows[0]).signature = 41;
        Arc::make_mut(&mut rows[1]).signature = 42;
    }
    let rules_key = 7;
    let plain_key = terminal_keyword_highlight_request_key(&snapshot, rules_key, 1..2);

    let mut wrapped = snapshot.clone();
    {
        let rows = Arc::make_mut(&mut wrapped.row_data);
        Arc::make_mut(&mut rows[1]).wrapped = true;
    }
    let wrapped_key = terminal_keyword_highlight_request_key(&wrapped, rules_key, 1..2);
    let expanded_key = terminal_keyword_highlight_request_key(&wrapped, rules_key, 0..2);

    assert_ne!(plain_key, wrapped_key);
    assert_eq!(wrapped_key, expanded_key);
    assert_eq!(wrapped_key.row_start, 0);
    assert_eq!(wrapped_key.row_end, 2);
}

#[test]
fn keyword_highlight_visible_rows_bound_retained_scroll_work() {
    let mut screen = TerminalScreen::new(80, 24);
    screen.advance_decoded_text(&terminal_test_output_lines(240));
    let snapshot = screen.viewport_snapshot_with_window(80, 64, 64);
    let viewport_rows = snapshot.viewport_rows;
    let rows = terminal_keyword_highlight_visible_rows(
        &snapshot,
        80,
        viewport_rows,
        snapshot.scrollback_len,
    );
    let anchor = terminal_snapshot_anchor_row_for_display_offset(
        &snapshot,
        80,
        viewport_rows,
        snapshot.scrollback_len,
    );

    assert!(rows.contains(&anchor));
    assert!(rows.len() <= viewport_rows.saturating_add(2));
    assert!(snapshot.row_count() > rows.len());
    assert_ne!(
        rows,
        terminal_keyword_highlight_visible_rows(
            &snapshot,
            81,
            viewport_rows,
            snapshot.scrollback_len,
        )
    );
}

#[test]
fn keyword_highlight_prefetch_covers_two_viewports_of_local_scroll() {
    let mut screen = TerminalScreen::new(80, 24);
    screen.advance_decoded_text(&terminal_test_output_lines(240));
    let snapshot = screen.viewport_snapshot_with_window(80, 64, 64);
    let viewport_rows = snapshot.viewport_rows;
    let rows = terminal_keyword_highlight_prefetch_rows(
        &snapshot,
        80,
        viewport_rows,
        snapshot.scrollback_len,
        2,
    );

    for display_offset in 80..=80usize.saturating_add(viewport_rows.saturating_mul(2)) {
        let visible_rows = terminal_keyword_highlight_visible_rows(
            &snapshot,
            display_offset,
            viewport_rows,
            snapshot.scrollback_len,
        );
        assert!(rows.start <= visible_rows.start);
        assert!(rows.end >= visible_rows.end);
    }
    assert!(rows.len() <= viewport_rows.saturating_mul(5).saturating_add(2));
    assert!(snapshot.row_count() > rows.len());
}

#[test]
fn keyword_highlight_prefetch_uses_visible_window_under_output_pressure() {
    let mut screen = TerminalScreen::new(80, 24);
    screen.advance_decoded_text(&terminal_test_output_lines(240));
    let snapshot = screen.viewport_snapshot_with_window(80, 64, 64);
    let viewport_rows = snapshot.viewport_rows;

    let rows = terminal_keyword_highlight_prefetch_rows(
        &snapshot,
        80,
        viewport_rows,
        snapshot.scrollback_len,
        0,
    );
    let visible_rows = terminal_keyword_highlight_visible_rows(
        &snapshot,
        80,
        viewport_rows,
        snapshot.scrollback_len,
    );

    assert_eq!(rows, visible_rows);
    assert!(rows.len() <= viewport_rows.saturating_add(2));
}

#[test]
fn keyword_highlight_prefetch_viewports_follow_output_pressure() {
    assert_eq!(
        terminal_keyword_highlight_prefetch_viewports(false),
        TERMINAL_KEYWORD_HIGHLIGHT_PREFETCH_VIEWPORTS
    );
    assert_eq!(
        terminal_keyword_highlight_prefetch_viewports(true),
        TERMINAL_KEYWORD_HIGHLIGHT_PRESSURE_PREFETCH_VIEWPORTS
    );
}

#[test]
fn keyword_highlight_pressure_delay_uses_throttle_window() {
    assert_eq!(
        terminal_keyword_highlight_pressure_delay(false, Some(Duration::ZERO)),
        None
    );
    assert_eq!(terminal_keyword_highlight_pressure_delay(true, None), None);
    assert_eq!(
        terminal_keyword_highlight_pressure_delay(true, Some(Duration::ZERO)),
        Some(TERMINAL_KEYWORD_HIGHLIGHT_PRESSURE_THROTTLE)
    );
    assert_eq!(
        terminal_keyword_highlight_pressure_delay(
            true,
            Some(TERMINAL_KEYWORD_HIGHLIGHT_PRESSURE_THROTTLE + Duration::from_millis(1)),
        ),
        None
    );
}

#[test]
fn keyword_highlight_deferred_task_coalesces_latest_snapshot_under_pressure() {
    let mut cx = TestAppContext::single();
    let surface = cx.new(|_| TerminalSurface::new("session"));
    let mut first_screen = TerminalScreen::default();
    first_screen.advance_decoded_text("alpha\n");
    let first_snapshot = Arc::new(first_screen.viewport_snapshot(0));
    let mut second_screen = TerminalScreen::default();
    second_screen.advance_decoded_text("beta\n");
    let second_snapshot = Arc::new(second_screen.viewport_snapshot(0));
    let rules = Arc::new(vec![nyaterm_core::ResolvedKeywordHighlightRule {
        id: "beta".to_string(),
        name: "Beta".to_string(),
        patterns: vec!["beta".to_string()],
        color: "#ff0000".to_string(),
        enabled: true,
    }]);

    cx.update_entity(&surface, |surface, cx| {
        surface.keyword_rules = rules.clone();
        apply_test_frame_snapshot!(
            surface,
            first_snapshot.clone(),
            0,
            0.0,
            0,
            first_snapshot.scrollback_len,
            first_snapshot.viewport_rows,
            false,
            None,
            0,
            false,
            false,
            "block",
        );
        surface.keyword_highlight_last_started_at = Some(Instant::now());
        surface.schedule_keyword_highlights(false, true, cx);
        assert!(surface.keyword_highlight_deferred_task.is_some());
        let first_pending_key = surface.keyword_highlight_pending_key;

        apply_test_frame_snapshot!(
            surface,
            second_snapshot.clone(),
            0,
            0.0,
            0,
            second_snapshot.scrollback_len,
            second_snapshot.viewport_rows,
            false,
            None,
            0,
            false,
            false,
            "block",
        );
        surface.schedule_keyword_highlights(false, true, cx);
        assert!(surface.keyword_highlight_deferred_task.is_some());
        assert_ne!(surface.keyword_highlight_pending_key, first_pending_key);
        surface.keyword_highlight_last_started_at = None;
    });

    cx.executor()
        .advance_clock(TERMINAL_KEYWORD_HIGHLIGHT_PRESSURE_THROTTLE);
    cx.run_until_parked();

    cx.read_entity(&surface, |surface, _| {
        assert!(surface.keyword_highlight_deferred_task.is_none());
        assert!(surface.keyword_highlight_task.is_none());
        let highlights = surface
            .keyword_highlights
            .as_ref()
            .expect("deferred task should publish latest highlights");
        assert!(highlights.matches_snapshot_rows(
            second_snapshot.as_ref(),
            surface.palette,
            0..second_snapshot.row_count(),
        ));
        assert_eq!(highlights.range_count(), 1);
    });
}

#[test]
fn cancelling_pending_keyword_highlights_keeps_published_highlights() {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text("alpha\nbeta\n");
    let snapshot = screen.viewport_snapshot(0);
    let rules = vec![nyaterm_core::ResolvedKeywordHighlightRule {
        id: "alpha".to_string(),
        name: "Alpha".to_string(),
        patterns: vec!["alpha".to_string()],
        color: "#ff0000".to_string(),
        enabled: true,
    }];
    let rules = Arc::new(rules);
    let highlighter = Arc::new(compile_terminal_keyword_highlighter(rules.as_ref()));
    let highlights = Arc::new(precompute_terminal_keyword_highlights(
        &snapshot,
        highlighter.as_ref(),
        nyaterm_ui::theme_palette("github-dark"),
        None,
    ));
    let pending_key = terminal_keyword_highlight_request_key(
        &snapshot,
        terminal_keyword_rules_key(&rules),
        0..snapshot.row_count(),
    );
    let mut surface = TerminalSurface::new("session");
    surface.keyword_highlight_generation = 41;
    surface.keyword_highlight_pending_key = Some(pending_key);
    surface.keyword_highlighter_rules = Some(rules);
    surface.keyword_highlighter = Some(highlighter);
    surface.keyword_highlights = Some(highlights.clone());

    surface.cancel_pending_keyword_highlights();

    assert_eq!(surface.keyword_highlight_generation, 42);
    assert!(surface.keyword_highlight_pending_key.is_none());
    assert!(surface.keyword_highlight_task.is_none());
    assert!(
        surface
            .keyword_highlights
            .as_ref()
            .is_some_and(|stored| Arc::ptr_eq(stored, &highlights))
    );
    assert!(surface.keyword_highlighter_rules.is_some());
    assert!(surface.keyword_highlighter.is_some());
}

#[test]
fn transient_empty_keyword_rules_keep_pending_highlights() {
    let mut surface = TerminalSurface::new("session");
    let pending_key = TerminalKeywordHighlightRequestKey {
        rules_key: 7,
        display_offset: 0,
        row_start: 0,
        row_end: 1,
        line_signatures_key: 9,
    };
    surface.keyword_rules = Arc::new(Vec::new());
    surface.keyword_highlight_generation = 41;
    surface.keyword_highlight_pending_key = Some(pending_key);
    surface.keyword_highlighter_rules = Some(Arc::new(Vec::new()));
    surface.keyword_highlighter = Some(Arc::new(compile_terminal_keyword_highlighter(&[])));
    surface.keyword_highlights = Some(Arc::new(precompute_terminal_keyword_highlights(
        &TerminalScreen::default().viewport_snapshot(0),
        surface.keyword_highlighter.as_ref().unwrap().as_ref(),
        nyaterm_ui::theme_palette("github-dark"),
        None,
    )));

    assert!(surface.handle_empty_keyword_rules_for_highlights(false));
    assert_eq!(surface.keyword_highlight_generation, 41);
    assert_eq!(surface.keyword_highlight_pending_key, Some(pending_key));
    assert!(surface.keyword_highlights.is_some());
    assert!(surface.keyword_highlighter.is_some());
}

#[test]
fn configured_empty_keyword_rules_clear_pending_highlights() {
    let mut surface = TerminalSurface::new("session");
    let pending_key = TerminalKeywordHighlightRequestKey {
        rules_key: 7,
        display_offset: 0,
        row_start: 0,
        row_end: 1,
        line_signatures_key: 9,
    };
    surface.keyword_rules = Arc::new(Vec::new());
    surface.keyword_highlight_generation = 41;
    surface.keyword_highlight_pending_key = Some(pending_key);
    surface.keyword_highlighter_rules = Some(Arc::new(Vec::new()));
    surface.keyword_highlighter = Some(Arc::new(compile_terminal_keyword_highlighter(&[])));
    surface.keyword_highlights = Some(Arc::new(precompute_terminal_keyword_highlights(
        &TerminalScreen::default().viewport_snapshot(0),
        surface.keyword_highlighter.as_ref().unwrap().as_ref(),
        nyaterm_ui::theme_palette("github-dark"),
        None,
    )));

    assert!(surface.handle_empty_keyword_rules_for_highlights(true));
    assert_eq!(surface.keyword_highlight_generation, 42);
    assert!(surface.keyword_highlight_pending_key.is_none());
    assert!(surface.keyword_highlights.is_none());
    assert!(surface.keyword_highlighter.is_none());
    assert!(surface.keyword_highlighter_rules.is_none());
}

#[test]
fn cached_keyword_highlighter_reuses_equal_rule_sets() {
    let cached_rules = Arc::new(vec![nyaterm_core::ResolvedKeywordHighlightRule {
        id: "alpha".to_string(),
        name: "Alpha".to_string(),
        patterns: vec!["alpha".to_string()],
        color: "#ff0000".to_string(),
        enabled: true,
    }]);
    let equivalent_rules = Arc::new(cached_rules.as_ref().clone());
    assert!(!Arc::ptr_eq(&cached_rules, &equivalent_rules));
    let highlighter = Arc::new(compile_terminal_keyword_highlighter(cached_rules.as_ref()));
    let mut surface = TerminalSurface::new("session");
    surface.keyword_highlighter_rules = Some(cached_rules);
    surface.keyword_highlighter = Some(highlighter.clone());

    assert!(
        surface
            .cached_keyword_highlighter_for_rules(&equivalent_rules)
            .is_some_and(|cached| Arc::ptr_eq(&cached, &highlighter))
    );
}

#[test]
fn applying_identical_scroll_visual_state_is_a_noop() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");
    apply_test_frame_snapshot!(
        surface, snapshot, 0, 0.0, 0, 10, rows, false, None, 0, false, false, "block",
    );
    let state = TerminalScrollVisualState {
        session_id: "session".to_string(),
        scroll_offset: 0,
        scroll_residual_lines: 0.0,
        display_offset: 0,
        scrollback_len: 10,
        viewport_rows: rows,
        has_new_while_scrolled: false,
        performance_overlay: None,
        skipped_output_chars: 0,
    };
    let revision_before = surface.revision;

    assert!(!surface.apply_scroll_visual_state(state));
    assert_eq!(surface.revision, revision_before);
}

#[test]
fn repeated_pending_scroll_visual_state_does_not_need_repaint() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");
    apply_test_frame_snapshot!(
        surface, snapshot, 0, 0.0, 0, 10, rows, false, None, 0, false, false, "block",
    );
    let state = TerminalScrollVisualState {
        session_id: "session".to_string(),
        scroll_offset: 1,
        scroll_residual_lines: 0.0,
        display_offset: 1,
        scrollback_len: 10,
        viewport_rows: rows,
        has_new_while_scrolled: false,
        performance_overlay: None,
        skipped_output_chars: 0,
    };

    assert!(surface.scroll_visual_state_needs_repaint(&state));
    assert!(!surface.apply_scroll_visual_state(state.clone()));

    assert!(!surface.scroll_visual_state_needs_repaint(&state));
    assert!(surface.scroll_snapshot_pending);
}

#[test]
fn scroll_without_target_snapshot_preserves_stale_paint_state() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");
    surface.keyword_rules = Arc::new(vec![nyaterm_core::ResolvedKeywordHighlightRule {
        id: "test".to_string(),
        name: "test".to_string(),
        patterns: vec!["test".to_string()],
        color: "#ff0000".to_string(),
        enabled: true,
    }]);
    surface.set_decorations_and_keywords(
        vec![TerminalLineDecorations {
            link_ranges: vec![(1, 3)],
            ..TerminalLineDecorations::default()
        }],
        surface.keyword_rules.clone(),
        true,
        "block",
    );

    apply_test_frame_snapshot!(
        surface, snapshot, 0, 0.0, 0, 10, rows, false, None, 0, true, true, "block",
    );
    surface.update_scroll_chrome_without_snapshot(&TerminalScrollVisualState {
        session_id: "session".to_string(),
        scroll_offset: 1,
        scroll_residual_lines: 0.0,
        display_offset: 1,
        scrollback_len: 10,
        viewport_rows: rows,
        has_new_while_scrolled: false,
        performance_overlay: None,
        skipped_output_chars: 0,
    });

    assert_eq!(surface.display_offset, 0);
    assert!(surface.scroll_snapshot_pending);
    assert!(!surface.keyword_rules.is_empty());
    assert_eq!(surface.decorations[0].link_ranges, vec![(1, 3)]);
    assert!(surface.has_action_link_decorations);
    assert_eq!(
        terminal_effective_visual_scroll_offset_px(TerminalVisualScrollGeometry {
            snapshot_pending: surface.scroll_snapshot_pending,
            target_offset: surface.scroll_offset,
            displayed_offset: surface.display_offset,
            residual_lines: surface.scroll_residual_lines,
            viewport_anchor_row: 0,
            snapshot_rows: rows,
            viewport_rows: rows,
            cell_height: 16.0,
        },),
        0.0
    );
}

#[test]
fn degraded_empty_decorations_preserve_stale_paint_state() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface, snapshot, 0, 0.0, 0, 10, rows, false, None, 0, false, true, "block",
    );
    surface.set_decorations_and_keywords(
        vec![TerminalLineDecorations {
            search_ranges: vec![(2, 5)],
            link_ranges: vec![(1, 3)],
            ..TerminalLineDecorations::default()
        }],
        Arc::new(Vec::new()),
        true,
        "block",
    );

    surface.set_decorations_and_keywords_preserving_stale(
        Vec::new(),
        Arc::new(Vec::new()),
        true,
        "beam",
        true,
    );

    assert_eq!(surface.decorations[0].search_ranges, vec![(2, 5)]);
    assert_eq!(surface.decorations[0].link_ranges, vec![(1, 3)]);
    assert_eq!(surface.cursor_style, "beam");

    surface.set_decorations_and_keywords_preserving_stale(
        vec![TerminalLineDecorations {
            active_search_ranges: vec![(6, 8)],
            ..TerminalLineDecorations::default()
        }],
        Arc::new(Vec::new()),
        true,
        "underline",
        true,
    );

    assert!(surface.decorations[0].search_ranges.is_empty());
    assert_eq!(surface.decorations[0].active_search_ranges, vec![(6, 8)]);
    assert_eq!(surface.cursor_style, "underline");
}

#[test]
fn empty_decorations_clear_stale_links_when_preserve_not_allowed() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface, snapshot, 0, 0.0, 0, 10, rows, false, None, 0, true, true, "block",
    );
    surface.set_decorations_and_keywords(
        vec![TerminalLineDecorations {
            link_ranges: vec![(1, 3)],
            ..TerminalLineDecorations::default()
        }],
        Arc::new(Vec::new()),
        true,
        "block",
    );

    assert!(surface.set_decorations_and_keywords_preserving_stale(
        Vec::new(),
        Arc::new(Vec::new()),
        true,
        "block",
        false,
    ));

    assert!(surface.decorations.is_empty());
}

#[test]
fn restored_keyword_rules_report_paint_detail_change_without_new_frame() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");
    let rules = Arc::new(vec![nyaterm_core::ResolvedKeywordHighlightRule {
        id: "alpha".to_string(),
        name: "Alpha".to_string(),
        patterns: vec!["alpha".to_string()],
        color: "#ff0000".to_string(),
        enabled: true,
    }]);
    let decorations = vec![TerminalLineDecorations {
        search_ranges: vec![(1, 5)],
        ..TerminalLineDecorations::default()
    }];

    assert!(apply_test_frame_snapshot!(
        surface,
        snapshot.clone(),
        0,
        0.0,
        0,
        10,
        rows,
        false,
        None,
        0,
        false,
        true,
        "block",
    ));
    assert!(surface.set_decorations_and_keywords(
        decorations.clone(),
        Arc::new(Vec::new()),
        true,
        "block",
    ));

    assert!(!apply_test_frame_snapshot!(
        surface, snapshot, 0, 0.0, 0, 10, rows, false, None, 0, false, true, "block",
    ));
    assert!(surface.set_decorations_and_keywords(decorations, rules.clone(), true, "block",));
    assert_eq!(surface.keyword_rules.as_ref(), rules.as_ref());
    assert!(!surface.set_decorations_and_keywords(
        surface.decorations.to_vec(),
        rules,
        true,
        "block",
    ));
}

#[test]
fn pending_scroll_uses_retained_rows_before_target_snapshot_arrives() {
    assert_eq!(
        terminal_effective_visual_scroll_offset_px(TerminalVisualScrollGeometry {
            snapshot_pending: true,
            target_offset: 9,
            displayed_offset: 0,
            residual_lines: 0.0,
            viewport_anchor_row: 12,
            snapshot_rows: 40,
            viewport_rows: 20,
            cell_height: 16.0,
        }),
        144.0
    );
    assert_eq!(
        terminal_effective_visual_scroll_offset_px(TerminalVisualScrollGeometry {
            snapshot_pending: true,
            target_offset: 40,
            displayed_offset: 0,
            residual_lines: 0.0,
            viewport_anchor_row: 12,
            snapshot_rows: 40,
            viewport_rows: 20,
            cell_height: 16.0,
        }),
        192.0
    );
    assert_eq!(
        terminal_effective_visual_scroll_offset_px(TerminalVisualScrollGeometry {
            snapshot_pending: true,
            target_offset: 0,
            displayed_offset: 6,
            residual_lines: -3.0,
            viewport_anchor_row: 12,
            snapshot_rows: 40,
            viewport_rows: 20,
            cell_height: 16.0,
        }),
        -128.0
    );
}

#[test]
fn pending_scroll_with_live_retained_rows_aligns_target_viewport() {
    let (snapshot, viewport_rows) = terminal_test_retained_live_snapshot(160);
    let target_offset = viewport_rows;
    let anchor = terminal_snapshot_anchor_row_for_display_offset(
        &snapshot,
        0,
        viewport_rows,
        snapshot.scrollback_len,
    );
    assert!(anchor >= viewport_rows);

    let cell_h = 16.0;
    let visual_y_offset =
        terminal_effective_visual_scroll_offset_px(TerminalVisualScrollGeometry {
            snapshot_pending: true,
            target_offset,
            displayed_offset: 0,
            residual_lines: 0.0,
            viewport_anchor_row: anchor,
            snapshot_rows: snapshot.row_count(),
            viewport_rows,
            cell_height: cell_h,
        }) - anchor as f32 * cell_h;
    let target_anchor = terminal_snapshot_anchor_row_for_display_offset(
        &snapshot,
        target_offset,
        viewport_rows,
        snapshot.scrollback_len,
    );

    assert_eq!(visual_y_offset, -(target_anchor as f32) * cell_h);
}

#[test]
fn gutter_visible_rows_follow_visual_scroll_window() {
    assert_eq!(
        terminal_surface_visible_rows_for_viewport(20, 200, 0.0, 16.0),
        0..21
    );
    assert_eq!(
        terminal_surface_visible_rows_for_viewport(20, 200, -160.0, 16.0),
        9..31
    );
    assert_eq!(
        terminal_surface_visible_rows_for_viewport(20, 200, 80.0, 16.0),
        0..16
    );
}

#[test]
fn gutter_visible_rows_clamp_large_retained_windows() {
    let rows = terminal_surface_visible_rows_for_viewport(40, 1200, -8000.0, 16.0);

    assert!(rows.start >= 499);
    assert!(rows.end <= 542);
    assert!(rows.len() <= 43);
}

#[test]
fn matching_scroll_snapshot_clears_pending_visual_freeze() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        snapshot.clone(),
        0,
        0.0,
        0,
        10,
        rows,
        false,
        None,
        0,
        true,
        true,
        "block",
    );
    surface.update_scroll_chrome_without_snapshot(&TerminalScrollVisualState {
        session_id: "session".to_string(),
        scroll_offset: 1,
        scroll_residual_lines: 0.0,
        display_offset: 1,
        scrollback_len: 10,
        viewport_rows: rows,
        has_new_while_scrolled: false,
        performance_overlay: None,
        skipped_output_chars: 0,
    });
    assert!(surface.scroll_snapshot_pending);

    apply_test_frame_snapshot!(
        surface, snapshot, 1, 0.0, 1, 10, rows, false, None, 0, true, true, "block",
    );

    assert!(!surface.scroll_snapshot_pending);
    assert_eq!(surface.display_offset, 1);
    assert_eq!(
        terminal_effective_visual_scroll_offset_px(TerminalVisualScrollGeometry {
            snapshot_pending: surface.scroll_snapshot_pending,
            target_offset: surface.scroll_offset,
            displayed_offset: surface.display_offset,
            residual_lines: surface.scroll_residual_lines,
            viewport_anchor_row: 0,
            snapshot_rows: rows,
            viewport_rows: rows,
            cell_height: 16.0,
        },),
        0.0
    );
}

#[test]
fn repeated_pending_scroll_chrome_update_is_noop() {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text(&terminal_test_output_lines(80));
    let snapshot = Arc::new(screen.viewport_snapshot(0));
    let rows = snapshot.row_count();
    let scrollback_len = snapshot.scrollback_len;
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        snapshot,
        0,
        0.0,
        0,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );

    assert!(
        surface.update_scroll_chrome_without_snapshot(&TerminalScrollVisualState {
            session_id: "session".to_string(),
            scroll_offset: 1,
            scroll_residual_lines: 0.0,
            display_offset: 1,
            scrollback_len,
            viewport_rows: rows,
            has_new_while_scrolled: false,
            performance_overlay: None,
            skipped_output_chars: 0,
        })
    );
    assert!(surface.scroll_snapshot_pending);
    let revision_before_repeat = surface.revision;

    assert!(
        !surface.update_scroll_chrome_without_snapshot(&TerminalScrollVisualState {
            session_id: "session".to_string(),
            scroll_offset: 1,
            scroll_residual_lines: 0.0,
            display_offset: 1,
            scrollback_len,
            viewport_rows: rows,
            has_new_while_scrolled: false,
            performance_overlay: None,
            skipped_output_chars: 0,
        })
    );
    assert_eq!(surface.revision, revision_before_repeat);
    assert!(surface.scroll_snapshot_pending);
}

#[test]
fn identical_frame_snapshot_update_is_noop() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let scrollback_len = snapshot.scrollback_len;
    let mut surface = TerminalSurface::new("session");

    assert!(apply_test_frame_snapshot!(
        surface,
        snapshot.clone(),
        0,
        0.0,
        0,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    ));
    let revision_before_repeat = surface.revision;

    assert!(!apply_test_frame_snapshot!(
        surface,
        snapshot,
        0,
        0.0,
        0,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    ));
    assert_eq!(surface.revision, revision_before_repeat);
}

#[test]
fn local_surface_scroll_state_reuses_covering_snapshot_immediately() {
    let (snapshot, rows) = terminal_test_retained_live_snapshot(80);
    let snapshot = Arc::new(snapshot);
    let scrollback_len = snapshot.scrollback_len;
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        snapshot,
        0,
        0.0,
        0,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );
    let text_updated = surface.apply_scroll_visual_state(TerminalScrollVisualState {
        session_id: "session".to_string(),
        scroll_offset: 1,
        scroll_residual_lines: 0.25,
        display_offset: 1,
        scrollback_len,
        viewport_rows: rows,
        has_new_while_scrolled: true,
        performance_overlay: None,
        skipped_output_chars: 7,
    });

    assert!(text_updated);
    assert!(!surface.scroll_snapshot_pending);
    assert_eq!(surface.display_offset, 1);
    assert_eq!(surface.scroll_residual_lines, 0.25);
    assert!(surface.has_new_while_scrolled);
    assert_eq!(surface.skipped_output_chars, 7);
}

#[test]
fn promoting_current_scroll_window_does_not_retain_it_again() {
    let (snapshot, rows) = terminal_test_retained_live_snapshot(80);
    let snapshot = Arc::new(snapshot);
    let scrollback_len = snapshot.scrollback_len;
    let mut surface = TerminalSurface::new("session");
    surface.snapshot = Some(snapshot.clone());

    assert!(surface.promote_snapshot_covering_display_offset(1, rows, scrollback_len));
    assert!(surface.retained_snapshots.is_empty());
    assert!(surface.retained_rows.is_empty());
    assert!(Arc::ptr_eq(surface.snapshot.as_ref().unwrap(), &snapshot));
}

#[test]
fn local_surface_wheel_updates_visual_state_before_app_sync() {
    let (snapshot, rows) = terminal_test_retained_live_snapshot(80);
    let snapshot = Arc::new(snapshot);
    let scrollback_len = snapshot.scrollback_len;
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        snapshot,
        0,
        0.0,
        0,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );

    assert!(surface.can_handle_scroll_wheel_locally());
    assert_eq!(
        surface.apply_local_scroll_wheel_visual_state(0.35),
        Some(TerminalSurfaceLocalScrollResult {
            generation: 1,
            visual_changed: true,
            needs_text_snapshot: false,
        })
    );
    assert_eq!(surface.scroll_offset, 0);
    assert!((surface.scroll_residual_lines - 0.35).abs() < f32::EPSILON * 8.0);
    assert_eq!(surface.display_offset, 0);

    assert_eq!(
        surface.apply_local_scroll_wheel_visual_state(0.70),
        Some(TerminalSurfaceLocalScrollResult {
            generation: 2,
            visual_changed: true,
            needs_text_snapshot: false,
        })
    );
    assert_eq!(surface.scroll_offset, 1);
    assert!((surface.scroll_residual_lines - 0.05).abs() < f32::EPSILON * 8.0);
    assert_eq!(surface.display_offset, 1);
    assert!(!surface.scroll_snapshot_pending);
}

#[test]
fn local_surface_fractional_wheel_keeps_live_text_window_stable() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface, snapshot, 0, 0.0, 0, 10, rows, false, None, 0, false, false, "block",
    );

    let result = surface
        .apply_local_scroll_wheel_visual_state(0.60)
        .expect("local scroll result");

    assert!(result.visual_changed);
    assert!(!result.needs_text_snapshot);
    assert_eq!(surface.scroll_offset, 0);
    assert!((surface.scroll_residual_lines - 0.60).abs() < f32::EPSILON * 8.0);
    assert_eq!(surface.display_offset, 0);
    assert!(!surface.scroll_snapshot_pending);
}

#[test]
fn local_surface_fractional_scroll_counts_as_visual_scroll_for_cursor() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        snapshot.clone(),
        0,
        0.0,
        0,
        10,
        rows,
        false,
        None,
        0,
        false,
        true,
        "block",
    );
    assert!(surface.show_cursor);

    surface
        .apply_local_scroll_wheel_visual_state(0.60)
        .expect("local scroll result");
    assert!(surface.visual_scroll_active());
    assert!(!surface.show_cursor);

    surface.set_decorations_and_keywords(Vec::new(), Arc::new(Vec::new()), true, "block");
    assert!(!surface.show_cursor);

    apply_test_frame_snapshot!(
        surface, snapshot, 0, 0.60, 0, 10, rows, false, None, 0, false, true, "block",
    );
    assert!(!surface.show_cursor);
}

#[test]
fn selection_visual_update_preserves_existing_decorations() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        snapshot.clone(),
        0,
        0.0,
        0,
        10,
        rows,
        false,
        None,
        0,
        false,
        true,
        "block",
    );
    surface.set_decorations_and_keywords(
        vec![TerminalLineDecorations {
            link_ranges: vec![(1, 3)],
            ..TerminalLineDecorations::default()
        }],
        Arc::new(Vec::new()),
        true,
        "block",
    );

    assert!(
        surface.set_selection_visual(Some(TerminalSelection::from_range(
            TerminalBufferCellPos::new(0, 2),
            TerminalBufferCellPos::new(1, usize::MAX),
        )))
    );
    assert_eq!(surface.decorations[0].link_ranges, vec![(1, 3)]);
    assert_eq!(
        surface.decorations[0].selection_cols,
        Some((2, snapshot.cols))
    );
    assert_eq!(
        surface.decorations[1].selection_cols,
        Some((0, snapshot.cols))
    );

    assert!(surface.set_selection_visual(None));
    assert_eq!(surface.decorations[0].link_ranges, vec![(1, 3)]);
    assert_eq!(surface.decorations[0].selection_cols, None);
    assert_eq!(surface.decorations[1].selection_cols, None);
}

#[test]
fn empty_selection_press_and_release_preserve_action_link_decorations() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface, snapshot, 0, 0.0, 0, 10, rows, false, None, 0, true, true, "block",
    );
    let decorations = vec![TerminalLineDecorations {
        selected_occurrence_ranges: vec![(0, 1)],
        search_ranges: vec![(1, 2)],
        active_search_ranges: vec![(2, 3)],
        link_ranges: vec![(3, 7)],
        command_mark: Some(nyaterm_terminal::ShellCommandMark::Prompt),
        ..TerminalLineDecorations::default()
    }];
    surface.set_decorations_and_keywords(decorations.clone(), Arc::new(Vec::new()), true, "block");
    let revision = surface.revision;
    let anchor = TerminalSelection::with_anchor(TerminalBufferCellPos::new(0, 4));

    // MouseDown records an empty anchor without changing anything that is painted.
    assert!(!surface.set_selection_visual(Some(anchor)));
    assert_eq!(surface.selection_visual, Some(anchor));
    assert_eq!(surface.selection_visual_row_range, None);
    assert_eq!(surface.decorations.as_ref(), decorations.as_slice());
    assert_eq!(surface.revision, revision);

    // MouseUp clears the empty selection and must leave the same underlines in place.
    assert!(!surface.set_selection_visual(None));
    assert_eq!(surface.selection_visual, None);
    assert_eq!(surface.selection_visual_row_range, None);
    assert_eq!(surface.decorations.as_ref(), decorations.as_slice());
    assert_eq!(surface.revision, revision);
}

#[test]
fn selection_visual_row_range_tracks_absolute_lines_and_union() {
    let snapshot = TerminalScreen::default().viewport_snapshot(0);
    assert_eq!(terminal_selection_visual_row_range(None, &snapshot), None);
    assert_eq!(
        terminal_selection_visual_row_range(
            Some(TerminalSelection::with_anchor(TerminalBufferCellPos::new(
                2, 4,
            ))),
            &snapshot
        ),
        None
    );
    assert_eq!(
        terminal_selection_visual_row_range(Some(TerminalSelection::all_buffer(80)), &snapshot),
        Some(0..snapshot.row_count())
    );
    assert_eq!(
        terminal_selection_visual_row_range(
            Some(TerminalSelection::from_range(
                TerminalBufferCellPos::new(5, 1),
                TerminalBufferCellPos::new(7, 3),
            )),
            &snapshot,
        ),
        Some(5..8)
    );
    assert_eq!(
        terminal_selection_visual_row_range(
            Some(TerminalSelection::from_range(
                TerminalBufferCellPos::new(8, 1),
                TerminalBufferCellPos::new(13, 3),
            )),
            &snapshot,
        ),
        Some(8..14.min(snapshot.row_count()))
    );
    assert_eq!(
        terminal_selection_visual_row_union(Some(2..4), Some(3..6)),
        Some(2..6)
    );
    assert_eq!(
        terminal_selection_visual_row_union(Some(2..4), None),
        Some(2..4)
    );
    assert_eq!(
        terminal_selection_visual_row_union(None, Some(3..6)),
        Some(3..6)
    );
    assert_eq!(terminal_selection_visual_row_union(None, None), None);
}

#[test]
fn selection_visual_update_replaces_only_selection_cols() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        snapshot.clone(),
        0,
        0.0,
        0,
        10,
        rows,
        false,
        None,
        0,
        false,
        true,
        "block",
    );

    let mut decorations = vec![TerminalLineDecorations::default(); snapshot.row_count()];
    decorations[0] = TerminalLineDecorations {
        selected_occurrence_ranges: vec![(0, 1)],
        search_ranges: vec![(1, 2)],
        active_search_ranges: vec![(2, 3)],
        link_ranges: vec![(3, 7)],
        command_mark: Some(nyaterm_terminal::ShellCommandMark::Prompt),
        ..TerminalLineDecorations::default()
    };
    decorations[2].selection_cols = Some((3, snapshot.cols));
    decorations[3].selection_cols = Some((0, 6));
    decorations[8].search_ranges = vec![(2, 5)];
    surface.set_decorations_and_keywords(decorations, Arc::new(Vec::new()), true, "block");

    assert_eq!(surface.selection_visual_row_range, Some(2..4));
    let revision_before = surface.revision;
    assert!(
        surface.set_selection_visual(Some(TerminalSelection::from_range(
            TerminalBufferCellPos::new(3, 1),
            TerminalBufferCellPos::new(4, 4),
        )))
    );

    assert_eq!(
        surface.decorations[0].selected_occurrence_ranges,
        vec![(0, 1)]
    );
    assert_eq!(surface.decorations[0].search_ranges, vec![(1, 2)]);
    assert_eq!(surface.decorations[0].active_search_ranges, vec![(2, 3)]);
    assert_eq!(surface.decorations[0].link_ranges, vec![(3, 7)]);
    assert_eq!(
        surface.decorations[0].command_mark,
        Some(nyaterm_terminal::ShellCommandMark::Prompt)
    );
    assert_eq!(surface.decorations[2].selection_cols, None);
    assert_eq!(
        surface.decorations[3].selection_cols,
        Some((1, snapshot.cols))
    );
    assert_eq!(surface.decorations[4].selection_cols, Some((0, 5)));
    assert_eq!(surface.decorations[8].search_ranges, vec![(2, 5)]);
    assert_eq!(surface.selection_visual_row_range, Some(3..5));
    assert_eq!(surface.revision, revision_before.saturating_add(1));

    let revision_before_same_selection = surface.revision;
    assert!(
        !surface.set_selection_visual(Some(TerminalSelection::from_range(
            TerminalBufferCellPos::new(3, 1),
            TerminalBufferCellPos::new(4, 4),
        )))
    );
    assert_eq!(surface.revision, revision_before_same_selection);

    assert!(surface.set_selection_visual(None));
    assert_eq!(
        surface.decorations[0].selected_occurrence_ranges,
        vec![(0, 1)]
    );
    assert_eq!(surface.decorations[0].search_ranges, vec![(1, 2)]);
    assert_eq!(surface.decorations[0].active_search_ranges, vec![(2, 3)]);
    assert_eq!(surface.decorations[0].link_ranges, vec![(3, 7)]);
    assert_eq!(
        surface.decorations[0].command_mark,
        Some(nyaterm_terminal::ShellCommandMark::Prompt)
    );
    assert_eq!(surface.decorations[3].selection_cols, None);
    assert_eq!(surface.decorations[4].selection_cols, None);
    assert_eq!(surface.decorations[8].search_ranges, vec![(2, 5)]);
    assert_eq!(surface.selection_visual_row_range, None);
}

#[test]
fn local_surface_fractional_scroll_prefetches_adjacent_snapshot() {
    assert_eq!(
        terminal_surface_fractional_prefetch_offset(0, 0.35, 10),
        Some(1)
    );
    assert_eq!(
        terminal_surface_fractional_prefetch_offset(4, 0.35, 10),
        Some(5)
    );
    assert_eq!(
        terminal_surface_fractional_prefetch_offset(4, -0.35, 10),
        Some(3)
    );
    assert_eq!(
        terminal_surface_fractional_prefetch_offset(0, -0.35, 10),
        None
    );
    assert_eq!(
        terminal_surface_fractional_prefetch_offset(10, 0.35, 10),
        None
    );
    assert_eq!(
        terminal_surface_fractional_prefetch_offset(1, f32::NAN, 10),
        None
    );
}

#[test]
fn local_surface_snapshot_requests_are_deduped_until_covered() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface, snapshot, 0, 0.0, 0, 10, rows, false, None, 0, false, false, "block",
    );

    assert_eq!(
        surface.scroll_snapshot_request_offsets_to_enqueue(vec![1, 1, 2]),
        vec![1, 2]
    );
    assert_eq!(
        surface.scroll_snapshot_request_offsets_to_enqueue(vec![1, 2]),
        Vec::<usize>::new()
    );

    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text(&terminal_test_output_lines(80));
    let covering = Arc::new(screen.viewport_snapshot(1));
    let scrollback_len = covering.scrollback_len;
    apply_test_frame_snapshot!(
        surface,
        covering,
        1,
        0.0,
        1,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );

    assert!(!surface.pending_scroll_snapshot_offsets.contains(&1));
    assert_eq!(
        surface.scroll_snapshot_request_offsets_to_enqueue(vec![1, 3]),
        vec![3]
    );
}

#[test]
fn local_surface_snapshot_request_dedupe_resets_when_scrollback_changes() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface, snapshot, 0, 0.0, 0, 10, rows, false, None, 0, false, false, "block",
    );
    assert_eq!(
        surface.scroll_snapshot_request_offsets_to_enqueue(vec![4]),
        vec![4]
    );
    assert_eq!(
        surface.scroll_snapshot_request_offsets_to_enqueue(vec![4]),
        Vec::<usize>::new()
    );

    surface.update_scroll_chrome_without_snapshot(&TerminalScrollVisualState {
        session_id: "session".to_string(),
        scroll_offset: 5,
        scroll_residual_lines: 0.0,
        display_offset: 5,
        scrollback_len: 11,
        viewport_rows: rows,
        has_new_while_scrolled: true,
        performance_overlay: None,
        skipped_output_chars: 0,
    });

    assert_eq!(
        surface.scroll_snapshot_request_offsets_to_enqueue(vec![4, 5]),
        vec![4, 5]
    );
}

#[test]
fn local_surface_wheel_consumes_edge_noop_without_visual_sync() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");

    let frame_applied = apply_test_frame_snapshot!(
        surface, snapshot, 0, 0.0, 0, 0, rows, false, None, 0, false, false, "block",
    );
    assert!(frame_applied);

    assert_eq!(
        surface.apply_local_scroll_wheel_visual_state(-0.5),
        Some(TerminalSurfaceLocalScrollResult {
            generation: 0,
            visual_changed: false,
            needs_text_snapshot: false,
        })
    );
    assert_eq!(surface.scroll_offset, 0);
    assert_eq!(surface.scroll_residual_lines, 0.0);
    assert_eq!(surface.scroll_interaction_generation, 0);
    assert!(!surface.scroll_snapshot_pending);
}

#[test]
fn local_surface_wheel_flags_missing_text_snapshot_immediately() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface, snapshot, 0, 0.0, 0, 10, rows, false, None, 0, false, false, "block",
    );

    let result = surface
        .apply_local_scroll_wheel_visual_state(4.0)
        .expect("local scroll result");

    assert!(result.visual_changed);
    assert!(result.needs_text_snapshot);
    assert_eq!(surface.scroll_offset, 4);
    assert_eq!(surface.display_offset, 0);
    assert!(surface.scroll_snapshot_pending);
}

#[test]
fn pending_local_surface_scroll_survives_stale_full_frame_paint() {
    let (snapshot, rows) = terminal_test_retained_live_snapshot(80);
    let snapshot = Arc::new(snapshot);
    let scrollback_len = snapshot.scrollback_len;
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        snapshot.clone(),
        0,
        0.0,
        0,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );
    let result = surface
        .apply_local_scroll_wheel_visual_state(1.0)
        .expect("local scroll result");
    assert!(result.visual_changed);
    assert_eq!(surface.scroll_offset, 1);
    assert_eq!(surface.display_offset, 1);
    assert!(surface.remember_pending_local_scroll_sync(
        surface.current_scroll_visual_state(),
        result.generation,
    ));

    let frame_applied = apply_test_frame_snapshot!(
        surface,
        snapshot,
        0,
        0.0,
        0,
        scrollback_len,
        rows,
        true,
        None,
        0,
        false,
        true,
        "block",
    );

    assert!(!frame_applied);
    assert_eq!(surface.scroll_offset, 1);
    assert_eq!(surface.display_offset, 1);
    assert!(surface.has_new_while_scrolled);
    assert!(!surface.show_cursor);
    assert!(!surface.scroll_snapshot_pending);
}

#[test]
fn local_surface_wheel_respects_protocol_scroll_modes() {
    let mut surface = TerminalSurface::new("session");
    assert!(surface.can_handle_scroll_wheel_locally());

    let mouse_reporting = TerminalProtocolState {
        mouse_reporting: true,
        ..TerminalProtocolState::default()
    };
    surface.set_protocol_state(mouse_reporting);
    assert!(!surface.can_handle_scroll_wheel_locally());

    let alternate_scroll = TerminalProtocolState {
        alternate_screen: true,
        alternate_scroll: true,
        ..TerminalProtocolState::default()
    };
    surface.set_protocol_state(alternate_scroll);
    assert!(!surface.can_handle_scroll_wheel_locally());
}

#[test]
fn local_surface_scroll_app_sync_is_frame_coalesced() {
    let mut surface = TerminalSurface::new("session");
    let first = TerminalScrollVisualState {
        session_id: "session".to_string(),
        scroll_offset: 1,
        scroll_residual_lines: 0.25,
        display_offset: 1,
        scrollback_len: 10,
        viewport_rows: 4,
        has_new_while_scrolled: false,
        performance_overlay: None,
        skipped_output_chars: 0,
    };
    let mut second = first.clone();
    second.scroll_offset = 2;
    second.display_offset = 2;

    assert!(surface.remember_pending_local_scroll_sync(first, 1));
    assert!(!surface.remember_pending_local_scroll_sync(second, 2));

    let pending = surface
        .pending_local_scroll_sync
        .as_ref()
        .expect("pending sync");
    assert!(surface.local_scroll_sync_armed);
    assert_eq!(pending.generation, 2);
    assert_eq!(pending.state.scroll_offset, 2);
    assert_eq!(pending.state.display_offset, 2);
}

#[test]
fn local_surface_scroll_state_marks_pending_when_snapshot_missing() {
    let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
    let rows = snapshot.row_count();
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface, snapshot, 0, 0.0, 0, 10, rows, false, None, 0, false, false, "block",
    );
    let text_updated = surface.apply_scroll_visual_state(TerminalScrollVisualState {
        session_id: "session".to_string(),
        scroll_offset: 4,
        scroll_residual_lines: 0.0,
        display_offset: 4,
        scrollback_len: 10,
        viewport_rows: rows,
        has_new_while_scrolled: false,
        performance_overlay: None,
        skipped_output_chars: 0,
    });

    assert!(!text_updated);
    assert!(surface.scroll_snapshot_pending);
    assert_eq!(surface.display_offset, 0);
    assert_eq!(surface.scroll_offset, 4);
}

#[test]
fn local_surface_scroll_state_promotes_retained_snapshot_window() {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text(&terminal_test_output_lines(120));
    let first_offset = 8;
    let second_offset = 30;
    let first_snapshot = Arc::new(screen.viewport_snapshot(first_offset));
    let second_snapshot = Arc::new(screen.viewport_snapshot(second_offset));
    let rows = first_snapshot.row_count().max(1);
    let scrollback_len = first_snapshot.scrollback_len;
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        first_snapshot.clone(),
        first_offset,
        0.0,
        first_offset,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );
    apply_test_frame_snapshot!(
        surface,
        second_snapshot,
        second_offset,
        0.0,
        second_offset,
        scrollback_len,
        rows,
        false,
        None,
        0,
        true,
        false,
        "block",
    );
    assert_eq!(
        surface.snapshot.as_ref().unwrap().display_offset,
        second_offset
    );
    assert!(surface.has_action_link_decorations);

    let text_updated = surface.apply_scroll_visual_state(TerminalScrollVisualState {
        session_id: "session".to_string(),
        scroll_offset: first_offset,
        scroll_residual_lines: 0.0,
        display_offset: first_offset,
        scrollback_len,
        viewport_rows: rows,
        has_new_while_scrolled: false,
        performance_overlay: None,
        skipped_output_chars: 0,
    });

    assert!(text_updated);
    assert!(!surface.scroll_snapshot_pending);
    assert_eq!(surface.display_offset, first_offset);
    assert!(surface.has_action_link_decorations);
    assert_eq!(
        surface.snapshot.as_ref().unwrap().display_offset,
        first_offset
    );
    assert!(Arc::ptr_eq(
        surface.snapshot.as_ref().unwrap(),
        &first_snapshot
    ));
}

#[test]
fn retained_snapshot_promotion_remaps_absolute_selection_rows() {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text(&terminal_test_output_lines(120));
    let live_snapshot = Arc::new(screen.viewport_snapshot(0));
    let history_offset = 8;
    let history_snapshot = Arc::new(screen.viewport_snapshot(history_offset));
    let rows = live_snapshot.row_count().max(1);
    let scrollback_len = live_snapshot.scrollback_len;
    let live_start = live_snapshot
        .total_rows
        .saturating_sub(live_snapshot.display_offset)
        .saturating_sub(live_snapshot.row_count());
    let selected_line = live_start.saturating_add(2);
    let history_start = history_snapshot
        .total_rows
        .saturating_sub(history_snapshot.display_offset)
        .saturating_sub(history_snapshot.row_count());
    let live_row = selected_line.saturating_sub(live_start);
    let history_row = selected_line.saturating_sub(history_start);
    assert_ne!(live_row, history_row);
    assert!(history_row < history_snapshot.row_count());

    let mut surface = TerminalSurface::new("session");
    apply_test_frame_snapshot!(
        surface,
        history_snapshot.clone(),
        history_offset,
        0.0,
        history_offset,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );
    apply_test_frame_snapshot!(
        surface,
        live_snapshot,
        0,
        0.0,
        0,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );
    let selection = TerminalSelection::from_range(
        TerminalBufferCellPos::new(selected_line, 2),
        TerminalBufferCellPos::new(selected_line, 5),
    );
    assert!(surface.set_selection_visual(Some(selection)));
    assert_eq!(surface.decorations[live_row].selection_cols, Some((2, 6)));

    assert!(
        surface.apply_scroll_visual_state(TerminalScrollVisualState {
            session_id: "session".to_string(),
            scroll_offset: history_offset,
            scroll_residual_lines: 0.0,
            display_offset: history_offset,
            scrollback_len,
            viewport_rows: rows,
            has_new_while_scrolled: false,
            performance_overlay: None,
            skipped_output_chars: 0,
        })
    );

    assert!(Arc::ptr_eq(
        surface.snapshot.as_ref().unwrap(),
        &history_snapshot
    ));
    assert_eq!(surface.decorations[live_row].selection_cols, None);
    assert_eq!(
        surface.decorations[history_row].selection_cols,
        Some((2, 6))
    );
    assert_eq!(surface.selection_visual, Some(selection));
}

#[test]
fn prefetched_snapshot_is_retained_without_changing_current_paint() {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text(&terminal_test_output_lines(160));
    let live_snapshot = Arc::new(screen.viewport_snapshot(0));
    let viewport_rows = live_snapshot.viewport_rows.max(1);
    let scrollback_len = live_snapshot.scrollback_len;
    let prefetch_offset = viewport_rows.saturating_mul(2).min(scrollback_len);
    let prefetched_snapshot = Arc::new(screen.viewport_snapshot_with_window(
        prefetch_offset,
        viewport_rows.saturating_mul(2),
        viewport_rows.saturating_mul(2),
    ));
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        live_snapshot.clone(),
        0,
        0.0,
        0,
        scrollback_len,
        viewport_rows,
        false,
        None,
        0,
        false,
        true,
        "block",
    );
    let revision = surface.revision;

    assert!(surface.retain_prefetched_snapshot(prefetched_snapshot.clone()));
    assert_eq!(surface.revision, revision);
    assert_eq!(surface.display_offset, 0);
    assert!(Arc::ptr_eq(
        surface.snapshot.as_ref().unwrap(),
        &live_snapshot
    ));
    assert!(surface.has_snapshot_covering_display_offset(
        prefetch_offset,
        viewport_rows,
        scrollback_len
    ));

    let mut refreshed_prefetch = prefetched_snapshot.as_ref().clone();
    let rows = Arc::make_mut(&mut refreshed_prefetch.row_data);
    Arc::make_mut(&mut rows[0]).text = "refreshed".to_string();
    assert!(surface.retain_prefetched_snapshot(Arc::new(refreshed_prefetch)));
    assert_eq!(
        surface
            .retained_snapshots
            .last()
            .and_then(|snapshot| snapshot.line(0)),
        Some("refreshed")
    );
}

#[test]
fn local_surface_scroll_state_synthesizes_cross_window_on_hot_path() {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text(&terminal_test_output_lines(160));
    let live_snapshot = Arc::new(screen.viewport_snapshot(0));
    let rows = live_snapshot.row_count().max(2);
    let older_offset = rows;
    let target_offset = rows / 2;
    let older_snapshot = Arc::new(screen.viewport_snapshot(older_offset));
    let scrollback_len = live_snapshot.scrollback_len;
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        live_snapshot,
        0,
        0.0,
        0,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );
    apply_test_frame_snapshot!(
        surface,
        older_snapshot,
        older_offset,
        0.0,
        older_offset,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );

    assert!(surface.snapshot.as_ref().is_some_and(|snapshot| {
        !terminal_snapshot_covers_display_offset(snapshot, target_offset, rows, scrollback_len)
    }));

    let text_updated = surface.apply_scroll_visual_state(TerminalScrollVisualState {
        session_id: "session".to_string(),
        scroll_offset: target_offset,
        scroll_residual_lines: 0.0,
        display_offset: target_offset,
        scrollback_len,
        viewport_rows: rows,
        has_new_while_scrolled: false,
        performance_overlay: None,
        skipped_output_chars: 0,
    });

    let snapshot = surface.snapshot.as_ref().expect("retained snapshot");
    assert!(text_updated);
    assert!(!surface.scroll_snapshot_pending);
    assert_eq!(surface.scroll_offset, target_offset);
    assert_eq!(surface.display_offset, target_offset);
    assert!(terminal_snapshot_covers_display_offset(
        snapshot,
        target_offset,
        rows,
        scrollback_len
    ));
}

#[test]
fn local_surface_scroll_state_synthesizes_row_cache_on_hot_path() {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text(&terminal_test_output_lines(160));
    let live_snapshot = Arc::new(screen.viewport_snapshot(0));
    let rows = live_snapshot.row_count().max(2);
    let older_offset = rows;
    let target_offset = rows / 2;
    let older_snapshot = Arc::new(screen.viewport_snapshot(older_offset));
    let scrollback_len = live_snapshot.scrollback_len;
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        live_snapshot,
        0,
        0.0,
        0,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );
    apply_test_frame_snapshot!(
        surface,
        older_snapshot,
        older_offset,
        0.0,
        older_offset,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );
    surface.retained_snapshots.clear();
    assert!(!surface.retained_rows.is_empty());

    let text_updated = surface.apply_scroll_visual_state(TerminalScrollVisualState {
        session_id: "session".to_string(),
        scroll_offset: target_offset,
        scroll_residual_lines: 0.0,
        display_offset: target_offset,
        scrollback_len,
        viewport_rows: rows,
        has_new_while_scrolled: false,
        performance_overlay: None,
        skipped_output_chars: 0,
    });

    let snapshot = surface.snapshot.as_ref().expect("retained snapshot");
    assert!(text_updated);
    assert!(!surface.scroll_snapshot_pending);
    assert_eq!(surface.scroll_offset, target_offset);
    assert_eq!(surface.display_offset, target_offset);
    assert!(terminal_snapshot_covers_display_offset(
        snapshot,
        target_offset,
        rows,
        scrollback_len
    ));
}

#[test]
fn retained_rows_can_synthesize_snapshot_when_no_retained_window_covers_target() {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text(&terminal_test_output_lines(160));
    let live_snapshot = Arc::new(screen.viewport_snapshot(0));
    let rows = live_snapshot.row_count().max(2);
    let older_offset = rows;
    let target_offset = rows / 2;
    let older_snapshot = Arc::new(screen.viewport_snapshot(older_offset));
    let scrollback_len = live_snapshot.scrollback_len;
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        live_snapshot,
        0,
        0.0,
        0,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );
    apply_test_frame_snapshot!(
        surface,
        older_snapshot,
        older_offset,
        0.0,
        older_offset,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );
    surface.retained_snapshots.clear();

    assert!(
        surface
            .retained_snapshot_covering_display_offset(target_offset, rows, scrollback_len)
            .is_none()
    );
    assert!(surface.has_snapshot_covering_display_offset(target_offset, rows, scrollback_len));
    let synthesized = surface
        .snapshot_covering_display_offset(target_offset, rows, scrollback_len)
        .expect("retained rows should synthesize the target viewport");
    assert!(synthesized.row_count() > rows);
    assert!(synthesized.display_offset <= target_offset);
    assert!(terminal_snapshot_covers_display_offset(
        synthesized.as_ref(),
        target_offset,
        rows,
        scrollback_len
    ));
    assert!(terminal_snapshot_covers_display_offset(
        synthesized.as_ref(),
        target_offset.saturating_sub(1),
        rows,
        scrollback_len
    ));
    assert!(terminal_snapshot_covers_display_offset(
        synthesized.as_ref(),
        target_offset.saturating_add(1).min(scrollback_len),
        rows,
        scrollback_len
    ));
}

#[test]
fn local_surface_scroll_state_does_not_synthesize_across_gap() {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text(&terminal_test_output_lines(200));
    let live_snapshot = Arc::new(screen.viewport_snapshot(0));
    let rows = live_snapshot.row_count().max(2);
    let far_offset = rows.saturating_mul(2);
    let target_offset = rows;
    let far_snapshot = Arc::new(screen.viewport_snapshot(far_offset));
    let scrollback_len = live_snapshot.scrollback_len;
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        live_snapshot,
        0,
        0.0,
        0,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );
    apply_test_frame_snapshot!(
        surface,
        far_snapshot,
        far_offset,
        0.0,
        far_offset,
        scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );

    let text_updated = surface.apply_scroll_visual_state(TerminalScrollVisualState {
        session_id: "session".to_string(),
        scroll_offset: target_offset,
        scroll_residual_lines: 0.0,
        display_offset: target_offset,
        scrollback_len,
        viewport_rows: rows,
        has_new_while_scrolled: false,
        performance_overlay: None,
        skipped_output_chars: 0,
    });

    assert!(!surface.has_snapshot_covering_display_offset(target_offset, rows, scrollback_len));
    assert!(!text_updated);
    assert!(surface.scroll_snapshot_pending);
    assert_eq!(surface.display_offset, far_offset);
}

#[test]
fn retained_snapshot_stays_valid_when_output_growth_reanchors_offset() {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text(&terminal_test_output_lines(80));
    let old_display_offset = 6;
    let snapshot = Arc::new(screen.viewport_snapshot(old_display_offset));
    let rows = snapshot.row_count();
    let old_scrollback_len = snapshot.scrollback_len;
    let growth = 3;
    let new_display_offset = old_display_offset + growth;
    let new_scrollback_len = old_scrollback_len + growth;

    assert!(terminal_snapshot_covers_display_offset(
        snapshot.as_ref(),
        old_display_offset,
        rows,
        old_scrollback_len
    ));
    assert!(terminal_snapshot_covers_display_offset(
        snapshot.as_ref(),
        new_display_offset,
        rows,
        new_scrollback_len
    ));
    assert_eq!(
        terminal_snapshot_anchor_row_for_display_offset(
            snapshot.as_ref(),
            old_display_offset,
            rows,
            old_scrollback_len
        ),
        terminal_snapshot_anchor_row_for_display_offset(
            snapshot.as_ref(),
            new_display_offset,
            rows,
            new_scrollback_len
        )
    );
}

#[test]
fn output_growth_scroll_position_reuse_does_not_mark_snapshot_pending() {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text(&terminal_test_output_lines(80));
    let old_display_offset = 6;
    let snapshot = Arc::new(screen.viewport_snapshot(old_display_offset));
    let rows = snapshot.row_count();
    let old_scrollback_len = snapshot.scrollback_len;
    let growth = 3;
    let new_display_offset = old_display_offset + growth;
    let new_scrollback_len = old_scrollback_len + growth;
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        snapshot,
        old_display_offset,
        0.0,
        old_display_offset,
        old_scrollback_len,
        rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );
    assert!(
        surface
            .snapshot_covering_display_offset(new_display_offset, rows, new_scrollback_len)
            .is_some()
    );

    surface.update_scroll_position_without_snapshot(&TerminalScrollVisualState {
        session_id: "session".to_string(),
        scroll_offset: new_display_offset,
        scroll_residual_lines: 0.0,
        display_offset: new_display_offset,
        scrollback_len: new_scrollback_len,
        viewport_rows: rows,
        has_new_while_scrolled: true,
        performance_overlay: None,
        skipped_output_chars: 0,
    });

    assert!(!surface.scroll_snapshot_pending);
    assert_eq!(surface.display_offset, new_display_offset);
    assert!(surface.has_new_while_scrolled);
    assert_eq!(
        terminal_effective_visual_scroll_offset_px(TerminalVisualScrollGeometry {
            snapshot_pending: surface.scroll_snapshot_pending,
            target_offset: surface.scroll_offset,
            displayed_offset: surface.display_offset,
            residual_lines: surface.scroll_residual_lines,
            viewport_anchor_row: 0,
            snapshot_rows: rows,
            viewport_rows: rows,
            cell_height: 16.0,
        },),
        0.0
    );
}

#[test]
fn scroll_position_reuse_preserves_stale_decorations_until_recomputed() {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text(&terminal_test_output_lines(80));
    let old_display_offset = 6;
    let snapshot = Arc::new(screen.viewport_snapshot(old_display_offset));
    let rows = snapshot.row_count();
    let old_scrollback_len = snapshot.scrollback_len;
    let growth = 3;
    let new_display_offset = old_display_offset + growth;
    let new_scrollback_len = old_scrollback_len + growth;
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        snapshot,
        old_display_offset,
        0.0,
        old_display_offset,
        old_scrollback_len,
        rows,
        false,
        None,
        0,
        true,
        true,
        "block",
    );
    surface.set_decorations_and_keywords(
        vec![TerminalLineDecorations {
            search_ranges: vec![(2, 5)],
            link_ranges: vec![(1, 3)],
            ..TerminalLineDecorations::default()
        }],
        Arc::new(Vec::new()),
        true,
        "block",
    );

    surface.update_scroll_position_without_snapshot(&TerminalScrollVisualState {
        session_id: "session".to_string(),
        scroll_offset: new_display_offset,
        scroll_residual_lines: 0.0,
        display_offset: new_display_offset,
        scrollback_len: new_scrollback_len,
        viewport_rows: rows,
        has_new_while_scrolled: true,
        performance_overlay: None,
        skipped_output_chars: 0,
    });

    assert_eq!(surface.display_offset, new_display_offset);
    assert_eq!(surface.decorations[0].search_ranges, vec![(2, 5)]);
    assert_eq!(surface.decorations[0].link_ranges, vec![(1, 3)]);
    assert!(surface.has_action_link_decorations);
}

#[test]
fn surface_retained_scroll_state_resets_when_scrollback_shrinks() {
    let mut old_screen = TerminalScreen::default();
    old_screen.advance_decoded_text(&terminal_test_output_lines(120));
    let old_offset = 12;
    let old_snapshot = Arc::new(old_screen.viewport_snapshot(old_offset));
    let old_rows = old_snapshot.row_count();
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        old_snapshot,
        old_offset,
        0.0,
        old_offset,
        old_screen.scrollback_len(),
        old_rows,
        false,
        None,
        0,
        true,
        false,
        "block",
    );
    assert_eq!(surface.retained_snapshots.len(), 1);
    assert!(surface.has_action_link_decorations);

    let mut new_screen = TerminalScreen::default();
    new_screen.advance_decoded_text("after clear\n");
    let new_snapshot = Arc::new(new_screen.viewport_snapshot(0));
    let new_rows = new_snapshot.row_count();
    apply_test_frame_snapshot!(
        surface,
        new_snapshot.clone(),
        0,
        0.0,
        0,
        new_screen.scrollback_len(),
        new_rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );

    assert_eq!(surface.retained_snapshots.len(), 1);
    assert_eq!(surface.retained_snapshots[0].display_offset, 0);
    assert_eq!(
        surface.retained_snapshots[0].total_rows,
        new_snapshot.total_rows
    );
    assert!(!surface.has_action_link_decorations);
}

#[test]
fn surface_retained_scroll_state_resets_when_viewport_rows_change() {
    let mut screen = TerminalScreen::default();
    screen.advance_decoded_text(&terminal_test_output_lines(120));
    let old_offset = 12;
    let old_snapshot = Arc::new(screen.viewport_snapshot(old_offset));
    let old_rows = old_snapshot.row_count().max(2);
    let new_snapshot = Arc::new(screen.viewport_snapshot(0));
    let new_viewport_rows = old_rows - 1;
    let mut surface = TerminalSurface::new("session");

    apply_test_frame_snapshot!(
        surface,
        old_snapshot,
        old_offset,
        0.0,
        old_offset,
        screen.scrollback_len(),
        old_rows,
        false,
        None,
        0,
        true,
        false,
        "block",
    );
    assert_eq!(surface.retained_snapshots.len(), 1);
    assert!(!surface.retained_rows.is_empty());

    apply_test_frame_snapshot!(
        surface,
        new_snapshot,
        0,
        0.0,
        0,
        screen.scrollback_len(),
        new_viewport_rows,
        false,
        None,
        0,
        false,
        false,
        "block",
    );

    assert_eq!(surface.retained_snapshots.len(), 1);
    assert_eq!(surface.retained_snapshots[0].display_offset, 0);
    assert_eq!(surface.viewport_rows, new_viewport_rows);
    assert!(!surface.has_action_link_decorations);
}
