use super::*;

impl NyaTermApp {
    /// Ensure every live session appears in the multi-leaf layout once it is enabled.
    pub(in crate::features) fn reconcile_terminal_windows(&mut self) {
        let live_ids = self
            .ordered_tab_sessions()
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
            // Attempt one-shot restore of multi-leaf layout from settings.
            self.try_restore_terminal_window_layout();
            if self.terminal_windows.is_none() {
                // Flat mode: no multi-leaf layout until user splits a tab out.
                return;
            }
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

    pub(in crate::features) fn ensure_terminal_windows_root(&mut self) {
        if self.terminal_windows.is_some() {
            return;
        }
        let tab_ids = self
            .ordered_tab_sessions()
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
    pub(in crate::features) fn split_active_tab_to_new_window_leaf(
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
        self.persist_terminal_window_layout();
        cx.notify();
    }

    pub(in crate::features) fn activate_terminal_window_tab(
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

    pub(in crate::features) fn move_active_tab_to_leaf(
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
            self.persist_terminal_window_layout();
        }
        cx.notify();
    }

    pub(in crate::features) fn close_terminal_window_layout(&mut self, cx: &mut Context<Self>) {
        self.terminal_windows = None;
        self.focused_terminal_window_leaf_id = None;
        self.terminal_window_drop = None;
        self.terminal_status = "restored flat tab strip".to_string();
        self.persist_terminal_window_layout();
        cx.notify();
    }

    pub(in crate::features) fn terminal_windows_is_multi_leaf(&self) -> bool {
        matches!(
            self.terminal_windows,
            Some(TerminalWindowNode::Split { .. })
        )
    }

    pub(in crate::features) fn sync_terminal_windows_active_tab(&mut self, session_id: &str) {
        if self.terminal_windows.is_none() {
            return;
        }
        // Multi-leaf tab ids are tab roots; map secondary pane focus to its strip tab.
        let tab_id = self.tab_root_for_session(session_id);
        let Some(root) = self.terminal_windows.as_mut() else {
            return;
        };
        let _ = root.set_active_tab(&tab_id);
        if let Some(leaf_id) = find_leaf_with_tab(root, &tab_id) {
            self.focused_terminal_window_leaf_id = Some(leaf_id);
        }
    }

    pub(in crate::features) fn place_tab_before_in_terminal_windows(
        &mut self,
        tab_id: String,
        before_tab_id: String,
        cx: &mut Context<Self>,
    ) {
        self.terminal_window_drop = None;
        let Some(root) = self.terminal_windows.as_mut() else {
            return;
        };
        if root.place_tab_before(&tab_id, &before_tab_id) {
            let _ = root.set_active_tab(&tab_id);
            self.focused_terminal_window_leaf_id =
                find_leaf_with_tab(root, &tab_id).or_else(|| root.first_leaf_id());
            self.activate_session_id(&tab_id);
            self.terminal_status = format!(
                "moved tab {} before {}",
                short_id(&tab_id),
                short_id(&before_tab_id)
            );
            self.persist_terminal_window_layout();
        }
        cx.notify();
    }

    pub(in crate::features) fn set_terminal_window_drop(
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

    pub(in crate::features) fn clear_terminal_window_drop(&mut self, cx: &mut Context<Self>) {
        if self.terminal_window_drop.take().is_some() {
            cx.notify();
        }
    }

    pub(in crate::features) fn dock_tab_on_terminal_window_leaf(
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
        self.terminal_status = format!("docked tab {} ({})", short_id(&tab_id), zone_label);
        self.persist_terminal_window_layout();
        cx.notify();
    }

    /// Apply Tauri smart-split / tile layout: each open tab becomes its own multi-leaf window.
    pub(in crate::features) fn apply_smart_split(
        &mut self,
        mode: SmartSplitMode,
        cx: &mut Context<Self>,
    ) {
        let tab_ids = self
            .ordered_tab_sessions()
            .into_iter()
            .map(|session| session.id)
            .collect::<Vec<_>>();
        if tab_ids.is_empty() {
            self.terminal_status = "no tabs to tile".to_string();
            cx.notify();
            return;
        }
        let Some(layout) = TerminalWindowNode::build_smart_split_layout(&tab_ids, mode) else {
            self.terminal_status = "unable to build tile layout".to_string();
            cx.notify();
            return;
        };
        // Clear global pane splits so multi-leaf rendering takes precedence cleanly.
        self.workspace_split = None;
        self.workspace_split_resize = None;
        if let Some(active) = self.active_session_id.clone() {
            let mut root = layout;
            let _ = root.set_active_tab(&active);
            self.focused_terminal_window_leaf_id =
                find_leaf_with_tab(&root, &active).or_else(|| root.first_leaf_id());
            self.terminal_windows = Some(root);
        } else {
            self.focused_terminal_window_leaf_id = layout.first_leaf_id();
            self.terminal_windows = Some(layout);
        }
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        self.terminal_status = format!("applied {}", mode.label().to_ascii_lowercase());
        self.persist_terminal_window_layout();
        // Global pane layout is obsolete while multi-leaf is active.
        self.persist_workspace_pane_layout();
        cx.notify();
    }

    pub(in crate::features) fn persist_terminal_window_layout(&mut self) {
        if !self.settings.startup_restore || !self.settings.startup_restore_window_layout {
            return;
        }
        // Defer disk write — layout changes must not open redb on the UI hot path.
        self.terminal_runtime.window_layout_persist_dirty = true;
    }

    pub(in crate::features) fn try_restore_terminal_window_layout(&mut self) {
        if self.terminal_windows_restored {
            return;
        }
        if !self.settings.startup_restore || !self.settings.startup_restore_window_layout {
            self.terminal_windows_restored = true;
            return;
        }
        // Do not open the config DB during connect/register; wait for idle.
        if !self.pending_session_starts.is_empty() || self.runtime_output_pressure_active() {
            return;
        }
        let ordered = self
            .ordered_tab_sessions()
            .into_iter()
            .map(|session| session.id)
            .collect::<Vec<_>>();
        // Wait until sessions exist so tab indexes can map correctly.
        if ordered.is_empty() {
            return;
        }
        self.terminal_windows_restored = true;
        let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) else {
            return;
        };
        let Ok(Some(layout)) = store.load_terminal_window_layout() else {
            return;
        };
        let Some(restored) = TerminalWindowNode::restore_layout(&layout, &ordered) else {
            return;
        };
        if !matches!(restored, TerminalWindowNode::Split { .. }) {
            return;
        }
        self.focused_terminal_window_leaf_id = restored.first_leaf_id();
        if let Some(active) = self.active_session_id.clone() {
            let mut root = restored;
            let _ = root.set_active_tab(&active);
            self.focused_terminal_window_leaf_id =
                find_leaf_with_tab(&root, &active).or_else(|| root.first_leaf_id());
            self.terminal_windows = Some(root);
        } else {
            self.terminal_windows = Some(restored);
        }
        self.terminal_status = "restored multi-leaf window layout".to_string();
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
