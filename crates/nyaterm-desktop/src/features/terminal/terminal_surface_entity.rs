use gpui::{
    Context, Entity, FontFallbacks, IntoElement, MouseButton, Render, ScrollDelta,
    ScrollWheelEvent, SharedString, Window, div, font, prelude::*, px, rgb, rgba,
};
use nyaterm_terminal::{TerminalScreen, TerminalSnapshot};

use crate::features::NyaTermApp;
use crate::features::formatting::format_terminal_line_timestamp_ms;
use crate::features::terminal::terminal_runtime::{
    TerminalScrollVisualState, terminal_display_offset_from_state,
    terminal_local_scroll_delta_lines_from_state, terminal_scroll_needs_text_first_repaint,
    terminal_scroll_track_ratio, terminal_visual_scroll_active_for_state,
};
use crate::features::terminal::terminal_selection_runtime::{
    terminal_bounds_tracker, terminal_gutter_metrics, terminal_line_number_digits,
};
use crate::features::terminal::terminal_surface::TERMINAL_SCROLLBAR_COLUMN_WIDTH;
use crate::models::{TerminalPerformanceOverlay, TerminalProtocolState, TerminalSelection};
use crate::terminal::{
    NyaTerminalElement, NyaTerminalLayoutCache, TerminalKeywordHighlightSnapshot,
    TerminalKeywordHighlighter, TerminalLineDecorations, compile_terminal_keyword_highlighter,
    precompute_terminal_keyword_highlights_for_rows, terminal_keyword_highlight_expanded_rows,
    terminal_keyword_rules_key,
};
use crate::theme::ThemePalette;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Process-wide surface paint counter (Phase 0 isolation diagnostics).
pub(in crate::features) static TERMINAL_SURFACE_PAINT_COUNT: AtomicU64 = AtomicU64::new(0);
pub(in crate::features) static FULL_SHELL_PAINT_COUNT: AtomicU64 = AtomicU64::new(0);
const TERMINAL_SURFACE_RETAINED_SNAPSHOT_LIMIT: usize = 12;
const TERMINAL_SURFACE_RETAINED_ROW_LIMIT: usize = 4096;
const TERMINAL_SURFACE_SYNTHESIZED_WINDOW_MIN_EXTRA_ROWS: usize = 32;
const TERMINAL_SURFACE_SYNTHESIZED_WINDOW_MAX_EXTRA_ROWS: usize = 192;
const TERMINAL_SURFACE_SCROLL_PENDING_WARN_AFTER: Duration = Duration::from_millis(48);
const TERMINAL_SURFACE_SCROLL_PENDING_WARN_INTERVAL: Duration = Duration::from_millis(500);
const TERMINAL_SURFACE_LOCAL_SCROLL_SYNC_DELAY: Duration = Duration::from_millis(16);
const TERMINAL_KEYWORD_HIGHLIGHT_PREFETCH_VIEWPORTS: usize = 2;
const TERMINAL_KEYWORD_HIGHLIGHT_SLOW: Duration = Duration::from_millis(8);

