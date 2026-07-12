use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let width = self.left_panel_width.clamp(160., 720.);
        let palette = self.theme_palette();
        div()
            .w(px(width))
            .flex_none()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .child(self.side_panel_stack(PanelSide::Left, cx))
    }

    pub(in crate::ui::view) fn left_panel_body(
        &mut self,
        panel: NavItem,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match panel {
            NavItem::Transfers => self.transfers_view(cx).into_any_element(),
            NavItem::Tunnels => self.tunnels_view(cx).into_any_element(),
            NavItem::SecurityAuth => self.security_auth_panel(cx).into_any_element(),
            NavItem::SyncBackupHistory => self.sync_backup_history_panel(cx).into_any_element(),
            NavItem::Migration => self.migration_view().into_any_element(),
            NavItem::ActiveSessions => self.active_sessions_panel(cx).into_any_element(),
            _ => self.left_workspace_summary(cx).into_any_element(),
        }
    }

    /// Tauri ActiveSessions panel: search strip + dense session rows (no workspace cards).
    pub(in crate::ui::view) fn active_sessions_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let sessions = self.ordered_sessions();
        let session_count = sessions.len();
        let query = self
            .active_sessions_search_draft
            .trim()
            .to_ascii_lowercase();
        let mut rows = div().flex().flex_col().gap_1().p_2();
        let mut visible_count = 0usize;
        if sessions.is_empty() {
            rows = rows.child(
                div()
                    .py_4()
                    .text_center()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child("No active sessions"),
            );
        } else {
            for session in sessions {
                let display_name = self.session_display_name_by_info(&session);
                let haystack = format!(
                    "{} {} {} {}",
                    display_name,
                    session.name,
                    session_kind_label(session.kind),
                    session.id
                )
                .to_ascii_lowercase();
                if !query.is_empty() && !haystack.contains(&query) {
                    continue;
                }
                visible_count += 1;
                rows = rows.child(self.active_session_row(session, display_name, cx));
            }
            if visible_count == 0 {
                rows = rows.child(
                    div()
                        .py_4()
                        .text_center()
                        .text_size(px(11.))
                        .text_color(rgb(palette.text_dimmed))
                        .child("No matching sessions"),
                );
            }
        }

        let count_label = if query.is_empty() {
            session_count.to_string()
        } else {
            format!("{visible_count}/{session_count}")
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(palette.surface))
            .child(
                div()
                    .h(px(36.))
                    .flex_none()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.section_header))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div().flex_1().min_w_0().child(
                            transfer_input(
                                "active-sessions-search-input",
                                "Search sessions",
                                self.active_sessions_search_draft.clone(),
                                true,
                                self.theme_palette(),
                            )
                            .h(px(28.))
                            .track_focus(&self.active_sessions_search_focus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                window.focus(&this.active_sessions_search_focus);
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(
                                |this, event: &KeyDownEvent, _, cx| {
                                    this.handle_active_sessions_search_key_down(event, cx);
                                },
                            )),
                        ),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(count_label),
                    ),
            )
            .child(
                div()
                    .id(SharedString::from("active-sessions-list"))
                    .flex_1()
                    .min_h_0()
                    .overflow_scroll()
                    .scrollbar_width(px(6.))
                    .child(rows),
            )
    }

    pub(in crate::ui::view) fn left_workspace_summary(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let sessions = self.ordered_sessions();
        let session_count = sessions.len();
        let query = self
            .active_sessions_search_draft
            .trim()
            .to_ascii_lowercase();
        let mut active_session_rows = div().mt_3().flex().flex_col().gap_2();
        let mut visible_count = 0usize;
        if sessions.is_empty() {
            active_session_rows = active_session_rows.child(empty_panel(
                "No active runtime sessions.",
                self.theme_palette(),
            ));
        } else {
            for session in sessions {
                let display_name = self.session_display_name_by_info(&session);
                let haystack = format!(
                    "{} {} {} {}",
                    display_name,
                    session.name,
                    session_kind_label(session.kind),
                    session.id
                )
                .to_ascii_lowercase();
                if !query.is_empty() && !haystack.contains(&query) {
                    continue;
                }
                visible_count += 1;
                active_session_rows =
                    active_session_rows.child(self.active_session_row(session, display_name, cx));
            }
            if visible_count == 0 {
                active_session_rows = active_session_rows.child(empty_panel(
                    "No matching active sessions.",
                    self.theme_palette(),
                ));
            }
        }

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text_muted))
                            .child("WORKSPACE"),
                    )
                    .child(capability_line(
                        palette,
                        "Active Sessions",
                        session_count.to_string(),
                    ))
                    .child(capability_line(
                        palette,
                        "Profiles",
                        self.connections.len().to_string(),
                    ))
                    .child(capability_line(
                        palette,
                        "Quick Commands",
                        self.quick_commands.len().to_string(),
                    ))
                    .child(capability_line(
                        palette,
                        "Tunnels",
                        self.tunnels.len().to_string(),
                    )),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text_muted))
                                    .child("ACTIVE SESSIONS"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(if query.is_empty() {
                                        rgb(palette.text_muted)
                                    } else {
                                        rgb(palette.success)
                                    })
                                    .child(if query.is_empty() {
                                        session_count.to_string()
                                    } else {
                                        format!("{visible_count}/{session_count}")
                                    }),
                            ),
                    )
                    .child(
                        transfer_input(
                            "active-sessions-search-input",
                            "Search sessions",
                            self.active_sessions_search_draft.clone(),
                            true,
                            self.theme_palette(),
                        )
                        .mt_3()
                        .track_focus(&self.active_sessions_search_focus)
                        .on_click(cx.listener(|this, _, window, cx| {
                            window.focus(&this.active_sessions_search_focus);
                            cx.notify();
                        }))
                        .on_key_down(cx.listener(
                            |this, event: &KeyDownEvent, _, cx| {
                                this.handle_active_sessions_search_key_down(event, cx);
                            },
                        )),
                    )
                    .child(active_session_rows),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(palette.text_muted))
                                    .child("START"),
                            )
                            .child(status_pill(
                                status_label(&self.terminal_status),
                                rgb(palette.accent),
                                rgb(palette.hover),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "left-start-local",
                                "Local",
                                cx.listener(|this, _, window, cx| {
                                    this.start_local_session(window, cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "left-probe",
                                "Probe",
                                cx.listener(|this, _, _, cx| {
                                    this.send_probe_command(cx);
                                }),
                            )),
                    ),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child("Runtime"),
                    )
                    .child(div().mt_1().text_sm().child(match self.runtime.mode() {
                        RuntimeMode::Portable => "Portable",
                        RuntimeMode::Installed => "Installed",
                    }))
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(self.runtime.config_dir().display().to_string()),
                    ),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(if self.store_status.ready {
                        rgb(palette.hover)
                    } else {
                        rgb(palette.hover)
                    })
                    .bg(rgb(palette.input))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child("Config Store"),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_sm()
                            .text_color(if self.store_status.ready {
                                rgb(palette.success)
                            } else {
                                rgb(palette.danger)
                            })
                            .child(self.store_status.message.clone()),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(self.store_status.path.clone()),
                    ),
            )
    }

    fn active_session_row(
        &mut self,
        session: SessionInfo,
        display_name: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let session_id = session.id.clone();
        let rename_session_id = session.id.clone();
        let reconnect_session_id = session.id.clone();
        let close_session_id = session.id.clone();
        let menu_session_id = session.id.clone();
        let custom_color = self.session_tab_colors.get(&session.id).copied();
        let is_active = self.active_session_id.as_deref() == Some(session.id.as_str());
        let is_disconnected = self.is_session_disconnected(&session.id);
        let has_unread = self
            .terminal_views
            .get(&session.id)
            .is_some_and(|view| view.has_unread);
        let busy_action = self.active_session_busy_actions.get(&session.id).cloned();
        let is_busy = busy_action.is_some();
        let menu_open = self.active_session_menu_id.as_deref() == Some(session.id.as_str());
        let can_reconnect = !is_busy && self.pending_session_name.is_none();
        let can_disconnect = !is_busy && !is_disconnected;
        let accent = if let Some(custom_color) = custom_color {
            rgb(custom_color)
        } else if is_disconnected {
            rgb(palette.text_dimmed)
        } else if is_active {
            rgb(palette.success)
        } else if has_unread {
            rgb(palette.warning)
        } else {
            rgb(0x22c55e)
        };
        let row_bg = if let Some(custom_color) = custom_color {
            rgba((custom_color << 8) | if is_active { 0x22 } else { 0x12 })
        } else if is_active {
            rgb(palette.hover)
        } else {
            rgb(palette.surface)
        };
        let hover_bg = if let Some(custom_color) = custom_color {
            rgba((custom_color << 8) | if is_active { 0x30 } else { 0x20 })
        } else {
            rgb(palette.hover)
        };
        // Tauri ActiveSessions: full display name + type badge + full mono session id.
        let kind = session_kind_label(session.kind).to_ascii_uppercase();
        let full_id = session.id.clone();
        let id_preview = truncate_preview(&full_id, 42);
        let title = truncate_preview(&display_name, 32);
        let reconnect_label = if busy_action.as_deref() == Some("reconnect") {
            "Reconnecting…"
        } else {
            "Reconnect"
        };
        let disconnect_label = if busy_action.as_deref() == Some("disconnect") {
            "Disconnecting…"
        } else {
            "Disconnect"
        };

        div()
            .id(SharedString::from(format!(
                "active-session-row-{session_id}"
            )))
            .relative()
            .h(px(48.))
            .rounded_md()
            .px_2()
            .bg(row_bg)
            .when(is_disconnected, |this| this.opacity(0.5))
            .when(is_busy, |this| this.opacity(0.72))
            .cursor_pointer()
            .hover(move |this| this.bg(hover_bg))
            .child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().size(px(8.)).rounded_full().bg(accent).flex_none())
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .text_xs()
                                            .font_weight(FontWeight(600.))
                                            .text_color(rgb(palette.text))
                                            .overflow_hidden()
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .px_1()
                                            .py(px(1.))
                                            .rounded_sm()
                                            .bg(rgb(palette.hover))
                                            .text_size(px(10.))
                                            .font_weight(FontWeight(700.))
                                            .text_color(rgb(palette.text_dimmed))
                                            .child(kind),
                                    ),
                            )
                            .child(
                                div()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text_dimmed))
                                    .overflow_hidden()
                                    .child(id_preview),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_0()
                            .flex_none()
                            .child(session_action_svg_button(
                                palette,
                                format!("active-session-rename-{rename_session_id}"),
                                "icons/session/rename.svg",
                                !is_busy,
                                cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    if this
                                        .active_session_busy_actions
                                        .contains_key(&rename_session_id)
                                    {
                                        return;
                                    }
                                    this.active_session_menu_id = None;
                                    this.open_rename_session(rename_session_id.clone(), window, cx);
                                }),
                            ))
                            .child(
                                div()
                                    .relative()
                                    .child(session_action_svg_button(
                                        palette,
                                        format!("active-session-more-{menu_session_id}"),
                                        "icons/session/more.svg",
                                        !is_busy,
                                        cx.listener(move |this, _, _, cx| {
                                            cx.stop_propagation();
                                            if this
                                                .active_session_busy_actions
                                                .contains_key(&menu_session_id)
                                            {
                                                return;
                                            }
                                            if this.active_session_menu_id.as_deref()
                                                == Some(menu_session_id.as_str())
                                            {
                                                this.active_session_menu_id = None;
                                            } else {
                                                this.active_session_menu_id =
                                                    Some(menu_session_id.clone());
                                            }
                                            cx.notify();
                                        }),
                                    ))
                                    .when(menu_open, |this| {
                                        this.child(
                                            div()
                                                .id(SharedString::from(format!(
                                                    "active-session-menu-{session_id}"
                                                )))
                                                .absolute()
                                                .top(px(30.))
                                                .right(px(0.))
                                                .w(px(148.))
                                                .rounded_md()
                                                .border_1()
                                                .border_color(rgb(palette.border))
                                                .bg(rgb(palette.surface))
                                                .shadow_lg()
                                                .py_1()
                                                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                                    cx.stop_propagation();
                                                })
                                                .child(active_session_menu_item(
                                                    palette,
                                                    format!(
                                                        "active-session-reconnect-{reconnect_session_id}"
                                                    ),
                                                    reconnect_label,
                                                    "icons/session/reconnect.svg",
                                                    can_reconnect,
                                                    busy_action.as_deref() == Some("reconnect"),
                                                    false,
                                                    cx.listener(move |this, _, window, cx| {
                                                        cx.stop_propagation();
                                                        this.active_session_menu_id = None;
                                                        if this
                                                            .active_session_busy_actions
                                                            .contains_key(&reconnect_session_id)
                                                            || this.pending_session_name.is_some()
                                                        {
                                                            cx.notify();
                                                            return;
                                                        }
                                                        this.select_session(
                                                            reconnect_session_id.clone(),
                                                            cx,
                                                        );
                                                        this.reconnect_session(
                                                            reconnect_session_id.clone(),
                                                            window,
                                                            cx,
                                                        );
                                                    }),
                                                ))
                                                .child(active_session_menu_item(
                                                    palette,
                                                    format!(
                                                        "active-session-disconnect-{close_session_id}"
                                                    ),
                                                    disconnect_label,
                                                    "icons/session/disconnect.svg",
                                                    can_disconnect,
                                                    busy_action.as_deref() == Some("disconnect"),
                                                    true,
                                                    cx.listener(move |this, _, _, cx| {
                                                        cx.stop_propagation();
                                                        this.active_session_menu_id = None;
                                                        if this
                                                            .active_session_busy_actions
                                                            .contains_key(&close_session_id)
                                                            || this.is_session_disconnected(
                                                                &close_session_id,
                                                            )
                                                        {
                                                            cx.notify();
                                                            return;
                                                        }
                                                        this.disconnect_session(
                                                            close_session_id.clone(),
                                                            cx,
                                                        );
                                                    }),
                                                )),
                                        )
                                    }),
                            ),
                    ),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.active_session_menu_id = None;
                this.select_session(session_id.clone(), cx);
            }))
    }

    fn left_connections_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let mut rows = div().flex().flex_col().gap_2();
        if self.connections.is_empty() {
            rows = rows.child(empty_panel(
                "No saved connections imported yet.",
                self.theme_palette(),
            ));
        } else {
            for connection in self.connections.iter().take(8).cloned() {
                rows = rows.child(compact_connection_row(
                    palette,
                    &connection,
                    cx.listener({
                        let connection = connection.clone();
                        move |this, _, window, cx| {
                            this.start_saved_connection(connection.clone(), window, cx);
                        }
                    }),
                ));
            }
        }

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text_muted))
                                    .child("SAVED CONNECTIONS"),
                            )
                            .child(status_pill(
                                status_label(&self.terminal_status),
                                rgb(palette.accent),
                                rgb(palette.hover),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "left-connections-local",
                                "Local",
                                cx.listener(|this, _, window, cx| {
                                    this.start_local_session(window, cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "left-connections-refresh",
                                "Refresh",
                                cx.listener(|this, _, _, cx| {
                                    this.refresh_store_from_runtime();
                                    this.terminal_status = "connections refreshed".to_string();
                                    cx.notify();
                                }),
                            )),
                    ),
            )
            .child(rows)
    }

    fn left_network_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let mut rows = div().flex().flex_col().gap_2();
        if self.tunnels.is_empty() {
            rows = rows.child(empty_panel(
                "No SSH tunnels configured.",
                self.theme_palette(),
            ));
        } else {
            for tunnel in self.tunnels.iter().take(8).cloned() {
                let is_pending = self.pending_tunnels.iter().any(|id| id == &tunnel.id);
                let is_open = self.tunnel_manager.is_open(&tunnel.id).unwrap_or(false);
                rows = rows.child(compact_tunnel_row(
                    palette,
                    &tunnel,
                    is_open,
                    is_pending,
                    cx.listener({
                        let tunnel = tunnel.clone();
                        move |this, _, window, cx| {
                            this.start_tunnel_job(tunnel.clone(), window, cx);
                        }
                    }),
                    cx.listener({
                        let tunnel_id = tunnel.id.clone();
                        move |this, _, _, cx| {
                            this.close_tunnel_job(tunnel_id.clone(), cx);
                        }
                    }),
                ));
            }
        }

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text_muted))
                            .child("NETWORK"),
                    )
                    .child(capability_line(
                        palette,
                        "Configured Tunnels",
                        self.tunnels.len().to_string(),
                    ))
                    .child(capability_line(
                        palette,
                        "Pending",
                        self.pending_tunnels.len().to_string(),
                    ))
                    .child(capability_line(
                        palette,
                        "Active SSH",
                        if self.active_ssh_config.is_some() {
                            "ready"
                        } else {
                            "none"
                        },
                    )),
            )
            .child(rows)
    }

    fn left_transfers_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let mut jobs = div().flex().flex_col().gap_2();
        if self.transfer_jobs.is_empty() {
            jobs = jobs.child(empty_panel(
                "No SFTP transfer jobs yet.",
                self.theme_palette(),
            ));
        } else {
            for job in self.transfer_jobs.iter().rev().take(5) {
                jobs = jobs.child(compact_transfer_job_row(palette, job));
            }
        }

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text_muted))
                            .child("SFTP"),
                    )
                    .child(capability_line(
                        palette,
                        "SSH Session",
                        if self.active_ssh_config.is_some() {
                            "ready"
                        } else {
                            "none"
                        },
                    ))
                    .child(capability_line(
                        palette,
                        "Remote Path",
                        truncate_preview(&self.transfer_remote_path, 28),
                    ))
                    .child(capability_line(
                        palette,
                        "Duplicate Policy",
                        duplicate_policy_label(self.transfer_duplicate_policy),
                    ))
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "left-sftp-list",
                                "List",
                                cx.listener(|this, _, window, cx| {
                                    this.start_sftp_list_job(window, cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "left-sftp-download",
                                "Download",
                                cx.listener(|this, _, window, cx| {
                                    this.start_sftp_download_job(window, cx);
                                }),
                            )),
                    ),
            )
            .child(jobs)
    }

    fn left_settings_panel(&mut self, _cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text_muted))
                            .child("SETTINGS"),
                    )
                    .child(capability_line(
                        palette,
                        "Theme",
                        self.settings.theme.clone(),
                    ))
                    .child(capability_line(
                        palette,
                        "Terminal Font",
                        format!(
                            "{} {}",
                            self.settings.terminal_font_family, self.settings.terminal_font_size
                        ),
                    ))
                    .child(capability_line(
                        palette,
                        "Host Key Policy",
                        self.settings.host_key_policy.clone(),
                    ))
                    .child(capability_line(
                        palette,
                        "AI",
                        if self.ai_settings.enabled {
                            "enabled"
                        } else {
                            "disabled"
                        },
                    )),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(if self.store_status.ready {
                        rgb(palette.hover)
                    } else {
                        rgb(palette.hover)
                    })
                    .bg(rgb(palette.input))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text_muted))
                            .child("CONFIG STORE"),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_sm()
                            .text_color(if self.store_status.ready {
                                rgb(palette.success)
                            } else {
                                rgb(palette.danger)
                            })
                            .child(self.store_status.message.clone()),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(self.store_status.path.clone()),
                    ),
            )
    }

    fn nav_button(
        &self,
        label: &'static str,
        badge: String,
        item: NavItem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let selected = self.selected_nav == item;
        div()
            .id(SharedString::from(format!("nav-{label}")))
            .h(px(34.))
            .px_3()
            .flex()
            .items_center()
            .rounded_sm()
            .cursor_pointer()
            .text_xs()
            .justify_between()
            .gap_2()
            .when(selected, |this| {
                this.bg(rgb(palette.hover)).text_color(rgb(0xffffff))
            })
            .when(!selected, |this| {
                this.text_color(rgb(palette.text_muted))
                    .hover(|hover| hover.bg(rgb(palette.hover)).text_color(rgb(0xffffff)))
            })
            .child(div().min_w_0().overflow_hidden().child(label))
            .child(
                div()
                    .flex_none()
                    .rounded_sm()
                    .bg(if selected {
                        rgb(palette.hover)
                    } else {
                        rgb(palette.input)
                    })
                    .px_2()
                    .py_1()
                    .font_family("JetBrains Mono")
                    .text_size(px(10.))
                    .text_color(if selected {
                        rgb(palette.accent)
                    } else {
                        rgb(palette.text_muted)
                    })
                    .child(badge),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(item, cx)))
    }
}

