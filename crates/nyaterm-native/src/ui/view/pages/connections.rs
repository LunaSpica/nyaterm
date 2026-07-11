use gpui::{
    Context, FontWeight, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ScrollDelta,
    ScrollWheelEvent, SharedString, div, prelude::*, px, rgb, svg,
};
use nyaterm_domain::{Group, ProxyConfig, SavedConnection, truncate_preview};
use std::collections::HashMap;

use crate::ui::components::small_button;
use crate::ui::models::{
    ConnectionContextMenuState, ConnectionDeleteConfirmState, ConnectionEditorField,
    ConnectionEditorState, ConnectionGroupContextMenuState, ConnectionGroupDeleteConfirmState,
    ConnectionGroupEditorState, ConnectionKindTab, ConnectionSortMode,
};

use super::super::{
    ConnectionDragKind, ConnectionDragPayload, ConnectionDragPreview, ConnectionDropPosition,
    ConnectionDropTarget, ConnectionEditorToggle, NyaTermApp, connection_type_icon,
    format_last_used_ms, modal_dialog_footer, modal_dialog_shell, resolve_connection_icon,
    transfer_input,
};

impl NyaTermApp {
    pub(in crate::ui::view) fn connections_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let query = self.connection_search_draft.trim().to_ascii_lowercase();
        let sections = connection_sections(
            &self.connections,
            &self.connection_groups,
            &query,
            self.connection_sort_mode,
        );
        let visible_count = sections
            .iter()
            .map(|section| section.connections.len())
            .sum::<usize>();
        let selected_count = self.selected_connections().len();

        // Flatten expanded tree for virtual window (group header 28px, connection 34px).
        let flat_rows = flatten_connection_rows(&sections, &self.expanded_connection_groups);
        const CONN_VIEWPORT_ROWS: usize = 36;
        const CONN_OVERSCAN: usize = 8;
        let total_rows = flat_rows.len();
        let window_capacity = CONN_VIEWPORT_ROWS + CONN_OVERSCAN * 2;
        let max_offset = total_rows.saturating_sub(CONN_VIEWPORT_ROWS.min(total_rows));
        if self.connection_list_offset > max_offset {
            self.connection_list_offset = max_offset;
        }
        let scroll_row = self.connection_list_offset.min(max_offset);
        let window_start = scroll_row.saturating_sub(CONN_OVERSCAN);
        let window_end = (window_start + window_capacity).min(total_rows);
        let pad_top: f32 = flat_rows
            .iter()
            .take(window_start)
            .map(ConnectionListRow::height_px)
            .sum();
        let pad_bottom: f32 = flat_rows
            .iter()
            .skip(window_end)
            .map(ConnectionListRow::height_px)
            .sum();
        let visible_rows = flat_rows
            .get(window_start..window_end)
            .unwrap_or(&[])
            .to_vec();
        let palette = self.theme_palette();

