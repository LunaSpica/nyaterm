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
        let has_connect_failure =
            self.last_connect_failure_name.is_some() && self.last_connect_failure_error.is_some();
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

        if sessions.is_empty() && !self.has_pending_session_start() && !has_connect_failure {
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
                let close_session_id = session.id.clone();
                let tab_group_name = SharedString::from(format!("session-tab-group-{session_id}"));
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
                let tab_title = truncate_preview(&display_name, 28);
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
                    .unwrap_or(palette.primary);
                let accent = if let Some(custom_color) = custom_color {
                    rgb(custom_color)
                } else if is_disconnected {
                    rgb(palette.danger)
                } else if is_active {
                    rgb(palette.primary)
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
                        .group(tab_group_name.clone())
                        .h_full()
                        .min_w(px(118.))
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
                                    "session-tab-close-{close_session_id}"
                                )))
                                .size(px(18.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_sm()
                                .text_xs()
                                .text_color(rgb(palette.text_muted))
                                .when(!is_active, |this| {
                                    this.opacity(0.)
                                        .group_hover(tab_group_name.clone(), |style| {
                                            style.opacity(1.)
                                        })
                                })
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
        let open_tabs_label = self.tr("terminal.openTabs").to_string();
        let new_session_label = self.tr("terminal.newSession").to_string();
        let tab_strip_has_overflow = self.session_tab_strip_scroll.max_offset().width > px(0.);
        // Tauri shows Open Tabs only when the strip actually overflows.
        let show_open_tabs_menu = tab_strip_has_overflow || open_tabs_menu;

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
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_, _, _, cx| cx.stop_propagation()),
                    )
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
                            .child(
                                svg()
                                    .size(px(16.))
                                    .flex_none()
                                    .path("icons/chevron-down.svg"),
                            )
                            .tooltip(move |_, cx| {
                                cx.new(|_| {
                                    crate::features::ChromeTooltip::new(open_tabs_label.clone())
                                })
                                .into()
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_open_tabs_menu(cx);
                            })),
                    )
                    .when(open_tabs_menu, |this| {
                        this.child(self.render_open_tabs_menu(cx))
                    }),
            );
        }

        session_actions = session_actions.child(
            div()
                .relative()
                .h_full()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|_, _, _, cx| cx.stop_propagation()),
                )
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
                        .text_color(rgb(palette.text_muted))
                        .cursor_pointer()
                        .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
                        .child(svg().size(px(16.)).flex_none().path("icons/conn/add.svg"))
                        .tooltip(move |_, cx| {
                            cx.new(|_| {
                                crate::features::ChromeTooltip::new(new_session_label.clone())
                            })
                            .into()
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_new_session_menu(cx);
                        })),
                )
                .when(new_session_menu, |this| {
                    this.child(self.render_new_session_menu(cx))
                }),
        );

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
