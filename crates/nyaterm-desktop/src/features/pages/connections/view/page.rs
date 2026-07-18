use super::*;

impl NyaTermApp {
    pub(in crate::features) fn connections_view(
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
        let selected_connections = self.selected_connections();
        let selected_count = selected_connections.len();

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
                        rows = rows.child(div().mx_2().my_1().h(px(1.)).bg(rgb(palette.border)));
                    }
                    ConnectionListRow::GroupHeader(section) => {
                        rows = rows.child(self.connection_section(section, true, cx));
                    }
                    ConnectionListRow::EmptyGroup { depth } => {
                        rows = rows.child(
                            div()
                                .px_2()
                                .py_1()
                                .pl(px(connection_tree_indent_px(depth)))
                                .h(px(28.))
                                .text_size(px(11.))
                                .text_color(rgb(palette.text_dimmed))
                                .child("Empty group"),
                        );
                    }
                    ConnectionListRow::Connection { connection, depth } => {
                        rows = rows.child(self.saved_connection_row(connection, depth, cx));
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
    }

    pub(in crate::features) fn connections_search_bar(
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
            .child(icon_action_button(
                palette,
                "connections-sort",
                sort_label,
                cx.listener(|this, _, _, cx| {
                    this.cycle_connection_sort_mode(cx);
                }),
            ))
            .child(icon_action_button(
                palette,
                "connections-temp-ssh",
                "icons/conn/flash.svg",
                cx.listener(|this, _, window, cx| {
                    this.open_temporary_ssh_link_dialog(window, cx);
                }),
            ))
            .child(icon_action_button(
                palette,
                "connections-new-group",
                "icons/conn/folder.svg",
                cx.listener(|this, _, window, cx| {
                    this.open_connection_group_editor(None, None, window, cx);
                }),
            ))
            .child(icon_action_button(
                palette,
                "connections-new",
                "icons/conn/add.svg",
                cx.listener(|this, _, window, cx| {
                    this.open_connection_editor(None, None, false, window, cx);
                }),
            ))
            .child(
                div()
                    .relative()
                    .child(icon_action_button(
                        palette,
                        "connections-more",
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

    pub(in crate::features) fn connections_selection_strip(
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
                    .hover(|this| {
                        this.bg(rgb(palette.surface_elevated))
                            .text_color(rgb(palette.text))
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.clear_selected_connections(cx);
                    }))
                    .child("Clear"),
            )
    }
}
