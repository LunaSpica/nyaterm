use super::*;

impl NyaTermApp {
    fn ai_ask_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let agent_mode = self.ai_settings.default_mode == AiMode::Agent;
        let file_action_ready = self
            .ai_prepared_request
            .as_ref()
            .is_some_and(|request| request.action == AiAction::CustomFileAction);
        let ai_running = self.ai_chat_pending || self.ai_agent_loop.is_some();
        let action_label = if ai_running {
            "Cancel"
        } else if file_action_ready {
            "Run"
        } else if agent_mode {
            "Agent"
        } else {
            "Ask"
        };
        let mut command_rows = div().mt_3().flex().flex_col().gap_2();
        for (index, card) in self.ai_command_cards.iter().cloned().take(8).enumerate() {
            let risk = risk_label(card.risk_level.as_ref());
            let title = if card.title.trim().is_empty() {
                "Command".to_string()
            } else {
                card.title.clone()
            };
            command_rows = command_rows.child(
                div()
                    .border_t_1()
                    .border_color(rgb(0x2a3140))
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
                                    .text_color(rgb(0xe5edf7))
                                    .child(title),
                            )
                            .child(status_pill(risk, rgb(0xfacc15), rgb(0x3a2f14))),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0xaeb7c8))
                            .line_height(px(18.))
                            .child(truncate_preview(&card.command, 120)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x64748b))
                                    .child(truncate_preview(&card.explanation, 80)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(small_button(
                                        format!("ai-command-save-{index}"),
                                        "Save",
                                        cx.listener(move |this, _, _, cx| {
                                            this.save_ai_command_card(index, cx);
                                        }),
                                    ))
                                    .child(small_button(
                                        format!("ai-command-insert-{index}"),
                                        "Insert",
                                        cx.listener(move |this, _, _, cx| {
                                            this.insert_ai_command_card(index, cx);
                                        }),
                                    ))
                                    .child(small_button(
                                        format!("ai-command-run-{index}"),
                                        "Run",
                                        cx.listener(move |this, _, _, cx| {
                                            this.run_ai_command_card(index, cx);
                                        }),
                                    )),
                            ),
                    ),
            );
        }
        let mut agent_step_rows = div();
        if agent_mode || !self.ai_agent_steps.is_empty() {
            agent_step_rows = agent_step_rows
                .mt_3()
                .border_t_1()
                .border_color(rgb(0x2a3140))
                .pt_2()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(0xe5edf7))
                        .child("Agent Steps"),
                );
            if self.ai_agent_steps.is_empty() {
                agent_step_rows = agent_step_rows.child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x98a3b8))
                        .child("No Agent steps yet."),
                );
            } else {
                for step in self.ai_agent_steps.iter().cloned().rev().take(16).rev() {
                    let (label, fg, bg) = ai_agent_step_status_style(step.status);
                    agent_step_rows = agent_step_rows.child(
                        div()
                            .flex()
                            .items_start()
                            .justify_between()
                            .gap_2()
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
                                            .text_color(rgb(0xe5edf7))
                                            .child(format!(
                                                "{}. {}",
                                                step.step_index.saturating_add(1),
                                                truncate_preview(&step.title, 40)
                                            )),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0x98a3b8))
                                            .line_height(px(18.))
                                            .child(truncate_preview(&step.detail, 120)),
                                    ),
                            )
                            .child(status_pill(label, rgb(fg), rgb(bg))),
                    );
                }
            }
        }
        let model_label = self
            .ai_settings
            .default_model_id
            .clone()
            .unwrap_or_else(|| "default model".to_string());
        let mode_label = if agent_mode { "Agent" } else { "Ask" };
        let enabled = self.ai_settings.enabled;

        // Full-height AI shell (Tauri-like): toolbar / scroll body / composer.
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
                    .gap_1()
                    .child(mode_button(
                        "ai-mode-ask",
                        "Ask",
                        !agent_mode,
                        cx.listener(|this, _, _, cx| {
                            this.set_ai_mode(AiMode::Ask, cx);
                        }),
                    ))
                    .child(mode_button(
                        "ai-mode-agent",
                        "Agent",
                        agent_mode,
                        cx.listener(|this, _, _, cx| {
                            this.set_ai_mode(AiMode::Agent, cx);
                        }),
                    ))
                    .child(
                        div()
                            .ml_1()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(11.))
                            .text_color(rgb(0x8b949e))
                            .overflow_hidden()
                            .child(truncate_preview(&model_label, 28)),
                    )
                    .child(status_pill(
                        if !enabled {
                            "off"
                        } else if ai_running {
                            "run"
                        } else {
                            "ok"
                        },
                        if !enabled {
                            rgb(0x8b949e)
                        } else if ai_running {
                            rgb(0xfacc15)
                        } else {
                            rgb(0x6ee7b7)
                        },
                        if !enabled {
                            rgb(0x21262d)
                        } else if ai_running {
                            rgb(0x3a2f14)
                        } else {
                            rgb(0x12342a)
                        },
                    ))
                    .child(icon_button(
                        "ai-new-chat",
                        "＋",
                        cx.listener(|this, _, _, cx| {
                            this.ai_prompt_draft.clear();
                            this.ai_response_preview = if this.ai_settings.default_mode
                                == AiMode::Agent
                            {
                                "Agent mode ready".to_string()
                            } else {
                                "Ask mode ready".to_string()
                            };
                            this.ai_command_cards.clear();
                            this.ai_agent_steps.clear();
                            this.ai_chat_messages.clear();
                            this.ai_streaming_assistant_id = None;
                            this.ai_prepared_request = None;
                            this.ai_chat_session_id = format!("ai-session-{}", uuid());
                            this.ai_status = "new AI chat".to_string();
                            cx.notify();
                        }),
                    ))
                    .child(icon_button(
                        "ai-open-settings",
                        "⚙",
                        cx.listener(|this, _, _, cx| {
                            this.settings_active_tab = SettingsTab::AiGeneral;
                            this.open_page(NavItem::Settings, cx);
                        }),
                    )),
            )
            .child(
                div()
                    .id(SharedString::from("ai-transcript-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_scroll()
                    .scrollbar_width(px(6.))
                    .px_3()
                    .py_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(self.ai_transcript_body(
                        mode_label,
                        enabled,
                        agent_step_rows,
                        command_rows,
                        cx,
                    )),
            )
            .child(
                div()
                    .flex_none()
                    .border_t_1()
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x12171f))
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        transfer_input(
                            "ai-ask-prompt",
                            if agent_mode {
                                "Describe a task for the agent…"
                            } else {
                                "Ask about the terminal or generate a command…"
                            },
                            self.ai_prompt_draft.clone(),
                            true,
                        )
                        .h(px(56.))
                        .track_focus(&self.ai_chat_focus)
                        .on_click(cx.listener(|this, _, window, cx| {
                            window.focus(&this.ai_chat_focus);
                            cx.notify();
                        }))
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                            cx.stop_propagation();
                            this.handle_ai_prompt_key_down(event, cx);
                        })),
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
                                    .text_color(rgb(0x6e7681))
                                    .child(if file_action_ready {
                                        "File action ready — press Run".to_string()
                                    } else {
                                        format!(
                                            "{} · Enter to send",
                                            if agent_mode { "Agent" } else { "Ask" }
                                        )
                                    }),
                            )
                            .child(small_button(
                                "ai-ask-run",
                                action_label,
                                cx.listener(|this, _, _, cx| {
                                    if this.ai_chat_pending || this.ai_agent_loop.is_some() {
                                        this.cancel_ai_chat(cx);
                                    } else {
                                        this.start_ai_ask(cx);
                                    }
                                }),
                            )),
                    ),
            )
    }

    fn command_center_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let sessions = self.session_manager.list_sessions().unwrap_or_default();
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
                    .text_color(rgb(0x98a3b8))
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
                        .border_color(rgb(0x2a3140))
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
                                        .text_color(rgb(0xe5edf7))
                                        .child(truncate_preview(&display_name, 42)),
                                )
                                .child(div().text_xs().text_color(rgb(0x98a3b8)).child(format!(
                                    "{} · {}",
                                    session_kind_label(session.kind),
                                    compact_id(&session.id)
                                ))),
                        )
                        .child(status_pill(
                            if is_active { "active" } else { "open" },
                            if is_active {
                                rgb(0x6ee7b7)
                            } else {
                                rgb(0x93c5fd)
                            },
                            if is_active {
                                rgb(0x12342a)
                            } else {
                                rgb(0x17233a)
                            },
                        )),
                );
            }
        }

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x151923))
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
                    .child(status_pill("native", rgb(0x6ee7b7), rgb(0x12342a))),
            )
            .child(
                div()
                    .mt_3()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .child(metric("Active", active_label))
                    .child(metric("Sync", provider)),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(small_button(
                        "command-center-new-session",
                        "New",
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Connections, cx);
                        }),
                    ))
                    .child(small_button(
                        "command-center-active-sessions",
                        "Sessions",
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Workspace, cx);
                        }),
                    ))
                    .child(small_button(
                        "command-center-settings",
                        "Settings",
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Settings, cx);
                        }),
                    ))
                    .child(small_button(
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
                        "command-center-sync-history",
                        "History",
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Settings, cx);
                        }),
                    ))
                    .child(small_button(
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
                    .text_color(rgb(0x98a3b8))
                    .line_height(px(18.))
                    .child(format!(
                        "{sync_label} sync · {} · {}",
                        truncate_preview(&self.cloud_sync_status, 84),
                        truncate_preview(&self.update_status, 84)
                    )),
            )
            .child(session_rows)
    }

    fn command_search_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let results = self.command_search_results();
        let mut rows = div().mt_3().flex().flex_col().gap_2();
        if results.is_empty() {
            rows = rows.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x98a3b8))
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
                        .border_color(rgb(0x2a3140))
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
                                        .text_color(rgb(0xe5edf7))
                                        .child(truncate_preview(&result.display, 44)),
                                )
                                .child(div().text_xs().text_color(rgb(0x64748b)).child(meta)),
                        )
                        .child(
                            div()
                                .font_family("JetBrains Mono")
                                .text_xs()
                                .text_color(rgb(0xaeb7c8))
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
                                    format!("command-search-insert-{index}"),
                                    "Insert",
                                    cx.listener(move |this, _, _, cx| {
                                        this.insert_command_search_result(index, cx);
                                    }),
                                ))
                                .child(small_button(
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
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x151923))
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
                        rgb(0x93c5fd),
                        rgb(0x17233a),
                    )),
            )
            .child(
                transfer_input(
                    "command-search-input",
                    "Search",
                    self.command_search_draft.clone(),
                    true,
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

    fn command_history_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        // Tauri CommandHistory: header meta is shared PanelHeader; body is dense mono list.
        let history = self.active_session_history_commands();
        let mut rows = div().flex().flex_col().gap_0().p_2();
        if history.is_empty() {
            rows = rows.child(
                div()
                    .py_4()
                    .text_center()
                    .text_size(px(11.))
                    .text_color(rgb(0x6e7681))
                    .child("No commands yet"),
            );
        } else {
            for (index, command) in history.into_iter().enumerate() {
                let run_index = index;
                let insert_index = index;
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
                        .hover(|this| this.bg(rgb(0x1c2128)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            // Single click inserts; double-click not separate in GPUI easily —
                            // keep insert on click and Run via trailing action.
                            this.insert_history_command(insert_index, cx);
                        }))
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(0x6e7681))
                                .child("›"),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .font_family("JetBrains Mono")
                                .text_size(px(11.))
                                .text_color(rgb(0xc9d1d9))
                                .overflow_hidden()
                                .child(truncate_preview(&command, 96)),
                        )
                        .child(icon_button(
                            format!("history-run-{index}"),
                            "▶",
                            cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.run_history_command(run_index, cx);
                            }),
                        )),
                );
            }
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(0x161b22))
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

    pub(in crate::ui::view) fn right_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let width = self.right_panel_width.clamp(200., 720.);
        div()
            .w(px(width))
            .flex_none()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(rgb(0x30363d))
            .bg(rgb(0x161b22))
            .child(self.side_panel_stack(PanelSide::Right, cx))
    }

    fn right_panel_meta(&self) -> &'static str {
        match self.current_right_panel().unwrap_or(NavItem::Connections) {
            NavItem::Connections => "saved connections",
            NavItem::AiAssistant => "assistant",
            NavItem::ActiveSessions => "sessions",
            NavItem::CommandHistory => "history",
            NavItem::Stats => "resource monitor",
            NavItem::Processes => "process manager",
            NavItem::Docker => "docker manager",
            NavItem::Translation => "translation",
            NavItem::Recording => "recording",
            other => other.label(),
        }
    }

    pub(in crate::ui::view) fn right_panel_body(
        &mut self,
        panel: NavItem,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match panel {
            NavItem::Connections => self.connections_view(cx).into_any_element(),
            NavItem::AiAssistant => self.ai_assistant_panel(cx).into_any_element(),
            NavItem::ActiveSessions => self.active_sessions_panel(cx).into_any_element(),
            NavItem::CommandHistory => self.command_history_panel(cx).into_any_element(),
            NavItem::Stats if self.settings.ui_show_remote_stats => {
                self.stats_view(cx).into_any_element()
            }
            NavItem::Stats => disabled_inspector_panel(
                "Remote Stats Disabled",
                "Enable Remote Stats in Settings > Terminal Session > General.",
            )
            .into_any_element(),
            NavItem::Processes if self.settings.ui_show_process_manager => {
                self.processes_view(cx).into_any_element()
            }
            NavItem::Processes => disabled_inspector_panel(
                "Process Manager Disabled",
                "Enable Process Manager in Settings > Terminal Session > General.",
            )
            .into_any_element(),
            NavItem::Docker if self.settings.ui_show_docker_manager => {
                self.docker_view(cx).into_any_element()
            }
            NavItem::Docker => disabled_inspector_panel(
                "Docker Manager Disabled",
                "Enable Docker Manager in Settings > Terminal Session > General.",
            )
            .into_any_element(),
            NavItem::Translation => self.translation_view(cx).into_any_element(),
            NavItem::Recording => self.recording_panel(cx).into_any_element(),
            _ => self.ai_assistant_panel(cx).into_any_element(),
        }
    }


    fn ai_transcript_body(
        &self,
        mode_label: &'static str,
        enabled: bool,
        agent_step_rows: impl IntoElement,
        command_rows: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let _ = cx;
        let mut body = div().flex().flex_col().gap_2();
        if self.ai_chat_messages.is_empty() {
            body = body.child(self.ai_empty_transcript(mode_label, enabled));
        } else {
            for message in &self.ai_chat_messages {
                body = body.child(self.ai_message_bubble(message));
            }
        }
        body.child(agent_step_rows).child(command_rows)
    }

    fn ai_empty_transcript(&self, mode_label: &'static str, enabled: bool) -> impl IntoElement {
        let has_model = self
            .ai_settings
            .default_model_id
            .as_ref()
            .is_some_and(|id| !id.trim().is_empty());
        div()
            .min_h(px(160.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .px_3()
            .child(
                div()
                    .text_size(px(22.))
                    .text_color(rgb(0x8b949e))
                    .child("✦"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(0xc9d1d9))
                    .child(if !enabled {
                        "AI is disabled"
                    } else if !has_model {
                        "Set up an AI model"
                    } else {
                        "Start a conversation"
                    }),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(0x8b949e))
                    .child(if !enabled {
                        "Enable AI in Settings to use Ask/Agent.".to_string()
                    } else if !has_model {
                        "Open Settings → AI to pick a model and API key.".to_string()
                    } else {
                        format!(
                            "{mode_label} replies appear here · session {}",
                            compact_id(&self.ai_chat_session_id)
                        )
                    }),
            )
    }

    fn ai_message_bubble(&self, message: &AiMessage) -> impl IntoElement {
        let is_user = matches!(message.role, AiMessageRole::User);
        let streaming = self
            .ai_streaming_assistant_id
            .as_deref()
            .is_some_and(|id| id == message.id);
        let role_label = if is_user { "USER" } else { "AI" };
        let content = if message.content.trim().is_empty() {
            if streaming {
                "…".to_string()
            } else {
                String::new()
            }
        } else {
            message.content.clone()
        };
        let mut bubble = div()
            .id(SharedString::from(format!("ai-msg-{}", message.id)))
            .rounded_md()
            .border_1()
            .border_color(if is_user {
                rgb(0x1f6feb)
            } else {
                rgb(0x30363d)
            })
            .bg(if is_user {
                rgb(0x122033)
            } else {
                rgb(0x0d1117)
            })
            .px_3()
            .py_2()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_size(px(10.))
                    .font_weight(FontWeight(800.))
                    .text_color(rgb(0x8b949e))
                    .child(role_label),
            );
        if let Some(reasoning) = message
            .reasoning_content
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            bubble = bubble.child(
                div()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x12171f))
                    .px_2()
                    .py_1()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(0x8b949e))
                            .child("REASONING"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x8b949e))
                            .line_height(px(16.))
                            .child(truncate_preview(reasoning, 480)),
                    ),
            );
        }
        bubble = bubble.child(
            div()
                .text_xs()
                .text_color(rgb(0xc9d1d9))
                .line_height(px(18.))
                .child(if content.is_empty() {
                    if streaming {
                        "Thinking…".to_string()
                    } else {
                        String::new()
                    }
                } else {
                    // Keep more content than the old 320-char status preview.
                    truncate_preview(&content, 4000)
                }),
        );
        if !message.command_cards.is_empty() {
            bubble = bubble.child(
                div()
                    .mt_1()
                    .text_size(px(10.))
                    .text_color(rgb(0x8b949e))
                    .child(format!(
                        "{} command card(s)",
                        message.command_cards.len()
                    )),
            );
        }
        if streaming {
            bubble = bubble.child(
                div()
                    .text_size(px(10.))
                    .text_color(rgb(0x58a6ff))
                    .child("streaming…"),
            );
        }
        bubble
    }

    fn ai_assistant_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        // Tauri AIAssistantPanel: toolbar + scroll transcript + bottom composer.
        // Shared stack already renders PanelHeader; body fills remaining height.
        self.ai_ask_panel(cx)
    }

    fn right_ai_command_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.command_center_panel(cx))
            .child(self.ai_ask_panel(cx))
            .child(self.recording_panel(cx))
            .child(self.command_search_panel(cx))
            .child(self.quick_commands_panel(cx))
            .child(self.command_history_panel(cx))
    }

    fn right_stats_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let stats = self.remote_stats.clone().unwrap_or_default();
        let memory_total = stats.memory.used.saturating_add(stats.memory.available);
        let memory_percent = if memory_total > 0 {
            stats.memory.used as f64 / memory_total as f64 * 100.
        } else {
            0.
        };
        let disk_summary = stats
            .disks
            .iter()
            .max_by_key(|disk| disk.use_percent)
            .map(|disk| format!("{} {}%", disk.mount, disk.use_percent))
            .unwrap_or_else(|| "n/a".to_string());

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                inspector_card("Resource Monitor")
                    .child(capability_line(
                        "SSH",
                        if self.active_ssh_config.is_some() {
                            "ready"
                        } else {
                            "none"
                        },
                    ))
                    .child(capability_line(
                        "Host",
                        if stats.system.hostname.trim().is_empty() {
                            "n/a".to_string()
                        } else {
                            truncate_preview(&stats.system.hostname, 34)
                        },
                    ))
                    .child(capability_line("CPU", format!("{:.1}%", stats.cpu.usage)))
                    .child(capability_line("Memory", format!("{memory_percent:.0}%")))
                    .child(capability_line("Disk", disk_summary))
                    .child(div().mt_3().child(small_button(
                        "right-stats-refresh",
                        if self.stats_pending {
                            "Loading"
                        } else {
                            "Refresh"
                        },
                        cx.listener(|this, _, window, cx| {
                            this.refresh_stats(window, cx);
                        }),
                    ))),
            )
            .child(inspector_card("Networks").child(compact_network_rows(&stats.networks)))
            .child(inspector_status_line(self.stats_status.clone()))
    }

    fn right_process_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let top_process = self.processes.iter().max_by(|left, right| {
            left.cpu_percent
                .partial_cmp(&right.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let top_label = top_process
            .map(|process| {
                format!(
                    "{} {:.1}% CPU",
                    truncate_preview(&process.command, 18),
                    process.cpu_percent
                )
            })
            .unwrap_or_else(|| "n/a".to_string());

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                inspector_card("Process Manager")
                    .child(capability_line(
                        "SSH",
                        if self.active_ssh_config.is_some() {
                            "ready"
                        } else {
                            "none"
                        },
                    ))
                    .child(capability_line(
                        "Processes",
                        self.processes.len().to_string(),
                    ))
                    .child(capability_line("Top CPU", top_label))
                    .child(div().mt_3().child(small_button(
                        "right-process-refresh",
                        if self.process_pending {
                            "Loading"
                        } else {
                            "Refresh"
                        },
                        cx.listener(|this, _, window, cx| {
                            this.refresh_processes(window, cx);
                        }),
                    ))),
            )
            .child(inspector_card("Hot Processes").child(compact_process_rows(&self.processes)))
            .child(inspector_status_line(self.process_status.clone()))
    }

    fn right_docker_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let overview = self.docker_overview.clone().unwrap_or_default();
        let running = overview
            .containers
            .iter()
            .filter(|container| container.state == "running")
            .count();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                inspector_card("Docker")
                    .child(capability_line(
                        "SSH",
                        if self.active_ssh_config.is_some() {
                            "ready"
                        } else {
                            "none"
                        },
                    ))
                    .child(capability_line(
                        "Available",
                        if overview.available { "yes" } else { "no" },
                    ))
                    .child(capability_line(
                        "Version",
                        truncate_preview(&overview.version, 24),
                    ))
                    .child(capability_line(
                        "Containers",
                        overview.containers.len().to_string(),
                    ))
                    .child(capability_line("Running", running.to_string()))
                    .child(div().mt_3().child(small_button(
                        "right-docker-refresh",
                        if self.docker_pending {
                            "Loading"
                        } else {
                            "Refresh"
                        },
                        cx.listener(|this, _, window, cx| {
                            this.refresh_docker(window, cx);
                        }),
                    ))),
            )
            .child(
                inspector_card("Containers")
                    .child(compact_docker_container_rows(&overview.containers)),
            )
            .child(inspector_status_line(self.docker_status.clone()))
    }

    fn right_translation_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let translated = self
            .translate_result
            .as_ref()
            .map(|result| truncate_preview(&result.translated, 180))
            .unwrap_or_else(|| "No translation result yet.".to_string());
        let detected = self
            .translate_result
            .as_ref()
            .map(|result| result.detected_language.clone())
            .unwrap_or_else(|| "n/a".to_string());

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                inspector_card("Translation")
                    .child(capability_line("Provider", self.translate_provider.clone()))
                    .child(capability_line(
                        "Target",
                        self.translate_target_language.clone(),
                    ))
                    .child(capability_line("Detected", detected))
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .line_height(px(18.))
                            .text_color(rgb(0xcbd5e1))
                            .child(translated),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .gap_2()
                            .child(small_button(
                                "right-translation-run",
                                if self.translate_pending {
                                    "Translating"
                                } else {
                                    "Translate"
                                },
                                cx.listener(|this, _, window, cx| {
                                    this.run_translation(window, cx);
                                }),
                            ))
                            .child(small_button(
                                "right-translation-save",
                                "Save",
                                cx.listener(|this, _, _, cx| {
                                    this.save_translation_settings(cx);
                                }),
                            )),
                    ),
            )
            .child(inspector_status_line(self.translate_status.clone()))
    }
}

pub(in crate::ui::view) fn disabled_inspector_panel(title: &'static str, detail: &'static str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            inspector_card(title)
                .child(
                    div()
                        .mt_3()
                        .text_xs()
                        .line_height(px(18.))
                        .text_color(rgb(0x98a3b8))
                        .child(detail),
                )
                .child(
                    div()
                        .mt_3()
                        .text_size(px(10.))
                        .text_color(rgb(0x64748b))
                        .child("The page remains available, but background refresh and actions stay paused while disabled."),
                ),
        )
}
