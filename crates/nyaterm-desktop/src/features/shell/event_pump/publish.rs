use super::*;

use crate::models::MainMode;

impl NyaTermApp {
    pub(in crate::features) fn publish_store_snapshots(&mut self, cx: &mut Context<Self>) {
        self.publish_store_snapshots_with_scope(cx, true);
    }

    pub(super) fn publish_store_snapshots_with_scope(
        &mut self,
        cx: &mut Context<Self>,
        include_sideband: bool,
    ) {
        // Source of truth: NyaTermApp / FeatureState. Entity stores receive
        // one-way read-model snapshots here; they do not drive app mutations.
        if !include_sideband && self.published_core_store_snapshots_are_current(cx) {
            return;
        }
        self.terminal.view.runtime.last_store_snapshot_publish_at = Some(Instant::now());

        let workspace = crate::entities::WorkspaceSnapshot {
            active_session_id: self.active_session_id.clone(),
            // Local tab-root order only; avoid SessionManager::list_sessions on the UI tick.
            ordered_tab_roots: self
                .session_order
                .iter()
                .filter(|session_id| !self.is_secondary_pane_session(session_id))
                .cloned()
                .collect(),
            selected_nav: self.selected_nav.label().to_string(),
            main_mode: match self.main_mode {
                MainMode::Workspace => "Workspace",
                MainMode::Page => "Page",
            }
            .to_string(),
            active_left_panel: self.active_left_panel.map(|item| item.label().to_string()),
            active_right_panel: self.active_right_panel.map(|item| item.label().to_string()),
            left_sidebar_collapsed: self.left_sidebar_collapsed,
            right_inspector_collapsed: self.right_inspector_collapsed,
            workspace_split_active: self.workspace_split.is_some(),
            terminal_windows_active: self.terminal.windows.tree.is_some(),
        };

        // Prefer local metadata over SessionManager::list_sessions so publish
        // never takes the transport session map lock on the UI tick.
        let live_session_ids = self
            .session_metadata
            .iter()
            .filter(|(_, metadata)| !metadata.disconnected)
            .map(|(session_id, _)| session_id.clone())
            .collect();
        let pending_start_count =
            self.pending_session_starts.len() + self.pending_saved_connection_queue.len();
        let sessions = crate::entities::SessionSnapshot {
            active_session_id: self.active_session_id.clone(),
            ordered_session_ids: self.session_order.clone(),
            live_session_ids,
            metadata_count: self.session_metadata.len(),
            terminal_view_count: self.terminal.view.views.len(),
            pending_start_count,
            host_prompt_active: self.active_host_key_prompt.is_some(),
            credential_prompt_active: self.active_credential_prompt.is_some()
                || self.active_keyboard_interactive_prompt.is_some(),
            zmodem_session_count: self.zmodem_sessions.len(),
        };

        let overlays = crate::entities::OverlaySnapshot {
            tab_actions_open: self.tab_actions_session_id.is_some(),
            rename_open: self.rename_session_id.is_some(),
            color_picker_open: self.color_picker_open,
            session_info_open: self.session_info_open,
            startup_command_open: self.startup_command_open,
            temporary_ssh_link_open: self.temporary_ssh_link_open,
            multi_line_paste_open: self.multi_line_paste.is_some(),
            terminal_actions_open: self.terminal.menus.actions_open,
            terminal_context_menu_open: self.terminal.menus.context_menu.is_some(),
            action_link_menu_open: self.action_link_menu.is_some(),
            action_link_tooltip_open: self.action_link_tooltip.is_some(),
            command_suggestions_open: self.command_suggestions.is_some(),
            credential_suggestions_open: self.credential_suggestions.is_some(),
            close_all_sessions_confirm_open: self.close_all_sessions_confirm_open,
            locked: self.is_locked,
        };

        self.stores.workspace.update(cx, |store, cx| {
            if store.replace_snapshot(workspace) {
                cx.notify();
            }
        });
        self.stores.sessions.update(cx, |store, cx| {
            if store.replace_snapshot(sessions) {
                cx.notify();
            }
        });
        self.stores.overlays.update(cx, |store, cx| {
            if store.replace_snapshot(overlays) {
                cx.notify();
            }
        });
    }

