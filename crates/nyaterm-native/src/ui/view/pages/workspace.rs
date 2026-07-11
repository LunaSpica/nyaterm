use gpui::{
    Context, FontWeight, IntoElement, SharedString, div, prelude::*, px, relative, rgb, rgba,
};

use crate::ui::models::{
    TabDockEdge, TabDockZone, TerminalWindowNode, WorkspacePaneNode, WorkspaceSplitDirection,
};
use super::super::{
    NyaTermApp, SessionTabDragPayload, SessionTabDragPreview, ThemePalette, session_kind_label,
    short_id, truncate_preview,
};

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
                let focused_leaf =
                    self.focused_terminal_window_leaf_id.as_deref() == Some(id.as_str());
                let drop_zone = self
                    .terminal_window_drop
                    .as_ref()
                    .filter(|(leaf, _)| leaf == &id)
                    .map(|(_, zone)| *zone);
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
                    let kind_label = self
                        .ordered_sessions()
                        .into_iter()
                        .find(|session| session.id == *tab_id)
                        .map(|session| session_kind_label(session.kind))
                        .unwrap_or("Session");
                    let drag_payload = SessionTabDragPayload {
                        session_id: tab_id.clone(),
                        display_name: title.clone(),
                        kind_label,
                    };
                    strip = strip.child(
                        div()
                            .id(SharedString::from(format!("tw-tab-{leaf_id}-{select_id}")))
                            .h(px(24.))
                            .px_2()
                            .rounded_sm()
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .cursor_move()
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
                            .on_drag(drag_payload, |payload, position, _, cx| {
                                cx.new(|_| SessionTabDragPreview::new(payload.clone(), position))
                            })
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
                let drop_leaf_id_move = id.clone();
                let drop_leaf_id_drop = id.clone();
                let content = div()
                    .id(SharedString::from(format!("tw-leaf-content-{id}")))
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .relative()
                    .can_drop(|drag, _, _| drag.is::<SessionTabDragPayload>())
                    .on_drag_move(cx.listener(
                        move |this, event: &gpui::DragMoveEvent<SessionTabDragPayload>, _, cx| {
                            let _ = event.drag(cx);
                            let bounds = event.bounds;
                            let pos = event.event.position;
                            let width = f32::from(bounds.size.width).max(1.0);
                            let height = f32::from(bounds.size.height).max(1.0);
                            let local_x = (f32::from(pos.x - bounds.origin.x)).clamp(0.0, width);
                            let local_y = (f32::from(pos.y - bounds.origin.y)).clamp(0.0, height);
                            let zone = TabDockZone::detect(local_x, local_y, width, height);
                            this.set_terminal_window_drop(drop_leaf_id_move.clone(), zone, cx);
                        },
                    ))
                    .on_drop(cx.listener(
                        move |this, payload: &SessionTabDragPayload, _, cx| {
                            let zone = this
                                .terminal_window_drop
                                .as_ref()
                                .filter(|(leaf, _)| leaf == &drop_leaf_id_drop)
                                .map(|(_, zone)| *zone)
                                .unwrap_or(TabDockZone::Center);
                            this.dock_tab_on_terminal_window_leaf(
                                payload.session_id.clone(),
                                drop_leaf_id_drop.clone(),
                                zone,
                                cx,
                            );
                        },
                    ))
                    .child(canvas)
                    .when_some(drop_zone, |this, zone| {
                        this.child(Self::tab_dock_drop_overlay(zone, palette))
                    });
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
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener({
                            let leaf_id = id.clone();
                            move |this, _, _, cx| {
                                this.focused_terminal_window_leaf_id = Some(leaf_id.clone());
                                cx.notify();
                            }
                        }),
                    )
                    .child(strip)
                    .child(content)
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

    fn tab_dock_drop_overlay(zone: TabDockZone, palette: ThemePalette) -> impl IntoElement {
        let label = match zone {
            TabDockZone::Center => "Merge into window",
            TabDockZone::Edge(TabDockEdge::Left) => "Split left",
            TabDockZone::Edge(TabDockEdge::Right) => "Split right",
            TabDockZone::Edge(TabDockEdge::Top) => "Split top",
            TabDockZone::Edge(TabDockEdge::Bottom) => "Split bottom",
        };
        let accent = rgb(palette.accent);
        let mut zone_box = div()
            .absolute()
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .border_2()
            .border_color(accent)
            .bg(rgba((palette.accent << 8) | 0x28));
        zone_box = match zone {
            TabDockZone::Center => zone_box.inset_2(),
            TabDockZone::Edge(TabDockEdge::Left) => zone_box
                .top_2()
                .bottom_2()
                .left_2()
                .w(relative(0.38)),
            TabDockZone::Edge(TabDockEdge::Right) => zone_box
                .top_2()
                .bottom_2()
                .right_2()
                .w(relative(0.38)),
            TabDockZone::Edge(TabDockEdge::Top) => zone_box
                .left_2()
                .right_2()
                .top_2()
                .h(relative(0.38)),
            TabDockZone::Edge(TabDockEdge::Bottom) => zone_box
                .left_2()
                .right_2()
                .bottom_2()
                .h(relative(0.38)),
        };
        div()
            .absolute()
            .inset_0()
            .child(
                zone_box.child(
                    div()
                        .rounded_sm()
                        .border_1()
                        .border_color(accent)
                        .bg(rgb(palette.surface))
                        .px_3()
                        .py_1()
                        .text_xs()
                        .font_weight(FontWeight(600.))
                        .text_color(rgb(palette.text))
                        .child(label),
                ),
            )
    }
}
