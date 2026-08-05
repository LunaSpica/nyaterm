use gpui::{Context, div, prelude::*};
use nyaterm_transport::SessionKind;

use super::super::terminal_action_prompt_text;
use super::{CompactTabActionsMenuState, TabActionCapabilities};
use crate::features::NyaTermApp;
use crate::models::SessionLaunchConfig;

impl NyaTermApp {
    pub(in crate::features) fn tab_action_can_spawn_session(&self, session_id: &str) -> bool {
        self.session.metadata(session_id).is_some_and(|metadata| {
            matches!(metadata.launch_config, SessionLaunchConfig::Local(_))
                || metadata
                    .source_connection_id
                    .as_deref()
                    .is_some_and(|id| !id.trim().is_empty())
        })
    }

    pub(super) fn tab_action_can_show_session_info(&self, session_id: &str) -> bool {
        self.session
            .metadata(session_id)
            .and_then(|metadata| metadata.source_connection_id.as_deref())
            .is_some_and(|id| !id.trim().is_empty())
    }

    pub(in crate::features) fn tab_actions_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let palette = self.theme_palette();
        let Some(tab_root_id) = self
            .session
            .dialog_tab_actions_session_id()
            .map(str::to_string)
        else {
            return div().into_any_element();
        };
        let sessions = self.session.ordered_sessions();
        if !sessions.iter().any(|session| session.id == tab_root_id) {
            self.session.dialog_close_tab_actions();
            return div().into_any_element();
        }

        let session_id = self.active_pane_for_tab_root(&tab_root_id);
        let Some(active_session) = sessions
            .iter()
            .find(|session| session.id == session_id)
            .cloned()
        else {
            self.session.dialog_close_tab_actions();
            return div().into_any_element();
        };
        let active_color = self.session.tab_color(&tab_root_id);
        let locked = self.tab_tree_is_locked(&tab_root_id);
        let can_copy_ssh = self.session.ssh_address(&session_id).is_some();
        let busy_action = self.session.busy_action(&session_id).map(str::to_string);
        let is_busy = busy_action.is_some();
        let is_disconnected = self.session.is_disconnected(&session_id);
        let can_spawn_session = self.tab_action_can_spawn_session(&session_id);
        let can_session_info = self.tab_action_can_show_session_info(&session_id);
        let can_multiplex = active_session.kind == SessionKind::Ssh && !is_busy && !is_disconnected;
        let can_reconnect =
            is_disconnected && can_spawn_session && !is_busy && !self.session.start_has_pending();
        let can_disconnect = !is_busy && !is_disconnected;
        let can_use_ai = !is_busy && !is_disconnected;
        let tab_sessions = self.ordered_tab_sessions();
        let can_close_inactive = tab_sessions.len() > 1;
        let can_close_right = tab_sessions
            .iter()
            .position(|session| session.id == tab_root_id)
            .is_some_and(|index| index + 1 < tab_sessions.len());
        let can_unsplit = self
            .shell
            .workspace_pane_root(&tab_root_id)
            .is_some_and(|root| root.is_split())
            || self.shell.workspace_split().is_some();
        let scroll_offset = self.terminal.session_scroll_offset(&session_id);
        let visible_for_ai = terminal_action_prompt_text(
            &self
                .terminal_snapshot_for_session(Some(session_id.as_str()), scroll_offset)
                .rows()
                .iter()
                .map(|row| row.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            2_800,
        );
        let buffer_for_ai =
            terminal_action_prompt_text(self.terminal_buffer_tail_for_session(&session_id), 4_000);

        self.compact_tab_actions_menu(
            palette,
            CompactTabActionsMenuState {
                session_id,
                tab_root_id,
                active_color,
                locked,
                capabilities: TabActionCapabilities {
                    can_copy_ssh,
                    can_spawn_session,
                    can_multiplex,
                    can_reconnect,
                    can_disconnect,
                    can_use_ai,
                    can_session_info,
                    can_close_inactive,
                    can_close_right,
                    can_unsplit,
                },
                visible_for_ai,
                buffer_for_ai,
            },
            cx,
        )
    }
}
