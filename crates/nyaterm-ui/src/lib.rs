//! Shared GPUI theme tokens and reusable presentation widgets for NyaTerm.

mod theme;
mod widgets;

pub use theme::{APPEARANCE_THEME_IDS, ThemePalette, appearance_theme_label, theme_palette};
pub use widgets::{
    capability_line, empty_panel, icon_button, mode_button, section_header, session_info_row,
    small_button, status_pill, svg_icon_button,
};