#[derive(Clone)]
struct TerminalSurfacePendingScrollSync {
    state: TerminalScrollVisualState,
    generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TerminalSurfaceLocalScrollResult {
    generation: u64,
    visual_changed: bool,
    needs_text_snapshot: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TerminalKeywordHighlightRequestKey {
    rules_key: u64,
    display_offset: usize,
    row_start: usize,
    row_end: usize,
    line_signatures_key: u64,
}

fn terminal_keyword_highlight_request_key(
    snapshot: &TerminalSnapshot,
    rules_key: u64,
    rows: Range<usize>,
) -> TerminalKeywordHighlightRequestKey {
    let rows = terminal_keyword_highlight_expanded_rows(snapshot, rows);
    let row_start = rows.start.min(snapshot.row_count());
    let row_end = rows.end.min(snapshot.row_count()).max(row_start);
    let mut hasher = DefaultHasher::new();
    snapshot
        .rows()
        .get(row_start..row_end)
        .unwrap_or_default()
        .iter()
        .map(|row| (row.signature, row.wrapped))
        .collect::<Vec<_>>()
        .hash(&mut hasher);
    TerminalKeywordHighlightRequestKey {
        rules_key,
        display_offset: snapshot.display_offset,
        row_start,
        row_end,
        line_signatures_key: hasher.finish(),
    }
}

fn terminal_keyword_rule_sets_equal(
    left: &Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>>,
    right: &Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>>,
) -> bool {
    Arc::ptr_eq(left, right) || left.as_ref() == right.as_ref()
}

fn empty_terminal_keyword_rules() -> Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>> {
    static RULES: OnceLock<Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>>> = OnceLock::new();
    Arc::clone(RULES.get_or_init(|| Arc::new(Vec::new())))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::features) struct TerminalSurfaceHitTestScrollGeometry {
    pub(in crate::features) snapshot_pending: bool,
    pub(in crate::features) display_offset: usize,
    pub(in crate::features) snapshot_rows: usize,
    pub(in crate::features) viewport_anchor_row: usize,
}

pub(in crate::features) struct TerminalSurfacePaintChrome {
    pub palette: ThemePalette,
    pub font_family: String,
    pub font_fallbacks: Option<FontFallbacks>,
    pub font_size: f32,
    pub normal_weight: f32,
    pub bold_weight: f32,
    pub cell_width: f32,
    pub cell_height: f32,
    pub show_line_numbers: bool,
    pub show_timestamps: bool,
    pub show_timestamp_ms: bool,
    pub is_active: bool,
    pub visual_bell: bool,
}

pub(in crate::features) struct TerminalSurfaceFrameSnapshot {
    pub snapshot: Arc<TerminalSnapshot>,
    pub scroll: TerminalScrollVisualState,
    pub has_action_link_decorations: bool,
    pub show_cursor: bool,
    pub cursor_style: String,
}

impl TerminalSurfaceFrameSnapshot {
    pub(in crate::features) fn new(
        snapshot: Arc<TerminalSnapshot>,
        scroll: TerminalScrollVisualState,
    ) -> Self {
        Self {
            snapshot,
            scroll,
            has_action_link_decorations: false,
            show_cursor: false,
            cursor_style: "block".to_string(),
        }
    }

    #[cfg(test)]
    fn with_output_state(
        mut self,
        has_new_while_scrolled: bool,
        performance_overlay: Option<TerminalPerformanceOverlay>,
        skipped_output_chars: u64,
    ) -> Self {
        self.scroll.has_new_while_scrolled = has_new_while_scrolled;
        self.scroll.performance_overlay = performance_overlay;
        self.scroll.skipped_output_chars = skipped_output_chars;
        self
    }

    pub(in crate::features) fn with_presentation(
        mut self,
        has_action_link_decorations: bool,
        show_cursor: bool,
        cursor_style: impl Into<String>,
    ) -> Self {
        self.has_action_link_decorations = has_action_link_decorations;
        self.show_cursor = show_cursor;
        self.cursor_style = cursor_style.into();
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::features) struct TerminalVisualScrollGeometry {
    pub snapshot_pending: bool,
    pub target_offset: usize,
    pub displayed_offset: usize,
    pub residual_lines: f32,
    pub viewport_anchor_row: usize,
    pub snapshot_rows: usize,
    pub viewport_rows: usize,
    pub cell_height: f32,
}

pub(in crate::features) fn terminal_surface_paint_count() -> u64 {
    TERMINAL_SURFACE_PAINT_COUNT.load(Ordering::Relaxed)
}

pub(in crate::features) fn full_shell_paint_count() -> u64 {
    FULL_SHELL_PAINT_COUNT.load(Ordering::Relaxed)
}

/// Per-session GPUI entity that owns terminal grid paint state.
///
/// Output frames notify this entity only; chrome (tabs/sidebars/status) stays
/// on `NyaTermApp` and is notified only for unread/effects/layout changes.
pub(in crate::features) struct TerminalSurface {
    session_id: String,
    /// Parent app for scroll/selection actions that still live on NyaTermApp.
    app: Option<Entity<NyaTermApp>>,
    snapshot: Option<Arc<TerminalSnapshot>>,
    retained_snapshots: Vec<Arc<TerminalSnapshot>>,
    retained_rows: BTreeMap<usize, Arc<nyaterm_terminal::TerminalSnapshotRow>>,
    keyword_rules: Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>>,
    keyword_highlights: Option<Arc<TerminalKeywordHighlightSnapshot>>,
    keyword_highlight_generation: u64,
    keyword_highlight_task: Option<gpui::Task<()>>,
    keyword_highlight_pending_key: Option<TerminalKeywordHighlightRequestKey>,
    keyword_highlighter_rules: Option<Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>>>,
    keyword_highlighter: Option<Arc<TerminalKeywordHighlighter>>,
    decorations: Arc<[TerminalLineDecorations]>,
    selection_visual: Option<TerminalSelection>,
    selection_visual_row_range: Option<Range<usize>>,
    palette: ThemePalette,
    font_family: String,
    font_fallbacks: Option<FontFallbacks>,
    font_size: f32,
    normal_weight: f32,
    bold_weight: f32,
    cell_width: f32,
    cell_height: f32,
    show_cursor: bool,
    cursor_style: String,
    layout_cache: Arc<Mutex<NyaTerminalLayoutCache>>,
    show_line_numbers: bool,
    show_timestamps: bool,
    show_timestamp_ms: bool,
    scroll_offset: usize,
    scroll_residual_lines: f32,
    display_offset: usize,
    scroll_snapshot_pending: bool,
    scrollback_len: usize,
    viewport_rows: usize,
    has_new_while_scrolled: bool,
    has_action_link_decorations: bool,
    performance_overlay: Option<TerminalPerformanceOverlay>,
    skipped_output_chars: u64,
    visual_bell: bool,
    transparent_background: bool,
    is_active: bool,
    protocol_state: TerminalProtocolState,
    scroll_interaction_generation: u64,
    pending_local_scroll_sync: Option<TerminalSurfacePendingScrollSync>,
    local_scroll_sync_armed: bool,
    pending_scroll_snapshot_offsets: BTreeSet<usize>,
    revision: u64,
    scroll_snapshot_pending_since: Option<Instant>,
    last_scroll_snapshot_pending_warn_at: Option<Instant>,
}

impl TerminalSurface {
    pub(in crate::features) fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            app: None,
            snapshot: None,
            retained_snapshots: Vec::new(),
            retained_rows: BTreeMap::new(),
            keyword_rules: Arc::new(Vec::new()),
            keyword_highlights: None,
            keyword_highlight_generation: 0,
            keyword_highlight_task: None,
            keyword_highlight_pending_key: None,
            keyword_highlighter_rules: None,
            keyword_highlighter: None,
            decorations: Arc::from(Vec::<TerminalLineDecorations>::new()),
            selection_visual: None,
            selection_visual_row_range: None,
            palette: crate::theme::theme_palette("github-dark"),
            font_family: "monospace".to_string(),
            font_fallbacks: None,
            font_size: 14.0,
            normal_weight: 400.0,
            bold_weight: 700.0,
            cell_width: 8.0,
            cell_height: 16.0,
            show_cursor: false,
            cursor_style: "block".to_string(),
            layout_cache: Arc::new(Mutex::new(NyaTerminalLayoutCache::default())),
            show_line_numbers: false,
            show_timestamps: false,
            show_timestamp_ms: false,
            scroll_offset: 0,
            scroll_residual_lines: 0.0,
            display_offset: 0,
            scroll_snapshot_pending: false,
            scrollback_len: 0,
            viewport_rows: 1,
            has_new_while_scrolled: false,
            has_action_link_decorations: false,
            performance_overlay: None,
            skipped_output_chars: 0,
            visual_bell: false,
            transparent_background: false,
            is_active: false,
            protocol_state: TerminalProtocolState::default(),
            scroll_interaction_generation: 0,
            pending_local_scroll_sync: None,
            local_scroll_sync_armed: false,
            pending_scroll_snapshot_offsets: BTreeSet::new(),
            revision: 0,
            scroll_snapshot_pending_since: None,
            last_scroll_snapshot_pending_warn_at: None,
        }
    }

    pub(in crate::features) fn has_snapshot(&self) -> bool {
        self.snapshot.is_some()
    }

    pub(in crate::features) fn hit_test_scroll_geometry(
        &self,
    ) -> Option<TerminalSurfaceHitTestScrollGeometry> {
        let snapshot = self.snapshot.as_ref()?;
        let viewport_anchor_row = terminal_snapshot_anchor_row_for_display_offset(
            snapshot.as_ref(),
            self.display_offset,
            self.viewport_rows,
            self.scrollback_len,
        );
        Some(TerminalSurfaceHitTestScrollGeometry {
            snapshot_pending: self.scroll_snapshot_pending,
            display_offset: self.display_offset,
            snapshot_rows: snapshot.row_count(),
            viewport_anchor_row,
        })
    }

    pub(in crate::features) fn snapshot_covering_display_offset(
        &self,
        display_offset: usize,
        viewport_rows: usize,
        scrollback_len: usize,
    ) -> Option<Arc<TerminalSnapshot>> {
        self.snapshot
            .as_ref()
            .filter(|snapshot| {
                terminal_snapshot_covers_display_offset(
                    snapshot,
                    display_offset,
                    viewport_rows,
                    scrollback_len,
                )
            })
            .cloned()
            .or_else(|| {
                self.retained_snapshots
                    .iter()
                    .filter(|snapshot| {
                        terminal_snapshot_covers_display_offset(
                            snapshot,
                            display_offset,
                            viewport_rows,
                            scrollback_len,
                        )
                    })
                    .min_by_key(|snapshot| snapshot.display_offset.abs_diff(display_offset))
                    .cloned()
            })
            .or_else(|| {
                self.synthesize_snapshot_covering_display_offset(
                    display_offset,
                    viewport_rows,
                    scrollback_len,
                )
            })
    }

    pub(in crate::features) fn retained_snapshot_covering_display_offset(
        &self,
        display_offset: usize,
        viewport_rows: usize,
        scrollback_len: usize,
    ) -> Option<Arc<TerminalSnapshot>> {
        self.snapshot
            .as_ref()
            .filter(|snapshot| {
                terminal_snapshot_covers_display_offset(
                    snapshot,
                    display_offset,
                    viewport_rows,
                    scrollback_len,
                )
            })
            .cloned()
            .or_else(|| {
                self.retained_snapshots
                    .iter()
                    .filter(|snapshot| {
                        terminal_snapshot_covers_display_offset(
                            snapshot,
                            display_offset,
                            viewport_rows,
                            scrollback_len,
                        )
                    })
                    .min_by_key(|snapshot| snapshot.display_offset.abs_diff(display_offset))
                    .cloned()
            })
    }

    pub(in crate::features) fn has_snapshot_covering_display_offset(
        &self,
        display_offset: usize,
        viewport_rows: usize,
        scrollback_len: usize,
    ) -> bool {
        if self
            .retained_snapshot_covering_display_offset(
                display_offset,
                viewport_rows,
                scrollback_len,
            )
            .is_some()
        {
            return true;
        }
        self.can_synthesize_snapshot_covering_display_offset(
            display_offset,
            viewport_rows,
            scrollback_len,
        )
    }

    pub(in crate::features) fn retain_prefetched_snapshot(
        &mut self,
        snapshot: Arc<TerminalSnapshot>,
    ) -> bool {
        if snapshot.row_count() == 0
            || self.snapshot.as_ref().is_some_and(|current| {
                current.cols != snapshot.cols
                    || current.viewport_rows != snapshot.viewport_rows
                    || snapshot.scrollback_len < self.scrollback_len
            })
            || self
                .retained_snapshots
                .iter()
                .any(|retained| Arc::ptr_eq(retained, &snapshot))
        {
            return false;
        }
        self.remember_retained_snapshot(snapshot);
        self.prune_pending_scroll_snapshot_offsets();
        true
    }

    pub(in crate::features) fn set_app(&mut self, app: Entity<NyaTermApp>) {
        self.app = Some(app);
    }

    pub(in crate::features) fn apply_frame_snapshot(
        &mut self,
        frame: TerminalSurfaceFrameSnapshot,
    ) -> bool {
        // Decorations/keywords are pushed separately so frame notifies can keep
        // selection/search highlights until the next decoration rebuild.
        let TerminalSurfaceFrameSnapshot {
            snapshot,
            mut scroll,
            has_action_link_decorations,
            show_cursor,
            cursor_style,
        } = frame;
        scroll.session_id.clone_from(&self.session_id);
        scroll.viewport_rows = scroll.viewport_rows.max(1);
        let TerminalScrollVisualState {
            session_id: _,
            scroll_offset,
            scroll_residual_lines,
            display_offset,
            scrollback_len,
            viewport_rows,
            has_new_while_scrolled,
            performance_overlay,
            skipped_output_chars,
        } = scroll.clone();
        let desired_show_cursor = show_cursor
            && !terminal_visual_scroll_active_for_state(scroll_offset, scroll_residual_lines);
        let retained_rows_should_reset =
            self.retained_rows_should_reset(snapshot.as_ref(), scrollback_len, viewport_rows);
        let pending_local_scroll_state = (!retained_rows_should_reset)
            .then(|| self.pending_local_scroll_state_reanchored_for_frame(&scroll))
            .flatten();
        if !retained_rows_should_reset
            && pending_local_scroll_state.is_none()
            && !self.has_pending_local_scroll_sync()
            && self
                .snapshot
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, &snapshot))
            && self.scroll_offset == scroll_offset
            && (self.scroll_residual_lines - scroll_residual_lines).abs() < f32::EPSILON * 8.0
            && self.display_offset == display_offset
            && self.scrollback_len == scrollback_len
            && self.viewport_rows == viewport_rows
            && self.has_new_while_scrolled == has_new_while_scrolled
            && self.has_action_link_decorations == has_action_link_decorations
            && self.performance_overlay == performance_overlay
            && self.skipped_output_chars == skipped_output_chars
            && self.show_cursor == desired_show_cursor
            && self.cursor_style == cursor_style
        {
            return false;
        }
        if retained_rows_should_reset {
            self.clear_retained_scroll_state();
        }
        self.remember_retained_snapshot(snapshot.clone());
        if let Some(state) = pending_local_scroll_state {
            let text_updated = self.apply_scroll_visual_state(state);
            if text_updated {
                self.show_cursor = false;
            }
            return false;
        }
        self.snapshot = Some(snapshot);
        self.scroll_offset = scroll_offset;
        self.scroll_residual_lines = scroll_residual_lines;
        self.display_offset = display_offset;
        self.scroll_snapshot_pending = false;
        self.scroll_snapshot_pending_since = None;
        self.clear_pending_scroll_snapshot_offsets_if_scrollback_changed(scrollback_len);
        self.scrollback_len = scrollback_len;
        self.viewport_rows = viewport_rows;
        self.has_new_while_scrolled = has_new_while_scrolled;
        self.has_action_link_decorations = has_action_link_decorations;
        self.performance_overlay = performance_overlay;
        self.skipped_output_chars = skipped_output_chars;
        self.show_cursor = desired_show_cursor;
        self.cursor_style = cursor_style;
        self.prune_pending_scroll_snapshot_offsets();
        self.revision = self.revision.saturating_add(1);
        true
    }

    fn pending_local_scroll_state_reanchored_for_frame(
        &self,
        frame: &TerminalScrollVisualState,
    ) -> Option<TerminalScrollVisualState> {
        let pending = self.pending_local_scroll_sync.as_ref()?;
        let mut state = pending.state.clone();
        if state.scroll_offset == frame.scroll_offset
            && state.display_offset == frame.display_offset
            && (state.scroll_residual_lines - frame.scroll_residual_lines).abs()
                < f32::EPSILON * 8.0
        {
            return None;
        }
        if state.scroll_offset == 0 {
            state.scroll_residual_lines = 0.0;
        } else {
            state.scroll_offset = state
                .scroll_offset
                .saturating_add(frame.scrollback_len.saturating_sub(state.scrollback_len))
                .min(frame.scrollback_len);
            if state.scroll_offset >= frame.scrollback_len && state.scroll_residual_lines > 0.0 {
                state.scroll_residual_lines = 0.0;
            }
        }
        state.scrollback_len = frame.scrollback_len;
        state.viewport_rows = frame.viewport_rows.max(1);
        state.display_offset = terminal_display_offset_from_state(
            state.scroll_offset,
            state.scroll_residual_lines,
            state.scrollback_len,
        );
        state.has_new_while_scrolled = frame.has_new_while_scrolled
            || terminal_visual_scroll_active_for_state(
                state.scroll_offset,
                state.scroll_residual_lines,
            );
        state.performance_overlay = frame.performance_overlay;
        state.skipped_output_chars = frame.skipped_output_chars;
        Some(state)
    }

    fn retained_rows_should_reset(
        &self,
        snapshot: &TerminalSnapshot,
        scrollback_len: usize,
        viewport_rows: usize,
    ) -> bool {
        let viewport_rows = viewport_rows.max(1);
        let Some(previous) = self.snapshot.as_ref() else {
            return false;
        };
        previous.cols != snapshot.cols
            || self.viewport_rows != viewport_rows
            || scrollback_len < self.scrollback_len
            || snapshot.total_rows < previous.total_rows
    }

    fn clear_retained_scroll_state(&mut self) {
        self.retained_snapshots.clear();
        self.retained_rows.clear();
        self.decorations = Arc::from(Vec::<TerminalLineDecorations>::new());
        self.selection_visual = None;
        self.selection_visual_row_range = None;
        self.has_action_link_decorations = false;
        self.scroll_snapshot_pending = false;
        self.scroll_snapshot_pending_since = None;
        self.pending_scroll_snapshot_offsets.clear();
    }

    pub(in crate::features) fn update_scroll_chrome_without_snapshot(
        &mut self,
        state: &TerminalScrollVisualState,
    ) -> bool {
        let scroll_offset = state.scroll_offset;
        let scroll_residual_lines = state.scroll_residual_lines;
        let display_offset = state.display_offset;
        let scrollback_len = state.scrollback_len;
        let viewport_rows = state.viewport_rows;
        let has_new_while_scrolled = state.has_new_while_scrolled;
        let performance_overlay = state.performance_overlay;
        let skipped_output_chars = state.skipped_output_chars;
        let state_matches = self.scroll_offset == scroll_offset
            && (self.scroll_residual_lines - scroll_residual_lines).abs() < f32::EPSILON * 8.0
            && self.scrollback_len == scrollback_len
            && self.viewport_rows == viewport_rows.max(1)
            && self.has_new_while_scrolled == has_new_while_scrolled
            && self.performance_overlay == performance_overlay
            && self.skipped_output_chars == skipped_output_chars
            && !self.show_cursor
            && if self.snapshot.is_some() {
                self.scroll_snapshot_pending == (self.display_offset != display_offset)
            } else {
                self.display_offset == display_offset && !self.scroll_snapshot_pending
            };
        if state_matches {
            return false;
        }
        self.scroll_offset = scroll_offset;
        self.scroll_residual_lines = scroll_residual_lines;
        if self.snapshot.is_none() {
            self.display_offset = display_offset;
        }
        self.set_scroll_snapshot_pending(
            self.snapshot.is_some() && self.display_offset != display_offset,
        );
        self.clear_pending_scroll_snapshot_offsets_if_scrollback_changed(scrollback_len);
        self.scrollback_len = scrollback_len;
        self.viewport_rows = viewport_rows.max(1);
        self.has_new_while_scrolled = has_new_while_scrolled;
        self.performance_overlay = performance_overlay;
        self.skipped_output_chars = skipped_output_chars;
        // Keep decorations tied to the retained snapshot while the target
        // scrollback snapshot is still loading. The editor follows the same
        // stale-until-recomputed rule for highlights: old adornments are better
        // than a visible flash to an undecorated terminal surface.
        self.show_cursor = false;
        self.revision = self.revision.saturating_add(1);
        true
    }

    pub(in crate::features) fn update_scroll_position_without_snapshot(
        &mut self,
        state: &TerminalScrollVisualState,
    ) -> bool {
        let scroll_offset = state.scroll_offset;
        let scroll_residual_lines = state.scroll_residual_lines;
        let display_offset = state.display_offset;
        let scrollback_len = state.scrollback_len;
        let viewport_rows = state.viewport_rows;
        let has_new_while_scrolled = state.has_new_while_scrolled;
        let performance_overlay = state.performance_overlay;
        let skipped_output_chars = state.skipped_output_chars;
        let viewport_rows = viewport_rows.max(1);
        let snapshot_covers_display_offset = self.snapshot.as_ref().is_some_and(|snapshot| {
            terminal_snapshot_covers_display_offset(
                snapshot.as_ref(),
                display_offset,
                viewport_rows,
                scrollback_len,
            )
        });
        let state_matches = snapshot_covers_display_offset
            && self.scroll_offset == scroll_offset
            && (self.scroll_residual_lines - scroll_residual_lines).abs() < f32::EPSILON * 8.0
            && self.display_offset == display_offset
            && self.scrollback_len == scrollback_len
            && self.viewport_rows == viewport_rows
            && self.has_new_while_scrolled == has_new_while_scrolled
            && self.performance_overlay == performance_overlay
            && self.skipped_output_chars == skipped_output_chars
            && (!self.visual_scroll_active() || !self.show_cursor);
        if state_matches {
            return false;
        }
        self.promote_snapshot_covering_display_offset(
            display_offset,
            viewport_rows,
            scrollback_len,
        );
        // This is the scroll-only path. Keep stale decorations attached until a
        // full surface sync computes replacements, matching Zed/editor style
        // stale-until-recomputed highlights and avoiding a flash to plain text.
        self.scroll_offset = scroll_offset;
        self.scroll_residual_lines = scroll_residual_lines;
        self.display_offset = display_offset;
        self.scroll_snapshot_pending = false;
        self.scroll_snapshot_pending_since = None;
        self.clear_pending_scroll_snapshot_offsets_if_scrollback_changed(scrollback_len);
        self.scrollback_len = scrollback_len;
        self.viewport_rows = viewport_rows.max(1);
        self.has_new_while_scrolled = has_new_while_scrolled;
        self.performance_overlay = performance_overlay;
        self.skipped_output_chars = skipped_output_chars;
        if self.visual_scroll_active() {
            self.show_cursor = false;
        }
        self.prune_pending_scroll_snapshot_offsets();
        self.revision = self.revision.saturating_add(1);
        true
    }

    fn scroll_snapshot_request_offsets_to_enqueue(&mut self, offsets: Vec<usize>) -> Vec<usize> {
        let mut offsets = offsets
            .into_iter()
            .filter(|offset| *offset > 0)
            .collect::<Vec<_>>();
        offsets.sort_unstable();
        offsets.dedup();
        offsets
            .into_iter()
            .filter(|offset| {
                !self.has_snapshot_covering_display_offset(
                    *offset,
                    self.viewport_rows,
                    self.scrollback_len,
                ) && self.pending_scroll_snapshot_offsets.insert(*offset)
            })
            .collect()
    }

    fn clear_pending_scroll_snapshot_offsets_if_scrollback_changed(
        &mut self,
        next_scrollback_len: usize,
    ) {
        if next_scrollback_len != self.scrollback_len {
            self.pending_scroll_snapshot_offsets.clear();
        }
    }

    fn prune_pending_scroll_snapshot_offsets(&mut self) {
        if self.pending_scroll_snapshot_offsets.is_empty() {
            return;
        }
        let viewport_rows = self.viewport_rows;
        let scrollback_len = self.scrollback_len;
        let resolved = self
            .pending_scroll_snapshot_offsets
            .iter()
            .copied()
            .filter(|offset| {
                self.has_snapshot_covering_display_offset(*offset, viewport_rows, scrollback_len)
            })
            .collect::<Vec<_>>();
        for offset in resolved {
            self.pending_scroll_snapshot_offsets.remove(&offset);
        }
    }

    fn set_scroll_snapshot_pending(&mut self, pending: bool) {
        if pending {
            if !self.scroll_snapshot_pending {
                self.scroll_snapshot_pending_since = Some(Instant::now());
            }
        } else {
            self.scroll_snapshot_pending_since = None;
        }
        self.scroll_snapshot_pending = pending;
    }

    fn maybe_log_scroll_snapshot_pending(&mut self, snapshot: &TerminalSnapshot) {
        if !self.scroll_snapshot_pending {
            return;
        }
        let Some(pending_since) = self.scroll_snapshot_pending_since else {
            return;
        };
        let now = Instant::now();
        let pending_for = now.saturating_duration_since(pending_since);
        if pending_for < TERMINAL_SURFACE_SCROLL_PENDING_WARN_AFTER {
            return;
        }
        if self
            .last_scroll_snapshot_pending_warn_at
            .is_some_and(|last| {
                now.saturating_duration_since(last) < TERMINAL_SURFACE_SCROLL_PENDING_WARN_INTERVAL
            })
        {
            return;
        }
        self.last_scroll_snapshot_pending_warn_at = Some(now);
        tracing::warn!(
            diagnostic = "terminal_surface_scroll_snapshot_pending",
            session_id = %self.session_id,
            scroll_offset = self.scroll_offset,
            display_offset = self.display_offset,
            residual_lines = self.scroll_residual_lines,
            scrollback_len = self.scrollback_len,
            viewport_rows = self.viewport_rows,
            snapshot_display_offset = snapshot.display_offset,
            snapshot_rows = snapshot.row_count(),
            snapshot_total_rows = snapshot.total_rows,
            retained_snapshots = self.retained_snapshots.len(),
            retained_rows = self.retained_rows.len(),
            pending_ms = pending_for.as_millis(),
            "terminal surface retained text while waiting for target scroll snapshot"
        );
    }

    fn remember_retained_snapshot(&mut self, snapshot: Arc<TerminalSnapshot>) {
        if snapshot.row_count() == 0 {
            return;
        }
        self.remember_retained_snapshot_rows(snapshot.as_ref());
        self.retained_snapshots.retain(|retained| {
            !(retained.display_offset == snapshot.display_offset
                && retained.total_rows == snapshot.total_rows
                && retained.row_count() == snapshot.row_count())
        });
        self.retained_snapshots.push(snapshot);
        let excess = self
            .retained_snapshots
            .len()
            .saturating_sub(TERMINAL_SURFACE_RETAINED_SNAPSHOT_LIMIT);
        if excess > 0 {
            self.retained_snapshots.drain(0..excess);
        }
    }

    fn remember_retained_snapshot_rows(&mut self, snapshot: &TerminalSnapshot) -> usize {
        let Some((start, _)) = terminal_snapshot_absolute_window(snapshot) else {
            return 0;
        };
        let mut refreshed_rows = 0usize;
        for row in 0..snapshot.row_count() {
            let Some(abs_row) = start.checked_add(row) else {
                continue;
            };
            let Some(snapshot_row) = snapshot.rows().get(row) else {
                continue;
            };
            if self.retained_rows.get(&abs_row).is_some_and(|retained| {
                Arc::ptr_eq(retained, snapshot_row) || retained.as_ref() == snapshot_row.as_ref()
            }) {
                continue;
            }
            self.retained_rows.insert(abs_row, Arc::clone(snapshot_row));
            refreshed_rows = refreshed_rows.saturating_add(1);
        }
        while self.retained_rows.len() > TERMINAL_SURFACE_RETAINED_ROW_LIMIT {
            let Some(drop_key) = self.retained_rows.keys().next().copied() else {
                break;
            };
            self.retained_rows.remove(&drop_key);
        }
        refreshed_rows
    }

    fn promote_snapshot_covering_display_offset(
        &mut self,
        display_offset: usize,
        viewport_rows: usize,
        scrollback_len: usize,
    ) -> bool {
        // Scrolling usually stays inside the retained window already assigned
        // to the surface. Keep that Arc in place: rebuilding retained rows here
        // clones the entire viewport on every pixel-wheel event.
        if self.snapshot.as_ref().is_some_and(|snapshot| {
            terminal_snapshot_covers_display_offset(
                snapshot,
                display_offset,
                viewport_rows,
                scrollback_len,
            )
        }) {
            return true;
        }
        let Some(snapshot) =
            self.snapshot_covering_display_offset(display_offset, viewport_rows, scrollback_len)
        else {
            return false;
        };
        let already_retained = self
            .retained_snapshots
            .iter()
            .any(|retained| Arc::ptr_eq(retained, &snapshot));
        if !already_retained {
            self.remember_retained_snapshot(snapshot.clone());
        }
        self.snapshot = Some(snapshot);
        true
    }

    fn can_synthesize_snapshot_covering_display_offset(
        &self,
        display_offset: usize,
        viewport_rows: usize,
        scrollback_len: usize,
    ) -> bool {
        let viewport_rows = viewport_rows.max(1);
        let real_total_rows = scrollback_len.saturating_add(viewport_rows);
        let desired_end = real_total_rows.saturating_sub(display_offset);
        let desired_start = desired_end.saturating_sub(viewport_rows);
        self.retained_rows_cover_absolute_range(desired_start, desired_end)
            || self.snapshot_sources_cover_absolute_range(desired_start, desired_end)
    }

    fn retained_rows_cover_absolute_range(&self, desired_start: usize, desired_end: usize) -> bool {
        let Some(first_row) = self.retained_rows.get(&desired_start) else {
            return false;
        };
        let cols = first_row.cells.len();
        if cols == 0 {
            return false;
        }
        (desired_start..desired_end).all(|abs_row| {
            self.retained_rows
                .get(&abs_row)
                .is_some_and(|row| row.cells.len() == cols)
        })
    }

    fn snapshot_sources_cover_absolute_range(
        &self,
        desired_start: usize,
        desired_end: usize,
    ) -> bool {
        let source_cols = self
            .snapshot
            .iter()
            .chain(self.retained_snapshots.iter())
            .filter(|snapshot| {
                terminal_snapshot_absolute_window(snapshot)
                    .is_some_and(|(start, end)| start < desired_end && desired_start < end)
            })
            .map(|snapshot| snapshot.cols)
            .find(|cols| *cols > 0);
        let Some(cols) = source_cols else {
            return false;
        };
        if self
            .snapshot
            .iter()
            .chain(self.retained_snapshots.iter())
            .any(|snapshot| snapshot.cols > 0 && snapshot.cols != cols)
        {
            return false;
        }
        (desired_start..desired_end).all(|abs_row| {
            self.snapshot
                .iter()
                .chain(self.retained_snapshots.iter())
                .any(|snapshot| {
                    terminal_snapshot_row_for_absolute_row(snapshot, abs_row).is_some_and(|row| {
                        snapshot
                            .row(row)
                            .is_some_and(|snapshot_row| snapshot_row.cells.len() == cols)
                    })
                })
        })
    }

    fn synthesize_snapshot_covering_display_offset(
        &self,
        display_offset: usize,
        viewport_rows: usize,
        scrollback_len: usize,
    ) -> Option<Arc<TerminalSnapshot>> {
        let viewport_rows = viewport_rows.max(1);
        let real_total_rows = scrollback_len.saturating_add(viewport_rows);
        let desired_end = real_total_rows.checked_sub(display_offset)?;
        let desired_start = desired_end.checked_sub(viewport_rows)?;
        let extra_rows = terminal_surface_synthesized_window_extra_rows(viewport_rows);
        let retained_window = self.retained_rows_window_around(
            desired_start,
            desired_end,
            real_total_rows,
            extra_rows,
        );
        if let Some((window_start, window_end)) = retained_window
            && let Some(snapshot) = self.synthesize_snapshot_from_retained_rows(
                viewport_rows,
                scrollback_len,
                window_start,
                window_end,
            )
        {
            return Some(snapshot);
        }
        let mut sources: Vec<&Arc<TerminalSnapshot>> = Vec::new();
        if let Some(snapshot) = self.snapshot.as_ref() {
            sources.push(snapshot);
        }
        sources.extend(self.retained_snapshots.iter());

        let cols = sources
            .iter()
            .filter(|snapshot| {
                terminal_snapshot_absolute_window(snapshot)
                    .is_some_and(|(start, end)| start < desired_end && desired_start < end)
            })
            .map(|snapshot| snapshot.cols)
            .find(|cols| *cols > 0)?;
        if sources
            .iter()
            .any(|snapshot| snapshot.cols > 0 && snapshot.cols != cols)
        {
            return None;
        }

        let mut rows = Vec::with_capacity(viewport_rows);

        for abs_row in desired_start..desired_end {
            let (snapshot, row) = sources
                .iter()
                .filter_map(|snapshot| {
                    terminal_snapshot_row_for_absolute_row(snapshot, abs_row)
                        .map(|row| (*snapshot, row))
                })
                .min_by_key(|(snapshot, _)| snapshot.display_offset.abs_diff(display_offset))?;
            rows.push(snapshot.rows().get(row)?.clone());
        }

        Some(Arc::new(TerminalSnapshot::from_rows(
            nyaterm_terminal::TerminalSnapshotMeta {
                cols,
                viewport_rows,
                cursor: hidden_terminal_cursor_snapshot(),
                selection: None,
                scrollback_len,
                total_rows: real_total_rows,
                display_offset,
                images: Vec::new(),
            },
            rows,
        )))
    }

    fn synthesize_snapshot_from_retained_rows(
        &self,
        viewport_rows: usize,
        scrollback_len: usize,
        window_start: usize,
        window_end: usize,
    ) -> Option<Arc<TerminalSnapshot>> {
        let first_row = self.retained_rows.get(&window_start)?;
        let cols = first_row.cells.len();
        let rows = window_end.checked_sub(window_start)?;
        if cols == 0 || rows < viewport_rows.max(1) {
            return None;
        }
        let real_total_rows = scrollback_len.saturating_add(viewport_rows);
        let display_offset = real_total_rows.checked_sub(window_end)?;
        let mut row_data = Vec::with_capacity(rows);

        for abs_row in window_start..window_end {
            let row = self.retained_rows.get(&abs_row)?;
            if row.cells.len() != cols {
                return None;
            }
            row_data.push(Arc::clone(row));
        }
        Some(Arc::new(TerminalSnapshot::from_rows(
            nyaterm_terminal::TerminalSnapshotMeta {
                cols,
                viewport_rows,
                cursor: hidden_terminal_cursor_snapshot(),
                selection: None,
                scrollback_len,
                total_rows: real_total_rows,
                display_offset,
                images: Vec::new(),
            },
            row_data,
        )))
    }

    fn retained_rows_window_around(
        &self,
        desired_start: usize,
        desired_end: usize,
        real_total_rows: usize,
        extra_rows: usize,
    ) -> Option<(usize, usize)> {
        if !self.retained_rows_cover_absolute_range(desired_start, desired_end) {
            return None;
        }
        let cols = self.retained_rows.get(&desired_start)?.cells.len();
        let row_is_compatible = |absolute_row: usize| {
            self.retained_rows
                .get(&absolute_row)
                .is_some_and(|row| row.cells.len() == cols)
        };

        let mut window_start = desired_start;
        while desired_start.saturating_sub(window_start) < extra_rows && window_start > 0 {
            let previous = window_start - 1;
            if !row_is_compatible(previous) {
                break;
            }
            window_start = previous;
        }

        let mut window_end = desired_end;
        while window_end.saturating_sub(desired_end) < extra_rows
            && window_end < real_total_rows
            && row_is_compatible(window_end)
        {
            window_end = window_end.saturating_add(1);
        }
        Some((window_start, window_end))
    }

    pub(in crate::features) fn set_paint_chrome(
        &mut self,
        chrome: TerminalSurfacePaintChrome,
    ) -> bool {
        let TerminalSurfacePaintChrome {
            palette,
            font_family,
            font_fallbacks,
            font_size,
            normal_weight,
            bold_weight,
            cell_width,
            cell_height,
            show_line_numbers,
            show_timestamps,
            show_timestamp_ms,
            is_active,
            visual_bell,
        } = chrome;
        let cell_width = cell_width.max(1.0);
        let cell_height = cell_height.max(1.0);
        let state_matches = self.palette == palette
            && self.font_family == font_family
            && self.font_fallbacks == font_fallbacks
            && (self.font_size - font_size).abs() < f32::EPSILON * 8.0
            && (self.normal_weight - normal_weight).abs() < f32::EPSILON * 8.0
            && (self.bold_weight - bold_weight).abs() < f32::EPSILON * 8.0
            && (self.cell_width - cell_width).abs() < f32::EPSILON * 8.0
            && (self.cell_height - cell_height).abs() < f32::EPSILON * 8.0
            && self.show_line_numbers == show_line_numbers
            && self.show_timestamps == show_timestamps
            && self.show_timestamp_ms == show_timestamp_ms
            && self.is_active == is_active
            && self.visual_bell == visual_bell;
        if state_matches {
            return false;
        }
        self.palette = palette;
        self.font_family = font_family;
        self.font_fallbacks = font_fallbacks;
        self.font_size = font_size;
        self.normal_weight = normal_weight;
        self.bold_weight = bold_weight;
        self.cell_width = cell_width;
        self.cell_height = cell_height;
        self.show_line_numbers = show_line_numbers;
        self.show_timestamps = show_timestamps;
        self.show_timestamp_ms = show_timestamp_ms;
        self.is_active = is_active;
        self.visual_bell = visual_bell;
        true
    }

    pub(in crate::features) fn set_background_transparent(&mut self, transparent: bool) -> bool {
        if self.transparent_background == transparent {
            return false;
        }
        self.transparent_background = transparent;
        true
    }

    pub(in crate::features) fn set_protocol_state(
        &mut self,
        protocol_state: TerminalProtocolState,
    ) -> bool {
        if self.protocol_state == protocol_state {
            return false;
        }
        self.protocol_state = protocol_state;
        true
    }

    pub(in crate::features) fn set_layout_cache(
        &mut self,
        layout_cache: Arc<Mutex<NyaTerminalLayoutCache>>,
    ) -> bool {
        if Arc::ptr_eq(&self.layout_cache, &layout_cache) {
            return false;
        }
        self.layout_cache = layout_cache;
        true
    }

    pub(in crate::features) fn set_decorations_and_keywords(
        &mut self,
        decorations: impl Into<Arc<[TerminalLineDecorations]>>,
        keyword_rules: Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>>,
        show_cursor: bool,
        cursor_style: impl Into<String>,
    ) -> bool {
        let decorations = decorations.into();
        self.apply_decorations_and_keywords(decorations, keyword_rules, show_cursor, cursor_style)
    }

    pub(in crate::features) fn set_decorations_and_keywords_preserving_stale(
        &mut self,
        decorations: impl Into<Arc<[TerminalLineDecorations]>>,
        keyword_rules: Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>>,
        show_cursor: bool,
        cursor_style: impl Into<String>,
        allow_empty_stale: bool,
    ) -> bool {
        let decorations = decorations.into();
        if allow_empty_stale && decorations.is_empty() && !self.decorations.is_empty() {
            let show_cursor = show_cursor && !self.visual_scroll_active();
            let cursor_style = cursor_style.into();
            let changed = !terminal_keyword_rule_sets_equal(&self.keyword_rules, &keyword_rules)
                || self.show_cursor != show_cursor
                || self.cursor_style != cursor_style;
            self.keyword_rules = keyword_rules;
            self.show_cursor = show_cursor;
            self.cursor_style = cursor_style;
            return changed;
        }
        self.apply_decorations_and_keywords(decorations, keyword_rules, show_cursor, cursor_style)
    }

    fn apply_decorations_and_keywords(
        &mut self,
        decorations: Arc<[TerminalLineDecorations]>,
        keyword_rules: Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>>,
        show_cursor: bool,
        cursor_style: impl Into<String>,
    ) -> bool {
        let selection_visual_row_range =
            terminal_selection_visual_row_range_from_decorations(&decorations);
        let show_cursor = show_cursor && !self.visual_scroll_active();
        let cursor_style = cursor_style.into();
        let changed = self.selection_visual.is_some()
            || self.selection_visual_row_range != selection_visual_row_range
            || self.decorations.as_ref() != decorations.as_ref()
            || !terminal_keyword_rule_sets_equal(&self.keyword_rules, &keyword_rules)
            || self.show_cursor != show_cursor
            || self.cursor_style != cursor_style;
        if !changed {
            return false;
        }
        self.selection_visual = None;
        self.selection_visual_row_range = selection_visual_row_range;
        self.decorations = decorations;
        self.keyword_rules = keyword_rules;
        self.show_cursor = show_cursor;
        self.cursor_style = cursor_style;
        true
    }

    pub(in crate::features) fn schedule_keyword_highlights(
        &mut self,
        clear_if_empty: bool,
        cx: &mut Context<Self>,
    ) {
        if self.handle_empty_keyword_rules_for_highlights(clear_if_empty) {
            return;
        }
        let Some(snapshot) = self.snapshot.clone() else {
            return;
        };
        let visible_rows = terminal_keyword_highlight_visible_rows(
            snapshot.as_ref(),
            self.display_offset,
            self.viewport_rows,
            self.scrollback_len,
        );
        let visible_rows =
            terminal_keyword_highlight_expanded_rows(snapshot.as_ref(), visible_rows);
        let rules = self.keyword_rules.clone();
        let rules_key = terminal_keyword_rules_key(rules.as_slice());
        if self.keyword_highlights.as_ref().is_some_and(|highlights| {
            highlights.rules_key() == rules_key
                && highlights.matches_snapshot_rows(
                    snapshot.as_ref(),
                    self.palette,
                    visible_rows.clone(),
                )
        }) {
            if self.keyword_highlight_task.is_none() {
                self.keyword_highlight_pending_key = None;
            }
            return;
        }
        let requested_rows = terminal_keyword_highlight_prefetch_rows(
            snapshot.as_ref(),
            self.display_offset,
            self.viewport_rows,
            self.scrollback_len,
        );
        let requested_row_count = requested_rows.len();
        let requested_rows =
            terminal_keyword_highlight_expanded_rows(snapshot.as_ref(), requested_rows);
        let expanded_requested_row_count = requested_rows.len();
        let request_key = terminal_keyword_highlight_request_key(
            snapshot.as_ref(),
            rules_key,
            requested_rows.clone(),
        );
        if self.keyword_highlight_pending_key == Some(request_key) {
            return;
        }
        // Let one parse finish, then have its completion schedule the newest
        // surface snapshot. Replacing the task on every output frame can starve
        // highlighting indefinitely during continuous input.
        if self.keyword_highlight_task.is_some() {
            return;
        }
        self.keyword_highlight_generation = self.keyword_highlight_generation.saturating_add(1);
        let generation = self.keyword_highlight_generation;
        self.keyword_highlight_pending_key = Some(request_key);
        // Keep the last published snapshot drawable while the replacement is parsed in the
        // background, matching the editor's stale-until-reparsed behavior.
        let highlighter = self.cached_keyword_highlighter_for_rules(&rules);
        let palette = self.palette;
        let previous_highlights = self.keyword_highlights.clone();
        self.keyword_highlight_task = Some(cx.spawn(async move |this, cx| {
            let (rules, highlighter, highlights, highlight_duration) = cx
                .background_spawn(async move {
                    let highlighter = highlighter.unwrap_or_else(|| {
                        Arc::new(compile_terminal_keyword_highlighter(rules.as_ref()))
                    });
                    let highlight_started_at = Instant::now();
                    let highlights = precompute_terminal_keyword_highlights_for_rows(
                        snapshot.as_ref(),
                        highlighter.as_ref(),
                        palette,
                        previous_highlights.as_deref(),
                        requested_rows,
                    );
                    let highlight_duration = highlight_started_at.elapsed();
                    (rules, highlighter, highlights, highlight_duration)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.keyword_highlight_generation != generation {
                    return;
                }
                this.keyword_highlight_task = None;
                this.keyword_highlight_pending_key = None;
                let publish = terminal_keyword_rule_sets_equal(&this.keyword_rules, &rules);
                if publish {
                    if highlight_duration >= TERMINAL_KEYWORD_HIGHLIGHT_SLOW {
                        tracing::warn!(
                            diagnostic = "terminal_keyword_highlight_slow",
                            session_id = %this.session_id,
                            rules = rules.len(),
                            requested_rows = requested_row_count,
                            expanded_rows = expanded_requested_row_count,
                            known_rows = highlights.known_row_count(),
                            ranges = highlights.range_count(),
                            duration_us = highlight_duration.as_micros(),
                            "slow terminal keyword highlight precompute"
                        );
                    }
                    this.keyword_highlighter_rules = Some(rules);
                    this.keyword_highlighter = Some(highlighter);
                    this.keyword_highlights = Some(Arc::new(highlights));
                    cx.notify();
                }
                // The surface may have advanced while this task was running.
                // Schedule exactly one follow-up for its latest snapshot.
                this.schedule_keyword_highlights(clear_if_empty, cx);
            });
        }));
    }

    fn handle_empty_keyword_rules_for_highlights(&mut self, clear_if_empty: bool) -> bool {
        if !self.keyword_rules.is_empty() {
            return false;
        }
        if clear_if_empty {
            self.cancel_pending_keyword_highlights();
            self.keyword_highlighter_rules = None;
            self.keyword_highlighter = None;
            self.keyword_highlights = None;
        }
        true
    }

    fn cached_keyword_highlighter_for_rules(
        &self,
        rules: &Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>>,
    ) -> Option<Arc<TerminalKeywordHighlighter>> {
        self.keyword_highlighter_rules
            .as_ref()
            .filter(|cached_rules| terminal_keyword_rule_sets_equal(cached_rules, rules))
            .and(self.keyword_highlighter.clone())
    }

    fn cancel_pending_keyword_highlights(&mut self) {
        if self.keyword_highlight_task.is_none() && self.keyword_highlight_pending_key.is_none() {
            return;
        }
        self.keyword_highlight_generation = self.keyword_highlight_generation.saturating_add(1);
        self.keyword_highlight_task = None;
        self.keyword_highlight_pending_key = None;
    }

    pub(in crate::features) fn set_selection_visual(
        &mut self,
        selection: Option<TerminalSelection>,
    ) -> bool {
        let Some(snapshot) = self.snapshot.as_ref() else {
            return false;
        };
        let line_count = snapshot.row_count();
        if line_count == 0 {
            return false;
        }
        if self.selection_visual == selection {
            return false;
        }

        let next_rows = terminal_selection_visual_row_range(selection, line_count);
        let update_rows = terminal_selection_visual_row_union(
            self.selection_visual_row_range.clone(),
            next_rows.clone(),
        );
        let Some(update_rows) = update_rows else {
            if self.decorations.is_empty() {
                self.selection_visual = selection;
                self.selection_visual_row_range = None;
                return false;
            }
            self.selection_visual = selection;
            self.selection_visual_row_range = None;
            self.decorations = Arc::from(Vec::<TerminalLineDecorations>::new());
            self.revision = self.revision.saturating_add(1);
            return true;
        };

        let mut next = if self.decorations.is_empty() {
            vec![TerminalLineDecorations::default(); line_count]
        } else {
            let mut decorations = self.decorations.as_ref().to_vec();
            decorations.resize(line_count, TerminalLineDecorations::default());
            decorations
        };

        let mut changed = false;
        for line_index in update_rows {
            let selection_cols = selection.and_then(|selection| {
                let viewport_row = line_index.checked_sub(selection.viewport_anchor_row)?;
                let (start, end) = selection.cols_for_row(viewport_row)?;
                let start = start.min(snapshot.cols);
                let end = end.min(snapshot.cols);
                (end > start).then_some((start, end))
            });
            let decoration = &mut next[line_index];
            if decoration.selection_cols != selection_cols {
                decoration.selection_cols = selection_cols;
                changed = true;
            }
        }
        if !changed {
            self.selection_visual = selection;
            self.selection_visual_row_range = next_rows;
            return false;
        }

        self.selection_visual = selection;
        self.selection_visual_row_range = next_rows;
        if next
            .iter()
            .all(|decoration| *decoration == TerminalLineDecorations::default())
        {
            self.decorations = Arc::from(Vec::<TerminalLineDecorations>::new());
        } else {
            self.decorations = Arc::from(next);
        }
        self.revision = self.revision.saturating_add(1);
        true
    }

    fn visual_scroll_active(&self) -> bool {
        terminal_visual_scroll_active_for_state(self.scroll_offset, self.scroll_residual_lines)
    }

    pub(in crate::features) fn scroll_visual_state_matches(
        &self,
        state: &TerminalScrollVisualState,
    ) -> bool {
        self.scroll_offset == state.scroll_offset
            && (self.scroll_residual_lines - state.scroll_residual_lines).abs() < f32::EPSILON * 8.0
            && self.display_offset == state.display_offset
            && self.scrollback_len == state.scrollback_len
            && self.viewport_rows == state.viewport_rows
            && self.has_new_while_scrolled == state.has_new_while_scrolled
            && self.performance_overlay == state.performance_overlay
            && self.skipped_output_chars == state.skipped_output_chars
    }

    fn scroll_position_state_matches(&self, state: &TerminalScrollVisualState) -> bool {
        self.snapshot.as_ref().is_some_and(|snapshot| {
            terminal_snapshot_covers_display_offset(
                snapshot.as_ref(),
                state.display_offset,
                state.viewport_rows,
                state.scrollback_len,
            )
        }) && self.scroll_visual_state_matches(state)
            && self.display_offset == state.display_offset
            && !self.scroll_snapshot_pending
            && (!self.visual_scroll_active() || !self.show_cursor)
    }

    fn scroll_chrome_state_matches(&self, state: &TerminalScrollVisualState) -> bool {
        self.scroll_offset == state.scroll_offset
            && (self.scroll_residual_lines - state.scroll_residual_lines).abs() < f32::EPSILON * 8.0
            && self.scrollback_len == state.scrollback_len
            && self.viewport_rows == state.viewport_rows.max(1)
            && self.has_new_while_scrolled == state.has_new_while_scrolled
            && self.performance_overlay == state.performance_overlay
            && self.skipped_output_chars == state.skipped_output_chars
            && if self.snapshot.is_some() {
                self.scroll_snapshot_pending == (self.display_offset != state.display_offset)
            } else {
                self.display_offset == state.display_offset && !self.scroll_snapshot_pending
            }
            && !self.show_cursor
    }

    pub(in crate::features) fn has_pending_local_scroll_sync(&self) -> bool {
        self.pending_local_scroll_sync.is_some()
    }

    fn defer_surface_repaint(
        app: Entity<NyaTermApp>,
        session_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        cx.defer(move |cx| {
            app.update(cx, |this, cx| {
                this.notify_terminal_scroll_after_state_change(session_id.as_deref(), cx);
            });
        });
    }

    fn defer_local_scroll_snapshot_requests(
        app: Entity<NyaTermApp>,
        session_id: String,
        request_offsets: Vec<usize>,
        cx: &mut Context<Self>,
    ) {
        if session_id.is_empty() || request_offsets.is_empty() {
            return;
        }
        cx.defer(move |cx| {
            app.update(cx, |this, _cx| {
                for request_offset in request_offsets {
                    this.request_terminal_frame_snapshot_for_user_scroll(
                        session_id.as_str(),
                        request_offset,
                    );
                }
            });
        });
    }

    pub(in crate::features) fn apply_scroll_visual_state(
        &mut self,
        state: TerminalScrollVisualState,
    ) -> bool {
        if self.has_snapshot_covering_display_offset(
            state.display_offset,
            state.viewport_rows,
            state.scrollback_len,
        ) {
            if self.scroll_position_state_matches(&state) {
                return false;
            }
            self.update_scroll_position_without_snapshot(&state);
            true
        } else {
            if self.scroll_chrome_state_matches(&state) {
                return false;
            }
            self.update_scroll_chrome_without_snapshot(&state);
            false
        }
    }

    fn scroll_visual_state_needs_repaint(&self, state: &TerminalScrollVisualState) -> bool {
        if self.has_snapshot_covering_display_offset(
            state.display_offset,
            state.viewport_rows,
            state.scrollback_len,
        ) {
            !self.scroll_position_state_matches(state)
        } else {
            !self.scroll_chrome_state_matches(state)
        }
    }

    fn handle_scroll_wheel(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) {
        let Some(app) = self.app.clone() else {
            return;
        };
        let session_id = self.session_id.clone();
        if session_id.is_empty() {
            return;
        }
        let raw_lines = match event.delta {
            ScrollDelta::Lines(delta) => delta.y,
            ScrollDelta::Pixels(delta) => f32::from(delta.y) / self.cell_height.max(1.0),
        };
        if self.can_handle_scroll_wheel_locally()
            && let Some(result) = self.apply_local_scroll_wheel_visual_state(raw_lines)
        {
            if result.visual_changed {
                let state = self.current_scroll_visual_state();
                self.schedule_keyword_highlights(false, cx);
                cx.notify();
                let mut request_offsets = Vec::new();
                if result.needs_text_snapshot && state.display_offset > 0 {
                    request_offsets.push(state.display_offset);
                }
                if let Some(prefetch_offset) = terminal_surface_fractional_prefetch_offset(
                    state.scroll_offset,
                    state.scroll_residual_lines,
                    state.scrollback_len,
                ) {
                    request_offsets.push(prefetch_offset);
                }
                request_offsets.sort_unstable();
                request_offsets.dedup();
                let request_offsets =
                    self.scroll_snapshot_request_offsets_to_enqueue(request_offsets);
                if !request_offsets.is_empty() {
                    Self::defer_local_scroll_snapshot_requests(
                        app.clone(),
                        state.session_id.clone(),
                        request_offsets,
                        cx,
                    );
                }
                self.queue_local_scroll_app_sync(app, state, result.generation, cx);
            }
            cx.stop_propagation();
            return;
        }
        let result = app.update(cx, |this, cx| {
            this.terminal_scroll_wheel_state_for_session(
                session_id.as_str(),
                raw_lines,
                event.position,
                event.modifiers,
                cx,
            )
        });

        if let Some(state) = result.visual_state {
            let repaint_needed = self.scroll_visual_state_needs_repaint(&state);
            let text_updated = self.apply_scroll_visual_state(state.clone());
            let needs_text_first_repaint =
                terminal_scroll_needs_text_first_repaint(&state, text_updated);
            if repaint_needed {
                cx.notify();
            }
            if needs_text_first_repaint {
                app.update(cx, |this, cx| {
                    this.notify_terminal_scroll_position_only(session_id.as_str(), cx);
                });
            }
        }
        if result.defer_repaint {
            Self::defer_surface_repaint(app, Some(session_id), cx);
        }
        if result.handled {
            cx.stop_propagation();
        }
    }

    fn can_handle_scroll_wheel_locally(&self) -> bool {
        !self.protocol_state.mouse_reporting
            && self.protocol_state.alternate_scroll_payload(1).is_none()
    }

    fn apply_local_scroll_wheel_visual_state(
        &mut self,
        raw_lines: f32,
    ) -> Option<TerminalSurfaceLocalScrollResult> {
        if raw_lines == 0.0 || !raw_lines.is_finite() {
            return None;
        }
        let (delta_lines, next_residual) = terminal_local_scroll_delta_lines_from_state(
            self.scroll_offset,
            self.scroll_residual_lines,
            self.scrollback_len,
            raw_lines,
        );
        if delta_lines == 0 && next_residual == self.scroll_residual_lines {
            return Some(TerminalSurfaceLocalScrollResult {
                generation: self.scroll_interaction_generation,
                visual_changed: false,
                needs_text_snapshot: false,
            });
        }

        let next_offset = if delta_lines > 0 {
            self.scroll_offset.saturating_add(delta_lines as usize)
        } else {
            self.scroll_offset.saturating_sub((-delta_lines) as usize)
        }
        .min(self.scrollback_len);
        let display_offset =
            terminal_display_offset_from_state(next_offset, next_residual, self.scrollback_len);
        let state = TerminalScrollVisualState {
            session_id: self.session_id.clone(),
            scroll_offset: next_offset,
            scroll_residual_lines: next_residual,
            display_offset,
            scrollback_len: self.scrollback_len,
            viewport_rows: self.viewport_rows,
            has_new_while_scrolled: if next_offset == 0 {
                false
            } else {
                self.has_new_while_scrolled
            },
            performance_overlay: self.performance_overlay,
            skipped_output_chars: self.skipped_output_chars,
        };
        let text_updated = self.apply_scroll_visual_state(state);
        self.scroll_interaction_generation = self.scroll_interaction_generation.saturating_add(1);
        Some(TerminalSurfaceLocalScrollResult {
            generation: self.scroll_interaction_generation,
            visual_changed: true,
            needs_text_snapshot: !text_updated
                && self.scroll_snapshot_pending
                && display_offset > 0,
        })
    }

    fn current_scroll_visual_state(&self) -> TerminalScrollVisualState {
        TerminalScrollVisualState {
            session_id: self.session_id.clone(),
            scroll_offset: self.scroll_offset,
            scroll_residual_lines: self.scroll_residual_lines,
            display_offset: self.display_offset,
            scrollback_len: self.scrollback_len,
            viewport_rows: self.viewport_rows,
            has_new_while_scrolled: self.has_new_while_scrolled,
            performance_overlay: self.performance_overlay,
            skipped_output_chars: self.skipped_output_chars,
        }
    }

    fn queue_local_scroll_app_sync(
        &mut self,
        app: Entity<NyaTermApp>,
        state: TerminalScrollVisualState,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        if !self.remember_pending_local_scroll_sync(state, generation) {
            return;
        }
        let surface = cx.entity();
        cx.spawn(async move |_, cx| {
            cx.background_executor()
                .timer(TERMINAL_SURFACE_LOCAL_SCROLL_SYNC_DELAY)
                .await;
            let _ = surface.update(cx, |surface, cx| {
                surface.flush_local_scroll_app_sync(app, cx);
            });
        })
        .detach();
    }

    fn remember_pending_local_scroll_sync(
        &mut self,
        state: TerminalScrollVisualState,
        generation: u64,
    ) -> bool {
        self.pending_local_scroll_sync =
            Some(TerminalSurfacePendingScrollSync { state, generation });
        if self.local_scroll_sync_armed {
            return false;
        }
        self.local_scroll_sync_armed = true;
        true
    }

    fn flush_local_scroll_app_sync(&mut self, app: Entity<NyaTermApp>, cx: &mut Context<Self>) {
        self.local_scroll_sync_armed = false;
        let Some(pending) = self.pending_local_scroll_sync.take() else {
            return;
        };
        let state = app.update(cx, |this, cx| {
            this.sync_terminal_local_scroll_visual_state_from_surface(pending.state, cx)
        });
        if self.scroll_interaction_generation != pending.generation {
            return;
        }
        if let Some(state) = state {
            let repaint_needed = self.scroll_visual_state_needs_repaint(&state);
            let text_updated = self.apply_scroll_visual_state(state.clone());
            let text_first_repaint_ready = terminal_surface_text_first_repaint_ready(
                &state,
                text_updated,
                app.update(cx, |this, _cx| {
                    this.terminal_scroll_text_cached_for_session(
                        state.session_id.as_str(),
                        state.display_offset,
                    )
                }),
            );
            if text_first_repaint_ready {
                let session_id = state.session_id.clone();
                // The app notification may read/update this surface. Wait until
                // the current entity update has released its GPUI lease.
                cx.defer(move |cx| {
                    app.update(cx, |this, cx| {
                        this.notify_terminal_scroll_position_only(session_id.as_str(), cx);
                    });
                });
            }
            if repaint_needed {
                cx.notify();
            }
        }
    }

    fn scrollbar_element(&self, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui::relative;
        let palette = self.palette;
        let session_id = self.session_id.clone();
        let is_active = self.is_active;
        let scroll_offset = self.scroll_offset;
        let max = self.scrollback_len;
        let viewport_rows = self.viewport_rows.max(1);
        let show = max > 0;
        let thumb_ratio = if max == 0 {
            1.0
        } else {
            let viewport = viewport_rows as f32;
            (viewport / (viewport + max as f32)).clamp(0.12, 1.0)
        };
        let travel = (1.0 - thumb_ratio).max(0.0);
        let thumb_top_ratio = if max == 0 {
            0.0
        } else {
            travel * (1.0 - (scroll_offset as f32 / max as f32).clamp(0.0, 1.0))
        };
        let app = self.app.clone();
        let track_id = format!("terminal-scrollbar-track-{session_id}");
        let thumb_id = format!("terminal-scrollbar-thumb-{session_id}");

        div()
            .id(SharedString::from(format!(
                "terminal-scrollbar-{session_id}"
            )))
            .w(px(TERMINAL_SCROLLBAR_COLUMN_WIDTH))
            .flex_none()
            .h_full()
            .py(px(2.))
            .pr(px(2.))
            .opacity(if show { 1.0 } else { 0.35 })
            .child(
                div()
                    .id(SharedString::from(track_id))
                    .relative()
                    .size_full()
                    .rounded_full()
                    .bg(rgb(palette.border))
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, {
                        let session_id = session_id.clone();
                        let app = app.clone();
                        cx.listener(move |_this, event: &gpui::MouseDownEvent, _window, cx| {
                            let Some(app) = app.clone() else {
                                return;
                            };
                            let repaint_session_id = app.update(cx, |this, cx| {
                                if !session_id.is_empty() {
                                    this.activate_workspace_pane(session_id.clone(), cx);
                                }
                                let drag_session_id =
                                    (!session_id.is_empty()).then_some(session_id.clone());
                                let mut repaint_session_id = this
                                    .begin_terminal_scrollbar_drag_state_only(
                                        drag_session_id.clone(),
                                    );
                                let Some(bounds) = (if session_id.is_empty() {
                                    this.terminal.layout.surface_bounds
                                } else {
                                    this.terminal
                                        .layout
                                        .session_surface_bounds
                                        .get(&session_id)
                                        .copied()
                                        .or(this.terminal.layout.surface_bounds)
                                }) else {
                                    return repaint_session_id;
                                };
                                let ratio = terminal_scroll_track_ratio(bounds, event.position.y);
                                repaint_session_id = this
                                    .set_terminal_scroll_from_track_ratio_for_session_state_only(
                                        drag_session_id.as_deref(),
                                        ratio,
                                    )
                                    .or(repaint_session_id);
                                repaint_session_id
                            });
                            Self::defer_surface_repaint(app, repaint_session_id, cx);
                            cx.stop_propagation();
                        })
                    })
                    .when(show, |this| {
                        this.child(
                            div()
                                .id(SharedString::from(thumb_id))
                                .absolute()
                                .left(px(1.))
                                .right(px(1.))
                                .top(relative(thumb_top_ratio))
                                .h(relative(thumb_ratio))
                                .min_h(px(18.))
                                .rounded_full()
                                .bg(rgb(if is_active {
                                    palette.link
                                } else {
                                    palette.text_muted
                                }))
                                .opacity(0.85),
                        )
                    }),
            )
    }
}

