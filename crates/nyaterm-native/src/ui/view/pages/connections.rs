use gpui::{
    Context, FontWeight, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, SharedString, div,
    prelude::*, px, rgb,
};
use nyaterm_domain::{Group, SavedConnection, truncate_preview};
use std::collections::HashMap;

use crate::ui::components::small_button;
use crate::ui::models::{
    ConnectionContextMenuState, ConnectionDeleteConfirmState, ConnectionEditorField,
    ConnectionEditorState, ConnectionGroupContextMenuState, ConnectionGroupDeleteConfirmState,
    ConnectionGroupEditorState, ConnectionKindTab, ConnectionSortMode,
};

use super::super::{
    ConnectionDragKind, ConnectionDragPayload, ConnectionDragPreview, ConnectionDropPosition,
    ConnectionDropTarget, ConnectionEditorToggle, NyaTermApp, format_last_used_ms, transfer_input,
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

        let mut list = div()
            .id(SharedString::from("connections-list-scroll"))
            .flex_1()
            .min_h_0()
            .overflow_scroll()
            .scrollbar_width(px(6.))
            .flex()
            .flex_col()
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
                            .text_color(rgb(0x8b949e))
                            .child("No saved connections"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x6e7681))
                            .child("Create one or open a temporary SSH link."),
                    ),
            );
        } else if visible_count == 0 {
            list = list.child(
                div()
                    .px_4()
                    .py_8()
                    .text_size(px(11.))
                    .text_color(rgb(0x6e7681))
                    .child("No connections match the current search."),
            );
        } else {
            for section in sections {
                list = list.child(self.connection_section(section, cx));
            }
        }

        // Tauri: PanelHeader (shared stack) + search/action strip + flat tree list.
        // Count is shown in the shared panel header via meta; strip hosts search + icons.
        let _ = selected_count;
        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(rgb(0x161b22))
            .child(self.connections_search_bar(visible_count, cx))
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

        div()
            .h(px(36.))
            .px_2()
            .flex()
            .items_center()
            .gap_1()
            .border_b_1()
            .border_color(rgb(0x30363d))
            .bg(rgb(0x12171f))
            .child(
                div()
                    .id(SharedString::from("connection-search-input"))
                    .h(px(28.))
                    .flex_1()
                    .min_w_0()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x0d1117))
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
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x6e7681))
                            .child("⌕"),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(12.))
                            .text_color(if self.connection_search_draft.is_empty() {
                                rgb(0x6e7681)
                            } else {
                                rgb(0xc9d1d9)
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
                                .text_color(rgb(0x8b949e))
                                .cursor_pointer()
                                .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xc9d1d9)))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.connection_search_draft.clear();
                                    window.focus(&this.connection_search_focus);
                                    cx.notify();
                                }))
                                .child("×"),
                        )
                    }),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(rgb(0x6e7681))
                    .child(visible_count.to_string()),
            )
            .child(icon_action_button(
                "connections-sort",
                sort_label,
                cx.listener(|this, _, _, cx| {
                    this.cycle_connection_sort_mode(cx);
                }),
            ))
            .child(icon_action_button(
                "connections-temp-ssh",
                "⚡",
                cx.listener(|this, _, window, cx| {
                    this.open_temporary_ssh_link_dialog(window, cx);
                }),
            ))
            .child(icon_action_button(
                "connections-new-group",
                "📁+",
                cx.listener(|this, _, window, cx| {
                    this.open_connection_group_editor(None, None, window, cx);
                }),
            ))
            .child(icon_action_button(
                "connections-new",
                "+",
                cx.listener(|this, _, window, cx| {
                    this.open_connection_editor(None, None, false, window, cx);
                }),
            ))
            .child(
                div()
                    .relative()
                    .child(icon_action_button(
                        "connections-more",
                        "⋮",
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
                                .border_color(rgb(0x30363d))
                                .bg(rgb(0x161b22))
                                .shadow_sm()
                                .py_1()
                                                                .child(menu_item(
                                    "connections-export",
                                    "Export config",
                                    cx.listener(|this, _, _, cx| {
                                        this.connections_more_menu_open = false;
                                        this.prompt_config_export(cx);
                                    }),
                                ))
                                .child(menu_item(
                                    "connections-import",
                                    "Import config",
                                    cx.listener(|this, _, _, cx| {
                                        this.connections_more_menu_open = false;
                                        this.prompt_config_import(cx);
                                    }),
                                ))
                                .child(menu_item(
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
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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

        if expanded {
            if section.connections.is_empty() && !section.is_root {
                body = body.child(
                    div()
                        .px_2()
                        .py_1()
                        .pl(px(28.))
                        .text_size(px(11.))
                        .text_color(rgb(0x6e7681))
                        .child("Empty group"),
                );
            } else {
                for connection in section.connections {
                    body = body.child(self.saved_connection_row(connection, !section.is_root, cx));
                }
            }
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
                            rgb(0x122033)
                        } else if section
                            .group_id
                            .as_ref()
                            .is_some_and(|id| {
                                self.hovered_connection_group_id.as_deref() == Some(id.as_str())
                            })
                        {
                            rgb(0x1c2128)
                        } else {
                            rgb(0x161b22)
                        }
                    })
                    .when(
                        self.connection_drop_target.as_ref().is_some_and(|target| {
                            target.kind == ConnectionDragKind::Group
                                && target.position == ConnectionDropPosition::Inside
                                && target.id.as_deref() == section.group_id.as_deref()
                        }),
                        |this| this.border_1().border_color(rgb(0x388bfd)),
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
                                    .text_color(rgb(0x8b949e))
                                    .child(if section.is_root {
                                        "·".to_string()
                                    } else if expanded {
                                        "▾".to_string()
                                    } else {
                                        "▸".to_string()
                                    }),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(0xc9d1d9))
                                    .child(truncate_preview(&group_label, 28)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x6e7681))
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
                                .child(icon_action_button(
                                    format!(
                                        "connection-group-edit-{}",
                                        group_id_for_edit.clone().unwrap_or_default()
                                    ),
                                    "✎",
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
                                    format!(
                                        "connection-group-delete-{}",
                                        group_id_for_delete.clone().unwrap_or_default()
                                    ),
                                    "🗑",
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
        let kind_glyph = match kind {
            "SSH" => "⌂",
            "Local" => "▣",
            "Telnet" => "⇄",
            _ => "☰",
        };
        let show_actions = hovered || selected;
        let endpoint = connection.endpoint();
        let last_used = format_last_used_ms(connection.last_used_at_ms);
        let drop_target = self.connection_drop_target.as_ref().filter(|target| {
            target.kind == ConnectionDragKind::Connection
                && target.id.as_deref() == Some(connection.id.as_str())
        });
        let show_before = drop_target.is_some_and(|t| t.position == ConnectionDropPosition::Before);
        let show_after = drop_target.is_some_and(|t| t.position == ConnectionDropPosition::After);
        let show_inside = drop_target.is_some_and(|t| t.position == ConnectionDropPosition::Inside);
        let row_id = connection.id.clone();

        div()
            .id(SharedString::from(format!("connection-row-{}", connection.id)))
            .relative()
            .h(px(44.))
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .pl(if indented { px(24.) } else { px(8.) })
            .bg(if selected {
                rgb(0x122033)
            } else if show_inside {
                rgb(0x122033)
            } else if hovered {
                rgb(0x1c2128)
            } else {
                rgb(0x161b22)
            })
            .when(show_inside, |this| this.border_1().border_color(rgb(0x388bfd)))
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
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.open_connection_context_menu(menu_id.clone(), event, cx);
                }),
            )
            .on_click(cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                if event.click_count() >= 2 {
                    this.start_saved_connection(connect_connection_dbl.clone(), window, cx);
                    return;
                }
                this.toggle_connection_selected(select_id.clone(), cx);
            }))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(match kind {
                        "SSH" => rgb(0x3fb950),
                        "Local" => rgb(0x58a6ff),
                        "Telnet" => rgb(0xd29922),
                        _ => rgb(0xbc8cff),
                    })
                    .child(kind_glyph),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap_0()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight(600.))
                            .text_color(if selected {
                                rgb(0x58a6ff)
                            } else {
                                rgb(0xc9d1d9)
                            })
                            .overflow_hidden()
                            .child(truncate_preview(&connection.name, 36)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .font_family("JetBrains Mono")
                            .text_size(px(10.))
                            .text_color(rgb(0x6e7681))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .overflow_hidden()
                                    .child(truncate_preview(&endpoint, 42)),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .text_color(rgb(0x8b949e))
                                    .child(last_used),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_0()
                    .opacity(if show_actions { 1. } else { 0. })
                    .child(icon_action_button(
                        format!("connection-connect-{}", connection.id),
                        "↗",
                        cx.listener(move |this, _, window, cx| {
                            this.start_saved_connection(connect_connection.clone(), window, cx);
                        }),
                    ))
                    .child(icon_action_button(
                        format!("connection-edit-{}", connection.id),
                        "✎",
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
                        format!("connection-delete-{}", connection.id),
                        "🗑",
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
                        .bg(rgb(0x58a6ff)),
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
                        .bg(rgb(0x58a6ff)),
                )
            })
    }


    fn connection_details_tooltip(
        &self,
        connection_id: String,
        _cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(connection) = self
            .connections
            .iter()
            .find(|connection| connection.id == connection_id)
        else {
            return div().into_any_element();
        };
        let rows = connection_detail_rows(connection);
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
                            .text_color(rgb(0x6e7681))
                            .child(label),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(11.))
                            .text_color(rgb(0xc9d1d9))
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
            .top(px(48.))
                        .w(px(220.))
            .rounded_md()
            .border_1()
            .border_color(rgb(0x30363d))
            .bg(rgb(0x161b22))
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
        let password_display = if editor.password.is_empty() {
            if editor.existing_password.is_some() {
                "•••••••• (saved)".to_string()
            } else {
                String::new()
            }
        } else {
            "•".repeat(editor.password.chars().count().min(24))
        };

        div()
            .id(SharedString::from("connection-editor-panel"))
            .border_t_1()
            .border_color(rgb(0x30363d))
            .bg(rgb(0x0d1117))
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .max_h(px(420.))
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
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0xc9d1d9))
                            .child(title),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                "connection-editor-save",
                                if editor.connect_after_save {
                                    "Save+Open"
                                } else {
                                    "Save"
                                },
                                cx.listener(|this, _, window, cx| {
                                    this.save_connection_editor(window, cx);
                                }),
                            ))
                            .child(small_button(
                                "connection-editor-close",
                                "Close",
                                cx.listener(|this, _, _, cx| {
                                    this.close_connection_editor(cx);
                                }),
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(kind_chip(
                        "SSH",
                        editor.kind == ConnectionKindTab::Ssh,
                        cx.listener(|this, _, _, cx| {
                            this.set_connection_editor_kind(ConnectionKindTab::Ssh, cx);
                        }),
                    ))
                    .child(kind_chip(
                        "Local",
                        editor.kind == ConnectionKindTab::Local,
                        cx.listener(|this, _, _, cx| {
                            this.set_connection_editor_kind(ConnectionKindTab::Local, cx);
                        }),
                    ))
                    .child(kind_chip(
                        "Telnet",
                        editor.kind == ConnectionKindTab::Telnet,
                        cx.listener(|this, _, _, cx| {
                            this.set_connection_editor_kind(ConnectionKindTab::Telnet, cx);
                        }),
                    ))
                    .child(kind_chip(
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
                        "connection-editor-name",
                        "Name",
                        editor.name.clone(),
                        editor.focused_field == ConnectionEditorField::Name,
                        cx.listener(|this, _, window, cx| {
                            this.focus_connection_editor_field(ConnectionEditorField::Name, window, cx);
                        }),
                    ))
                    .child(editor_field(
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
                                    .text_color(rgb(0x8b949e))
                                    .child(format!("Group · {group_label}")),
                            )
                            .child(small_button(
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
                                        .text_color(rgb(0x8b949e))
                                        .child(format!("Auth · {}", editor.auth_mode)),
                                )
                                .child(small_button(
                                    "connection-editor-auth",
                                    "Cycle",
                                    cx.listener(|this, _, _, cx| {
                                        this.cycle_connection_editor_auth_mode(cx);
                                    }),
                                )),
                        )
                        .when(editor.auth_mode == "password", |this| {
                            this.child(editor_field(
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
                                            .text_color(rgb(0x8b949e))
                                            .child(format!("Key · {}", truncate_preview(&key_label, 24))),
                                    )
                                    .child(small_button(
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
                                        .text_color(rgb(0x8b949e))
                                        .child(format!("OTP · {}", truncate_preview(&otp_label, 24))),
                                )
                                .child(small_button(
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
                                .gap_1()
                                .child(toggle_chip(
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
                    .when(editor.kind == ConnectionKindTab::Local, |this| {
                        this.child(editor_field(
                            "connection-editor-shell",
                            "Shell",
                            editor.shell_path.clone(),
                            editor.focused_field == ConnectionEditorField::ShellPath,
                            cx.listener(|this, _, window, cx| {
                                this.focus_connection_editor_field(
                                    ConnectionEditorField::ShellPath,
                                    window,
                                    cx,
                                );
                            }),
                        ))
                        .child(editor_field(
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
                        .child(editor_field(
                            "connection-editor-cwd",
                            "Working Dir",
                            editor.working_dir.clone(),
                            editor.focused_field == ConnectionEditorField::WorkingDir,
                            cx.listener(|this, _, window, cx| {
                                this.focus_connection_editor_field(
                                    ConnectionEditorField::WorkingDir,
                                    window,
                                    cx,
                                );
                            }),
                        ))
                    })
                    .when(editor.kind == ConnectionKindTab::Telnet, |this| {
                        this.child(
                            div()
                                .grid()
                                .grid_cols(2)
                                .gap_2()
                                .child(editor_field(
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
                                .gap_1()
                                .child(toggle_chip(
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
                                    "Local Echo",
                                    editor.local_echo,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_connection_editor_flag(
                                            ConnectionEditorToggle::LocalEcho,
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
                                        .text_color(rgb(0x8b949e))
                                        .child(format!(
                                            "Port · {}",
                                            if editor.serial_port.is_empty() {
                                                "Select port"
                                            } else {
                                                &editor.serial_port
                                            }
                                        )),
                                )
                                .child(small_button(
                                    "connection-editor-serial-port",
                                    "Cycle",
                                    cx.listener(|this, _, _, cx| {
                                        this.cycle_connection_editor_serial_port(cx);
                                    }),
                                )),
                        )
                        .child(editor_field(
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
                    }),
            )
            .when_some(editor.error.clone(), |this, error| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(rgb(0xff7b72))
                        .child(error),
                )
            })
    }

    fn connection_group_editor_panel(
        &mut self,
        editor: ConnectionGroupEditorState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let title = if editor.id.is_some() {
            "Edit Group"
        } else {
            "New Group"
        };
        div()
            .id(SharedString::from("connection-group-editor-panel"))
            .border_t_1()
            .border_color(rgb(0x30363d))
            .bg(rgb(0x0d1117))
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
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
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0xc9d1d9))
                            .child(title),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                "connection-group-save",
                                "Save",
                                cx.listener(|this, _, _, cx| {
                                    this.save_connection_group_editor(cx);
                                }),
                            ))
                            .child(small_button(
                                "connection-group-close",
                                "Close",
                                cx.listener(|this, _, _, cx| {
                                    this.close_connection_group_editor(cx);
                                }),
                            )),
                    ),
            )
            .child(editor_field(
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
                this.child(div().text_xs().text_color(rgb(0xff7b72)).child(error))
            })
    }

    fn connection_delete_confirm_panel(
        &mut self,
        confirm: ConnectionDeleteConfirmState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .border_t_1()
            .border_color(rgb(0x30363d))
            .bg(rgb(0x1c0f12))
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight(800.))
                    .text_color(rgb(0xff7b72))
                    .child("Delete Connection"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0xc9d1d9))
                    .child(format!("Delete \"{}\"?", confirm.label)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(small_button(
                        "connection-delete-confirm",
                        "Delete",
                        cx.listener(|this, _, _, cx| {
                            this.confirm_connection_delete(cx);
                        }),
                    ))
                    .child(small_button(
                        "connection-delete-cancel",
                        "Cancel",
                        cx.listener(|this, _, _, cx| {
                            this.close_connection_delete_confirm(cx);
                        }),
                    )),
            )
    }

    fn connection_group_delete_confirm_panel(
        &mut self,
        confirm: ConnectionGroupDeleteConfirmState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .border_t_1()
            .border_color(rgb(0x30363d))
            .bg(rgb(0x1c0f12))
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight(800.))
                    .text_color(rgb(0xff7b72))
                    .child("Delete Group"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0xc9d1d9))
                    .child(format!(
                        "Delete \"{}\" ({} connections, {} child groups)?",
                        confirm.label, confirm.connection_count, confirm.child_group_count
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(small_button(
                        "connection-group-delete-confirm",
                        "Delete",
                        cx.listener(|this, _, _, cx| {
                            this.confirm_connection_group_delete(cx);
                        }),
                    ))
                    .child(small_button(
                        "connection-group-delete-cancel",
                        "Cancel",
                        cx.listener(|this, _, _, cx| {
                            this.close_connection_group_delete_confirm(cx);
                        }),
                    )),
            )
    }

    fn connection_context_menu_overlay(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
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
                .is_some_and(|conn| self.selected_connection_ids.contains(&conn.id))
        {
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
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .py_1()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(menu_item_owned(
                        "connection-context-connect",
                        connect_label,
                        cx.listener(move |this, _, window, cx| {
                            this.close_connection_context_menus(cx);
                            let selected = this.selected_connections();
                            if selected.len() > 1
                                && selected.iter().any(|conn| conn.id == connection_id)
                            {
                                this.start_selected_saved_connections(window, cx);
                            } else if let Some(connection) = connection_for_connect.clone() {
                                this.start_saved_connection(connection, window, cx);
                            }
                        }),
                    ))
                    .child(menu_item(
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
                    .child(menu_separator())
                    .child(menu_item(
                        "connection-context-rename",
                        "Rename",
                        cx.listener(move |this, _, window, cx| {
                            this.close_connection_context_menus(cx);
                            this.rename_connection(connection_for_rename.clone(), window, cx);
                        }),
                    ))
                    .child(menu_item(
                        "connection-context-copy",
                        "Copy",
                        cx.listener(move |this, _, _, cx| {
                            this.close_connection_context_menus(cx);
                            this.copy_connection_by_id(connection_for_copy.clone(), cx);
                        }),
                    ))
                    .child(menu_separator())
                    .child(menu_item(
                        "connection-context-delete",
                        "Delete",
                        cx.listener(move |this, _, _, cx| {
                            this.close_connection_context_menus(cx);
                            this.open_connection_delete_confirm(connection_for_delete.clone(), cx);
                        }),
                    )),
            )
    }

    fn connection_group_context_menu_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .py_1()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(menu_item(
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
                        this.child(menu_separator()).child(menu_item(
                            "connection-group-context-open-all",
                            "Open all",
                            cx.listener(move |this, _, window, cx| {
                                this.close_connection_context_menus(cx);
                                this.start_group_connections(group_id_open.clone(), window, cx);
                            }),
                        ))
                    })
                    .child(menu_separator())
                    .child(menu_item(
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
    if !root.is_empty() || sections.is_empty() {
        sections.insert(
            0,
            ConnectionSection {
                group_id: None,
                label: "Ungrouped".to_string(),
                is_root: true,
                connections: root,
            },
        );
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

fn kind_chip(
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
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
            rgb(0x8b949e)
        })
        .bg(if selected {
            rgb(0x238636)
        } else {
            rgb(0x21262d)
        })
        .hover(|this| this.bg(rgb(0x30363d)))
        .child(label)
        .on_click(on_click)
}

fn toggle_chip(
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
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
            rgb(0x3fb950)
        } else {
            rgb(0x8b949e)
        })
        .bg(if selected {
            rgb(0x12261a)
        } else {
            rgb(0x21262d)
        })
        .hover(|this| this.bg(rgb(0x30363d)))
        .child(label)
        .on_click(on_click)
}

fn editor_field(
    id: impl Into<String>,
    label: &'static str,
    value: String,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    transfer_input(id, label, value, active).on_click(on_click)
}

fn icon_action_button(
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_size(px(11.))
        .text_color(rgb(0x8b949e))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xc9d1d9)))
        .on_click(on_click)
        .child(label)
}

fn menu_separator() -> impl IntoElement {
    div()
        .h(px(1.))
        .mx_2()
        .my_1()
        .bg(rgb(0x30363d))
}

fn menu_item(
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .text_size(px(12.))
        .text_color(rgb(0xc9d1d9))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)))
        .on_click(on_click)
        .child(label)
}

fn menu_item_owned(
    id: impl Into<String>,
    label: String,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .text_size(px(12.))
        .text_color(rgb(0xc9d1d9))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)))
        .on_click(on_click)
        .child(label)
}


fn connection_detail_rows(connection: &SavedConnection) -> Vec<(&'static str, String)> {
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
            ..
        } => {
            rows.push(("Host", host.clone()));
            rows.push(("Port", port.to_string()));
            rows.push(("User", username.clone()));
        }
        nyaterm_domain::ConnectionType::LocalTerminal {
            shell_path,
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
            rows.push((
                "CWD",
                working_dir
                    .clone()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "—".to_string()),
            ));
        }
        nyaterm_domain::ConnectionType::Telnet { host, port, .. } => {
            rows.push(("Host", host.clone()));
            rows.push(("Port", port.to_string()));
        }
        nyaterm_domain::ConnectionType::Serial {
            port_name,
            baud_rate,
            ..
        } => {
            rows.push(("Port", port_name.clone()));
            rows.push(("Baud", baud_rate.to_string()));
        }
    }
    rows.push(("Last", format_last_used_ms(connection.last_used_at_ms)));
    rows.push(("Desc", description));
    rows
}
