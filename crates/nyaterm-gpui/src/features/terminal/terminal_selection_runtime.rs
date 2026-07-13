use super::*;
use gpui::{Bounds, ClickEvent, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point};

/// Approximate monospaced cell metrics used for hit-testing the painted terminal grid.
/// Keep in sync with `terminal_line_element` row height and surface font size.
const CELL_WIDTH_RATIO: f32 = 0.62;
const LINE_HEIGHT_RATIO: f32 = 1.25;


#[path = "terminal_selection_runtime/helpers.rs"]
mod helpers;
pub(in crate::features) use helpers::*;

#[path = "terminal_selection_runtime/metrics.rs"]
mod metrics;
#[path = "terminal_selection_runtime/selection.rs"]
mod selection;
#[path = "terminal_selection_runtime/action_links.rs"]
mod action_links;
#[path = "terminal_selection_runtime/smart_input.rs"]
mod smart_input;