pub(in crate::features) fn terminal_visual_scroll_offset_px(
    target_offset: usize,
    displayed_offset: usize,
    residual_lines: f32,
    cell_height: f32,
) -> f32 {
    terminal_visual_scroll_line_delta(target_offset, displayed_offset, residual_lines)
        * cell_height.max(1.0)
}

fn terminal_visual_scroll_line_delta(
    target_offset: usize,
    displayed_offset: usize,
    residual_lines: f32,
) -> f32 {
    let line_delta = target_offset as isize - displayed_offset as isize;
    line_delta as f32 + residual_lines
}

fn terminal_surface_text_first_repaint_ready(
    state: &TerminalScrollVisualState,
    text_updated: bool,
    text_snapshot_cached: bool,
) -> bool {
    terminal_scroll_needs_text_first_repaint(state, text_updated) && text_snapshot_cached
}

fn terminal_retained_visual_scroll_line_bounds(
    viewport_anchor_row: usize,
    snapshot_rows: usize,
    viewport_rows: usize,
) -> (f32, f32) {
    let viewport_rows = viewport_rows.max(1);
    let older_rows = viewport_anchor_row.min(snapshot_rows);
    let newer_rows = snapshot_rows.saturating_sub(
        viewport_anchor_row
            .saturating_add(viewport_rows)
            .min(snapshot_rows),
    );
    (-(newer_rows as f32), older_rows as f32)
}

