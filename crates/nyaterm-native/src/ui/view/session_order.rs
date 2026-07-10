use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn register_session(
        &mut self,
        session_id: &str,
        metadata: SessionRuntimeMetadata,
    ) {
        if !self.session_order.iter().any(|id| id == session_id) {
            self.session_order.push(session_id.to_string());
        }
        self.session_metadata
            .insert(session_id.to_string(), metadata);
        self.terminal_views
            .entry(session_id.to_string())
            .or_insert_with(TerminalViewState::new);
    }

    pub(in crate::ui::view) fn move_session_after(
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

    pub(in crate::ui::view) fn move_session_to_index(&mut self, session_id: &str, index: usize) {
        let Some(current_index) = self.session_order.iter().position(|id| id == session_id) else {
            return;
        };
        let session_id = self.session_order.remove(current_index);
        let index = index.min(self.session_order.len());
        self.session_order.insert(index, session_id);
    }

    pub(in crate::ui::view) fn ordered_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.session_manager.list_sessions().unwrap_or_default();
        let mut by_id = sessions
            .into_iter()
            .map(|session| (session.id.clone(), session))
            .collect::<HashMap<_, _>>();
        let mut ordered = Vec::new();
        for session_id in &self.session_order {
            if let Some(session) = by_id.remove(session_id) {
                ordered.push(session);
            }
        }
        ordered.extend(by_id.into_values());
        ordered
    }
}
