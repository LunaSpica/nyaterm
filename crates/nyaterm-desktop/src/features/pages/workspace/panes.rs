use super::*;
use gpui::AnimationExt;
use std::time::Duration;

impl NyaTermApp {
    fn workspace_reconnect_pending_state(&self, session_id: &str) -> impl IntoElement {
        let palette = self.theme_palette();
        let name = self
            .session_display_name(session_id)
            .unwrap_or_else(|| short_id(session_id).to_string());
        let detail = self
            .tr("savedConnections.connecting")
            .replace("{{name}}", &name);
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .bg(self.shell_transparent_color(self.terminal_theme_palette().terminal_bg))
            .child(
                svg()
                    .size(px(28.))
                    .path("icons/conn/connect.svg")
                    .text_color(rgb(palette.primary))
                    .with_animation(
                        SharedString::from(format!("reconnect-spinner-{session_id}")),
                        gpui::Animation::new(Duration::from_millis(900)).repeat(),
                        |svg, delta| {
                            svg.with_transformation(gpui::Transformation::rotate(gpui::percentage(
                                delta,
                            )))
                        },
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .max_w(px(320.))
                    .px_4()
                    .text_center()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("tabCtx.reconnecting")),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(detail),
                    ),
            )
    }

    fn workspace_reconnect_failed_state(
        &mut self,
        session_id: String,
        error: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let reconnect_session_id = session_id.clone();
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .bg(self.shell_transparent_color(self.terminal_theme_palette().terminal_bg))
            .child(
                svg()
                    .size(px(32.))
                    .path("icons/session/disconnect.svg")
                    .text_color(rgb(palette.danger)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .px_6()
                    .text_center()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("terminal.connectionFailed")),
                    )
                    .child(
                        div()
                            .max_w(px(320.))
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(truncate_preview(&error, 180)),
                    ),
            )
            .child(small_button(
                palette,
                format!("workspace-reconnect-{session_id}"),
                self.tr("tabCtx.reconnect"),
                cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.reconnect_session(reconnect_session_id.clone(), window, cx);
                }),
            ))
    }

    pub(super) fn workspace_session_content(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let reconnect_pending = self
            .pending_session_starts
            .values()
            .any(|pending| pending.reconnect_session_id.as_deref() == Some(session_id.as_str()));
        if reconnect_pending {
            return self
                .workspace_reconnect_pending_state(&session_id)
                .into_any_element();
        }
        if let Some(error) = self.reconnect_session_failures.get(&session_id).cloned() {
            return self
                .workspace_reconnect_failed_state(session_id, error, cx)
                .into_any_element();
        }
        self.terminal_canvas_for(session_id, cx).into_any_element()
    }

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
                let content = self.workspace_session_content(session_id.clone(), cx);
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
                pane.child(div().flex_1().min_h_0().overflow_hidden().child(content))
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
