//! GPUI terminal painting helpers (element + line rendering).

// Shared imports for submodules (`use super::*`).
pub use gpui::{
    App, Bounds, Element, ElementId, Font, FontStyle, FontWeight, GlobalElementId, Hsla,
    InspectorElementId, IntoElement, KeyDownEvent, LayoutId, PaintQuad, Pixels, ShapedLine,
    SharedString, StrikethroughStyle, Style, TextRun, UnderlineStyle, Window, div, fill, font,
    point, prelude::*, px, relative, rgb, size,
};
pub use nyaterm_core::ResolvedKeywordHighlightRule;
pub use nyaterm_terminal::{TerminalScreen, TerminalSnapshot};

mod types;
use types::*;

mod ansi;
mod element;
mod input;
mod keywords;
mod paint;

// Re-export helpers so sibling modules resolve them via `use super::*`.
use ansi::*;
pub use keywords::*;
pub use paint::*;

pub use element::{
    NyaTerminalElement, TerminalBufferMatch, TerminalLineDecorations, TerminalSearchFlags,
};
pub use input::{
    initial_terminal_screen, terminal_key_bytes, terminal_screen_from_output, trim_terminal_output,
};
pub use keywords::terminal_buffer_matches;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
