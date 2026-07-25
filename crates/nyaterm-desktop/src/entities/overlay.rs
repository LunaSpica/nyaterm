#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlaySnapshot {
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuickSwitchState {
    open: bool,
    query: String,
    marked_text: String,
    selected_index: usize,
}

impl QuickSwitchState {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn marked_text(&self) -> &str {
        &self.marked_text
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }
}

#[derive(Debug, Default)]
pub struct OverlayStore {
    quick_switch: QuickSwitchState,
    snapshot: Option<OverlaySnapshot>,
}

impl OverlayStore {
    pub fn quick_switch(&self) -> &QuickSwitchState {
        &self.quick_switch
    }

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

    pub fn open_quick_switch(&mut self) -> bool {
        let next = QuickSwitchState {
            open: true,
            ..QuickSwitchState::default()
        };
        if self.quick_switch == next {
            return false;
        }
        self.quick_switch = next;
        true
    }

    pub fn close_quick_switch(&mut self) -> bool {
        if self.quick_switch == QuickSwitchState::default() {
            return false;
        }
        self.quick_switch = QuickSwitchState::default();
        true
    }

    pub fn reset_quick_switch_input_and_close(&mut self) -> bool {
        self.close_quick_switch()
    }

    pub fn set_quick_switch_selected_index(&mut self, selected_index: usize) -> bool {
        if self.quick_switch.selected_index == selected_index {
            return false;
        }
        self.quick_switch.selected_index = selected_index;
        true
    }

    pub fn clamp_quick_switch_selected_index(&mut self, item_count: usize) -> bool {
        if item_count == 0 || self.quick_switch.selected_index < item_count {
            return false;
        }
        self.set_quick_switch_selected_index(item_count - 1)
    }

    pub fn push_quick_switch_query(&mut self, input: &str) -> bool {
        if input.is_empty() {
            return false;
        }
        self.quick_switch.query.push_str(input);
        self.quick_switch.selected_index = 0;
        true
    }

    pub fn pop_quick_switch_query(&mut self) -> bool {
        let changed = self.quick_switch.query.pop().is_some();
        let selection_changed = self.quick_switch.selected_index != 0;
        self.quick_switch.selected_index = 0;
        changed || selection_changed
    }

    pub fn set_quick_switch_marked_text(&mut self, marked_text: impl Into<String>) -> bool {
        let marked_text = marked_text.into();
        if self.quick_switch.marked_text == marked_text {
            return false;
        }
        self.quick_switch.marked_text = marked_text;
        true
    }

    pub fn clear_quick_switch_marked_text(&mut self) -> bool {
        self.set_quick_switch_marked_text(String::new())
    }

    pub fn replace_quick_switch_text(&mut self, text: &str) -> bool {
        let had_marked_text = !self.quick_switch.marked_text.is_empty();
        self.quick_switch.marked_text.clear();
        if text.is_empty() {
            return had_marked_text;
        }
        self.quick_switch.query.push_str(text);
        self.quick_switch.selected_index = 0;
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
