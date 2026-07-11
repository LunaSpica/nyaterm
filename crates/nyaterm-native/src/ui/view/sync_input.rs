use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn open_sync_groups(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sync_groups_open = true;
        if self.sync_groups_selected_id.is_none() {
            self.sync_groups_selected_id = self.sync_groups.first().map(|group| group.id.clone());
        }
        self.terminal_status = "sync groups opened".to_string();
        window.focus(&self.sync_groups_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn close_sync_groups(&mut self, cx: &mut Context<Self>) {
        self.sync_groups_open = false;
        self.terminal_status = "sync groups closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn create_sync_group(&mut self, cx: &mut Context<Self>) {
        let index = self.sync_groups.len();
        let color = self.next_sync_group_color();
        let group = SyncInputGroup {
            id: format!("sync-group-{}", uuid()),
            name: format!("Sync Group {}", index + 1),
            color,
            session_ids: Vec::new(),
            paused_session_ids: Vec::new(),
            enabled: true,
        };
        self.sync_groups_selected_id = Some(group.id.clone());
        self.sync_groups.push(group);
        self.terminal_status = "sync group created".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn delete_selected_sync_group(&mut self, cx: &mut Context<Self>) {
        let Some(group_id) = self.sync_groups_selected_id.clone() else {
            self.terminal_status = "no sync group selected".to_string();
            cx.notify();
            return;
        };
        self.sync_groups.retain(|group| group.id != group_id);
        self.sync_groups_selected_id = self.sync_groups.first().map(|group| group.id.clone());
        self.terminal_status = "sync group deleted".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn select_sync_group(
        &mut self,
        group_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.sync_groups.iter().any(|group| group.id == group_id) {
            self.sync_groups_selected_id = Some(group_id);
            self.terminal_status = "sync group selected".to_string();
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn toggle_selected_sync_group_enabled(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(group) = self.selected_sync_group_mut() else {
            self.terminal_status = "no sync group selected".to_string();
            cx.notify();
            return;
        };
        group.enabled = !group.enabled;
        self.terminal_status = if group.enabled {
            "sync group enabled".to_string()
        } else {
            "sync group disabled".to_string()
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_session_in_selected_sync_group(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(group) = self.selected_sync_group_mut() else {
            self.terminal_status = "create or select a sync group first".to_string();
            cx.notify();
            return;
        };
        if group.session_ids.iter().any(|id| id == &session_id) {
            group.session_ids.retain(|id| id != &session_id);
            group.paused_session_ids.retain(|id| id != &session_id);
            self.terminal_status = "session removed from sync group".to_string();
        } else {
            group.session_ids.push(session_id);
            self.terminal_status = "session added to sync group".to_string();
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_session_paused_in_selected_sync_group(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(group) = self.selected_sync_group_mut() else {
            self.terminal_status = "create or select a sync group first".to_string();
            cx.notify();
            return;
        };
        if !group.session_ids.iter().any(|id| id == &session_id) {
            self.terminal_status = "session is not in the selected sync group".to_string();
            cx.notify();
            return;
        }
        if group.paused_session_ids.iter().any(|id| id == &session_id) {
            group.paused_session_ids.retain(|id| id != &session_id);
            self.terminal_status = "session sync resumed".to_string();
        } else {
            group.paused_session_ids.push(session_id);
            self.terminal_status = "session sync paused".to_string();
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn sync_peer_session_ids(&self, session_id: &str) -> Vec<String> {
        let live_ids = self
            .session_manager
            .list_sessions()
            .unwrap_or_default()
            .into_iter()
            .map(|session| session.id)
            .collect::<HashSet<_>>();
        let mut peers = HashSet::new();
        for group in &self.sync_groups {
            if !group.enabled
                || !group.session_ids.iter().any(|id| id == session_id)
                || group.paused_session_ids.iter().any(|id| id == session_id)
            {
                continue;
            }
            for peer_id in &group.session_ids {
                if peer_id != session_id
                    && live_ids.contains(peer_id)
                    && !group.paused_session_ids.iter().any(|id| id == peer_id)
                {
                    peers.insert(peer_id.clone());
                }
            }
        }
        // Tauri broadcastToAll: fan-out to every other live session.
        if self.broadcast_to_all {
            for peer_id in &live_ids {
                if peer_id != session_id {
                    peers.insert(peer_id.clone());
                }
            }
        }
        let mut peers = peers.into_iter().collect::<Vec<_>>();
        peers.sort();
        peers
    }

    pub(in crate::ui::view) fn toggle_broadcast_to_all(&mut self, cx: &mut Context<Self>) {
        self.broadcast_to_all = !self.broadcast_to_all;
        self.terminal_status = if self.broadcast_to_all {
            "broadcast to all sessions enabled".to_string()
        } else {
            "broadcast to all sessions disabled".to_string()
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn active_sync_group_label(&self, session_id: &str) -> Option<String> {
        self.active_sync_group_for_session(session_id)
            .filter(|group| !group.paused_session_ids.iter().any(|id| id == session_id))
            .map(|group| group.name.clone())
    }

    /// First enabled group that includes this session (paused still counts for chrome).
    /// Matches Tauri `getActiveGroupForSession`.
    pub(in crate::ui::view) fn active_sync_group_for_session(
        &self,
        session_id: &str,
    ) -> Option<&SyncInputGroup> {
        self.sync_groups.iter().find(|group| {
            group.enabled && group.session_ids.iter().any(|id| id == session_id)
        })
    }

    pub(in crate::ui::view) fn is_session_paused_in_active_sync_group(
        &self,
        session_id: &str,
    ) -> bool {
        self.active_sync_group_for_session(session_id)
            .is_some_and(|group| group.paused_session_ids.iter().any(|id| id == session_id))
    }

    /// Pause/resume the current session inside its active enabled sync group.
    pub(in crate::ui::view) fn toggle_session_paused_in_active_sync_group(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(group_id) = self
            .active_sync_group_for_session(&session_id)
            .map(|group| group.id.clone())
        else {
            self.terminal_status = "session is not in an active sync group".to_string();
            cx.notify();
            return;
        };
        let Some(group) = self.sync_groups.iter_mut().find(|group| group.id == group_id) else {
            self.terminal_status = "sync group not found".to_string();
            cx.notify();
            return;
        };
        if group.paused_session_ids.iter().any(|id| id == &session_id) {
            group.paused_session_ids.retain(|id| id != &session_id);
            self.terminal_status = "session sync resumed".to_string();
        } else {
            group.paused_session_ids.push(session_id);
            self.terminal_status = "session sync paused".to_string();
        }
        cx.notify();
    }

    /// Remove session from its active enabled sync group (Tauri Leave).
    pub(in crate::ui::view) fn leave_active_sync_group(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(group_id) = self
            .active_sync_group_for_session(&session_id)
            .map(|group| group.id.clone())
        else {
            self.terminal_status = "session is not in an active sync group".to_string();
            cx.notify();
            return;
        };
        if let Some(group) = self.sync_groups.iter_mut().find(|group| group.id == group_id) {
            group.session_ids.retain(|id| id != &session_id);
            group.paused_session_ids.retain(|id| id != &session_id);
        }
        self.sync_groups
            .retain(|group| !group.session_ids.is_empty());
        if self
            .sync_groups_selected_id
            .as_deref()
            .is_some_and(|id| !self.sync_groups.iter().any(|group| group.id == id))
        {
            self.sync_groups_selected_id = self.sync_groups.first().map(|group| group.id.clone());
        }
        self.terminal_status = "left sync group".to_string();
        cx.notify();
    }

    /// Disable the active enabled sync group without deleting it (Tauri Close Group).
    pub(in crate::ui::view) fn close_active_sync_group_for_session(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(group_id) = self
            .active_sync_group_for_session(&session_id)
            .map(|group| group.id.clone())
        else {
            self.terminal_status = "session is not in an active sync group".to_string();
            cx.notify();
            return;
        };
        if let Some(group) = self.sync_groups.iter_mut().find(|group| group.id == group_id) {
            group.enabled = false;
        }
        self.terminal_status = "sync group closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn purge_session_from_sync_groups(&mut self, session_id: &str) {
        for group in &mut self.sync_groups {
            group.session_ids.retain(|id| id != session_id);
            group.paused_session_ids.retain(|id| id != session_id);
        }
        self.sync_groups
            .retain(|group| !group.session_ids.is_empty());
        if self
            .sync_groups_selected_id
            .as_deref()
            .is_some_and(|id| !self.sync_groups.iter().any(|group| group.id == id))
        {
            self.sync_groups_selected_id = self.sync_groups.first().map(|group| group.id.clone());
        }
    }

    pub(in crate::ui::view) fn selected_sync_group(&self) -> Option<&SyncInputGroup> {
        self.sync_groups_selected_id
            .as_deref()
            .and_then(|id| self.sync_groups.iter().find(|group| group.id == id))
    }

    fn selected_sync_group_mut(&mut self) -> Option<&mut SyncInputGroup> {
        let selected_id = self.sync_groups_selected_id.clone()?;
        self.sync_groups
            .iter_mut()
            .find(|group| group.id == selected_id)
    }

    fn next_sync_group_color(&self) -> u32 {
        SYNC_GROUP_COLORS
            .iter()
            .copied()
            .find(|color| self.sync_groups.iter().all(|group| group.color != *color))
            .unwrap_or(SYNC_GROUP_COLORS[self.sync_groups.len() % SYNC_GROUP_COLORS.len()])
    }
}
