use std::sync::Arc;

use gpui::{
    AppContext as _, Context, FontWeight, IntoElement, MouseButton, MouseDownEvent, Render,
    SharedString, Window, div,
    prelude::{
        FluentBuilder, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled,
    },
    px, relative, rgb, rgba, svg,
};
use nyaterm_core::SavedConnection;

use crate::features::{
    ConnectionDragKind, ConnectionDragPayload, ConnectionDragPreview, ConnectionDropPosition,
    ConnectionDropTarget, NyaTermApp, connection_type_icon, resolve_connection_icon,
};

use super::super::list::{
    ConnectionSection, connection_detail_rows, connection_tree_indent_px, icon_action_button,
};

/// The label/value card shown after hovering a saved connection.
///
/// This is a tooltip rather than an absolutely positioned child of the row: the
/// panel clips its overflow, and rows painted after the hovered one would cover
/// an in-tree card anyway. As a tooltip it is deferred to the top paint layer and
/// flips to stay inside the window, which is what the old UI got from portalling.
pub(in crate::features) struct ConnectionDetailsTooltip {
    rows: Arc<[(&'static str, String)]>,
}

impl ConnectionDetailsTooltip {
    pub(in crate::features) fn new(rows: Arc<[(&'static str, String)]>) -> Self {
        Self { rows }
    }
}

impl Render for ConnectionDetailsTooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut grid = div().flex().flex_col().gap_1();
        for (label, value) in self.rows.iter() {
            grid = grid.child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(
                        div()
                            .w(px(60.))
                            .flex_none()
                            .text_size(px(11.))
                            .text_color(rgb(0x8f98aa))
                            .child(*label),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(11.))
                            .text_color(rgb(0xe5edf7))
                            .child(value.clone()),
                    ),
            );
        }

        div()
            .w(px(228.))
            .rounded_md()
            .border_1()
            .border_color(rgb(0x334155))
            .bg(rgba(0x151b24f2))
            .shadow_lg()
            .px_3()
            .py_2()
            .child(grid)
    }
}

