use gpui::{Context, FontWeight, IntoElement, SharedString, div, prelude::*, px, relative, rgb};

use crate::ui::models::{TerminalWindowNode, WorkspacePaneNode, WorkspaceSplitDirection};
use super::super::{NyaTermApp, short_id, truncate_preview};

impl NyaTermApp {
    pub(in crate::ui::view) fn workspace_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Match the Tauri shell: tab strip sits directly above the terminal surface.
        self.reconcile_terminal_windows();
        let multi_leaf = self.terminal_windows_is_multi_leaf();
        let mut workspace = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(rgb(palette.bg));
        if !multi_leaf {
            workspace = workspace.child(self.session_tab_strip(cx));
        }

        if let Some(prompt) = self.active_host_key_prompt.clone() {
            workspace = workspace.child(self.host_key_prompt_banner(prompt, cx));
        }
        if let Some(prompt) = self.active_credential_prompt.clone() {
            workspace = workspace.child(self.credential_prompt_banner(prompt, cx));
        }

        workspace
            .child(self.workspace_terminal_area(cx))
            .child(self.bottom_panel_view(cx))
    }

    fn workspace_terminal_area(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let palette = self.theme_palette();
        self.prune_workspace_split();
        self.reconcile_terminal_windows();
        if self.active_session_id.is_none() {
            return self.empty_workspace_state(cx).into_any_element();
        }
        // Multi-leaf tab windows (Tauri TabWindowsWorkspace) take precedence over
        // single-tree pane splits when active.
        if let Some(window_root) = self.terminal_windows.clone() {
            if matches!(window_root, TerminalWindowNode::Split { .. }) {
                return div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .p_1()
                    .bg(rgb(palette.bg))
                    .child(self.render_terminal_window_node(window_root, cx))
                    .into_any_element();
            }
        }
        let Some(root) = self.workspace_split.clone() else {
            return self.terminal_canvas(cx).into_any_element();
        };

        div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .p_1()
            .bg(rgb(palette.bg))
            .child(self.render_workspace_pane_node(root, true, cx))
            .into_any_element()
    }

    fn render_terminal_window_node(
        &mut self,
        node: TerminalWindowNode,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let palette = self.theme_palette();
        match node {
            TerminalWindowNode::Leaf {
                id,
                tab_ids,
                active_tab_id,
            } => {
                let active = active_tab_id
                    .clone()
                    .or_else(|| tab_ids.first().cloned())
                    .unwrap_or_default();
                let focused_leaf = self.focused_terminal_window_leaf_id.as_deref() == Some(id.as_str());
                let mut strip = div()
                    .h(px(30.))
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_1()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface));
                for tab_id in &tab_ids {
                    let is_active_tab = active.as_str() == tab_id.as_str();
                    let title = self
                        .session_display_name(tab_id)
                        .unwrap_or_else(|| short_id(tab_id).to_string());
                    let leaf_id = id.clone();
                    let select_id = tab_id.clone();
                    strip = strip.child(
                        div()
                            .id(SharedString::from(format!("tw-tab-{leaf_id}-{select_id}")))
                            .h(px(24.))
                            .px_2()
                            .rounded_sm()
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .bg(if is_active_tab {
                                rgb(palette.hover)
                            } else {
                                rgb(palette.surface)
                            })
                            .border_1()
                            .border_color(if is_active_tab {
                                rgb(palette.accent)
                            } else {
                                rgb(palette.border)
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.activate_terminal_window_tab(
                                    leaf_id.clone(),
                                    select_id.clone(),
                                    cx,
                                );
                                window.focus(&this.terminal_focus);
                            }))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight(if is_active_tab { 700. } else { 500. }))
                                    .text_color(if is_active_tab {
                                        rgb(palette.text)
                                    } else {
                                        rgb(palette.text_muted)
                                    })
                                    .child(truncate_preview(&title, 18)),
                            ),
                    );
                }
                let canvas = if active.is_empty() {
                    div().flex_1().into_any_element()
                } else {
                    self.terminal_canvas_for(active.clone(), false, cx)
                        .into_any_element()
                };
                div()
                    .id(SharedString::from(format!("tw-leaf-{id}")))
                    .size_full()
                    .min_h_0()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .rounded_sm()
                    .border_1()
                    .border_color(if focused_leaf {
                        rgb(palette.accent)
                    } else {
                        rgb(palette.border)
                    })
                    .overflow_hidden()
                    .child(strip)
                    .child(div().flex_1().min_h_0().child(canvas))
                    .into_any_element()
            }
            TerminalWindowNode::Split {
                id,
                direction,
                ratio_percent,
                first,
                second,
            } => {
                let first_el = self.render_terminal_window_node(*first, cx);
                let second_el = self.render_terminal_window_node(*second, cx);
                let primary_basis =
                    relative(WorkspacePaneNode::primary_weight(ratio_percent) / 100.);
                let secondary_basis =
                    relative(WorkspacePaneNode::secondary_weight(ratio_percent) / 100.);
                let divider = self.workspace_split_resize_handle(id.clone(), direction, cx);
                match direction {
                    WorkspaceSplitDirection::Horizontal => div()
                        .id(SharedString::from(format!("tw-split-{id}")))
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
                        .id(SharedString::from(format!("tw-split-{id}")))
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

    fn render_workspace_pane_node(
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
                let mut pane = div()
                    .id(SharedString::from(format!("workspace-leaf-{session_id}")))
                    .size_full()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.activate_workspace_pane(session_id.clone(), cx);
                        window.focus(&this.terminal_focus);
                        cx.notify();
                    }));
                if show_chrome {
                    pane = pane
                        .rounded_sm()
                        .border_1()
                        .border_color(if is_active {
                            rgb(palette.accent)
                        } else {
                            rgb(palette.border)
                        });
                }
                pane.child(canvas).into_any_element()
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
                let primary_basis = relative(WorkspacePaneNode::primary_weight(ratio_percent) / 100.);
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
