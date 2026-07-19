use super::*;

const CONNECTION_HOVER_INTENT_DELAY: Duration = Duration::from_millis(350);

impl NyaTermApp {
    pub(in crate::features) fn dismiss_connection_hover(&mut self, cx: &mut Context<Self>) {
        let had_hovered = self.hovered_connection_id.take().is_some();
        let had_pending = self.connection_hover_pending.take().is_some();
        let changed = had_hovered || had_pending;
        if changed {
            cx.notify();
        }
    }

    pub(in crate::features) fn poll_connection_hover_delay(&mut self) -> bool {
        let Some((connection_id, started_at)) = self.connection_hover_pending.clone() else {
            return false;
        };
        if !connection_hover_intent_ready(started_at, Instant::now()) {
            return false;
        }
        self.connection_hover_pending = None;
        if self.hovered_connection_id.as_deref() == Some(connection_id.as_str()) {
            return false;
        }
        self.hovered_connection_id = Some(connection_id);
        true
    }

    fn begin_connection_hover_intent(&mut self, connection_id: String) {
        if self.hovered_connection_id.as_deref() == Some(connection_id.as_str())
            || self
                .connection_hover_pending
                .as_ref()
                .is_some_and(|(pending_id, _)| pending_id == &connection_id)
        {
            return;
        }
        self.connection_hover_pending = Some((connection_id, Instant::now()));
    }

    fn clear_connection_hover_intent(&mut self, connection_id: &str) -> bool {
        let mut changed = false;
        if self
            .connection_hover_pending
            .as_ref()
            .is_some_and(|(pending_id, _)| pending_id == connection_id)
        {
            self.connection_hover_pending = None;
        }
        if self.hovered_connection_id.as_deref() == Some(connection_id) {
            self.hovered_connection_id = None;
            changed = true;
        }
        changed
    }

    pub(in crate::features) fn connection_section(
        &mut self,
        section: ConnectionSection,
        header_only: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let expanded = section
            .group_id
            .as_ref()
            .map(|id| self.expanded_connection_groups.contains(id))
            .unwrap_or(true);
        let group_id = section.group_id.clone();
        let group_id_for_edit = section.group_id.clone();
        let group_id_for_delete = section.group_id.clone();
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
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_2()
                    .pl(px(8. + section.depth as f32 * 16.))
                    .cursor_pointer()
                    .bg({
                        let drop_inside =
                            self.connection_drop_target.as_ref().is_some_and(|target| {
                                target.kind == ConnectionDragKind::Group
                                    && target.position == ConnectionDropPosition::Inside
                                    && target.id.as_deref() == section.group_id.as_deref()
                            });
                        if drop_inside {
                            rgb(palette.hover)
                        } else if section.group_id.as_ref().is_some_and(|id| {
                            self.hovered_connection_group_id.as_deref() == Some(id.as_str())
                        }) {
                            rgb(palette.hover)
                        } else {
                            rgb(palette.surface)
                        }
                    })
                    .when(
                        self.connection_drop_target.as_ref().is_some_and(|target| {
                            target.kind == ConnectionDragKind::Group
                                && target.position == ConnectionDropPosition::Inside
                                && target.id.as_deref() == section.group_id.as_deref()
                        }),
                        |this| this.border_1().border_color(rgb(self.theme_palette().link)),
                    )
                    .on_hover({
                        let hover_group = section.group_id.clone();
                        cx.listener(move |this, hovered: &bool, _, cx| {
                            if let Some(group_id) = hover_group.clone() {
                                if *hovered {
                                    this.hovered_connection_group_id = Some(group_id);
                                } else if this.hovered_connection_group_id.as_deref()
                                    == Some(group_id.as_str())
                                {
                                    this.hovered_connection_group_id = None;
                                }
                                cx.notify();
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
                                    if this.connection_drop_target.as_ref() != Some(&next) {
                                        this.connection_drop_target = Some(next);
                                        cx.notify();
                                    }
                                }
                            }))
                            .on_drop(cx.listener(
                                move |this, payload: &ConnectionDragPayload, _, cx| {
                                    let position = this
                                        .connection_drop_target
                                        .as_ref()
                                        .filter(|t| t.id.as_deref() == Some(drop_group_id.as_str()))
                                        .map(|t| t.position)
                                        .unwrap_or(ConnectionDropPosition::Inside);
                                    this.connection_drop_target = None;
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
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .min_w_0()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(if expanded { "▾" } else { "▸" }),
                            )
                            .child(connection_type_icon(
                                palette,
                                resolve_connection_icon(Some("folder"), "SSH"),
                                false,
                                13.,
                            ))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(palette.text))
                                    .child(truncate_preview(&group_label, 28)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text_dimmed))
                                    .child(count.to_string()),
                            ),
                    )
                    .when(!section.is_root, |this| {
                        let show_group_actions = section.group_id.as_ref().is_some_and(|id| {
                            self.hovered_connection_group_id.as_deref() == Some(id.as_str())
                        });
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .opacity(if show_group_actions { 1. } else { 0. })
                                .child(icon_action_button(
                                    palette,
                                    format!(
                                        "connection-group-edit-{}",
                                        group_id_for_edit.clone().unwrap_or_default()
                                    ),
                                    "icons/net/edit.svg",
                                    self.tr("savedConnections.renameFolder"),
                                    cx.listener(move |this, _, window, cx| {
                                        cx.stop_propagation();
                                        this.open_connection_group_editor(
                                            group_id_for_edit.clone(),
                                            None,
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(icon_action_button(
                                    palette,
                                    format!(
                                        "connection-group-delete-{}",
                                        group_id_for_delete.clone().unwrap_or_default()
                                    ),
                                    "icons/net/delete.svg",
                                    self.tr("savedConnections.deleteFolder"),
                                    cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        if let Some(group_id) = group_id_for_delete.clone() {
                                            this.open_connection_group_delete_confirm(group_id, cx);
                                        }
                                    }),
                                )),
                        )
                    }),
            )
            .child(body)
    }

