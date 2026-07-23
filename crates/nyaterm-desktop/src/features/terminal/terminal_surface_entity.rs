use super::*;
use crate::features::terminal_runtime::{
    TerminalScrollVisualState, terminal_display_offset_from_state,
    terminal_local_scroll_delta_lines_from_state, terminal_scroll_needs_text_first_repaint,
    terminal_scroll_track_ratio, terminal_visual_scroll_active_for_state,
};
use crate::features::terminal_selection_runtime::{
    terminal_gutter_metrics, terminal_line_number_digits,
};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Process-wide surface paint counter (Phase 0 isolation diagnostics).
pub(in crate::features) static TERMINAL_SURFACE_PAINT_COUNT: AtomicU64 = AtomicU64::new(0);
pub(in crate::features) static FULL_SHELL_PAINT_COUNT: AtomicU64 = AtomicU64::new(0);
const TERMINAL_SURFACE_RETAINED_SNAPSHOT_LIMIT: usize = 12;
const TERMINAL_SURFACE_RETAINED_ROW_LIMIT: usize = 4096;
const TERMINAL_SURFACE_SCROLL_PENDING_WARN_AFTER: Duration = Duration::from_millis(48);
const TERMINAL_SURFACE_SCROLL_PENDING_WARN_INTERVAL: Duration = Duration::from_millis(500);
const TERMINAL_SURFACE_LOCAL_SCROLL_SYNC_DELAY: Duration = Duration::from_millis(16);

#[derive(Clone)]
struct TerminalSurfaceRetainedRow {
    cols: usize,
    cells: Vec<nyaterm_terminal::RenderCell>,
    line: String,
    styled_line: Vec<nyaterm_terminal::StyledSpan>,
    line_signature: u64,
    line_timestamp_ms: Option<u64>,
    line_wrapped: bool,
    hyperlink_line: Vec<nyaterm_terminal::HyperlinkSpan>,
    command_mark: Option<nyaterm_terminal::ShellCommandMark>,
}

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
pub(in crate::features) struct TerminalSurfaceHitTestScrollGeometry {
    pub(in crate::features) snapshot_pending: bool,
    pub(in crate::features) display_offset: usize,
    pub(in crate::features) snapshot_rows: usize,
    pub(in crate::features) viewport_anchor_row: usize,
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
    retained_rows: BTreeMap<usize, TerminalSurfaceRetainedRow>,
    keyword_rules: Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>>,
    keyword_highlights: Option<Arc<TerminalKeywordHighlightSnapshot>>,
    keyword_highlight_generation: u64,
    keyword_highlight_task: Option<gpui::Task<()>>,
    keyword_highlighter_rules: Option<Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>>>,
    keyword_highlighter: Option<Arc<TerminalKeywordHighlighter>>,
    decorations: Arc<[TerminalLineDecorations]>,
    selection_visual: Option<TerminalSelection>,
    selection_visual_row_range: Option<Range<usize>>,
    palette: ThemePalette,
    font_family: String,
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
            keyword_highlighter_rules: None,
            keyword_highlighter: None,
            decorations: Arc::from(Vec::<TerminalLineDecorations>::new()),
            selection_visual: None,
            selection_visual_row_range: None,
            palette: crate::theme::theme_palette("github-dark"),
            font_family: "monospace".to_string(),
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

