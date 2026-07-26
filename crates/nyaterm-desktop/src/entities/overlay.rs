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
}

impl OverlayStore {
    pub fn quick_switch(&self) -> &QuickSwitchState {
        &self.quick_switch
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
}
