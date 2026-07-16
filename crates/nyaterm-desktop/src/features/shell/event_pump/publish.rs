use super::*;

impl NyaTermApp {
    pub(in crate::features) fn publish_store_snapshots(&mut self, cx: &mut Context<Self>) {
        self.publish_store_snapshots_with_scope(cx, true);
    }

    pub(super) fn publish_store_snapshots_with_scope(
        &mut self,
        cx: &mut Context<Self>,
        include_sideband: bool,
    ) {
        self.terminal_runtime.last_store_snapshot_publish_at = Some(Instant::now());

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
            terminal_windows_active: self.terminal_windows.is_some(),
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
            terminal_view_count: self.terminal_views.len(),
            pending_start_count,
            host_prompt_active: self.active_host_key_prompt.is_some(),
            credential_prompt_active: self.active_credential_prompt.is_some(),
            zmodem_session_count: self.zmodem_sessions.len(),
        };

        let overlays = crate::entities::OverlaySnapshot {
            quick_switch_open: self.quick_switch_open,
            tab_actions_open: self.tab_actions_session_id.is_some(),
            rename_open: self.rename_session_id.is_some(),
            color_picker_open: self.color_picker_open,
            session_info_open: self.session_info_open,
            startup_command_open: self.startup_command_open,
            temporary_ssh_link_open: self.temporary_ssh_link_open,
            multi_line_paste_open: self.multi_line_paste.is_some(),
            terminal_actions_open: self.terminal_actions_open,
            terminal_context_menu_open: self.terminal_context_menu.is_some(),
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

        if !include_sideband {
            return;
        }

        let settings = crate::entities::SettingsSnapshot {
            active_tab: self.settings_active_tab.label().to_string(),
            has_master_password: self.settings.has_master_password,
            security_unlocked: self.security_secrets_unlocked,
            cloud_sync_enabled: self.cloud_sync_settings.enabled,
            startup_restore: self.settings.startup_restore,
        };
        let connections = crate::entities::ConnectionsSnapshot {
            connection_count: self.connections.len(),
            group_count: self.connection_groups.len(),
            search_active: !self.connection_search_draft.trim().is_empty(),
            editor_open: self.connection_editor.is_some(),
            group_editor_open: self.connection_group_editor.is_some(),
            delete_confirm_open: self.connection_delete_confirm.is_some()
                || self.connection_group_delete_confirm.is_some(),
            sort_mode: format!("{:?}", self.connection_sort_mode),
        };
        let active_job_count = self
            .transfer_jobs
            .iter()
            .filter(|job| {
                !matches!(
                    job.status,
                    TransferJobStatus::Completed
                        | TransferJobStatus::Failed
                        | TransferJobStatus::Cancelled
                )
            })
            .count();
        let transfers = crate::entities::TransferSnapshot {
            job_count: self.transfer_jobs.len(),
            active_job_count,
            browser_path: self.transfer_browser_path.clone(),
            selected_count: self.transfer_selected_remote_paths.len(),
            browser_busy: self.transfer_browser_home_dir_pending
                || self.transfer_path_prompt.is_some(),
        };
        let ai = crate::entities::AiSnapshot {
            chat_pending: self.ai_chat_pending,
            message_count: self.ai_chat_messages.len(),
            session_id: self.ai_chat_session_id.clone(),
            agent_active: self.ai_agent_loop.is_some(),
        };
        let cloud_sync = crate::entities::CloudSyncSnapshot {
            enabled: self.cloud_sync_settings.enabled,
            provider: self.cloud_sync_settings.provider.clone(),
            conflict_active: self.cloud_sync_conflict.is_some(),
            last_status: self.cloud_sync_status.clone(),
        };
        let remote_ops = crate::entities::RemoteOpsSnapshot {
            process_count: self.processes.len(),
            docker_tab: self.docker_tab.label().to_string(),
            stats_ready: self.remote_stats.is_some(),
            confirm_open: self.docker_confirm.is_some() || self.process_signal_confirm.is_some(),
        };

        self.stores.settings.update(cx, |store, cx| {
            if store.replace_snapshot(settings) {
                cx.notify();
            }
        });
        self.stores.connections.update(cx, |store, cx| {
            if store.replace_snapshot(connections) {
                cx.notify();
            }
        });
        self.stores.transfers.update(cx, |store, cx| {
            if store.replace_snapshot(transfers) {
                cx.notify();
            }
        });
        self.stores.ai.update(cx, |store, cx| {
            if store.replace_snapshot(ai) {
                cx.notify();
            }
        });
        self.stores.cloud_sync.update(cx, |store, cx| {
            if store.replace_snapshot(cloud_sync) {
                cx.notify();
            }
        });
        self.stores.remote_ops.update(cx, |store, cx| {
            if store.replace_snapshot(remote_ops) {
                cx.notify();
            }
        });
    }
}
