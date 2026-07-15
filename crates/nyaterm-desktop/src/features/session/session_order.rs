use super::*;

impl NyaTermApp {
    pub(in crate::features) fn register_session(
        &mut self,
        session_id: &str,
        metadata: SessionRuntimeMetadata,
    ) {
        if !self.session_order.iter().any(|id| id == session_id) {
            self.session_order.push(session_id.to_string());
        }
        self.session_metadata
            .insert(session_id.to_string(), metadata);
        let encoding = self.settings.interaction_default_encoding.clone();
        let view = self
            .terminal_views
            .entry(session_id.to_string())
            .or_insert_with(TerminalViewState::new);
        view.set_encoding(&encoding);
        self.reconcile_terminal_windows();
        if self.startup_restore_complete {
            self.persist_open_tabs();
        }
    }

    pub(in crate::features) fn move_session_after(
        &mut self,
        session_id: &str,
        after_session_id: &str,
    ) {
        if session_id == after_session_id {
            return;
        }
        let Some(mut session_index) = self.session_order.iter().position(|id| id == session_id)
        else {
            return;
        };
        let Some(mut after_index) = self
            .session_order
            .iter()
            .position(|id| id == after_session_id)
        else {
            return;
        };
        let session_id = self.session_order.remove(session_index);
        if session_index < after_index {
            after_index = after_index.saturating_sub(1);
        }
        session_index = (after_index + 1).min(self.session_order.len());
        self.session_order.insert(session_index, session_id);
    }

    pub(in crate::features) fn move_session_to_index(&mut self, session_id: &str, index: usize) {
        let Some(current_index) = self.session_order.iter().position(|id| id == session_id) else {
            return;
        };
        let session_id = self.session_order.remove(current_index);
        let index = index.min(self.session_order.len());
        self.session_order.insert(index, session_id);
    }

    pub(in crate::features) fn ordered_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.session_manager.list_sessions().unwrap_or_default();
        let mut by_id = sessions
            .into_iter()
            .map(|session| (session.id.clone(), session))
            .collect::<HashMap<_, _>>();
        let mut ordered = Vec::new();
        for session_id in &self.session_order {
            if let Some(session) = by_id.remove(session_id) {
                ordered.push(session);
            } else if let Some(info) = self.disconnected_session_info(session_id) {
                // Keep disconnected tabs in the strip for reconnect (Tauri parity).
                ordered.push(info);
            }
        }
        ordered.extend(by_id.into_values());
        ordered
    }

    /// True when this session is a secondary leaf inside another tab's pane tree
    /// (Tauri: multiple SessionPanes under one Tab, only one strip entry).
    pub(in crate::features) fn is_secondary_pane_session(&self, session_id: &str) -> bool {
        self.session_tab_owner
            .get(session_id)
            .is_some_and(|owner| owner != session_id)
    }

    /// Owning tab-root session id for a leaf (self when the session is a tab root).
    pub(in crate::features) fn tab_root_for_session(&self, session_id: &str) -> String {
        let mut current = session_id.to_string();
        // Flatten owner chains defensively.
        for _ in 0..8 {
            match self.session_tab_owner.get(&current) {
                Some(owner) if owner != &current => current = owner.clone(),
                _ => break,
            }
        }
        current
    }

    /// Sessions shown in the global tab strip / multi-leaf tab lists (tab roots only).
    pub(in crate::features) fn ordered_tab_sessions(&self) -> Vec<SessionInfo> {
        self.ordered_sessions()
            .into_iter()
            .filter(|session| !self.is_secondary_pane_session(&session.id))
            .collect()
    }

    /// Prefer the currently focused leaf when it belongs to `tab_root`, else the tab root.
    pub(in crate::features) fn active_pane_for_tab_root(&self, tab_root: &str) -> String {
        if let Some(active) = self.active_session_id.as_deref() {
            if self.tab_root_for_session(active) == tab_root {
                return active.to_string();
            }
        }
        if let Some(root) = self.session_pane_roots.get(tab_root) {
            if let Some(first) = root.session_ids().into_iter().next() {
                return first;
            }
        }
        tab_root.to_string()
    }

    pub(in crate::features) fn is_session_disconnected(&self, session_id: &str) -> bool {
        self.session_metadata
            .get(session_id)
            .is_some_and(|metadata| metadata.disconnected)
    }

    pub(in crate::features) fn disconnected_session_info(
        &self,
        session_id: &str,
    ) -> Option<SessionInfo> {
        let metadata = self.session_metadata.get(session_id)?;
        if !metadata.disconnected {
            return None;
        }
        Some(session_info_from_metadata(session_id, metadata))
    }
}

fn session_info_from_metadata(session_id: &str, metadata: &SessionRuntimeMetadata) -> SessionInfo {
    match &metadata.launch_config {
        SessionLaunchConfig::Local(config) => SessionInfo {
            id: session_id.to_string(),
            name: config.name.clone(),
            kind: SessionKind::LocalPty,
            working_dir: config.working_dir.clone(),
            cols: config.cols,
            rows: config.rows,
        },
        SessionLaunchConfig::Ssh(config) => SessionInfo {
            id: session_id.to_string(),
            name: config.name.clone(),
            kind: SessionKind::Ssh,
            working_dir: None,
            cols: config.cols,
            rows: config.rows,
        },
        SessionLaunchConfig::Telnet(config) => SessionInfo {
            id: session_id.to_string(),
            name: config.name.clone(),
            kind: if config.raw_tcp {
                SessionKind::RawTcp
            } else {
                SessionKind::Telnet
            },
            working_dir: None,
            cols: config.cols,
            rows: config.rows,
        },
        SessionLaunchConfig::Serial(config) => SessionInfo {
            id: session_id.to_string(),
            name: config.name.clone(),
            kind: SessionKind::Serial,
            working_dir: None,
            cols: 80,
            rows: 24,
        },
    }
}
