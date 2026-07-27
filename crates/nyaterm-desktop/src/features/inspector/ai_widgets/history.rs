use super::*;

impl NyaTermApp {
    pub(in crate::features) fn open_ai_clear_history_confirm(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.ai.history.sessions.is_empty() {
            return;
        }
        self.ai.history.clear_confirm_open = true;
        self.ai.chat.message_menu = None;
        self.ai.discovery.menu_open = false;
        self.ai.panel.execution_menu_open = false;
        window.focus(&self.ai.history.clear_confirm_focus);
        cx.notify();
    }

    pub(in crate::features) fn cancel_ai_clear_history_confirm(&mut self, cx: &mut Context<Self>) {
        self.ai.history.cancel_clear_confirm();
        cx.notify();
    }

    pub(in crate::features) fn confirm_ai_clear_history(&mut self, cx: &mut Context<Self>) {
        if !self.ai.history.clear_confirm_open {
            return;
        }
        self.ai.history.clear_confirm_open = false;
        self.ai.history.open = false;
        self.clear_all_ai_history(cx);
    }

    pub(in crate::features) fn open_ai_auto_execution_confirm(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.ai.agent.auto_execution_confirm_open = true;
        self.ai.panel.execution_menu_open = false;
        self.ai.history.open = false;
        self.ai.chat.message_menu = None;
        self.ai.discovery.menu_open = false;
        window.focus(&self.ai.agent.auto_execution_confirm_focus);
        cx.notify();
    }

    pub(in crate::features) fn cancel_ai_auto_execution_confirm(&mut self, cx: &mut Context<Self>) {
        self.ai.agent.cancel_auto_execution_confirm();
        cx.notify();
    }

    pub(in crate::features) fn confirm_ai_auto_execution(&mut self, cx: &mut Context<Self>) {
        if !self.ai.agent.auto_execution_confirm_open {
            return;
        }
        self.ai.agent.auto_execution_confirm_open = false;
        self.set_ai_command_mode(AgentCommandExecutionMode::Auto, cx);
        self.ai.panel.status = "Agent execution mode: auto".to_string();
        cx.notify();
    }

    pub(in crate::features) fn ai_auto_execution_confirm_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();

