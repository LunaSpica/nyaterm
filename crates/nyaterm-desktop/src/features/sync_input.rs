use std::collections::HashSet;

use gpui::{Context, Window};
use nyaterm_core::uuid;
use nyaterm_transport::SessionInfo;

use super::{NyaTermApp, SYNC_GROUP_COLORS};
use crate::models::SyncInputGroup;

impl NyaTermApp {
    pub(in crate::features) fn open_sync_groups(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sync_groups_open = true;
        self.sync_groups_search_draft.clear();
        self.forget_text_inputs("sync.groups.search");
        self.forget_text_inputs("sync.group-name.");
        self.sync_groups_delete_pending = None;
        if self.sync_groups_selected_id.is_none() {
            self.sync_groups_selected_id = self.sync_groups.first().map(|group| group.id.clone());
        }
        self.terminal.view.status = "sync groups opened".to_string();
        window.focus(&self.sync_groups_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_sync_groups(&mut self, cx: &mut Context<Self>) {
        self.sync_groups_open = false;
        self.sync_groups_search_draft.clear();
        self.forget_text_inputs("sync.groups.search");
        self.forget_text_inputs("sync.group-name.");
        self.sync_groups_delete_pending = None;
        self.terminal.view.status = "sync groups closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn create_sync_group(&mut self, cx: &mut Context<Self>) {
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
        self.terminal.view.status = "sync group created".to_string();
        cx.notify();
    }

    pub(in crate::features) fn delete_selected_sync_group(&mut self, cx: &mut Context<Self>) {
        let Some(group_id) = self.sync_groups_selected_id.clone() else {
            self.terminal.view.status = "no sync group selected".to_string();
            cx.notify();
            return;
        };
        self.sync_groups.retain(|group| group.id != group_id);
        self.sync_groups_selected_id = self.sync_groups.first().map(|group| group.id.clone());
        self.terminal.view.status = "sync group deleted".to_string();
        cx.notify();
    }

    pub(in crate::features) fn request_delete_selected_sync_group(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(group_id) = self.sync_groups_selected_id.clone() else {
            self.terminal.view.status = "no sync group selected".to_string();
            cx.notify();
            return;
        };
        self.sync_groups_delete_pending = Some(group_id);
        cx.notify();
    }

    pub(in crate::features) fn cancel_delete_sync_group(&mut self, cx: &mut Context<Self>) {
        self.sync_groups_delete_pending = None;
        cx.notify();
    }

    pub(in crate::features) fn confirm_delete_sync_group(&mut self, cx: &mut Context<Self>) {
        let Some(group_id) = self.sync_groups_delete_pending.take() else {
            return;
        };
        self.sync_groups_selected_id = Some(group_id);
        self.delete_selected_sync_group(cx);
    }

    pub(in crate::features) fn apply_sync_groups_search(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.sync_groups_search_draft = text;
        cx.notify();
    }

    pub(in crate::features) fn apply_sync_group_name(
        &mut self,
        group_id: &str,
        text: String,
        cx: &mut Context<Self>,
    ) {
        if update_sync_group_name(&mut self.sync_groups, group_id, text) {
            cx.notify();
        }
    }

    pub(in crate::features) fn select_all_sync_group_sessions(&mut self, cx: &mut Context<Self>) {
        let session_ids = self
            .ordered_sessions()
            .into_iter()
            .map(|session| session.id)
            .collect::<Vec<_>>();
        let Some(group) = self.selected_sync_group_mut() else {
            return;
        };
        group.session_ids = session_ids;
        group.paused_session_ids.clear();
        cx.notify();
    }

    pub(in crate::features) fn clear_sync_group_sessions(&mut self, cx: &mut Context<Self>) {
        let Some(group) = self.selected_sync_group_mut() else {
            return;
        };
        group.session_ids.clear();
        group.paused_session_ids.clear();
        cx.notify();
    }

    pub(in crate::features) fn add_filtered_sync_group_sessions(&mut self, cx: &mut Context<Self>) {
        let query = self.sync_groups_search_draft.trim().to_ascii_lowercase();
        let session_ids = self
            .ordered_sessions()
            .into_iter()
            .filter(|session| self.sync_group_session_matches_search(session, &query))
            .map(|session| session.id)
            .collect::<Vec<_>>();
        let Some(group) = self.selected_sync_group_mut() else {
            return;
        };
        for session_id in session_ids {
            if !group.session_ids.iter().any(|id| id == &session_id) {
                group.session_ids.push(session_id);
            }
        }
        group.paused_session_ids.clear();
        cx.notify();
    }

    pub(in crate::features) fn remove_filtered_sync_group_sessions(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let query = self.sync_groups_search_draft.trim().to_ascii_lowercase();
        let remove_ids = self
            .ordered_sessions()
            .into_iter()
            .filter(|session| self.sync_group_session_matches_search(session, &query))
            .map(|session| session.id)
            .collect::<HashSet<_>>();
        let Some(group) = self.selected_sync_group_mut() else {
            return;
        };
        group.session_ids.retain(|id| !remove_ids.contains(id));
        group
            .paused_session_ids
            .retain(|id| !remove_ids.contains(id));
        cx.notify();
    }

    pub(in crate::features) fn select_same_host_sync_group_sessions(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let selected_ids = self
            .selected_sync_group()
            .map(|group| group.session_ids.clone())
            .unwrap_or_default();
        let selected_hosts = selected_ids
            .iter()
            .filter_map(|id| self.session_ssh_host(id))
            .collect::<HashSet<_>>();
        if selected_hosts.is_empty() {
            return;
        }
        let matching_ids = self
            .ordered_sessions()
            .into_iter()
            .filter(|session| {
                self.session_ssh_host(&session.id)
                    .is_some_and(|host| selected_hosts.contains(&host))
            })
            .map(|session| session.id)
            .collect::<Vec<_>>();
        let Some(group) = self.selected_sync_group_mut() else {
            return;
        };
        group.session_ids = matching_ids;
        group.paused_session_ids.clear();
        cx.notify();
    }

    pub(in crate::features) fn sync_group_session_matches_search(
        &self,
        session: &SessionInfo,
        query: &str,
    ) -> bool {
        if query.is_empty() {
            return true;
        }
        let endpoint = self.session_endpoint(&session.id).unwrap_or_default();
        let host = self.session_ssh_host(&session.id).unwrap_or_default();
        format!(
            "{} {:?} {} {} {}",
            self.session_display_name_by_info(session),
            session.kind,
            endpoint,
            host,
            session.id
        )
        .to_ascii_lowercase()
        .contains(query)
    }

    pub(in crate::features) fn select_sync_group(
        &mut self,
        group_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.sync_groups.iter().any(|group| group.id == group_id) {
            self.sync_groups_selected_id = Some(group_id);
            self.terminal.view.status = "sync group selected".to_string();
            cx.notify();
        }
    }

    pub(in crate::features) fn toggle_selected_sync_group_enabled(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(group) = self.selected_sync_group_mut() else {
            self.terminal.view.status = "no sync group selected".to_string();
            cx.notify();
            return;
        };
        group.enabled = !group.enabled;
        self.terminal.view.status = if group.enabled {
            "sync group enabled".to_string()
        } else {
            "sync group disabled".to_string()
        };
        cx.notify();
    }

    pub(in crate::features) fn toggle_session_in_selected_sync_group(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(group) = self.selected_sync_group_mut() else {
            self.terminal.view.status = "create or select a sync group first".to_string();
            cx.notify();
            return;
        };
        if group.session_ids.iter().any(|id| id == &session_id) {
            group.session_ids.retain(|id| id != &session_id);
            group.paused_session_ids.retain(|id| id != &session_id);
            self.terminal.view.status = "session removed from sync group".to_string();
        } else {
            group.session_ids.push(session_id);
            self.terminal.view.status = "session added to sync group".to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn toggle_session_paused_in_selected_sync_group(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(group) = self.selected_sync_group_mut() else {
            self.terminal.view.status = "create or select a sync group first".to_string();
            cx.notify();
            return;
        };
        if !group.session_ids.iter().any(|id| id == &session_id) {
            self.terminal.view.status = "session is not in the selected sync group".to_string();
            cx.notify();
            return;
        }
        if group.paused_session_ids.iter().any(|id| id == &session_id) {
            group.paused_session_ids.retain(|id| id != &session_id);
            self.terminal.view.status = "session sync resumed".to_string();
        } else {
            group.paused_session_ids.push(session_id);
            self.terminal.view.status = "session sync paused".to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn sync_peer_session_ids(&self, session_id: &str) -> Vec<String> {
        let live_ids = self
            .session_metadata
            .iter()
            .filter(|(_, metadata)| !metadata.disconnected)
            .map(|(session_id, _)| session_id.clone())
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

    pub(in crate::features) fn toggle_broadcast_to_all(&mut self, cx: &mut Context<Self>) {
        self.broadcast_to_all = !self.broadcast_to_all;
        self.terminal.view.status = if self.broadcast_to_all {
            "broadcast to all sessions enabled".to_string()
        } else {
            "broadcast to all sessions disabled".to_string()
        };
        cx.notify();
    }

    /// First enabled group that includes this session (paused still counts for chrome).
    /// Matches Tauri `getActiveGroupForSession`.
    pub(in crate::features) fn active_sync_group_for_session(
        &self,
        session_id: &str,
    ) -> Option<&SyncInputGroup> {
        self.sync_groups
            .iter()
            .find(|group| group.enabled && group.session_ids.iter().any(|id| id == session_id))
    }

    pub(in crate::features) fn is_session_paused_in_active_sync_group(
        &self,
        session_id: &str,
    ) -> bool {
        self.active_sync_group_for_session(session_id)
            .is_some_and(|group| group.paused_session_ids.iter().any(|id| id == session_id))
    }

    /// Pause/resume the current session inside its active enabled sync group.
    pub(in crate::features) fn toggle_session_paused_in_active_sync_group(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(group_id) = self
            .active_sync_group_for_session(&session_id)
            .map(|group| group.id.clone())
        else {
            self.terminal.view.status = "session is not in an active sync group".to_string();
            cx.notify();
            return;
        };
        let Some(group) = self
            .sync_groups
            .iter_mut()
            .find(|group| group.id == group_id)
        else {
            self.terminal.view.status = "sync group not found".to_string();
            cx.notify();
            return;
        };
        if group.paused_session_ids.iter().any(|id| id == &session_id) {
            group.paused_session_ids.retain(|id| id != &session_id);
            self.terminal.view.status = "session sync resumed".to_string();
        } else {
            group.paused_session_ids.push(session_id);
            self.terminal.view.status = "session sync paused".to_string();
        }
        cx.notify();
    }

    /// Remove session from its active enabled sync group (Tauri Leave).
    pub(in crate::features) fn leave_active_sync_group(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(group_id) = self
            .active_sync_group_for_session(&session_id)
            .map(|group| group.id.clone())
        else {
            self.terminal.view.status = "session is not in an active sync group".to_string();
            cx.notify();
            return;
        };
        if let Some(group) = self
            .sync_groups
            .iter_mut()
            .find(|group| group.id == group_id)
        {
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
        self.terminal.view.status = "left sync group".to_string();
        cx.notify();
    }

    /// Disable the active enabled sync group without deleting it (Tauri Close Group).
    pub(in crate::features) fn close_active_sync_group_for_session(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(group_id) = self
            .active_sync_group_for_session(&session_id)
            .map(|group| group.id.clone())
        else {
            self.terminal.view.status = "session is not in an active sync group".to_string();
            cx.notify();
            return;
        };
        if let Some(group) = self
            .sync_groups
            .iter_mut()
            .find(|group| group.id == group_id)
        {
            group.enabled = false;
        }
        self.terminal.view.status = "sync group closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn purge_session_from_sync_groups(&mut self, session_id: &str) {
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

    pub(in crate::features) fn selected_sync_group(&self) -> Option<&SyncInputGroup> {
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

fn update_sync_group_name(groups: &mut [SyncInputGroup], group_id: &str, text: String) -> bool {
    let Some(group) = groups.iter_mut().find(|group| group.id == group_id) else {
        return false;
    };
    group.name = text;
    true
}

#[cfg(test)]
mod tests {
    use super::update_sync_group_name;
    use crate::models::SyncInputGroup;

    fn group(id: &str, name: &str) -> SyncInputGroup {
        SyncInputGroup {
            id: id.to_string(),
            name: name.to_string(),
            color: 0,
            session_ids: Vec::new(),
            paused_session_ids: Vec::new(),
            enabled: true,
        }
    }

    #[test]
    fn group_name_input_updates_only_its_addressed_group() {
        let mut groups = vec![group("one", "One"), group("two", "Two")];

        assert!(update_sync_group_name(
            &mut groups,
            "two",
            "Renamed".to_string()
        ));
        assert_eq!(groups[0].name, "One");
        assert_eq!(groups[1].name, "Renamed");
        assert!(!update_sync_group_name(
            &mut groups,
            "missing",
            "Ignored".to_string()
        ));
    }
}
