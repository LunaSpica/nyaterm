//! GPUI terminal painting helpers (element + line rendering).

mod types;

mod ansi;
mod element;
mod images;
mod input;
mod keywords;
mod paint;

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
    precompute_terminal_keyword_highlights_for_rows, terminal_keyword_highlight_expanded_rows,
    terminal_keyword_rules_key,
};

#[cfg(test)]
mod tests;