pub(in crate::features) fn terminal_effective_visual_scroll_offset_px(
    geometry: TerminalVisualScrollGeometry,
) -> f32 {
    let TerminalVisualScrollGeometry {
        snapshot_pending,
        target_offset,
        displayed_offset,
        residual_lines,
        viewport_anchor_row,
        snapshot_rows,
        viewport_rows,
        cell_height,
    } = geometry;
    if !snapshot_pending {
        return terminal_visual_scroll_offset_px(
            target_offset,
            displayed_offset,
            residual_lines,
            cell_height,
        );
    }
    let (min_lines, max_lines) = terminal_retained_visual_scroll_line_bounds(
        viewport_anchor_row,
        snapshot_rows,
        viewport_rows,
    );
    terminal_visual_scroll_line_delta(target_offset, displayed_offset, residual_lines)
        .clamp(min_lines, max_lines)
        * cell_height.max(1.0)
}

pub(in crate::features) fn terminal_snapshot_covers_display_offset(
    snapshot: &TerminalSnapshot,
    display_offset: usize,
    viewport_rows: usize,
    scrollback_len: usize,
) -> bool {
    let viewport_rows = viewport_rows.max(1);
    let real_total_rows = scrollback_len.saturating_add(viewport_rows);
    let Some((snapshot_start, snapshot_end)) = terminal_snapshot_absolute_window(snapshot) else {
        return false;
    };
    let desired_end = real_total_rows.saturating_sub(display_offset);
    let desired_start = desired_end.saturating_sub(viewport_rows);
    snapshot_start <= desired_start && desired_end <= snapshot_end
}

