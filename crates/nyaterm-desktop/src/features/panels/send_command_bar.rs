use super::*;

#[path = "send_command_bar/controls.rs"]
mod controls;
#[path = "send_command_bar/editor.rs"]
mod editor;
#[path = "send_command_bar/footer.rs"]
mod footer;
#[path = "send_command_bar/header.rs"]
mod header;
#[path = "send_command_bar/state.rs"]
mod state;

impl NyaTermApp {
    pub(in crate::features) fn bottom_command_send_bar(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let state = self.send_command_bar_view_state();
        let palette = state.palette;
        let header = self.send_command_bar_header(&state, cx);
        let controls = self.send_command_bar_controls(&state, cx);
        let editor = self.send_command_bar_editor(&state, cx);
        let progress = self.send_command_bar_progress(&state);
        let footer = self.send_command_bar_footer(&state, cx);

        // Tauri SendCommandPanel: title row + labeled control groups + editor + action footer.
        div()
            .h(px(240.))
            .flex_none()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .child(header)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .px_2()
                    .py_1()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(controls)
                    .child(editor)
                    .when(state.is_sending, |this| this.child(progress))
                    .child(footer),
            )
    }
}
