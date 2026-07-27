#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuickSwitchState {
    open: bool,
    query: String,
    selected_index: usize,
}

impl QuickSwitchState {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn query(&self) -> &str {
        &self.query
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

    pub fn set_quick_switch_query(&mut self, query: String) -> bool {
        let changed = self.quick_switch.query != query;
        let selection_changed = self.quick_switch.selected_index != 0;
        self.quick_switch.query = query;
        self.quick_switch.selected_index = 0;
        changed || selection_changed
    }
}
