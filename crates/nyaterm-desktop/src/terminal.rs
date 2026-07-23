pub(crate) use nyaterm_terminal_gpui::{
    NyaTerminalElement, NyaTerminalLayoutCache, TerminalBufferMatch, TerminalKeyMode,
    TerminalKeywordHighlightSnapshot, TerminalKeywordHighlighter, TerminalLineDecorations,
    TerminalSearchFlags, TerminalTextCell, compile_terminal_keyword_highlighter,
    precompute_terminal_keyword_highlights, terminal_buffer_matches,
    terminal_byte_index_for_cell_col, terminal_is_zero_width_mark, terminal_key_bytes_with_mode,
    terminal_key_release_bytes_with_mode, terminal_screen_from_output, terminal_text_cell_slice,
    terminal_text_cells,
};

pub(crate) fn initial_terminal_screen() -> nyaterm_terminal::TerminalScreen {
    nyaterm_terminal_gpui::initial_terminal_screen(crate::features::INITIAL_TERMINAL_BANNER)
}
