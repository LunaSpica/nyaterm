//! GPUI terminal painting helpers (element + line rendering).

// Shared imports for submodules (`use super::*`).
pub(crate) use gpui::{
    App, Bounds, Element, ElementId, Font, FontStyle, FontWeight, GlobalElementId, Hsla,
    InspectorElementId, IntoElement, KeyDownEvent, LayoutId, PaintQuad, Pixels, ShapedLine,
    SharedString, StrikethroughStyle, Style, TextRun, UnderlineStyle, Window, div, fill, font,
    point, prelude::*, px, relative, rgb, size,
};
pub(crate) use nyaterm_core::ResolvedKeywordHighlightRule;
pub(crate) use nyaterm_terminal::{TerminalScreen, TerminalSnapshot};

pub(crate) use crate::features::INITIAL_TERMINAL_BANNER;

mod types;
use types::*;

mod paint;
mod ansi;
mod keywords;
mod input;
mod element;

// Re-export helpers so sibling modules resolve them via `use super::*`.
pub(crate) use paint::*;
use ansi::*;
pub(crate) use keywords::*;

pub(crate) use element::{
    NyaTerminalElement, TerminalBufferMatch, TerminalLineDecorations, TerminalSearchFlags,
};
pub(crate) use input::{
    initial_terminal_screen, terminal_key_bytes, terminal_screen_from_output, trim_terminal_output,
};
pub(crate) use keywords::terminal_buffer_matches;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
