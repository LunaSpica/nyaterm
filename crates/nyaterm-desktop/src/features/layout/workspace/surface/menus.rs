use super::*;

impl NyaTermApp {
    pub(in crate::features) fn render_open_tabs_menu(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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
            .child(div().mx_2().my_1().h(px(1.)).bg(rgb(palette.border)));

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
                let accent = if let Some(color) = self.session_tab_colors.get(&session_id).copied()
                {
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
                        .child(div().size(px(8.)).rounded_full().bg(accent))
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

    pub(in crate::features) fn render_new_session_menu(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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

        menu = menu
            .child(div().mx_2().my_1().h(px(1.)).bg(rgb(palette.border)))
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
            .child(div().mx_2().my_1().h(px(1.)).bg(rgb(palette.border)))
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

    pub(in crate::features) fn new_session_connection_row(
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
            .id(SharedString::from(format!(
                "new-session-conn-{connection_id}"
            )))
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
}
