pub(crate) use nyaterm_terminal_gpui::{
    NyaTerminalElement, TerminalBufferMatch, TerminalLineDecorations, TerminalSearchFlags,
    terminal_buffer_matches, terminal_key_bytes, terminal_screen_from_output, trim_terminal_output,
};

pub(crate) fn initial_terminal_screen() -> nyaterm_terminal::TerminalScreen {
    nyaterm_terminal_gpui::initial_terminal_screen(crate::features::INITIAL_TERMINAL_BANNER)
}
