use super::*;

impl NyaTermApp {
    pub(in crate::features) fn active_sessions_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let sessions = self.ordered_sessions();
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
                    .child(self.tr("panel.noActiveSessions")),
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
                        .child(self.tr("activeSessions.noMatches")),
                );
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
                    .h(px(40.))
                    .flex_none()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.section_header))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id(SharedString::from("active-sessions-search-input"))
                            .h(px(28.))
                            .flex_1()
                            .min_w_0()
                            .rounded_md()
                            .bg(rgb(palette.hover))
                            .px_2()
                            .flex()
                            .items_center()
                            .gap_2()
                            .cursor_text()
                            .track_focus(&self.active_sessions_search_focus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                window.focus(&this.active_sessions_search_focus);
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.handle_active_sessions_search_key_down(event, cx);
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
                                    .overflow_hidden()
                                    .text_size(px(12.))
                                    .text_color(if self.active_sessions_search_draft.is_empty() {
                                        rgb(palette.text_dimmed)
                                    } else {
                                        rgb(palette.text)
                                    })
                                    .child(if self.active_sessions_search_draft.is_empty() {
                                        self.tr("activeSessions.searchPlaceholder").to_string()
                                    } else {
                                        self.active_sessions_search_draft.clone()
                                    }),
                            ),
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

    pub(in crate::features) fn left_workspace_summary(
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
                                rgb(palette.link),
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

    pub(in crate::features) fn active_session_row(
        &mut self,
        session: SessionInfo,
        display_name: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let session_id = session.id.clone();
        let rename_session_id = session.id.clone();
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
                                    .font_family(crate::features::gpui_code_font_family())
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
                                    this.active_session_menu = None;
                                    this.open_rename_session(rename_session_id.clone(), window, cx);
                                }),
                            ))
                            .child(session_action_svg_button(
                                palette,
                                format!("active-session-more-{menu_session_id}"),
                                "icons/session/more.svg",
                                !is_busy,
                                cx.listener(move |this, event: &ClickEvent, _, cx| {
                                    cx.stop_propagation();
                                    if this
                                        .active_session_busy_actions
                                        .contains_key(&menu_session_id)
                                    {
                                        return;
                                    }
                                    let point = event.position();
                                    if this
                                        .active_session_menu
                                        .as_ref()
                                        .is_some_and(|menu| menu.session_id == menu_session_id)
                                    {
                                        this.active_session_menu = None;
                                    } else {
                                        this.active_session_menu = Some(ActiveSessionMenuState {
                                            session_id: menu_session_id.clone(),
                                            x: point.x,
                                            y: point.y,
                                        });
                                    }
                                    cx.notify();
                                }),
                            )),
                    ),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.active_session_menu = None;
                this.select_session(session_id.clone(), cx);
            }))
    }
}
