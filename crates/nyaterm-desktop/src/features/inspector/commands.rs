use super::*;

impl NyaTermApp {
    pub(in crate::features) fn command_center_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let sessions = self
            .ordered_sessions()
            .into_iter()
            .filter(|session| !self.is_session_disconnected(&session.id))
            .collect::<Vec<_>>();
        let active_label = self
            .active_session_id
            .as_deref()
            .map(compact_id)
            .unwrap_or_else(|| "none".to_string());
        let provider = configured_cloud_sync_provider(&self.cloud_sync_settings);
        let provider_action = provider != "local_directory";
        let sync_label = if provider_action { "Provider" } else { "Local" };
        let mut session_rows = div().mt_3().flex().flex_col().gap_2();
        if sessions.is_empty() {
            session_rows = session_rows.child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .child("No active runtime sessions."),
            );
        } else {
            for session in sessions.into_iter().take(3) {
                let display_name = self.session_display_name_by_info(&session);
                let is_active = self.active_session_id.as_deref() == Some(session.id.as_str());
                session_rows = session_rows.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .border_t_1()
                        .border_color(rgb(palette.border))
                        .pt_2()
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(palette.text))
                                        .child(truncate_preview(&display_name, 42)),
                                )
                                .child(div().text_xs().text_color(rgb(palette.text_muted)).child(
                                    format!(
                                        "{} · {}",
                                        session_kind_label(session.kind),
                                        compact_id(&session.id)
                                    ),
                                )),
                        )
                        .child(status_pill(
                            if is_active { "active" } else { "open" },
                            if is_active {
                                rgb(palette.success)
                            } else {
                                rgb(palette.link)
                            },
                            if is_active {
                                rgb(palette.hover)
                            } else {
                                rgb(palette.hover)
                            },
                        )),
                );
            }
        }

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Command Center"),
                    )
                    .child(status_pill(
                        "native",
                        rgb(palette.success),
                        rgb(palette.hover),
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .child(metric(palette, "Active", active_label))
                    .child(metric(palette, "Sync", provider)),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(small_button(
                        palette,
                        "command-center-new-session",
                        "New",
                        cx.listener(|this, _, window, cx| {
                            this.open_connection_editor(None, None, false, window, cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "command-center-active-sessions",
                        "Sessions",
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Workspace, cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "command-center-settings",
                        "Settings",
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Settings, cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "command-center-update-check",
                        "Updates",
                        cx.listener(|this, _, _, cx| {
                            this.start_update_check(cx);
                        }),
                    )),
            )
            .child(
                div()
                    .mt_2()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(small_button(
                        palette,
                        "command-center-sync-push",
                        "Push",
                        cx.listener(move |this, _, _, cx| {
                            if provider_action {
                                this.prompt_provider_cloud_sync_push(cx);
                            } else {
                                this.prompt_local_cloud_sync_push(cx);
                            }
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "command-center-sync-pull",
                        "Pull",
                        cx.listener(move |this, _, _, cx| {
                            if provider_action {
                                this.prompt_provider_cloud_sync_pull(cx);
                            } else {
                                this.prompt_local_cloud_sync_pull(cx);
                            }
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "command-center-sync-history",
                        "History",
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Settings, cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "command-center-migration",
                        "Migration",
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Migration, cx);
                        }),
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .line_height(px(18.))
                    .child(format!(
                        "{sync_label} sync · {} · {}",
                        truncate_preview(&self.cloud_sync_status, 84),
                        truncate_preview(&self.update_status, 84)
                    )),
            )
            .child(session_rows)
    }

    pub(in crate::features) fn command_search_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let results = self.command_search_results();
        let mut rows = div().mt_3().flex().flex_col().gap_2();
        if results.is_empty() {
            rows = rows.child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .line_height(px(18.))
                    .child("No matches."),
            );
        } else {
            for (index, result) in results.into_iter().enumerate() {
                let meta = format!(
                    "{} · {}",
                    command_source_label(&result.source),
                    result.score
                );
                rows = rows.child(
                    div()
                        .border_t_1()
                        .border_color(rgb(palette.border))
                        .pt_2()
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
                                        .text_xs()
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(palette.text))
                                        .child(truncate_preview(&result.display, 44)),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(palette.text_dimmed))
                                        .child(meta),
                                ),
                        )
                        .child(
                            div()
                                .font_family(crate::features::gpui_code_font_family())
                                .text_xs()
                                .text_color(rgb(palette.text_muted))
                                .line_height(px(18.))
                                .child(truncate_preview(&result.command, 120)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_end()
                                .gap_1()
                                .child(small_button(
                                    palette,
                                    format!("command-search-insert-{index}"),
                                    "Insert",
                                    cx.listener(move |this, _, _, cx| {
                                        this.insert_command_search_result(index, cx);
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    format!("command-search-run-{index}"),
                                    "Run",
                                    cx.listener(move |this, _, _, cx| {
                                        this.run_command_search_result(index, cx);
                                    }),
                                )),
                        ),
                );
            }
        }

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Command Search"),
                    )
                    .child(status_pill(
                        if self.command_search_draft.trim().is_empty() {
                            "idle"
                        } else {
                            "matched"
                        },
                        rgb(palette.link),
                        rgb(palette.hover),
                    )),
            )
            .child(
                transfer_input(
                    "command-search-input",
                    "Search",
                    self.command_search_draft.clone(),
                    true,
                    self.theme_palette(),
                )
                .mt_3()
                .track_focus(&self.command_search_focus)
                .on_click(cx.listener(|this, _, window, cx| {
                    window.focus(&this.command_search_focus);
                    cx.notify();
                }))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.handle_command_search_key_down(event, cx);
                })),
            )
            .child(rows)
    }

    pub(in crate::features) fn command_history_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Tauri CommandHistory: dense mono list; double-click sends. GPUI: click runs (single-click proxy).
        let history = self.active_session_history_commands();
        let mut rows = div().flex().flex_col().gap_0().p_2();
        if history.is_empty() {
            rows = rows.child(
                div()
                    .py_4()
                    .text_center()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child("No commands yet"),
            );
        } else {
            for (index, command) in history.into_iter().enumerate() {
                let run_index = index;
                rows = rows.child(
                    div()
                        .id(SharedString::from(format!("command-history-row-{index}")))
                        .h(px(28.))
                        .px_2()
                        .rounded_sm()
                        .flex()
                        .items_center()
                        .gap_1()
                        .cursor_pointer()
                        .hover(|this| this.bg(rgb(palette.hover)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.run_history_command(run_index, cx);
                        }))
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(palette.text_dimmed))
                                .child("›"),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .font_family(crate::features::gpui_code_font_family())
                                .text_size(px(12.))
                                .text_color(rgb(palette.text))
                                .overflow_hidden()
                                .child(truncate_preview(&command, 120)),
                        ),
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
                    .id(SharedString::from("command-history-list"))
                    .flex_1()
                    .min_h_0()
                    .overflow_scroll()
                    .scrollbar_width(px(6.))
                    .child(rows),
            )
    }
}
