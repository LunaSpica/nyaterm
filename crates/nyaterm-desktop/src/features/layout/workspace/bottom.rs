use super::*;

use crate::models::BottomPanelMode;

impl NyaTermApp {
    pub(in crate::features) fn bottom_panel_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match self.bottom_panel {
            BottomPanelMode::QuickCommands => {
                let palette = self.theme_palette();
                div()
                    .h(px(self.quick_cmd_height.clamp(36., 520.)))
                    .flex_none()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.surface))
                    .child(self.quick_commands_panel(cx))
                    .into_any_element()
            }
            BottomPanelMode::CommandSend => self.bottom_command_send_bar(cx).into_any_element(),
            BottomPanelMode::Hidden => div().into_any_element(),
        }
    }
}