pub(in crate::features) fn terminal_snapshot_anchor_row_for_display_offset(
    snapshot: &TerminalSnapshot,
    display_offset: usize,
    viewport_rows: usize,
    scrollback_len: usize,
) -> usize {
    let viewport_rows = viewport_rows.max(1);
    let real_total_rows = scrollback_len.saturating_add(viewport_rows);
    let desired_end = real_total_rows.saturating_sub(display_offset);
    let desired_start = desired_end.saturating_sub(viewport_rows);
    terminal_snapshot_absolute_window(snapshot)
        .map(|(snapshot_start, _)| desired_start.saturating_sub(snapshot_start))
        .unwrap_or(0)
}

fn terminal_keyword_highlight_visible_rows(
    snapshot: &TerminalSnapshot,
    display_offset: usize,
    viewport_rows: usize,
    scrollback_len: usize,
) -> Range<usize> {
    let anchor = terminal_snapshot_anchor_row_for_display_offset(
        snapshot,
        display_offset,
        viewport_rows,
        scrollback_len,
    );
    let start = anchor.saturating_sub(1).min(snapshot.row_count());
    let end = anchor
        .saturating_add(viewport_rows.max(1))
        .saturating_add(1)
        .min(snapshot.row_count())
        .max(start);
    start..end
}

