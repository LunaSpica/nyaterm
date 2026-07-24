//! GPUI terminal painting helpers (element + line rendering).

// Shared imports for submodules (`use super::*`).
pub use gpui::{
    App, Bounds, ContentMask, Element, ElementId, Font, FontStyle, FontWeight, GlobalElementId,
    Hsla, InspectorElementId, IntoElement, KeyDownEvent, KeyUpEvent, LayoutId, PaintQuad, Pixels,
    ShapedLine, SharedString, StrikethroughStyle, Style, TextRun, UnderlineStyle, Window, div,
    fill, font, point, prelude::*, px, relative, rgb, size,
};
pub use nyaterm_core::ResolvedKeywordHighlightRule;
pub use nyaterm_terminal::{
    ShellCommandMark, TerminalScreen, TerminalSnapshot, TerminalTextCell,
    alternate_scroll_key_bytes, terminal_byte_index_for_cell_col, terminal_cell_col_for_byte_index,
    terminal_cell_count, terminal_char_cell_width, terminal_is_zero_width_mark,
    terminal_text_cell_slice, terminal_text_cells,
};

mod types;
use types::*;

mod ansi;
mod element;
mod images;
mod input;
mod keywords;
mod paint;

// Re-export helpers so sibling modules resolve them via `use super::*`.
use ansi::*;
pub use images::*;
pub use keywords::*;
pub use paint::*;

pub use element::{
    NyaTerminalElement, NyaTerminalLayoutCache, TerminalBufferMatch, TerminalLineDecorations,
    TerminalSearchFlags,
};
pub use input::{
    TerminalKeyMode, initial_terminal_screen, terminal_key_bytes, terminal_key_bytes_with_mode,
    terminal_key_release_bytes_with_mode, terminal_screen_from_output, trim_terminal_output,
};
pub use keywords::terminal_buffer_matches;
pub use keywords::{
    TerminalKeywordHighlightSnapshot, TerminalKeywordHighlighter,
    compile_terminal_keyword_highlighter, precompute_terminal_keyword_highlights,
    precompute_terminal_keyword_highlights_for_rows,
};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
