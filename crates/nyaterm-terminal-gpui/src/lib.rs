//! GPUI terminal painting helpers (element + line rendering).

mod types;

mod ansi;
mod element;
mod images;
mod input;
mod keywords;
mod paint;

fn resolve_cell_fg(palette: nyaterm_ui::ThemePalette, style: nyaterm_terminal::CellStyle) -> u32 {
    if style.reverse {
        if let Some(rgb) = style.bg_rgb {
            return rgb;
        }
        return style
            .bg
            .map(|index| palette.terminal_ansi_color(index))
            .unwrap_or(palette.terminal_bg);
    }
    if let Some(rgb) = style.fg_rgb {
        return rgb;
    }
    match style.fg {
        Some(index) if style.bold && index < 8 => palette.terminal_ansi_color(index + 8),
        Some(index) => palette.terminal_ansi_color(index),
        None => palette.terminal_fg,
    }
}

fn resolve_cell_bg(
    palette: nyaterm_ui::ThemePalette,
    style: nyaterm_terminal::CellStyle,
) -> Option<u32> {
    if style.reverse {
        if let Some(rgb) = style.fg_rgb {
            return Some(rgb);
        }
        return Some(match style.fg {
            Some(index) if style.bold && index < 8 => palette.terminal_ansi_color(index + 8),
            Some(index) => palette.terminal_ansi_color(index),
            None => palette.terminal_fg,
        });
    }
    style
        .bg_rgb
        .or_else(|| style.bg.map(|index| palette.terminal_ansi_color(index)))
}

pub use element::{
    NyaTerminalElement, NyaTerminalLayoutCache, TerminalBufferMatch, TerminalGridSelection,
    TerminalLineDecorations, TerminalSearchFlags,
};
pub use input::{
    TerminalKeyMode, initial_terminal_screen, terminal_key_bytes, terminal_key_bytes_with_mode,
    terminal_key_release_bytes_with_mode, terminal_screen_from_output, trim_terminal_output,
};
pub use keywords::terminal_buffer_matches;
pub use keywords::{
    TerminalKeywordHighlightPrecomputeStats, TerminalKeywordHighlightSnapshot,
    TerminalKeywordHighlighter, compile_terminal_keyword_highlighter,
    precompute_terminal_keyword_highlights, precompute_terminal_keyword_highlights_for_rows,
    precompute_terminal_keyword_highlights_for_rows_with_stats,
    precompute_terminal_keyword_highlights_for_rows_with_stats_and_cancel,
    terminal_keyword_highlight_expanded_rows, terminal_keyword_rules_key,
};

#[cfg(test)]
mod tests;
