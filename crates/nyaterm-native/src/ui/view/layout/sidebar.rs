use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let running_transfers = self
            .transfer_jobs
            .iter()
            .filter(|job| {
                matches!(
                    job.status,
                    TransferJobStatus::Running | TransferJobStatus::Cancelling
                )
            })
            .count();
        let open_tunnels = self
            .tunnels
            .iter()
            .filter(|tunnel| self.tunnel_manager.is_open(&tunnel.id).unwrap_or(false))
            .count();
        let transfer_badge = if running_transfers > 0 {
            format!("{running_transfers}/{}", self.transfer_jobs.len())
        } else {
            self.transfer_jobs.len().to_string()
        };
        let tunnel_badge = if open_tunnels > 0 {
            format!("{open_tunnels}/{}", self.tunnels.len())
        } else {
            self.tunnels.len().to_string()
        };

        div()
            .w(px(292.))
            .flex_none()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(rgb(0x242a35))
            .bg(rgb(0x151923))
            .child(panel_header("Explorer", self.left_panel_meta()))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_2()
                    .border_b_1()
                    .border_color(rgb(0x242a35))
                    .child(self.nav_button(
                        "Saved Connections",
                        self.connections.len().to_string(),
                        NavItem::Connections,
                        cx,
                    ))
                    .child(self.nav_button(
                        "File Explorer / SFTP",
                        transfer_badge,
                        NavItem::Transfers,
                        cx,
                    ))
                    .child(self.nav_button("Network Tunnels", tunnel_badge, NavItem::Tunnels, cx))
                    .child(self.nav_button(
                        "Security / Settings",
                        if self.store_status.ready {
                            "ok".to_string()
                        } else {
                            "err".to_string()
                        },
                        NavItem::Settings,
                        cx,
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(self.left_panel_summary(cx)),
            )
    }

    fn left_panel_summary(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        match self.selected_nav {
            NavItem::Connections => self.left_connections_panel(cx).into_any_element(),
            NavItem::Transfers => self.left_transfers_panel(cx).into_any_element(),
            NavItem::Tunnels => self.left_network_panel(cx).into_any_element(),
            NavItem::Settings => self.left_settings_panel(cx).into_any_element(),
            _ => self.left_workspace_summary(cx).into_any_element(),
        }
    }

    fn left_workspace_summary(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
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
        let focus_session_id = session.id.clone();
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

        div()
            .id(SharedString::from(format!(
                "active-session-row-{session_id}"
            )))
            .rounded_md()
            .border_1()
            .border_color(if is_active {
                custom_color.map(rgb).unwrap_or_else(|| rgb(0x3b4c64))
            } else {
                rgb(0x2a3140)
            })
            .bg(row_bg)
            .p_2()
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
                            .gap_1()
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
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(0xe5edf7))
                                            .overflow_hidden()
                                            .child(truncate_preview(&display_name, 26)),
                                    )
                                    .child(status_pill(
                                        status_label,
                                        if is_active {
                                            rgb(0x6ee7b7)
                                        } else if has_unread {
                                            rgb(0xfacc15)
                                        } else {
                                            rgb(0x93c5fd)
                                        },
                                        if is_active {
                                            rgb(0x12342a)
                                        } else if has_unread {
                                            rgb(0x3a2f14)
                                        } else {
                                            rgb(0x17233a)
                                        },
                                    )),
                            )
                            .child(
                                div()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(10.))
                                    .text_color(rgb(0x98a3b8))
                                    .child(format!(
                                        "{} · {}",
                                        session_kind_label(session.kind),
                                        short_id(&session.id)
                                    )),
                            ),
                    ),
            )
            .child(
                div()
                    .mt_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_1()
                    .child(small_button(
                        format!("active-session-focus-{focus_session_id}"),
                        "Focus",
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.select_session(focus_session_id.clone(), cx);
                        }),
                    ))
                    .child(small_button(
                        format!("active-session-rename-{rename_session_id}"),
                        "Rename",
                        cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.open_rename_session(rename_session_id.clone(), window, cx);
                        }),
                    ))
                    .child(small_button(
                        format!("active-session-reconnect-{reconnect_session_id}"),
                        "Reconn",
                        cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.select_session(reconnect_session_id.clone(), cx);
                            this.reconnect_active_session(window, cx);
                        }),
                    ))
                    .child(small_button(
                        format!("active-session-close-{close_session_id}"),
                        "Close",
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.close_session(close_session_id.clone(), cx);
                        }),
                    )),
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
