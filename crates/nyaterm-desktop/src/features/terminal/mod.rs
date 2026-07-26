//! Terminal input, selection, search, painting surface and view runtime.

use super::*;

mod input_runtime;
mod send_command_runtime;
mod terminal_context_menu_runtime;
pub(in crate::features) mod terminal_runtime;
mod terminal_search_runtime;
mod terminal_selection_runtime;
mod terminal_surface;
mod terminal_surface_entity;

pub(in crate::features) use terminal_selection_runtime::terminal_bounds_tracker;
pub(in crate::features) use terminal_surface::terminal_snapshot_absolute_range;
pub(in crate::features) use terminal_surface_entity::{
    FULL_SHELL_PAINT_COUNT, TerminalSurface, TerminalSurfaceHitTestScrollGeometry,
    full_shell_paint_count, terminal_effective_visual_scroll_offset_px,
    terminal_snapshot_anchor_row_for_display_offset, terminal_snapshot_covers_display_offset,
    terminal_surface_paint_count,
};
