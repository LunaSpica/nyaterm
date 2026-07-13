#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlaySnapshot {
    pub quick_switch_open: bool,
    pub tab_actions_open: bool,
    pub rename_open: bool,
    pub color_picker_open: bool,
    pub session_info_open: bool,
    pub startup_command_open: bool,
    pub temporary_ssh_link_open: bool,
    pub multi_line_paste_open: bool,
    pub terminal_actions_open: bool,
    pub terminal_context_menu_open: bool,
    pub action_link_menu_open: bool,
    pub action_link_tooltip_open: bool,
    pub command_suggestions_open: bool,
    pub credential_suggestions_open: bool,
    pub close_all_sessions_confirm_open: bool,
    pub locked: bool,
}

impl Default for OverlaySnapshot {
    fn default() -> Self {
        Self {
            quick_switch_open: false,
            tab_actions_open: false,
            rename_open: false,
            color_picker_open: false,
            session_info_open: false,
            startup_command_open: false,
            temporary_ssh_link_open: false,
            multi_line_paste_open: false,
            terminal_actions_open: false,
            terminal_context_menu_open: false,
            action_link_menu_open: false,
            action_link_tooltip_open: false,
            command_suggestions_open: false,
            credential_suggestions_open: false,
            close_all_sessions_confirm_open: false,
            locked: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct OverlayStore {
    snapshot: Option<OverlaySnapshot>,
}

impl OverlayStore {
    pub fn snapshot(&self) -> Option<&OverlaySnapshot> {
        self.snapshot.as_ref()
    }

    pub fn replace_snapshot(&mut self, snapshot: OverlaySnapshot) -> bool {
        if self.snapshot.as_ref() == Some(&snapshot) {
            return false;
        }
        self.snapshot = Some(snapshot);
        true
    }

    pub fn set_quick_switch_open(&mut self, open: bool) -> bool {
        let snapshot = self.snapshot.get_or_insert_with(OverlaySnapshot::default);
        if snapshot.quick_switch_open == open {
            return false;
        }
        snapshot.quick_switch_open = open;
        true
    }

    pub fn set_tab_actions_open(&mut self, open: bool) -> bool {
        let snapshot = self.snapshot.get_or_insert_with(OverlaySnapshot::default);
        if snapshot.tab_actions_open == open {
            return false;
        }
        snapshot.tab_actions_open = open;
        true
    }

    pub fn set_terminal_context_menu_open(&mut self, open: bool) -> bool {
        let snapshot = self.snapshot.get_or_insert_with(OverlaySnapshot::default);
        if snapshot.terminal_context_menu_open == open {
            return false;
        }
        snapshot.terminal_context_menu_open = open;
        if open {
            snapshot.action_link_menu_open = false;
        }
        true
    }

    pub fn set_action_link_menu_open(&mut self, open: bool) -> bool {
        let snapshot = self.snapshot.get_or_insert_with(OverlaySnapshot::default);
        if snapshot.action_link_menu_open == open {
            return false;
        }
        snapshot.action_link_menu_open = open;
        if open {
            snapshot.terminal_context_menu_open = false;
        }
        true
    }

    pub fn set_locked(&mut self, locked: bool) -> bool {
        let snapshot = self.snapshot.get_or_insert_with(OverlaySnapshot::default);
        if snapshot.locked == locked {
            return false;
        }
        snapshot.locked = locked;
        true
    }
}
