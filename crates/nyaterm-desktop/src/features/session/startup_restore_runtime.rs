use gpui::{Context, Window};
use nyaterm_core::{
    AiExecutionProfile, ConnectionStore, RestorableOpenTab, RestorablePaneNode,
    RestorableWorkspacePaneNode,
};
use nyaterm_transport::{LocalSessionConfig, SessionInfo};

use crate::features::{NyaTermApp, SavedConnectionStartOptions};
use crate::models::{
    MainMode, NavItem, SessionLaunchConfig, WorkspacePaneNode, WorkspaceSplitDirection,
};

impl NyaTermApp {
    fn mark_startup_restore_complete(&mut self) {
        self.session.mark_restore_complete();
    }

    /// Mark open tabs (and multi-leaf layout) dirty for a later idle flush.
    ///
    /// Connect/register must not open the config database or rewrite settings on
    /// the UI thread — that path was a major connect-time freeze source.
    pub(in crate::features) fn persist_open_tabs(&mut self) {
        if !self.settings.summary.startup_restore {
            return;
        }
        self.terminal.view.runtime.open_tabs_persist_dirty = true;
        // Keep multi-leaf layout indexes aligned with the same ordered tab list.
        self.persist_terminal_window_layout();
    }

    /// Force a durable open-tabs write (window close / explicit quit paths).
    pub(in crate::features) fn flush_open_tabs_now(&mut self) {
        if !self.settings.summary.startup_restore {
            self.terminal.view.runtime.open_tabs_persist_dirty = false;
            self.terminal.view.runtime.window_layout_persist_dirty = false;
            return;
        }
        self.terminal.view.runtime.open_tabs_persist_dirty = true;
        self.terminal.view.runtime.window_layout_persist_dirty = true;
        self.flush_pending_session_persistence_sync();
    }

    /// Idle plane: snapshot dirty state and write config on a background thread.
    ///
    /// Serialization stays on the UI thread (local maps only). Opening redb and
    /// rewriting settings is never done on the UI tick — that freezes connect
    /// and the first idle frame after connect.
    pub(in crate::features) fn flush_pending_session_persistence(&mut self) {
        if !self.settings.summary.startup_restore {
            self.terminal.view.runtime.open_tabs_persist_dirty = false;
            self.terminal.view.runtime.window_layout_persist_dirty = false;
            return;
        }
        let need_tabs = self.terminal.view.runtime.open_tabs_persist_dirty;
        let need_layout = self.terminal.view.runtime.window_layout_persist_dirty
            && self.settings.summary.startup_restore_window_layout;
        if !need_tabs && !need_layout {
            return;
        }

        let tabs = need_tabs.then(|| self.serialize_open_tabs());
        let layout = if need_layout {
            let ordered = self
                .ordered_tab_sessions()
                .into_iter()
                .map(|session| session.id)
                .collect::<Vec<_>>();
            Some(
                self.terminal
                    .windows
                    .tree
                    .as_ref()
                    .filter(|_| self.terminal_windows_is_multi_leaf())
                    .and_then(|root| root.serialize_layout(&ordered)),
            )
        } else {
            None
        };

        // Clear dirty before spawn so repeated idle ticks do not re-queue while
        // the worker is still writing. Window-close uses the sync path below.
        if need_tabs {
            self.terminal.view.runtime.open_tabs_persist_dirty = false;
        }
        if need_layout {
            self.terminal.view.runtime.window_layout_persist_dirty = false;
        }

        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key = self.runtime.portable_key_path().map(ToOwned::to_owned);
        std::thread::Builder::new()
            .name("nyaterm-persist-tabs".into())
            .spawn(move || {
                let Ok(store) =
                    ConnectionStore::open_with_portable_key_path(config_dir, portable_key)
                else {
                    tracing::warn!(
                        diagnostic = "session_persist",
                        "failed to open config store for deferred tab persist"
                    );
                    return;
                };
                if let Some(tabs) = tabs.as_ref() {
                    if let Err(error) = store.save_open_tabs(tabs) {
                        tracing::warn!(
                            diagnostic = "session_persist",
                            error = %error,
                            "failed to save open tabs in background"
                        );
                    }
                }
                if let Some(layout) = layout.as_ref() {
                    if let Err(error) = store.save_terminal_window_layout(layout.as_ref()) {
                        tracing::warn!(
                            diagnostic = "session_persist",
                            error = %error,
                            "failed to save window layout in background"
                        );
                    }
                }
            })
            .ok();
    }

