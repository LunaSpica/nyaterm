use gpui::Context;
use nyaterm_core::ConnectionStore;

use crate::features::{NyaTermApp, short_id};
use crate::models::{MainMode, NavItem, SmartSplitMode, TabDockZone, TerminalWindowNode};

impl NyaTermApp {
    /// Ensure every live session appears in the multi-leaf layout once it is enabled.
    pub(in crate::features) fn reconcile_terminal_windows(&mut self) {
        // Flat strip mode (default): nothing to reconcile. Avoid allocating a full
        // SessionInfo list on every residual call.
        if self.terminal.windows.tree.is_none() {
            return;
        }

        let live_ids = self
            .session
            .order
            .iter()
            .filter(|session_id| {
                self.session.metadata.contains_key(*session_id)
                    && !self.is_secondary_pane_session(session_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        if live_ids.is_empty() {
            self.terminal.windows.tree = None;
            self.shell.workspace.focused_terminal_leaf_id = None;
            return;
        }

        if let Some(root) = self.terminal.windows.tree.as_mut() {
            // Drop closed sessions.
            for tab_id in root.collect_tab_ids() {
                if !live_ids.iter().any(|id| id == &tab_id) {
                    if let Some(next) = root.remove_tab(&tab_id) {
                        *root = next;
                    } else {
                        self.terminal.windows.tree = None;
                        break;
                    }
                }
            }
        }

        if let Some(root) = self.terminal.windows.tree.as_mut() {
            let preferred = self.shell.workspace.focused_terminal_leaf_id.clone();
            for tab_id in &live_ids {
                root.ensure_tab(tab_id, preferred.as_deref());
            }
            if let Some(active) = self.session.active_id.clone() {
                let _ = root.set_active_tab(&active);
            }
            if self
                .shell
                .workspace
                .focused_terminal_leaf_id
                .as_ref()
                .is_none_or(|id| !root.leaf_ids().iter().any(|leaf| leaf == id))
            {
                self.shell.workspace.focused_terminal_leaf_id = root.first_leaf_id();
            }
        }
    }

    pub(in crate::features) fn ensure_terminal_windows_root(&mut self) {
        if self.terminal.windows.tree.is_some() {
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
        let active = self.session.active_id.clone();
        let root = TerminalWindowNode::leaf(tab_ids, active);
        self.shell.workspace.focused_terminal_leaf_id = root.first_leaf_id();
        self.terminal.windows.tree = Some(root);
    }

    pub(in crate::features) fn activate_terminal_window_tab(
        &mut self,
        leaf_id: String,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        self.ensure_terminal_windows_root();
        if let Some(root) = self.terminal.windows.tree.as_mut() {
            let _ = set_leaf_active(root, &leaf_id, &session_id);
            let _ = root.set_active_tab(&session_id);
        }
        self.shell.workspace.focused_terminal_leaf_id = Some(leaf_id);
        self.activate_session_id_with_surface_sync(&session_id, cx);
        self.shell.navigation.selected_nav = NavItem::Workspace;
        self.shell.navigation.main_mode = MainMode::Workspace;
        cx.notify();
    }

    pub(in crate::features) fn terminal_windows_is_multi_leaf(&self) -> bool {
        matches!(
            self.terminal.windows.tree,
            Some(TerminalWindowNode::Split { .. })
        )
    }

    pub(in crate::features) fn sync_terminal_windows_active_tab(&mut self, session_id: &str) {
        if self.terminal.windows.tree.is_none() {
            return;
        }
        // Multi-leaf tab ids are tab roots; map secondary pane focus to its strip tab.
        let tab_id = self.tab_root_for_session(session_id);
        let Some(root) = self.terminal.windows.tree.as_mut() else {
            return;
        };
        let _ = root.set_active_tab(&tab_id);
        if let Some(leaf_id) = find_leaf_with_tab(root, &tab_id) {
            self.shell.workspace.focused_terminal_leaf_id = Some(leaf_id);
        }
    }

    pub(in crate::features) fn place_tab_before_in_terminal_windows(
        &mut self,
        tab_id: String,
        before_tab_id: String,
        cx: &mut Context<Self>,
    ) {
        self.terminal.windows.drop = None;
        let Some(root) = self.terminal.windows.tree.as_mut() else {
            return;
        };
        if root.place_tab_before(&tab_id, &before_tab_id) {
            let _ = root.set_active_tab(&tab_id);
            self.shell.workspace.focused_terminal_leaf_id =
                find_leaf_with_tab(root, &tab_id).or_else(|| root.first_leaf_id());
            self.activate_session_id_with_surface_sync(&tab_id, cx);
            self.terminal.view.status = format!(
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
        if self.terminal.windows.drop != next {
            self.terminal.windows.drop = next;
            cx.notify();
        }
    }

    pub(in crate::features) fn clear_terminal_window_drop(&mut self, cx: &mut Context<Self>) {
        if self.terminal.windows.drop.take().is_some() {
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
        self.terminal.windows.drop = None;
        self.ensure_terminal_windows_root();
        let Some(root) = self.terminal.windows.tree.as_mut() else {
            cx.notify();
            return;
        };
        if !root.contains_tab(&tab_id) {
            self.terminal.view.status = format!("unknown tab {}", short_id(&tab_id));
            cx.notify();
            return;
        }
        // Dropping onto the sole-tab same leaf is a no-op for edge; center is fine.
        if !root.dock_tab(&tab_id, &target_leaf_id, zone) {
            self.terminal.view.status = "tab dock had no effect".to_string();
            cx.notify();
            return;
        }
        let _ = root.set_active_tab(&tab_id);
        self.shell.workspace.focused_terminal_leaf_id =
            find_leaf_with_tab(root, &tab_id).or_else(|| root.first_leaf_id());
        self.activate_session_id_with_surface_sync(&tab_id, cx);
        self.shell.navigation.selected_nav = NavItem::Workspace;
        self.shell.navigation.main_mode = MainMode::Workspace;
        let zone_label = match zone {
            TabDockZone::Center => "merged into leaf".to_string(),
            TabDockZone::Edge(edge) => format!("split to {}", edge.label()),
        };
        self.terminal.view.status = format!("docked tab {} ({})", short_id(&tab_id), zone_label);
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
            self.terminal.view.status = "no tabs to tile".to_string();
            cx.notify();
            return;
        }
        let Some(layout) = TerminalWindowNode::build_smart_split_layout(&tab_ids, mode) else {
            self.terminal.view.status = "unable to build tile layout".to_string();
            cx.notify();
            return;
        };
        // Clear global pane splits so multi-leaf rendering takes precedence cleanly.
        self.shell.workspace.split = None;
        self.shell.workspace.split_resize = None;
        if let Some(active) = self.session.active_id.clone() {
            let mut root = layout;
            let _ = root.set_active_tab(&active);
            self.shell.workspace.focused_terminal_leaf_id =
                find_leaf_with_tab(&root, &active).or_else(|| root.first_leaf_id());
            self.terminal.windows.tree = Some(root);
        } else {
            self.shell.workspace.focused_terminal_leaf_id = layout.first_leaf_id();
            self.terminal.windows.tree = Some(layout);
        }
        self.shell.navigation.selected_nav = NavItem::Workspace;
        self.shell.navigation.main_mode = MainMode::Workspace;
        self.terminal.view.status = format!("applied {}", mode.label().to_ascii_lowercase());
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
        self.terminal.view.runtime.window_layout_persist_dirty = true;
    }

    pub(in crate::features) fn try_restore_terminal_window_layout(&mut self) {
        if self.terminal.windows.restored {
            return;
        }
        if !self.settings.startup_restore || !self.settings.startup_restore_window_layout {
            self.terminal.windows.restored = true;
            return;
        }
        // Do not open the config DB during connect/register; wait for idle.
        if self.session.start.has_pending() || self.runtime_output_pressure_active() {
            return;
        }
        let ordered = self
            .ordered_tab_sessions()
            .into_iter()
            .map(|session| session.id)
            .collect::<Vec<_>>();
        // Wait until startup restore has created sessions so tab indexes can map
        // correctly. Once startup restore is complete, an empty session list
        // means there is nothing to restore; mark this done so the runtime can
        // enter the quiet cadence.
        if ordered.is_empty() {
            if self.session.restore.is_complete() {
                self.terminal.windows.restored = true;
            }
            return;
        }
        self.terminal.windows.restored = true;
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
        self.shell.workspace.focused_terminal_leaf_id = restored.first_leaf_id();
        if let Some(active) = self.session.active_id.clone() {
            let mut root = restored;
            let _ = root.set_active_tab(&active);
            self.shell.workspace.focused_terminal_leaf_id =
                find_leaf_with_tab(&root, &active).or_else(|| root.first_leaf_id());
            self.terminal.windows.tree = Some(root);
        } else {
            self.terminal.windows.tree = Some(restored);
        }
        self.terminal.view.status = "restored multi-leaf window layout".to_string();
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
