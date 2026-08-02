use gpui::{Bounds, Entity, IntoElement, Pixels, prelude::*};

use crate::features::NyaTermApp;

pub(in crate::features) const TERMINAL_SCROLLBAR_COLUMN_WIDTH: f32 = 8.0;
pub(in crate::features) const TERMINAL_SCROLLBAR_TRACK_PADDING_Y: f32 = 2.0;
pub(in crate::features) const TERMINAL_SCROLLBAR_TRACK_PADDING_RIGHT: f32 = 2.0;
pub(in crate::features) const TERMINAL_SCROLLBAR_THUMB_WIDTH: f32 = 3.0;
pub(in crate::features) const TERMINAL_SCROLLBAR_THUMB_ACTIVE_WIDTH: f32 = 5.0;
pub(in crate::features) const TERMINAL_SCROLLBAR_MIN_THUMB_HEIGHT: f32 = 18.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::features) struct TerminalScrollbarMetrics {
    pub track_height: f32,
    pub thumb_height: f32,
    pub thumb_top: f32,
    pub thumb_travel: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::features) struct TerminalScrollbarInput {
    pub viewport_rows: usize,
    pub scrollback_rows: usize,
    pub scroll_offset: usize,
    pub track_height: f32,
    pub min_thumb_height: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::features) struct TerminalScrollbarDragState {
    pub session_id: Option<String>,
    pub grab_offset_y: f32,
}

pub(in crate::features) fn terminal_scrollbar_metrics(
    input: TerminalScrollbarInput,
) -> TerminalScrollbarMetrics {
    let track_height = finite_non_negative(input.track_height);
    let min_thumb_height = finite_non_negative(input.min_thumb_height);
    let viewport_rows = input.viewport_rows.max(1);
    let scrollback_rows = input.scrollback_rows;
    let content_rows = viewport_rows.saturating_add(scrollback_rows).max(1);
    let max_offset = scrollback_rows;
    let proportional = if track_height <= 0.0 {
        0.0
    } else {
        track_height * viewport_rows as f32 / content_rows as f32
    };
    let thumb_height = proportional.max(min_thumb_height).min(track_height);
    let thumb_travel = (track_height - thumb_height).max(0.0);
    let position_ratio = if max_offset == 0 {
        0.0
    } else {
        1.0 - (input.scroll_offset.min(max_offset) as f32 / max_offset as f32)
    };
    let thumb_top = thumb_travel * position_ratio.clamp(0.0, 1.0);
    TerminalScrollbarMetrics {
        track_height,
        thumb_height,
        thumb_top,
        thumb_travel,
    }
}

pub(in crate::features) fn terminal_scroll_offset_from_pointer(
    pointer_y: f32,
    track_top: f32,
    metrics: TerminalScrollbarMetrics,
    grab_offset_y: f32,
    max_offset: usize,
) -> usize {
    if max_offset == 0 || metrics.thumb_travel <= 0.0 {
        return 0;
    }
    let local_thumb_top = (finite_or(pointer_y, track_top)
        - finite_or(track_top, 0.0)
        - finite_non_negative(grab_offset_y))
    .clamp(0.0, metrics.thumb_travel);
    let ratio = (local_thumb_top / metrics.thumb_travel).clamp(0.0, 1.0);
    ((1.0 - ratio) * max_offset as f32).round() as usize
}

pub(in crate::features) fn terminal_scrollbar_grab_offset_for_pointer(
    pointer_y: f32,
    track_top: f32,
    metrics: TerminalScrollbarMetrics,
) -> f32 {
    let local_y = finite_or(pointer_y, track_top) - finite_or(track_top, 0.0);
    if local_y >= metrics.thumb_top && local_y <= metrics.thumb_top + metrics.thumb_height {
        (local_y - metrics.thumb_top).clamp(0.0, metrics.thumb_height)
    } else {
        metrics.thumb_height * 0.5
    }
}

pub(in crate::features) fn terminal_scrollbar_thumb_color(
    palette: crate::theme::ThemePalette,
    active: bool,
) -> u32 {
    if active {
        palette.link
    } else {
        palette.text_muted
    }
}