    /// Synchronous durable write used by window-close / quit (must not race exit).
    fn flush_pending_session_persistence_sync(&mut self) {
        if !self.settings.summary.startup_restore {
            self.terminal.view.runtime.open_tabs_persist_dirty = false;
            self.terminal.view.runtime.window_layout_persist_dirty = false;
            return;
        }
        let need_tabs = self.terminal.view.runtime.open_tabs_persist_dirty;
        let need_layout = self.terminal.view.runtime.window_layout_persist_dirty
            && self.settings.summary.startup_restore_window_layout;
        if !need_tabs && !need_layout {
            return;
        }

        let tabs = need_tabs.then(|| self.serialize_open_tabs());
        let layout = if need_layout {
            let ordered = self
                .ordered_tab_sessions()
                .into_iter()
                .map(|session| session.id)
                .collect::<Vec<_>>();
            Some(
                self.terminal
                    .windows
                    .tree
                    .as_ref()
                    .filter(|_| self.terminal_windows_is_multi_leaf())
                    .and_then(|root| root.serialize_layout(&ordered)),
            )
        } else {
            None
        };

        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key = self.runtime.portable_key_path().map(ToOwned::to_owned);
        match ConnectionStore::open_with_portable_key_path(config_dir, portable_key) {
            Ok(store) => {
                if let Some(tabs) = tabs.as_ref() {
                    match store.save_open_tabs(tabs) {
                        Ok(()) => self.terminal.view.runtime.open_tabs_persist_dirty = false,
                        Err(error) => {
                            self.terminal.view.status =
                                format!("failed to save open tabs: {error}");
                        }
                    }
                }
                if let Some(layout) = layout.as_ref() {
                    match store.save_terminal_window_layout(layout.as_ref()) {
                        Ok(()) => self.terminal.view.runtime.window_layout_persist_dirty = false,
                        Err(error) => {
                            self.terminal.view.status =
                                format!("failed to save window layout: {error}");
                        }
                    }
                }
            }
            Err(error) => {
                self.terminal.view.status = format!("failed to open config store: {error}");
            }
        }
    }

