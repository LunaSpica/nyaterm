use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub active_session_id: Option<String>,
    pub ordered_session_ids: Vec<String>,
    pub live_session_ids: Vec<String>,
    pub metadata_count: usize,
    pub terminal_view_count: usize,
    pub pending_start_count: usize,
    pub host_prompt_active: bool,
    pub credential_prompt_active: bool,
    pub zmodem_session_count: usize,
}

#[derive(Debug, Default)]
pub struct SessionStore {
    live_sessions: HashSet<String>,
    active_session_id: Option<String>,
    ordered_session_ids: Vec<String>,
    snapshot: Option<SessionSnapshot>,
}

impl SessionStore {
    pub fn live_session_count(&self) -> usize {
        self.live_sessions.len()
    }

    pub fn is_live(&self, session_id: &str) -> bool {
        self.live_sessions.contains(session_id)
    }

    pub fn active_session_id(&self) -> Option<&str> {
        self.active_session_id.as_deref()
    }

    pub fn ordered_session_ids(&self) -> &[String] {
        &self.ordered_session_ids
    }

    pub fn mark_live(&mut self, session_id: impl Into<String>) {
        self.live_sessions.insert(session_id.into());
    }

    pub fn mark_closed(&mut self, session_id: &str) {
        self.live_sessions.remove(session_id);
    }

    pub fn snapshot(&self) -> Option<&SessionSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn replace_snapshot(&mut self, snapshot: SessionSnapshot) -> bool {
        if self.snapshot.as_ref() == Some(&snapshot) {
            return false;
        }
        self.active_session_id = snapshot.active_session_id.clone();
        self.ordered_session_ids = snapshot.ordered_session_ids.clone();
        self.live_sessions = snapshot.live_session_ids.iter().cloned().collect();
        self.snapshot = Some(snapshot);
        true
    }

    pub fn activate(&mut self, session_id: impl Into<String>) -> bool {
        let session_id = session_id.into();
        if self.active_session_id.as_deref() == Some(session_id.as_str()) {
            return false;
        }
        self.active_session_id = Some(session_id.clone());
        let snapshot = self.snapshot.get_or_insert_with(SessionSnapshot::default);
        snapshot.active_session_id = Some(session_id);
        true
    }

    pub fn set_ordered_session_ids(&mut self, ordered_session_ids: Vec<String>) -> bool {
        if self.ordered_session_ids == ordered_session_ids {
            return false;
        }
        self.ordered_session_ids = ordered_session_ids.clone();
        let snapshot = self.snapshot.get_or_insert_with(SessionSnapshot::default);
        snapshot.ordered_session_ids = ordered_session_ids;
        true
    }

    pub fn move_session_to_index(&mut self, session_id: &str, index: usize) -> bool {
        let Some(current_index) = self
            .ordered_session_ids
            .iter()
            .position(|id| id == session_id)
        else {
            return false;
        };
        let session_id = self.ordered_session_ids.remove(current_index);
        let index = index.min(self.ordered_session_ids.len());
        self.ordered_session_ids.insert(index, session_id);
        let snapshot = self.snapshot.get_or_insert_with(SessionSnapshot::default);
        snapshot.ordered_session_ids = self.ordered_session_ids.clone();
        true
    }

    pub fn remove_session(&mut self, session_id: &str) -> bool {
        let before_len = self.ordered_session_ids.len();
        self.ordered_session_ids.retain(|id| id != session_id);
        self.live_sessions.remove(session_id);
        if self.active_session_id.as_deref() == Some(session_id) {
            self.active_session_id = None;
        }
        let changed = before_len != self.ordered_session_ids.len();
        if changed {
            let snapshot = self.snapshot.get_or_insert_with(SessionSnapshot::default);
            snapshot.ordered_session_ids = self.ordered_session_ids.clone();
            snapshot.live_session_ids = self.live_sessions.iter().cloned().collect();
            snapshot.active_session_id = self.active_session_id.clone();
        }
        changed
    }
}

impl Default for SessionSnapshot {
    fn default() -> Self {
        Self {
            active_session_id: None,
            ordered_session_ids: Vec::new(),
            live_session_ids: Vec::new(),
            metadata_count: 0,
            terminal_view_count: 0,
            pending_start_count: 0,
            host_prompt_active: false,
            credential_prompt_active: false,
            zmodem_session_count: 0,
        }
    }
}
