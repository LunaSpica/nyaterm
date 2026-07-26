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
            .bg(self.shell_surface_color(palette.surface))
            .child(self.side_panel_stack(PanelSide::Right, cx))
    }

    pub(in crate::features) fn right_panel_body(
        &mut self,
        panel: NavItem,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        self.panel_body(panel, cx)
    }
}
