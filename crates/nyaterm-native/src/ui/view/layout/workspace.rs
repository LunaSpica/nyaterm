use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn main_surface(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(rgb(0x0b0d12))
            .child(
                if self.main_mode == MainMode::Workspace || self.selected_nav == NavItem::Workspace
                {
                    self.workspace_view(cx).into_any_element()
                } else {
                    match self.selected_nav {
                        NavItem::Workspace => self.workspace_view(cx).into_any_element(),
                        NavItem::Connections => self.connections_view(cx).into_any_element(),
                        NavItem::Tunnels => self.tunnels_view(cx).into_any_element(),
                        NavItem::Stats => self.stats_view(cx).into_any_element(),
                        NavItem::Processes => self.processes_view(cx).into_any_element(),
                        NavItem::Docker => self.docker_view(cx).into_any_element(),
                        NavItem::Translation => self.translation_view(cx).into_any_element(),
                        NavItem::Transfers => self.transfers_view(cx).into_any_element(),
                        NavItem::Settings => self.settings_view(cx).into_any_element(),
                        NavItem::Migration => self.migration_view().into_any_element(),
                    }
                },
            )
    }

    pub(in crate::ui::view) fn session_tab_strip(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let sessions = self.ordered_sessions();
        let session_count = sessions.len();
        let mut tabs = div()
            .h_full()
            .min_w_0()
            .flex_1()
            .flex()
            .items_center()
            .overflow_hidden();

        if let Some(pending_name) = self.pending_session_name.clone() {
            tabs = tabs.child(
                div()
                    .h_full()
                    .min_w(px(178.))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_r_1()
                    .border_color(rgb(0x202633))
                    .bg(rgb(0x151b24))
                    .child(div().size(px(8.)).rounded_full().bg(rgb(0xfacc15)))
                    .child(
                        div()
                            .min_w_0()
                            .text_xs()
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(0xe5edf7))
                            .overflow_hidden()
                            .child(format!("Connecting {pending_name}")),
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
                    .text_color(rgb(0x8f98aa))
                    .child(div().size(px(8.)).rounded_full().bg(rgb(0x64748b)))
                    .child("No sessions"),
            );
        } else {
            for session in sessions {
                let display_name = self.session_display_name_by_info(&session);
                let session_id = session.id.clone();
                let actions_session_id = session.id.clone();
                let close_session_id = session.id.clone();
                let drag_payload = SessionTabDragPayload {
                    session_id: session.id.clone(),
                    display_name: display_name.clone(),
                    kind_label: session_kind_label(session.kind),
                };
                let drop_target_session_id = session.id.clone();
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
                let bg = if let Some(custom_color) = custom_color {
                    rgba((custom_color << 8) | if is_active { 0x24 } else { 0x14 })
                } else if is_active {
                    rgb(0x151b24)
                } else {
                    rgb(0x0f131a)
                };
                let hover_bg = if let Some(custom_color) = custom_color {
                    rgba((custom_color << 8) | if is_active { 0x32 } else { 0x22 })
                } else {
                    rgb(0x18202b)
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
                        .border_r_1()
                        .border_color(if is_active {
                            custom_color.map(rgb).unwrap_or_else(|| rgb(0x334155))
                        } else {
                            rgb(0x202633)
                        })
                        .bg(bg)
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
                        .child(div().size(px(8.)).rounded_full().bg(accent))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap_0()
                                .overflow_hidden()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(0xe5edf7))
                                        .overflow_hidden()
                                        .child(truncate_preview(&display_name, 32)),
                                )
                                .child(div().text_size(px(10.)).text_color(rgb(0x8f98aa)).child(
                                    format!(
                                        "{} · {}",
                                        session_kind_label(session.kind),
                                        short_id(&session.id)
                                    ),
                                )),
                        )
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
                                .text_color(rgb(0x94a3b8))
                                .hover(|this| this.bg(rgb(0x2a3140)).text_color(rgb(0x6ee7b7)))
                                .child("...")
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
                                .text_color(rgb(0x94a3b8))
                                .hover(|this| this.bg(rgb(0x2a3140)).text_color(rgb(0xfca5a5)))
                                .child("x")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.close_session(close_session_id.clone(), cx);
                                })),
                        )
                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            this.handle_session_tab_click(session_id.clone(), event, window, cx);
                        })),
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
                    .border_color(rgb(0x202633))
                    .hover(|this| this.bg(rgb(0x14211e)))
                    .on_drop(cx.listener(|this, payload: &SessionTabDragPayload, _, cx| {
                        this.reorder_session_to_end(payload.session_id.clone(), cx);
                    })),
            );
        }

        let mut session_actions = div()
            .h_full()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .border_l_1()
            .border_color(rgb(0x202633))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x8f98aa))
                    .child(format!("{session_count} open")),
            )
            .child(small_button(
                "workspace-new-local-session",
                "New",
                cx.listener(|this, _, window, cx| {
                    this.start_local_session(window, cx);
                }),
            ));
        if let Some(active_session_id) = self.active_session_id.clone() {
            let can_copy_ssh = self.session_ssh_address(&active_session_id).is_some();
            let inactive_anchor = active_session_id.clone();
            let right_anchor = active_session_id.clone();
            session_actions = session_actions
                .child(small_button(
                    "workspace-tab-color",
                    "Color",
                    cx.listener(|this, _, window, cx| {
                        this.open_tab_color_picker(window, cx);
                    }),
                ))
                .child(small_button(
                    "workspace-copy-session-name",
                    "Name",
                    cx.listener(|this, _, _, cx| {
                        this.copy_active_session_name(cx);
                    }),
                ))
                .child(small_button(
                    "workspace-copy-session-endpoint",
                    "Endpt",
                    cx.listener(|this, _, _, cx| {
                        this.copy_active_session_endpoint(cx);
                    }),
                ))
                .when(can_copy_ssh, |this| {
                    this.child(small_button(
                        "workspace-copy-ssh-host",
                        "IP",
                        cx.listener(|this, _, _, cx| {
                            this.copy_active_session_ssh_host(cx);
                        }),
                    ))
                    .child(small_button(
                        "workspace-copy-ssh-address",
                        "SSH",
                        cx.listener(|this, _, _, cx| {
                            this.copy_active_session_ssh_address(cx);
                        }),
                    ))
                })
                .child(small_button(
                    "workspace-session-info",
                    "Info",
                    cx.listener(|this, _, window, cx| {
                        this.open_active_session_info(window, cx);
                    }),
                ))
                .child(small_button(
                    "workspace-close-inactive-sessions",
                    "Others",
                    cx.listener(move |this, _, _, cx| {
                        this.close_inactive_sessions(inactive_anchor.clone(), cx);
                    }),
                ))
                .child(small_button(
                    "workspace-close-right-sessions",
                    "Right",
                    cx.listener(move |this, _, _, cx| {
                        this.close_sessions_to_right(right_anchor.clone(), cx);
                    }),
                ));
        }
        if session_count > 0 {
            session_actions = session_actions.child(small_button(
                "workspace-close-all-sessions",
                "All",
                cx.listener(|this, _, window, cx| {
                    this.open_close_all_sessions_confirm(window, cx);
                }),
            ));
        }

        div()
            .h(px(34.))
            .flex()
            .items_center()
            .border_b_1()
            .border_color(rgb(0x202633))
            .bg(rgb(0x11151c))
            .child(tabs)
            .child(session_actions)
    }

    pub(in crate::ui::view) fn empty_workspace_state(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(0x07090d))
            .px_6()
            .child(
                div()
                    .w(px(520.))
                    .max_w_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(
                        div()
                            .size(px(168.))
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x202633))
                            .bg(rgb(0x0d1118))
                            .opacity(0.62)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(84.))
                            .font_weight(FontWeight(900.))
                            .text_color(rgb(0x293241))
                            .child("N"),
                    )
                    .child(
                        div()
                            .mt_8()
                            .grid()
                            .grid_cols(2)
                            .gap_3()
                            .child(empty_workspace_action(
                                "Temporary SSH",
                                "connections",
                                cx.listener(|this, _, _, cx| {
                                    this.select(NavItem::Connections, cx);
                                }),
                            ))
                            .child(empty_workspace_action(
                                "Open Chat",
                                "AI assistant",
                                cx.listener(|this, _, window, cx| {
                                    window.focus(&this.ai_chat_focus);
                                    this.ai_status = "AI assistant focused".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(empty_workspace_action(
                                "Show Commands",
                                "quick commands",
                                cx.listener(|this, _, window, cx| {
                                    window.focus(&this.command_search_focus);
                                    this.terminal_status = "command search focused".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(empty_workspace_action(
                                "Start Local",
                                "new terminal",
                                cx.listener(|this, _, window, cx| {
                                    this.start_local_session(window, cx);
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
            BottomPanelMode::QuickCommands => self.bottom_quick_commands_bar(cx).into_any_element(),
            BottomPanelMode::CommandSend => self.bottom_command_send_bar(cx).into_any_element(),
            BottomPanelMode::Hidden => div().into_any_element(),
        }
    }

    fn bottom_quick_commands_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .text_color(rgb(0x64748b))
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
            .border_color(rgb(0x202633))
            .bg(rgb(0x11151c))
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
                                    .text_color(rgb(0x9ca3af))
                                    .child("Quick Commands"),
                            )
                            .child(status_pill("bottom panel", rgb(0x93c5fd), rgb(0x17253b))),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "bottom-command-search",
                                "Search",
                                cx.listener(|this, _, window, cx| {
                                    window.focus(&this.command_search_focus);
                                    this.terminal_status = "command search focused".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(small_button(
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

        div()
            .id(SharedString::from(format!("bottom-quick-command-{index}")))
            .w(px(176.))
            .h(px(56.))
            .flex_none()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x151b24))
            .p_2()
            .cursor_pointer()
            .hover(|this| this.bg(rgb(0x1c2431)))
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
                            .text_color(rgb(0xe5edf7))
                            .child(title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x6ee7b7))
                            .child(execute_label),
                    ),
            )
            .child(
                div()
                    .mt_1()
                    .text_xs()
                    .text_color(rgb(0x98a3b8))
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