    pub(in crate::features) fn saved_connection_row(
        &mut self,
        connection: SavedConnection,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.selected_connection_ids.contains(&connection.id);
        let hovered = self.hovered_connection_id.as_deref() == Some(connection.id.as_str());
        let connect_connection = connection.clone();
        let connect_connection_dbl = connection.clone();
        let edit_id = connection.id.clone();
        let delete_id = connection.id.clone();
        let select_id = connection.id.clone();
        let hover_id = connection.id.clone();
        let menu_id = connection.id.clone();
        let kind = connection.kind_label();
        let icon_def = resolve_connection_icon(connection.icon.as_deref(), kind);
        let show_actions = hovered || selected;
        let details_tooltip = self.connection_details_tooltip(connection.clone());
        let _endpoint = connection.endpoint();
        let _last_used = format_last_used_ms(connection.last_used_at_ms);
        let drop_target = self.connection_drop_target.as_ref().filter(|target| {
            target.kind == ConnectionDragKind::Connection
                && target.id.as_deref() == Some(connection.id.as_str())
        });
        let show_before = drop_target.is_some_and(|t| t.position == ConnectionDropPosition::Before);
        let show_after = drop_target.is_some_and(|t| t.position == ConnectionDropPosition::After);
        let show_inside = drop_target.is_some_and(|t| t.position == ConnectionDropPosition::Inside);
        let row_id = connection.id.clone();

        // Tauri ConnectionItem: py-1.5 single-line row (~34px) with hover actions.
        let palette = self.theme_palette();
        div()
            .id(SharedString::from(format!(
                "connection-row-{}",
                connection.id
            )))
            .relative()
            .h(px(34.))
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .pl(px(connection_tree_indent_px(depth)))
            .bg(if selected {
                rgb(palette.hover)
            } else if show_inside {
                rgb(palette.hover)
            } else if hovered {
                rgb(palette.hover)
            } else {
                rgb(palette.surface)
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
                    if this.connection_drop_target.as_ref() != Some(&next) {
                        this.connection_drop_target = Some(next);
                        cx.notify();
                    }
                }
            }))
            .on_drop({
                let target_id = connection.id.clone();
                cx.listener(move |this, payload: &ConnectionDragPayload, _, cx| {
                    let position = this
                        .connection_drop_target
                        .as_ref()
                        .filter(|target| target.id.as_deref() == Some(target_id.as_str()))
                        .map(|target| target.position)
                        .unwrap_or(ConnectionDropPosition::Before);
                    this.connection_drop_target = None;
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
                                .connections
                                .iter()
                                .find(|c| c.id == target_id)
                                .and_then(|c| c.group_id.clone());
                            this.move_group_into_group(payload.id.clone(), parent, cx);
                        }
                    }
                })
            })
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                if *hovered {
                    this.begin_connection_hover_intent(hover_id.clone());
                } else if this.clear_connection_hover_intent(&hover_id) {
                    cx.notify();
                }
            }))
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
            // Single-line name row; details live in the explicit selection panel.
            .child(connection_type_icon(palette, icon_def, selected, 14.))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .text_size(px(12.))
                    .font_weight(FontWeight(500.))
                    .text_color(if selected {
                        rgb(palette.link)
                    } else {
                        rgb(palette.text)
                    })
                    .overflow_hidden()
                    .child(truncate_preview(&connection.name, 48)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_0()
                    .rounded_sm()
                    .bg(if show_actions {
                        rgb(palette.hover)
                    } else {
                        rgb(palette.surface)
                    })
                    .opacity(if show_actions { 1. } else { 0. })
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
                    ))
                    .child(icon_action_button(
                        palette,
                        format!("connection-delete-{}", connection.id),
                        "icons/net/delete.svg",
                        self.tr("savedConnections.delete"),
                        cx.listener(move |this, _, _, cx| {
                            this.open_connection_delete_confirm(delete_id.clone(), cx);
                        }),
                    )),
            )
            .when(hovered, |this| this.child(details_tooltip))
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

    pub(in crate::features) fn connection_details_tooltip(
        &self,
        connection: SavedConnection,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let rows = connection_detail_rows(&connection, &self.connections, &self.proxies);
        let mut grid = div().grid().gap_1();
        for (label, value) in rows.into_iter().take(6) {
            grid = grid.child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(
                        div()
                            .w(px(66.))
                            .flex_none()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(label),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text))
                            .child(truncate_preview(&value, 52)),
                    ),
            );
        }

        div()
            .absolute()
            .top(px(30.))
            .left(px(28.))
            .w(px(260.))
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface_elevated))
            .px_3()
            .py_2()
            .child(
                div()
                    .mb_1()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .text_size(px(11.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text))
                            .child(truncate_preview(&connection.name, 34)),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(connection.kind_label()),
                    ),
            )
            .child(grid)
    }
}

fn connection_hover_intent_ready(started_at: Instant, now: Instant) -> bool {
    now.saturating_duration_since(started_at) >= CONNECTION_HOVER_INTENT_DELAY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_hover_intent_waits_for_delay() {
        let started_at = Instant::now();

        assert!(!connection_hover_intent_ready(
            started_at,
            started_at + CONNECTION_HOVER_INTENT_DELAY - Duration::from_millis(1)
        ));
        assert!(connection_hover_intent_ready(
            started_at,
            started_at + CONNECTION_HOVER_INTENT_DELAY
        ));
    }
}
