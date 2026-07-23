use super::*;

pub(in crate::features) const TERMINAL_INPUT_LATENCY_WINDOW: Duration = Duration::from_millis(80);

#[path = "terminal_runtime/buffer.rs"]
mod buffer;
#[path = "terminal_runtime/paste.rs"]
mod paste;
#[path = "terminal_runtime/scroll.rs"]
mod scroll;
pub(in crate::features) use scroll::{
    TERMINAL_USER_SCROLL_ACTIVE_WINDOW, TerminalScrollVisualState,
    terminal_display_offset_from_state, terminal_local_scroll_delta_lines_from_state,
    terminal_scroll_needs_text_first_repaint, terminal_scroll_track_ratio,
    terminal_visual_scroll_active_for_state,
};
#[path = "terminal_runtime/sessions.rs"]
mod sessions;
#[path = "terminal_runtime/view_io.rs"]
mod view_io;
pub(in crate::features) use view_io::terminal_visual_display_offset;
