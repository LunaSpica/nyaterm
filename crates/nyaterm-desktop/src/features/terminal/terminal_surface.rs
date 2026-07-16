use super::*;

#[path = "terminal_surface/helpers.rs"]
mod helpers;
use helpers::*;

#[path = "terminal_surface/decorations.rs"]
mod decorations;
pub(in crate::features) use decorations::{
    build_terminal_line_decorations, terminal_line_decorations_cache_key,
    terminal_line_decorations_needed, terminal_snapshot_absolute_range,
};

#[path = "terminal_surface/canvas.rs"]
mod canvas;
#[path = "terminal_surface/chrome.rs"]
mod chrome;