impl NyaTermApp {
    pub(in crate::ui::view) fn security_auth_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active_tab = self.security_auth_tab;
        let key_count = self.connection_ssh_keys.len();
        let password_count = self.connection_saved_passwords.len();
        let credential_count = self.connection_saved_credentials.len();
        let otp_count = self.connection_otp_entries.len();
        let master = if self.settings.has_master_password {
            "configured"
        } else {
            "not set"
        };
        let palette = self.theme_palette();

        let mut body = div()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap_1()
            .p_2();

        match active_tab {
            SecurityAuthTab::Keys => {
                if let Some(editor) = self.security_key_editor.clone() {
                    body = body.child(self.security_key_editor_view(editor, cx));
                } else if self.connection_ssh_keys.is_empty() {
                    body = body.child(empty_panel(
                        "No SSH keys yet. Add a private key to use key auth.",
                        self.theme_palette(),
                    ));
                } else {
                    for key in self.connection_ssh_keys.clone() {
                        let key_id = key.id.clone();
                        let edit_id = key.id.clone();
                        let delete_id = key.id.clone();
                        body = body.child(
                            // Tauri security-auth: dense single-row list items + trailing actions.
                            div()
                                .h(px(42.))
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.input))
                                .px_2()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap(px(1.))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .min_w_0()
                                                        .flex_1()
                                                        .text_xs()
                                                        .font_weight(FontWeight(600.))
                                                        .text_color(rgb(palette.text))
                                                        .overflow_hidden()
                                                        .child(truncate_preview(&key.name, 28)),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(10.))
                                                        .text_color(if key.has_key_data {
                                                            rgb(palette.success)
                                                        } else {
                                                            rgb(palette.text_muted)
                                                        })
                                                        .child(if key.has_key_data {
                                                            "ready"
                                                        } else {
                                                            "empty"
                                                        }),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(rgb(palette.text_dimmed))
                                                .overflow_hidden()
                                                .child(format!(
                                                    "{} · cert {}",
                                                    compact_id(&key_id),
                                                    if key.has_cert_data { "yes" } else { "no" }
                                                )),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(small_button(
                                            palette,
                                            format!("security-key-edit-{key_id}"),
                                            "Edit",
                                            cx.listener(move |this, _, window, cx| {
                                                this.open_security_key_editor(
                                                    Some(edit_id.clone()),
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            format!("security-key-del-{key_id}"),
                                            "Del",
                                            cx.listener(move |this, _, _, cx| {
                                                this.request_delete_security_key(
                                                    delete_id.clone(),
                                                    cx,
                                                );
                                            }),
                                        )),
                                ),
                        );
                    }
                }
            }
            SecurityAuthTab::Passwords => {
                if let Some(editor) = self.security_password_editor.clone() {
                    body = body.child(self.security_password_editor_view(editor, cx));
                } else if self.connection_saved_passwords.is_empty() {
                    body = body.child(empty_panel("No saved passwords yet.", self.theme_palette()));
                } else {
                    for entry in self.connection_saved_passwords.clone() {
                        let id = entry.id.clone();
                        let edit_id = entry.id.clone();
                        let delete_id = entry.id.clone();
                        let reveal_id = entry.id.clone();
                        let copy_id = entry.id.clone();
                        let is_revealed = self.security_revealed_passwords.contains_key(&entry.id);
                        let revealed_value =
                            self.security_revealed_passwords.get(&entry.id).cloned();
                        // Tauri: masked until revealed; revealed shows secret + Copy.
                        let secret_line = if is_revealed {
                            revealed_value
                                .clone()
                                .filter(|v| !v.is_empty())
                                .unwrap_or_else(|| "empty".to_string())
                        } else if entry.has_password {
                            String::new()
                        } else {
                            "empty".to_string()
                        };
                        body = body.child(
                            div()
                                .min_h(px(42.))
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.input))
                                .px_2()
                                .py_1()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap(px(1.))
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight(600.))
                                                .text_color(rgb(palette.text))
                                                .overflow_hidden()
                                                .child(truncate_preview(&entry.name, 28)),
                                        )
                                        .when(is_revealed, |this| {
                                            this.child(
                                                div()
                                                    .flex()
                                                    .items_start()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .min_w_0()
                                                            .flex_1()
                                                            .font_family("JetBrains Mono")
                                                            .text_size(px(11.))
                                                            .text_color(rgb(palette.text_muted))
                                                            .child(truncate_preview(
                                                                &secret_line,
                                                                36,
                                                            )),
                                                    )
                                                    .when(
                                                        revealed_value
                                                            .as_ref()
                                                            .is_some_and(|v| !v.is_empty()),
                                                        |this| {
                                                            this.child(small_button(
                                                                palette,
                                                                format!("security-pw-copy-{id}"),
                                                                "Copy",
                                                                cx.listener(
                                                                    move |this, _, window, cx| {
                                                                        this.copy_security_password(
                                                                            copy_id.clone(),
                                                                            window,
                                                                            cx,
                                                                        );
                                                                    },
                                                                ),
                                                            ))
                                                        },
                                                    ),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(small_button(
                                            palette,
                                            format!("security-pw-show-{id}"),
                                            if is_revealed { "Hide" } else { "Show" },
                                            cx.listener(move |this, _, window, cx| {
                                                this.reveal_security_password(
                                                    reveal_id.clone(),
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            format!("security-pw-edit-{id}"),
                                            "Edit",
                                            cx.listener(move |this, _, window, cx| {
                                                this.open_security_password_editor(
                                                    Some(edit_id.clone()),
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            format!("security-pw-del-{id}"),
                                            "Del",
                                            cx.listener(move |this, _, _, cx| {
                                                this.request_delete_security_password(
                                                    delete_id.clone(),
                                                    cx,
                                                );
                                            }),
                                        )),
                                ),
                        );
                    }
                }
            }
            SecurityAuthTab::Credentials => {
                if let Some(editor) = self.security_credential_editor.clone() {
                    body = body.child(self.security_credential_editor_view(editor, cx));
                } else if self.connection_saved_credentials.is_empty() {
                    body = body.child(empty_panel(
                        "No autofill credentials yet.",
                        self.theme_palette(),
                    ));
                } else {
                    for entry in self.connection_saved_credentials.clone() {
                        let id = entry.id.clone();
                        let edit_id = entry.id.clone();
                        let delete_id = entry.id.clone();
                        let reveal_id = entry.id.clone();
                        let toggle_id = entry.id.clone();
                        let is_revealed =
                            self.security_revealed_credentials.contains_key(&entry.id);
                        let secret = self
                            .security_revealed_credentials
                            .get(&entry.id)
                            .cloned()
                            .unwrap_or_default();
                        body = body.child(
                            div()
                                .min_h(px(48.))
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.input))
                                .px_2()
                                .py_1()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap(px(1.))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .min_w_0()
                                                        .flex_1()
                                                        .text_xs()
                                                        .font_weight(FontWeight(600.))
                                                        .text_color(rgb(palette.text))
                                                        .overflow_hidden()
                                                        .child(truncate_preview(&entry.name, 24)),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(10.))
                                                        .text_color(if entry.enabled {
                                                            rgb(palette.success)
                                                        } else {
                                                            rgb(palette.text_muted)
                                                        })
                                                        .child(if entry.enabled {
                                                            "on"
                                                        } else {
                                                            "off"
                                                        }),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(rgb(palette.text_dimmed))
                                                .overflow_hidden()
                                                .child(if is_revealed {
                                                    format!(
                                                        "{} · {}",
                                                        truncate_preview(&entry.username, 18),
                                                        truncate_preview(&secret, 16)
                                                    )
                                                } else {
                                                    truncate_preview(&entry.username, 28)
                                                }),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(small_button(
                                            palette,
                                            format!("security-cred-toggle-{id}"),
                                            if entry.enabled { "Off" } else { "On" },
                                            cx.listener(move |this, _, _, cx| {
                                                this.toggle_security_credential_list_enabled(
                                                    toggle_id.clone(),
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            format!("security-cred-show-{id}"),
                                            if is_revealed { "Hide" } else { "Show" },
                                            cx.listener(move |this, _, window, cx| {
                                                this.reveal_security_credential_password(
                                                    reveal_id.clone(),
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            format!("security-cred-edit-{id}"),
                                            "Edit",
                                            cx.listener(move |this, _, window, cx| {
                                                this.open_security_credential_editor(
                                                    Some(edit_id.clone()),
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            format!("security-cred-del-{id}"),
                                            "Del",
                                            cx.listener(move |this, _, _, cx| {
                                                this.request_delete_security_credential(
                                                    delete_id.clone(),
                                                    cx,
                                                );
                                            }),
                                        )),
                                ),
                        );
                    }
                }
            }
            SecurityAuthTab::Otp => {
                if let Some(editor) = self.security_otp_editor.clone() {
                    body = body.child(self.security_otp_editor_view(editor, cx));
                } else if self.connection_otp_entries.is_empty() {
                    body = body.child(empty_panel(
                        "No OTP accounts yet. Add TOTP/HOTP for auto-fill.",
                        self.theme_palette(),
                    ));
                } else {
                    for entry in self.connection_otp_entries.clone() {
                        let otp_id = entry.id.clone();
                        let edit_id = entry.id.clone();
                        let delete_id = entry.id.clone();
                        let code_id = entry.id.clone();
                        let title = if !entry.issuer.trim().is_empty()
                            || !entry.username.trim().is_empty()
                        {
                            format!(
                                "{}{}",
                                entry.issuer,
                                if entry.username.trim().is_empty() {
                                    String::new()
                                } else if entry.issuer.trim().is_empty() {
                                    entry.username.clone()
                                } else {
                                    format!(" ({})", entry.username)
                                }
                            )
                        } else {
                            compact_id(&entry.id)
                        };
                        let code_raw = self
                            .security_otp_codes
                            .get(&entry.id)
                            .cloned()
                            .unwrap_or_else(|| "------".to_string());
                        let code_display = format_otp_code_display(&code_raw);
                        let is_totp = entry.otp_type.eq_ignore_ascii_case("totp");
                        let period = entry.period.max(1);
                        let remaining = if is_totp {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0);
                            period - (now % period)
                        } else {
                            0
                        };
                        let meta = if is_totp {
                            format!(
                                "{} · {} · {}d · {remaining}s left",
                                entry.otp_type.to_uppercase(),
                                entry.algorithm,
                                entry.digits,
                            )
                        } else {
                            format!(
                                "{} · {} · {}d · ctr {}",
                                entry.otp_type.to_uppercase(),
                                entry.algorithm,
                                entry.digits,
                                entry.counter,
                            )
                        };
                        let copy_id = entry.id.clone();
                        body = body.child(
                            div()
                                .h(px(52.))
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.input))
                                .px_2()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap(px(1.))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .min_w_0()
                                                        .flex_1()
                                                        .text_xs()
                                                        .font_weight(FontWeight(600.))
                                                        .text_color(rgb(palette.text))
                                                        .overflow_hidden()
                                                        .child(truncate_preview(&title, 24)),
                                                )
                                                .child(
                                                    div()
                                                        .font_family("JetBrains Mono")
                                                        .text_sm()
                                                        .font_weight(FontWeight(700.))
                                                        .text_color(rgb(if code_raw == "------" {
                                                            palette.text_muted
                                                        } else {
                                                            palette.accent
                                                        }))
                                                        .child(code_display),
                                                )
                                                .when(is_totp && code_raw != "------", |this| {
                                                    this.child(
                                                        div()
                                                            .text_size(px(10.))
                                                            .font_family("JetBrains Mono")
                                                            .text_color(rgb(if remaining <= 5 {
                                                                palette.warning
                                                            } else {
                                                                palette.text_dimmed
                                                            }))
                                                            .child(format!("{remaining}s")),
                                                    )
                                                }),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(rgb(palette.text_dimmed))
                                                .overflow_hidden()
                                                .child(meta),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(small_button(
                                            palette,
                                            format!("security-otp-code-{otp_id}"),
                                            if is_totp { "Gen" } else { "Next" },
                                            cx.listener(move |this, _, window, cx| {
                                                this.generate_security_otp_code(
                                                    code_id.clone(),
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            format!("security-otp-copy-{otp_id}"),
                                            "Copy",
                                            cx.listener(move |this, _, window, cx| {
                                                this.copy_security_otp_code(
                                                    copy_id.clone(),
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            format!("security-otp-edit-{otp_id}"),
                                            "Edit",
                                            cx.listener(move |this, _, window, cx| {
                                                this.open_security_otp_editor(
                                                    Some(edit_id.clone()),
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            format!("security-otp-del-{otp_id}"),
                                            "Del",
                                            cx.listener(move |this, _, _, cx| {
                                                this.request_delete_security_otp(
                                                    delete_id.clone(),
                                                    cx,
                                                );
                                            }),
                                        )),
                                ),
                        );
                    }
                }
            }
        }

        if let Some(confirm) = self.security_delete_confirm.clone() {
            body = body.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.danger))
                    .bg(rgb(palette.hover))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.danger))
                            .child(format!("Delete {}?", confirm.label)),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "security-delete-confirm",
                                "Delete",
                                cx.listener(|this, _, _, cx| {
                                    this.confirm_security_delete(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "security-delete-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_security_delete(cx);
                                }),
                            )),
                    ),
            );
        }

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(rgb(palette.surface))
            .child(
                div()
                    .px_3()
                    .pt_3()
                    .pb_2()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.section_header))
                    .flex()
                    .flex_col()
                    .gap_2()
                    // Tauri SecurityAuthPanel: full-width 4-col segment tabs under PanelHeader.
                    .child(
                        div()
                            .h(px(32.))
                            .w_full()
                            .rounded_md()
                            .bg(rgb(palette.surface_elevated))
                            .p(px(2.))
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .child(self.security_tab_chip(SecurityAuthTab::Keys, cx))
                            .child(self.security_tab_chip(SecurityAuthTab::Passwords, cx))
                            .child(self.security_tab_chip(SecurityAuthTab::Otp, cx))
                            .child(self.security_tab_chip(SecurityAuthTab::Credentials, cx)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text_dimmed))
                                    .child(self.security_status.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(rgb(palette.text_dimmed))
                                            .child(format!(
                                                "MP {master} · K{key_count}/P{password_count}/C{credential_count}/O{otp_count}"
                                            )),
                                    )
                                    .when(
                                        self.security_auth_tab == SecurityAuthTab::Otp
                                            && !self.connection_otp_entries.is_empty(),
                                        |this| {
                                            this.child(small_button(
                                                palette,
                                                "security-otp-refresh-all",
                                                "Refresh",
                                                cx.listener(|this, _, window, cx| {
                                                    this.refresh_visible_security_otp_codes(
                                                        window, cx,
                                                    );
                                                }),
                                            ))
                                        },
                                    )
                                    .child(small_button(palette, 
                                        "security-add-item",
                                        "Add",
                                        cx.listener(|this, _, window, cx| {
                                            match this.security_auth_tab {
                                                SecurityAuthTab::Keys => {
                                                    this.open_security_key_editor(
                                                        None, window, cx,
                                                    );
                                                }
                                                SecurityAuthTab::Passwords => {
                                                    this.open_security_password_editor(
                                                        None, window, cx,
                                                    );
                                                }
                                                SecurityAuthTab::Credentials => {
                                                    this.open_security_credential_editor(
                                                        None, window, cx,
                                                    );
                                                }
                                                SecurityAuthTab::Otp => {
                                                    this.open_security_otp_editor(
                                                        None, window, cx,
                                                    );
                                                }
                                            }
                                        }),
                                    )),
                            ),
                    ),
            )
            .child(body)
            .child(self.security_secret_footer(cx))
            .when(self.security_unlock_prompt_open, |this| {
                this.child(self.security_unlock_prompt(cx))
            })
    }

    fn security_secret_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let unlocked = !self.security_secrets_locked();
        let has_master = self.settings.has_master_password;
        let palette = self.theme_palette();
        div()
            .flex_none()
            .border_t_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.section_header))
            .px_2()
            .py_2()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(if unlocked {
                        rgb(palette.success)
                    } else {
                        rgb(palette.warning)
                    })
                    .child(if !has_master {
                        "Secrets open (no master password)"
                    } else if unlocked {
                        "Secrets unlocked"
                    } else {
                        "Secrets locked"
                    }),
            )
            .child(div().flex().items_center().gap_1().child(small_button(
                palette,
                "security-secrets-toggle",
                if unlocked && has_master {
                    "Lock"
                } else if unlocked {
                    "Open"
                } else {
                    "Unlock"
                },
                cx.listener(|this, _, window, cx| {
                    if this.security_secrets_locked() {
                        this.open_security_unlock_prompt(window, cx);
                    } else if this.settings.has_master_password {
                        this.lock_security_secrets(cx);
                    } else {
                        this.security_status =
                            "set a master password in Settings to lock secrets".to_string();
                        cx.notify();
                    }
                }),
            )))
    }

    fn security_unlock_prompt(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let draft = if self.security_unlock_draft.is_empty() {
            " ".to_string()
        } else {
            "•".repeat(self.security_unlock_draft.chars().count().min(32))
        };
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x0d1117cc))
            .child(
                div()
                    .w(px(280.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .track_focus(&self.security_unlock_focus)
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        this.handle_security_unlock_key_down(event, cx);
                    }))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child("Unlock Secrets"),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_muted))
                            .child("Enter master password to view/copy secrets."),
                    )
                    .child(
                        div()
                            .id(SharedString::from("security-unlock-input"))
                            .h(px(32.))
                            .px_2()
                            .flex()
                            .items_center()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(palette.accent))
                            .bg(rgb(palette.input))
                            .font_family("JetBrains Mono")
                            .text_xs()
                            .text_color(rgb(palette.text))
                            .child(draft),
                    )
                    .when_some(self.security_unlock_error.clone(), |this, error| {
                        this.child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(palette.danger))
                                .child(error),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "security-unlock-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.close_security_unlock_prompt(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "security-unlock-submit",
                                "Unlock",
                                cx.listener(|this, _, _, cx| {
                                    this.submit_security_unlock(cx);
                                }),
                            )),
                    ),
            )
    }

    fn security_tab_chip(&self, tab: SecurityAuthTab, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.security_auth_tab == tab;
        let palette = self.theme_palette();
        // Tauri TabsTrigger text-xs inside h-8 grid segment.
        div()
            .id(SharedString::from(format!("security-tab-{}", tab.label())))
            .h(px(28.))
            .flex_1()
            .px_1()
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .cursor_pointer()
            .text_size(px(11.))
            .font_weight(FontWeight(if selected { 600. } else { 500. }))
            .text_color(if selected {
                rgb(palette.text)
            } else {
                rgb(palette.text_muted)
            })
            .bg(if selected {
                rgb(palette.input)
            } else {
                rgb(palette.surface_elevated)
            })
            .hover(move |this| {
                this.bg(if selected {
                    rgb(palette.input)
                } else {
                    rgb(palette.hover)
                })
                .text_color(rgb(palette.text))
            })
            .child(tab.label())
            .on_click(cx.listener(move |this, _, _, cx| {
                this.set_security_auth_tab(tab, cx);
            }))
    }

    fn security_key_editor_view(
        &mut self,
        editor: SecurityKeyEditorState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = if editor.id.is_some() {
            "Edit SSH Key"
        } else {
            "New SSH Key"
        };
        let key_path_label = if !editor.key_file_path.trim().is_empty() {
            truncate_preview(&editor.key_file_path, 36)
        } else if editor.has_key_data {
            "loaded (unchanged)".to_string()
        } else {
            "no key file selected".to_string()
        };
        let cert_path_label = if !editor.cert_file_path.trim().is_empty() {
            truncate_preview(&editor.cert_file_path, 36)
        } else if editor.has_cert_data {
            "loaded (unchanged)".to_string()
        } else {
            "optional certificate".to_string()
        };
        let passphrase_display = if editor.passphrase.is_empty() {
            " ".to_string()
        } else {
            "•".repeat(editor.passphrase.chars().count().min(24))
        };

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.bg))
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .track_focus(&self.security_key_editor_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_security_key_editor_key_down(event, window, cx);
            }))
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight(800.))
                    .text_color(rgb(palette.text))
                    .child(title),
            )
            .child(security_editor_field(
                palette,
                "security-key-name",
                "Name",
                if editor.name.is_empty() {
                    " ".to_string()
                } else {
                    editor.name.clone()
                },
                editor.focused_field == SecurityKeyEditorField::Name,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_key_field(SecurityKeyEditorField::Name, window, cx);
                }),
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_muted))
                            .child("Private Key"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .id(SharedString::from("security-key-path"))
                                    .flex_1()
                                    .min_w_0()
                                    .h(px(28.))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(
                                        if editor.focused_field == SecurityKeyEditorField::KeyPath {
                                            rgb(palette.accent)
                                        } else {
                                            rgb(palette.border)
                                        },
                                    )
                                    .bg(rgb(palette.input))
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text))
                                    .cursor_pointer()
                                    .child(key_path_label)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.focus_security_key_field(
                                            SecurityKeyEditorField::KeyPath,
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(small_button(
                                palette,
                                "security-key-browse",
                                "Browse",
                                cx.listener(|this, _, window, cx| {
                                    this.pick_security_key_file(false, window, cx);
                                }),
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_muted))
                            .child("Certificate"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .id(SharedString::from("security-cert-path"))
                                    .flex_1()
                                    .min_w_0()
                                    .h(px(28.))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(
                                        if editor.focused_field == SecurityKeyEditorField::CertPath
                                        {
                                            rgb(palette.accent)
                                        } else {
                                            rgb(palette.border)
                                        },
                                    )
                                    .bg(rgb(palette.input))
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text))
                                    .cursor_pointer()
                                    .child(cert_path_label)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.focus_security_key_field(
                                            SecurityKeyEditorField::CertPath,
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(small_button(
                                palette,
                                "security-cert-browse",
                                "Browse",
                                cx.listener(|this, _, window, cx| {
                                    this.pick_security_key_file(true, window, cx);
                                }),
                            )),
                    ),
            )
            .child(security_editor_field(
                palette,
                "security-key-passphrase",
                "Passphrase",
                passphrase_display,
                editor.focused_field == SecurityKeyEditorField::Passphrase,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_key_field(SecurityKeyEditorField::Passphrase, window, cx);
                }),
            ))
            .when_some(editor.error.clone(), |this, error| {
                this.child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette.danger))
                        .child(error),
                )
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(small_button(
                        palette,
                        "security-key-save",
                        "Save",
                        cx.listener(|this, _, window, cx| {
                            this.save_security_key_editor(window, cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "security-key-cancel",
                        "Cancel",
                        cx.listener(|this, _, _, cx| {
                            this.close_security_key_editor(cx);
                        }),
                    )),
            )
    }

    fn security_otp_editor_view(
        &mut self,
        editor: SecurityOtpEditorState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = if editor.id.is_some() {
            "Edit OTP"
        } else {
            "New OTP"
        };
        let secret_display = if editor.secret.is_empty() {
            if editor.has_secret {
                "loaded (unchanged)".to_string()
            } else {
                " ".to_string()
            }
        } else {
            "•".repeat(editor.secret.chars().count().min(24))
        };

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.bg))
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .track_focus(&self.security_otp_editor_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_security_otp_editor_key_down(event, window, cx);
            }))
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight(800.))
                    .text_color(rgb(palette.text))
                    .child(title),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(security_type_chip(
                        palette,
                        "TOTP",
                        editor.otp_type != "hotp",
                        cx.listener(|this, _, _, cx| {
                            this.set_security_otp_type("totp", cx);
                        }),
                    ))
                    .child(security_type_chip(
                        palette,
                        "HOTP",
                        editor.otp_type == "hotp",
                        cx.listener(|this, _, _, cx| {
                            this.set_security_otp_type("hotp", cx);
                        }),
                    ))
                    .child(div().flex_1())
                    .child(
                        div()
                            .id(SharedString::from("security-otp-algo"))
                            .h(px(22.))
                            .px_2()
                            .flex()
                            .items_center()
                            .rounded_sm()
                            .text_size(px(10.))
                            .font_weight(FontWeight(700.))
                            .cursor_pointer()
                            .text_color(rgb(palette.text))
                            .bg(rgb(palette.surface_elevated))
                            .hover(|this| this.bg(rgb(palette.border)))
                            .child(editor.algorithm.clone())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cycle_security_otp_algorithm(cx);
                            })),
                    ),
            )
            .child(security_editor_field(
                palette,
                "security-otp-issuer",
                "Issuer",
                if editor.issuer.is_empty() {
                    " ".to_string()
                } else {
                    editor.issuer.clone()
                },
                editor.focused_field == SecurityOtpEditorField::Issuer,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_otp_field(SecurityOtpEditorField::Issuer, window, cx);
                }),
            ))
            .child(security_editor_field(
                palette,
                "security-otp-username",
                "Account",
                if editor.username.is_empty() {
                    " ".to_string()
                } else {
                    editor.username.clone()
                },
                editor.focused_field == SecurityOtpEditorField::Username,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_otp_field(SecurityOtpEditorField::Username, window, cx);
                }),
            ))
            .child(security_editor_field(
                palette,
                "security-otp-secret",
                "Secret",
                secret_display,
                editor.focused_field == SecurityOtpEditorField::Secret,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_otp_field(SecurityOtpEditorField::Secret, window, cx);
                }),
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(3)
                    .gap_2()
                    .child(security_editor_field(
                        palette,
                        "security-otp-digits",
                        "Digits",
                        if editor.digits.is_empty() {
                            " ".to_string()
                        } else {
                            editor.digits.clone()
                        },
                        editor.focused_field == SecurityOtpEditorField::Digits,
                        cx.listener(|this, _, window, cx| {
                            this.focus_security_otp_field(
                                SecurityOtpEditorField::Digits,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(security_editor_field(
                        palette,
                        "security-otp-period",
                        "Period",
                        if editor.period.is_empty() {
                            " ".to_string()
                        } else {
                            editor.period.clone()
                        },
                        editor.focused_field == SecurityOtpEditorField::Period,
                        cx.listener(|this, _, window, cx| {
                            this.focus_security_otp_field(
                                SecurityOtpEditorField::Period,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(security_editor_field(
                        palette,
                        "security-otp-counter",
                        "Counter",
                        if editor.counter.is_empty() {
                            " ".to_string()
                        } else {
                            editor.counter.clone()
                        },
                        editor.focused_field == SecurityOtpEditorField::Counter,
                        cx.listener(|this, _, window, cx| {
                            this.focus_security_otp_field(
                                SecurityOtpEditorField::Counter,
                                window,
                                cx,
                            );
                        }),
                    )),
            )
            .when_some(editor.error.clone(), |this, error| {
                this.child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette.danger))
                        .child(error),
                )
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(small_button(
                        palette,
                        "security-otp-save",
                        "Save",
                        cx.listener(|this, _, window, cx| {
                            this.save_security_otp_editor(window, cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "security-otp-cancel",
                        "Cancel",
                        cx.listener(|this, _, _, cx| {
                            this.close_security_otp_editor(cx);
                        }),
                    )),
            )
    }

    pub(in crate::ui::view) fn sync_backup_history_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Tauri SyncBackupHistoryPanel:
        // shared PanelHeader + status strip + optional conflict card + dense history list.
        let provider = configured_cloud_sync_provider(&self.cloud_sync_settings);
        let provider_label = format_cloud_provider(&provider);
        let enabled = self.cloud_sync_settings.enabled;
        let state = if !enabled {
            "disabled"
        } else if self.cloud_sync_conflict.is_some() {
            "conflict"
        } else if self.cloud_sync_status.to_ascii_lowercase().contains("fail") {
            "failed"
        } else if self.cloud_sync_status.to_ascii_lowercase().contains("push")
            || self.cloud_sync_status.to_ascii_lowercase().contains("pull")
            || self
                .cloud_sync_status
                .to_ascii_lowercase()
                .contains("running")
        {
            "running"
        } else if self
            .cloud_sync_status
            .to_ascii_lowercase()
            .contains("success")
            || self
                .cloud_sync_status
                .to_ascii_lowercase()
                .contains("synced")
            || self
                .cloud_sync_status
                .to_ascii_lowercase()
                .contains("ready")
        {
            "success"
        } else {
            "idle"
        };
        let state_label = match state {
            "disabled" => "Disabled",
            "conflict" => "Conflict",
            "failed" => "Failed",
            "running" => "Running",
            "success" => "Success",
            _ => "Idle",
        };
        let status_message = self.cloud_sync_status.clone();
        let history = self.cloud_sync_history.clone();
        let expanded = self.cloud_sync_history_expanded.clone();
        let conflict = self.cloud_sync_conflict.clone();

        let mut rows = div().flex().flex_col();
        if history.is_empty() {
            rows = rows.child(
                div()
                    .py_6()
                    .px_3()
                    .text_center()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child("No sync history yet"),
            );
        } else {
            for entry in history {
                let entry_id = entry.id.clone();
                let is_open = expanded.contains(&entry_id);
                let copy_message = entry.message.clone();
                rows = rows.child(cloud_sync_history_row(
                    palette,
                    entry,
                    is_open,
                    cx.listener(move |this, _, _, cx| {
                        this.toggle_cloud_sync_history_details(&entry_id, cx);
                    }),
                    cx.listener(move |this, _, _, cx| {
                        if copy_message.trim().is_empty() {
                            this.terminal_status = "history entry has no message".to_string();
                        } else {
                            cx.write_to_clipboard(ClipboardItem::new_string(copy_message.clone()));
                            this.terminal_status = "sync history message copied".to_string();
                        }
                        cx.notify();
                    }),
                ));
            }
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(palette.surface))
            .child(
                div()
                    .flex_none()
                    .h(px(36.))
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.section_header))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .min_w_0()
                            .flex_1()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .size(px(8.))
                                    .rounded_full()
                                    .flex_none()
                                    .bg(cloud_sync_status_dot_color(palette, state)),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.text_muted))
                                    .child("State"),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(600.))
                                    .text_color(cloud_sync_status_text_color(palette, state))
                                    .overflow_hidden()
                                    .child(state_label),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.border))
                                    .child("·"),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.text_muted))
                                    .overflow_hidden()
                                    .child(provider_label),
                            )
                            .child(toolbar_svg_button(
                                palette,
                                SharedString::from("sync-history-refresh"),
                                "icons/fe/refresh.svg",
                                cx.listener(|this, _, _, cx| {
                                    this.refresh_cloud_sync_history();
                                    this.terminal_status =
                                        "cloud sync history refreshed".to_string();
                                    cx.notify();
                                }),
                            )),
                    )
                    .when(
                        !status_message.trim().is_empty() && conflict.is_none(),
                        |this| {
                            this.child(
                                div()
                                    .mt_1()
                                    .pl_4()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(truncate_preview(&status_message, 120)),
                            )
                        },
                    ),
            )
            .when_some(conflict, |this, conflict| {
                this.child(
                    div()
                        .flex_none()
                        .m_2()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(palette.warning))
                        .bg(rgb(palette.input))
                        .child(
                            div()
                                .px_3()
                                .py_2()
                                .border_b_1()
                                .border_color(rgb(palette.warning))
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(palette.warning))
                                        .child("Sync conflict"),
                                ),
                        )
                        .child(
                            div()
                                .px_3()
                                .py_2()
                                .text_size(px(11.))
                                .text_color(rgb(palette.text))
                                .child(conflict.message.clone()),
                        )
                        .child(
                            div().px_3().pb_2().grid().grid_cols(1).gap_2().child(
                                div()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.input))
                                    .px_2()
                                    .py_1()
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(rgb(palette.text_muted))
                                            .child("Provider"),
                                    )
                                    .child(
                                        div()
                                            .mt_0()
                                            .font_family("JetBrains Mono")
                                            .text_size(px(11.))
                                            .text_color(rgb(palette.text))
                                            .child(format_cloud_provider(&conflict.provider)),
                                    ),
                            ),
                        )
                        .child(
                            div()
                                .px_3()
                                .pb_3()
                                .flex()
                                .gap_2()
                                .child(small_button(
                                    palette,
                                    "sync-panel-force-pull",
                                    "Use remote",
                                    cx.listener({
                                        let provider_action = conflict.provider_action;
                                        move |this, _, _, cx| {
                                            this.prompt_cloud_sync_force_pull(provider_action, cx);
                                        }
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    "sync-panel-force-push",
                                    "Use local",
                                    cx.listener({
                                        let provider_action = conflict.provider_action;
                                        move |this, _, _, cx| {
                                            this.prompt_cloud_sync_force_push(provider_action, cx);
                                        }
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    "sync-panel-conflict-dismiss",
                                    "Dismiss",
                                    cx.listener(|this, _, _, cx| {
                                        this.dismiss_cloud_sync_conflict(cx);
                                    }),
                                )),
                        ),
                )
            })
            .child(
                div()
                    .flex_none()
                    .px_3()
                    .py_1()
                    .border_b_1()
                    .border_color(rgb(palette.surface_elevated))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text_dimmed))
                            .child("HISTORY"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                palette,
                                "sync-panel-push",
                                "Push",
                                cx.listener(|this, _, _, cx| {
                                    if configured_cloud_sync_provider(&this.cloud_sync_settings)
                                        != "local_directory"
                                    {
                                        this.prompt_provider_cloud_sync_push(cx);
                                    } else {
                                        this.prompt_local_cloud_sync_push(cx);
                                    }
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "sync-panel-pull",
                                "Pull",
                                cx.listener(|this, _, _, cx| {
                                    if configured_cloud_sync_provider(&this.cloud_sync_settings)
                                        != "local_directory"
                                    {
                                        this.prompt_provider_cloud_sync_pull(cx);
                                    } else {
                                        this.prompt_local_cloud_sync_pull(cx);
                                    }
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "sync-panel-settings",
                                "Settings",
                                cx.listener(|this, _, _, cx| {
                                    this.settings_active_tab = SettingsTab::SyncBackup;
                                    this.open_page(NavItem::Settings, cx);
                                }),
                            )),
                    ),
            )
            .child(
                div()
                    .id(SharedString::from("sync-backup-history-list"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(rows),
            )
    }

    fn security_password_editor_view(
        &mut self,
        editor: SecurityPasswordEditorState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = if editor.id.is_some() {
            "Edit Password"
        } else {
            "New Password"
        };
        let password_display = if editor.password.is_empty() {
            if editor.has_password {
                "loaded (unchanged)".to_string()
            } else {
                " ".to_string()
            }
        } else if editor.show_password {
            truncate_preview(&editor.password, 48)
        } else {
            "•".repeat(editor.password.chars().count().min(24))
        };
        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.bg))
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .track_focus(&self.security_password_editor_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_security_password_editor_key_down(event, window, cx);
            }))
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight(800.))
                    .text_color(rgb(palette.text))
                    .child(title),
            )
            .child(security_editor_field(
                palette,
                "security-pw-name",
                "Name",
                if editor.name.is_empty() {
                    " ".to_string()
                } else {
                    editor.name.clone()
                },
                editor.focused_field == SecurityPasswordEditorField::Name,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_password_field(
                        SecurityPasswordEditorField::Name,
                        window,
                        cx,
                    );
                }),
            ))
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap_1()
                    .child(div().min_w_0().flex_1().child(security_editor_field(
                        palette,
                        "security-pw-value",
                        "Password",
                        password_display,
                        editor.focused_field == SecurityPasswordEditorField::Password,
                        cx.listener(|this, _, window, cx| {
                            this.focus_security_password_field(
                                SecurityPasswordEditorField::Password,
                                window,
                                cx,
                            );
                        }),
                    )))
                    .child(small_button(
                        palette,
                        "security-pw-toggle-vis",
                        if editor.show_password { "Hide" } else { "Show" },
                        cx.listener(|this, _, _, cx| {
                            this.toggle_security_password_editor_visibility(cx);
                        }),
                    )),
            )
            .when_some(editor.error.clone(), |this, error| {
                this.child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette.danger))
                        .child(error),
                )
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(small_button(
                        palette,
                        "security-pw-save",
                        "Save",
                        cx.listener(|this, _, window, cx| {
                            this.save_security_password_editor(window, cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "security-pw-cancel",
                        "Cancel",
                        cx.listener(|this, _, _, cx| {
                            this.close_security_password_editor(cx);
                        }),
                    )),
            )
    }

    fn security_credential_editor_view(
        &mut self,
        editor: SecurityCredentialEditorState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = if editor.id.is_some() {
            "Edit Credential"
        } else {
            "New Credential"
        };
        let password_display = if editor.password.is_empty() {
            if editor.has_password {
                "loaded (unchanged)".to_string()
            } else {
                " ".to_string()
            }
        } else {
            "•".repeat(editor.password.chars().count().min(24))
        };
        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.bg))
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .track_focus(&self.security_credential_editor_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_security_credential_editor_key_down(event, window, cx);
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(title),
                    )
                    .child(small_button(
                        palette,
                        "security-cred-enabled",
                        if editor.enabled {
                            "Enabled"
                        } else {
                            "Disabled"
                        },
                        cx.listener(|this, _, _, cx| {
                            this.toggle_security_credential_enabled(cx);
                        }),
                    )),
            )
            .child(security_editor_field(
                palette,
                "security-cred-name",
                "Name",
                if editor.name.is_empty() {
                    " ".to_string()
                } else {
                    editor.name.clone()
                },
                editor.focused_field == SecurityCredentialEditorField::Name,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_credential_field(
                        SecurityCredentialEditorField::Name,
                        window,
                        cx,
                    );
                }),
            ))
            .child(security_editor_field(
                palette,
                "security-cred-user",
                "Username",
                if editor.username.is_empty() {
                    " ".to_string()
                } else {
                    editor.username.clone()
                },
                editor.focused_field == SecurityCredentialEditorField::Username,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_credential_field(
                        SecurityCredentialEditorField::Username,
                        window,
                        cx,
                    );
                }),
            ))
            .child(security_editor_field(
                palette,
                "security-cred-pass",
                "Password",
                password_display,
                editor.focused_field == SecurityCredentialEditorField::Password,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_credential_field(
                        SecurityCredentialEditorField::Password,
                        window,
                        cx,
                    );
                }),
            ))
            .child(security_editor_field(
                palette,
                "security-cred-user-re",
                "User Prompt RE",
                if editor.username_prompt_regex.is_empty() {
                    " ".to_string()
                } else {
                    editor.username_prompt_regex.clone()
                },
                editor.focused_field == SecurityCredentialEditorField::UsernameRegex,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_credential_field(
                        SecurityCredentialEditorField::UsernameRegex,
                        window,
                        cx,
                    );
                }),
            ))
            .child(security_editor_field(
                palette,
                "security-cred-pass-re",
                "Pass Prompt RE",
                if editor.password_prompt_regex.is_empty() {
                    " ".to_string()
                } else {
                    editor.password_prompt_regex.clone()
                },
                editor.focused_field == SecurityCredentialEditorField::PasswordRegex,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_credential_field(
                        SecurityCredentialEditorField::PasswordRegex,
                        window,
                        cx,
                    );
                }),
            ))
            .when_some(editor.error.clone(), |this, error| {
                this.child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(palette.danger))
                        .child(error),
                )
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(small_button(
                        palette,
                        "security-cred-save",
                        "Save",
                        cx.listener(|this, _, window, cx| {
                            this.save_security_credential_editor(window, cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "security-cred-cancel",
                        "Cancel",
                        cx.listener(|this, _, _, cx| {
                            this.close_security_credential_editor(cx);
                        }),
                    )),
            )
    }
}