fn terminal_keyword_highlight_prefetch_rows(
    snapshot: &TerminalSnapshot,
    display_offset: usize,
    viewport_rows: usize,
    scrollback_len: usize,
) -> Range<usize> {
    let viewport_rows = viewport_rows.max(1);
    let anchor = terminal_snapshot_anchor_row_for_display_offset(
        snapshot,
        display_offset,
        viewport_rows,
        scrollback_len,
    );
    let extra_rows = viewport_rows.saturating_mul(TERMINAL_KEYWORD_HIGHLIGHT_PREFETCH_VIEWPORTS);
    let start = anchor
        .saturating_sub(extra_rows)
        .saturating_sub(1)
        .min(snapshot.row_count());
    let end = anchor
        .saturating_add(viewport_rows)
        .saturating_add(extra_rows)
        .saturating_add(1)
        .min(snapshot.row_count())
        .max(start);
    start..end
}

fn hidden_terminal_cursor_snapshot() -> nyaterm_terminal::CursorSnapshot {
    nyaterm_terminal::CursorSnapshot {
        row: usize::MAX,
        col: 0,
        shape: nyaterm_terminal::CursorShape::Hidden,
        visible: false,
        blinking: false,
    }
}

fn terminal_surface_synthesized_window_extra_rows(viewport_rows: usize) -> usize {
    viewport_rows.max(1).saturating_mul(2).clamp(
        TERMINAL_SURFACE_SYNTHESIZED_WINDOW_MIN_EXTRA_ROWS,
        TERMINAL_SURFACE_SYNTHESIZED_WINDOW_MAX_EXTRA_ROWS,
    )
}

