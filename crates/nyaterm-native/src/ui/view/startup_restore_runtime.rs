use super::*;
use nyaterm_domain::{RestorableOpenTab, RestorablePaneNode, RestorableWorkspacePaneNode};

impl NyaTermApp {
    pub(in crate::ui::view) fn persist_open_tabs(&mut self) {
        if !self.settings.startup_restore {
            return;
        }
        let tabs = self.serialize_open_tabs();
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_open_tabs(&tabs))
        {
            Ok(()) => {}
            Err(error) => {
                self.terminal_status = format!("failed to save open tabs: {error}");
            }
        }
        // Keep multi-leaf layout indexes aligned with the same ordered tab list.
        self.persist_terminal_window_layout();
    }

    pub(in crate::ui::view) fn serialize_open_tabs(&self) -> Vec<RestorableOpenTab> {
        // Prefer a single Tauri-style open_tabs entry when one pane tree covers every session.
        if let Some(tabs) = self.serialize_open_tabs_as_single_pane_tab() {
            return tabs;
        }

        // One strip tab per tab-root; attach RestorablePaneNode when that tab is split.
        self.ordered_tab_sessions()
            .into_iter()
            .map(|session| {
                let mut tab = self.serialize_open_tab_for_session(&session);
                if let Some(root) = self.session_pane_roots.get(&session.id) {
                    if root.is_split() {
                        if let Some(pane_root) = self.workspace_pane_to_restorable_pane(root) {
                            tab.root = Some(pane_root);
                            tab.active_pane_id = Some(self.active_pane_for_tab_root(&session.id));
                        }
                    }
                }
                tab
            })
            .collect()
    }

    fn serialize_open_tab_for_session(&self, session: &SessionInfo) -> RestorableOpenTab {
        let metadata = self.session_metadata.get(&session.id);
        let connection_id = metadata.and_then(|meta| meta.source_connection_id.clone());
        let session_type = match metadata.map(|meta| &meta.launch_config) {
            Some(SessionLaunchConfig::Ssh(_)) => "SSH",
            Some(SessionLaunchConfig::Telnet(_)) => "Telnet",
            Some(SessionLaunchConfig::Serial(_)) => "Serial",
            Some(SessionLaunchConfig::Local(_)) | None => "Local",
        }
        .to_string();
        let custom_name = self.session_custom_names.get(&session.id).cloned();
        let tab_color = self
            .session_tab_colors
            .get(&session.id)
            .map(|color| format!("#{color:06x}"));
        let title = self.session_display_name_by_info(session);
        RestorableOpenTab::with_leaf_root(
            title,
            session_type,
            connection_id,
            custom_name,
            tab_color,
        )
    }

    /// When every session is present in a global workspace split, emit one open_tabs
    /// entry whose `root` is a Tauri RestorablePaneNode tree (for interop).
    fn serialize_open_tabs_as_single_pane_tab(&self) -> Option<Vec<RestorableOpenTab>> {
        // Only collapse to one open_tabs entry when exactly one split tree covers every session.
        if self.session_pane_roots.len() > 1 {
            return None;
        }
        let root = self
            .session_pane_roots
            .values()
            .find(|root| root.is_split())
            .or(self.workspace_split.as_ref())?;
        if !root.is_split() {
            return None;
        }
        let ordered = self.ordered_sessions();
        if ordered.len() < 2 {
            return None;
        }
        let split_ids = root.session_ids();
        if split_ids.len() != ordered.len() {
            return None;
        }
        // Require the pane tree to cover exactly the ordered session set.
        let ordered_set = ordered
            .iter()
            .map(|session| session.id.as_str())
            .collect::<std::collections::HashSet<_>>();
        if split_ids
            .iter()
            .any(|id| !ordered_set.contains(id.as_str()))
        {
            return None;
        }

        let pane_root = self.workspace_pane_to_restorable_pane(root)?;
        let first = ordered.first()?;
        let mut tab = self.serialize_open_tab_for_session(first);
        // Title/type from first leaf; root carries the full tree.
        tab.root = Some(pane_root);
        tab.active_pane_id = self.active_session_id.clone();
        Some(vec![tab])
    }

    fn workspace_pane_to_restorable_pane(
        &self,
        node: &WorkspacePaneNode,
    ) -> Option<RestorablePaneNode> {
        match node {
            WorkspacePaneNode::Leaf { session_id } => {
                let session = self
                    .ordered_sessions()
                    .into_iter()
                    .find(|session| &session.id == session_id)?;
                let tab = self.serialize_open_tab_for_session(&session);
                // Use runtime session id as RestorablePane leaf id so active_pane_id roundtrips.
                Some(RestorablePaneNode::Leaf {
                    id: session_id.clone(),
                    title: tab.title,
                    session_type: tab.session_type,
                    connection_id: tab.connection_id,
                })
            }
            WorkspacePaneNode::Split {
                id,
                direction,
                ratio_percent,
                first,
                second,
            } => {
                let first = self.workspace_pane_to_restorable_pane(first);
                let second = self.workspace_pane_to_restorable_pane(second);
                match (first, second) {
                    (None, None) => None,
                    (Some(only), None) | (None, Some(only)) => Some(only),
                    (Some(first), Some(second)) => {
                        let ratio =
                            (WorkspacePaneNode::clamped_ratio_percent(*ratio_percent) as f64)
                                / 100.0;
                        Some(RestorablePaneNode::Split {
                            id: id.clone(),
                            direction: match direction {
                                WorkspaceSplitDirection::Horizontal => "horizontal".to_string(),
                                WorkspaceSplitDirection::Vertical => "vertical".to_string(),
                            },
                            ratio: ratio.clamp(0.2, 0.8),
                            first: Box::new(first),
                            second: Box::new(second),
                        })
                    }
                }
            }
        }
    }

    pub(in crate::ui::view) fn try_restore_open_tabs(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.open_tabs_restored {
            return;
        }
        self.open_tabs_restored = true;
        if !self.settings.startup_restore {
            self.startup_restore_complete = true;
            return;
        }
        if !self.ordered_sessions().is_empty() {
            self.startup_restore_complete = true;
            return;
        }
        let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) else {
            self.startup_restore_complete = true;
            return;
        };
        let Ok(tabs) = store.load_open_tabs() else {
            self.startup_restore_complete = true;
            return;
        };
        if tabs.is_empty() {
            self.startup_restore_complete = true;
            return;
        }
        // Expand Tauri per-tab pane trees into a flat restore queue of sessions.
        // Remember every multi-pane root so we can reinstall per-tab trees after connect.
        self.startup_pending_pane_layouts.clear();
        self.startup_pending_active_pane_indexes.clear();
        let mut expanded = Vec::new();
        let mut base_index = 0usize;
        for tab in &tabs {
            if let Some(layout) = tab.workspace_pane_layout_from_root(base_index) {
                self.startup_pending_pane_layouts.push(layout);
            }
            if let (Some(root), Some(active_pane_id)) =
                (tab.root.as_ref(), tab.active_pane_id.as_ref())
            {
                if let Some(leaf_offset) = root
                    .collect_leaves()
                    .iter()
                    .position(|leaf| &leaf.id == active_pane_id)
                {
                    self.startup_pending_active_pane_indexes
                        .push(base_index + leaf_offset);
                }
            }
            let sessions = tab.expanded_sessions();
            base_index += sessions.len();
            for session in sessions {
                expanded.push(RestorableOpenTab {
                    title: session.title,
                    session_type: session.session_type,
                    connection_id: session.connection_id,
                    custom_name: session.custom_name,
                    tab_color: session.tab_color,
                    active_pane_id: None,
                    root: None,
                });
            }
        }
        if expanded.is_empty() {
            self.startup_restore_complete = true;
            return;
        }
        self.startup_restore_queue = expanded;
        self.terminal_status = format!(
            "restoring {} workspace tab(s)...",
            self.startup_restore_queue.len()
        );
        self.pump_startup_restore_queue(window, cx);
    }

    pub(in crate::ui::view) fn pump_startup_restore_queue(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.startup_restore_complete {
            return;
        }
        if self.pending_session_name.is_some() {
            return;
        }
        let Some(tab) = self.startup_restore_queue.first().cloned() else {
            self.finish_startup_restore(cx);
            return;
        };

        let started = self.start_restorable_open_tab(&tab, window, cx);
        self.startup_restore_queue.remove(0);
        if !started {
            // Keep draining sync failures until pending async work or queue empty.
            self.pump_startup_restore_queue(window, cx);
        }
    }

    fn finish_startup_restore(&mut self, cx: &mut Context<Self>) {
        if self.startup_restore_complete {
            return;
        }
        self.startup_restore_complete = true;
        // After all tabs reconnect, attempt multi-leaf then global pane layout restore.
        self.terminal_windows_restored = false;
        self.workspace_pane_layout_restored = false;
        self.try_restore_terminal_window_layout();
        // Prefer stored ui.workspace_pane_layout only when no open_tabs per-tab roots exist.
        // open_tabs[].root maps to per-tab session_pane_roots (Tauri Tab.root).
        let pending_layouts = std::mem::take(&mut self.startup_pending_pane_layouts);
        let pending_active = std::mem::take(&mut self.startup_pending_active_pane_indexes);
        if pending_layouts.is_empty() {
            self.try_restore_workspace_pane_layout();
        } else {
            self.workspace_pane_layout_restored = true;
            for layout in pending_layouts {
                self.apply_restorable_workspace_pane_layout(layout);
            }
            // Focus last requested active pane leaf if still present.
            if let Some(index) = pending_active.last().copied() {
                let ordered = self
                    .ordered_sessions()
                    .into_iter()
                    .map(|session| session.id)
                    .collect::<Vec<_>>();
                if let Some(session_id) = ordered.get(index) {
                    self.activate_session_id(session_id);
                    self.sync_workspace_split_from_active_tab();
                }
            }
        }
        if self.terminal_windows_is_multi_leaf() {
            self.terminal_status = "restored workspace tabs and window layout".to_string();
        } else if !self.session_pane_roots.is_empty()
            || self.workspace_split.as_ref().is_some_and(|root| root.is_split())
        {
            self.terminal_status = "restored workspace tabs and pane layout".to_string();
        } else if !self.ordered_sessions().is_empty() {
            self.terminal_status = "restored workspace tabs".to_string();
        }
        if !self.ordered_sessions().is_empty() {
            self.persist_open_tabs();
        }
        cx.notify();
    }

    fn start_restorable_open_tab(
        &mut self,
        tab: &RestorableOpenTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let custom_name = tab
            .custom_name
            .clone()
            .filter(|value| !value.trim().is_empty());
        let tab_color = parse_restorable_tab_color(tab.tab_color.as_deref());
        let session_type = tab.session_type.to_ascii_lowercase();

        if let Some(connection_id) = tab.connection_id.as_ref().filter(|id| !id.is_empty()) {
            let connection = self
                .connections
                .iter()
                .find(|connection| &connection.id == connection_id)
                .cloned();
            let Some(connection) = connection else {
                self.terminal_status =
                    format!("restore skipped missing connection {connection_id}");
                return false;
            };
            match connection.config.clone() {
                ConnectionType::Ssh {
                    ai_execution_profile,
                    ..
                } => {
                    self.ensure_event_pump(window, cx);
                    let config = match self.build_ssh_session_config(&connection, &mut Vec::new()) {
                        Ok(config) => config,
                        Err(error) => {
                            self.terminal_status =
                                format!("restore SSH prepare failed: {error}");
                            return false;
                        }
                    };
                    self.begin_background_ssh_start(
                        connection.name,
                        config,
                        Some(connection.id),
                        ai_execution_profile,
                        custom_name,
                        tab_color,
                        None,
                        None,
                        None,
                        None,
                        cx,
                    );
                    return true;
                }
                _ => {
                    self.start_saved_connection(connection, window, cx);
                    if let Some(session_id) = self.active_session_id.clone() {
                        if let Some(name) = custom_name {
                            self.session_custom_names.insert(session_id.clone(), name);
                        }
                        if let Some(color) = tab_color {
                            self.session_tab_colors.insert(session_id, color);
                        }
                    }
                    return true;
                }
            }
        }

        if session_type == "local" || session_type.is_empty() {
            self.start_local_session(window, cx);
            if let Some(session_id) = self.active_session_id.clone() {
                if let Some(name) = custom_name {
                    self.session_custom_names.insert(session_id.clone(), name);
                }
                if let Some(color) = tab_color {
                    self.session_tab_colors.insert(session_id, color);
                }
            }
            return true;
        }

        self.terminal_status = format!(
            "restore skipped unsupported tab {} ({})",
            tab.title, tab.session_type
        );
        false
    }

    fn apply_restorable_workspace_pane_layout(
        &mut self,
        layout: RestorableWorkspacePaneNode,
    ) {
        if self.terminal_windows_is_multi_leaf() {
            return;
        }
        let ordered = self
            .ordered_sessions()
            .into_iter()
            .map(|session| session.id)
            .collect::<Vec<_>>();
        if ordered.len() < 2 {
            return;
        }
        let Some(restored) = WorkspacePaneNode::restore_layout(&layout, &ordered) else {
            return;
        };
        if !restored.is_split() {
            return;
        }
        // Key the tree by its first leaf so secondary leaves leave the strip.
        let Some(first) = restored.session_ids().into_iter().next() else {
            return;
        };
        // Avoid clobbering an existing distinct per-tab tree for the same root.
        if let Some(existing) = self.session_pane_roots.get(&first) {
            if existing != &restored {
                // Prefer the newly restored tree from open_tabs for this root.
            }
        }
        self.session_pane_roots.insert(first.clone(), restored);
        self.rebuild_session_tab_owners();
        if self.active_session_id.is_none() {
            self.active_session_id = Some(first);
        }
        self.sync_workspace_split_from_active_tab();
        self.workspace_pane_layout_restored = true;
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        self.terminal_status = "restored pane layout from open_tabs root".to_string();
    }
}

fn parse_restorable_tab_color(value: Option<&str>) -> Option<u32> {
    let raw = value?.trim().trim_start_matches('#');
    if raw.len() != 6 {
        return None;
    }
    u32::from_str_radix(raw, 16).ok()
}