        let mut list = div()
            .id(SharedString::from("connections-list-scroll"))
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .p_1()
            .flex()
            .flex_col()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    // Click empty background clears multi-select (Tauri list onMouseDown).
                    if !this.selected_connection_ids.is_empty() {
                        this.clear_selected_connections(cx);
                    }
                }),
            )
            .on_drop(cx.listener(|this, payload: &ConnectionDragPayload, _, cx| {
                this.connection_drop_target = None;
                match payload.kind {
                    ConnectionDragKind::Connection => {
                        this.move_connection_into_group(payload.id.clone(), None, cx);
                    }
                    ConnectionDragKind::Group => {
                        this.move_group_into_group(payload.id.clone(), None, cx);
                    }
                }
            }))
            .on_scroll_wheel(cx.listener(move |this, event: &ScrollWheelEvent, _, cx| {
                let max_offset = total_rows.saturating_sub(CONN_VIEWPORT_ROWS.min(total_rows));
                if max_offset == 0 {
                    return;
                }
                let delta_rows = match event.delta {
                    ScrollDelta::Lines(delta) => delta.y,
                    ScrollDelta::Pixels(delta) => f32::from(delta.y) / 34.,
                };
                let next = (this.connection_list_offset as f32 - delta_rows)
                    .round()
                    .clamp(0., max_offset as f32) as usize;
                if next != this.connection_list_offset {
                    this.connection_list_offset = next;
                    cx.stop_propagation();
                    cx.notify();
                }
            }));
        if self.connections.is_empty() {
            list = list.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .px_4()
                    .py_8()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette.text_muted))
                            .child("No saved connections"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_dimmed))
                            .child("Create one or open a temporary SSH link."),
                    ),
            );
        } else if visible_count == 0 {
            list = list.child(
                div()
                    .px_4()
                    .py_8()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child("No connections match the current search."),
            );
        } else {
            let mut rows = div().flex().flex_col();
            if pad_top > 0. {
                rows = rows.child(div().h(px(pad_top)).w_full().flex_none());
            }
            for row in visible_rows {
                match row {
                    ConnectionListRow::Separator => {
                        rows = rows.child(
                            div()
                                .mx_2()
                                .my_1()
                                .h(px(1.))
                                .bg(rgb(palette.border)),
                        );
                    }
                    ConnectionListRow::GroupHeader(section) => {
                        rows = rows.child(self.connection_section(section, true, cx));
                    }
                    ConnectionListRow::EmptyGroup => {
                        rows = rows.child(
                            div()
                                .px_2()
                                .py_1()
                                .pl(px(28.))
                                .h(px(28.))
                                .text_size(px(11.))
                                .text_color(rgb(palette.text_dimmed))
                                .child("Empty group"),
                        );
                    }
                    ConnectionListRow::Connection { connection, indented } => {
                        rows = rows.child(self.saved_connection_row(connection, indented, cx));
                    }
                }
            }
            if pad_bottom > 0. {
                rows = rows.child(div().h(px(pad_bottom)).w_full().flex_none());
            }
            list = list.child(rows);
        }

        // Tauri: PanelHeader (shared stack) + search/action strip + flat tree list.
        // Count is shown in the shared panel header via meta; strip hosts search + icons.
        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(rgb(palette.surface))
            .child(self.connections_search_bar(visible_count, cx))
            .when(selected_count > 0, |this| {
                this.child(self.connections_selection_strip(selected_count, cx))
            })
            .child(list)
            .when_some(self.connection_editor.clone(), |this, editor| {
                this.child(self.connection_editor_panel(editor, cx))
            })
            .when_some(self.connection_group_editor.clone(), |this, editor| {
                this.child(self.connection_group_editor_panel(editor, cx))
            })
            .when_some(self.connection_delete_confirm.clone(), |this, confirm| {
                this.child(self.connection_delete_confirm_panel(confirm, cx))
            })
            .when_some(
                self.connection_group_delete_confirm.clone(),
                |this, confirm| this.child(self.connection_group_delete_confirm_panel(confirm, cx)),
            )
            .when(self.connection_context_menu.is_some(), |this| {
                this.child(self.connection_context_menu_overlay(cx))
            })
            .when(self.connection_group_context_menu.is_some(), |this| {
                this.child(self.connection_group_context_menu_overlay(cx))
            })
            .when_some(self.connection_details_tooltip_id.clone(), |this, id| {
                this.child(self.connection_details_tooltip(id, cx))
            })
    }

    fn connections_search_bar(
        &mut self,
        visible_count: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let _ = visible_count;
        let search_value = if self.connection_search_draft.is_empty() {
            "Filter connections".to_string()
        } else {
            self.connection_search_draft.clone()
        };
        let sort_label = match self.connection_sort_mode {
            ConnectionSortMode::Default => "↕",
            ConnectionSortMode::NameAsc => "A↑",
            ConnectionSortMode::NameDesc => "A↓",
            ConnectionSortMode::Recent => "⏱",
        };
        let more_open = self.connections_more_menu_open;

        // Tauri search strip: px-2 py-1.5, input h-7.
        let palette = self.theme_palette();
        div()
            .h(px(36.))
            .px_2()
            .flex()
            .items_center()
            .gap_1()
            .border_b_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.section_header))
            .child(
                div()
                    .id(SharedString::from("connection-search-input"))
                    .h(px(28.))
                    .flex_1()
                    .min_w_0()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.hover))
                    .px_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .track_focus(&self.connection_search_focus)
                    .on_click(cx.listener(|this, _, window, cx| {
                        window.focus(&this.connection_search_focus);
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        cx.stop_propagation();
                        this.handle_connection_search_key_down(event, cx);
                    }))
                    .child(
                        svg()
                            .size(px(14.))
                            .flex_none()
                            .path("icons/fe/search.svg")
                            .text_color(rgb(palette.text_dimmed)),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(12.))
                            .text_color(if self.connection_search_draft.is_empty() {
                                rgb(palette.text_dimmed)
                            } else {
                                rgb(palette.text)
                            })
                            .child(search_value),
                    )
                    .when(!self.connection_search_draft.is_empty(), |this| {
                        this.child(
                            div()
                                .id(SharedString::from("connection-search-clear"))
                                .size(px(18.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_sm()
                                .text_size(px(10.))
                                .text_color(rgb(palette.text_muted))
                                .cursor_pointer()
                                .hover(move |this| {
                                    this.bg(rgb(palette.surface_elevated))
                                        .text_color(rgb(palette.text))
                                })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.connection_search_draft.clear();
                                    this.connection_list_offset = 0;
                                    window.focus(&this.connection_search_focus);
                                    cx.notify();
                                }))
                                .child("×"),
                        )
                    }),
            )
            // Count lives in PanelHeader (Tauri).
            .child(icon_action_button(palette, "connections-sort",
                sort_label,
                cx.listener(|this, _, _, cx| {
                    this.cycle_connection_sort_mode(cx);
                }),
            ))
            .child(icon_action_button(palette, "connections-temp-ssh",
                "icons/conn/flash.svg",
                cx.listener(|this, _, window, cx| {
                    this.open_temporary_ssh_link_dialog(window, cx);
                }),
            ))
            .child(icon_action_button(palette, "connections-new-group",
                "icons/conn/folder.svg",
                cx.listener(|this, _, window, cx| {
                    this.open_connection_group_editor(None, None, window, cx);
                }),
            ))
            .child(icon_action_button(palette, "connections-new",
                "icons/conn/add.svg",
                cx.listener(|this, _, window, cx| {
                    this.open_connection_editor(None, None, false, window, cx);
                }),
            ))
            .child(
                div()
                    .relative()
                    .child(icon_action_button(palette, "connections-more",
                        "icons/conn/more.svg",
                        cx.listener(|this, _, _, cx| {
                            this.connections_more_menu_open = !this.connections_more_menu_open;
                            cx.notify();
                        }),
                    ))
                    .when(more_open, |this| {
                        this.child(
                            div()
                                .id(SharedString::from("connections-more-menu"))
                                .absolute()
                                .top(px(30.))
                                .right(px(0.))
                                .w(px(148.))
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.surface))
                                .shadow_sm()
                                .py_1()
                                                                .child(menu_item(
                                    palette,
                                    "connections-export",
                                    "Export config",
                                    cx.listener(|this, _, _, cx| {
                                        this.connections_more_menu_open = false;
                                        this.prompt_config_export(cx);
                                    }),
                                ))
                                .child(menu_item(
                                    palette,
                                    "connections-import",
                                    "Import config",
                                    cx.listener(|this, _, _, cx| {
                                        this.connections_more_menu_open = false;
                                        this.prompt_config_import(cx);
                                    }),
                                ))
                                .child(menu_item(
                                    palette,
                                    "connections-refresh",
                                    "Refresh",
                                    cx.listener(|this, _, _, cx| {
                                        this.connections_more_menu_open = false;
                                        this.refresh_store_from_runtime();
                                        this.terminal_status = "connections refreshed".to_string();
                                        cx.notify();
                                    }),
                                ))
                                .child(menu_item(
                                    palette,
                                    "connections-local",
                                    "Local shell",
                                    cx.listener(|this, _, window, cx| {
                                        this.connections_more_menu_open = false;
                                        this.start_local_session(window, cx);
                                    }),
                                )),
                        )
                    }),
            )
    }

    fn connection_section(
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
        let count = section.connections.len();
        let mut body = div().flex().flex_col();

        if expanded && !header_only {
            if section.connections.is_empty() && !section.is_root {
                body = body.child(
                    div()
                        .px_2()
                        .py_1()
                        .pl(px(28.))
                        .text_size(px(11.))
                        .text_color(rgb(palette.text_dimmed))
                        .child("Empty group"),
                );
            } else {
                for connection in section.connections {
                    body = body.child(self.saved_connection_row(connection, !section.is_root, cx));
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
                    .cursor_pointer()
                    .bg({
                        let drop_inside = self.connection_drop_target.as_ref().is_some_and(|target| {
                            target.kind == ConnectionDragKind::Group
                                && target.position == ConnectionDropPosition::Inside
                                && target.id.as_deref()
                                    == section.group_id.as_deref()
                        });
                        if drop_inside {
                            rgb(palette.hover)
                        } else if section
                            .group_id
                            .as_ref()
                            .is_some_and(|id| {
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
                        |this| this.border_1().border_color(rgb(self.theme_palette().accent)),
                    )
                    .on_hover({
                        let hover_group = section.group_id.clone();
                        cx.listener(move |this, hovered: &bool, _, cx| {
                            if let Some(group_id) = hover_group.clone() {
                                if *hovered {
                                    this.hovered_connection_group_id = Some(group_id);
                                } else if this.hovered_connection_group_id.as_deref()
                                    == Some(group_id.as_str()) {
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
                    .on_mouse_down(
                        MouseButton::Right,
                        {
                            let menu_group = section.group_id.clone();
                            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                if let Some(group_id) = menu_group.clone() {
                                    cx.stop_propagation();
                                    this.open_connection_group_context_menu(group_id, event, cx);
                                }
                            })
                        },
                    )
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
                                    cx.new(|_| ConnectionDragPreview::new(payload.clone(), position))
                                },
                            )
                            .on_drag_move(cx.listener({
                                let target_id = drop_group_id.clone();
                                move |this, event: &gpui::DragMoveEvent<ConnectionDragPayload>, _, cx| {
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
                            .on_drop(cx.listener(move |this, payload: &ConnectionDragPayload, _, cx| {
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
                            }))
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
                        let show_group_actions = section
                            .group_id
                            .as_ref()
                            .is_some_and(|id| {
                                self.hovered_connection_group_id.as_deref() == Some(id.as_str())
                            });
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .opacity(if show_group_actions { 1. } else { 0. })
                                .child(icon_action_button(palette, format!(
                                        "connection-group-edit-{}",
                                        group_id_for_edit.clone().unwrap_or_default()
                                    ),
                                    "icons/net/edit.svg",
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
                                .child(icon_action_button(palette, format!(
                                        "connection-group-delete-{}",
                                        group_id_for_delete.clone().unwrap_or_default()
                                    ),
                                    "icons/net/delete.svg",
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

    fn connections_selection_strip(
        &mut self,
        selected_count: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Tauri multi-select strip under search bar.
        div()
            .h(px(30.))
            .px_2()
            .flex()
            .items_center()
            .gap_1()
            .border_b_1()
            .border_color(rgb(palette.accent))
            .bg(rgb(0x0d2137))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .text_size(px(11.))
                    .font_weight(FontWeight(600.))
                    .text_color(rgb(palette.accent))
                    .child(format!("{selected_count} selected")),
            )
            .child(
                div()
                    .id(SharedString::from("connections-selection-open"))
                    .h(px(22.))
                    .px_2()
                    .rounded_sm()
                    .flex()
                    .items_center()
                    .text_size(px(11.))
                    .font_weight(FontWeight(600.))
                    .text_color(rgb(palette.text))
                    .bg(rgb(palette.surface_elevated))
                    .cursor_pointer()
                    .hover(|this| this.bg(rgb(palette.border)))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.start_selected_saved_connections(window, cx);
                    }))
                    .child("Open"),
            )
            .child(
                div()
                    .id(SharedString::from("connections-selection-copy"))
                    .h(px(22.))
                    .px_2()
                    .rounded_sm()
                    .flex()
                    .items_center()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text))
                    .cursor_pointer()
                    .hover(|this| this.bg(rgb(palette.surface_elevated)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.copy_selected_connections(cx);
                    }))
                    .child("Copy"),
            )
            .child(
                div()
                    .id(SharedString::from("connections-selection-delete"))
                    .h(px(22.))
                    .px_2()
                    .rounded_sm()
                    .flex()
                    .items_center()
                    .text_size(px(11.))
                    .text_color(rgb(palette.danger))
                    .cursor_pointer()
                    .hover(|this| this.bg(rgb(0x3a1717)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.delete_selected_connections(cx);
                    }))
                    .child("Delete"),
            )
            .child(
                div()
                    .id(SharedString::from("connections-selection-clear"))
                    .h(px(22.))
                    .px_2()
                    .rounded_sm()
                    .flex()
                    .items_center()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_muted))
                    .cursor_pointer()
                    .hover(|this| this.bg(rgb(palette.surface_elevated)).text_color(rgb(palette.text)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.clear_selected_connections(cx);
                    }))
                    .child("Clear"),
            )
    }

    fn saved_connection_row(
        &mut self,
        connection: SavedConnection,
        indented: bool,
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
            .id(SharedString::from(format!("connection-row-{}", connection.id)))
            .relative()
            .h(px(34.))
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .pl(if indented { px(24.) } else { px(8.) })
            .bg(if selected {
                rgb(palette.hover)
            } else if show_inside {
                rgb(palette.hover)
            } else if hovered {
                rgb(palette.hover)
            } else {
                rgb(palette.surface)
            })
            .when(show_inside, |this| this.border_1().border_color(rgb(palette.accent)))
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
                        this.connection_details_tooltip_id = None;
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
                    this.hovered_connection_id = Some(hover_id.clone());
                    this.connection_details_tooltip_id = Some(hover_id.clone());
                } else if this.hovered_connection_id.as_deref() == Some(hover_id.as_str()) {
                    this.hovered_connection_id = None;
                    if this.connection_details_tooltip_id.as_deref() == Some(hover_id.as_str()) {
                        this.connection_details_tooltip_id = None;
                    }
                }
                cx.notify();
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
            .on_click(cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                cx.stop_propagation();
                if event.click_count() >= 2 {
                    this.start_saved_connection(connect_connection_dbl.clone(), window, cx);
                    return;
                }
                let modifiers = event.modifiers();
                let additive = modifiers.control || modifiers.platform;
                let range = modifiers.shift;
                this.select_connection(select_id.clone(), additive, range, cx);
            }))
            // Single-line name row (endpoint/last-used live in hover tooltip, like Tauri).
            .child(connection_type_icon(palette, icon_def, selected, 14.))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .text_size(px(12.))
                    .font_weight(FontWeight(500.))
                    .text_color(if selected {
                        rgb(palette.accent)
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
                    .bg(if show_actions { rgb(palette.hover) } else { rgb(palette.surface) })
                    .opacity(if show_actions { 1. } else { 0. })
                    .child(icon_action_button(palette, format!("connection-connect-{}", connection.id),
                        "icons/conn/connect.svg",
                        cx.listener(move |this, _, window, cx| {
                            this.start_saved_connection(connect_connection.clone(), window, cx);
                        }),
                    ))
                    .child(icon_action_button(palette, format!("connection-edit-{}", connection.id),
                        "icons/net/edit.svg",
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
                    .child(icon_action_button(palette, format!("connection-delete-{}", connection.id),
                        "icons/net/delete.svg",
                        cx.listener(move |this, _, _, cx| {
                            this.open_connection_delete_confirm(delete_id.clone(), cx);
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
                        .bg(rgb(palette.accent)),
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
                        .bg(rgb(palette.accent)),
                )
            })
    }


    fn connection_details_tooltip(
        &self,
        connection_id: String,
        _cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let palette = self.theme_palette();
        let Some(connection) = self
            .connections
            .iter()
            .find(|connection| connection.id == connection_id)
        else {
            return div().into_any_element();
        };
        let rows = connection_detail_rows(connection, &self.connections, &self.proxies);
        let mut grid = div().flex().flex_col().gap_1();
        for (label, value) in rows {
            grid = grid.child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(
                        div()
                            .w(px(64.))
                            .flex_none()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(label),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text))
                            .child(value),
                    ),
            );
        }
        div()
            .id(SharedString::from(format!(
                "connection-details-tooltip-{}",
                connection.id
            )))
            .absolute()
            .left(px(8.))
            .top(px(38.))
                        .w(px(220.))
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .shadow_lg()
            .px_2()
            .py_2()
            .child(grid)
            .into_any_element()
    }

    fn connection_editor_panel(
        &mut self,
        editor: ConnectionEditorState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = if editor.id.is_some() {
            "Edit Connection"
        } else {
            "New Connection"
        };
        let group_label = editor
            .group_id
            .as_deref()
            .and_then(|id| {
                self.connection_groups
                    .iter()
                    .find(|group| group.id == id)
                    .map(|group| group.name.clone())
            })
            .unwrap_or_else(|| "Ungrouped".to_string());
        let key_label = editor
            .key_id
            .as_deref()
            .and_then(|id| {
                self.connection_ssh_keys
                    .iter()
                    .find(|key| key.id == id)
                    .map(|key| key.name.clone())
            })
            .unwrap_or_else(|| "No key".to_string());
        let otp_label = editor
            .otp_id
            .as_deref()
            .and_then(|id| {
                self.connection_otp_entries
                    .iter()
                    .find(|entry| entry.id == id)
                    .map(|entry| {
                        if entry.issuer.is_empty() {
                            entry.username.clone()
                        } else if entry.username.is_empty() {
                            entry.issuer.clone()
                        } else {
                            format!("{} ({})", entry.issuer, entry.username)
                        }
                    })
            })
            .unwrap_or_else(|| "No OTP".to_string());
        let proxy_label = editor
            .proxy_id
            .as_deref()
            .and_then(|id| {
                self.proxies
                    .iter()
                    .find(|proxy| proxy.id == id)
                    .map(|proxy| {
                        if proxy.protocol == "proxycommand" {
                            let command = proxy.command.as_deref().unwrap_or("").trim();
                            if command.is_empty() {
                                format!("{} · {}", proxy.name, proxy.protocol.to_ascii_uppercase())
                            } else {
                                format!(
                                    "{} · {} {}",
                                    proxy.name,
                                    proxy.protocol.to_ascii_uppercase(),
                                    truncate_preview(command, 18)
                                )
                            }
                        } else {
                            format!(
                                "{} · {} {}:{}",
                                proxy.name,
                                proxy.protocol.to_ascii_uppercase(),
                                proxy.host,
                                proxy.port
                            )
                        }
                    })
            })
            .unwrap_or_else(|| "No Proxy".to_string());
        let jump_label = editor
            .proxy_jump_id
            .as_deref()
            .and_then(|id| {
                self.connections
                    .iter()
                    .find(|connection| connection.id == id)
                    .map(|connection| connection.name.clone())
            })
            .unwrap_or_else(|| "No Jump Host".to_string());
        let password_display = if editor.password.is_empty() {
            if editor.existing_password.is_some() {
                "•••••••• (saved)".to_string()
            } else {
                String::new()
            }
        } else {
            "•".repeat(editor.password.chars().count().min(24))
        };

        let save_label = if editor.connect_after_save {
            "Save+Open"
        } else {
            "Save"
        };
        let card = div()
            .id(SharedString::from("connection-editor-panel"))
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .max_h(px(640.))
            .overflow_hidden()
            .track_focus(&self.connection_editor_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.connection_editor_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_connection_editor_key_down(event, window, cx);
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text))
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette.text_muted))
                            .child("Create or edit a saved session profile."),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(kind_chip(
                        palette,
                        "SSH",
                        editor.kind == ConnectionKindTab::Ssh,
                        cx.listener(|this, _, _, cx| {
                            this.set_connection_editor_kind(ConnectionKindTab::Ssh, cx);
                        }),
                    ))
                    .child(kind_chip(
                        palette,
                        "Local",
                        editor.kind == ConnectionKindTab::Local,
                        cx.listener(|this, _, _, cx| {
                            this.set_connection_editor_kind(ConnectionKindTab::Local, cx);
                        }),
                    ))
                    .child(kind_chip(
                        palette,
                        "Telnet",
                        editor.kind == ConnectionKindTab::Telnet,
                        cx.listener(|this, _, _, cx| {
                            this.set_connection_editor_kind(ConnectionKindTab::Telnet, cx);
                        }),
                    ))
                    .child(kind_chip(
                        palette,
                        "Serial",
                        editor.kind == ConnectionKindTab::Serial,
                        cx.listener(|this, _, _, cx| {
                            this.set_connection_editor_kind(ConnectionKindTab::Serial, cx);
                        }),
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(editor_field(
                        palette,
                        "connection-editor-name",
                        "Name",
                        editor.name.clone(),
                        editor.focused_field == ConnectionEditorField::Name,
                        cx.listener(|this, _, window, cx| {
                            this.focus_connection_editor_field(ConnectionEditorField::Name, window, cx);
                        }),
                    ))
                    .child(editor_field(
                        palette,
                        "connection-editor-description",
                        "Description",
                        editor.description.clone(),
                        editor.focused_field == ConnectionEditorField::Description,
                        cx.listener(|this, _, window, cx| {
                            this.focus_connection_editor_field(
                                ConnectionEditorField::Description,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text_muted))
                                    .child(format!("Group · {group_label}")),
                            )
                            .child(small_button(palette, 
                                "connection-editor-group",
                                "Cycle",
                                cx.listener(|this, _, _, cx| {
                                    this.cycle_connection_editor_group(cx);
                                }),
                            )),
                    )
                    .when(editor.kind == ConnectionKindTab::Ssh, |this| {
                        this.child(
                            div()
                                .grid()
                                .grid_cols(2)
                                .gap_2()
                                .child(editor_field(
                                    palette,
                                    "connection-editor-host",
                                    "Host",
                                    editor.host.clone(),
                                    editor.focused_field == ConnectionEditorField::Host,
                                    cx.listener(|this, _, window, cx| {
                                        this.focus_connection_editor_field(
                                            ConnectionEditorField::Host,
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(editor_field(
                                    palette,
                                    "connection-editor-port",
                                    "Port",
                                    editor.port.clone(),
                                    editor.focused_field == ConnectionEditorField::Port,
                                    cx.listener(|this, _, window, cx| {
                                        this.focus_connection_editor_field(
                                            ConnectionEditorField::Port,
                                            window,
                                            cx,
                                        );
                                    }),
                                )),
                        )
                        .child(editor_field(
                            palette,
                            "connection-editor-username",
                            "Username",
                            editor.username.clone(),
                            editor.focused_field == ConnectionEditorField::Username,
                            cx.listener(|this, _, window, cx| {
                                this.focus_connection_editor_field(
                                    ConnectionEditorField::Username,
                                    window,
                                    cx,
                                );
                            }),
                        ))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(format!("Auth · {}", editor.auth_mode)),
                                )
                                .child(small_button(palette, 
                                    "connection-editor-auth",
                                    "Cycle",
                                    cx.listener(|this, _, _, cx| {
                                        this.cycle_connection_editor_auth_mode(cx);
                                    }),
                                )),
                        )
                        .when(editor.auth_mode == "password", |this| {
                            this.child(editor_field(
                                palette,
                                "connection-editor-password",
                                "Password",
                                password_display.clone(),
                                editor.focused_field == ConnectionEditorField::Password,
                                cx.listener(|this, _, window, cx| {
                                    this.focus_connection_editor_field(
                                        ConnectionEditorField::Password,
                                        window,
                                        cx,
                                    );
                                }),
                            ))
                        })
                        .when(editor.auth_mode == "key" || editor.auth_mode == "certificate", |this| {
                            this.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child(format!("Key · {}", truncate_preview(&key_label, 24))),
                                    )
                                    .child(small_button(palette, 
                                        "connection-editor-key",
                                        "Cycle",
                                        cx.listener(|this, _, _, cx| {
                                            this.cycle_connection_editor_key(cx);
                                        }),
                                    )),
                            )
                        })
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(format!("OTP · {}", truncate_preview(&otp_label, 24))),
                                )
                                .child(small_button(palette, 
                                    "connection-editor-otp",
                                    "Cycle",
                                    cx.listener(|this, _, _, cx| {
                                        this.cycle_connection_editor_otp(cx);
                                    }),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(palette.text_muted))
                                                .child(format!(
                                                    "Proxy · {}",
                                                    truncate_preview(&proxy_label, 36)
                                                )),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(rgb(palette.text_dimmed))
                                                .child("Optional SOCKS/HTTP/ProxyCommand for this SSH target."),
                                        ),
                                )
                                .child(small_button(palette,
                                    "connection-editor-proxy",
                                    "Cycle",
                                    cx.listener(|this, _, _, cx| {
                                        this.cycle_connection_editor_proxy(cx);
                                    }),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(palette.text_muted))
                                                .child(format!(
                                                    "Jump Host · {}",
                                                    truncate_preview(&jump_label, 36)
                                                )),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(rgb(palette.text_dimmed))
                                                .child("ProxyJump via another saved SSH connection."),
                                        ),
                                )
                                .child(small_button(palette,
                                    "connection-editor-jump",
                                    "Cycle",
                                    cx.listener(|this, _, _, cx| {
                                        this.cycle_connection_editor_jump(cx);
                                    }),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(palette.text_muted))
                                                .child(format!(
                                                    "Backspace · {}",
                                                    match editor.backspace_mode.as_str() {
                                                        "ctrl-h" | "bs" | "ctrl_h" => "Ctrl+H (BS)",
                                                        _ => "DEL (0x7F)",
                                                    }
                                                )),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(rgb(palette.text_dimmed))
                                                .child("Terminal backspace key encoding for remote shells."),
                                        ),
                                )
                                .child(small_button(palette,
                                    "connection-editor-backspace",
                                    "Cycle",
                                    cx.listener(|this, _, _, cx| {
                                        this.cycle_connection_editor_backspace(cx);
                                    }),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(toggle_chip(
                                    palette,
                                    "OTP Fill",
                                    editor.auto_fill_otp,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_connection_editor_flag(
                                            ConnectionEditorToggle::AutoFillOtp,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(toggle_chip(
                                    palette,
                                    "X11",
                                    editor.x11_forwarding,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_connection_editor_flag(
                                            ConnectionEditorToggle::X11,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(toggle_chip(
                                    palette,
                                    "Post Login",
                                    editor.post_login_enabled,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_connection_editor_flag(
                                            ConnectionEditorToggle::PostLogin,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(toggle_chip(
                                    palette,
                                    "Open After Save",
                                    editor.connect_after_save,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_connection_editor_flag(
                                            ConnectionEditorToggle::ConnectAfterSave,
                                            cx,
                                        );
                                    }),
                                )),
                        )
                        .when(editor.post_login_enabled, |this| {
                            this.child(
                                div()
                                    .pl_3()
                                    .ml_1()
                                    .border_l_1()
                                    .border_color(rgb(palette.border))
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(editor_field(
                                        palette,
                                        "connection-editor-post-login-command",
                                        "Post-login command",
                                        editor.post_login_command.clone(),
                                        editor.focused_field
                                            == ConnectionEditorField::PostLoginCommand,
                                        cx.listener(|this, _, window, cx| {
                                            this.focus_connection_editor_field(
                                                ConnectionEditorField::PostLoginCommand,
                                                window,
                                                cx,
                                            );
                                        }),
                                    ))
                                    .child(editor_field(
                                        palette,
                                        "connection-editor-post-login-delay",
                                        "Delay (ms)",
                                        editor.post_login_delay_ms.clone(),
                                        editor.focused_field
                                            == ConnectionEditorField::PostLoginDelay,
                                        cx.listener(|this, _, window, cx| {
                                            this.focus_connection_editor_field(
                                                ConnectionEditorField::PostLoginDelay,
                                                window,
                                                cx,
                                            );
                                        }),
                                    )),
                            )
                        })
                    })
                    .when(editor.kind == ConnectionKindTab::Local, |this| {
                        let shell_path = editor.shell_path.clone();
                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child("Shell path"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .gap_1()
                                        .child(kind_chip(
                                            palette,
                                            "Bash",
                                            shell_path == "bash"
                                                || shell_path.ends_with("/bash")
                                                || shell_path.ends_with("\\bash"),
                                            cx.listener(|this, _, _, cx| {
                                                this.set_connection_editor_shell_path("bash", cx);
                                            }),
                                        ))
                                        .child(kind_chip(
                                            palette,
                                            "Zsh",
                                            shell_path == "zsh"
                                                || shell_path.ends_with("/zsh")
                                                || shell_path.ends_with("\\zsh"),
                                            cx.listener(|this, _, _, cx| {
                                                this.set_connection_editor_shell_path("zsh", cx);
                                            }),
                                        ))
                                        .child(kind_chip(
                                            palette,
                                            "Fish",
                                            shell_path == "fish"
                                                || shell_path.ends_with("/fish")
                                                || shell_path.ends_with("\\fish"),
                                            cx.listener(|this, _, _, cx| {
                                                this.set_connection_editor_shell_path("fish", cx);
                                            }),
                                        ))
                                        .child(kind_chip(
                                            palette,
                                            "sh",
                                            shell_path == "sh"
                                                || shell_path.ends_with("/sh")
                                                || shell_path.ends_with("\\sh"),
                                            cx.listener(|this, _, _, cx| {
                                                this.set_connection_editor_shell_path("sh", cx);
                                            }),
                                        ))
                                        .child(kind_chip(
                                            palette,
                                            "PowerShell",
                                            shell_path == "powershell.exe"
                                                || shell_path.ends_with("powershell.exe")
                                                || shell_path.ends_with("pwsh")
                                                || shell_path.ends_with("pwsh.exe"),
                                            cx.listener(|this, _, _, cx| {
                                                this.set_connection_editor_shell_path(
                                                    "powershell.exe",
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(kind_chip(
                                            palette,
                                            "CMD",
                                            shell_path == "cmd.exe"
                                                || shell_path.ends_with("cmd.exe"),
                                            cx.listener(|this, _, _, cx| {
                                                this.set_connection_editor_shell_path(
                                                    "cmd.exe",
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(kind_chip(
                                            palette,
                                            "WSL",
                                            shell_path == "wsl.exe"
                                                || shell_path.ends_with("wsl.exe"),
                                            cx.listener(|this, _, _, cx| {
                                                this.set_connection_editor_shell_path(
                                                    "wsl.exe",
                                                    cx,
                                                );
                                            }),
                                        )),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_end()
                                .gap_2()
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .child(editor_field(
                                            palette,
                                            "connection-editor-shell",
                                            "Shell",
                                            editor.shell_path.clone(),
                                            editor.focused_field
                                                == ConnectionEditorField::ShellPath,
                                            cx.listener(|this, _, window, cx| {
                                                this.focus_connection_editor_field(
                                                    ConnectionEditorField::ShellPath,
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        )),
                                )
                                .child(small_button(
                                    palette,
                                    "connection-editor-shell-browse",
                                    "Browse",
                                    cx.listener(|this, _, _, cx| {
                                        this.prompt_connection_editor_shell_path(cx);
                                    }),
                                )),
                        )
                        .child(editor_field(
                            palette,
                            "connection-editor-args",
                            "Args",
                            editor.shell_args.clone(),
                            editor.focused_field == ConnectionEditorField::ShellArgs,
                            cx.listener(|this, _, window, cx| {
                                this.focus_connection_editor_field(
                                    ConnectionEditorField::ShellArgs,
                                    window,
                                    cx,
                                );
                            }),
                        ))
                        .child(
                            div()
                                .flex()
                                .items_end()
                                .gap_2()
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .child(editor_field(
                                            palette,
                                            "connection-editor-cwd",
                                            "Working Dir",
                                            editor.working_dir.clone(),
                                            editor.focused_field
                                                == ConnectionEditorField::WorkingDir,
                                            cx.listener(|this, _, window, cx| {
                                                this.focus_connection_editor_field(
                                                    ConnectionEditorField::WorkingDir,
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        )),
                                )
                                .child(small_button(
                                    palette,
                                    "connection-editor-cwd-browse",
                                    "Browse",
                                    cx.listener(|this, _, _, cx| {
                                        this.prompt_connection_editor_working_dir(cx);
                                    }),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(toggle_chip(
                                    palette,
                                    "Open After Save",
                                    editor.connect_after_save,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_connection_editor_flag(
                                            ConnectionEditorToggle::ConnectAfterSave,
                                            cx,
                                        );
                                    }),
                                )),
                        )
                    })
.when(editor.kind == ConnectionKindTab::Telnet, |this| {
                        this.child(
                            div()
                                .grid()
                                .grid_cols(2)
                                .gap_2()
                                .child(editor_field(
                                    palette,
                                    "connection-editor-telnet-host",
                                    "Host",
                                    editor.host.clone(),
                                    editor.focused_field == ConnectionEditorField::Host,
                                    cx.listener(|this, _, window, cx| {
                                        this.focus_connection_editor_field(
                                            ConnectionEditorField::Host,
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(editor_field(
                                    palette,
                                    "connection-editor-telnet-port",
                                    "Port",
                                    editor.port.clone(),
                                    editor.focused_field == ConnectionEditorField::Port,
                                    cx.listener(|this, _, window, cx| {
                                        this.focus_connection_editor_field(
                                            ConnectionEditorField::Port,
                                            window,
                                            cx,
                                        );
                                    }),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(format!(
                                            "Backspace · {}",
                                            match editor.backspace_mode.as_str() {
                                                "ctrl-h" | "bs" | "ctrl_h" => "Ctrl+H (BS)",
                                                _ => "DEL (0x7F)",
                                            }
                                        )),
                                )
                                .child(small_button(palette,
                                    "connection-editor-telnet-backspace",
                                    "Cycle",
                                    cx.listener(|this, _, _, cx| {
                                        this.cycle_connection_editor_backspace(cx);
                                    }),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(toggle_chip(
                                    palette,
                                    "Raw TCP",
                                    editor.raw_tcp_cli,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_connection_editor_flag(
                                            ConnectionEditorToggle::RawTcp,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(toggle_chip(
                                    palette,
                                    "Local Echo",
                                    editor.local_echo,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_connection_editor_flag(
                                            ConnectionEditorToggle::LocalEcho,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(toggle_chip(
                                    palette,
                                    "Open After Save",
                                    editor.connect_after_save,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_connection_editor_flag(
                                            ConnectionEditorToggle::ConnectAfterSave,
                                            cx,
                                        );
                                    }),
                                )),
                        )
                    })
                    .when(editor.kind == ConnectionKindTab::Serial, |this| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(format!(
                                            "Port · {}",
                                            if editor.serial_port.is_empty() {
                                                "Select port"
                                            } else {
                                                &editor.serial_port
                                            }
                                        )),
                                )
                                .child(small_button(palette, 
                                    "connection-editor-serial-port",
                                    "Cycle",
                                    cx.listener(|this, _, _, cx| {
                                        this.cycle_connection_editor_serial_port(cx);
                                    }),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(editor_field(
                                    palette,
                                    "connection-editor-baud",
                                    "Baud",
                                    editor.baud_rate.clone(),
                                    editor.focused_field == ConnectionEditorField::BaudRate,
                                    cx.listener(|this, _, window, cx| {
                                        this.focus_connection_editor_field(
                                            ConnectionEditorField::BaudRate,
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(small_button(palette,
                                    "connection-editor-baud-preset",
                                    "Preset",
                                    cx.listener(|this, _, _, cx| {
                                        this.cycle_connection_editor_baud_preset(cx);
                                    }),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(format!("Data bits · {}", editor.data_bits)),
                                )
                                .child(small_button(palette,
                                    "connection-editor-data-bits",
                                    "Cycle",
                                    cx.listener(|this, _, _, cx| {
                                        this.cycle_connection_editor_data_bits(cx);
                                    }),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(format!(
                                            "Parity · {}",
                                            editor.parity.to_ascii_uppercase()
                                        )),
                                )
                                .child(small_button(palette,
                                    "connection-editor-parity",
                                    "Cycle",
                                    cx.listener(|this, _, _, cx| {
                                        this.cycle_connection_editor_parity(cx);
                                    }),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(format!("Stop bits · {}", editor.stop_bits)),
                                )
                                .child(small_button(palette,
                                    "connection-editor-stop-bits",
                                    "Cycle",
                                    cx.listener(|this, _, _, cx| {
                                        this.cycle_connection_editor_stop_bits(cx);
                                    }),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(format!(
                                            "Backspace · {}",
                                            match editor.backspace_mode.as_str() {
                                                "ctrl-h" | "bs" | "ctrl_h" => "Ctrl+H (BS)",
                                                _ => "DEL (0x7F)",
                                            }
                                        )),
                                )
                                .child(small_button(palette,
                                    "connection-editor-serial-backspace",
                                    "Cycle",
                                    cx.listener(|this, _, _, cx| {
                                        this.cycle_connection_editor_backspace(cx);
                                    }),
                                )),
                        )
                    }),
            )
            .when_some(editor.error.clone(), |this, error| {
                this.child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette.danger))
                        .child(error),
                )
            })
            .child(
                div()
                    .mt_1()
                    .pt_3()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .child(small_button(palette, 
                        "connection-editor-close",
                        "Cancel",
                        cx.listener(|this, _, _, cx| {
                            this.close_connection_editor(cx);
                        }),
                    ))
                    .child(small_button(palette, 
                        "connection-editor-save",
                        save_label,
                        cx.listener(|this, _, window, cx| {
                            this.save_connection_editor(window, cx);
                        }),
                    )),
            );
        modal_dialog_shell(palette, "connection-editor-modal", 560., card)
    }

    fn connection_group_editor_panel(
        &mut self,
        editor: ConnectionGroupEditorState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = if editor.id.is_some() {
            "Edit Group"
        } else {
            "New Group"
        };
        let card = div()
            .id(SharedString::from("connection-group-editor-panel"))
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .track_focus(&self.connection_group_editor_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.connection_group_editor_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                this.handle_connection_group_editor_key_down(event, cx);
            }))
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.text))
                    .child(title),
            )
            .child(editor_field(
                palette,
                "connection-group-name",
                "Group Name",
                editor.name.clone(),
                true,
                cx.listener(|this, _, window, cx| {
                    window.focus(&this.connection_group_editor_focus);
                    cx.notify();
                }),
            ))
            .when_some(editor.error.clone(), |this, error| {
                this.child(div().text_size(px(12.)).text_color(rgb(palette.danger)).child(error))
            })
            .child(modal_dialog_footer(palette, 
                "connection-group-close",
                "connection-group-save",
                "Save",
                cx.listener(|this, _, _, cx| {
                    this.close_connection_group_editor(cx);
                }),
                cx.listener(|this, _, _, cx| {
                    this.save_connection_group_editor(cx);
                }),
            ));
        modal_dialog_shell(palette, "connection-group-editor-modal", 420., card)
    }
    fn connection_delete_confirm_panel(
        &mut self,
        confirm: ConnectionDeleteConfirmState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let card = div()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.danger))
                    .child("Delete Connection"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(palette.text))
                    .child(format!("Delete \"{}\"?", confirm.label)),
            )
            .child(modal_dialog_footer(palette, 
                "connection-delete-cancel",
                "connection-delete-confirm",
                "Delete",
                cx.listener(|this, _, _, cx| {
                    this.close_connection_delete_confirm(cx);
                }),
                cx.listener(|this, _, _, cx| {
                    this.confirm_connection_delete(cx);
                }),
            ));
        modal_dialog_shell(palette, "connection-delete-confirm-modal", 420., card)
    }
    fn connection_group_delete_confirm_panel(
        &mut self,
        confirm: ConnectionGroupDeleteConfirmState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let card = div()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.danger))
                    .child("Delete Group"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(palette.text))
                    .child(format!(
                        "Delete \"{}\" ({} connections, {} child groups)?",
                        confirm.label, confirm.connection_count, confirm.child_group_count
                    )),
            )
            .child(modal_dialog_footer(palette, 
                "connection-group-delete-cancel",
                "connection-group-delete-confirm",
                "Delete",
                cx.listener(|this, _, _, cx| {
                    this.close_connection_group_delete_confirm(cx);
                }),
                cx.listener(|this, _, _, cx| {
                    this.confirm_connection_group_delete(cx);
                }),
            ));
        modal_dialog_shell(palette, "connection-group-delete-modal", 440., card)
    }
    fn connection_context_menu_overlay(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let state = self.connection_context_menu.clone().unwrap_or(ConnectionContextMenuState {
            connection_id: String::new(),
            x: px(24.),
            y: px(24.),
        });
        let connection = self
            .connections
            .iter()
            .find(|connection| connection.id == state.connection_id)
            .cloned();
        let selected_count = self.selected_connections().len();
        let connect_label = if selected_count > 1
            && connection
                .as_ref()
                .is_some_and(|conn| self.selected_connection_ids.contains(&conn.id)) {
            format!("Connect selected ({selected_count})")
        } else {
            "Connect".to_string()
        };
        let connection_id = state.connection_id.clone();
        let connection_for_connect = connection.clone();
        let connection_for_edit = connection_id.clone();
        let connection_for_rename = connection_id.clone();
        let connection_for_copy = connection_id.clone();
        let connection_for_delete = connection_id.clone();

        div()
            .id(SharedString::from("connection-context-menu-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_connection_context_menus(cx);
            }))
            .child(
                div()
                    .id(SharedString::from("connection-context-menu"))
                    .absolute()
                    .top(state.y)
                    .left(state.x)
                    .w(px(180.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .py_1()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(menu_item_owned(
                        palette,
                        "connection-context-connect",
                        connect_label,
                        cx.listener(move |this, _, window, cx| {
                            this.close_connection_context_menus(cx);
                            let selected = this.selected_connections();
                            if selected.len() > 1
                                && selected.iter().any(|conn| conn.id == connection_id) {
                                this.start_selected_saved_connections(window, cx);
                            } else if let Some(connection) = connection_for_connect.clone() {
                                this.start_saved_connection(connection, window, cx);
                            }
                        }),
                    ))
                    .child(menu_item(
                        palette,
                        "connection-context-edit",
                        "Edit",
                        cx.listener(move |this, _, window, cx| {
                            this.close_connection_context_menus(cx);
                            this.open_connection_editor(
                                Some(connection_for_edit.clone()),
                                None,
                                false,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(menu_separator(palette))
                    .child(menu_item(
                        palette,
                        "connection-context-rename",
                        "Rename",
                        cx.listener(move |this, _, window, cx| {
                            this.close_connection_context_menus(cx);
                            this.rename_connection(connection_for_rename.clone(), window, cx);
                        }),
                    ))
                    .child(menu_item(
                        palette,
                        "connection-context-copy",
                        if selected_count > 1 { "Copy selected" } else { "Copy" },
                        cx.listener(move |this, _, _, cx| {
                            this.close_connection_context_menus(cx);
                            if this.selected_connections().len() > 1 {
                                this.copy_selected_connections(cx);
                            } else {
                                this.copy_connection_by_id(connection_for_copy.clone(), cx);
                            }
                        }),
                    ))
                    .child(menu_separator(palette))
                    .child(menu_item(
                        palette,
                        "connection-context-delete",
                        if selected_count > 1 { "Delete selected" } else { "Delete" },
                        cx.listener(move |this, _, _, cx| {
                            this.close_connection_context_menus(cx);
                            if this.selected_connections().len() > 1 {
                                this.delete_selected_connections(cx);
                            } else {
                                this.open_connection_delete_confirm(
                                    connection_for_delete.clone(),
                                    cx,
                                );
                            }
                        }),
                    )),
            )
    }

    fn connection_group_context_menu_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let state = self
            .connection_group_context_menu
            .clone()
            .unwrap_or(ConnectionGroupContextMenuState {
                group_id: String::new(),
                x: px(24.),
                y: px(24.),
            });
        let group_id = state.group_id.clone();
        let group_id_new = group_id.clone();
        let group_id_folder = group_id.clone();
        let group_id_open = group_id.clone();
        let group_id_edit = group_id.clone();
        let group_id_delete = group_id.clone();
        let total_in_group = {
            let mut group_ids = std::collections::HashSet::from([group_id.clone()]);
            let mut changed = true;
            while changed {
                changed = false;
                for group in &self.connection_groups {
                    if let Some(parent) = group.parent_id.as_ref() {
                        if group_ids.contains(parent) && group_ids.insert(group.id.clone()) {
                            changed = true;
                        }
                    }
                }
            }
            self.connections
                .iter()
                .filter(|connection| {
                    connection
                        .group_id
                        .as_ref()
                        .is_some_and(|id| group_ids.contains(id))
                })
                .count()
        };

        div()
            .id(SharedString::from("connection-group-context-menu-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_connection_context_menus(cx);
            }))
            .child(
                div()
                    .id(SharedString::from("connection-group-context-menu"))
                    .absolute()
                    .top(state.y)
                    .left(state.x)
                    .w(px(180.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .py_1()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(menu_item(
                        palette,
                        "connection-group-context-new",
                        "New connection",
                        cx.listener(move |this, _, window, cx| {
                            this.close_connection_context_menus(cx);
                            this.open_connection_editor(
                                None,
                                Some(group_id_new.clone()),
                                false,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(menu_item(
                        palette,
                        "connection-group-context-folder",
                        "New folder",
                        cx.listener(move |this, _, window, cx| {
                            this.close_connection_context_menus(cx);
                            this.open_connection_group_editor(
                                None,
                                Some(group_id_folder.clone()),
                                window,
                                cx,
                            );
                        }),
                    ))
                    .when(total_in_group > 0, |this| {
                        this.child(menu_separator(palette)).child(menu_item(
                            palette,
                            "connection-group-context-open-all",
                            "Open all",
                            cx.listener(move |this, _, window, cx| {
                                this.close_connection_context_menus(cx);
                                this.start_group_connections(group_id_open.clone(), window, cx);
                            }),
                        ))
                    })
                    .child(menu_separator(palette))
                    .child(menu_item(
                        palette,
                        "connection-group-context-rename",
                        "Rename",
                        cx.listener(move |this, _, window, cx| {
                            this.close_connection_context_menus(cx);
                            this.open_connection_group_editor(
                                Some(group_id_edit.clone()),
                                None,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(menu_item(
                        palette,
                        "connection-group-context-delete",
                        "Delete",
                        cx.listener(move |this, _, _, cx| {
                            this.close_connection_context_menus(cx);
                            this.open_connection_group_delete_confirm(group_id_delete.clone(), cx);
                        }),
                    )),
            )
    }

}

#[derive(Clone)]
enum ConnectionListRow {
    Separator,
    GroupHeader(ConnectionSection),
    EmptyGroup,
    Connection {
        connection: SavedConnection,
        indented: bool,
    },
}

impl ConnectionListRow {
    fn height_px(&self) -> f32 {
        match self {
            Self::Separator => 10.,
            Self::GroupHeader(_) | Self::EmptyGroup => 28.,
            Self::Connection { .. } => 34.,
        }
    }
}

fn flatten_connection_rows(
    sections: &[ConnectionSection],
    expanded_groups: &std::collections::HashSet<String>,
) -> Vec<ConnectionListRow> {
    let has_groups = sections.iter().any(|section| !section.is_root);
    let mut rows = Vec::new();
    for section in sections {
        if section.is_root {
            if has_groups && !section.connections.is_empty() {
                rows.push(ConnectionListRow::Separator);
            }
            for connection in &section.connections {
                rows.push(ConnectionListRow::Connection {
                    connection: connection.clone(),
                    indented: false,
                });
            }
            continue;
        }

        rows.push(ConnectionListRow::GroupHeader(section.clone()));
        let expanded = section
            .group_id
            .as_ref()
            .map(|id| expanded_groups.contains(id))
            .unwrap_or(true);
        if !expanded {
            continue;
        }
        if section.connections.is_empty() {
            rows.push(ConnectionListRow::EmptyGroup);
            continue;
        }
        for connection in &section.connections {
            rows.push(ConnectionListRow::Connection {
                connection: connection.clone(),
                indented: true,
            });
        }
    }
    rows
}

#[derive(Clone)]
struct ConnectionSection {
    group_id: Option<String>,
    label: String,
    is_root: bool,
    connections: Vec<SavedConnection>,
}

fn connection_sections(
    connections: &[SavedConnection],
    groups: &[Group],
    query: &str,
    sort_mode: ConnectionSortMode,
) -> Vec<ConnectionSection> {
    let mut by_group: HashMap<Option<String>, Vec<SavedConnection>> = HashMap::new();
    for connection in connections {
        if !connection_matches(connection, query) {
            continue;
        }
        by_group
            .entry(connection.group_id.clone())
            .or_default()
            .push(connection.clone());
    }
    for list in by_group.values_mut() {
        sort_connections(list, sort_mode);
    }

    let mut sections = Vec::new();
    let mut ordered_groups = groups.to_vec();
    ordered_groups.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then_with(|| left.name.to_ascii_lowercase().cmp(&right.name.to_ascii_lowercase()))
    });
    for group in ordered_groups {
        let connections = by_group.remove(&Some(group.id.clone())).unwrap_or_default();
        if !query.is_empty() && connections.is_empty() {
            continue;
        }
        sections.push(ConnectionSection {
            group_id: Some(group.id),
            label: group.name,
            is_root: false,
            connections,
        });
    }
    let root = by_group.remove(&None).unwrap_or_default();
    // Tauri: folders first, then ungrouped connections (no "Ungrouped" header).
    if !root.is_empty() || sections.is_empty() {
        sections.push(ConnectionSection {
            group_id: None,
            label: "Ungrouped".to_string(),
            is_root: true,
            connections: root,
        });
    }
    sections
}

fn connection_matches(connection: &SavedConnection, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let haystack = format!(
        "{} {} {} {} {}",
        connection.name,
        connection.endpoint(),
        connection.kind_label(),
        connection.description.clone().unwrap_or_default(),
        connection.id
    )
    .to_ascii_lowercase();
    haystack.contains(query)
}

fn sort_connections(connections: &mut [SavedConnection], mode: ConnectionSortMode) {
    match mode {
        ConnectionSortMode::Default => connections.sort_by(|left, right| {
            left.sort_order
                .cmp(&right.sort_order)
                .then_with(|| left.name.to_ascii_lowercase().cmp(&right.name.to_ascii_lowercase()))
        }),
        ConnectionSortMode::NameAsc => connections
            .sort_by(|left, right| left.name.to_ascii_lowercase().cmp(&right.name.to_ascii_lowercase())),
        ConnectionSortMode::NameDesc => connections
            .sort_by(|left, right| right.name.to_ascii_lowercase().cmp(&left.name.to_ascii_lowercase())),
        ConnectionSortMode::Recent => connections.sort_by(|left, right| {
            right
                .last_used_at_ms
                .unwrap_or(0)
                .cmp(&left.last_used_at_ms.unwrap_or(0))
                .then_with(|| left.name.to_ascii_lowercase().cmp(&right.name.to_ascii_lowercase()))
        }),
    }
}

fn kind_chip(palette: crate::ui::theme::ThemePalette,
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,) -> impl IntoElement {    div()
        .id(SharedString::from(format!("connection-kind-{label}")))
        .h(px(24.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .text_xs()
        .font_weight(FontWeight(700.))
        .cursor_pointer()
        .text_color(if selected {
            rgb(0xffffff)
        } else {
            rgb(palette.text_muted)
        })
        .bg(if selected {
            rgb(palette.success)
        } else {
            rgb(palette.surface_elevated)
        })
        .hover(|this| this.bg(rgb(palette.border)))
        .child(label)
        .on_click(on_click)
}

fn toggle_chip(palette: crate::ui::theme::ThemePalette,
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,) -> impl IntoElement {    div()
        .id(SharedString::from(format!("connection-toggle-{label}")))
        .h(px(22.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .text_size(px(10.))
        .font_weight(FontWeight(700.))
        .cursor_pointer()
        .text_color(if selected {
            rgb(palette.success)
        } else {
            rgb(palette.text_muted)
        })
        .bg(if selected {
            rgb(0x12261a)
        } else {
            rgb(palette.surface_elevated)
        })
        .hover(|this| this.bg(rgb(palette.border)))
        .child(label)
        .on_click(on_click)
}

fn editor_field(palette: crate::ui::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    value: String,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,) -> impl IntoElement {
    transfer_input(id, label, value, active, palette).on_click(on_click)
}

fn icon_action_button(palette: crate::ui::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,) -> impl IntoElement {    // label may be a glyph fallback or an icons/*.svg path.
    let is_svg = label.starts_with("icons/") && label.ends_with(".svg");
    div()
        .id(SharedString::from(id.into()))
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_size(px(11.))
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)).text_color(rgb(palette.text)))
        .on_click(on_click)
        .when(is_svg, |this| {
            this.child(
                svg()
                    .size(px(14.))
                    .flex_none()
                    .path(label),
            )
        })
        .when(!is_svg, |this| this.child(label))
}

fn menu_separator(palette: crate::ui::theme::ThemePalette) -> impl IntoElement {    div()
        .h(px(1.))
        .mx_2()
        .my_1()
        .bg(rgb(palette.border))
}

fn menu_item(palette: crate::ui::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,) -> impl IntoElement {    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .text_size(px(12.))
        .text_color(rgb(palette.text))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)))
        .on_click(on_click)
        .child(label)
}

fn menu_item_owned(palette: crate::ui::theme::ThemePalette,
    id: impl Into<String>,
    label: String,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,) -> impl IntoElement {    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .text_size(px(12.))
        .text_color(rgb(palette.text))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)))
        .on_click(on_click)
        .child(label)
}


fn connection_detail_rows(
    connection: &SavedConnection,
    all_connections: &[SavedConnection],
    proxies: &[ProxyConfig],
) -> Vec<(&'static str, String)> {
    let description = connection
        .description
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("—")
        .to_string();
    let mut rows = vec![
        ("Type", connection.kind_label().to_string()),
        ("Name", connection.name.clone()),
    ];
    match &connection.config {
        nyaterm_domain::ConnectionType::Ssh {
            host,
            port,
            username,
            backspace_mode,
            x11_forwarding,
            ..
        } => {
            rows.push(("Host", host.clone()));
            rows.push(("Port", port.to_string()));
            rows.push(("User", username.clone()));
            rows.push((
                "BS",
                match backspace_mode.as_str() {
                    "ctrl-h" | "bs" | "ctrl_h" => "Ctrl+H".to_string(),
                    _ => "DEL".to_string(),
                },
            ));
            if *x11_forwarding {
                rows.push(("X11", "on".to_string()));
            }
            if let Some(network) = connection.network.as_ref() {
                if let Some(proxy_id) = network.proxy_id.as_deref() {
                    let proxy_label = proxies
                        .iter()
                        .find(|proxy| proxy.id == proxy_id)
                        .map(|proxy| proxy.name.clone())
                        .unwrap_or_else(|| truncate_preview(proxy_id, 16));
                    rows.push(("Proxy", proxy_label));
                }
                if network.proxy_jump_id.is_some() {
                    let chain = format_jump_host_chain(connection, all_connections);
                    rows.push(("Jump", chain));
                }
            }
        }
        nyaterm_domain::ConnectionType::LocalTerminal {
            shell_path,
            shell_args,
            working_dir,
            ..
        } => {
            rows.push((
                "Shell",
                if shell_path.trim().is_empty() {
                    "system".to_string()
                } else {
                    shell_path.clone()
                },
            ));
            if !shell_args.trim().is_empty() {
                rows.push(("Args", truncate_preview(shell_args, 28)));
            }
            rows.push((
                "CWD",
                working_dir
                    .clone()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "—".to_string()),
            ));
        }
        nyaterm_domain::ConnectionType::Telnet {
            host,
            port,
            backspace_mode,
            raw_tcp_cli,
            local_echo,
            ..
        } => {
            rows.push(("Host", host.clone()));
            rows.push(("Port", port.to_string()));
            rows.push((
                "BS",
                match backspace_mode.as_str() {
                    "ctrl-h" | "bs" | "ctrl_h" => "Ctrl+H".to_string(),
                    _ => "DEL".to_string(),
                },
            ));
            if *raw_tcp_cli {
                rows.push(("Mode", "raw tcp".to_string()));
            }
            if *local_echo {
                rows.push(("Echo", "local".to_string()));
            }
        }
        nyaterm_domain::ConnectionType::Serial {
            port_name,
            baud_rate,
            data_bits,
            parity,
            stop_bits,
            backspace_mode,
            ..
        } => {
            rows.push(("Port", port_name.clone()));
            rows.push(("Baud", baud_rate.to_string()));
            rows.push(("Data", data_bits.to_string()));
            rows.push(("Parity", parity.clone()));
            rows.push(("Stop", stop_bits.clone()));
            rows.push((
                "BS",
                match backspace_mode.as_str() {
                    "ctrl-h" | "bs" | "ctrl_h" => "Ctrl+H".to_string(),
                    _ => "DEL".to_string(),
                },
            ));
        }
    }
    rows.push(("Last", format_last_used_ms(connection.last_used_at_ms)));
    rows.push(("Desc", description));
    rows
}

fn format_jump_host_chain(
    connection: &SavedConnection,
    all_connections: &[SavedConnection],
) -> String {
    let Some(mut jump_id) = connection
        .network
        .as_ref()
        .and_then(|network| network.proxy_jump_id.clone())
    else {
        return "—".to_string();
    };
    let by_id: std::collections::HashMap<&str, &SavedConnection> = all_connections
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect();
    let mut seen = std::collections::HashSet::new();
    seen.insert(connection.id.clone());
    let mut labels = Vec::new();
    loop {
        if !seen.insert(jump_id.clone()) {
            labels.push("↺ cycle".to_string());
            break;
        }
        let Some(jump) = by_id.get(jump_id.as_str()) else {
            labels.push(format!("missing:{jump_id}"));
            break;
        };
        labels.push(jump.name.clone());
        match jump
            .network
            .as_ref()
            .and_then(|network| network.proxy_jump_id.clone())
        {
            Some(next) => jump_id = next,
            None => break,
        }
    }
    if labels.is_empty() {
        "—".to_string()
    } else {
        labels.join(" → ")
    }
}
