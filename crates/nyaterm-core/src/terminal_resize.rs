#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalResizeGeometry {
    pub cols: u16,
    pub rows: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalBackendResize {
    pub cols: u16,
    pub rows: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl TerminalBackendResize {
    pub fn new(cols: u16, rows: u16, pixel_width: u16, pixel_height: u16) -> Self {
        Self {
            cols,
            rows,
            pixel_width,
            pixel_height,
        }
    }
}

pub fn terminal_resize_geometry_for_size(
    width: f32,
    height: f32,
    cell_width: f32,
    cell_height: f32,
    padding: f32,
    gutter_width: f32,
) -> TerminalResizeGeometry {
    let cell_width = cell_width.max(1.);
    let cell_height = cell_height.max(1.);
    let usable_width = (width - padding * 2. - gutter_width).max(cell_width);
    let usable_height = (height - padding * 2.).max(cell_height);
    let raw_cols = (usable_width / cell_width).floor();
    let raw_rows = (usable_height / cell_height).floor();
    let clamped_cols = raw_cols.clamp(20., 500.);
    let clamped_rows = raw_rows.clamp(4., 200.);
    let cols = clamped_cols as u16;
    let rows = clamped_rows as u16;
    let pixel_width = if (raw_cols - clamped_cols).abs() < f32::EPSILON {
        usable_width
    } else {
        clamped_cols * cell_width
    };
    let pixel_height = if (raw_rows - clamped_rows).abs() < f32::EPSILON {
        usable_height
    } else {
        clamped_rows * cell_height
    };
    let pixel_width = pixel_width.round().clamp(1., u16::MAX as f32) as u16;
    let pixel_height = pixel_height.round().clamp(1., u16::MAX as f32) as u16;
    TerminalResizeGeometry {
        cols,
        rows,
        pixel_width,
        pixel_height,
    }
}

pub fn terminal_backend_resize_changed(
    last: Option<TerminalBackendResize>,
    next: TerminalBackendResize,
) -> bool {
    last != Some(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_geometry_keeps_usable_pixel_remainder() {
        let geometry = terminal_resize_geometry_for_size(812., 612., 10., 20., 8., 72.);

        assert_eq!(
            geometry,
            TerminalResizeGeometry {
                cols: 72,
                rows: 29,
                pixel_width: 724,
                pixel_height: 596,
            }
        );
    }

    #[test]
    fn resize_geometry_clamps_grid_but_keeps_nonzero_pixels() {
        let geometry = terminal_resize_geometry_for_size(10., 10., 10., 20., 8., 72.);

        assert_eq!(geometry.cols, 20);
        assert_eq!(geometry.rows, 4);
        assert_eq!(geometry.pixel_width, 200);
        assert_eq!(geometry.pixel_height, 80);
    }

    #[test]
    fn backend_resize_detects_pixel_only_changes() {
        let first = TerminalBackendResize::new(80, 24, 800, 432);

        assert!(terminal_backend_resize_changed(None, first));
        assert!(!terminal_backend_resize_changed(Some(first), first));
        assert!(terminal_backend_resize_changed(
            Some(first),
            TerminalBackendResize::new(80, 24, 960, 432)
        ));
        assert!(terminal_backend_resize_changed(
            Some(first),
            TerminalBackendResize::new(80, 25, 800, 450)
        ));
    }
}
