use super::*;

impl NyaTermApp {
    pub(in crate::features) fn sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut width = self.left_panel_width.clamp(160., 720.);
        if !cfg!(target_os = "macos") && self.last_viewport_size.0 < 1024. {
            width = width.min((self.last_viewport_size.0 - 80.).max(120.));
        }
        let palette = self.theme_palette();
        div()
            .w(px(width))
            .flex_none()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(rgb(palette.border))
            .bg(self.shell_surface_color(palette.surface))
            .child(self.side_panel_stack(PanelSide::Left, cx))
    }

    pub(in crate::features) fn left_panel_body(
        &mut self,
        panel: NavItem,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        self.panel_body(panel, cx)
    }

    pub(in crate::features) fn panel_body(
        &mut self,
        panel: NavItem,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let palette = self.theme_palette();
        match panel {
            NavItem::Transfers => self.transfers_view(cx).into_any_element(),
            NavItem::Tunnels => self.tunnels_view(cx).into_any_element(),
            NavItem::SecurityAuth => self.security_auth_panel(cx).into_any_element(),
            NavItem::SyncBackupHistory => self.sync_backup_history_panel(cx).into_any_element(),
            NavItem::Migration => self.migration_view().into_any_element(),
            NavItem::Connections => self.connections_view(cx).into_any_element(),
            NavItem::AiAssistant => self.ai_assistant_panel(cx).into_any_element(),
            NavItem::ActiveSessions => self.active_sessions_panel(cx).into_any_element(),
            NavItem::CommandHistory => self.command_history_panel(cx).into_any_element(),
            NavItem::Stats if self.settings.ui_show_remote_stats => {
                self.stats_view(cx).into_any_element()
            }
            NavItem::Stats => crate::features::inspector::disabled_inspector_panel(
                palette,
                self.tr("panel.resourceMonitorDisabled"),
            )
            .into_any_element(),
            NavItem::Processes if self.settings.ui_show_process_manager => {
                self.processes_view(cx).into_any_element()
            }
            NavItem::Processes => crate::features::inspector::disabled_inspector_panel(
                palette,
                self.tr("processManager.disabled"),
            )
            .into_any_element(),
            NavItem::Docker if self.settings.ui_show_docker_manager => {
                self.docker_view(cx).into_any_element()
            }
            NavItem::Docker => crate::features::inspector::disabled_inspector_panel(
                palette,
                self.tr("dockerManager.disabled"),
            )
            .into_any_element(),
            NavItem::Recording => self.recording_panel(cx).into_any_element(),
            NavItem::Workspace | NavItem::Settings => {
                self.left_workspace_summary(cx).into_any_element()
            }
        }
    }
}
