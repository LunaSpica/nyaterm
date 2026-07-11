use gpui::{
    Context, FontWeight, IntoElement, SharedString, div, prelude::*, px, relative, rgb, rgba, svg,
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
                let global_index: std::collections::HashMap<String, usize> = self
                    .ordered_sessions()
                    .into_iter()
                    .enumerate()
                    .map(|(index, session)| (session.id, index + 1))
                    .collect();
                for tab_id in &tab_ids {
                    let is_active_tab = active.as_str() == tab_id.as_str();
                    let tab_number = global_index.get(tab_id).copied().unwrap_or(0);
                    let title = self
                        .session_display_name(tab_id)
                        .unwrap_or_else(|| short_id(tab_id).to_string());
                    let leaf_id = id.clone();
                    let select_id = tab_id.clone();
                    let close_id = tab_id.clone();
                    let actions_id = tab_id.clone();
                    let drop_before_id = tab_id.clone();
                    let (kind_label, kind_icon) = self
                        .ordered_sessions()
                        .into_iter()
                        .find(|session| session.id == *tab_id)
                        .map(|session| {
                            (
                                session_kind_label(session.kind),
                                multi_leaf_session_kind_icon(session.kind),
                            )
                        })
                        .unwrap_or(("Session", "icons/conn/terminal.svg"));
                    let custom_color = self.session_tab_colors.get(tab_id).copied();
                    let is_disconnected = self.is_session_disconnected(tab_id);
                    let has_unread = self
                        .terminal_views
                        .get(tab_id)
                        .is_some_and(|view| view.has_unread);
                    let sync_group = self.active_sync_group_for_session(tab_id);
                    let sync_paused = self.is_session_paused_in_active_sync_group(tab_id);
                    let show_sync_indicator = self.broadcast_to_all || sync_group.is_some();
                    let sync_indicator_color = sync_group
                        .map(|group| group.color)
                        .unwrap_or(palette.accent);
                    let accent = if let Some(custom_color) = custom_color {
                        rgb(custom_color)
                    } else if is_disconnected {
                        rgb(palette.danger)
                    } else if is_active_tab {
                        rgb(palette.success)
                    } else if has_unread {
                        rgb(palette.warning)
                    } else {
                        rgb(palette.text_dimmed)
                    };
                    let bg = if let Some(custom_color) = custom_color {
                        rgba((custom_color << 8) | if is_active_tab { 0x24 } else { 0x14 })
                    } else if is_active_tab {
                        rgb(palette.hover)
                    } else {
                        rgb(palette.surface)
                    };
                    let drag_payload = SessionTabDragPayload {
                        session_id: tab_id.clone(),
                        display_name: title.clone(),
                        kind_label,
                    };
                    let tab_title = if is_disconnected {
                        format!("{} · disconnected", truncate_preview(&title, 14))
                    } else {
                        truncate_preview(&title, 18)
                    };
                    strip = strip.child(
                        div()
                            .id(SharedString::from(format!("tw-tab-{leaf_id}-{select_id}")))
                            .h(px(24.))
                            .px_1()
                            .pl_2()
                            .rounded_sm()
                            .flex()
                            .items_center()
                            .gap_1()
                            .relative()
                            .cursor_pointer()
                            .cursor_move()
                            .bg(bg)
                            .when(is_disconnected, |this| this.opacity(0.78))
                            .border_1()
                            .border_color(if is_active_tab {
                                rgb(palette.accent)
                            } else {
                                rgb(palette.border)
                            })
                            .when(is_active_tab, |this| {
                                this.child(
                                    div()
                                        .absolute()
                                        .top_0()
                                        .left_0()
                                        .right_0()
                                        .h(px(2.))
                                        .bg(accent),
                                )
                            })
                            .when(custom_color.is_some(), |this| {
                                this.child(
                                    div()
                                        .absolute()
                                        .top_0()
                                        .bottom_0()
                                        .left_0()
                                        .w(px(3.))
                                        .bg(accent),
                                )
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.activate_terminal_window_tab(
                                    leaf_id.clone(),
                                    select_id.clone(),
                                    cx,
                                );
                                window.focus(&this.terminal_focus);
                            }))
                            .on_mouse_down(
                                gpui::MouseButton::Right,
                                cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.open_tab_actions(actions_id.clone(), window, cx);
                                }),
                            )
                            .on_drag(drag_payload, |payload, position, _, cx| {
                                cx.new(|_| SessionTabDragPreview::new(payload.clone(), position))
                            })
                            .on_drop(cx.listener(
                                move |this, payload: &SessionTabDragPayload, _, cx| {
                                    this.place_tab_before_in_terminal_windows(
                                        payload.session_id.clone(),
                                        drop_before_id.clone(),
                                        cx,
                                    );
                                },
                            ))
                            .child(
                                div()
                                    .size(px(12.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        svg()
                                            .size(px(10.))
                                            .path(kind_icon)
                                            .text_color(accent),
                                    ),
                            )
                            .when(tab_number > 0, |this| {
                                this.child(
                                    div()
                                        .min_w(px(10.))
                                        .text_size(px(10.))
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .child(format!("{tab_number}")),
                                )
                            })
                            .child(
                                div()
                                    .min_w_0()
                                    .text_xs()
                                    .font_weight(FontWeight(if is_active_tab { 700. } else { 500. }))
                                    .text_color(if is_disconnected {
                                        rgb(palette.text_dimmed)
                                    } else if is_active_tab {
                                        rgb(palette.text)
                                    } else {
                                        rgb(palette.text_muted)
                                    })
                                    .child(tab_title),
                            )
                            .when(show_sync_indicator, |this| {
                                this.child(
                                    div()
                                        .size(px(12.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .opacity(if sync_paused { 0.4 } else { 1. })
                                        .child(
                                            svg()
                                                .size(px(10.))
                                                .path("icons/sync.svg")
                                                .text_color(rgb(sync_indicator_color)),
                                        ),
                                )
                            })
                            .when(has_unread && !is_active_tab, |this| {
                                this.child(
                                    div()
                                        .size(px(7.))
                                        .rounded_full()
                                        .bg(rgb(palette.success)),
                                )
                            })
                            .child(
                                div()
                                    .id(SharedString::from(format!(
                                        "tw-tab-close-{id}-{close_id}"
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
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.close_session(close_id.clone(), cx);
                                    })),
                            ),
                    );
                }
                strip = strip.child(
                    div()
                        .id(SharedString::from(format!("tw-leaf-add-{id}")))
                        .size(px(22.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_sm()
                        .text_xs()
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(palette.text_muted))
                        .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
                        .cursor_pointer()
                        .child("+")
                        .on_click(cx.listener({
                            let leaf_id = id.clone();
                            move |this, _, window, cx| {
                                this.focused_terminal_window_leaf_id = Some(leaf_id.clone());
                                this.start_local_session(window, cx);
                            }
                        })),
                );
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

fn multi_leaf_session_kind_icon(kind: nyaterm_session::SessionKind) -> &'static str {
    match kind {
        nyaterm_session::SessionKind::Ssh => "icons/conn/server.svg",
        nyaterm_session::SessionKind::Telnet | nyaterm_session::SessionKind::RawTcp => {
            "icons/conn/telnet.svg"
        }
        nyaterm_session::SessionKind::Serial => "icons/conn/serial.svg",
        nyaterm_session::SessionKind::LocalPty => "icons/conn/terminal.svg",
    }
}