    pub(in crate::features) fn serialize_open_tabs(&self) -> Vec<RestorableOpenTab> {
        // Prefer a single Tauri-style open_tabs entry when one pane tree covers every session.
        if let Some(tabs) = self.serialize_open_tabs_as_single_pane_tab() {
            return tabs;
        }

        // One strip tab per tab-root; attach RestorablePaneNode when that tab is split.
        self.ordered_tab_sessions()
            .into_iter()
            .map(|session| {
                let mut tab = self.serialize_open_tab_for_session(&session);
                if let Some(root) = self.shell.workspace.pane_roots.get(&session.id) {
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
        let metadata = self.session.metadata.get(&session.id);
        let connection_id = metadata.and_then(|meta| meta.source_connection_id.clone());
        let session_type = match metadata.map(|meta| &meta.launch_config) {
            Some(SessionLaunchConfig::Ssh(_)) => "SSH",
            Some(SessionLaunchConfig::Telnet(_)) => "Telnet",
            Some(SessionLaunchConfig::Serial(_)) => "Serial",
            Some(SessionLaunchConfig::Local(_)) | None => "Local",
        }
        .to_string();
        let custom_name = self.session.custom_names.get(&session.id).cloned();
        let tab_color = self
            .session
            .tab_colors
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
        if self.shell.workspace.pane_roots.len() > 1 {
            return None;
        }
        let root = self
            .shell
            .workspace
            .pane_roots
            .values()
            .find(|root| root.is_split())
            .or(self.shell.workspace.split.as_ref())?;
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
        tab.active_pane_id = self.session.active_id.clone();
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
                        let ratio = (WorkspacePaneNode::clamped_ratio_percent(*ratio_percent)
                            as f64)
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

    pub(in crate::features) fn try_restore_open_tabs(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let should_restore = self
            .stores
            .startup_restore
            .update(cx, |store, _| store.mark_open_tabs_restored());
        if !should_restore {
            return;
        }
        if !self.settings.summary.startup_restore {
            self.mark_startup_restore_complete();
            return;
        }
        if !self.ordered_sessions().is_empty() {
            self.mark_startup_restore_complete();
            return;
        }
        let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) else {
            self.mark_startup_restore_complete();
            return;
        };
        let Ok(tabs) = store.load_open_tabs() else {
            self.mark_startup_restore_complete();
            return;
        };
        drop(store);
        if tabs.is_empty() {
            self.mark_startup_restore_complete();
            return;
        }
        // Expand Tauri per-tab pane trees into a flat restore queue of sessions.
        // Remember every multi-pane root so we can reinstall per-tab trees after connect.
        let mut pending_pane_layouts = Vec::new();
        let mut pending_active_pane_indexes = Vec::new();
        let mut expanded = Vec::new();
        let mut base_index = 0usize;
        for tab in &tabs {
            if let Some(layout) = tab.workspace_pane_layout_from_root(base_index) {
                pending_pane_layouts.push(layout);
            }
            if let (Some(root), Some(active_pane_id)) =
                (tab.root.as_ref(), tab.active_pane_id.as_ref())
            {
                if let Some(leaf_offset) = root
                    .collect_leaves()
                    .iter()
                    .position(|leaf| &leaf.id == active_pane_id)
                {
                    pending_active_pane_indexes.push(base_index + leaf_offset);
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
            self.mark_startup_restore_complete();
            return;
        }
        let queue_len = expanded.len();
        self.stores.startup_restore.update(cx, |store, _| {
            store.clear_pending_layouts();
            for layout in pending_pane_layouts {
                store.push_pending_pane_layout(layout);
            }
            for index in pending_active_pane_indexes {
                store.push_pending_active_pane_index(index);
            }
            store.set_queue(expanded);
        });
        self.terminal.view.status = format!("restoring {} workspace tab(s)...", queue_len);
        self.pump_startup_restore_queue(window, cx);
    }

    pub(in crate::features) fn pump_startup_restore_queue(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.session.restore_is_complete() {
            return;
        }
        if self.has_pending_session_start() {
            return;
        }
        let Some(tab) = self
            .stores
            .startup_restore
            .update(cx, |store, _| store.pop_next_tab())
        else {
            self.finish_startup_restore(cx);
            return;
        };

        let started = self.start_restorable_open_tab(&tab, window, cx);
        if !started {
            // Keep draining sync failures until pending async work or queue empty.
            self.pump_startup_restore_queue(window, cx);
        }
    }

    fn finish_startup_restore(&mut self, cx: &mut Context<Self>) {
        if self.session.restore_is_complete() {
            return;
        }
        self.mark_startup_restore_complete();
        // After all tabs reconnect, attempt multi-leaf then global pane layout restore.
        self.terminal.windows.restored = false;
        self.shell.workspace.pane_layout_restored = false;
        self.try_restore_terminal_window_layout();
        // Prefer stored ui.workspace_pane_layout only when no open_tabs per-tab roots exist.
        // open_tabs[].root maps to per-tab session_pane_roots (Tauri Tab.root).
        let pending_layouts = self
            .stores
            .startup_restore
            .update(cx, |store, _| store.take_pending_pane_layouts());
        let pending_active = self
            .stores
            .startup_restore
            .update(cx, |store, _| store.take_pending_active_pane_indexes());
        if pending_layouts.is_empty() {
            self.try_restore_workspace_pane_layout();
        } else {
            self.shell.workspace.pane_layout_restored = true;
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
            self.terminal.view.status = "restored workspace tabs and window layout".to_string();
        } else if !self.shell.workspace.pane_roots.is_empty()
            || self
                .shell
                .workspace
                .split
                .as_ref()
                .is_some_and(|root| root.is_split())
        {
            self.terminal.view.status = "restored workspace tabs and pane layout".to_string();
        } else if !self.ordered_sessions().is_empty() {
            self.terminal.view.status = "restored workspace tabs".to_string();
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
                .connection_catalog
                .connections()
                .iter()
                .find(|connection| &connection.id == connection_id)
                .cloned();
            let Some(connection) = connection else {
                self.terminal.view.status =
                    format!("restore skipped missing connection {connection_id}");
                return false;
            };
            self.start_saved_connection_with_options(
                connection,
                SavedConnectionStartOptions {
                    custom_name,
                    tab_color,
                    ..Default::default()
                },
                window,
                cx,
            );
            return true;
        }

        if session_type == "local" || session_type.is_empty() {
            let mut config = LocalSessionConfig::default();
            self.apply_desired_geometry_to_local_config(&mut config);
            self.begin_background_session_start(
                config.name.clone(),
                SessionLaunchConfig::Local(config),
                None,
                AiExecutionProfile::Posix,
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

        self.terminal.view.status = format!(
            "restore skipped unsupported tab {} ({})",
            tab.title, tab.session_type
        );
        false
    }

    fn apply_restorable_workspace_pane_layout(&mut self, layout: RestorableWorkspacePaneNode) {
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
        if let Some(existing) = self.shell.workspace.pane_roots.get(&first) {
            if existing != &restored {
                // Prefer the newly restored tree from open_tabs for this root.
            }
        }
        self.shell
            .workspace
            .pane_roots
            .insert(first.clone(), restored);
        self.rebuild_session_tab_owners();
        if self.session.active_id.is_none() {
            self.session.active_id = Some(first);
        }
        self.sync_workspace_split_from_active_tab();
        self.shell.workspace.pane_layout_restored = true;
        self.shell.navigation.selected_nav = NavItem::Workspace;
        self.shell.navigation.main_mode = MainMode::Workspace;
        self.terminal.view.status = "restored pane layout from open_tabs root".to_string();
    }
}

fn parse_restorable_tab_color(value: Option<&str>) -> Option<u32> {
    let raw = value?.trim().trim_start_matches('#');
    if raw.len() != 6 {
        return None;
    }
    u32::from_str_radix(raw, 16).ok()
}
