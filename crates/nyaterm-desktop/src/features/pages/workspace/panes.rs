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
                let is_disconnected = self.is_session_disconnected(&session_id);
                let title = self
                    .session_display_name(&session_id)
                    .unwrap_or_else(|| short_id(&session_id).to_string());
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
                    // Tauri PaneWorkspace leaf chrome: accent border + compact title strip.
                    pane = pane
                        .rounded_sm()
                        .border_1()
                        .border_color(if is_active {
                            rgb(palette.link)
                        } else {
                            rgb(palette.border)
                        })
                        .child(
                            div()
                                .h(px(22.))
                                .flex_none()
                                .px_2()
                                .flex()
                                .items_center()
                                .gap_1()
                                .border_b_1()
                                .border_color(rgb(palette.border))
                                .bg(if is_active {
                                    rgb(palette.hover)
                                } else {
                                    rgb(palette.surface)
                                })
                                .child(div().size(px(7.)).rounded_full().bg(if is_disconnected {
                                    rgb(palette.danger)
                                } else if is_active {
                                    rgb(palette.success)
                                } else {
                                    rgb(palette.text_dimmed)
                                }))
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .text_size(px(11.))
                                        .font_weight(if is_active {
                                            FontWeight(700.)
                                        } else {
                                            FontWeight(500.)
                                        })
                                        .text_color(if is_disconnected {
                                            rgb(palette.text_dimmed)
                                        } else {
                                            rgb(palette.text)
                                        })
                                        .overflow_hidden()
                                        .child(truncate_preview(&title, 36)),
                                )
                                .child(
                                    div()
                                        .id(SharedString::from(format!(
                                            "workspace-leaf-close-{session_id}"
                                        )))
                                        .size(px(16.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_sm()
                                        .text_size(px(10.))
                                        .text_color(rgb(palette.text_muted))
                                        .hover(|this| {
                                            this.bg(rgb(palette.border))
                                                .text_color(rgb(palette.danger))
                                        })
                                        .child("x")
                                        .on_click(cx.listener({
                                            let close_id = session_id.clone();
                                            move |this, _, _, cx| {
                                                cx.stop_propagation();
                                                // Close only this leaf (secondary panes collapse tree).
                                                this.close_session(close_id.clone(), cx);
                                            }
                                        })),
                                ),
                        );
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
