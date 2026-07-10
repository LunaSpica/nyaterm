use gpui::{Context, FontWeight, IntoElement, div, prelude::*, px, relative, rgb};

use crate::ui::components::{small_button, status_pill};
use crate::ui::models::{NavItem, WorkspaceSplitDirection};

use super::super::{NyaTermApp, split_divider, status_label};

impl NyaTermApp {
    pub(in crate::ui::view) fn workspace_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected_nav = self.selected_nav;
        let mut workspace_actions = div().flex().items_center().gap_2().child(status_pill(
            status_label(&self.terminal_status),
            rgb(0x93c5fd),
            rgb(0x17253b),
        ));
        if selected_nav != NavItem::Workspace {
            workspace_actions = workspace_actions.child(small_button(
                format!("workspace-full-view-{}", selected_nav.label()),
                "Full View",
                cx.listener(move |this, _, _, cx| {
                    this.open_page(selected_nav, cx);
                }),
            ));
        }
        if self.active_session_id.is_some() {
            workspace_actions = workspace_actions
                .child(small_button(
                    "workspace-split-horizontal",
                    "Split H",
                    cx.listener(|this, _, window, cx| {
                        this.split_workspace_with_duplicate(
                            WorkspaceSplitDirection::Horizontal,
                            window,
                            cx,
                        );
                    }),
                ))
                .child(small_button(
                    "workspace-split-vertical",
                    "Split V",
                    cx.listener(|this, _, window, cx| {
                        this.split_workspace_with_duplicate(
                            WorkspaceSplitDirection::Vertical,
                            window,
                            cx,
                        );
                    }),
                ));
        }
        if let Some(split) = self.workspace_split.as_ref() {
            workspace_actions = workspace_actions
                .child(status_pill(
                    if split.direction == WorkspaceSplitDirection::Horizontal {
                        "H Split"
                    } else {
                        "V Split"
                    },
                    rgb(0x6ee7b7),
                    rgb(0x12342a),
                ))
                .child(
                    div()
                        .rounded_sm()
                        .px_2()
                        .py_1()
                        .text_xs()
                        .text_color(rgb(0x93c5fd))
                        .bg(rgb(0x17253b))
                        .child(format!(
                            "{} / {}",
                            split.primary_weight(),
                            split.secondary_weight()
                        )),
                )
                .child(small_button(
                    "workspace-split-ratio-minus",
                    "-10",
                    cx.listener(|this, _, _, cx| {
                        this.adjust_workspace_split_ratio(-10, cx);
                    }),
                ))
                .child(small_button(
                    "workspace-split-ratio-plus",
                    "+10",
                    cx.listener(|this, _, _, cx| {
                        this.adjust_workspace_split_ratio(10, cx);
                    }),
                ))
                .child(small_button(
                    "workspace-unsplit",
                    "Unsplit",
                    cx.listener(|this, _, _, cx| {
                        this.unsplit_workspace(cx);
                    }),
                ));
        }
        let mut workspace = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(rgb(0x0b0d12))
            .child(self.session_tab_strip(cx))
            .child(
                div()
                    .h(px(38.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .border_b_1()
                    .border_color(rgb(0x202633))
                    .bg(rgb(0x0f131a))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(0x98a3b8))
                            .child("Terminal Workspace"),
                    )
                    .child(workspace_actions),
            );

        if let Some(prompt) = self.active_host_key_prompt.clone() {
            workspace = workspace.child(self.host_key_prompt_banner(prompt, cx));
        }
        if let Some(prompt) = self.active_credential_prompt.clone() {
            workspace = workspace.child(self.credential_prompt_banner(prompt, cx));
        }

        div()
            .flex()
            .flex_1()
            .min_h_0()
            .bg(rgb(0x0b0d12))
            .child(
                workspace
                    .child(self.workspace_terminal_area(cx))
                    .child(self.bottom_panel_view(cx)),
            )
            .when(!self.right_inspector_collapsed, |this| {
                this.child(self.right_panel(cx))
            })
    }

    fn workspace_terminal_area(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        self.prune_workspace_split();
        let Some(active_session_id) = self.active_session_id.clone() else {
            return self.empty_workspace_state(cx).into_any_element();
        };
        let Some(split) = self.workspace_split.clone() else {
            return self.terminal_canvas(cx).into_any_element();
        };

        let primary_id = if self.live_session_ids().contains(&split.primary_session_id) {
            split.primary_session_id.clone()
        } else {
            active_session_id.clone()
        };
        let secondary_id = split.secondary_session_id.clone();
        let first = self
            .terminal_canvas_for(primary_id, true, cx)
            .into_any_element();
        let second = self
            .terminal_canvas_for(secondary_id, true, cx)
            .into_any_element();
        let divider = split_divider(split.direction);
        let primary_basis = relative(split.primary_weight() / 100.);
        let secondary_basis = relative(split.secondary_weight() / 100.);

        match split.direction {
            WorkspaceSplitDirection::Horizontal => div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .gap_1()
                .p_1()
                .bg(rgb(0x07090d))
                .child(
                    div()
                        .flex_none()
                        .flex_basis(primary_basis)
                        .min_h_0()
                        .child(first),
                )
                .child(divider)
                .child(
                    div()
                        .flex_none()
                        .flex_basis(secondary_basis)
                        .min_h_0()
                        .child(second),
                )
                .into_any_element(),
            WorkspaceSplitDirection::Vertical => div()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .flex()
                .gap_1()
                .p_1()
                .bg(rgb(0x07090d))
                .child(
                    div()
                        .flex_none()
                        .flex_basis(primary_basis)
                        .min_w_0()
                        .child(first),
                )
                .child(divider)
                .child(
                    div()
                        .flex_none()
                        .flex_basis(secondary_basis)
                        .min_w_0()
                        .child(second),
                )
                .into_any_element(),
        }
    }
}
