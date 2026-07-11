use super::*;

impl NyaTermApp {
    /// Ensure every live session appears in the multi-leaf layout once it is enabled.
    pub(in crate::ui::view) fn reconcile_terminal_windows(&mut self) {
        let live_ids = self
            .ordered_sessions()
            .into_iter()
            .map(|session| session.id)
            .collect::<Vec<_>>();
        if live_ids.is_empty() {
            self.terminal_windows = None;
            self.focused_terminal_window_leaf_id = None;
            return;
        }

        if let Some(root) = self.terminal_windows.as_mut() {
            // Drop closed sessions.
            for tab_id in root.collect_tab_ids() {
                if !live_ids.iter().any(|id| id == &tab_id) {
                    if let Some(next) = root.remove_tab(&tab_id) {
                        *root = next;
                    } else {
                        self.terminal_windows = None;
                        break;
                    }
                }
            }
        }

        if self.terminal_windows.is_none() {
            // Flat mode: no multi-leaf layout until user splits a tab out.
            return;
        }

        if let Some(root) = self.terminal_windows.as_mut() {
            let preferred = self.focused_terminal_window_leaf_id.clone();
            for tab_id in &live_ids {
                root.ensure_tab(tab_id, preferred.as_deref());
            }
            if let Some(active) = self.active_session_id.clone() {
                let _ = root.set_active_tab(&active);
            }
            if self
                .focused_terminal_window_leaf_id
                .as_ref()
                .is_none_or(|id| !root.leaf_ids().iter().any(|leaf| leaf == id))
            {
                self.focused_terminal_window_leaf_id = root.first_leaf_id();
            }
        }
    }

    pub(in crate::ui::view) fn ensure_terminal_windows_root(&mut self) {
        if self.terminal_windows.is_some() {
            return;
        }
        let tab_ids = self
            .ordered_sessions()
            .into_iter()
            .map(|session| session.id)
            .collect::<Vec<_>>();
        if tab_ids.is_empty() {
            return;
        }
        let active = self.active_session_id.clone();
        let root = TerminalWindowNode::leaf(tab_ids, active);
        self.focused_terminal_window_leaf_id = root.first_leaf_id();
        self.terminal_windows = Some(root);
    }

    /// Split the active tab into a new window leaf (Tauri "Open in New Window" in-app).
    pub(in crate::ui::view) fn split_active_tab_to_new_window_leaf(
        &mut self,
        direction: WorkspaceSplitDirection,
        edge: SplitEdge,
        cx: &mut Context<Self>,
    ) {
        let Some(session_id) = self.active_session_id.clone() else {
            self.terminal_status = "no active session to split into a window leaf".to_string();
            cx.notify();
            return;
        };
        self.ensure_terminal_windows_root();
        let Some(root) = self.terminal_windows.as_mut() else {
            self.terminal_status = "no sessions available for window split".to_string();
            cx.notify();
            return;
        };
        if !root.split_tab_to_edge(&session_id, direction, edge) {
            // Only one tab in leaf — cannot detach into a second leaf.
            self.terminal_status =
                "need at least two tabs in a leaf to open a new window pane".to_string();
            cx.notify();
            return;
        }
        let _ = root.set_active_tab(&session_id);
        self.focused_terminal_window_leaf_id =
            find_leaf_with_tab(root, &session_id).or_else(|| root.first_leaf_id());
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        self.terminal_status = format!(
            "opened session {} in a new window leaf ({})",
            short_id(&session_id),
            direction.label().to_ascii_lowercase()
        );
        cx.notify();
    }

    pub(in crate::ui::view) fn activate_terminal_window_tab(
        &mut self,
        leaf_id: String,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        self.ensure_terminal_windows_root();
        if let Some(root) = self.terminal_windows.as_mut() {
            let _ = set_leaf_active(root, &leaf_id, &session_id);
            let _ = root.set_active_tab(&session_id);
        }
        self.focused_terminal_window_leaf_id = Some(leaf_id);
        self.activate_session_id(&session_id);
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        cx.notify();
    }

