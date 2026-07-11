use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn main_surface(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        // The main surface always hosts the terminal workspace. Side panels are
        // rendered by the shell around this surface to match the Tauri layout.
        let palette = self.theme_palette();
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(rgb(palette.bg))
            .child(self.workspace_view(cx))
    }

    pub(in crate::ui::view) fn session_tab_strip(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let sessions = self.ordered_tab_sessions();
        let session_count = sessions.len();
        // Child index layout for ScrollHandle: optional connecting tab, then sessions,
        // then optional end drop zone. Failed chrome is trailing after sessions.
        let connecting_tab_present = self.pending_session_name.is_some();
        if self.session_tab_scroll_into_view_pending {
            if let Some(active_id) = self.active_session_id.as_deref() {
                if let Some(index) = sessions.iter().position(|session| session.id == active_id) {
                    let child_index = index + usize::from(connecting_tab_present);
                    self.session_tab_strip_scroll.scroll_to_item(child_index);
                }
            }
            self.session_tab_scroll_into_view_pending = false;
        }
        let mut tabs = div()
            .id("session-tab-strip-scroll")
            .h_full()
            .min_w_0()
            .flex_1()
            .flex()
            .items_center()
            // Tauri tab-strip-scroll: horizontal overflow instead of clipping tabs.
            .overflow_x_scroll()
            .overflow_y_hidden()
            .track_scroll(&self.session_tab_strip_scroll);

        if let Some(pending_name) = self.pending_session_name.clone() {
            tabs = tabs.child(
                div()
                    .h_full()
                    .min_w(px(178.))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .relative()
                    .border_r_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .child(
                        // Active-connecting top accent (Tauri connecting spinner color → warning).
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .right_0()
                            .h(px(2.))
                            .bg(rgb(palette.warning)),
                    )
                    .child(
                        div()
                            .size(px(14.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                svg()
                                    .size(px(12.))
                                    .path("icons/conn/connect.svg")
                                    .text_color(rgb(palette.warning)),
                            ),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_0()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(palette.text))
                                    .overflow_hidden()
                                    .child(truncate_preview(&pending_name, 22)),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.warning))
                                    .child("Connecting…"),
                            ),
                    ),
            );
        }

        if sessions.is_empty() && self.pending_session_name.is_none() {
            tabs = tabs.child(
                div()
                    .h_full()
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .child(div().size(px(8.)).rounded_full().bg(rgb(palette.text_dimmed)))
                    .child("No sessions"),
            );
        } else {
            for (tab_index, session) in sessions.into_iter().enumerate() {
                let display_name = self.session_display_name_by_info(&session);
                let session_id = session.id.clone();
                let actions_session_id = session.id.clone();
                let close_session_id = session.id.clone();
                let tab_number = tab_index + 1;
                let kind_icon = session_kind_icon_path(session.kind);
                let tooltip_title = display_name.clone();
                let tooltip_lines = self.session_tab_tooltip_lines(&session.id);
                let drag_payload = SessionTabDragPayload {
                    session_id: session.id.clone(),
                    display_name: display_name.clone(),
                    kind_label: session_kind_label(session.kind),
                };
                let drop_target_session_id = session.id.clone();
                let custom_color = self.session_tab_colors.get(&session.id).copied();
                // Active when any leaf under this tab root is focused.
                let is_active = self
                    .active_session_id
                    .as_deref()
                    .is_some_and(|id| self.tab_root_for_session(id) == session.id);
                let leaf_ids = self
                    .session_pane_roots
                    .get(&session.id)
                    .map(|root| root.session_ids())
                    .unwrap_or_else(|| vec![session.id.clone()]);
                let is_disconnected = leaf_ids
                    .iter()
                    .any(|id| self.is_session_disconnected(id));
                let is_split_tab = self
                    .session_pane_roots
                    .get(&session.id)
                    .is_some_and(|root| root.is_split());
                let pane_count = self
                    .session_pane_roots
                    .get(&session.id)
                    .map(|root| root.session_ids().len())
                    .unwrap_or(1);
                let tab_title = if is_disconnected {
                    format!("{} · disconnected", truncate_preview(&display_name, 20))
                } else if is_split_tab {
                    format!("{} · {pane_count}p", truncate_preview(&display_name, 22))
                } else {
                    truncate_preview(&display_name, 28)
                };
                let has_unread = leaf_ids.iter().any(|id| {
                    self.terminal_views
                        .get(id)
                        .is_some_and(|view| view.has_unread)
                });
                let sync_group = leaf_ids
                    .iter()
                    .find_map(|id| self.active_sync_group_for_session(id));
                let sync_paused = leaf_ids
                    .iter()
                    .any(|id| self.is_session_paused_in_active_sync_group(id));
                let show_sync_indicator = self.broadcast_to_all || sync_group.is_some();
                let sync_indicator_color = sync_group
                    .map(|group| group.color)
                    .unwrap_or(palette.accent);
                let accent = if let Some(custom_color) = custom_color {
                    rgb(custom_color)
                } else if is_disconnected {
                    rgb(palette.danger)
                } else if is_active {
                    rgb(palette.success)
                } else if has_unread {
                    rgb(palette.warning)
                } else {
                    rgb(palette.text_dimmed)
                };
                let bg = if let Some(custom_color) = custom_color {
                    rgba((custom_color << 8) | if is_active { 0x24 } else { 0x14 })
                } else if is_active {
                    rgb(palette.hover)
                } else {
                    rgb(palette.bg)
                };
                let hover_bg = if let Some(custom_color) = custom_color {
                    rgba((custom_color << 8) | if is_active { 0x32 } else { 0x22 })
                } else {
                    rgb(palette.hover)
                };
                tabs = tabs.child(
                    div()
                        .id(SharedString::from(format!("session-tab-{session_id}")))
                        .h_full()
                        .min_w(px(162.))
                        .max_w(px(236.))
                        .px_3()
                        .flex()
                        .items_center()
                        .gap_2()
                        .relative()
                        .when(is_active, |this| {
                            this.child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .h(px(2.))
                                    .w_full()
                                    .bg(accent),
                            )
                        })
                        .border_r_1()
                        .border_color(if is_active {
                            custom_color.map(rgb).unwrap_or_else(|| rgb(palette.border))
                        } else {
                            rgb(palette.border)
                        })
                        .bg(bg)
                        .when(is_disconnected, |this| this.opacity(0.78))
                        .cursor_pointer()
                        .hover(move |this| this.bg(hover_bg))
                        .cursor_move()
                        .on_drag(drag_payload, |payload, position, _, cx| {
                            cx.new(|_| SessionTabDragPreview::new(payload.clone(), position))
                        })
                        .on_drop(cx.listener(
                            move |this, payload: &SessionTabDragPayload, _, cx| {
                                this.reorder_session_before(
                                    payload.session_id.clone(),
                                    drop_target_session_id.clone(),
                                    cx,
                                );
                            },
                        ))
                        .when(custom_color.is_some(), move |this| {
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
                        // Tauri tab: top accent when active, icon + name + close.
                        .when(is_active, |this| {
                            this.child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .right_0()
                                    .h(px(2.))
                                    .bg(accent),
                            )
                            .child(
                                // Cover tab strip bottom border so the active tab blends into the terminal.
                                div()
                                    .absolute()
                                    .bottom_0()
                                    .left_0()
                                    .right_0()
                                    .h(px(1.))
                                    .bg(rgb(palette.bg)),
                            )
                        })
                        .child(
                            div()
                                .size(px(14.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    svg()
                                        .size(px(12.))
                                        .path(kind_icon)
                                        .text_color(accent),
                                ),
                        )
                        .child(
                            div()
                                .min_w(px(12.))
                                .text_size(px(11.))
                                .font_weight(FontWeight(700.))
                                .text_color(rgb(palette.text_dimmed))
                                .child(format!("{tab_number}")),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .text_size(px(12.))
                                .font_weight(if is_active {
                                    FontWeight(600.)
                                } else {
                                    FontWeight(500.)
                                })
                                .text_color(if is_disconnected {
                                    rgb(palette.text_dimmed)
                                } else if is_active {
                                    rgb(palette.text)
                                } else {
                                    rgb(palette.text_muted)
                                })
                                .overflow_hidden()
                                .child(tab_title.clone()),
                        )
                        .when(show_sync_indicator, |this| {
                            this.child(
                                div()
                                    .size(px(14.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .opacity(if sync_paused { 0.4 } else { 1. })
                                    .child(
                                        svg()
                                            .size(px(11.))
                                            .path("icons/sync.svg")
                                            .text_color(rgb(sync_indicator_color)),
                                    ),
                            )
                        })
                        .when(has_unread && !is_active, |this| {
                            this.child(
                                div()
                                    .size(px(8.))
                                    .rounded_full()
                                    .bg(rgb(palette.success)),
                            )
                        })
                        .child(
                            div()
                                .id(SharedString::from(format!(
                                    "session-tab-actions-{actions_session_id}"
                                )))
                                .size(px(22.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_sm()
                                .text_xs()
                                .font_weight(FontWeight(800.))
                                .text_color(rgb(palette.text_muted))
                                .hover(|this| this.bg(rgb(palette.border)).text_color(rgb(palette.success)))
                                .child(
                                    svg()
                                        .size(px(12.))
                                        .path("icons/conn/more.svg"),
                                )
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.open_tab_actions(actions_session_id.clone(), window, cx);
                                })),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!(
                                    "session-tab-close-{close_session_id}"
                                )))
                                .size(px(18.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_sm()
                                .text_xs()
                                .text_color(rgb(palette.text_muted))
                                .hover(|this| this.bg(rgb(palette.border)).text_color(rgb(palette.danger)))
                                .child("x")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.close_session(close_session_id.clone(), cx);
                                })),
                        )
                        .tooltip(move |_, cx| {
                            cx.new(|_| {
                                SessionTabTooltip::new(tooltip_title.clone(), tooltip_lines.clone())
                            })
                            .into()
                        })
                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            this.handle_session_tab_click(session_id.clone(), event, window, cx);
                        })),
                );
            }
        }

        // Tauri connectError tab chrome: ephemeral failed connect pill after sessions.
        if self.pending_session_name.is_none() {
            if let (Some(failed_name), Some(failed_error)) = (
                self.last_connect_failure_name.clone(),
                self.last_connect_failure_error.clone(),
            ) {
                let dismiss_name = failed_name.clone();
                tabs = tabs.child(
                    div()
                        .id("session-tab-connect-failed")
                        .h_full()
                        .min_w(px(178.))
                        .max_w(px(280.))
                        .px_3()
                        .flex()
                        .items_center()
                        .gap_2()
                        .relative()
                        .border_r_1()
                        .border_color(rgb(palette.border))
                        .bg(rgba((palette.danger << 8) | 0x18))
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .right_0()
                                .h(px(2.))
                                .bg(rgb(palette.danger)),
                        )
                        .child(
                            div()
                                .size(px(14.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    svg()
                                        .size(px(12.))
                                        .path("icons/session/disconnect.svg")
                                        .text_color(rgb(palette.danger)),
                                ),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap_0()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(palette.danger))
                                        .overflow_hidden()
                                        .child(format!("Failed {failed_name}")),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(rgb(palette.text_muted))
                                        .overflow_hidden()
                                        .child(truncate_preview(&failed_error, 36)),
                                ),
                        )
                        .child(
                            div()
                                .id("session-tab-connect-failed-dismiss")
                                .size(px(18.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_sm()
                                .text_xs()
                                .text_color(rgb(palette.text_muted))
                                .hover(|this| {
                                    this.bg(rgb(palette.border)).text_color(rgb(palette.danger))
                                })
                                .child("x")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    if this.last_connect_failure_name.as_deref()
                                        == Some(dismiss_name.as_str())
                                    {
                                        this.last_connect_failure_name = None;
                                        this.last_connect_failure_error = None;
                                        cx.notify();
                                    }
                                })),
                        )
                        .tooltip(move |_, cx| {
                            cx.new(|_| {
                                SessionTabTooltip::new(
                                    format!("Failed {failed_name}"),
                                    vec![failed_error.clone()],
                                )
                            })
                            .into()
                        }),
                );
            }
        }

        if session_count > 1 {
            tabs = tabs.child(
                div()
                    .id("session-tab-drop-end")
                    .h_full()
                    .min_w(px(28.))
                    .flex_none()
                    .border_l_1()
                    .border_color(rgb(palette.border))
                    .hover(|this| this.bg(rgb(palette.hover)))
                    .on_drop(cx.listener(|this, payload: &SessionTabDragPayload, _, cx| {
                        this.reorder_session_to_end(payload.session_id.clone(), cx);
                    })),
            );
        }

        // Tauri TabBar trailing chrome: optional open-tabs overflow menu + new session menu.
        let open_tabs_menu = self.open_tabs_menu_open;
        let new_session_menu = self.new_session_menu_open;
        let tab_strip_has_overflow =
            self.session_tab_strip_scroll.max_offset().width > px(0.);
        // Tauri shows Open Tabs when the strip overflows; keep a small-count fallback.
        let show_open_tabs_menu =
            tab_strip_has_overflow || session_count >= 4 || open_tabs_menu;

        let mut session_actions = div()
            .h_full()
            .flex()
            .items_center()
            .gap_0()
            .border_l_1()
            .border_color(rgb(palette.border));

        if show_open_tabs_menu {
            session_actions = session_actions.child(
                div()
                    .relative()
                    .h_full()
                    .child(
                        div()
                            .id("workspace-open-tabs-menu")
                            .h_full()
                            .w(px(32.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_r_1()
                            .border_color(rgb(palette.border))
                            .bg(if open_tabs_menu {
                                rgb(palette.hover)
                            } else {
                                rgb(palette.surface)
                            })
                            .text_color(rgb(palette.text_muted))
                            .cursor_pointer()
                            .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
                            .child("▾")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_open_tabs_menu(cx);
                            })),
                    )
                    .when(open_tabs_menu, |this| {
                        this.child(self.render_open_tabs_menu(cx))
                    }),
            );
        }

        session_actions = session_actions
            .child(
                div()
                    .relative()
                    .h_full()
                    .child(
                        div()
                            .id("workspace-new-session-menu")
                            .h_full()
                            .w(px(36.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_r_1()
                            .border_color(rgb(palette.border))
                            .bg(if new_session_menu {
                                rgb(palette.hover)
                            } else {
                                rgb(palette.surface)
                            })
                            .text_size(px(16.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text_muted))
                            .cursor_pointer()
                            .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
                            .child("+")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_new_session_menu(cx);
                            })),
                    )
                    .when(new_session_menu, |this| {
                        this.child(self.render_new_session_menu(cx))
                    }),
            )
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_2()
                    .child(small_button(
                        palette,
                        "workspace-quick-switch",
                        "Switch",
                        cx.listener(|this, _, window, cx| {
                            this.close_open_tabs_menu(cx);
                            this.close_new_session_menu(cx);
                            this.open_quick_switch(window, cx);
                        }),
                    )),
            );
        if self.active_session_id.is_some() {
            session_actions = session_actions
                .child(small_button(palette, 
                    "workspace-split-horizontal",
                    "H",
                    cx.listener(|this, _, window, cx| {
                        this.split_workspace_with_duplicate(
                            WorkspaceSplitDirection::Horizontal,
                            window,
                            cx,
                        );
                    }),
                ))
                .child(small_button(palette, 
                    "workspace-split-vertical",
                    "V",
                    cx.listener(|this, _, window, cx| {
                        this.split_workspace_with_duplicate(
                            WorkspaceSplitDirection::Vertical,
                            window,
                            cx,
                        );
                    }),
                ))
                .child(small_button(palette,
                    "workspace-window-right",
                    "W|",
                    cx.listener(|this, _, _, cx| {
                        this.split_active_tab_to_new_window_leaf(
                            WorkspaceSplitDirection::Vertical,
                            SplitEdge::After,
                            cx,
                        );
                    }),
                ))
                .child(small_button(palette,
                    "workspace-window-below",
                    "W—",
                    cx.listener(|this, _, _, cx| {
                        this.split_active_tab_to_new_window_leaf(
                            WorkspaceSplitDirection::Horizontal,
                            SplitEdge::After,
                            cx,
                        );
                    }),
                ))
                .child(small_button(palette,
                    "workspace-smart-split",
                    "Tile",
                    cx.listener(|this, _, _, cx| {
                        this.apply_smart_split(SmartSplitMode::Auto, cx);
                    }),
                ));
        }
        if self.terminal_windows_is_multi_leaf() {
            session_actions = session_actions.child(small_button(palette,
                "workspace-window-merge",
                "Merge",
                cx.listener(|this, _, _, cx| {
                    this.close_terminal_window_layout(cx);
                }),
            ));
        }
        if self
            .active_session_id
            .as_deref()
            .map(|id| self.tab_root_for_session(id))
            .and_then(|root| self.session_pane_roots.get(&root))
            .is_some_and(|root| root.is_split())
            || self.workspace_split.is_some()
        {
            session_actions = session_actions
                .child(small_button(palette, 
                    "workspace-split-ratio-dec",
                    "−",
                    cx.listener(|this, _, _, cx| {
                        this.adjust_workspace_split_ratio(-5, cx);
                    }),
                ))
                .child(small_button(palette, 
                    "workspace-split-ratio-inc",
                    "+",
                    cx.listener(|this, _, _, cx| {
                        this.adjust_workspace_split_ratio(5, cx);
                    }),
                ))
                .child(small_button(palette, 
                    "workspace-unsplit",
                    "Unsplit",
                    cx.listener(|this, _, _, cx| {
                        this.unsplit_workspace(cx);
                    }),
                ));
        }
        if session_count > 0 {
            session_actions = session_actions.child(small_button(palette, 
                "workspace-close-all-sessions",
                "All",
                cx.listener(|this, _, window, cx| {
                    this.open_close_all_sessions_confirm(window, cx);
                }),
            ));
        }

        div()
            .h(px(36.)) // Tauri TabBar: h-9
            .flex()
            .items_center()
            .border_b_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .child(tabs)
            .child(session_actions)
    }

    fn render_open_tabs_menu(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        // Tauri openTabsMenuItems is reversed (rightmost first) but keeps global ordinals.
        let ordered = self.ordered_tab_sessions();
        let ordinals: std::collections::HashMap<String, usize> = ordered
            .iter()
            .enumerate()
            .map(|(index, session)| (session.id.clone(), index + 1))
            .collect();
        let mut sessions = ordered;
        sessions.reverse();
        let active_id = self.active_session_id.clone();
        let mut menu = div()
            .id("workspace-open-tabs-dropdown")
            .absolute()
            .top(px(36.))
            .right_0()
            .w(px(280.))
            .max_h(px(360.))
            .overflow_y_scroll()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .shadow_lg()
            .py_1()
            .flex()
            .flex_col()
                        .child(
                div()
                    .px_3()
                    .py_1()
                    .text_size(px(10.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.text_dimmed))
                    .child("Open Tabs"),
            )
            .child(
                div()
                    .mx_2()
                    .my_1()
                    .h(px(1.))
                    .bg(rgb(palette.border)),
            );

        if sessions.is_empty() {
            menu = menu.child(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(rgb(palette.text_muted))
                    .child("No open sessions"),
            );
        } else {
            for (index, session) in sessions.into_iter().enumerate() {
                let session_id = session.id.clone();
                let is_active = active_id.as_deref() == Some(session_id.as_str());
                let is_disconnected = self.is_session_disconnected(&session_id);
                let title = self.session_display_name_by_info(&session);
                let kind = session_kind_label(session.kind);
                let accent = if let Some(color) = self.session_tab_colors.get(&session_id).copied() {
                    rgb(color)
                } else if is_disconnected {
                    rgb(palette.danger)
                } else if is_active {
                    rgb(palette.success)
                } else {
                    rgb(palette.text_dimmed)
                };
                menu = menu.child(
                    div()
                        .id(SharedString::from(format!("open-tabs-menu-{session_id}")))
                        .h(px(32.))
                        .px_3()
                        .flex()
                        .items_center()
                        .gap_2()
                        .cursor_pointer()
                        .bg(if is_active {
                            rgb(palette.hover)
                        } else {
                            rgb(palette.surface)
                        })
                        .hover(|this| this.bg(rgb(palette.hover)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.close_open_tabs_menu(cx);
                            this.select_session(session_id.clone(), cx);
                            window.focus(&this.terminal_focus);
                        }))
                        .child(
                            div()
                                .size(px(8.))
                                .rounded_full()
                                .bg(accent),
                        )
                        .child(
                            div()
                                .min_w(px(14.))
                                .text_size(px(11.))
                                .font_weight(FontWeight(700.))
                                .text_color(rgb(palette.text_dimmed))
                                .child(format!(
                                    "{}",
                                    ordinals.get(&session.id).copied().unwrap_or(index + 1)
                                )),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .text_size(px(12.))
                                .font_weight(if is_active {
                                    FontWeight(700.)
                                } else {
                                    FontWeight(500.)
                                })
                                .text_color(if is_disconnected {
                                    rgb(palette.text_dimmed)
                                } else {
                                    rgb(palette.text)
                                })
                                .overflow_hidden()
                                .child(truncate_preview(&title, 28)),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(palette.text_dimmed))
                                .child(kind),
                        ),
                );
            }
        }
        menu
    }

    fn render_new_session_menu(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        // Tauri TabBar new-session: shell sessions + recent by last_used.
        let mut shell: Vec<_> = self
            .connections
            .iter()
            .filter(|connection| matches!(connection.config, ConnectionType::LocalTerminal { .. }))
            .cloned()
            .collect();
        shell.sort_by_key(|connection| connection.sort_order);
        let mut recent: Vec<_> = self
            .connections
            .iter()
            .filter(|connection| connection.last_used_at_ms.unwrap_or(0) > 0)
            .cloned()
            .collect();
        recent.sort_by(|left, right| {
            right
                .last_used_at_ms
                .unwrap_or(0)
                .cmp(&left.last_used_at_ms.unwrap_or(0))
        });
        recent.truncate(10);
        if recent.is_empty() {
            // Fallback when no usage timestamps yet: first non-shell connections.
            recent = self
                .connections
                .iter()
                .filter(|connection| {
                    !matches!(connection.config, ConnectionType::LocalTerminal { .. })
                })
                .cloned()
                .take(8)
                .collect();
        }

        let mut menu = div()
            .id("workspace-new-session-dropdown")
            .absolute()
            .top(px(36.))
            .right_0()
            .w(px(300.))
            .max_h(px(460.))
            .overflow_y_scroll()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .shadow_lg()
            .py_1()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("new-session-local")
                    .h(px(32.))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .hover(|this| this.bg(rgb(palette.hover)))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.close_new_session_menu(cx);
                        this.start_local_session(window, cx);
                    }))
                    .child(
                        svg()
                            .size(px(12.))
                            .path("icons/conn/terminal.svg")
                            .text_color(rgb(palette.success)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette.text))
                            .child("New Local Session"),
                    ),
            )
            .child(
                div()
                    .id("new-session-temp-ssh")
                    .h(px(32.))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .hover(|this| this.bg(rgb(palette.hover)))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.close_new_session_menu(cx);
                        this.open_temporary_ssh_link_dialog(window, cx);
                    }))
                    .child(
                        svg()
                            .size(px(12.))
                            .path("icons/conn/server.svg")
                            .text_color(rgb(palette.accent)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette.text))
                            .child("Temporary SSH Link"),
                    ),
            )
            .child(
                div()
                    .id("new-session-connections")
                    .h(px(32.))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .hover(|this| this.bg(rgb(palette.hover)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.close_new_session_menu(cx);
                        this.ensure_panel_open(NavItem::Connections);
                        cx.notify();
                    }))
                    .child(
                        svg()
                            .size(px(12.))
                            .path("icons/connections.svg")
                            .text_color(rgb(palette.text_muted)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette.text))
                            .child("All Connections…"),
                    ),
            );

        menu = menu.child(
            div()
                .mx_2()
                .my_1()
                .h(px(1.))
                .bg(rgb(palette.border)),
        )
        .child(
            div()
                .px_3()
                .py_1()
                .text_size(px(10.))
                .font_weight(FontWeight(700.))
                .text_color(rgb(palette.text_dimmed))
                .child("Shell Sessions"),
        );
        if shell.is_empty() {
            menu = menu.child(
                div()
                    .px_3()
                    .py_1()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child("No shell sessions"),
            );
        } else {
            for connection in shell {
                menu = menu.child(self.new_session_connection_row(palette, connection, cx));
            }
        }

        menu = menu
            .child(
                div()
                    .mx_2()
                    .my_1()
                    .h(px(1.))
                    .bg(rgb(palette.border)),
            )
            .child(
                div()
                    .px_3()
                    .py_1()
                    .text_size(px(10.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.text_dimmed))
                    .child("Recent Sessions"),
            );
        if recent.is_empty() {
            menu = menu.child(
                div()
                    .px_3()
                    .py_1()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child("No recent sessions"),
            );
        } else {
            for connection in recent {
                menu = menu.child(self.new_session_connection_row(palette, connection, cx));
            }
        }
        menu
    }

    fn new_session_connection_row(
        &mut self,
        palette: ThemePalette,
        connection: SavedConnection,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let connection_id = connection.id.clone();
        let name = connection.name.clone();
        let kind = connection.kind_label();
        let icon = match &connection.config {
            ConnectionType::Ssh { .. } => "icons/conn/server.svg",
            ConnectionType::Telnet { .. } => "icons/conn/telnet.svg",
            ConnectionType::Serial { .. } => "icons/conn/serial.svg",
            ConnectionType::LocalTerminal { .. } => "icons/conn/terminal.svg",
        };
        let endpoint = connection.endpoint();
        let label = if endpoint.is_empty() {
            name.clone()
        } else {
            format!("{name}")
        };
        div()
            .id(SharedString::from(format!("new-session-conn-{connection_id}")))
            .h(px(32.))
            .px_3()
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .hover(|this| this.bg(rgb(palette.hover)))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.close_new_session_menu(cx);
                if let Some(connection) = this
                    .connections
                    .iter()
                    .find(|item| item.id == connection_id)
                    .cloned()
                {
                    this.start_saved_connection(connection, window, cx);
                }
            }))
            .child(
                svg()
                    .size(px(12.))
                    .path(icon)
                    .text_color(rgb(palette.text_muted)),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette.text))
                            .overflow_hidden()
                            .child(truncate_preview(&label, 28)),
                    )
                    .when(!endpoint.is_empty() && endpoint != name, |this| {
                        this.child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(palette.text_dimmed))
                                .overflow_hidden()
                                .child(truncate_preview(&endpoint, 32)),
                        )
                    }),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(rgb(palette.text_dimmed))
                    .child(kind),
            )
    }

    pub(in crate::ui::view) fn empty_workspace_state(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Match Tauri EmptyWorkspaceState: large faded logo + label|shortcut rows.
        let temporary_ssh = self.display_shortcut_for("tab.temporarySshLink", "Ctrl+Alt+N");
        let open_chat = self.display_shortcut_for("view.openChat", "Ctrl+Alt+I");
        let show_commands = self.display_shortcut_for("view.showAllCommands", "Ctrl+Shift+P");
        let switch_terminal = self.display_shortcut_for("tab.quickSwitch", "Ctrl+Shift+S");

        let palette = self.theme_palette();
        let failure_banner = if let (Some(name), Some(error)) = (
            self.last_connect_failure_name.clone(),
            self.last_connect_failure_error.clone(),
        ) {
            Some((name, error))
        } else {
            None
        };
        let connecting_name = self.pending_session_name.clone();
        let terminal_palette = self.terminal_theme_palette();
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(terminal_palette.terminal_bg))
            .px_6()
            .child(
                div()
                    .w(px(544.))
                    .max_w_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(div().mb_9().child(nyaterm_logo_mark(terminal_palette, 256., 0.13)))
                    .when_some(connecting_name, |this, name| {
                        this.child(
                            div()
                                .mb_4()
                                .w(px(480.))
                                .max_w_full()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.warning))
                                .bg(rgba((palette.warning << 8) | 0x18))
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(palette.warning))
                                        .child(format!("Connecting {name}…")),
                                ),
                        )
                    })
                    .when_some(failure_banner, |this, (name, error)| {
                        this.child(
                            div()
                                .mb_4()
                                .w(px(480.))
                                .max_w_full()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.danger))
                                .bg(rgba((palette.danger << 8) | 0x18))
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(palette.danger))
                                        .child(format!("Failed to start {name}")),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_muted))
                                        .child(truncate_preview(&error, 120)),
                                ),
                        )
                    })
                    .child(
                        // Tauri EmptyWorkspaceState: grid w-fit max-w-[30rem] gap-x-4 gap-y-3
                        div()
                            .w(px(480.))
                            .max_w_full()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(empty_workspace_action(
                                palette,
                                "Temporary Link",
                                temporary_ssh,
                                cx.listener(|this, _, window, cx| {
                                    this.ensure_panel_open(NavItem::Connections);
                                    this.open_temporary_ssh_link_dialog(window, cx);
                                }),
                            ))
                            .child(empty_workspace_action(
                                palette,
                                "Open Chat",
                                open_chat,
                                cx.listener(|this, _, window, cx| {
                                    this.ensure_panel_open(NavItem::AiAssistant);
                                    window.focus(&this.ai_chat_focus);
                                    this.ai_status = "AI assistant focused".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(empty_workspace_action(
                                palette,
                                "Show All Commands",
                                show_commands,
                                cx.listener(|this, _, window, cx| {
                                    this.bottom_panel = BottomPanelMode::QuickCommands;
                                    window.focus(&this.command_search_focus);
                                    this.terminal_status = "quick commands opened".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(empty_workspace_action(
                                palette,
                                "Switch Terminal",
                                switch_terminal,
                                cx.listener(|this, _, window, cx| {
                                    this.open_quick_switch(window, cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::ui::view) fn bottom_panel_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match self.bottom_panel {
            BottomPanelMode::QuickCommands => {
                let palette = self.theme_palette();
                div()
                    .h(px(220.))
                    .flex_none()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .child(self.quick_commands_panel(cx))
                    .into_any_element()
            }
            BottomPanelMode::CommandSend => self.bottom_command_send_bar(cx).into_any_element(),
            BottomPanelMode::Hidden => div().into_any_element(),
        }
    }

    fn bottom_quick_commands_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();

        let commands = sorted_quick_commands(&self.quick_commands);
        let mut command_row = div()
            .flex()
            .items_center()
            .gap_2()
            .min_w_0()
            .overflow_hidden();
        if commands.is_empty() {
            command_row = command_row.child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.text_dimmed))
                    .child("No quick commands saved."),
            );
        } else {
            for (index, command) in commands.into_iter().take(5).enumerate() {
                command_row = command_row.child(self.bottom_quick_command_chip(index, command, cx));
            }
        }

        div()
            .h(px(112.))
            .flex_none()
            .border_t_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .p_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text_muted))
                                    .child("Quick Commands"),
                            )
                            .child(status_pill("bottom panel", rgb(palette.accent), rgb(palette.hover))),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(palette, 
                                "bottom-command-search",
                                "Search",
                                cx.listener(|this, _, window, cx| {
                                    window.focus(&this.command_search_focus);
                                    this.terminal_status = "command search focused".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(small_button(palette, 
                                "bottom-command-refresh",
                                "Refresh",
                                cx.listener(|this, _, _, cx| {
                                    this.refresh_quick_commands();
                                    this.terminal_status = "quick commands refreshed".to_string();
                                    cx.notify();
                                }),
                            )),
                    ),
            )
            .child(div().mt_3().child(command_row))
    }

    fn bottom_quick_command_chip(
        &self,
        index: usize,
        command: QuickCommand,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let title = truncate_preview(&command.label, 22);
        let preview = truncate_preview(&command.command, 38);
        let category = quick_command_category_label(&self.quick_command_categories, &command);
        let execute_label = if command.execution_mode.as_deref() == Some("append") {
            "Insert"
        } else {
            "Run"
        };
        let palette = self.theme_palette();

        div()
            .id(SharedString::from(format!("bottom-quick-command-{index}")))
            .w(px(176.))
            .h(px(56.))
            .flex_none()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .p_2()
            .cursor_pointer()
            .hover(move |this| this.bg(rgb(palette.hover)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(palette.success))
                            .child(execute_label),
                    ),
            )
            .child(
                div()
                    .mt_1()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .child(format!("{category} / {preview}")),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if execute_label == "Insert" {
                    this.insert_quick_command(index, cx);
                } else {
                    this.run_quick_command(index, cx);
                }
            }))
    }
}

fn session_kind_icon_path(kind: SessionKind) -> &'static str {
    match kind {
        SessionKind::Ssh => "icons/conn/server.svg",
        SessionKind::Telnet | SessionKind::RawTcp => "icons/conn/telnet.svg",
        SessionKind::Serial => "icons/conn/serial.svg",
        SessionKind::LocalPty => "icons/conn/terminal.svg",
    }
}