fn terminal_snapshot_row_for_absolute_row(
    snapshot: &TerminalSnapshot,
    absolute_row: usize,
) -> Option<usize> {
    let (start, end) = terminal_snapshot_absolute_window(snapshot)?;
    if absolute_row < start || absolute_row >= end {
        return None;
    }
    Some(absolute_row - start)
}

fn terminal_snapshot_absolute_window(snapshot: &TerminalSnapshot) -> Option<(usize, usize)> {
    if snapshot.row_count() == 0 {
        return None;
    }
    let end = snapshot.total_rows.saturating_sub(snapshot.display_offset);
    let start = end.saturating_sub(snapshot.row_count());
    Some((start, end))
}

fn terminal_surface_visible_rows_for_viewport(
    viewport_rows: usize,
    snapshot_rows: usize,
    visual_y_offset: f32,
    cell_height: f32,
) -> Range<usize> {
    if snapshot_rows == 0 {
        return 0..0;
    }
    let viewport_height = viewport_rows.max(1) as f32 * cell_height.max(1.0);
    let cell_height = cell_height.max(1.0);
    let overscan_rows = 1usize;
    let visible_start = ((-visual_y_offset) / cell_height).floor().max(0.0) as usize;
    let visible_end = ((viewport_height - visual_y_offset) / cell_height)
        .ceil()
        .max(0.0) as usize;
    let start = visible_start
        .saturating_sub(overscan_rows)
        .min(snapshot_rows);
    let end = visible_end.saturating_add(overscan_rows).min(snapshot_rows);
    if end < start {
        return start..start;
    }
    start..end
}

