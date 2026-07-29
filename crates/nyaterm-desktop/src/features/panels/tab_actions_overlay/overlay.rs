use gpui::{Context, div, prelude::*};
use nyaterm_transport::SessionKind;

use super::super::terminal_action_prompt_text;
use crate::features::NyaTermApp;
use crate::models::SessionLaunchConfig;

impl NyaTermApp {
    pub(super) fn tab_action_can_spawn_session(&self, session_id: &str) -> bool {
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
        let Some(session_id) = self
            .session
            .dialogs
            .tab_actions_session_id()
            .map(str::to_string)
        else {
            return div().into_any_element();
        };
        let sessions = self.ordered_sessions();
        let Some(session) = sessions
            .iter()
            .find(|session| session.id == session_id)
            .cloned()
        else {
            self.session.dialogs.close_tab_actions();
            return div().into_any_element();
        };

        let display_name = self.session_display_name_by_info(&session);
        let active_color = self.session.tab_color(&session_id);
        let can_copy_ssh = self.session_ssh_address(&session_id).is_some();
        let busy_action = self.session.busy_action(&session_id).map(str::to_string);
        let is_busy = busy_action.is_some();
        let is_disconnected = self.is_session_disconnected(&session_id);
        let can_spawn_session = self.tab_action_can_spawn_session(&session_id);
        let can_session_info = self.tab_action_can_show_session_info(&session_id);
        let can_multiplex = session.kind == SessionKind::Ssh && !is_busy && !is_disconnected;
        let can_reconnect = can_spawn_session && !is_busy && !self.has_pending_session_start();
        let can_disconnect = !is_busy && !is_disconnected;
        let can_use_ai = !is_busy && !is_disconnected;
        let can_close_inactive = sessions.len() > 1;
        let can_close_right = sessions
            .iter()
            .position(|session| session.id == session_id)
            .is_some_and(|index| index + 1 < sessions.len());
        let can_unsplit = self
            .session
            .active_id()
            .map(|id| self.tab_root_for_session(id))
            .and_then(|root| self.shell.workspace.pane_roots.get(&root))
            .is_some_and(|root| root.is_split())
            || self.shell.workspace.split.is_some();
        let scroll_offset = self
            .terminal
            .view
            .views
            .get(&session_id)
            .map(|view| view.scroll_offset)
            .unwrap_or(0);
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
            session_id,
            &session,
            &display_name,
            active_color,
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
            visible_for_ai,
            buffer_for_ai,
            sessions.len(),
            cx,
        )
    }
}
