//! GPUI entity-state boundaries for the native shell.

use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct WorkspaceStore {
    active_session_id: Option<String>,
    ordered_tab_roots: Vec<String>,
}

impl WorkspaceStore {
    pub fn active_session_id(&self) -> Option<&str> {
        self.active_session_id.as_deref()
    }

    pub fn ordered_tab_roots(&self) -> &[String] {
        &self.ordered_tab_roots
    }

    pub fn activate_session(&mut self, session_id: impl Into<String>) {
        self.active_session_id = Some(session_id.into());
    }

    pub fn set_ordered_tab_roots(&mut self, roots: Vec<String>) {
        self.ordered_tab_roots = roots;
    }
}

#[derive(Debug, Default)]
pub struct SessionStore {
    live_sessions: HashSet<String>,
}

impl SessionStore {
    pub fn live_session_count(&self) -> usize {
        self.live_sessions.len()
    }

    pub fn is_live(&self, session_id: &str) -> bool {
        self.live_sessions.contains(session_id)
    }

    pub fn mark_live(&mut self, session_id: impl Into<String>) {
        self.live_sessions.insert(session_id.into());
    }

    pub fn mark_closed(&mut self, session_id: &str) {
        self.live_sessions.remove(session_id);
    }
}

#[derive(Debug, Default)]
pub struct SettingsStore;

#[derive(Debug, Default)]
pub struct ConnectionsStore;

#[derive(Debug, Default)]
pub struct TransferStore;

#[derive(Debug, Default)]
pub struct AiStore;

#[derive(Debug, Default)]
pub struct CloudSyncStore;

#[derive(Debug, Default)]
pub struct RemoteOpsStore;

#[derive(Debug, Default)]
pub struct OverlayStore;

#[cfg(test)]
mod tests {
    use super::{SessionStore, WorkspaceStore};

    #[test]
    fn workspace_store_tracks_active_session_and_tab_order() {
        let mut store = WorkspaceStore::default();
        store.activate_session("session-a");
        store.set_ordered_tab_roots(vec!["session-a".into(), "session-b".into()]);

        assert_eq!(store.active_session_id(), Some("session-a"));
        assert_eq!(store.ordered_tab_roots(), ["session-a", "session-b"]);
    }

    #[test]
    fn session_store_tracks_live_sessions() {
        let mut store = SessionStore::default();
        store.mark_live("session-a");
        store.mark_live("session-b");
        store.mark_closed("session-a");

        assert!(!store.is_live("session-a"));
        assert!(store.is_live("session-b"));
        assert_eq!(store.live_session_count(), 1);
    }
}
