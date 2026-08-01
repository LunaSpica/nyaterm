//! Terminal input, selection, search, painting surface and view runtime.

use gpui::{App, KeyBinding, actions};

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

pub(in crate::features) const TERMINAL_KEY_CONTEXT: &str = "Terminal";

actions!(terminal, [TerminalTab, TerminalShiftTab]);

pub(in crate::features) fn init_key_bindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", TerminalTab, Some(TERMINAL_KEY_CONTEXT)),
        KeyBinding::new("shift-tab", TerminalShiftTab, Some(TERMINAL_KEY_CONTEXT)),
    ]);
}

pub(in crate::features) use state::{TerminalFeatureFocus, TerminalFeatureState};
pub(in crate::features) use terminal_surface_entity::{
    FULL_SHELL_PAINT_COUNT, full_shell_paint_count, terminal_surface_paint_count,
};
pub(in crate::features) use window_state::{
    TerminalWindowDockResult, TerminalWindowReconcileResult,
};

#[cfg(test)]
mod tests {
    use gpui::{KeyBinding, KeyContext, Keymap, actions};

    use super::{TERMINAL_KEY_CONTEXT, TerminalShiftTab, TerminalTab};

    actions!(terminal_test, [RootTab, RootShiftTab]);

    #[test]
    fn terminal_tab_bindings_shadow_root_focus_navigation() {
        let mut keymap = Keymap::default();
        keymap.add_bindings([
            KeyBinding::new("tab", RootTab, Some("Root")),
            KeyBinding::new("shift-tab", RootShiftTab, Some("Root")),
            KeyBinding::new("tab", TerminalTab, Some(TERMINAL_KEY_CONTEXT)),
            KeyBinding::new("shift-tab", TerminalShiftTab, Some(TERMINAL_KEY_CONTEXT)),
        ]);
        let contexts = [
            KeyContext::parse("Root").unwrap(),
            KeyContext::parse(TERMINAL_KEY_CONTEXT).unwrap(),
        ];

        let (tab_bindings, tab_pending) =
            keymap.bindings_for_input(&[gpui::Keystroke::parse("tab").unwrap()], &contexts);
        let (shift_tab_bindings, shift_tab_pending) =
            keymap.bindings_for_input(&[gpui::Keystroke::parse("shift-tab").unwrap()], &contexts);

        assert!(!tab_pending);
        assert!(!shift_tab_pending);
        assert!(tab_bindings[0].action().partial_eq(&TerminalTab));
        assert!(shift_tab_bindings[0].action().partial_eq(&TerminalShiftTab));
    }
}
