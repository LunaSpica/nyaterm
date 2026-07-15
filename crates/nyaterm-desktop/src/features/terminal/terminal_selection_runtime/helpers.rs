use super::*;
use std::ops::Range;

/// Invisible canvas child that records the terminal output bounds for selection hit-testing.
pub(in crate::features) fn terminal_bounds_tracker(
    entity: gpui::Entity<NyaTermApp>,
    session_id: Option<String>,
    active: bool,
) -> impl IntoElement {
    let input_entity = entity.clone();
    let tracked_session_id = session_id.clone();
    gpui::canvas(
        move |bounds, _window, cx| {
            // Defer mutation so we never re-enter the entity while layout/prepaint is running.
            let entity = entity.clone();
            let session_id = tracked_session_id.clone();
            cx.defer(move |cx| {
                let _ = entity.update(cx, |this, _cx| {
                    this.remember_terminal_surface_bounds_for_session(
                        session_id.as_deref(),
                        bounds,
                    );
                });
            });
        },
        move |bounds, _state, window, cx| {
            if !active {
                return;
            }
            let focus = input_entity.read(cx).terminal_focus.clone();
            if input_entity
                .read(cx)
                .settings
                .interaction_mac_ime_compatibility
            {
                window.handle_input(&focus, ElementInputHandler::new(bounds, input_entity), cx);
            }
        },
    )
    .absolute()
    .size_full()
}

impl EntityInputHandler for NyaTermApp {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        if self.terminal_ime_marked_text.is_empty() {
            return None;
        }
        let len = self.terminal_ime_marked_text.encode_utf16().count();
        let start = range.start.min(len);
        let end = range.end.min(len).max(start);
        *adjusted_range = Some(start..end);
        Some(self.terminal_ime_marked_text.clone())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        None
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let len = self.terminal_ime_marked_text.encode_utf16().count();
        (len > 0).then_some(0..len)
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.terminal_ime_marked_text.clear();
    }

    fn replace_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal_ime_marked_text.clear();
        if !text.is_empty() {
            if let Some(selected) = self.smart_cursor_selected_input_range() {
                if self.replace_smart_input_selection(selected, text, cx) {
                    return;
                }
            }
            let bytes = text.as_bytes().to_vec();
            let has_buffer_selection = self.terminal_selection.is_some()
                && self.smart_cursor_selected_input_range().is_none();
            if has_buffer_selection {
                self.send_terminal_input_without_suggestion_track(bytes, cx);
            } else {
                self.send_terminal_input(bytes, cx);
            }
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal_ime_marked_text = new_text.to_string();
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let (cell_w, cell_h) = self.terminal_cell_size();
        let pad = self.terminal_content_padding_px();
        let gutter = self.terminal_gutter_width_px();
        let snapshot = self.terminal_snapshot_for_session(self.active_session_id.as_deref(), 0);
        let row = if snapshot.cursor_row == usize::MAX {
            snapshot.lines.len().saturating_sub(1)
        } else {
            snapshot.cursor_row.min(snapshot.rows.saturating_sub(1))
        };
        let col = snapshot.cursor_col.min(snapshot.cols.saturating_sub(1));
        let origin_x = f32::from(element_bounds.origin.x);
        let origin_y = f32::from(element_bounds.origin.y);
        let max_x = origin_x + f32::from(element_bounds.size.width) - cell_w.max(1.);
        let max_y = origin_y + f32::from(element_bounds.size.height) - cell_h.max(1.);
        let x = (origin_x + pad + gutter + col as f32 * cell_w)
            .min(max_x)
            .max(origin_x);
        let y = (origin_y + pad + row as f32 * cell_h)
            .min(max_y)
            .max(origin_y);
        Some(gpui::bounds(
            Point { x: px(x), y: px(y) },
            Size {
                width: px(cell_w.max(1.)),
                height: px(cell_h.max(1.)),
            },
        ))
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(0)
    }
}

pub(super) fn open_external_url_for_action(url: &str) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("empty url".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open url: {error}"))
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open url: {error}"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open url: {error}"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::features) enum SmartSelectionEdge {
    Start,
    End,
}
