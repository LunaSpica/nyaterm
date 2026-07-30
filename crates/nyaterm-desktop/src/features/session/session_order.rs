use nyaterm_transport::SessionInfo;

use crate::features::NyaTermApp;
use crate::models::SessionRuntimeMetadata;

impl NyaTermApp {
    pub(in crate::features) fn register_session(
        &mut self,
        session_id: &str,
        metadata: SessionRuntimeMetadata,
    ) {
        self.session.register_session_metadata(session_id, metadata);
        let encoding = self.settings.summary().interaction_default_encoding.clone();
        self.terminal.ensure_frame_session(
            session_id.to_string(),
            encoding,
            self.terminal_scrollback_line_limit(),
        );
        self.reconcile_terminal_windows();
        if self.session.restore_is_complete() {
            self.persist_open_tabs();
        }
    }

    /// Tab-root count for chrome (status bar) without allocating SessionInfo.
    pub(in crate::features) fn ordered_tab_session_count(&self) -> usize {
        self.session
            .session_order()
            .iter()
            .filter(|session_id| !self.is_secondary_pane_session(session_id))
            .count()
    }

    /// True when this session is a secondary leaf inside another tab's pane tree
    /// (Tauri: multiple SessionPanes under one Tab, only one strip entry).
    pub(in crate::features) fn is_secondary_pane_session(&self, session_id: &str) -> bool {
        self.shell
            .workspace_tab_owner(session_id)
            .is_some_and(|owner| owner != session_id)
    }

    /// Owning tab-root session id for a leaf (self when the session is a tab root).
    pub(in crate::features) fn tab_root_for_session(&self, session_id: &str) -> String {
        let mut current = session_id.to_string();
        // Flatten owner chains defensively.
        for _ in 0..8 {
            match self.shell.workspace_tab_owner(&current) {
                Some(owner) if owner != current => current = owner.to_string(),
                _ => break,
            }
        }
        current
    }

    /// Sessions shown in the global tab strip / multi-leaf tab lists (tab roots only).
    pub(in crate::features) fn ordered_tab_sessions(&self) -> Vec<SessionInfo> {
        let mut ordered = Vec::with_capacity(self.session.session_order_len());
        let mut seen = std::collections::HashSet::with_capacity(self.session.session_order_len());
        for session_id in self.session.session_order() {
            if !seen.insert(session_id.as_str()) {
                continue;
            }
            if self.is_secondary_pane_session(session_id) {
                continue;
            }
            if let Some(session) = self.session.session_info(session_id) {
                ordered.push(session);
            }
        }
        for (session_id, _) in self.session.metadata_entries() {
            if seen.insert(session_id)
                && !self.is_secondary_pane_session(session_id)
                && let Some(session) = self.session.session_info(session_id)
            {
                ordered.push(session);
            }
        }
        ordered
    }

    /// Prefer the currently focused leaf when it belongs to `tab_root`, else the tab root.
    pub(in crate::features) fn active_pane_for_tab_root(&self, tab_root: &str) -> String {
        if let Some(active) = self.session.active_id()
            && self.tab_root_for_session(active) == tab_root
        {
            return active.to_string();
        }
        if let Some(root) = self.shell.workspace_pane_root(tab_root)
            && let Some(first) = root.session_ids().into_iter().next()
        {
            return first;
        }
        tab_root.to_string()
    }
}
