use super::*;

impl NyaTermApp {
    pub(in crate::features) fn terminal_cell_size(&self) -> (f32, f32) {
        if let Some(metrics) = self.terminal_cell_metrics {
            return metrics;
        }
        self.fallback_terminal_cell_size()
    }

    pub(in crate::features) fn fallback_terminal_cell_size(&self) -> (f32, f32) {
        let font_size = self.settings.terminal_font_size.max(8) as f32;
        // Prefer painted fixed 18px when font is near default; scale with font otherwise.
        let cell_h = if (font_size - 14.).abs() < 0.5 {
            18.
        } else {
            (font_size * LINE_HEIGHT_RATIO).max(font_size + 2.)
        };
        // Tauri gutter fallback uses fontSize * 0.62 when measured cell is unavailable.
        let cell_w = (font_size * CELL_WIDTH_RATIO).max(4.);
        (cell_w, cell_h)
    }

    /// Refresh monospaced cell metrics from GPUI TextSystem for the configured terminal font.

    pub(in crate::features) fn refresh_terminal_cell_metrics(&mut self, cx: &App) {
        let font_size = self.settings.terminal_font_size.max(8) as f32;
        let family = self.gpui_terminal_font_family();
        let text_system = cx.text_system();
        let font_id = text_system.resolve_font(&gpui::font(SharedString::from(family)));
        let size = px(font_size);
        let measured_w = text_system
            .ch_advance(font_id, size)
            .or_else(|_| text_system.em_advance(font_id, size))
            .ok()
            .map(|w| f32::from(w))
            .filter(|w| w.is_finite() && *w > 1.0);
        let ascent = f32::from(text_system.ascent(font_id, size));
        let descent = f32::from(text_system.descent(font_id, size)).abs();
        let font_line = (ascent + descent).max(font_size + 2.);
        // Keep painter contract: default 14px font paints ~18px rows.
        let cell_h = if (font_size - 14.).abs() < 0.5 {
            18.
        } else {
            (font_size * LINE_HEIGHT_RATIO).max(font_line)
        };
        let cell_w = measured_w.unwrap_or_else(|| (font_size * CELL_WIDTH_RATIO).max(4.));
        let next = (cell_w, cell_h);
        if self.terminal_cell_metrics != Some(next) {
            self.terminal_cell_metrics = Some(next);
        }
    }

    pub(in crate::features) fn terminal_content_padding_px(&self) -> f32 {
        if self.settings.terminal_show_workspace_padding {
            16.
        } else {
            8.
        }
    }

    pub(in crate::features) fn terminal_gutter_width_px(&self) -> f32 {
        let (cell_w, _) = self.terminal_cell_size();
        terminal_gutter_metrics(
            cell_w,
            self.settings.terminal_font_size as f32,
            self.settings.terminal_show_timestamps,
            self.settings.terminal_show_timestamp_milliseconds,
            self.settings.terminal_show_line_numbers,
        )
        .total_width()
    }

    pub(in crate::features) fn terminal_timestamp_gutter_width_px(&self) -> f32 {
        let (cell_w, _) = self.terminal_cell_size();
        terminal_gutter_metrics(
            cell_w,
            self.settings.terminal_font_size as f32,
            true,
            self.settings.terminal_show_timestamp_milliseconds,
            false,
        )
        .timestamp_width
    }

    pub(in crate::features) fn terminal_line_number_gutter_width_px(&self) -> f32 {
        let (cell_w, _) = self.terminal_cell_size();
        terminal_gutter_metrics(
            cell_w,
            self.settings.terminal_font_size as f32,
            false,
            false,
            true,
        )
        .line_number_width
    }

    pub(in crate::features) fn active_terminal_grid_size(&self) -> (usize, usize) {
        self.terminal_grid_size_for_session(self.active_session_id.as_deref())
    }

    pub(in crate::features) fn terminal_grid_size_for_session(
        &self,
        session_id: Option<&str>,
    ) -> (usize, usize) {
        let offset = self.terminal_display_offset_for_session(session_id);
        let snapshot = self.terminal_snapshot_for_session(session_id, offset);
        let rows = self.terminal_viewport_rows_for_session(session_id);
        let cols = snapshot.cols.max(80);
        (rows, cols)
    }

