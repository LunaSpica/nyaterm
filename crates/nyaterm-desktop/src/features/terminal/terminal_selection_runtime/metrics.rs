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
        // Keep in sync with terminal_surface gutter column widths.
        let (cell_w, _) = self.terminal_cell_size();
        let gutter_font = (self.settings.terminal_font_size.max(8) as f32 * 0.85).max(8.);
        // Gutter text uses 0.85x terminal font; approximate char width proportionally.
        let gutter_cell_w =
            (cell_w * (gutter_font / self.settings.terminal_font_size.max(8) as f32)).max(4.);
        let mut width = 0.;
        if self.settings.terminal_show_timestamps {
            // HH:MM:SS = 8 chars, HH:MM:SS.mmm = 12 chars (+ small pad like Tauri).
            let cols = if self.settings.terminal_show_timestamp_milliseconds {
                12.
            } else {
                8.
            };
            width += (gutter_cell_w * cols + 2.).max(
                if self.settings.terminal_show_timestamp_milliseconds {
                    96.
                } else {
                    72.
                },
            );
        }
        if self.settings.terminal_show_line_numbers {
            // 5-digit absolute line numbers + pad.
            width += (gutter_cell_w * 5. + 2.).max(40.);
        }
        if self.settings.terminal_show_timestamps && self.settings.terminal_show_line_numbers {
            width += 4.; // gap_1
        }
        if width > 0. {
            width += 4.; // pr_1 trailing gutter padding
        }
        width
    }

    pub(in crate::features) fn terminal_timestamp_gutter_width_px(&self) -> f32 {
        let (cell_w, _) = self.terminal_cell_size();
        let gutter_font = (self.settings.terminal_font_size.max(8) as f32 * 0.85).max(8.);
        let gutter_cell_w =
            (cell_w * (gutter_font / self.settings.terminal_font_size.max(8) as f32)).max(4.);
        let cols = if self.settings.terminal_show_timestamp_milliseconds {
            12.
        } else {
            8.
        };
        (gutter_cell_w * cols + 2.).max(if self.settings.terminal_show_timestamp_milliseconds {
            96.
        } else {
            72.
        })
    }

    pub(in crate::features) fn terminal_line_number_gutter_width_px(&self) -> f32 {
        let (cell_w, _) = self.terminal_cell_size();
        let gutter_font = (self.settings.terminal_font_size.max(8) as f32 * 0.85).max(8.);
        let gutter_cell_w =
            (cell_w * (gutter_font / self.settings.terminal_font_size.max(8) as f32)).max(4.);
        (gutter_cell_w * 5. + 2.).max(40.)
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
        let rows = snapshot.lines.len().max(1);
        let cols = snapshot
            .lines
            .iter()
            .map(|line| terminal_cell_count(line))
            .max()
            .unwrap_or(80)
            .max(80);
        (rows, cols)
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
        let session_id = session_id.filter(|id| !id.is_empty());
        let bounds = session_id
            .and_then(|id| self.terminal_session_surface_bounds.get(id).copied())
            .or(self.terminal_surface_bounds)?;
        let (cell_w, cell_h) = self.terminal_cell_size();
        let pad = self.terminal_content_padding_px();
        let gutter = self.terminal_gutter_width_px();
        let local_x = f32::from(position.x - bounds.origin.x) - pad - gutter;
        let local_y = f32::from(position.y - bounds.origin.y) - pad;
        if local_y < -cell_h || local_x < -cell_w {
            // Still allow clamping when slightly outside.
        }
        let (rows, cols) = self.terminal_grid_size_for_session(session_id);
        let row = (local_y / cell_h).floor().max(0.) as usize;
        let col = (local_x / cell_w).floor().max(0.) as usize;
        Some(TerminalCellPos::new(
            row.min(rows.saturating_sub(1)),
            col.min(cols.saturating_sub(1)),
        ))
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