fn terminal_selection_visual_row_range(
    selection: Option<TerminalSelection>,
    line_count: usize,
) -> Option<Range<usize>> {
    let selection = selection?;
    if selection.all_buffer {
        return Some(0..line_count);
    }
    if selection.is_empty() {
        return None;
    }
    let (start, end) = selection.ordered();
    let start_row = selection
        .viewport_anchor_row
        .saturating_add(start.row)
        .min(line_count);
    let end_row = selection
        .viewport_anchor_row
        .saturating_add(end.row)
        .saturating_add(1)
        .min(line_count);
    (start_row < end_row).then_some(start_row..end_row)
}

fn terminal_selection_visual_row_union(
    previous: Option<Range<usize>>,
    next: Option<Range<usize>>,
) -> Option<Range<usize>> {
    match (previous, next) {
        (Some(previous), Some(next)) => {
            Some(previous.start.min(next.start)..previous.end.max(next.end))
        }
        (Some(previous), None) => Some(previous),
        (None, Some(next)) => Some(next),
        (None, None) => None,
    }
}

fn terminal_selection_visual_row_range_from_decorations(
    decorations: &[TerminalLineDecorations],
) -> Option<Range<usize>> {
    let start = decorations
        .iter()
        .position(|decoration| decoration.selection_cols.is_some())?;
    let end = decorations
        .iter()
        .rposition(|decoration| decoration.selection_cols.is_some())?
        .saturating_add(1);
    Some(start..end)
}

fn terminal_surface_fractional_prefetch_offset(
    scroll_offset: usize,
    residual_lines: f32,
    scrollback_len: usize,
) -> Option<usize> {
    if scrollback_len == 0 || residual_lines == 0.0 || !residual_lines.is_finite() {
        return None;
    }
    if residual_lines > 0.0 {
        return scroll_offset
            .checked_add(1)
            .filter(|offset| *offset <= scrollback_len && *offset > 0);
    }
    scroll_offset.checked_sub(1).filter(|offset| *offset > 0)
}

impl Render for TerminalSurface {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        TERMINAL_SURFACE_PAINT_COUNT.fetch_add(1, Ordering::Relaxed);
        let palette = self.palette;
        let cell_w = self.cell_width.max(1.0);
        let cell_h = self.cell_height.max(1.0);
        let snapshot = self
            .snapshot
            .clone()
            .unwrap_or_else(|| Arc::new(TerminalScreen::default().viewport_snapshot(0)));
        self.maybe_log_scroll_snapshot_pending(snapshot.as_ref());
        let viewport_anchor_row = terminal_snapshot_anchor_row_for_display_offset(
            snapshot.as_ref(),
            self.display_offset,
            self.viewport_rows,
            self.scrollback_len,
        );
        let visual_y_offset =
            terminal_effective_visual_scroll_offset_px(TerminalVisualScrollGeometry {
                snapshot_pending: self.scroll_snapshot_pending,
                target_offset: self.scroll_offset,
                displayed_offset: self.display_offset,
                residual_lines: self.scroll_residual_lines,
                viewport_anchor_row,
                snapshot_rows: snapshot.row_count(),
                viewport_rows: self.viewport_rows,
                cell_height: cell_h,
            }) - viewport_anchor_row as f32 * cell_h;
        let line_count = snapshot.row_count();
        let visible_gutter_rows = terminal_surface_visible_rows_for_viewport(
            self.viewport_rows,
            line_count,
            visual_y_offset,
            cell_h,
        );
        let gutter_enabled = self.show_line_numbers || self.show_timestamps;
        let performance_overlay = self.performance_overlay;
        let skipped_output_chars = self.skipped_output_chars;
        let visual_bell = self.visual_bell && self.is_active;
        let mut surface_font = font(SharedString::from(self.font_family.clone()));
        surface_font.fallbacks = self.font_fallbacks.clone();
        let app = self.app.clone();
        let session_id = self.session_id.clone();
        let is_active = self.is_active;
        let mut grid = NyaTerminalElement::new(
            snapshot.clone(),
            empty_terminal_keyword_rules(),
            self.decorations.clone(),
            self.show_cursor,
            self.cursor_style.clone(),
            cell_w,
            cell_h,
            palette,
            self.font_family.clone(),
            self.font_size,
            self.normal_weight,
            self.bold_weight,
        );
        grid = grid
            .with_font_fallbacks(self.font_fallbacks.clone())
            .with_layout_cache(self.layout_cache.clone())
            .with_layout_rows(self.viewport_rows)
            .with_fill_height(true)
            .with_visual_y_offset(visual_y_offset);
        if let Some(highlights) = self.keyword_highlights.clone() {
            grid = grid.with_keyword_highlights(highlights);
        }

        let gutter = if gutter_enabled {
            let gutter_metrics = terminal_gutter_metrics(
                cell_w,
                self.show_timestamps,
                self.show_timestamp_ms,
                self.show_line_numbers,
                terminal_line_number_digits(snapshot.as_ref()),
            );
            let line_number_digits = terminal_line_number_digits(snapshot.as_ref());
            let ts_w = gutter_metrics.timestamp_width;
            let ln_w = gutter_metrics.line_number_width;
            let gutter_viewport_width = (gutter_metrics.total_width() - 10.0).max(1.0);
            let abs_start = snapshot
                .total_rows
                .saturating_sub(snapshot.display_offset)
                .saturating_sub(snapshot.row_count());
            let gutter_y_offset = visual_y_offset + visible_gutter_rows.start as f32 * cell_h;
            let mut gutter_rows = div()
                .absolute()
                .left_0()
                .top(px(gutter_y_offset))
                .flex()
                .flex_col();
            for line_index in visible_gutter_rows {
                let snapshot_row = snapshot.row(line_index);
                let is_wrapped = snapshot_row.is_some_and(|row| row.wrapped);
                let has_rendered_row =
                    snapshot.cursor.row == usize::MAX || line_index <= snapshot.cursor.row;
                let ts_label = if self.show_timestamps && has_rendered_row && !is_wrapped {
                    snapshot_row
                        .and_then(|row| row.timestamp_ms)
                        .map(|ms| format_terminal_line_timestamp_ms(ms, self.show_timestamp_ms))
                        .unwrap_or_else(|| {
                            if self.show_timestamp_ms {
                                "             ".to_string()
                            } else {
                                "          ".to_string()
                            }
                        })
                } else {
                    String::new()
                };
                let line_label = if self.show_line_numbers && has_rendered_row && !is_wrapped {
                    format!(
                        "{:>width$}",
                        abs_start + line_index + 1,
                        width = line_number_digits,
                    )
                } else {
                    String::new()
                };
                gutter_rows = gutter_rows.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .min_h(px(cell_h))
                        .gap(px(gutter_metrics.gap_width))
                        .flex_none()
                        .pr(px(8.))
                        .text_color(rgb(palette.text_dimmed))
                        .font(surface_font.clone())
                        .text_size(px(self.font_size))
                        .when(self.show_timestamps, |this| {
                            this.child(div().w(px(ts_w)).flex_none().child(ts_label))
                        })
                        .when(self.show_line_numbers, |this| {
                            this.child(div().w(px(ln_w)).flex_none().child(line_label))
                        }),
                );
            }
            Some(
                div()
                    .relative()
                    .h_full()
                    .min_h_0()
                    .w(px(gutter_viewport_width))
                    .flex_none()
                    .mr(px(10.))
                    .overflow_hidden()
                    .border_r_1()
                    .border_color(rgb(palette.border))
                    .child(gutter_rows),
            )
        } else {
            None
        };

        let body = if let Some(gutter) = gutter {
            div()
                .flex()
                .flex_row()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .child(gutter)
                .child(div().flex_1().min_w_0().min_h_0().child(grid))
        } else {
            div()
                .flex()
                .flex_row()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .child(div().flex_1().min_w_0().min_h_0().child(grid))
        };

        let scrollbar = self.scrollbar_element(cx).into_any_element();

        div()
            .id(SharedString::from(format!(
                "terminal-surface-{}",
                self.session_id
            )))
            .size_full()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_row()
            .relative()
            .bg(if self.transparent_background {
                rgba(palette.terminal_bg << 8)
            } else {
                rgb(palette.terminal_bg)
            })
            .text_color(rgb(palette.terminal_fg))
            .font(surface_font)
            .text_size(px(self.font_size))
            .when(!self.protocol_state.mouse_reporting, |this| {
                this.cursor_text()
            })
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                this.handle_scroll_wheel(event, cx);
            }))
            .when(visual_bell, |this| {
                this.border_2().border_color(rgb(palette.warning))
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .relative()
                    .overflow_hidden()
                    .child(body)
                    .when_some(app, |this, app| {
                        this.child(terminal_bounds_tracker(
                            app,
                            Some(session_id.clone()),
                            is_active,
                        ))
                    })
                    .when_some(performance_overlay, |this, overlay| {
                        this.child(
                            div()
                                .absolute()
                                .left_2()
                                .top_2()
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .bg(rgb(palette.surface_elevated))
                                .border_1()
                                .border_color(rgb(palette.border))
                                .text_xs()
                                .text_color(rgb(palette.text_muted))
                                .child(match overlay {
                                    TerminalPerformanceOverlay::Overloaded => {
                                        format!("protecting output… skipped={skipped_output_chars}")
                                    }
                                    TerminalPerformanceOverlay::Recovered => {
                                        "render recovered".to_string()
                                    }
                                }),
                        )
                    }),
            )
            .child(scrollbar)
    }
}

#[cfg(test)]
mod tests;