    pub(in crate::features) fn terminal_viewport_rows_for_session(
        &self,
        session_id: Option<&str>,
    ) -> usize {
        let session_id = session_id.filter(|id| !id.is_empty());
        if let Some(session_id) = session_id
            && let Some(view) = self.terminal_views.get(session_id)
        {
            return view.viewport_rows_for_ui();
        }
        self.terminal_screen.viewport_snapshot(0).lines.len().max(1)
    }

    pub(in crate::features) fn terminal_scrollback_len_for_session(
        &self,
        session_id: Option<&str>,
    ) -> usize {
        let session_id = session_id.filter(|id| !id.is_empty());
        if let Some(session_id) = session_id
            && let Some(view) = self.terminal_views.get(session_id)
        {
            return view.scrollback_len_for_ui();
        }
        self.terminal_screen.scrollback_len()
    }

    pub(in crate::features) fn terminal_snapshot_row_for_session_viewport_row(
        &self,
        session_id: Option<&str>,
        snapshot: &nyaterm_terminal::TerminalSnapshot,
        display_offset: usize,
        viewport_row: usize,
    ) -> Option<usize> {
        terminal_snapshot_row_for_viewport_row(
            snapshot,
            display_offset,
            self.terminal_viewport_rows_for_session(session_id),
            self.terminal_scrollback_len_for_session(session_id),
            viewport_row,
        )
    }

    pub(in crate::features) fn point_to_terminal_cell(
        &self,
        position: Point<Pixels>,
    ) -> Option<TerminalCellPos> {
        self.point_to_terminal_cell_for_session(self.active_session_id.as_deref(), position)
    }

    pub(in crate::features) fn point_to_terminal_cell_for_session(
        &self,
        session_id: Option<&str>,
        position: Point<Pixels>,
    ) -> Option<TerminalCellPos> {
        let geometry = self.terminal_hit_test_geometry_for_session(session_id)?;
        Some(terminal_cell_for_visual_geometry(position, &geometry))
    }

    pub(in crate::features) fn terminal_hit_test_geometry_for_session(
        &self,
        session_id: Option<&str>,
    ) -> Option<TerminalHitTestGeometry> {
        let session_id = session_id.filter(|id| !id.is_empty());
        let bounds = session_id
            .and_then(|id| self.terminal_session_surface_bounds.get(id).copied())
            .or(self.terminal_surface_bounds)?;
        let (cell_w, cell_h) = self.terminal_cell_size();
        let pad = self.terminal_content_padding_px();
        let gutter = self.terminal_gutter_width_px();
        let (rows, cols) = self.terminal_grid_size_for_session(session_id);
        let display_offset = self.terminal_display_offset_for_session(session_id);
        let snapshot = self.terminal_snapshot_for_session(session_id, display_offset);
        let viewport_rows = self.terminal_viewport_rows_for_session(session_id);
        let scrollback_len = self.terminal_scrollback_len_for_session(session_id);
        let viewport_anchor_row = terminal_snapshot_anchor_row_for_display_offset(
            snapshot.as_ref(),
            display_offset,
            viewport_rows,
            scrollback_len,
        );
        let (scroll_offset, residual_lines) = if let Some(session_id) = session_id {
            self.terminal_views
                .get(session_id)
                .map(|view| {
                    (
                        view.scroll_offset,
                        self.terminal_scroll_residual_for_session(Some(session_id)),
                    )
                })
                .unwrap_or((display_offset, 0.0))
        } else {
            (
                self.terminal_scroll_offset,
                self.terminal_scroll_residual_for_session(None),
            )
        };
        let visual_y_offset = ((scroll_offset as isize - display_offset as isize) as f32
            + residual_lines)
            * cell_h.max(1.0)
            - (viewport_anchor_row as f32) * cell_h.max(1.0);
        Some(TerminalHitTestGeometry {
            bounds,
            cell_w,
            cell_h,
            padding: pad,
            gutter,
            rows,
            cols,
            display_offset,
            viewport_anchor_row,
            visual_y_offset,
        })
    }

