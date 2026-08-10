use gpui::{Context, IntoElement, div, prelude::*, px, rgb};

use crate::features::NyaTermApp;
use crate::models::{NavItem, PanelSide};

impl NyaTermApp {
    pub(in crate::features) fn sidebar(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut width = self.shell.left_panel_width().clamp(160., 720.);
        if !cfg!(target_os = "macos") && self.shell.viewport_size().0 < 1024. {
            width = width.min((self.shell.viewport_size().0 - 80.).max(120.));
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
            .child(self.side_panel_stack(PanelSide::Left, window, cx))
    }

    pub(in crate::features) fn left_panel_body(
        &mut self,
        panel: NavItem,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        self.panel_body(panel, window, cx)
    }

    pub(in crate::features) fn panel_body(
        &mut self,
        panel: NavItem,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let palette = self.theme_palette();
        match panel {
            NavItem::Transfers => self.transfers_view(cx).into_any_element(),
            NavItem::Tunnels => self.tunnels_view(cx).into_any_element(),
            NavItem::SecurityAuth => self.security_auth_panel(cx).into_any_element(),
            NavItem::SyncBackupHistory => self.sync_backup_history_panel(cx).into_any_element(),
            NavItem::Connections => self.connections_view(window, cx).into_any_element(),
            NavItem::AiAssistant => self.ai_assistant_panel(cx).into_any_element(),
            NavItem::ActiveSessions => self.active_sessions_panel(cx).into_any_element(),
            NavItem::CommandHistory => self.command_history_panel(cx).into_any_element(),
            NavItem::Stats if self.settings.summary().ui_show_remote_stats => {
                self.stats_view(cx).into_any_element()
            }
            NavItem::Stats => crate::features::inspector::disabled_inspector_panel(
                palette,
                self.tr("panel.resourceMonitorDisabled"),
            )
            .into_any_element(),
            NavItem::GpuMonitor if self.settings.summary().ui_show_gpu_monitor => {
                self.gpu_view(cx).into_any_element()
            }
            NavItem::GpuMonitor => crate::features::inspector::disabled_inspector_panel(
                palette,
                self.tr("panel.gpuMonitorDisabled"),
            )
            .into_any_element(),
            NavItem::AscendNpuMonitor if self.settings.summary().ui_show_ascend_npu_monitor => {
                self.npu_view(cx).into_any_element()
            }
            NavItem::AscendNpuMonitor => crate::features::inspector::disabled_inspector_panel(
                palette,
                self.tr("panel.npuMonitorDisabled"),
            )
            .into_any_element(),
            NavItem::Processes if self.settings.summary().ui_show_process_manager => {
                self.processes_view(cx).into_any_element()
            }
            NavItem::Processes => crate::features::inspector::disabled_inspector_panel(
                palette,
                self.tr("processManager.disabled"),
            )
            .into_any_element(),
            NavItem::Docker if self.settings.summary().ui_show_docker_manager => {
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
