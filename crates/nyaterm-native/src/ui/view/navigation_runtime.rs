use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn select(&mut self, item: NavItem, cx: &mut Context<Self>) {
        self.selected_nav = item;
        self.right_focus = RightFocus::Default;
        match item {
            NavItem::Stats | NavItem::Processes | NavItem::Docker | NavItem::Translation => {
                self.right_inspector_collapsed = false;
            }
            _ => {
                self.left_sidebar_collapsed = false;
            }
        }
        self.main_mode = if item == NavItem::Migration {
            MainMode::Page
        } else {
            MainMode::Workspace
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn open_page(&mut self, item: NavItem, cx: &mut Context<Self>) {
        self.selected_nav = item;
        self.right_focus = RightFocus::Default;
        self.left_sidebar_collapsed = false;
        self.main_mode = MainMode::Page;
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_left_sidebar(&mut self, cx: &mut Context<Self>) {
        self.left_sidebar_collapsed = !self.left_sidebar_collapsed;
        self.terminal_status = if self.left_sidebar_collapsed {
            "left explorer collapsed".to_string()
        } else {
            "left explorer expanded".to_string()
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_right_inspector(&mut self, cx: &mut Context<Self>) {
        self.right_inspector_collapsed = !self.right_inspector_collapsed;
        self.terminal_status = if self.right_inspector_collapsed {
            "right inspector collapsed".to_string()
        } else {
            "right inspector expanded".to_string()
        };
        cx.notify();
    }
}