    /// Capture painted bounds for hit-testing; called from a canvas prepaint under the output area.
    pub(in crate::features) fn remember_terminal_surface_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.terminal_surface_bounds = Some(bounds);
    }

    /// Capture painted bounds for a specific terminal pane and keep that pane's
    /// terminal model/backend PTY sized to its own viewport.
    pub(in crate::features) fn remember_terminal_surface_bounds_for_session(
        &mut self,
        session_id: Option<&str>,
        bounds: Bounds<Pixels>,
    ) -> bool {
        let session_id = session_id.filter(|id| !id.is_empty());
        if session_id.is_none() || session_id == self.active_session_id.as_deref() {
            self.remember_terminal_surface_bounds(bounds);
        }
        if let Some(session_id) = session_id {
            self.terminal_session_surface_bounds
                .insert(session_id.to_string(), bounds);
            self.resize_terminal_to_bounds_for_session(Some(session_id), bounds)
        } else {
            self.resize_terminal_to_bounds_for_session(None, bounds)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::features) struct TerminalHitTestGeometry {
    pub(in crate::features) bounds: Bounds<Pixels>,
    pub(in crate::features) cell_w: f32,
    pub(in crate::features) cell_h: f32,
    pub(in crate::features) padding: f32,
    pub(in crate::features) gutter: f32,
    pub(in crate::features) rows: usize,
    pub(in crate::features) cols: usize,
    pub(in crate::features) display_offset: usize,
    pub(in crate::features) viewport_anchor_row: usize,
    pub(in crate::features) visual_y_offset: f32,
}

pub(in crate::features) fn terminal_cell_for_visual_geometry(
    position: Point<Pixels>,
    geometry: &TerminalHitTestGeometry,
) -> TerminalCellPos {
    let cell_w = geometry.cell_w.max(1.0);
    let cell_h = geometry.cell_h.max(1.0);
    let local_x =
        f32::from(position.x - geometry.bounds.origin.x) - geometry.padding - geometry.gutter;
    let local_y = f32::from(position.y - geometry.bounds.origin.y) - geometry.padding;
    let snapshot_row = ((local_y - geometry.visual_y_offset) / cell_h)
        .floor()
        .max(0.0) as usize;
    let row = snapshot_row.saturating_sub(geometry.viewport_anchor_row);
    let col = (local_x / cell_w).floor().max(0.0) as usize;
    TerminalCellPos::new(
        row.min(geometry.rows.saturating_sub(1)),
        col.min(geometry.cols.saturating_sub(1)),
    )
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(in crate::features) struct TerminalGutterMetrics {
    pub timestamp_width: f32,
    pub line_number_width: f32,
    pub gap_width: f32,
    pub trailing_padding_width: f32,
}

impl TerminalGutterMetrics {
    pub(in crate::features) fn total_width(self) -> f32 {
        self.timestamp_width + self.line_number_width + self.gap_width + self.trailing_padding_width
    }
}

pub(in crate::features) fn terminal_gutter_metrics(
    cell_width: f32,
    font_size: f32,
    show_timestamps: bool,
    show_timestamp_ms: bool,
    show_line_numbers: bool,
) -> TerminalGutterMetrics {
    let font_size = font_size.max(8.0);
    let gutter_font = (font_size * 0.85).max(8.0);
    // Gutter text is painted at 0.85x terminal font; derive its column width
    // from the measured terminal cell so hit-testing, resize, and paint agree.
    let gutter_cell_w = (cell_width.max(1.0) * (gutter_font / font_size)).max(4.0);
    let timestamp_width = if show_timestamps {
        let cols = if show_timestamp_ms { 12.0 } else { 8.0 };
        (gutter_cell_w * cols + 2.0).max(if show_timestamp_ms { 96.0 } else { 72.0 })
    } else {
        0.0
    };
    let line_number_width = if show_line_numbers {
        (gutter_cell_w * 5.0 + 2.0).max(40.0)
    } else {
        0.0
    };
    let gap_width = if show_timestamps && show_line_numbers {
        4.0
    } else {
        0.0
    };
    let trailing_padding_width = if timestamp_width > 0.0 || line_number_width > 0.0 {
        4.0
    } else {
        0.0
    };

    TerminalGutterMetrics {
        timestamp_width,
        line_number_width,
        gap_width,
        trailing_padding_width,
    }
}

pub(in crate::features) fn terminal_snapshot_row_for_viewport_row(
    snapshot: &nyaterm_terminal::TerminalSnapshot,
    display_offset: usize,
    viewport_rows: usize,
    scrollback_len: usize,
    viewport_row: usize,
) -> Option<usize> {
    if viewport_row >= viewport_rows.max(1) {
        return None;
    }
    let anchor = terminal_snapshot_anchor_row_for_display_offset(
        snapshot,
        display_offset,
        viewport_rows,
        scrollback_len,
    );
    anchor
        .checked_add(viewport_row)
        .filter(|row| *row < snapshot.lines.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terminal_output_lines(count: usize) -> String {
        (0..count)
            .map(|index| format!("line {index:03}\n"))
            .collect::<String>()
    }

    #[test]
    fn snapshot_row_mapping_anchors_viewport_rows_inside_retained_snapshot() {
        let mut screen = nyaterm_terminal::TerminalScreen::default();
        screen.advance_decoded_text(&terminal_output_lines(80));
        let base = screen.viewport_snapshot(0);
        let viewport_rows = base.rows.max(1);
        let older = screen.viewport_snapshot(viewport_rows);
        let retained_older_rows = older.rows.min(viewport_rows);
        assert!(retained_older_rows > 0);

        let mut snapshot = base.clone();
        let mut lines = older
            .lines
            .into_iter()
            .take(retained_older_rows)
            .collect::<Vec<_>>();
        lines.extend(snapshot.lines);
        snapshot.lines = lines;
        snapshot.rows = snapshot.rows.saturating_add(retained_older_rows);

        let first_visible_row = terminal_snapshot_row_for_viewport_row(
            &snapshot,
            0,
            viewport_rows,
            snapshot.scrollback_len,
            0,
        );
        let last_visible_row = terminal_snapshot_row_for_viewport_row(
            &snapshot,
            0,
            viewport_rows,
            snapshot.scrollback_len,
            viewport_rows.saturating_sub(1),
        );

        assert_eq!(first_visible_row, Some(retained_older_rows));
        assert_eq!(snapshot.lines[first_visible_row.unwrap()], base.lines[0],);
        assert_eq!(
            snapshot.lines[last_visible_row.unwrap()],
            base.lines[viewport_rows.saturating_sub(1)],
        );
    }
    #[test]
    fn gutter_metrics_use_same_widths_for_ms_timestamps_and_total_hit_area() {
        let metrics = terminal_gutter_metrics(8.0, 14.0, true, true, true);

        assert_eq!(metrics.timestamp_width, 96.0);
        assert_eq!(metrics.line_number_width, 40.0);
        assert_eq!(metrics.gap_width, 4.0);
        assert_eq!(metrics.trailing_padding_width, 4.0);
        assert_eq!(metrics.total_width(), 144.0);
    }

    #[test]
    fn gutter_metrics_expand_with_large_terminal_font() {
        let metrics = terminal_gutter_metrics(18.0, 28.0, true, false, true);

        assert!(metrics.timestamp_width > 120.0);
        assert!(metrics.line_number_width > 70.0);
        assert_eq!(
            metrics.total_width(),
            metrics.timestamp_width
                + metrics.line_number_width
                + metrics.gap_width
                + metrics.trailing_padding_width
        );
    }

    #[test]
    fn terminal_visual_hit_test_maps_through_snapshot_anchor() {
        let geometry = TerminalHitTestGeometry {
            bounds: gpui::bounds(
                Point {
                    x: px(10.0),
                    y: px(20.0),
                },
                Size {
                    width: px(400.0),
                    height: px(300.0),
                },
            ),
            cell_w: 8.0,
            cell_h: 16.0,
            padding: 4.0,
            gutter: 12.0,
            rows: 24,
            cols: 80,
            display_offset: 10,
            viewport_anchor_row: 5,
            visual_y_offset: -80.0,
        };

        let cell = terminal_cell_for_visual_geometry(
            Point {
                x: px(10.0 + 4.0 + 12.0 + 3.0 * 8.0),
                y: px(20.0 + 4.0 + 10.0 * 16.0),
            },
            &geometry,
        );

        assert_eq!(cell, TerminalCellPos::new(10, 3));
    }

    #[test]
    fn terminal_visual_hit_test_follows_fractional_visual_offset() {
        let geometry = TerminalHitTestGeometry {
            bounds: gpui::bounds(
                Point {
                    x: px(0.0),
                    y: px(0.0),
                },
                Size {
                    width: px(400.0),
                    height: px(300.0),
                },
            ),
            cell_w: 8.0,
            cell_h: 16.0,
            padding: 0.0,
            gutter: 0.0,
            rows: 24,
            cols: 80,
            display_offset: 3,
            viewport_anchor_row: 5,
            visual_y_offset: -72.0,
        };

        let cell = terminal_cell_for_visual_geometry(
            Point {
                x: px(4.0 * 8.0),
                y: px(10.0 * 16.0),
            },
            &geometry,
        );

        assert_eq!(cell, TerminalCellPos::new(9, 4));
    }
}
