use super::*;
use crate::features::terminal_runtime::terminal_scroll_track_ratio;
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
    /// Parent app for scroll/selection actions that still live on NyaTermApp.
    app: Option<Entity<NyaTermApp>>,
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
    scrollback_len: usize,
    viewport_rows: usize,
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
            app: None,
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
            scrollback_len: 0,
            viewport_rows: 1,
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
        scrollback_len: usize,
        viewport_rows: usize,
        has_new_while_scrolled: bool,
        performance_overlay: Option<TerminalPerformanceOverlay>,
        skipped_output_chars: u64,
        show_cursor: bool,
        cursor_style: impl Into<String>,
    ) {
        // Decorations/keywords are pushed separately so frame notifies can keep
        // selection/search highlights until the next decoration rebuild.
        self.snapshot = Some(snapshot);
        self.scroll_offset = scroll_offset;
        self.scrollback_len = scrollback_len;
        self.viewport_rows = viewport_rows.max(1);
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
                            let _ = app.update(cx, |this, cx| {
                                if !session_id.is_empty() {
                                    this.activate_workspace_pane(session_id.clone(), cx);
                                }
                                let drag_session_id =
                                    (!session_id.is_empty()).then_some(session_id.clone());
                                this.begin_terminal_scrollbar_drag(drag_session_id.clone(), cx);
                                let Some(bounds) = (if session_id.is_empty() {
                                    this.terminal_surface_bounds
                                } else {
                                    this.terminal_session_surface_bounds
                                        .get(&session_id)
                                        .copied()
                                        .or(this.terminal_surface_bounds)
                                }) else {
                                    return;
                                };
                                let ratio = terminal_scroll_track_ratio(bounds, event.position.y);
                                this.set_terminal_scroll_from_track_ratio_for_session(
                                    drag_session_id.as_deref(),
                                    ratio,
                                    cx,
                                );
                            });
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
                                    palette.accent
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
            .on_click(cx.listener(move |_this, _, _, cx| {
                let Some(app) = app.clone() else {
                    return;
                };
                let _ = app.update(cx, |this, cx| {
                    this.scroll_terminal_to_bottom(cx);
                    this.terminal_status = "scrolled to live output".to_string();
                });
            }))
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(if has_new {
                        palette.warning
                    } else {
                        palette.accent
                    }))
                    .child(if has_new { "↓ New" } else { "↓ Live" }),
            )
    }
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
        let line_count = snapshot.lines.len();
        let gutter_enabled = self.show_line_numbers || self.show_timestamps;
        let show_scroll_fab = self.is_active && self.scroll_offset > 0;
        let performance_overlay = self.performance_overlay;
        let skipped_output_chars = self.skipped_output_chars;
        let visual_bell = self.visual_bell && self.is_active;

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
            div()
                .flex()
                .flex_row()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .child(gutter)
                .child(grid)
        } else {
            div()
                .flex()
                .flex_row()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .child(grid)
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
            .bg(rgb(palette.terminal_bg))
            .text_color(rgb(palette.terminal_fg))
            .font_family(self.font_family.clone())
            .text_size(px(self.font_size))
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
