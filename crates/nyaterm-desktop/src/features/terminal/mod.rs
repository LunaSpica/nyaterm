//! Terminal input, selection, search, painting surface and view runtime.

mod assist_state;
mod command_suggestions;
mod credential_autofill;
mod input_runtime;
mod send_command_runtime;
mod state;
mod terminal_context_menu_runtime;
pub(in crate::features) mod terminal_runtime;
mod terminal_search_runtime;
mod terminal_selection_runtime;
mod terminal_surface;
mod terminal_surface_entity;
mod view_state;
mod window_state;

pub(in crate::features) use state::{TerminalFeatureFocus, TerminalFeatureState};
pub(in crate::features) use terminal_surface_entity::{
    FULL_SHELL_PAINT_COUNT, full_shell_paint_count, terminal_surface_paint_count,
};
pub(in crate::features) use window_state::{
    TerminalWindowDockResult, TerminalWindowReconcileResult,
};