pub(in crate::features) fn terminal_scrollbar_track_bounds_tracker(
    entity: Entity<NyaTermApp>,
    session_id: Option<String>,
) -> impl IntoElement {
    gpui::canvas(
        move |bounds, window, cx| {
            let visible = window.content_mask().bounds.intersect(&bounds);
            if visible.size.width <= gpui::px(0.) || visible.size.height <= gpui::px(0.) {
                return;
            }
            let entity = entity.clone();
            let session_id = session_id.clone();
            cx.defer(move |cx| {
                entity.update(cx, |this, _cx| {
                    this.remember_terminal_scrollbar_track_bounds_for_session(
                        session_id.as_deref(),
                        visible,
                    );
                });
            });
        },
        |_bounds, _state, _window, _cx| {},
    )
    .absolute()
    .size_full()
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

pub(in crate::features) fn track_height(bounds: Bounds<Pixels>) -> f32 {
    finite_non_negative(f32::from(bounds.size.height))
}

#[cfg(test)]
mod tests {
    use super::{
        TerminalScrollbarInput, terminal_scroll_offset_from_pointer, terminal_scrollbar_metrics,
    };

    #[test]
    fn terminal_scrollbar_metrics_handle_no_scrollback() {
        let metrics = terminal_scrollbar_metrics(TerminalScrollbarInput {
            viewport_rows: 24,
            scrollback_rows: 0,
            scroll_offset: 0,
            track_height: 120.0,
            min_thumb_height: 18.0,
        });
        assert_eq!(metrics.thumb_height, 120.0);
        assert_eq!(metrics.thumb_top, 0.0);
        assert_eq!(metrics.thumb_travel, 0.0);
    }

    #[test]
    fn terminal_scrollbar_metrics_place_live_offset_at_bottom_and_max_at_top() {
        let live = terminal_scrollbar_metrics(TerminalScrollbarInput {
            viewport_rows: 10,
            scrollback_rows: 90,
            scroll_offset: 0,
            track_height: 100.0,
            min_thumb_height: 18.0,
        });
        let history = terminal_scrollbar_metrics(TerminalScrollbarInput {
            scroll_offset: 90,
            ..TerminalScrollbarInput {
                viewport_rows: 10,
                scrollback_rows: 90,
                scroll_offset: 0,
                track_height: 100.0,
                min_thumb_height: 18.0,
            }
        });
        assert_eq!(live.thumb_top, live.thumb_travel);
        assert_eq!(history.thumb_top, 0.0);
    }

    #[test]
    fn terminal_scrollbar_metrics_enforce_min_thumb_and_short_track() {
        let metrics = terminal_scrollbar_metrics(TerminalScrollbarInput {
            viewport_rows: 10,
            scrollback_rows: 10_000,
            scroll_offset: 0,
            track_height: 100.0,
            min_thumb_height: 18.0,
        });
        assert_eq!(metrics.thumb_height, 18.0);
        let short = terminal_scrollbar_metrics(TerminalScrollbarInput {
            track_height: 8.0,
            ..TerminalScrollbarInput {
                viewport_rows: 10,
                scrollback_rows: 10_000,
                scroll_offset: 0,
                track_height: 100.0,
                min_thumb_height: 18.0,
            }
        });
        assert_eq!(short.thumb_height, 8.0);
        assert_eq!(short.thumb_travel, 0.0);
    }

    #[test]
    fn terminal_scrollbar_pointer_mapping_clamps_and_respects_track_origin_and_grab() {
        let metrics = terminal_scrollbar_metrics(TerminalScrollbarInput {
            viewport_rows: 10,
            scrollback_rows: 90,
            scroll_offset: 0,
            track_height: 100.0,
            min_thumb_height: 20.0,
        });
        assert_eq!(
            terminal_scroll_offset_from_pointer(40.0, 50.0, metrics, 10.0, 90),
            90
        );
        assert_eq!(
            terminal_scroll_offset_from_pointer(180.0, 50.0, metrics, 10.0, 90),
            0
        );
        assert_eq!(
            terminal_scroll_offset_from_pointer(100.0, 50.0, metrics, 10.0, 90),
            45
        );
    }

    #[test]
    fn terminal_scrollbar_metrics_defend_against_invalid_values() {
        let metrics = terminal_scrollbar_metrics(TerminalScrollbarInput {
            viewport_rows: 1,
            scrollback_rows: usize::MAX,
            scroll_offset: usize::MAX,
            track_height: f32::NAN,
            min_thumb_height: f32::NAN,
        });
        assert_eq!(metrics.track_height, 0.0);
        assert_eq!(metrics.thumb_height, 0.0);
        assert_eq!(metrics.thumb_travel, 0.0);
        assert_eq!(
            terminal_scroll_offset_from_pointer(f32::NAN, 0.0, metrics, 0.0, 10),
            0
        );
    }
}
