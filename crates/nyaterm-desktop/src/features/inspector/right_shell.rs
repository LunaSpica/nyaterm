use super::*;

impl NyaTermApp {
    pub(in crate::features) fn right_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut width = self.right_panel_width.clamp(200., 720.);
        if !cfg!(target_os = "macos") && self.last_viewport_size.0 < 768. {
            width = width.min((self.last_viewport_size.0 - 80.).max(120.));
        }
        let palette = self.theme_palette();
        div()
            .w(px(width))
            .flex_none()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .child(self.side_panel_stack(PanelSide::Right, cx))
    }

    pub(in crate::features) fn right_panel_meta(&self) -> &'static str {
        match self.current_right_panel().unwrap_or(NavItem::Connections) {
            NavItem::Connections => "saved connections",
            NavItem::AiAssistant => "assistant",
            NavItem::ActiveSessions => "sessions",
            NavItem::CommandHistory => "history",
            NavItem::Stats => "resource monitor",
            NavItem::Processes => "process manager",
            NavItem::Docker => "docker manager",
            NavItem::Translation => "translation",
            NavItem::Recording => "recording",
            other => other.label(),
        }
    }

    pub(in crate::features) fn right_panel_body(
        &mut self,
        panel: NavItem,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let palette = self.theme_palette();
        match panel {
            NavItem::Connections => self.connections_view(cx).into_any_element(),
            NavItem::AiAssistant => self.ai_assistant_panel(cx).into_any_element(),
            NavItem::ActiveSessions => self.active_sessions_panel(cx).into_any_element(),
            NavItem::CommandHistory => self.command_history_panel(cx).into_any_element(),
            NavItem::Stats if self.settings.ui_show_remote_stats => {
                self.stats_view(cx).into_any_element()
            }
            NavItem::Stats => disabled_inspector_panel(
                palette,
                "Remote Stats Disabled",
                "Enable Remote Stats in Settings > Terminal Session > General.",
            )
            .into_any_element(),
            NavItem::Processes if self.settings.ui_show_process_manager => {
                self.processes_view(cx).into_any_element()
            }
            NavItem::Processes => disabled_inspector_panel(
                palette,
                "Process Manager Disabled",
                "Enable Process Manager in Settings > Terminal Session > General.",
            )
            .into_any_element(),
            NavItem::Docker if self.settings.ui_show_docker_manager => {
                self.docker_view(cx).into_any_element()
            }
            NavItem::Docker => disabled_inspector_panel(
                palette,
                "Docker Manager Disabled",
                "Enable Docker Manager in Settings > Terminal Session > General.",
            )
            .into_any_element(),
            NavItem::Translation => self.translation_view(cx).into_any_element(),
            NavItem::Recording => self.recording_panel(cx).into_any_element(),
            _ => self.ai_assistant_panel(cx).into_any_element(),
        }
    }
}