impl NyaTermApp {
    pub(in crate::features) fn connection_section(
        &mut self,
        section: ConnectionSection,
        header_only: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let expanded = self
            .connection_state
            .list_group_is_expanded(section.group_id.as_deref());
        let group_id = section.group_id.clone();
        let group_label = section.label.clone();
        let empty_group_label = self.tr("savedConnections.emptyGroup");
        let count = section.total_count;
        let mut body = div().flex().flex_col();

        if expanded && !header_only {
            if section.connections.is_empty() && !section.is_root && !section.has_child_groups {
                body = body.child(
                    div()
                        .px_2()
                        .py_1()
                        .pl(px(connection_tree_indent_px(section.depth + 1)))
                        .text_size(px(11.))
                        .text_color(rgb(palette.text_dimmed))
                        .child(empty_group_label),
                );
            } else {
                for connection in section.connections {
                    body = body.child(self.saved_connection_row(connection, section.depth + 1, cx));
                }
            }
        }

        // Tauri: ungrouped has no section header — just rows (optionally after a separator).
        if section.is_root {
            return div().flex().flex_col().child(body);
        }

        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .id(SharedString::from(format!(
                        "connection-section-{}",
                        section.group_id.clone().unwrap_or_else(|| "root".into())
                    )))
                    .relative()
                    .h(px(28.))
                    .min_w(relative(1.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .px_2()
                    .pl(px(8. + section.depth as f32 * 16.))
                    .rounded_sm()
                    .cursor_pointer()
                    .bg({
                        let drop_inside = self.connection_state.list_drop_position_for_kind_target(
                            ConnectionDragKind::Group,
                            section.group_id.as_deref(),
                        ) == Some(ConnectionDropPosition::Inside);
                        if drop_inside {
                            rgb(palette.hover)
                        } else if self
                            .connection_state
                            .list_group_is_hovered(section.group_id.as_deref())
                        {
                            rgb(palette.hover)
                        } else {
                            rgba(0x00000000)
                        }
                    })
                    .when(
                        self.connection_state.list_drop_position_for_kind_target(
                            ConnectionDragKind::Group,
                            section.group_id.as_deref(),
                        ) == Some(ConnectionDropPosition::Inside),
                        |this| this.border_1().border_color(rgb(self.theme_palette().link)),
                    )
                    .on_hover({
                        let hover_group = section.group_id.clone();
                        cx.listener(move |this, hovered: &bool, _, cx| {
                            if let Some(group_id) = hover_group.clone() {
                                if this
                                    .connection_state
                                    .set_list_group_hover(group_id, *hovered)
                                {
                                    cx.notify();
                                }
                            }
                        })
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_, _, _, cx| cx.stop_propagation()),
                    )
                    .on_mouse_down(MouseButton::Right, {
                        let menu_group = section.group_id.clone();
                        cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                            if let Some(group_id) = menu_group.clone() {
                                cx.stop_propagation();
                                this.open_connection_group_context_menu(group_id, event, cx);
                            }
                        })
                    })
                    .when_some(section.group_id.clone(), |this, drag_group_id| {
                        let drop_group_id = drag_group_id.clone();
                        let label = section.label.clone();
                        this.cursor_move()
                            .on_drag(
                                ConnectionDragPayload {
                                    kind: ConnectionDragKind::Group,
                                    id: drag_group_id.clone(),
                                    label,
                                },
                                |payload, position, _, cx| {
                                    cx.new(|_| {
                                        ConnectionDragPreview::new(payload.clone(), position)
                                    })
                                },
                            )
                            .on_drag_move(cx.listener({
                                let target_id = drop_group_id.clone();
                                move |this,
                                      event: &gpui::DragMoveEvent<ConnectionDragPayload>,
                                      _,
                                      cx| {
                                    let _ = event.drag(cx);
                                    let y = event.event.position.y;
                                    let bounds = event.bounds;
                                    let rel = if bounds.size.height > px(0.) {
                                        ((y - bounds.origin.y) / bounds.size.height).clamp(0., 1.)
                                    } else {
                                        0.5
                                    };
                                    let position = if rel < 0.25 {
                                        ConnectionDropPosition::Before
                                    } else if rel > 0.75 {
                                        ConnectionDropPosition::After
                                    } else {
                                        ConnectionDropPosition::Inside
                                    };
                                    let next = ConnectionDropTarget {
                                        id: Some(target_id.clone()),
                                        kind: ConnectionDragKind::Group,
                                        position,
                                    };
                                    if this.connection_state.set_list_drop_target_if_changed(next) {
                                        cx.notify();
                                    }
                                }
                            }))
                            .on_drop(cx.listener(
                                move |this, payload: &ConnectionDragPayload, _, cx| {
                                    let position =
                                        this.connection_state.list_drop_position_for_target(
                                            &drop_group_id,
                                            ConnectionDropPosition::Inside,
                                        );
                                    this.connection_state.clear_list_drop_target();
                                    match payload.kind {
                                        ConnectionDragKind::Connection => {
                                            this.move_connection_into_group(
                                                payload.id.clone(),
                                                Some(drop_group_id.clone()),
                                                cx,
                                            );
                                        }
                                        ConnectionDragKind::Group => match position {
                                            ConnectionDropPosition::Inside => {
                                                this.move_group_into_group(
                                                    payload.id.clone(),
                                                    Some(drop_group_id.clone()),
                                                    cx,
                                                );
                                            }
                                            _ => {
                                                this.move_group_before(
                                                    payload.id.clone(),
                                                    drop_group_id.clone(),
                                                    cx,
                                                );
                                            }
                                        },
                                    }
                                },
                            ))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        if let Some(group_id) = group_id.clone() {
                            this.toggle_connection_group_expanded(group_id, cx);
                        }
                    }))
                    // The name takes the slack so the count sits against the right
                    // edge of the panel, where Tauri puts it.
                    .child(
                        svg()
                            .size(px(14.))
                            .flex_none()
                            .path(if expanded {
                                "icons/chevron-down.svg"
                            } else {
                                "icons/fe/forward.svg"
                            })
                            .text_color(rgb(palette.text_muted)),
                    )
                    .child(connection_type_icon(
                        palette,
                        resolve_connection_icon(Some("folder"), "SSH"),
                        false,
                        16.,
                    ))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_xs()
                            .font_weight(FontWeight(500.))
                            .text_color(rgb(palette.text_muted))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(group_label.clone()),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_xs()
                            .text_color(rgb(palette.text_dimmed))
                            .child(count.to_string()),
                    ),
            )
            .child(body)
    }

    pub(in crate::features) fn saved_connection_row(
        &mut self,
        connection: SavedConnection,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self
            .connection_state
            .list_contains_selected_id(&connection.id);
        // The arrow keys walk filtered results without disturbing the selection,
        // so the active row gets its own fainter wash plus a ring.
        let keyboard_active = self
            .connection_state
            .list_connection_is_keyboard_active(&connection.id);
        let connect_connection = connection.clone();
        let connect_connection_dbl = connection.clone();
        let edit_id = connection.id.clone();
        let select_id = connection.id.clone();
        let menu_id = connection.id.clone();
        let kind = connection.kind_label();
        let icon_def = resolve_connection_icon(connection.icon.as_deref(), kind);
        let details_rows: Arc<[(&'static str, String)]> = connection_detail_rows(
            &connection,
            &self.connection_catalog.connections,
            &self.tunnel_state.catalog.proxies,
        )
        .into();
        let row_group = SharedString::from(format!("connection-row-group-{}", connection.id));
        let drop_position = self.connection_state.list_drop_position_for_kind_target(
            ConnectionDragKind::Connection,
            Some(&connection.id),
        );
        let show_before = drop_position == Some(ConnectionDropPosition::Before);
        let show_after = drop_position == Some(ConnectionDropPosition::After);
        let show_inside = drop_position == Some(ConnectionDropPosition::Inside);
        let row_id = connection.id.clone();

        // Tauri ConnectionItem: py-1.5 single-line row (~34px) with hover actions.
        let palette = self.theme_palette();
        div()
            .id(SharedString::from(format!(
                "connection-row-{}",
                connection.id
            )))
            .group(row_group.clone())
            .relative()
            .h(px(34.))
            // The list scrolls sideways, so a row is at least the panel width and
            // grows past it when the name is long.
            .min_w(relative(1.))
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .pl(px(connection_tree_indent_px(depth)))
            .bg(if selected {
                rgba((palette.primary << 8) | 0x1a)
            } else if keyboard_active {
                rgba((palette.primary << 8) | 0x12)
            } else if show_inside {
                rgb(palette.hover)
            } else {
                rgba(0x00000000)
            })
            .when(keyboard_active && !selected, |this| {
                this.border_1().border_color(rgb(palette.primary))
            })
            .hover(move |this| this.bg(rgb(palette.hover)))
            .when(show_inside, |this| {
                this.border_1().border_color(rgb(palette.link))
            })
            .cursor_pointer()
            .cursor_move()
            .on_drag(
                ConnectionDragPayload {
                    kind: ConnectionDragKind::Connection,
                    id: connection.id.clone(),
                    label: connection.name.clone(),
                },
                |payload, position, _, cx| {
                    cx.new(|_| ConnectionDragPreview::new(payload.clone(), position))
                },
            )
            .on_drag_move(cx.listener({
                let target_id = row_id.clone();
                move |this, event: &gpui::DragMoveEvent<ConnectionDragPayload>, _, cx| {
                    let _payload = event.drag(cx);
                    let y = event.event.position.y;
                    let bounds = event.bounds;
                    let rel = if bounds.size.height > px(0.) {
                        ((y - bounds.origin.y) / bounds.size.height).clamp(0., 1.)
                    } else {
                        0.5
                    };
                    let position = if rel < 0.33 {
                        ConnectionDropPosition::Before
                    } else if rel > 0.66 {
                        ConnectionDropPosition::After
                    } else {
                        // Mid band: treat as before for connections (reorder only).
                        ConnectionDropPosition::Before
                    };
                    let next = ConnectionDropTarget {
                        id: Some(target_id.clone()),
                        kind: ConnectionDragKind::Connection,
                        position,
                    };
                    if this.connection_state.set_list_drop_target_if_changed(next) {
                        cx.notify();
                    }
                }
            }))
            .on_drop({
                let target_id = connection.id.clone();
                cx.listener(move |this, payload: &ConnectionDragPayload, _, cx| {
                    let position = this
                        .connection_state
                        .list_drop_position_for_target(&target_id, ConnectionDropPosition::Before);
                    this.connection_state.clear_list_drop_target();
                    match payload.kind {
                        ConnectionDragKind::Connection => match position {
                            ConnectionDropPosition::After => {
                                this.move_connection_after(
                                    payload.id.clone(),
                                    target_id.clone(),
                                    cx,
                                );
                            }
                            _ => {
                                this.move_connection_before(
                                    payload.id.clone(),
                                    target_id.clone(),
                                    cx,
                                );
                            }
                        },
                        ConnectionDragKind::Group => {
                            let parent = this
                                .connection_catalog
                                .connections
                                .iter()
                                .find(|c| c.id == target_id)
                                .and_then(|c| c.group_id.clone());
                            this.move_group_into_group(payload.id.clone(), parent, cx);
                        }
                    }
                })
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| {
                    cx.stop_propagation();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.open_connection_context_menu(menu_id.clone(), event, cx);
                }),
            )
            .on_click(
                cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                    cx.stop_propagation();
                    if event.click_count() >= 2 {
                        this.start_saved_connection(connect_connection_dbl.clone(), window, cx);
                        return;
                    }
                    let modifiers = event.modifiers();
                    let additive = modifiers.control || modifiers.platform;
                    let range = modifiers.shift;
                    this.select_connection(select_id.clone(), additive, range, cx);
                }),
            )
            // Single-line name; the detail card is a real tooltip so it can hang
            // outside the panel instead of covering the rows underneath it.
            .child(connection_type_icon(palette, icon_def, selected, 16.))
            .child(
                div()
                    .id(SharedString::from(format!(
                        "connection-row-name-{}",
                        connection.id
                    )))
                    .flex_none()
                    .text_size(px(12.))
                    .font_weight(FontWeight(500.))
                    .text_color(if selected {
                        rgb(palette.link)
                    } else {
                        rgb(palette.text)
                    })
                    // Full name, never clipped — the list scrolls to reach it.
                    .whitespace_nowrap()
                    .child(connection.name.clone())
                    .tooltip(move |_, cx| {
                        cx.new(|_| ConnectionDetailsTooltip::new(details_rows.clone()))
                            .into()
                    }),
            )
            .child(div().flex_1().min_w_0())
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_0()
                    .rounded_sm()
                    .opacity(0.)
                    .group_hover(row_group.clone(), |this| {
                        this.bg(rgb(palette.hover)).opacity(1.)
                    })
                    .child(icon_action_button(
                        palette,
                        format!("connection-connect-{}", connection.id),
                        "icons/conn/connect.svg",
                        self.tr("savedConnections.connect"),
                        cx.listener(move |this, _, window, cx| {
                            this.start_saved_connection(connect_connection.clone(), window, cx);
                        }),
                    ))
                    .child(icon_action_button(
                        palette,
                        format!("connection-edit-{}", connection.id),
                        "icons/net/edit.svg",
                        self.tr("savedConnections.edit"),
                        cx.listener(move |this, _, window, cx| {
                            this.open_connection_editor(
                                Some(edit_id.clone()),
                                None,
                                false,
                                window,
                                cx,
                            );
                        }),
                    )),
            )
            .when(show_before, |this| {
                this.child(
                    div()
                        .absolute()
                        .left(px(8.))
                        .right(px(8.))
                        .top_0()
                        .h(px(2.))
                        .rounded_full()
                        .bg(rgb(palette.link)),
                )
            })
            .when(show_after, |this| {
                this.child(
                    div()
                        .absolute()
                        .left(px(8.))
                        .right(px(8.))
                        .bottom_0()
                        .h(px(2.))
                        .rounded_full()
                        .bg(rgb(palette.link)),
                )
            })
    }
}
