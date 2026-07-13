use super::*;

impl NyaTermApp {
    pub(in crate::features) fn status_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let tab_count = self.ordered_tab_sessions().len();
        let pane_count = self.ordered_sessions().len();
        let session_status = if let Some(pending) = self.pending_session_name.as_ref() {
            format!("connecting {pending}")
        } else if let Some(session_id) = self.active_session_id.as_deref() {
            let tab_root = self.tab_root_for_session(session_id);
            let leaf_name = self
                .session_display_name(session_id)
                .unwrap_or_else(|| short_id(session_id).to_string());
            let mut status = if tab_root != session_id {
                let tab_name = self
                    .session_display_name(&tab_root)
                    .unwrap_or_else(|| short_id(&tab_root).to_string());
                if tab_name == leaf_name {
                    leaf_name
                } else {
                    format!("{tab_name} › {leaf_name}")
                }
            } else {
                leaf_name
            };
            if let Some(endpoint) = self.session_endpoint(session_id) {
                status = format!("{status} · {endpoint}");
            }
            if self.is_session_disconnected(session_id) {
                status = format!("{status} · disconnected");
            } else if self
                .session_pane_roots
                .get(&tab_root)
                .is_some_and(|root| root.is_split())
            {
                let count = self
                    .session_pane_roots
                    .get(&tab_root)
                    .map(|root| root.session_ids().len())
                    .unwrap_or(1);
                status = format!("{status} · {count}p");
            }
            status
        } else if self.last_connect_failure_name.is_some() {
            "failed".to_string()
        } else {
            "idle".to_string()
        };
        let recording_count = self.recording_manager.list_recording_sessions().len();
        let recording_status = if recording_count == 0 {
            "off".to_string()
        } else {
            format!("{recording_count} active")
        };
        let running_transfers = self
            .transfer_jobs
            .iter()
            .filter(|job| {
                matches!(
                    job.status,
                    TransferJobStatus::Running | TransferJobStatus::Cancelling
                )
            })
            .count();
        let failed_transfers = self
            .transfer_jobs
            .iter()
            .filter(|job| job.status == TransferJobStatus::Failed)
            .count();
        let transfer_status = if running_transfers > 0 {
            format!("{running_transfers} running")
        } else if failed_transfers > 0 {
            format!("{failed_transfers} failed")
        } else if self.transfer_jobs.is_empty() {
            "idle".to_string()
        } else {
            format!("{} job(s)", self.transfer_jobs.len())
        };
        let bottom_panel = match self.bottom_panel {
            BottomPanelMode::QuickCommands => "quick commands",
            BottomPanelMode::CommandSend => "command send",
            BottomPanelMode::Hidden => "hidden",
        };
        let ai_status = if self.ai_agent_loop.is_some() {
            "agent".to_string()
        } else if self.ai_chat_pending {
            "chat".to_string()
        } else if self
            .ai_prepared_request
            .as_ref()
            .is_some_and(|request| request.action == AiAction::CustomFileAction)
        {
            "file".to_string()
        } else if self.ai_settings.enabled {
            match self.ai_settings.default_mode {
                AiMode::Ask => "ask",
                AiMode::Agent => "agent",
            }
            .to_string()
        } else {
            "off".to_string()
        };
        let (cpu_status, mem_status) = self
            .remote_stats
            .as_ref()
            .map(|stats| {
                let memory_total = stats.memory.used.saturating_add(stats.memory.available);
                let memory_percent = if memory_total > 0 {
                    stats.memory.used as f64 / memory_total as f64 * 100.
                } else {
                    0.
                };
                (
                    format!("{:.0}%", stats.cpu.usage),
                    format!("{memory_percent:.0}%"),
                )
            })
            .unwrap_or_else(|| ("n/a".to_string(), "n/a".to_string()));
        let lock_status = if self.is_locked {
            "locked".to_string()
        } else if self.settings.enable_screen_lock && self.settings.idle_lock_minutes > 0 {
            format!("auto {}m", self.settings.idle_lock_minutes)
        } else {
            "manual".to_string()
        };