        div()
            .id(SharedString::from("ai-auto-execution-confirm-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .p_3()
            .track_focus(&self.ai.agent.auto_execution_confirm_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.ai.agent.auto_execution_confirm_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                match event.keystroke.key.as_str() {
                    "escape" => this.cancel_ai_auto_execution_confirm(cx),
                    "enter" => this.confirm_ai_auto_execution(cx),
                    _ => {}
                }
            }))
            .child(
                div()
                    .id(SharedString::from("ai-auto-execution-confirm-dialog"))
                    .w(px(384.))
                    .max_w_full()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text))
                                    .child(self.tr("ai.autoExecutionConfirmTitle")),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(17.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(self.tr("ai.autoExecutionConfirmDesc")),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "ai-auto-execution-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_ai_auto_execution_confirm(cx);
                                }),
                            ))
                            .child(dialog_action_button(
                                palette,
                                "ai-auto-execution-confirm",
                                self.tr("ai.enableAutoExecution"),
                                true,
                                cx.listener(|this, _, _, cx| {
                                    this.confirm_ai_auto_execution(cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn ai_clear_history_confirm_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .id(SharedString::from("ai-clear-history-confirm-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .p_3()
            .track_focus(&self.ai.history.clear_confirm_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.ai.history.clear_confirm_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                match event.keystroke.key.as_str() {
                    "escape" => this.cancel_ai_clear_history_confirm(cx),
                    "enter" => this.confirm_ai_clear_history(cx),
                    _ => {}
                }
            }))
            .child(
                div()
                    .id(SharedString::from("ai-clear-history-confirm-dialog"))
                    .w(px(384.))
                    .max_w_full()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text))
                                    .child(self.tr("ai.clearHistoryTitle")),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(self.tr("ai.clearHistoryDesc")),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "ai-clear-history-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_ai_clear_history_confirm(cx);
                                }),
                            ))
                            .child(dialog_action_button(
                                palette,
                                "ai-clear-history-confirm",
                                self.tr("ai.clearHistory"),
                                true,
                                cx.listener(|this, _, _, cx| {
                                    this.confirm_ai_clear_history(cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn ai_execution_mode_menu(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let current = self.ai.settings.config.agent_command_execution_mode.clone();
        div()
            .id(SharedString::from("ai-execution-mode-menu"))
            .absolute()
            .top(px(4.))
            .right(px(8.))
            .w(px(260.))
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(self.shell_surface_color(palette.surface))
            .shadow_lg()
            .py_1()
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_, _, _| {})
            .child(
                div()
                    .px_3()
                    .py_1()
                    .text_size(px(11.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.text))
                    .child(self.tr("ai.agentCommandExecutionMode")),
            )
            .child(self.ai_execution_mode_item(
                "ai-exec-confirm",
                self.tr("ai.executionModeConfirmEach"),
                self.tr("ai.executionModeConfirmEachDesc"),
                AgentCommandExecutionMode::ConfirmEach,
                current == AgentCommandExecutionMode::ConfirmEach,
                cx,
            ))
            .child(self.ai_execution_mode_item(
                "ai-exec-smart",
                self.tr("ai.executionModeSmart"),
                self.tr("ai.executionModeSmartDesc"),
                AgentCommandExecutionMode::Smart,
                current == AgentCommandExecutionMode::Smart,
                cx,
            ))
            .child(self.ai_execution_mode_item(
                "ai-exec-auto",
                self.tr("ai.executionModeAuto"),
                self.tr("ai.executionModeAutoDesc"),
                AgentCommandExecutionMode::Auto,
                current == AgentCommandExecutionMode::Auto,
                cx,
            ))
            .child(tab_menu_separator(palette))
            .child(
                div()
                    .px_3()
                    .py_1()
                    .text_size(px(11.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.text))
                    .child(self.tr("ai.executionMethod")),
            )
            .child(self.ai_background_execution_item(cx))
    }

    pub(in crate::features) fn ai_background_execution_item(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let enabled = self.ai.settings.config.agent_background_execution_enabled;
        div()
            .id(SharedString::from("ai-exec-background"))
            .px_3()
            .py_2()
            .flex()
            .items_start()
            .gap_2()
            .cursor_pointer()
            .hover(|this| this.bg(rgb(palette.surface_elevated)))
            .on_click(cx.listener(|this, _, _, cx| {
                this.toggle_ai_background_execution(cx);
                cx.notify();
            }))
            .child(
                div()
                    .mt(px(1.))
                    .size(px(14.))
                    .rounded_sm()
                    .border_1()
                    .border_color(if enabled {
                        rgb(palette.link)
                    } else {
                        rgb(palette.border)
                    })
                    .bg(if enabled {
                        rgb(palette.link)
                    } else {
                        rgb(palette.input)
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(palette.bg))
                    .when(enabled, |this| {
                        this.child(
                            svg()
                                .size(px(11.))
                                .path("icons/check.svg")
                                .text_color(rgb(palette.bg)),
                        )
                    }),
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
                            .text_size(px(12.))
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("ai.backgroundAgentExecution")),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("ai.backgroundAgentExecutionDesc")),
                    ),
            )
    }

    pub(in crate::features) fn ai_execution_mode_item(
        &self,
        id: &'static str,
        title: &'static str,
        detail: &'static str,
        mode: AgentCommandExecutionMode,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .id(SharedString::from(id))
            .px_3()
            .py_2()
            .flex()
            .items_start()
            .gap_2()
            .cursor_pointer()
            .hover(|this| this.bg(rgb(palette.surface_elevated)))
            .on_click(cx.listener(move |this, _, window, cx| {
                if mode == AgentCommandExecutionMode::Auto
                    && this.ai.settings.config.agent_command_execution_mode
                        != AgentCommandExecutionMode::Auto
                {
                    this.open_ai_auto_execution_confirm(window, cx);
                    return;
                }
                this.set_ai_command_mode(mode.clone(), cx);
                this.ai.panel.execution_menu_open = false;
                this.ai.panel.status = format!(
                    "Agent execution mode: {}",
                    match mode {
                        AgentCommandExecutionMode::ConfirmEach => "confirm each",
                        AgentCommandExecutionMode::Smart => "smart",
                        AgentCommandExecutionMode::Auto => "auto",
                    }
                );
                cx.notify();
            }))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap_0()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight(600.))
                            .text_color(if selected {
                                rgb(palette.link)
                            } else {
                                rgb(palette.text)
                            })
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_muted))
                            .child(detail),
                    ),
            )
            .child(
                div()
                    .size(px(14.))
                    .flex_none()
                    .text_color(rgb(palette.link))
                    .when(selected, |this| {
                        this.child(
                            svg()
                                .size(px(13.))
                                .path("icons/check.svg")
                                .text_color(rgb(palette.link)),
                        )
                    }),
            )
    }

    pub(in crate::features) fn ai_history_popover(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let search_field = self.text_input(
            "ai.history-search",
            &self.ai.history.query.clone(),
            TextInputSetup::placeholder("Search history..."),
            cx,
        );
        let search_focus = search_field.read(cx).focus_handle();
        // Tauri AIAssistantPanel history card: search + Clear All + date-grouped sessions.
        let query = self.ai.history.query.trim().to_ascii_lowercase();
        let filtered: Vec<_> = self
            .ai
            .history
            .sessions
            .iter()
            .filter(|session| {
                if query.is_empty() {
                    return true;
                }
                session.title.to_ascii_lowercase().contains(&query)
                    || session.id.to_ascii_lowercase().contains(&query)
            })
            .cloned()
            .collect();
        let total_count = self.ai.history.sessions.len();
        let filtered_count = filtered.len();
        let history_actions_disabled = total_count == 0
            || self.ai.history.pending
            || self.ai.chat.pending
            || self.ai.agent.loop_state.is_some();
        let grouped = group_ai_sessions_by_date(&filtered);
        let mut rows = div().flex().flex_col().gap_1().p_2();
        if filtered_count == 0 {
            rows = rows.child(
                div()
                    .py_4()
                    .text_center()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child(if self.ai.history.pending {
                        "Loading history..."
                    } else if total_count == 0 {
                        "No chat history yet"
                    } else {
                        "No matching history"
                    }),
            );
        } else {
            for (group, sessions) in grouped {
                if sessions.is_empty() {
                    continue;
                }
                rows = rows.child(
                    div()
                        .px_2()
                        .py_1()
                        .text_size(px(10.))
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(palette.text_dimmed))
                        .child(group.label()),
                );
                for session in sessions.into_iter().take(48) {
                    let session_id = session.id.clone();
                    let delete_id = session.id.clone();
                    let active = self.ai.chat.session_id == session.id;
                    rows = rows.child(
                        div()
                            .id(SharedString::from(format!("ai-session-{}", session.id)))
                            .h(px(32.))
                            .px_2()
                            .rounded_md()
                            .flex()
                            .items_center()
                            .gap_1()
                            .bg(if active {
                                rgb(palette.hover)
                            } else {
                                rgba(0x00000000)
                            })
                            .hover(|this| this.bg(rgb(palette.surface_elevated)))
                            .child(
                                div()
                                    .id(SharedString::from(format!(
                                        "ai-session-open-{}",
                                        session.id
                                    )))
                                    .min_w_0()
                                    .flex_1()
                                    .text_size(px(12.))
                                    .text_color(rgb(palette.text))
                                    .overflow_hidden()
                                    .cursor_pointer()
                                    .child(truncate_preview(&session.title, 36))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.load_ai_session_messages(session_id.clone(), cx);
                                    })),
                            )
                            .child(svg_icon_button(
                                format!("ai-session-delete-{}", session.id),
                                "icons/fe/delete.svg",
                                14.,
                                palette,
                                cx.listener(move |this, _, _, cx| {
                                    this.delete_ai_session(delete_id.clone(), cx);
                                }),
                            )),
                    );
                }
            }
        }

        div()
            .id(SharedString::from("ai-history-popover"))
            .absolute()
            .top(px(4.))
            .left(px(8.))
            .right(px(8.))
            .max_h(px(352.))
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(self.shell_surface_color(palette.surface))
            .shadow_lg()
            .flex()
            .flex_col()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, |_, _, _| {})
            .child(
                div()
                    .p_2()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .child(
                        div()
                            .id(SharedString::from("ai-history-search"))
                            .h(px(28.))
                            .px_2()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.bg))
                            .flex()
                            .items_center()
                            .gap_2()
                            .cursor_text()
                            .on_mouse_down(MouseButton::Left, move |_, window, _| {
                                window.focus(&search_focus);
                            })
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                if event.keystroke.key == "escape" {
                                    cx.stop_propagation();
                                    this.ai.history.open = false;
                                    this.ai.history.query.clear();
                                    this.forget_text_inputs("ai.history-search");
                                    cx.notify();
                                }
                            }))
                            .child(
                                svg()
                                    .size(px(14.))
                                    .flex_none()
                                    .path("icons/ai/search.svg")
                                    .text_color(rgb(palette.text_dimmed)),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .text_size(px(12.))
                                    .text_color(rgb(palette.text))
                                    .child(search_field),
                            )
                            .when(!self.ai.history.query.is_empty(), |this| {
                                this.child(
                                    div()
                                        .id(SharedString::from("ai-history-search-clear"))
                                        .size(px(18.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_sm()
                                        .text_size(px(10.))
                                        .text_color(rgb(palette.text_muted))
                                        .cursor_pointer()
                                        .hover(|this| {
                                            this.bg(rgb(palette.surface_elevated))
                                                .text_color(rgb(palette.text))
                                        })
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.ai.history.query.clear();
                                            this.reset_text_input("ai.history-search", "", cx);
                                            cx.notify();
                                        }))
                                        .child(
                                            svg()
                                                .size(px(13.))
                                                .path("icons/window/close.svg")
                                                .text_color(rgb(palette.text_muted)),
                                        ),
                                )
                            }),
                    ),
            )
            .child(
                div()
                    .h(px(32.))
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("ai.history")),
                    )
                    .child(
                        div()
                            .id(SharedString::from("ai-history-clear-all"))
                            .h(px(22.))
                            .px_2()
                            .rounded_sm()
                            .flex()
                            .items_center()
                            .text_size(px(11.))
                            .text_color(if history_actions_disabled {
                                rgb(palette.border)
                            } else {
                                rgb(palette.text_muted)
                            })
                            .when(!history_actions_disabled, |this| {
                                this.cursor_pointer().hover(|this| {
                                    this.bg(rgb(palette.surface_elevated))
                                        .text_color(rgb(palette.text))
                                })
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if this.ai.history.sessions.is_empty()
                                    || this.ai.chat.pending
                                    || this.ai.agent.loop_state.is_some()
                                {
                                    return;
                                }
                                this.open_ai_clear_history_confirm(window, cx);
                            }))
                            .child(self.tr("ai.clearHistory")),
                    ),
            )
            .child(
                div()
                    .id(SharedString::from("ai-history-scroll"))
                    .flex_1()
                    .min_h_0()
                    .max_h(px(280.))
                    .overflow_scroll()
                    .scrollbar_width(px(6.))
                    .child(rows),
            )
    }
}
