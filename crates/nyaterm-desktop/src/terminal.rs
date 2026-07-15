pub(crate) use nyaterm_terminal_gpui::{
    NyaTerminalElement, NyaTerminalLayoutCache, TerminalBufferMatch, TerminalKeyMode,
    TerminalLineDecorations, TerminalSearchFlags, TerminalTextCell, terminal_buffer_matches,
    terminal_byte_index_for_cell_col, terminal_cell_count, terminal_is_zero_width_mark,
    terminal_key_bytes_with_mode, terminal_key_release_bytes_with_mode,
    terminal_screen_from_output, terminal_text_cell_slice, terminal_text_cells,
    trim_terminal_output,
};

pub(crate) fn initial_terminal_screen() -> nyaterm_terminal::TerminalScreen {
    nyaterm_terminal_gpui::initial_terminal_screen(crate::features::INITIAL_TERMINAL_BANNER)
}
