use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let width = self.left_panel_width.clamp(160., 720.);
        div()
            .w(px(width))
            .flex_none()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(rgb(0x30363d))
            .bg(rgb(0x161b22))
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
                    .text_color(rgb(0x6e7681))
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
                        .text_color(rgb(0x6e7681))
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
            .bg(rgb(0x161b22))
            .child(
                div()
                    .h(px(36.))
                    .flex_none()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x12171f))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                transfer_input(
                                    "active-sessions-search-input",
                                    "Search sessions",
                                    self.active_sessions_search_draft.clone(),
                                    true,
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
                            .text_color(rgb(0x6e7681))
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

    pub(in crate::ui::view) fn left_workspace_summary(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let sessions = self.ordered_sessions();
        let session_count = sessions.len();
        let query = self
            .active_sessions_search_draft
            .trim()
            .to_ascii_lowercase();
        let mut active_session_rows = div().mt_3().flex().flex_col().gap_2();
        let mut visible_count = 0usize;
        if sessions.is_empty() {
            active_session_rows =
                active_session_rows.child(empty_panel("No active runtime sessions."));
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
                active_session_rows =
                    active_session_rows.child(empty_panel("No matching active sessions."));
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
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x10151e))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(0x8f98aa))
                            .child("WORKSPACE"),
                    )
                    .child(capability_line(
                        "Active Sessions",
                        session_count.to_string(),
                    ))
                    .child(capability_line(
                        "Profiles",
                        self.connections.len().to_string(),
                    ))
                    .child(capability_line(
                        "Quick Commands",
                        self.quick_commands.len().to_string(),
                    ))
                    .child(capability_line("Tunnels", self.tunnels.len().to_string())),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x10151e))
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
                                    .text_color(rgb(0x8f98aa))
                                    .child("ACTIVE SESSIONS"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(if query.is_empty() {
                                        rgb(0x98a3b8)
                                    } else {
                                        rgb(0x6ee7b7)
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
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x10151e))
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
                                    .text_color(rgb(0x8f98aa))
                                    .child("START"),
                            )
                            .child(status_pill(
                                status_label(&self.terminal_status),
                                rgb(0x93c5fd),
                                rgb(0x17253b),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .gap_2()
                            .child(small_button(
                                "left-start-local",
                                "Local",
                                cx.listener(|this, _, window, cx| {
                                    this.start_local_session(window, cx);
                                }),
                            ))
                            .child(small_button(
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
                    .border_color(rgb(0x2d3442))
                    .bg(rgb(0x10131a))
                    .p_3()
                    .child(div().text_xs().text_color(rgb(0x8f98aa)).child("Runtime"))
                    .child(div().mt_1().text_sm().child(match self.runtime.mode() {
                        RuntimeMode::Portable => "Portable",
                        RuntimeMode::Installed => "Installed",
                    }))
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(rgb(0x8f98aa))
                            .child(self.runtime.config_dir().display().to_string()),
                    ),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(if self.store_status.ready {
                        rgb(0x244638)
                    } else {
                        rgb(0x4a2525)
                    })
                    .bg(rgb(0x10131a))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x8f98aa))
                            .child("Config Store"),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_sm()
                            .text_color(if self.store_status.ready {
                                rgb(0x6ee7b7)
                            } else {
                                rgb(0xfca5a5)
                            })
                            .child(self.store_status.message.clone()),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(rgb(0x8f98aa))
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
        let session_id = session.id.clone();
        let rename_session_id = session.id.clone();
        let reconnect_session_id = session.id.clone();
        let close_session_id = session.id.clone();
        let custom_color = self.session_tab_colors.get(&session.id).copied();
        let is_active = self.active_session_id.as_deref() == Some(session.id.as_str());
        let has_unread = self
            .terminal_views
            .get(&session.id)
            .is_some_and(|view| view.has_unread);
        let accent = if let Some(custom_color) = custom_color {
            rgb(custom_color)
        } else if is_active {
            rgb(0x6ee7b7)
        } else if has_unread {
            rgb(0xfacc15)
        } else {
            rgb(0x64748b)
        };
        let row_bg = if let Some(custom_color) = custom_color {
            rgba((custom_color << 8) | if is_active { 0x22 } else { 0x12 })
        } else if is_active {
            rgb(0x172033)
        } else {
            rgb(0x111722)
        };
        let hover_bg = if let Some(custom_color) = custom_color {
            rgba((custom_color << 8) | if is_active { 0x30 } else { 0x20 })
        } else {
            rgb(0x1a2230)
        };
        let status_label = if is_active {
            "active"
        } else if has_unread {
            "unread"
        } else {
            "open"
        };

        // Tauri ActiveSessions row: compact list item with type badge + icon actions.
        let kind = session_kind_label(session.kind);
        let short = short_id(&session.id).to_string();
        let _ = status_label;
        div()
            .id(SharedString::from(format!(
                "active-session-row-{session_id}"
            )))
            .rounded_md()
            .px_2()
            .py_1()
            .bg(if is_active { row_bg } else { rgb(0x161b22) })
            .cursor_pointer()
            .hover(move |this| this.bg(hover_bg))
            .child(
                div()
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
                                            .font_weight(FontWeight(700.))
                                            .text_color(rgb(0xe5edf7))
                                            .overflow_hidden()
                                            .child(truncate_preview(&display_name, 28)),
                                    )
                                    .child(
                                        div()
                                            .px_1()
                                            .rounded_sm()
                                            .bg(rgb(0x21262d))
                                            .text_size(px(10.))
                                            .font_weight(FontWeight(700.))
                                            .text_color(rgb(0x8b949e))
                                            .child(kind),
                                    ),
                            )
                            .child(
                                div()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(10.))
                                    .text_color(rgb(0x6e7681))
                                    .child(short),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_0()
                            .child(icon_button(
                                format!("active-session-rename-{rename_session_id}"),
                                "✎",
                                cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.open_rename_session(rename_session_id.clone(), window, cx);
                                }),
                            ))
                            .child(icon_button(
                                format!("active-session-reconnect-{reconnect_session_id}"),
                                "↻",
                                cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.select_session(reconnect_session_id.clone(), cx);
                                    this.reconnect_active_session(window, cx);
                                }),
                            ))
                            .child(icon_button(
                                format!("active-session-close-{close_session_id}"),
                                "×",
                                cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.close_session(close_session_id.clone(), cx);
                                }),
                            )),
                    ),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.select_session(session_id.clone(), cx);
            }))
    }

    fn left_connections_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut rows = div().flex().flex_col().gap_2();
        if self.connections.is_empty() {
            rows = rows.child(empty_panel("No saved connections imported yet."));
        } else {
            for connection in self.connections.iter().take(8).cloned() {
                rows = rows.child(compact_connection_row(
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
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x10151e))
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
                                    .text_color(rgb(0x8f98aa))
                                    .child("SAVED CONNECTIONS"),
                            )
                            .child(status_pill(
                                status_label(&self.terminal_status),
                                rgb(0x93c5fd),
                                rgb(0x17253b),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .gap_2()
                            .child(small_button(
                                "left-connections-local",
                                "Local",
                                cx.listener(|this, _, window, cx| {
                                    this.start_local_session(window, cx);
                                }),
                            ))
                            .child(small_button(
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
        let mut rows = div().flex().flex_col().gap_2();
        if self.tunnels.is_empty() {
            rows = rows.child(empty_panel("No SSH tunnels configured."));
        } else {
            for tunnel in self.tunnels.iter().take(8).cloned() {
                let is_pending = self.pending_tunnels.iter().any(|id| id == &tunnel.id);
                let is_open = self.tunnel_manager.is_open(&tunnel.id).unwrap_or(false);
                rows = rows.child(compact_tunnel_row(
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
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x10151e))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0x8f98aa))
                            .child("NETWORK"),
                    )
                    .child(capability_line(
                        "Configured Tunnels",
                        self.tunnels.len().to_string(),
                    ))
                    .child(capability_line(
                        "Pending",
                        self.pending_tunnels.len().to_string(),
                    ))
                    .child(capability_line(
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
        let mut jobs = div().flex().flex_col().gap_2();
        if self.transfer_jobs.is_empty() {
            jobs = jobs.child(empty_panel("No SFTP transfer jobs yet."));
        } else {
            for job in self.transfer_jobs.iter().rev().take(5) {
                jobs = jobs.child(compact_transfer_job_row(job));
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
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x10151e))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0x8f98aa))
                            .child("SFTP"),
                    )
                    .child(capability_line(
                        "SSH Session",
                        if self.active_ssh_config.is_some() {
                            "ready"
                        } else {
                            "none"
                        },
                    ))
                    .child(capability_line(
                        "Remote Path",
                        truncate_preview(&self.transfer_remote_path, 28),
                    ))
                    .child(capability_line(
                        "Duplicate Policy",
                        duplicate_policy_label(self.transfer_duplicate_policy),
                    ))
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .gap_2()
                            .child(small_button(
                                "left-sftp-list",
                                "List",
                                cx.listener(|this, _, window, cx| {
                                    this.start_sftp_list_job(window, cx);
                                }),
                            ))
                            .child(small_button(
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
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x10151e))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0x8f98aa))
                            .child("SETTINGS"),
                    )
                    .child(capability_line("Theme", self.settings.theme.clone()))
                    .child(capability_line(
                        "Terminal Font",
                        format!(
                            "{} {}",
                            self.settings.terminal_font_family, self.settings.terminal_font_size
                        ),
                    ))
                    .child(capability_line(
                        "Host Key Policy",
                        self.settings.host_key_policy.clone(),
                    ))
                    .child(capability_line(
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
                        rgb(0x244638)
                    } else {
                        rgb(0x4a2525)
                    })
                    .bg(rgb(0x10131a))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0x8f98aa))
                            .child("CONFIG STORE"),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_sm()
                            .text_color(if self.store_status.ready {
                                rgb(0x6ee7b7)
                            } else {
                                rgb(0xfca5a5)
                            })
                            .child(self.store_status.message.clone()),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
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
                this.bg(rgb(0x243044)).text_color(rgb(0xffffff))
            })
            .when(!selected, |this| {
                this.text_color(rgb(0xaeb7c8))
                    .hover(|hover| hover.bg(rgb(0x202632)).text_color(rgb(0xffffff)))
            })
            .child(div().min_w_0().overflow_hidden().child(label))
            .child(
                div()
                    .flex_none()
                    .rounded_sm()
                    .bg(if selected {
                        rgb(0x162235)
                    } else {
                        rgb(0x10151e)
                    })
                    .px_2()
                    .py_1()
                    .font_family("JetBrains Mono")
                    .text_size(px(10.))
                    .text_color(if selected {
                        rgb(0x93c5fd)
                    } else {
                        rgb(0x64748b)
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

        let mut body = div().flex_1().min_h_0().overflow_hidden().flex().flex_col().gap_2().p_2();

        match active_tab {
            SecurityAuthTab::Keys => {
                if let Some(editor) = self.security_key_editor.clone() {
                    body = body.child(self.security_key_editor_view(editor, cx));
                } else if self.connection_ssh_keys.is_empty() {
                    body = body.child(empty_panel("No SSH keys yet. Add a private key to use key auth."));
                } else {
                    for key in self.connection_ssh_keys.clone() {
                        let key_id = key.id.clone();
                        let edit_id = key.id.clone();
                        let delete_id = key.id.clone();
                        body = body.child(
                            div()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(0x30363d))
                                .bg(rgb(0x0d1117))
                                .px_2()
                                .py_1()
                                .flex()
                                .flex_col()
                                .gap_2()
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
                                                .text_xs()
                                                .font_weight(FontWeight(700.))
                                                .text_color(rgb(0xc9d1d9))
                                                .child(truncate_preview(&key.name, 28)),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(if key.has_key_data {
                                                    rgb(0x3fb950)
                                                } else {
                                                    rgb(0x8b949e)
                                                })
                                                .child(if key.has_key_data { "ready" } else { "empty" }),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(rgb(0x6e7681))
                                        .child(format!(
                                            "{} · cert {}",
                                            compact_id(&key_id),
                                            if key.has_cert_data { "yes" } else { "no" }
                                        )),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(small_button(
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
                                            format!("security-key-del-{key_id}"),
                                            "Del",
                                            cx.listener(move |this, _, _, cx| {
                                                this.request_delete_security_key(delete_id.clone(), cx);
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
                    body = body.child(empty_panel("No saved passwords yet."));
                } else {
                    for entry in self.connection_saved_passwords.clone() {
                        let id = entry.id.clone();
                        let edit_id = entry.id.clone();
                        let delete_id = entry.id.clone();
                        let reveal_id = entry.id.clone();
                        let revealed = self
                            .security_revealed_passwords
                            .get(&entry.id)
                            .cloned()
                            .unwrap_or_else(|| {
                                if entry.has_password {
                                    "••••••••".to_string()
                                } else {
                                    "empty".to_string()
                                }
                            });
                        body = body.child(
                            div()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(0x30363d))
                                .bg(rgb(0x0d1117))
                                .px_2()
                                .py_1()
                                .flex()
                                .flex_col()
                                .gap_2()
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
                                                .text_xs()
                                                .font_weight(FontWeight(700.))
                                                .text_color(rgb(0xc9d1d9))
                                                .child(truncate_preview(&entry.name, 28)),
                                        )
                                        .child(
                                            div()
                                                .font_family("JetBrains Mono")
                                                .text_size(px(10.))
                                                .text_color(rgb(0x8b949e))
                                                .child(truncate_preview(&revealed, 16)),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(small_button(
                                            format!("security-pw-show-{id}"),
                                            "Show",
                                            cx.listener(move |this, _, window, cx| {
                                                this.reveal_security_password(
                                                    reveal_id.clone(),
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
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
                    body = body.child(empty_panel("No autofill credentials yet."));
                } else {
                    for entry in self.connection_saved_credentials.clone() {
                        let id = entry.id.clone();
                        let edit_id = entry.id.clone();
                        let delete_id = entry.id.clone();
                        let reveal_id = entry.id.clone();
                        let secret = self
                            .security_revealed_credentials
                            .get(&entry.id)
                            .cloned()
                            .unwrap_or_else(|| {
                                if entry.has_password {
                                    "••••••••".to_string()
                                } else {
                                    "no password".to_string()
                                }
                            });
                        body = body.child(
                            div()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(0x30363d))
                                .bg(rgb(0x0d1117))
                                .px_2()
                                .py_1()
                                .flex()
                                .flex_col()
                                .gap_2()
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
                                                .text_xs()
                                                .font_weight(FontWeight(700.))
                                                .text_color(rgb(0xc9d1d9))
                                                .child(truncate_preview(&entry.name, 24)),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(if entry.enabled {
                                                    rgb(0x3fb950)
                                                } else {
                                                    rgb(0x8b949e)
                                                })
                                                .child(if entry.enabled { "on" } else { "off" }),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(rgb(0x6e7681))
                                        .child(format!(
                                            "{} · {}",
                                            truncate_preview(&entry.username, 18),
                                            truncate_preview(&secret, 12)
                                        )),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(small_button(
                                            format!("security-cred-show-{id}"),
                                            "Show",
                                            cx.listener(move |this, _, window, cx| {
                                                this.reveal_security_credential_password(
                                                    reveal_id.clone(),
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
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
                    body = body.child(empty_panel("No OTP accounts yet. Add TOTP/HOTP for auto-fill."));
                } else {
                    for entry in self.connection_otp_entries.clone() {
                        let otp_id = entry.id.clone();
                        let edit_id = entry.id.clone();
                        let delete_id = entry.id.clone();
                        let code_id = entry.id.clone();
                        let title = if !entry.issuer.trim().is_empty() || !entry.username.trim().is_empty()
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
                        let code = self
                            .security_otp_codes
                            .get(&entry.id)
                            .cloned()
                            .unwrap_or_else(|| "------".to_string());
                        body = body.child(
                            div()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(0x30363d))
                                .bg(rgb(0x0d1117))
                                .px_2()
                                .py_1()
                                .flex()
                                .flex_col()
                                .gap_2()
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
                                                .text_xs()
                                                .font_weight(FontWeight(700.))
                                                .text_color(rgb(0xc9d1d9))
                                                .child(truncate_preview(&title, 24)),
                                        )
                                        .child(
                                            div()
                                                .font_family("JetBrains Mono")
                                                .text_sm()
                                                .font_weight(FontWeight(800.))
                                                .text_color(rgb(0x58a6ff))
                                                .child(code),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(rgb(0x6e7681))
                                        .child(format!(
                                            "{} · {} · {}d · {}",
                                            entry.otp_type.to_uppercase(),
                                            entry.algorithm,
                                            entry.digits,
                                            if entry.has_secret { "secret" } else { "no secret" }
                                        )),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(small_button(
                                            format!("security-otp-code-{otp_id}"),
                                            "Code",
                                            cx.listener(move |this, _, window, cx| {
                                                this.generate_security_otp_code(
                                                    code_id.clone(),
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
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
                                            format!("security-otp-del-{otp_id}"),
                                            "Del",
                                            cx.listener(move |this, _, _, cx| {
                                                this.request_delete_security_otp(delete_id.clone(), cx);
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
                    .border_color(rgb(0xf85149))
                    .bg(rgb(0x2d1214))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(0xff7b72))
                            .child(format!("Delete {}?", confirm.label)),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(small_button(
                                "security-delete-confirm",
                                "Delete",
                                cx.listener(|this, _, _, cx| {
                                    this.confirm_security_delete(cx);
                                }),
                            ))
                            .child(small_button(
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
            .bg(rgb(0x161b22))
            .child(
                div()
                    .px_2()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x12171f))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(0x8b949e))
                                    .child("SECURITY / AUTH"),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(0x6e7681))
                                    .child(format!("MP {master} · K{key_count}/P{password_count}/C{credential_count}/O{otp_count}")),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(self.security_tab_chip(SecurityAuthTab::Keys, cx))
                            .child(self.security_tab_chip(SecurityAuthTab::Passwords, cx))
                            .child(self.security_tab_chip(SecurityAuthTab::Credentials, cx))
                            .child(self.security_tab_chip(SecurityAuthTab::Otp, cx))
                            .child(div().flex_1())
                            .child(small_button(
                                "security-add-item",
                                "Add",
                                cx.listener(|this, _, window, cx| {
                                    match this.security_auth_tab {
                                        SecurityAuthTab::Keys => {
                                            this.open_security_key_editor(None, window, cx);
                                        }
                                        SecurityAuthTab::Passwords => {
                                            this.open_security_password_editor(None, window, cx);
                                        }
                                        SecurityAuthTab::Credentials => {
                                            this.open_security_credential_editor(None, window, cx);
                                        }
                                        SecurityAuthTab::Otp => {
                                            this.open_security_otp_editor(None, window, cx);
                                        }
                                    }
                                }),
                            ))
                            .child(small_button(
                                "security-open-settings",
                                "Settings",
                                cx.listener(|this, _, _, cx| {
                                    this.open_page(NavItem::Settings, cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(0x6e7681))
                            .child(self.security_status.clone()),
                    ),
            )
            .child(body)
            .child(self.security_secret_footer(cx))
            .when(self.security_unlock_prompt_open, |this| {
                this.child(self.security_unlock_prompt(cx))
            })
    }

    fn security_secret_footer(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let unlocked = !self.security_secrets_locked();
        let has_master = self.settings.has_master_password;
        div()
            .flex_none()
            .border_t_1()
            .border_color(rgb(0x30363d))
            .bg(rgb(0x12171f))
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
                        rgb(0x3fb950)
                    } else {
                        rgb(0xd29922)
                    })
                    .child(if !has_master {
                        "Secrets open (no master password)"
                    } else if unlocked {
                        "Secrets unlocked"
                    } else {
                        "Secrets locked"
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(small_button(
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
                    )),
            )
    }

    fn security_unlock_prompt(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x161b22))
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
                            .text_color(rgb(0xc9d1d9))
                            .child("Unlock Secrets"),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(0x8b949e))
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
                            .border_color(rgb(0x4b6f97))
                            .bg(rgb(0x0d1320))
                            .font_family("JetBrains Mono")
                            .text_xs()
                            .text_color(rgb(0xe5edf7))
                            .child(draft),
                    )
                    .when_some(self.security_unlock_error.clone(), |this, error| {
                        this.child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(0xff7b72))
                                .child(error),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                "security-unlock-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.close_security_unlock_prompt(cx);
                                }),
                            ))
                            .child(small_button(
                                "security-unlock-submit",
                                "Unlock",
                                cx.listener(|this, _, _, cx| {
                                    this.submit_security_unlock(cx);
                                }),
                            )),
                    ),
            )
    }

    fn security_tab_chip(
        &self,
        tab: SecurityAuthTab,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.security_auth_tab == tab;
        div()
            .id(SharedString::from(format!("security-tab-{}", tab.label())))
            .h(px(24.))
            .px_2()
            .flex()
            .items_center()
            .rounded_sm()
            .cursor_pointer()
            .text_size(px(10.))
            .font_weight(FontWeight(700.))
            .text_color(if selected {
                rgb(0xffffff)
            } else {
                rgb(0x8b949e)
            })
            .bg(if selected {
                rgb(0x1f6feb)
            } else {
                rgb(0x21262d)
            })
            .hover(|this| this.bg(if selected { rgb(0x1f6feb) } else { rgb(0x30363d) }))
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
            .border_color(rgb(0x30363d))
            .bg(rgb(0x0d1117))
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
                    .text_color(rgb(0xc9d1d9))
                    .child(title),
            )
            .child(security_editor_field(
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
                            .text_color(rgb(0x8b949e))
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
                                    .border_color(if editor.focused_field
                                        == SecurityKeyEditorField::KeyPath
                                    {
                                        rgb(0x4b6f97)
                                    } else {
                                        rgb(0x303848)
                                    })
                                    .bg(rgb(0x0d1320))
                                    .text_size(px(10.))
                                    .text_color(rgb(0xc9d1d9))
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
                            .text_color(rgb(0x8b949e))
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
                                    .border_color(if editor.focused_field
                                        == SecurityKeyEditorField::CertPath
                                    {
                                        rgb(0x4b6f97)
                                    } else {
                                        rgb(0x303848)
                                    })
                                    .bg(rgb(0x0d1320))
                                    .text_size(px(10.))
                                    .text_color(rgb(0xc9d1d9))
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
                                "security-cert-browse",
                                "Browse",
                                cx.listener(|this, _, window, cx| {
                                    this.pick_security_key_file(true, window, cx);
                                }),
                            )),
                    ),
            )
            .child(security_editor_field(
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
                        .text_color(rgb(0xff7b72))
                        .child(error),
                )
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(small_button(
                        "security-key-save",
                        "Save",
                        cx.listener(|this, _, window, cx| {
                            this.save_security_key_editor(window, cx);
                        }),
                    ))
                    .child(small_button(
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
            .border_color(rgb(0x30363d))
            .bg(rgb(0x0d1117))
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
                    .text_color(rgb(0xc9d1d9))
                    .child(title),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(security_type_chip(
                        "TOTP",
                        editor.otp_type != "hotp",
                        cx.listener(|this, _, _, cx| {
                            this.set_security_otp_type("totp", cx);
                        }),
                    ))
                    .child(security_type_chip(
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
                            .text_color(rgb(0xc9d1d9))
                            .bg(rgb(0x21262d))
                            .hover(|this| this.bg(rgb(0x30363d)))
                            .child(editor.algorithm.clone())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cycle_security_otp_algorithm(cx);
                            })),
                    ),
            )
            .child(security_editor_field(
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
                        "security-otp-digits",
                        "Digits",
                        if editor.digits.is_empty() {
                            " ".to_string()
                        } else {
                            editor.digits.clone()
                        },
                        editor.focused_field == SecurityOtpEditorField::Digits,
                        cx.listener(|this, _, window, cx| {
                            this.focus_security_otp_field(SecurityOtpEditorField::Digits, window, cx);
                        }),
                    ))
                    .child(security_editor_field(
                        "security-otp-period",
                        "Period",
                        if editor.period.is_empty() {
                            " ".to_string()
                        } else {
                            editor.period.clone()
                        },
                        editor.focused_field == SecurityOtpEditorField::Period,
                        cx.listener(|this, _, window, cx| {
                            this.focus_security_otp_field(SecurityOtpEditorField::Period, window, cx);
                        }),
                    ))
                    .child(security_editor_field(
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
                        .text_color(rgb(0xff7b72))
                        .child(error),
                )
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(small_button(
                        "security-otp-save",
                        "Save",
                        cx.listener(|this, _, window, cx| {
                            this.save_security_otp_editor(window, cx);
                        }),
                    ))
                    .child(small_button(
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
        let provider = configured_cloud_sync_provider(&self.cloud_sync_settings);
        let history = self.cloud_sync_history.clone();
        let mut rows = div().mt_3().flex().flex_col().gap_2();
        if history.is_empty() {
            rows = rows.child(empty_panel("No cloud sync history yet."));
        } else {
            for entry in history.into_iter().take(10) {
                rows = rows.child(cloud_sync_history_row(entry));
            }
        }

        div()
            .size_full()
            .p_3()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x0d1117))
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
                                    .text_color(rgb(0x8b949e))
                                    .child("SYNC / BACKUP"),
                            )
                            .child(status_pill(
                                if self.cloud_sync_settings.enabled {
                                    "enabled"
                                } else {
                                    "disabled"
                                },
                                if self.cloud_sync_settings.enabled {
                                    rgb(0x3fb950)
                                } else {
                                    rgb(0x8b949e)
                                },
                                if self.cloud_sync_settings.enabled {
                                    rgb(0x12261a)
                                } else {
                                    rgb(0x161b22)
                                },
                            )),
                    )
                    .child(capability_line("Provider", provider))
                    .child(capability_line(
                        "Status",
                        truncate_preview(&self.cloud_sync_status, 48),
                    ))
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .child(small_button(
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
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x0d1117))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0x8b949e))
                            .child("RECENT HISTORY"),
                    )
                    .child(rows),
            )
    }


    fn security_password_editor_view(
        &mut self,
        editor: SecurityPasswordEditorState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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
        } else {
            "•".repeat(editor.password.chars().count().min(24))
        };
        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x30363d))
            .bg(rgb(0x0d1117))
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
                    .text_color(rgb(0xc9d1d9))
                    .child(title),
            )
            .child(security_editor_field(
                "security-pw-name",
                "Name",
                if editor.name.is_empty() {
                    " ".to_string()
                } else {
                    editor.name.clone()
                },
                editor.focused_field == SecurityPasswordEditorField::Name,
                cx.listener(|this, _, window, cx| {
                    this.focus_security_password_field(SecurityPasswordEditorField::Name, window, cx);
                }),
            ))
            .child(security_editor_field(
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
            ))
            .when_some(editor.error.clone(), |this, error| {
                this.child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(0xff7b72))
                        .child(error),
                )
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(small_button(
                        "security-pw-save",
                        "Save",
                        cx.listener(|this, _, window, cx| {
                            this.save_security_password_editor(window, cx);
                        }),
                    ))
                    .child(small_button(
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
            .border_color(rgb(0x30363d))
            .bg(rgb(0x0d1117))
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
                            .text_color(rgb(0xc9d1d9))
                            .child(title),
                    )
                    .child(small_button(
                        "security-cred-enabled",
                        if editor.enabled { "Enabled" } else { "Disabled" },
                        cx.listener(|this, _, _, cx| {
                            this.toggle_security_credential_enabled(cx);
                        }),
                    )),
            )
            .child(security_editor_field(
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
                        .text_color(rgb(0xff7b72))
                        .child(error),
                )
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(small_button(
                        "security-cred-save",
                        "Save",
                        cx.listener(|this, _, window, cx| {
                            this.save_security_credential_editor(window, cx);
                        }),
                    ))
                    .child(small_button(
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
    id: impl Into<String>,
    label: &'static str,
    value: String,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    transfer_input(id, label, value, active)
        .h(px(42.))
        .on_click(on_click)
}

fn security_type_chip(
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