    pub(in crate::features) fn session_id(&self) -> &str {
        &self.session_id
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
            snapshot_rows: snapshot.rows,
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

    pub(in crate::features) fn set_app(&mut self, app: Entity<NyaTermApp>) {
        self.app = Some(app);
    }

    pub(in crate::features) fn take_layout_cache(&self) -> Arc<Mutex<NyaTerminalLayoutCache>> {
        self.layout_cache.clone()
    }

    pub(in crate::features) fn apply_frame_snapshot(
        &mut self,
        snapshot: Arc<TerminalSnapshot>,
        scroll_offset: usize,
        scroll_residual_lines: f32,
        display_offset: usize,
        scrollback_len: usize,
        viewport_rows: usize,
        has_new_while_scrolled: bool,
        performance_overlay: Option<TerminalPerformanceOverlay>,
        skipped_output_chars: u64,
        has_action_link_decorations: bool,
        show_cursor: bool,
        cursor_style: impl Into<String>,
    ) -> bool {
        // Decorations/keywords are pushed separately so frame notifies can keep
        // selection/search highlights until the next decoration rebuild.
        let retained_rows_should_reset =
            self.retained_rows_should_reset(snapshot.as_ref(), scrollback_len, viewport_rows);
        let pending_local_scroll_state = (!retained_rows_should_reset)
            .then(|| {
                self.pending_local_scroll_state_reanchored_for_frame(
                    scroll_offset,
                    scroll_residual_lines,
                    display_offset,
                    scrollback_len,
                    viewport_rows,
                    has_new_while_scrolled,
                    performance_overlay,
                    skipped_output_chars,
                )
            })
            .flatten();
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
        self.viewport_rows = viewport_rows.max(1);
        self.has_new_while_scrolled = has_new_while_scrolled;
        self.has_action_link_decorations = has_action_link_decorations;
        self.performance_overlay = performance_overlay;
        self.skipped_output_chars = skipped_output_chars;
        self.show_cursor = show_cursor
            && !terminal_visual_scroll_active_for_state(scroll_offset, scroll_residual_lines);
        self.cursor_style = cursor_style.into();
        self.prune_pending_scroll_snapshot_offsets();
        self.revision = self.revision.saturating_add(1);
        true
    }

    fn pending_local_scroll_state_reanchored_for_frame(
        &self,
        frame_scroll_offset: usize,
        frame_scroll_residual_lines: f32,
        frame_display_offset: usize,
        frame_scrollback_len: usize,
        frame_viewport_rows: usize,
        has_new_while_scrolled: bool,
        performance_overlay: Option<TerminalPerformanceOverlay>,
        skipped_output_chars: u64,
    ) -> Option<TerminalScrollVisualState> {
        let pending = self.pending_local_scroll_sync.as_ref()?;
        let mut state = pending.state.clone();
        if state.scroll_offset == frame_scroll_offset
            && state.display_offset == frame_display_offset
            && (state.scroll_residual_lines - frame_scroll_residual_lines).abs()
                < f32::EPSILON * 8.0
        {
            return None;
        }
        if state.scroll_offset == 0 {
            state.scroll_residual_lines = 0.0;
        } else {
            state.scroll_offset = state
                .scroll_offset
                .saturating_add(frame_scrollback_len.saturating_sub(state.scrollback_len))
                .min(frame_scrollback_len);
            if state.scroll_offset >= frame_scrollback_len && state.scroll_residual_lines > 0.0 {
                state.scroll_residual_lines = 0.0;
            }
        }
        state.scrollback_len = frame_scrollback_len;
        state.viewport_rows = frame_viewport_rows.max(1);
        state.display_offset = terminal_display_offset_from_state(
            state.scroll_offset,
            state.scroll_residual_lines,
            state.scrollback_len,
        );
        state.has_new_while_scrolled = has_new_while_scrolled
            || terminal_visual_scroll_active_for_state(
                state.scroll_offset,
                state.scroll_residual_lines,
            );
        state.performance_overlay = performance_overlay;
        state.skipped_output_chars = skipped_output_chars;
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
        scroll_offset: usize,
        scroll_residual_lines: f32,
        display_offset: usize,
        scrollback_len: usize,
        viewport_rows: usize,
        has_new_while_scrolled: bool,
        performance_overlay: Option<TerminalPerformanceOverlay>,
        skipped_output_chars: u64,
    ) {
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
    }

    pub(in crate::features) fn update_scroll_position_without_snapshot(
        &mut self,
        scroll_offset: usize,
        scroll_residual_lines: f32,
        display_offset: usize,
        scrollback_len: usize,
        viewport_rows: usize,
        has_new_while_scrolled: bool,
        performance_overlay: Option<TerminalPerformanceOverlay>,
        skipped_output_chars: u64,
    ) {
        let previous_snapshot = self.snapshot.clone();
        self.promote_snapshot_covering_display_offset(
            display_offset,
            viewport_rows,
            scrollback_len,
        );
        let snapshot_changed = match (previous_snapshot.as_ref(), self.snapshot.as_ref()) {
            (Some(previous), Some(current)) => !Arc::ptr_eq(previous, current),
            (None, None) => false,
            _ => true,
        };
        if snapshot_changed {
            self.decorations = Arc::from(Vec::<TerminalLineDecorations>::new());
            self.selection_visual = None;
            self.selection_visual_row_range = None;
            self.has_action_link_decorations = false;
            self.show_cursor = false;
        }
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
            snapshot_rows = snapshot.rows,
            snapshot_total_rows = snapshot.total_rows,
            retained_snapshots = self.retained_snapshots.len(),
            retained_rows = self.retained_rows.len(),
            pending_ms = pending_for.as_millis(),
            "terminal surface retained text while waiting for target scroll snapshot"
        );
    }

