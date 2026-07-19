use super::*;

impl NyaTermApp {
    pub(super) fn render_workspace_pane_node(
        &mut self,
        node: WorkspacePaneNode,
        show_chrome: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let palette = self.theme_palette();
        match node {
            WorkspacePaneNode::Leaf { session_id } => {
                let is_active = self.active_session_id.as_deref() == Some(session_id.as_str());
                let canvas = self
                    .terminal_canvas_for(session_id.clone(), true, cx)
                    .into_any_element();
                let focus_id = session_id.clone();
                let mut pane = div()
                    .id(SharedString::from(format!("workspace-leaf-{session_id}")))
                    .size_full()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.activate_workspace_pane(focus_id.clone(), cx);
                        window.focus(&this.terminal_focus);
                        cx.notify();
                    }));
                if show_chrome {
                    // Tauri PaneWorkspace uses the pane border as the only split chrome.
                    pane = pane.border_1().border_color(if is_active {
                        rgb(palette.primary)
                    } else {
                        rgb(palette.border)
                    });
                }
                pane.child(div().flex_1().min_h_0().overflow_hidden().child(canvas))
                    .into_any_element()
            }
            WorkspacePaneNode::Split {
                id,
                direction,
                ratio_percent,
                first,
                second,
            } => {
                let first_el = self.render_workspace_pane_node(*first, true, cx);
                let second_el = self.render_workspace_pane_node(*second, true, cx);
                let divider = self.workspace_split_resize_handle(id.clone(), direction, cx);
                let primary_basis =
                    relative(WorkspacePaneNode::primary_weight(ratio_percent) / 100.);
                let secondary_basis =
                    relative(WorkspacePaneNode::secondary_weight(ratio_percent) / 100.);

                match direction {
                    WorkspaceSplitDirection::Horizontal => div()
                        .id(SharedString::from(format!("workspace-split-{id}")))
                        .size_full()
                        .min_h_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .flex_none()
                                .flex_basis(primary_basis)
                                .min_h(px(80.))
                                .overflow_hidden()
                                .child(first_el),
                        )
                        .child(divider)
                        .child(
                            div()
                                .flex_none()
                                .flex_basis(secondary_basis)
                                .min_h(px(80.))
                                .overflow_hidden()
                                .child(second_el),
                        )
                        .into_any_element(),
                    WorkspaceSplitDirection::Vertical => div()
                        .id(SharedString::from(format!("workspace-split-{id}")))
                        .size_full()
                        .min_h_0()
                        .min_w_0()
                        .flex()
                        .child(
                            div()
                                .flex_none()
                                .flex_basis(primary_basis)
                                .min_w(px(120.))
                                .overflow_hidden()
                                .child(first_el),
                        )
                        .child(divider)
                        .child(
                            div()
                                .flex_none()
                                .flex_basis(secondary_basis)
                                .min_w(px(120.))
                                .overflow_hidden()
                                .child(second_el),
                        )
                        .into_any_element(),
                }
            }
        }
    }
}