    pub(in crate::ui::view) fn move_active_tab_to_leaf(
        &mut self,
        target_leaf_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(session_id) = self.active_session_id.clone() else {
            return;
        };
        let Some(root) = self.terminal_windows.as_mut() else {
            return;
        };
        if root.move_tab_to_leaf(&session_id, &target_leaf_id) {
            self.focused_terminal_window_leaf_id = Some(target_leaf_id);
            let _ = root.set_active_tab(&session_id);
            self.terminal_status = format!("moved tab {} to window leaf", short_id(&session_id));
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn close_terminal_window_layout(&mut self, cx: &mut Context<Self>) {
        self.terminal_windows = None;
        self.focused_terminal_window_leaf_id = None;
        self.terminal_window_drop = None;
        self.terminal_status = "restored flat tab strip".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn terminal_windows_is_multi_leaf(&self) -> bool {
        matches!(self.terminal_windows, Some(TerminalWindowNode::Split { .. }))
    }

    pub(in crate::ui::view) fn sync_terminal_windows_active_tab(&mut self, session_id: &str) {
        let Some(root) = self.terminal_windows.as_mut() else {
            return;
        };
        let _ = root.set_active_tab(session_id);
        if let Some(leaf_id) = find_leaf_with_tab(root, session_id) {
            self.focused_terminal_window_leaf_id = Some(leaf_id);
        }
    }

    pub(in crate::ui::view) fn set_terminal_window_drop(
        &mut self,
        leaf_id: String,
        zone: TabDockZone,
        cx: &mut Context<Self>,
    ) {
        let next = Some((leaf_id, zone));
        if self.terminal_window_drop != next {
            self.terminal_window_drop = next;
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn clear_terminal_window_drop(&mut self, cx: &mut Context<Self>) {
        if self.terminal_window_drop.take().is_some() {
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn dock_tab_on_terminal_window_leaf(
        &mut self,
        tab_id: String,
        target_leaf_id: String,
        zone: TabDockZone,
        cx: &mut Context<Self>,
    ) {
        self.terminal_window_drop = None;
        self.ensure_terminal_windows_root();
        let Some(root) = self.terminal_windows.as_mut() else {
            cx.notify();
            return;
        };
        if !root.contains_tab(&tab_id) {
            self.terminal_status = format!("unknown tab {}", short_id(&tab_id));
            cx.notify();
            return;
        }
        // Dropping onto the sole-tab same leaf is a no-op for edge; center is fine.
        if !root.dock_tab(&tab_id, &target_leaf_id, zone) {
            self.terminal_status = "tab dock had no effect".to_string();
            cx.notify();
            return;
        }
        let _ = root.set_active_tab(&tab_id);
        self.focused_terminal_window_leaf_id =
            find_leaf_with_tab(root, &tab_id).or_else(|| root.first_leaf_id());
        self.activate_session_id(&tab_id);
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        let zone_label = match zone {
            TabDockZone::Center => "merged into leaf".to_string(),
            TabDockZone::Edge(edge) => format!("split to {}", edge.label()),
        };
        self.terminal_status = format!(
            "docked tab {} ({})",
            short_id(&tab_id),
            zone_label
        );
        // Collapse back to flat strip if only one leaf remains.
        if matches!(self.terminal_windows, Some(TerminalWindowNode::Leaf { .. })) {
            // Keep multi-leaf structure only when still split; leaf-only can stay for active tabs.
        }
        cx.notify();
    }

}

fn find_leaf_with_tab(node: &TerminalWindowNode, tab_id: &str) -> Option<String> {
    match node {
        TerminalWindowNode::Leaf { id, tab_ids, .. } => {
            if tab_ids.iter().any(|id| id == tab_id) {
                Some(id.clone())
            } else {
                None
            }
        }
        TerminalWindowNode::Split { first, second, .. } => {
            find_leaf_with_tab(first, tab_id).or_else(|| find_leaf_with_tab(second, tab_id))
        }
    }
}

fn set_leaf_active(node: &mut TerminalWindowNode, leaf_id: &str, tab_id: &str) -> bool {
    match node {
        TerminalWindowNode::Leaf {
            id,
            tab_ids,
            active_tab_id,
        } => {
            if id == leaf_id && tab_ids.iter().any(|id| id == tab_id) {
                *active_tab_id = Some(tab_id.to_string());
                true
            } else {
                false
            }
        }
        TerminalWindowNode::Split { first, second, .. } => {
            set_leaf_active(first, leaf_id, tab_id) || set_leaf_active(second, leaf_id, tab_id)
        }
    }
}