    fn remember_retained_snapshot(&mut self, snapshot: Arc<TerminalSnapshot>) {
        if snapshot.rows == 0 {
            return;
        }
        self.remember_retained_snapshot_rows(snapshot.as_ref());
        self.retained_snapshots.retain(|retained| {
            !(retained.display_offset == snapshot.display_offset
                && retained.total_rows == snapshot.total_rows
                && retained.rows == snapshot.rows)
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

    fn remember_retained_snapshot_rows(&mut self, snapshot: &TerminalSnapshot) {
        let Some((start, _)) = terminal_snapshot_absolute_window(snapshot) else {
            return;
        };
        for row in 0..snapshot.rows {
            let Some(abs_row) = start.checked_add(row) else {
                continue;
            };
            let Some(retained_row) = terminal_surface_retained_row_from_snapshot(snapshot, row)
            else {
                continue;
            };
            self.retained_rows.insert(abs_row, retained_row);
        }
        while self.retained_rows.len() > TERMINAL_SURFACE_RETAINED_ROW_LIMIT {
            let Some(drop_key) = self.retained_rows.keys().next().copied() else {
                break;
            };
            self.retained_rows.remove(&drop_key);
        }
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
        let cols = first_row.cols;
        if cols == 0 {
            return false;
        }
        (desired_start..desired_end).all(|abs_row| {
            self.retained_rows
                .get(&abs_row)
                .is_some_and(|row| row.cols == cols && row.cells.len() == cols)
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
                        let cell_start = row.saturating_mul(cols);
                        let cell_end = cell_start.saturating_add(cols);
                        snapshot.cells.get(cell_start..cell_end).is_some()
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
        if let Some(snapshot) = self.synthesize_snapshot_from_retained_rows(
            display_offset,
            viewport_rows,
            scrollback_len,
            desired_start,
            desired_end,
        ) {
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

        let mut cells = Vec::with_capacity(viewport_rows.saturating_mul(cols));
        let mut lines = Vec::with_capacity(viewport_rows);
        let mut styled_lines = Vec::with_capacity(viewport_rows);
        let mut line_signatures = Vec::with_capacity(viewport_rows);
        let mut line_timestamps_ms = Vec::with_capacity(viewport_rows);
        let mut line_wrapped = Vec::with_capacity(viewport_rows);
        let mut hyperlink_lines = Vec::with_capacity(viewport_rows);
        let mut command_marks = Vec::with_capacity(viewport_rows);

        for abs_row in desired_start..desired_end {
            let (snapshot, row) = sources
                .iter()
                .filter_map(|snapshot| {
                    terminal_snapshot_row_for_absolute_row(snapshot, abs_row)
                        .map(|row| (*snapshot, row))
                })
                .min_by_key(|(snapshot, _)| snapshot.display_offset.abs_diff(display_offset))?;
            let cell_start = row.checked_mul(cols)?;
            let cell_end = cell_start.checked_add(cols)?;
            cells.extend_from_slice(snapshot.cells.get(cell_start..cell_end)?);
            lines.push(snapshot.lines.get(row).cloned().unwrap_or_default());
            styled_lines.push(snapshot.styled_lines.get(row).cloned().unwrap_or_default());
            line_signatures.push(*snapshot.line_signatures.get(row).unwrap_or(&0));
            line_timestamps_ms.push(*snapshot.line_timestamps_ms.get(row).unwrap_or(&None));
            line_wrapped.push(*snapshot.line_wrapped.get(row).unwrap_or(&false));
            hyperlink_lines.push(
                snapshot
                    .hyperlink_lines
                    .get(row)
                    .cloned()
                    .unwrap_or_default(),
            );
            command_marks.push(*snapshot.command_marks.get(row).unwrap_or(&None));
        }

        Some(Arc::new(TerminalSnapshot {
            cols,
            viewport_rows,
            rows: viewport_rows,
            cells,
            cursor: hidden_terminal_cursor_snapshot(),
            selection: None,
            lines,
            styled_lines,
            line_signatures,
            line_timestamps_ms,
            line_wrapped,
            hyperlink_lines,
            cursor_row: usize::MAX,
            cursor_col: 0,
            scrollback_len,
            total_rows: real_total_rows,
            display_offset,
            images: Vec::new(),
            command_marks,
        }))
    }

    fn synthesize_snapshot_from_retained_rows(
        &self,
        display_offset: usize,
        viewport_rows: usize,
        scrollback_len: usize,
        desired_start: usize,
        desired_end: usize,
    ) -> Option<Arc<TerminalSnapshot>> {
        let first_row = self.retained_rows.get(&desired_start)?;
        let cols = first_row.cols;
        if cols == 0 {
            return None;
        }
        let real_total_rows = scrollback_len.saturating_add(viewport_rows);
        let mut cells = Vec::with_capacity(viewport_rows.saturating_mul(cols));
        let mut lines = Vec::with_capacity(viewport_rows);
        let mut styled_lines = Vec::with_capacity(viewport_rows);
        let mut line_signatures = Vec::with_capacity(viewport_rows);
        let mut line_timestamps_ms = Vec::with_capacity(viewport_rows);
        let mut line_wrapped = Vec::with_capacity(viewport_rows);
        let mut hyperlink_lines = Vec::with_capacity(viewport_rows);
        let mut command_marks = Vec::with_capacity(viewport_rows);

        for abs_row in desired_start..desired_end {
            let row = self.retained_rows.get(&abs_row)?;
            if row.cols != cols || row.cells.len() != cols {
                return None;
            }
            cells.extend_from_slice(&row.cells);
            lines.push(row.line.clone());
            styled_lines.push(row.styled_line.clone());
            line_signatures.push(row.line_signature);
            line_timestamps_ms.push(row.line_timestamp_ms);
            line_wrapped.push(row.line_wrapped);
            hyperlink_lines.push(row.hyperlink_line.clone());
            command_marks.push(row.command_mark);
        }

        Some(Arc::new(TerminalSnapshot {
            cols,
            viewport_rows,
            rows: viewport_rows,
            cells,
            cursor: hidden_terminal_cursor_snapshot(),
            selection: None,
            lines,
            styled_lines,
            line_signatures,
            line_timestamps_ms,
            line_wrapped,
            hyperlink_lines,
            cursor_row: usize::MAX,
            cursor_col: 0,
            scrollback_len,
            total_rows: real_total_rows,
            display_offset,
            images: Vec::new(),
            command_marks,
        }))
    }

    pub(in crate::features) fn set_paint_chrome(
        &mut self,
        palette: ThemePalette,
        font_family: String,
        font_size: f32,
        normal_weight: f32,
        bold_weight: f32,
        cell_width: f32,
        cell_height: f32,
        show_line_numbers: bool,
        show_timestamps: bool,
        show_timestamp_ms: bool,
        is_active: bool,
        visual_bell: bool,
    ) {
        self.palette = palette;
        self.font_family = font_family;
        self.font_size = font_size;
        self.normal_weight = normal_weight;
        self.bold_weight = bold_weight;
        self.cell_width = cell_width.max(1.0);
        self.cell_height = cell_height.max(1.0);
        self.show_line_numbers = show_line_numbers;
        self.show_timestamps = show_timestamps;
        self.show_timestamp_ms = show_timestamp_ms;
        self.is_active = is_active;
        self.visual_bell = visual_bell;
    }

    pub(in crate::features) fn set_background_transparent(&mut self, transparent: bool) {
        self.transparent_background = transparent;
    }

    pub(in crate::features) fn set_cursor_blink_visible(&mut self, show_cursor: bool) {
        self.show_cursor = show_cursor && !self.visual_scroll_active();
    }

    pub(in crate::features) fn set_protocol_state(
        &mut self,
        protocol_state: TerminalProtocolState,
    ) {
        self.protocol_state = protocol_state;
    }

    pub(in crate::features) fn set_visual_bell(&mut self, visual_bell: bool) {
        self.visual_bell = visual_bell;
    }

    pub(in crate::features) fn set_layout_cache(
        &mut self,
        layout_cache: Arc<Mutex<NyaTerminalLayoutCache>>,
    ) {
        self.layout_cache = layout_cache;
    }

    pub(in crate::features) fn set_decorations_and_keywords(
        &mut self,
        decorations: impl Into<Arc<[TerminalLineDecorations]>>,
        keyword_rules: Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>>,
        show_cursor: bool,
        cursor_style: impl Into<String>,
    ) {
        let decorations = decorations.into();
        self.selection_visual = None;
        self.selection_visual_row_range =
            terminal_selection_visual_row_range_from_decorations(&decorations);
        self.decorations = decorations;
        self.keyword_rules = keyword_rules;
        self.show_cursor = show_cursor && !self.visual_scroll_active();
        self.cursor_style = cursor_style.into();
    }

    pub(in crate::features) fn schedule_keyword_highlights(
        &mut self,
        clear_if_empty: bool,
        cx: &mut Context<Self>,
    ) {
        self.keyword_highlight_generation = self.keyword_highlight_generation.saturating_add(1);
        let generation = self.keyword_highlight_generation;
        self.keyword_highlight_task = None;
        // Keep the last published snapshot drawable while the replacement is parsed in the
        // background, matching the editor's stale-until-reparsed behavior.
        if self.keyword_rules.is_empty() {
            if clear_if_empty {
                self.keyword_highlights = None;
            }
            return;
        }
        let Some(snapshot) = self.snapshot.clone() else {
            return;
        };
        let rules = self.keyword_rules.clone();
        let highlighter = self
            .keyword_highlighter_rules
            .as_ref()
            .filter(|cached_rules| Arc::ptr_eq(cached_rules, &rules))
            .and(self.keyword_highlighter.clone());
        let palette = self.palette;
        self.keyword_highlight_task = Some(cx.spawn(async move |this, cx| {
            let (rules, highlighter, highlights) = cx
                .background_spawn(async move {
                    let highlighter = highlighter.unwrap_or_else(|| {
                        Arc::new(compile_terminal_keyword_highlighter(rules.as_ref()))
                    });
                    let highlights = precompute_terminal_keyword_highlights(
                        snapshot.as_ref(),
                        highlighter.as_ref(),
                        palette,
                    );
                    (rules, highlighter, highlights)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.keyword_highlight_generation != generation {
                    return;
                }
                this.keyword_highlighter_rules = Some(rules);
                this.keyword_highlighter = Some(highlighter);
                this.keyword_highlights = Some(Arc::new(highlights));
                cx.notify();
            });
        }));
    }

    pub(in crate::features) fn set_selection_visual(
        &mut self,
        selection: Option<TerminalSelection>,
    ) -> bool {
        let Some(snapshot) = self.snapshot.as_ref() else {
            return false;
        };
        let line_count = snapshot.lines.len();
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

    fn defer_surface_repaint(
        app: Entity<NyaTermApp>,
        session_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        cx.defer(move |cx| {
            let _ = app.update(cx, |this, cx| {
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
            let _ = app.update(cx, |this, _cx| {
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
            self.update_scroll_position_without_snapshot(
                state.scroll_offset,
                state.scroll_residual_lines,
                state.display_offset,
                state.scrollback_len,
                state.viewport_rows,
                state.has_new_while_scrolled,
                state.performance_overlay,
                state.skipped_output_chars,
            );
            true
        } else {
            self.update_scroll_chrome_without_snapshot(
                state.scroll_offset,
                state.scroll_residual_lines,
                state.display_offset,
                state.scrollback_len,
                state.viewport_rows,
                state.has_new_while_scrolled,
                state.performance_overlay,
                state.skipped_output_chars,
            );
            false
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
        if self.can_handle_scroll_wheel_locally() {
            if let Some(result) = self.apply_local_scroll_wheel_visual_state(raw_lines) {
                if result.visual_changed {
                    let state = self.current_scroll_visual_state();
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
            let text_updated = self.apply_scroll_visual_state(state.clone());
            let needs_text_first_repaint =
                terminal_scroll_needs_text_first_repaint(&state, text_updated);
            cx.notify();
            if needs_text_first_repaint {
                let _ = app.update(cx, |this, cx| {
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
            Timer::after(TERMINAL_SURFACE_LOCAL_SCROLL_SYNC_DELAY).await;
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
                    let _ = app.update(cx, |this, cx| {
                        this.notify_terminal_scroll_position_only(session_id.as_str(), cx);
                    });
                });
            }
            cx.notify();
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
            .w(px(10.))
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
                                    this.terminal_surface_bounds
                                } else {
                                    this.terminal_session_surface_bounds
                                        .get(&session_id)
                                        .copied()
                                        .or(this.terminal_surface_bounds)
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

    fn scroll_to_live_fab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.palette;
        let session_id = self.session_id.clone();
        let has_new = self.has_new_while_scrolled;
        let app = self.app.clone();
        div()
            .id(SharedString::from(format!(
                "terminal-scroll-bottom-{session_id}"
            )))
            .absolute()
            .right(px(22.))
            .bottom(px(14.))
            .h(px(30.))
            .px_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface_elevated))
            .shadow_md()
            .flex()
            .items_center()
            .gap_1()
            .cursor_pointer()
            .hover(move |style| style.bg(rgb(palette.hover)))
            .on_click(cx.listener(move |this, _, _, cx| {
                let Some(app) = app.clone() else {
                    return;
                };
                let repaint_session_id = app.update(cx, |this, cx| {
                    let repaint_session_id = this.scroll_terminal_to_bottom_state_only();
                    this.terminal_status = "scrolled to live output".to_string();
                    // Status bar is shell chrome; user-triggered, not hot path.
                    cx.notify();
                    repaint_session_id
                });
                this.scroll_offset = 0;
                this.scroll_residual_lines = 0.0;
                this.display_offset = 0;
                this.scroll_snapshot_pending = false;
                this.scroll_snapshot_pending_since = None;
                this.pending_scroll_snapshot_offsets.clear();
                this.has_new_while_scrolled = false;
                cx.notify();
                Self::defer_surface_repaint(app, repaint_session_id, cx);
            }))
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(if has_new {
                        palette.warning
                    } else {
                        palette.link
                    }))
                    .child(if has_new { "↓ New" } else { "↓ Live" }),
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
    snapshot_pending: bool,
    target_offset: usize,
    displayed_offset: usize,
    residual_lines: f32,
    viewport_anchor_row: usize,
    snapshot_rows: usize,
    viewport_rows: usize,
    cell_height: f32,
) -> f32 {
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

fn hidden_terminal_cursor_snapshot() -> nyaterm_terminal::CursorSnapshot {
    nyaterm_terminal::CursorSnapshot {
        row: usize::MAX,
        col: 0,
        shape: nyaterm_terminal::CursorShape::Hidden,
        visible: false,
        blinking: false,
    }
}

fn terminal_surface_retained_row_from_snapshot(
    snapshot: &TerminalSnapshot,
    row: usize,
) -> Option<TerminalSurfaceRetainedRow> {
    if snapshot.cols == 0 || row >= snapshot.rows {
        return None;
    }
    let cell_start = row.checked_mul(snapshot.cols)?;
    let cell_end = cell_start.checked_add(snapshot.cols)?;
    Some(TerminalSurfaceRetainedRow {
        cols: snapshot.cols,
        cells: snapshot.cells.get(cell_start..cell_end)?.to_vec(),
        line: snapshot.lines.get(row).cloned().unwrap_or_default(),
        styled_line: snapshot.styled_lines.get(row).cloned().unwrap_or_default(),
        line_signature: *snapshot.line_signatures.get(row).unwrap_or(&0),
        line_timestamp_ms: *snapshot.line_timestamps_ms.get(row).unwrap_or(&None),
        line_wrapped: *snapshot.line_wrapped.get(row).unwrap_or(&false),
        hyperlink_line: snapshot
            .hyperlink_lines
            .get(row)
            .cloned()
            .unwrap_or_default(),
        command_mark: *snapshot.command_marks.get(row).unwrap_or(&None),
    })
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
    if snapshot.rows == 0 {
        return None;
    }
    let end = snapshot.total_rows.saturating_sub(snapshot.display_offset);
    let start = end.saturating_sub(snapshot.rows);
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
        let visual_y_offset = terminal_effective_visual_scroll_offset_px(
            self.scroll_snapshot_pending,
            self.scroll_offset,
            self.display_offset,
            self.scroll_residual_lines,
            viewport_anchor_row,
            snapshot.rows,
            self.viewport_rows,
            cell_h,
        ) - viewport_anchor_row as f32 * cell_h;
        let line_count = snapshot.lines.len();
        let visible_gutter_rows = terminal_surface_visible_rows_for_viewport(
            self.viewport_rows,
            line_count,
            visual_y_offset,
            cell_h,
        );
        let gutter_enabled = self.show_line_numbers || self.show_timestamps;
        let show_scroll_fab = self.is_active && self.visual_scroll_active();
        let performance_overlay = self.performance_overlay;
        let skipped_output_chars = self.skipped_output_chars;
        let visual_bell = self.visual_bell && self.is_active;
        let mut grid = NyaTerminalElement::new(
            snapshot.clone(),
            Arc::new(Vec::new()),
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
            let abs_start = snapshot
                .total_rows
                .saturating_sub(snapshot.display_offset)
                .saturating_sub(snapshot.rows);
            let gutter_y_offset = visual_y_offset + visible_gutter_rows.start as f32 * cell_h;
            let mut gutter = div()
                .relative()
                .top(px(gutter_y_offset))
                .flex()
                .flex_col()
                .flex_none()
                .mr(px(10.))
                .border_r_1()
                .border_color(rgb(palette.border));
            for line_index in visible_gutter_rows {
                let is_wrapped = snapshot
                    .line_wrapped
                    .get(line_index)
                    .copied()
                    .unwrap_or(false);
                let has_rendered_row =
                    snapshot.cursor_row == usize::MAX || line_index <= snapshot.cursor_row;
                let ts_label = if self.show_timestamps && has_rendered_row && !is_wrapped {
                    snapshot
                        .line_timestamps_ms
                        .get(line_index)
                        .copied()
                        .flatten()
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
                gutter = gutter.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .min_h(px(cell_h))
                        .gap(px(gutter_metrics.gap_width))
                        .flex_none()
                        .pr(px(8.))
                        .text_color(rgb(palette.text_dimmed))
                        .font_family(self.font_family.clone())
                        .text_size(px(self.font_size))
                        .when(self.show_timestamps, |this| {
                            this.child(div().w(px(ts_w)).flex_none().child(ts_label))
                        })
                        .when(self.show_line_numbers, |this| {
                            this.child(div().w(px(ln_w)).flex_none().child(line_label))
                        }),
                );
            }
            Some(gutter)
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

        // Build interactive chrome one at a time to satisfy impl Trait borrow rules.
        let fab = if show_scroll_fab {
            Some(self.scroll_to_live_fab(cx).into_any_element())
        } else {
            None
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
            .font_family(self.font_family.clone())
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
                    })
                    .when_some(fab, |this, fab| this.child(fab)),
            )
            .child(scrollbar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terminal_test_output_lines(count: usize) -> String {
        (0..count)
            .map(|index| format!("line {index:03}\n"))
            .collect::<String>()
    }

    fn terminal_test_retained_live_snapshot(count: usize) -> (TerminalSnapshot, usize) {
        let mut screen = TerminalScreen::default();
        screen.advance_decoded_text(&terminal_test_output_lines(count));
        let base = screen.viewport_snapshot(0);
        let viewport_rows = base.rows.max(1);
        let older = screen.viewport_snapshot(viewport_rows);
        let retained_older_rows = older.rows.min(viewport_rows);
        let mut snapshot = base;
        if retained_older_rows == 0 || snapshot.cols == 0 {
            return (snapshot, viewport_rows);
        }

        let older_start = older.rows.saturating_sub(retained_older_rows);
        let mut cells = Vec::with_capacity((snapshot.rows + retained_older_rows) * snapshot.cols);
        for row in older_start..older.rows {
            let start = row.saturating_mul(older.cols);
            let end = start.saturating_add(older.cols).min(older.cells.len());
            cells.extend_from_slice(&older.cells[start..end]);
        }
        cells.extend(snapshot.cells);
        snapshot.cells = cells;

        let mut lines = older.lines[older_start..].to_vec();
        lines.extend(snapshot.lines);
        snapshot.lines = lines;

        let mut styled_lines = older.styled_lines[older_start..].to_vec();
        styled_lines.extend(snapshot.styled_lines);
        snapshot.styled_lines = styled_lines;

        let mut line_signatures = older.line_signatures[older_start..].to_vec();
        line_signatures.extend(snapshot.line_signatures);
        snapshot.line_signatures = line_signatures;

        let mut line_timestamps_ms = older.line_timestamps_ms[older_start..].to_vec();
        line_timestamps_ms.extend(snapshot.line_timestamps_ms);
        snapshot.line_timestamps_ms = line_timestamps_ms;

        let mut line_wrapped = older.line_wrapped[older_start..].to_vec();
        line_wrapped.extend(snapshot.line_wrapped);
        snapshot.line_wrapped = line_wrapped;

        let mut hyperlink_lines = older.hyperlink_lines[older_start..].to_vec();
        hyperlink_lines.extend(snapshot.hyperlink_lines);
        snapshot.hyperlink_lines = hyperlink_lines;

        let mut command_marks = older.command_marks[older_start..].to_vec();
        command_marks.extend(snapshot.command_marks);
        snapshot.command_marks = command_marks;

        snapshot.rows = snapshot.rows.saturating_add(retained_older_rows);
        (snapshot, viewport_rows)
    }

    #[test]
    fn visual_scroll_offset_tracks_target_display_and_residual() {
        assert_eq!(terminal_visual_scroll_offset_px(0, 0, 0.0, 16.0), 0.0);
        assert_eq!(terminal_visual_scroll_offset_px(1, 0, 0.25, 16.0), 20.0);
        assert_eq!(terminal_visual_scroll_offset_px(0, 1, -0.25, 16.0), -20.0);
        assert_eq!(terminal_visual_scroll_offset_px(4, 4, 0.5, 20.0), 10.0);
        assert_eq!(terminal_visual_scroll_offset_px(20, 0, 0.0, 16.0), 320.0);
        assert_eq!(
            terminal_effective_visual_scroll_offset_px(true, 1, 0, 0.25, 1, 4, 2, 16.0),
            16.0
        );
        assert_eq!(
            terminal_effective_visual_scroll_offset_px(false, 1, 0, 0.25, 0, 1, 1, 16.0),
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
    fn scroll_without_target_snapshot_preserves_stale_paint_state() {
        let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
        let rows = snapshot.rows;
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

        surface.apply_frame_snapshot(
            snapshot, 0, 0.0, 0, 10, rows, false, None, 0, true, true, "block",
        );
        surface.update_scroll_chrome_without_snapshot(1, 0.0, 1, 10, rows, false, None, 0);

        assert_eq!(surface.display_offset, 0);
        assert!(surface.scroll_snapshot_pending);
        assert!(!surface.keyword_rules.is_empty());
        assert_eq!(surface.decorations[0].link_ranges, vec![(1, 3)]);
        assert!(surface.has_action_link_decorations);
        assert_eq!(
            terminal_effective_visual_scroll_offset_px(
                surface.scroll_snapshot_pending,
                surface.scroll_offset,
                surface.display_offset,
                surface.scroll_residual_lines,
                0,
                rows,
                rows,
                16.0,
            ),
            0.0
        );
    }

    #[test]
    fn pending_scroll_uses_retained_rows_before_target_snapshot_arrives() {
        assert_eq!(
            terminal_effective_visual_scroll_offset_px(true, 9, 0, 0.0, 12, 40, 20, 16.0),
            144.0
        );
        assert_eq!(
            terminal_effective_visual_scroll_offset_px(true, 40, 0, 0.0, 12, 40, 20, 16.0),
            192.0
        );
        assert_eq!(
            terminal_effective_visual_scroll_offset_px(true, 0, 6, -3.0, 12, 40, 20, 16.0),
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
        let visual_y_offset = terminal_effective_visual_scroll_offset_px(
            true,
            target_offset,
            0,
            0.0,
            anchor,
            snapshot.rows,
            viewport_rows,
            cell_h,
        ) - anchor as f32 * cell_h;
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
        let rows = snapshot.rows;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
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
        surface.update_scroll_chrome_without_snapshot(1, 0.0, 1, 10, rows, false, None, 0);
        assert!(surface.scroll_snapshot_pending);

        surface.apply_frame_snapshot(
            snapshot, 1, 0.0, 1, 10, rows, false, None, 0, true, true, "block",
        );

        assert!(!surface.scroll_snapshot_pending);
        assert_eq!(surface.display_offset, 1);
        assert_eq!(
            terminal_effective_visual_scroll_offset_px(
                surface.scroll_snapshot_pending,
                surface.scroll_offset,
                surface.display_offset,
                surface.scroll_residual_lines,
                0,
                rows,
                rows,
                16.0,
            ),
            0.0
        );
    }

    #[test]
    fn local_surface_scroll_state_reuses_covering_snapshot_immediately() {
        let (snapshot, rows) = terminal_test_retained_live_snapshot(80);
        let snapshot = Arc::new(snapshot);
        let scrollback_len = snapshot.scrollback_len;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
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

        surface.apply_frame_snapshot(
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
        let rows = snapshot.rows;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
            snapshot, 0, 0.0, 0, 10, rows, false, None, 0, false, false, "block",
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
        let rows = snapshot.rows;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
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

        surface.set_cursor_blink_visible(true);
        assert!(!surface.show_cursor);

        surface.set_decorations_and_keywords(Vec::new(), Arc::new(Vec::new()), true, "block");
        assert!(!surface.show_cursor);

        surface.apply_frame_snapshot(
            snapshot, 0, 0.60, 0, 10, rows, false, None, 0, false, true, "block",
        );
        assert!(!surface.show_cursor);
    }

    #[test]
    fn selection_visual_update_preserves_existing_decorations() {
        let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
        let rows = snapshot.rows;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
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
                TerminalCellPos::new(0, 2),
                TerminalCellPos::new(1, usize::MAX),
                0,
                0,
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
    fn selection_visual_row_range_tracks_viewport_anchor_and_union() {
        assert_eq!(terminal_selection_visual_row_range(None, 8), None);
        assert_eq!(
            terminal_selection_visual_row_range(
                Some(TerminalSelection::new(TerminalCellPos::new(2, 4))),
                8
            ),
            None
        );
        assert_eq!(
            terminal_selection_visual_row_range(Some(TerminalSelection::all_buffer(80)), 8),
            Some(0..8)
        );
        assert_eq!(
            terminal_selection_visual_row_range(
                Some(TerminalSelection::from_range(
                    TerminalCellPos::new(2, 1),
                    TerminalCellPos::new(4, 3),
                    0,
                    3,
                )),
                10,
            ),
            Some(5..8)
        );
        assert_eq!(
            terminal_selection_visual_row_range(
                Some(TerminalSelection::from_range(
                    TerminalCellPos::new(0, 1),
                    TerminalCellPos::new(5, 3),
                    0,
                    8,
                )),
                10,
            ),
            Some(8..10)
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
        let rows = snapshot.rows;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
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

        let mut decorations = vec![TerminalLineDecorations::default(); snapshot.lines.len()];
        decorations[0].link_ranges = vec![(1, 3)];
        decorations[2].selection_cols = Some((3, snapshot.cols));
        decorations[3].selection_cols = Some((0, 6));
        decorations[8].search_ranges = vec![(2, 5)];
        surface.set_decorations_and_keywords(decorations, Arc::new(Vec::new()), true, "block");

        assert_eq!(surface.selection_visual_row_range, Some(2..4));
        let revision_before = surface.revision;
        assert!(
            surface.set_selection_visual(Some(TerminalSelection::from_range(
                TerminalCellPos::new(3, 1),
                TerminalCellPos::new(4, 4),
                0,
                0,
            )))
        );

        assert_eq!(surface.decorations[0].link_ranges, vec![(1, 3)]);
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
                TerminalCellPos::new(3, 1),
                TerminalCellPos::new(4, 4),
                0,
                0,
            )))
        );
        assert_eq!(surface.revision, revision_before_same_selection);

        assert!(surface.set_selection_visual(None));
        assert_eq!(surface.decorations[0].link_ranges, vec![(1, 3)]);
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
        let rows = snapshot.rows;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
            snapshot, 0, 0.0, 0, 10, rows, false, None, 0, false, false, "block",
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
        surface.apply_frame_snapshot(
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
        let rows = snapshot.rows;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
            snapshot, 0, 0.0, 0, 10, rows, false, None, 0, false, false, "block",
        );
        assert_eq!(
            surface.scroll_snapshot_request_offsets_to_enqueue(vec![4]),
            vec![4]
        );
        assert_eq!(
            surface.scroll_snapshot_request_offsets_to_enqueue(vec![4]),
            Vec::<usize>::new()
        );

        surface.update_scroll_chrome_without_snapshot(5, 0.0, 5, 11, rows, true, None, 0);

        assert_eq!(
            surface.scroll_snapshot_request_offsets_to_enqueue(vec![4, 5]),
            vec![4, 5]
        );
    }

    #[test]
    fn local_surface_wheel_consumes_edge_noop_without_visual_sync() {
        let snapshot = Arc::new(TerminalScreen::default().viewport_snapshot(0));
        let rows = snapshot.rows;
        let mut surface = TerminalSurface::new("session");

        let frame_applied = surface.apply_frame_snapshot(
            snapshot, 0, 0.0, 0, 0, rows, false, None, 0, false, false, "block",
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
        let rows = snapshot.rows;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
            snapshot, 0, 0.0, 0, 10, rows, false, None, 0, false, false, "block",
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

        surface.apply_frame_snapshot(
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

        let frame_applied = surface.apply_frame_snapshot(
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
        let rows = snapshot.rows;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
            snapshot, 0, 0.0, 0, 10, rows, false, None, 0, false, false, "block",
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
        let rows = first_snapshot.rows.max(1);
        let scrollback_len = first_snapshot.scrollback_len;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
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
        surface.apply_frame_snapshot(
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
        assert!(!surface.has_action_link_decorations);
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
    fn local_surface_scroll_state_synthesizes_cross_window_on_hot_path() {
        let mut screen = TerminalScreen::default();
        screen.advance_decoded_text(&terminal_test_output_lines(160));
        let live_snapshot = Arc::new(screen.viewport_snapshot(0));
        let rows = live_snapshot.rows.max(2);
        let older_offset = rows;
        let target_offset = rows / 2;
        let older_snapshot = Arc::new(screen.viewport_snapshot(older_offset));
        let scrollback_len = live_snapshot.scrollback_len;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
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
        surface.apply_frame_snapshot(
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
        let rows = live_snapshot.rows.max(2);
        let older_offset = rows;
        let target_offset = rows / 2;
        let older_snapshot = Arc::new(screen.viewport_snapshot(older_offset));
        let scrollback_len = live_snapshot.scrollback_len;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
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
        surface.apply_frame_snapshot(
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
        let rows = live_snapshot.rows.max(2);
        let older_offset = rows;
        let target_offset = rows / 2;
        let older_snapshot = Arc::new(screen.viewport_snapshot(older_offset));
        let scrollback_len = live_snapshot.scrollback_len;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
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
        surface.apply_frame_snapshot(
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
        assert_eq!(synthesized.display_offset, target_offset);
        assert!(terminal_snapshot_covers_display_offset(
            synthesized.as_ref(),
            target_offset,
            rows,
            scrollback_len
        ));
    }

    #[test]
    fn local_surface_scroll_state_does_not_synthesize_across_gap() {
        let mut screen = TerminalScreen::default();
        screen.advance_decoded_text(&terminal_test_output_lines(200));
        let live_snapshot = Arc::new(screen.viewport_snapshot(0));
        let rows = live_snapshot.rows.max(2);
        let far_offset = rows.saturating_mul(2);
        let target_offset = rows;
        let far_snapshot = Arc::new(screen.viewport_snapshot(far_offset));
        let scrollback_len = live_snapshot.scrollback_len;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
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
        surface.apply_frame_snapshot(
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
        let rows = snapshot.rows;
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
        let rows = snapshot.rows;
        let old_scrollback_len = snapshot.scrollback_len;
        let growth = 3;
        let new_display_offset = old_display_offset + growth;
        let new_scrollback_len = old_scrollback_len + growth;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
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

        surface.update_scroll_position_without_snapshot(
            new_display_offset,
            0.0,
            new_display_offset,
            new_scrollback_len,
            rows,
            true,
            None,
            0,
        );

        assert!(!surface.scroll_snapshot_pending);
        assert_eq!(surface.display_offset, new_display_offset);
        assert!(surface.has_new_while_scrolled);
        assert_eq!(
            terminal_effective_visual_scroll_offset_px(
                surface.scroll_snapshot_pending,
                surface.scroll_offset,
                surface.display_offset,
                surface.scroll_residual_lines,
                0,
                rows,
                rows,
                16.0,
            ),
            0.0
        );
    }

    #[test]
    fn surface_retained_scroll_state_resets_when_scrollback_shrinks() {
        let mut old_screen = TerminalScreen::default();
        old_screen.advance_decoded_text(&terminal_test_output_lines(120));
        let old_offset = 12;
        let old_snapshot = Arc::new(old_screen.viewport_snapshot(old_offset));
        let old_rows = old_snapshot.rows;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
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
        let new_rows = new_snapshot.rows;
        surface.apply_frame_snapshot(
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
        let old_rows = old_snapshot.rows.max(2);
        let new_snapshot = Arc::new(screen.viewport_snapshot(0));
        let new_viewport_rows = old_rows - 1;
        let mut surface = TerminalSurface::new("session");

        surface.apply_frame_snapshot(
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

        surface.apply_frame_snapshot(
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
}
