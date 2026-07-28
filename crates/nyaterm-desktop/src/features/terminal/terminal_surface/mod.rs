pub(in crate::features) const TERMINAL_SCROLLBAR_COLUMN_WIDTH: f32 = 10.0;

mod decorations;
mod helpers;
pub(in crate::features) use decorations::{
    build_terminal_line_decorations, terminal_action_links_cover_all_snapshot_rows,
    terminal_action_links_for_paint_snapshot, terminal_action_links_have_ranges_for_snapshot,
    terminal_action_links_overlap_snapshot, terminal_line_decorations_cache_key,
    terminal_line_decorations_needed, terminal_snapshot_absolute_range,
};

mod canvas;
mod chrome;