    fn published_core_store_snapshots_are_current(&self, cx: &mut Context<Self>) -> bool {
        let workspace_store = self.stores.workspace.clone();
        let sessions_store = self.stores.sessions.clone();
        let overlays_store = self.stores.overlays.clone();
        let workspace_current = workspace_store.read_with(cx, |store, _| {
            store
                .snapshot()
                .is_some_and(|snapshot| self.workspace_snapshot_is_current(snapshot))
        });
        let sessions_current =
            sessions_store.read_with(cx, |store, _| self.session_store_snapshot_is_current(store));
        let overlays_current = overlays_store.read_with(cx, |store, _| {
            store
                .snapshot()
                .is_some_and(|snapshot| self.overlay_snapshot_is_current(snapshot))
        });
        workspace_current && sessions_current && overlays_current
    }

    fn workspace_snapshot_is_current(&self, snapshot: &crate::entities::WorkspaceSnapshot) -> bool {
        let ordered_tab_roots_current = snapshot.ordered_tab_roots.len()
            == self
                .session_order
                .iter()
                .filter(|session_id| !self.is_secondary_pane_session(session_id))
                .count()
            && snapshot
                .ordered_tab_roots
                .iter()
                .map(String::as_str)
                .eq(self
                    .session_order
                    .iter()
                    .filter(|session_id| !self.is_secondary_pane_session(session_id))
                    .map(String::as_str));
        snapshot.active_session_id == self.active_session_id
            && ordered_tab_roots_current
            && snapshot.selected_nav == self.selected_nav.label()
            && snapshot.main_mode
                == match self.main_mode {
                    MainMode::Workspace => "Workspace",
                    MainMode::Page => "Page",
                }
            && snapshot.active_left_panel.as_deref()
                == self.active_left_panel.map(|item| item.label())
            && snapshot.active_right_panel.as_deref()
                == self.active_right_panel.map(|item| item.label())
            && snapshot.left_sidebar_collapsed == self.left_sidebar_collapsed
            && snapshot.right_inspector_collapsed == self.right_inspector_collapsed
            && snapshot.workspace_split_active == self.workspace_split.is_some()
            && snapshot.terminal_windows_active == self.terminal.windows.tree.is_some()
    }

    fn session_store_snapshot_is_current(&self, store: &crate::entities::SessionStore) -> bool {
        let Some(snapshot) = store.snapshot() else {
            return false;
        };
        let live_session_count = self
            .session_metadata
            .values()
            .filter(|metadata| !metadata.disconnected)
            .count();
        snapshot.active_session_id == self.active_session_id
            && store.ordered_session_ids() == self.session_order.as_slice()
            && store.live_session_count() == live_session_count
            && self
                .session_metadata
                .iter()
                .filter(|(_, metadata)| !metadata.disconnected)
                .all(|(session_id, _)| store.is_live(session_id))
            && snapshot.metadata_count == self.session_metadata.len()
            && snapshot.terminal_view_count == self.terminal.view.views.len()
            && snapshot.pending_start_count
                == self.pending_session_starts.len() + self.pending_saved_connection_queue.len()
            && snapshot.host_prompt_active == self.active_host_key_prompt.is_some()
            && snapshot.credential_prompt_active
                == (self.active_credential_prompt.is_some()
                    || self.active_keyboard_interactive_prompt.is_some())
            && snapshot.zmodem_session_count == self.zmodem_sessions.len()
    }

    fn overlay_snapshot_is_current(&self, snapshot: &crate::entities::OverlaySnapshot) -> bool {
        snapshot.tab_actions_open == self.tab_actions_session_id.is_some()
            && snapshot.rename_open == self.rename_session_id.is_some()
            && snapshot.color_picker_open == self.color_picker_open
            && snapshot.session_info_open == self.session_info_open
            && snapshot.startup_command_open == self.startup_command_open
            && snapshot.temporary_ssh_link_open == self.temporary_ssh_link_open
            && snapshot.multi_line_paste_open == self.multi_line_paste.is_some()
            && snapshot.terminal_actions_open == self.terminal.menus.actions_open
            && snapshot.terminal_context_menu_open == self.terminal.menus.context_menu.is_some()
            && snapshot.action_link_menu_open == self.action_link_menu.is_some()
            && snapshot.action_link_tooltip_open == self.action_link_tooltip.is_some()
            && snapshot.command_suggestions_open == self.command_suggestions.is_some()
            && snapshot.credential_suggestions_open == self.credential_suggestions.is_some()
            && snapshot.close_all_sessions_confirm_open == self.close_all_sessions_confirm_open
            && snapshot.locked == self.is_locked
    }
}
