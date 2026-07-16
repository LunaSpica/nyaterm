use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Process-wide surface paint counter (Phase 0 isolation diagnostics).
pub(in crate::features) static TERMINAL_SURFACE_PAINT_COUNT: AtomicU64 = AtomicU64::new(0);
pub(in crate::features) static FULL_SHELL_PAINT_COUNT: AtomicU64 = AtomicU64::new(0);

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
    snapshot: Option<Arc<TerminalSnapshot>>,
    keyword_rules: Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>>,
    decorations: Vec<TerminalLineDecorations>,
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
    has_new_while_scrolled: bool,
    performance_overlay: Option<TerminalPerformanceOverlay>,
    skipped_output_chars: u64,
    visual_bell: bool,
    is_active: bool,
    revision: u64,
}

impl TerminalSurface {
    pub(in crate::features) fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            snapshot: None,
            keyword_rules: Arc::new(Vec::new()),
            decorations: Vec::new(),
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
            has_new_while_scrolled: false,
            performance_overlay: None,
            skipped_output_chars: 0,
            visual_bell: false,
            is_active: false,
            revision: 0,
        }
    }

    pub(in crate::features) fn session_id(&self) -> &str {
        &self.session_id
    }

    pub(in crate::features) fn take_layout_cache(&self) -> Arc<Mutex<NyaTerminalLayoutCache>> {
        self.layout_cache.clone()
    }

    pub(in crate::features) fn apply_frame_snapshot(
        &mut self,
        snapshot: Arc<TerminalSnapshot>,
        scroll_offset: usize,
        has_new_while_scrolled: bool,
        performance_overlay: Option<TerminalPerformanceOverlay>,
        skipped_output_chars: u64,
        show_cursor: bool,
        cursor_style: impl Into<String>,
    ) {
        // Do not clear decorations/keywords here: shell paint owns those so that
        // frame-only surface notifies keep selection/search highlights until the
        // next chrome rebuild pushes fresh decorations.
        self.snapshot = Some(snapshot);
        self.scroll_offset = scroll_offset;
        self.has_new_while_scrolled = has_new_while_scrolled;
        self.performance_overlay = performance_overlay;
        self.skipped_output_chars = skipped_output_chars;
        self.show_cursor = show_cursor;
        self.cursor_style = cursor_style.into();
        self.revision = self.revision.saturating_add(1);
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

    pub(in crate::features) fn set_cursor_blink_visible(&mut self, show_cursor: bool) {
        self.show_cursor = show_cursor;
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
        decorations: Vec<TerminalLineDecorations>,
        keyword_rules: Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>>,
        show_cursor: bool,
        cursor_style: impl Into<String>,
    ) {
        self.decorations = decorations;
        self.keyword_rules = keyword_rules;
        self.show_cursor = show_cursor;
        self.cursor_style = cursor_style.into();
    }
}

impl Render for TerminalSurface {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        TERMINAL_SURFACE_PAINT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let palette = self.palette;
        let cell_w = self.cell_width.max(1.0);
        let cell_h = self.cell_height.max(1.0);
        let snapshot = self
            .snapshot
            .clone()
            .unwrap_or_else(|| Arc::new(TerminalScreen::default().viewport_snapshot(0)));
        let line_count = snapshot.lines.len();
        let gutter_enabled = self.show_line_numbers || self.show_timestamps;

        let mut grid = NyaTerminalElement::new(
            snapshot.clone(),
            self.keyword_rules.clone(),
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
        grid = grid.with_layout_cache(self.layout_cache.clone());

        let gutter = if gutter_enabled {
            let ts_w = if self.show_timestamps {
                if self.show_timestamp_ms { 88.0 } else { 72.0 }
            } else {
                0.0
            };
            let ln_w = if self.show_line_numbers { 40.0 } else { 0.0 };
            let abs_start = snapshot
                .total_rows
                .saturating_sub(snapshot.display_offset)
                .saturating_sub(snapshot.rows);
            let mut gutter = div().flex().flex_col().flex_none();
            for line_index in 0..line_count {
                let ts_label = if self.show_timestamps {
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
                let line_label = if self.show_line_numbers {
                    format!("{:>5}", abs_start + line_index + 1)
                } else {
                    String::new()
                };
                gutter = gutter.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .min_h(px(cell_h))
                        .gap_1()
                        .flex_none()
                        .pr_1()
                        .text_color(rgb(palette.text_dimmed))
                        .font_family(self.font_family.clone())
                        .text_size(px(self.font_size * 0.85))
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
            div().flex().flex_row().child(gutter).child(grid)
        } else {
            div().flex().flex_row().child(grid)
        };

        div()
            .id(SharedString::from(format!(
                "terminal-surface-{}",
                self.session_id
            )))
            .size_full()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(rgb(palette.terminal_bg))
            .text_color(rgb(palette.terminal_fg))
            .font_family(self.font_family.clone())
            .text_size(px(self.font_size))
            .when(self.visual_bell && self.is_active, |this| {
                this.border_2().border_color(rgb(palette.warning))
            })
            .child(body)
            .when(
                self.is_active && self.scroll_offset > 0 && self.has_new_while_scrolled,
                |this| {
                    this.child(
                        div()
                            .absolute()
                            .right_3()
                            .bottom_3()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(palette.surface_elevated))
                            .border_1()
                            .border_color(rgb(palette.border))
                            .text_xs()
                            .text_color(rgb(palette.warning))
                            .child("New"),
                    )
                },
            )
            .when_some(self.performance_overlay, |this, overlay| {
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
                                format!("protecting output… skipped={}", self.skipped_output_chars)
                            }
                            TerminalPerformanceOverlay::Recovered => "render recovered".to_string(),
                        }),
                )
            })
    }
}