fn security_editor_field(
    palette: crate::ui::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    value: String,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    transfer_input(id, label, value, active, palette)
        .h(px(42.))
        .on_click(on_click)
}

fn security_type_chip(
    palette: crate::ui::theme::ThemePalette,
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("security-type-{label}")))
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

fn session_action_svg_button(
    palette: crate::ui::theme::ThemePalette,
    id: impl Into<String>,
    icon_path: &'static str,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    // Tauri ActiveSessions action icons: h-7 ghost.
    div()
        .id(SharedString::from(id.into()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(if enabled {
            palette.text_muted
        } else {
            palette.text_dimmed
        }))
        .when(enabled, |this| {
            this.cursor_pointer().hover(|this| {
                this.bg(rgb(palette.surface_elevated))
                    .text_color(rgb(palette.text))
            })
        })
        .when(!enabled, |this| this.opacity(0.4))
        .child(svg().size(px(16.)).flex_none().path(icon_path))
        .on_click(move |event, window, cx| {
            if enabled {
                on_click(event, window, cx);
            }
        })
}

fn active_session_menu_item(
    palette: crate::ui::theme::ThemePalette,
    id: impl Into<String>,
    label: impl Into<String>,
    icon_path: &'static str,
    enabled: bool,
    busy: bool,
    destructive: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    let text_color = if !enabled {
        palette.text_dimmed
    } else if destructive {
        palette.danger
    } else {
        palette.text
    };
    div()
        .id(SharedString::from(id.into()))
        .px_3()
        .h(px(30.))
        .flex()
        .items_center()
        .gap_2()
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(|this| this.bg(rgb(palette.surface_elevated)))
        })
        .when(!enabled, |this| this.opacity(0.5))
        .child(
            svg()
                .size(px(14.))
                .flex_none()
                .path(icon_path)
                .text_color(rgb(text_color)),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(text_color))
                .child(if busy { format!("{label}") } else { label }),
        )
        .on_click(move |event, window, cx| {
            if enabled {
                on_click(event, window, cx);
            }
        })
}

fn format_otp_code_display(code: &str) -> String {
    let trimmed = code.trim();
    if trimmed.is_empty() || trimmed == "------" {
        return "------".to_string();
    }
    let digits: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    digits
        .as_bytes()
        .chunks(3)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}
