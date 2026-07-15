use super::*;

impl NyaTermApp {
    pub(in crate::features) fn main_surface(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
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

    pub(in crate::features) fn session_tab_strip(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let sessions = self.ordered_tab_sessions();
        let session_count = sessions.len();
        // Child index layout for ScrollHandle: optional connecting tab, then sessions,
        // then optional end drop zone. Failed chrome is trailing after sessions.
        let connecting_tab_present = self.has_pending_session_start();
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

        if let Some(pending_name) = self.pending_session_display_name() {
            let pending_detail = self.pending_session_tab_detail().unwrap_or("Connecting...");
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
                                    .child(pending_detail),
                            ),
                    ),
            );
        }

        if sessions.is_empty() && !self.has_pending_session_start() {
            tabs = tabs.child(
                div()
                    .h_full()
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .child(
                        div()
                            .size(px(8.))
                            .rounded_full()
                            .bg(rgb(palette.text_dimmed)),
                    )
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
                let is_disconnected = leaf_ids.iter().any(|id| self.is_session_disconnected(id));
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
                                .child(svg().size(px(12.)).path(kind_icon).text_color(accent)),
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
                            this.child(div().size(px(8.)).rounded_full().bg(rgb(palette.success)))
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
                                .hover(|this| {
                                    this.bg(rgb(palette.border))
                                        .text_color(rgb(palette.success))
                                })
                                .child(svg().size(px(12.)).path("icons/conn/more.svg"))
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
                                .hover(|this| {
                                    this.bg(rgb(palette.border)).text_color(rgb(palette.danger))
                                })
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
        if !self.has_pending_session_start() {
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
        let tab_strip_has_overflow = self.session_tab_strip_scroll.max_offset().width > px(0.);
        // Tauri shows Open Tabs when the strip overflows; keep a small-count fallback.
        let show_open_tabs_menu = tab_strip_has_overflow || session_count >= 4 || open_tabs_menu;

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
                .child(small_button(
                    palette,
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
                .child(small_button(
                    palette,
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
                .child(small_button(
                    palette,
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
                .child(small_button(
                    palette,
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
                .child(small_button(
                    palette,
                    "workspace-smart-split",
                    "Tile",
                    cx.listener(|this, _, _, cx| {
                        this.apply_smart_split(SmartSplitMode::Auto, cx);
                    }),
                ));
        }
        if self.terminal_windows_is_multi_leaf() {
            session_actions = session_actions.child(small_button(
                palette,
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
                .child(small_button(
                    palette,
                    "workspace-split-ratio-dec",
                    "−",
                    cx.listener(|this, _, _, cx| {
                        this.adjust_workspace_split_ratio(-5, cx);
                    }),
                ))
                .child(small_button(
                    palette,
                    "workspace-split-ratio-inc",
                    "+",
                    cx.listener(|this, _, _, cx| {
                        this.adjust_workspace_split_ratio(5, cx);
                    }),
                ))
                .child(small_button(
                    palette,
                    "workspace-unsplit",
                    "Unsplit",
                    cx.listener(|this, _, _, cx| {
                        this.unsplit_workspace(cx);
                    }),
                ));
        }
        if session_count > 0 {
            session_actions = session_actions.child(small_button(
                palette,
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
}
