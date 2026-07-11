use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn select(&mut self, item: NavItem, cx: &mut Context<Self>) {
        self.open_panel(item, cx);
    }

    pub(in crate::ui::view) fn open_page(&mut self, item: NavItem, cx: &mut Context<Self>) {
        if item == NavItem::Settings || item.opens_settings() {
            self.main_mode = MainMode::Page;
            self.selected_nav = NavItem::Settings;
            self.left_sidebar_collapsed = true;
            self.right_inspector_collapsed = true;
            self.terminal_status = "settings opened".to_string();
            cx.notify();
            return;
        }

        if item == NavItem::Migration {
            self.open_panel(NavItem::Migration, cx);
            return;
        }

        self.open_panel(item, cx);
    }

    pub(in crate::ui::view) fn open_panel(&mut self, item: NavItem, cx: &mut Context<Self>) {
        if item == NavItem::Settings || item.opens_settings() {
            self.open_page(NavItem::Settings, cx);
            return;
        }

        if self.panel_multi_open && (item.is_left_panel() || item.is_right_panel()) {
            self.open_or_toggle_panel(item, cx);
            return;
        }

        self.main_mode = MainMode::Workspace;
        self.selected_nav = item;
        self.right_focus = if item == NavItem::Recording {
            RightFocus::Recording
        } else {
            RightFocus::Default
        };

        if item.is_left_panel() {
            let already_open = self.active_left_panel == Some(item) && !self.left_sidebar_collapsed;
            if already_open {
                self.left_sidebar_collapsed = true;
                self.active_left_panel = None;
                self.terminal_status = format!("{} closed", item.label());
            } else {
                self.active_left_panel = Some(item);
                self.left_sidebar_collapsed = false;
                self.terminal_status = format!("{} opened", item.label());
            }
        } else if item.is_right_panel() {
            let already_open =
                self.active_right_panel == Some(item) && !self.right_inspector_collapsed;
            if already_open {
                self.right_inspector_collapsed = true;
                self.active_right_panel = None;
                self.right_focus = RightFocus::Default;
                self.terminal_status = format!("{} closed", item.label());
            } else {
                self.active_right_panel = Some(item);
                self.right_inspector_collapsed = false;
                self.terminal_status = format!("{} opened", item.label());
            }
        } else {
            self.left_sidebar_collapsed = false;
            self.right_inspector_collapsed = false;
        }

        self.persist_ui_layout();
        cx.notify();
    }

    pub(in crate::ui::view) fn ensure_panel_open(&mut self, item: NavItem) {
        if self.panel_multi_open && (item.is_left_panel() || item.is_right_panel()) {
            self.ensure_panel_in_stack(item);
            return;
        }
        self.main_mode = MainMode::Workspace;
        self.selected_nav = item;
        if item.is_left_panel() {
            self.active_left_panel = Some(item);
            self.left_sidebar_collapsed = false;
        } else if item.is_right_panel() {
            self.active_right_panel = Some(item);
            self.right_inspector_collapsed = false;
            self.right_focus = if item == NavItem::Recording {
                RightFocus::Recording
            } else {
                RightFocus::Default
            };
        }
    }

    pub(in crate::ui::view) fn close_settings(&mut self, cx: &mut Context<Self>) {
        self.main_mode = MainMode::Workspace;
        if self.active_left_panel.is_none() {
            self.left_sidebar_collapsed = true;
        } else {
            self.left_sidebar_collapsed = false;
        }
        if self.active_right_panel.is_none() {
            self.right_inspector_collapsed = true;
        } else {
            self.right_inspector_collapsed = false;
        }
        self.terminal_status = "workspace restored".to_string();
        self.persist_ui_layout();
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_left_sidebar(&mut self, cx: &mut Context<Self>) {
        if self.left_sidebar_collapsed {
            if self.active_left_panel.is_none() {
                self.active_left_panel = Some(NavItem::Transfers);
            }
            self.left_sidebar_collapsed = false;
            self.terminal_status = "left sidebar expanded".to_string();
        } else {
            self.left_sidebar_collapsed = true;
            self.terminal_status = "left sidebar collapsed".to_string();
        }
        self.persist_ui_layout();
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_right_inspector(&mut self, cx: &mut Context<Self>) {
        if self.right_inspector_collapsed {
            if self.active_right_panel.is_none() {
                self.active_right_panel = Some(NavItem::Connections);
            }
            self.right_inspector_collapsed = false;
            self.terminal_status = "right sidebar expanded".to_string();
        } else {
            self.right_inspector_collapsed = true;
            self.terminal_status = "right sidebar collapsed".to_string();
        }
        self.persist_ui_layout();
        cx.notify();
    }

    pub(in crate::ui::view) fn current_left_panel(&self) -> Option<NavItem> {
        if self.left_sidebar_collapsed {
            return None;
        }
        if self.panel_multi_open {
            if self.side_overlay_panel(PanelSide::Left).is_some()
                || !self.side_open_panel_ids(PanelSide::Left).is_empty()
            {
                return self
                    .side_overlay_panel(PanelSide::Left)
                    .or(self.active_left_panel)
                    .or_else(|| {
                        self.side_open_panel_ids(PanelSide::Left)
                            .first()
                            .and_then(|id| NavItem::from_persistence_id(id))
                    });
            }
            return None;
        }
        self.active_left_panel
    }

    pub(in crate::ui::view) fn current_right_panel(&self) -> Option<NavItem> {
        if self.right_inspector_collapsed {
            return None;
        }
        if self.panel_multi_open {
            if self.side_overlay_panel(PanelSide::Right).is_some()
                || !self.side_open_panel_ids(PanelSide::Right).is_empty()
            {
                return self
                    .side_overlay_panel(PanelSide::Right)
                    .or(self.active_right_panel)
                    .or_else(|| {
                        self.side_open_panel_ids(PanelSide::Right)
                            .first()
                            .and_then(|id| NavItem::from_persistence_id(id))
                    })
                    .or(if self.right_focus == RightFocus::Recording {
                        Some(NavItem::Recording)
                    } else {
                        None
                    });
            }
            return if self.right_focus == RightFocus::Recording {
                Some(NavItem::Recording)
            } else {
                None
            };
        }
        self.active_right_panel
            .or(if self.right_focus == RightFocus::Recording {
                Some(NavItem::Recording)
            } else {
                None
            })
    }

    pub(in crate::ui::view) fn left_side_open(&self) -> bool {
        self.current_left_panel().is_some()
            || (self.panel_multi_open && !self.side_open_panel_ids(PanelSide::Left).is_empty())
    }

    pub(in crate::ui::view) fn right_side_open(&self) -> bool {
        self.current_right_panel().is_some()
            || (self.panel_multi_open && !self.side_open_panel_ids(PanelSide::Right).is_empty())
    }
}