        let palette = self.theme_palette();
        div()
            .h(px(22.))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap_1()
            .px_2()
            .border_t_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.bg))
            .text_size(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .min_w_0()
                    .child(status_bar_button(
                        palette,
                        "status-session",
                        "Session",
                        session_status,
                        if self.pending_session_name.is_some() {
                            rgb(palette.warning)
                        } else if self
                            .active_session_id
                            .as_deref()
                            .is_some_and(|id| self.is_session_disconnected(id))
                        {
                            rgb(palette.danger)
                        } else if self.active_session_id.is_some() {
                            rgb(palette.success)
                        } else if self.last_connect_failure_name.is_some() {
                            rgb(palette.danger)
                        } else {
                            rgb(palette.text_muted)
                        },
                        cx.listener(|this, _, _, cx| {
                            this.main_mode = MainMode::Workspace;
                            this.terminal_status = "workspace focused".to_string();
                            cx.notify();
                        }),
                    ))
                    .child(status_bar_label(
                        palette,
                        "Tabs",
                        if pane_count > tab_count {
                            format!("{tab_count} ({pane_count}p)")
                        } else {
                            tab_count.to_string()
                        },
                        rgb(palette.accent),
                    ))
                    .child(status_bar_button(
                        palette,
                        "status-recording",
                        "Recording",
                        recording_status,
                        if recording_count > 0 {
                            rgb(palette.danger)
                        } else {
                            rgb(palette.text_muted)
                        },
                        cx.listener(|this, _, _, cx| {
                            this.ensure_panel_open(NavItem::Recording);
                            cx.notify();
                        }),
                    ))
                    .child(status_bar_button(
                        palette,
                        "status-transfer",
                        "Transfer",
                        transfer_status,
                        if running_transfers > 0 {
                            rgb(palette.warning)
                        } else if failed_transfers > 0 {
                            rgb(palette.danger)
                        } else {
                            rgb(palette.text_muted)
                        },
                        cx.listener(|this, _, _, cx| {
                            this.ensure_panel_open(NavItem::Transfers);
                            cx.notify();
                        }),
                    ))
                    .child(status_bar_label(
                        palette,
                        "Panel",
                        bottom_panel,
                        rgb(palette.accent),
                    ))
                    .child(status_bar_button(
                        palette,
                        "status-broadcast",
                        "Broadcast",
                        if self.broadcast_to_all {
                            "on".to_string()
                        } else {
                            "off".to_string()
                        },
                        if self.broadcast_to_all {
                            rgb(palette.warning)
                        } else {
                            rgb(palette.text_muted)
                        },
                        cx.listener(|this, _, _, cx| {
                            this.toggle_broadcast_to_all(cx);
                        }),
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_1()
                    .min_w_0()
                    .child(status_bar_button(
                        palette,
                        "status-ai",
                        "AI",
                        ai_status,
                        if self.ai_settings.enabled {
                            rgb(palette.accent)
                        } else {
                            rgb(palette.text_muted)
                        },
                        cx.listener(|this, _, _, cx| {
                            this.ensure_panel_open(NavItem::AiAssistant);
                            cx.notify();
                        }),
                    ))
                    .child(status_bar_button(
                        palette,
                        "status-cpu",
                        "CPU",
                        cpu_status,
                        rgb(palette.accent),
                        cx.listener(|this, _, _, cx| {
                            this.ensure_panel_open(NavItem::Stats);
                            cx.notify();
                        }),
                    ))
                    .child(status_bar_button(
                        palette,
                        "status-memory",
                        "MEM",
                        mem_status,
                        rgb(palette.accent),
                        cx.listener(|this, _, _, cx| {
                            this.ensure_panel_open(NavItem::Stats);
                            cx.notify();
                        }),
                    ))
                    .child(status_bar_label(
                        palette,
                        "Store",
                        if self.store_status.ready {
                            "online"
                        } else {
                            "offline"
                        },
                        if self.store_status.ready {
                            rgb(palette.success)
                        } else {
                            rgb(palette.danger)
                        },
                    ))
                    .child(status_bar_button(
                        palette,
                        "status-lock",
                        "Lock",
                        lock_status,
                        if self.is_locked {
                            rgb(palette.danger)
                        } else {
                            rgb(palette.text_muted)
                        },
                        cx.listener(|this, _, window, cx| {
                            this.lock_app(window, cx);
                        }),
                    )),
            )
    }

}
