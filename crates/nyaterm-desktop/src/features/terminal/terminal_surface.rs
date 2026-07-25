use super::*;

pub(in crate::features) const TERMINAL_SCROLLBAR_COLUMN_WIDTH: f32 = 10.0;

#[path = "terminal_surface/helpers.rs"]
mod helpers;
use helpers::*;

#[path = "terminal_surface/decorations.rs"]
mod decorations;
pub(in crate::features) use decorations::{
    build_terminal_line_decorations, terminal_action_links_cover_all_snapshot_rows,
    terminal_action_links_for_paint_snapshot, terminal_action_links_have_ranges_for_snapshot,
    terminal_action_links_overlap_snapshot, terminal_line_decorations_cache_key,
    terminal_line_decorations_needed, terminal_snapshot_absolute_range,
};

#[path = "terminal_surface/canvas.rs"]
mod canvas;
#[path = "terminal_surface/chrome.rs"]
mod chrome;
