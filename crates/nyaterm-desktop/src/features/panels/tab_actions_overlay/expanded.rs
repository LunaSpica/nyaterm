use super::*;

impl NyaTermApp {
    pub(in crate::features) fn expanded_tab_actions_dialog(
        &mut self,
        palette: ThemePalette,
        session_id: String,
        session: &SessionInfo,
        display_name: &str,
        active_color: Option<u32>,
        can_copy_ssh: bool,
        can_spawn_session: bool,
        can_multiplex: bool,
        can_reconnect: bool,
        can_disconnect: bool,
        can_use_ai: bool,
        can_session_info: bool,
        can_close_inactive: bool,
        can_close_right: bool,
        can_unsplit: bool,
        can_merge_windows: bool,
        visible_for_ai: String,
        buffer_for_ai: String,
        session_count: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let mut swatches = div().grid().grid_cols(6).gap_2();
        for (name, color) in TAB_PRESET_COLORS {
            let selected = active_color == Some(color);
            let color_session_id = session_id.clone();
            swatches = swatches.child(
                div()
                    .id(SharedString::from(format!("tab-actions-color-{name}")))
                    .size(px(24.))
                    .rounded_full()
                    .border_2()
                    .border_color(if selected {
                        rgb(0xffffff)
                    } else {
                        rgb(0x1f2937)
                    })
                    .bg(rgb(color))
                    .cursor_pointer()
                    .hover(|this| this.border_color(rgb(palette.text)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.select_session(color_session_id.clone(), cx);
                        this.close_tab_actions(cx);
                        this.set_active_session_tab_color(Some(color), cx);
                    })),
            );
        }

        let rename_session_id = session_id.clone();
        let copy_name_session_id = session_id.clone();
        let copy_endpoint_session_id = session_id.clone();
        let copy_host_session_id = session_id.clone();
        let copy_ssh_session_id = session_id.clone();
        let duplicate_session_id = session_id.clone();
        let multiplex_session_id = session_id.clone();
        let startup_session_id = session_id.clone();
        let multiplex_startup_session_id = session_id.clone();
        let split_horizontal_session_id = session_id.clone();
        let split_vertical_session_id = session_id.clone();
        let window_leaf_right_session_id = session_id.clone();
        let window_leaf_below_session_id = session_id.clone();
        let reconnect_session_id = session_id.clone();
        let info_session_id = session_id.clone();
        let close_session_id = session_id.clone();
        let disconnect_session_id = session_id.clone();
        let inactive_anchor = session_id.clone();
        let right_anchor = session_id.clone();
        let reset_color_session_id = session_id.clone();
        let explain_session_id = session_id.clone();
        let analyze_session_id = session_id.clone();

        let (viewport_w, _viewport_h) = self.last_viewport_size;
        let menu_x = ((viewport_w - 600.0) * 0.5).max(24.0);
        let menu_y = 74.0;

        div()
            .id(SharedString::from("tab-actions-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x030508d8))
            .track_focus(&self.tab_actions_focus)
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_tab_actions(cx);
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                if event.keystroke.key == "escape" {
                    this.close_tab_actions(cx);
                }
            }))
            .child(
                div()
                    .id(SharedString::from("tab-actions-dialog"))
                    .absolute()
                    .left(px(menu_x))
                    .top(px(menu_y))
                    .w(px(600.))
                    .max_h(px(640.))
                    .overflow_y_scroll()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface_elevated))
                    .shadow_lg()
                    .p_2()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .px_1()
                            .pb_1()
                            .border_b_1()
                            .border_color(rgb(palette.border))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(palette.text))
                                            .child(self.tr("tabActions.title")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(rgb(palette.text_dimmed))
                                            .child(format!(
                                                "{} · {} · {}",
                                                truncate_preview(display_name, 42),
                                                session_kind_label(session.kind),
                                                short_id(&session.id)
                                            )),
                                    ),
                            )
                            .child(small_button(
                                palette,
                                "tab-actions-close",
                                "Esc",
                                cx.listener(|this, _, _, cx| {
                                    this.close_tab_actions(cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .mt_2()
                            .grid()
                            .grid_cols(2)
                            .gap(px(12.))
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
                                            .text_color(rgb(palette.text))
                                            .child(self.tr("tabCtx.setColor")),
                                    )
                                    .child(div().mt_3().child(swatches))
                                    .child(div().mt_3().child(small_button(
                                        palette,
                                        "tab-actions-color-reset",
                                        self.tr("tabCtx.resetColor"),
                                        cx.listener(move |this, _, _, cx| {
                                            this.select_session(reset_color_session_id.clone(), cx);
                                            this.close_tab_actions(cx);
                                            this.set_active_session_tab_color(None, cx);
                                        }),
                                    ))),
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
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(palette.text))
                                            .child(self.tr("tabActions.identity")),
                                    )
                                    .child(
                                        div()
                                            .mt_3()
                                            .grid()
                                            .grid_cols(2)
                                            .gap_2()
                                            .child(tab_action_button(
                                                palette,
                                                "tab-actions-rename",
                                                self.tr("tabCtx.rename"),
                                                self.tr("tabActions.editTabTitle"),
                                                cx.listener(move |this, _, window, cx| {
                                                    this.close_tab_actions(cx);
                                                    this.open_rename_session(
                                                        rename_session_id.clone(),
                                                        window,
                                                        cx,
                                                    );
                                                }),
                                            ))
                                            .child(tab_action_button(
                                                palette,
                                                "tab-actions-copy-name",
                                                self.tr("tabCtx.copyName"),
                                                self.tr("tabActions.clipboardTitle"),
                                                cx.listener(move |this, _, _, cx| {
                                                    this.select_session(
                                                        copy_name_session_id.clone(),
                                                        cx,
                                                    );
                                                    this.close_tab_actions(cx);
                                                    this.copy_active_session_name(cx);
                                                }),
                                            ))
                                            .child(tab_action_button(
                                                palette,
                                                "tab-actions-copy-endpoint",
                                                self.tr("tabActions.copyEndpoint"),
                                                self.tr("tabActions.hostOrShell"),
                                                cx.listener(move |this, _, _, cx| {
                                                    this.select_session(
                                                        copy_endpoint_session_id.clone(),
                                                        cx,
                                                    );
                                                    this.close_tab_actions(cx);
                                                    this.copy_active_session_endpoint(cx);
                                                }),
                                            ))
                                            .when(can_copy_ssh, |this| {
                                                this.child(tab_action_button(
                                                    palette,
                                                    "tab-actions-copy-ip",
                                                    self.tr("tabCtx.copyIp"),
                                                    self.tr("tabActions.sshHost"),
                                                    cx.listener(move |this, _, _, cx| {
                                                        this.select_session(
                                                            copy_host_session_id.clone(),
                                                            cx,
                                                        );
                                                        this.close_tab_actions(cx);
                                                        this.copy_active_session_ssh_host(cx);
                                                    }),
                                                ))
                                                .child(tab_action_button(
                                                    palette,
                                                    "tab-actions-copy-ssh",
                                                    self.tr("tabActions.copySsh"),
                                                    self.tr("tabActions.userAtHost"),
                                                    cx.listener(move |this, _, _, cx| {
                                                        this.select_session(
                                                            copy_ssh_session_id.clone(),
                                                            cx,
                                                        );
                                                        this.close_tab_actions(cx);
                                                        this.copy_active_session_ssh_address(cx);
                                                    }),
                                                ))
                                            }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(3)
                            .gap_2()
                            .child(tab_action_button_enabled(
                                palette,
                                "tab-actions-duplicate",
                                self.tr("tabCtx.duplicate"),
                                self.tr("tabActions.newSameSession"),
                                can_spawn_session,
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(duplicate_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    if !this.tab_action_can_spawn_session(&duplicate_session_id) {
                                        this.terminal_status =
                                            "active session cannot be duplicated".to_string();
                                        cx.notify();
                                        return;
                                    }
                                    this.duplicate_active_session(window, cx);
                                }),
                            ))
                            .when(can_multiplex, |this| {
                                this.child(tab_action_button(
                                    palette,
                                    "tab-actions-multiplex",
                                    self.tr("tabActions.multiplex"),
                                self.tr("tabActions.reuseSshChannel"),
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(multiplex_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    if this
                                        .active_session_busy_actions
                                        .contains_key(&multiplex_session_id)
                                        || this.is_session_disconnected(&multiplex_session_id)
                                    {
                                        this.terminal_status =
                                            "SSH multiplex is unavailable for this session".to_string();
                                        cx.notify();
                                        return;
                                    }
                                    this.multiplex_active_ssh_session(window, cx);
                                }),
                                ))
                            })
                            .child(tab_action_button_enabled(
                                palette,
                                "tab-actions-duplicate-command",
                                self.tr("tabActions.duplicateAndRun"),
                                self.tr("tabActions.startupCommand"),
                                can_spawn_session,
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(startup_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    if !this.tab_action_can_spawn_session(&startup_session_id) {
                                        this.terminal_status =
                                            "active session cannot be duplicated".to_string();
                                        cx.notify();
                                        return;
                                    }
                                    this.open_startup_command_dialog(window, cx);
                                }),
                            ))
                            .when(can_multiplex, |this| {
                                this.child(tab_action_button(
                                    palette,
                                    "tab-actions-multiplex-command",
                                    self.tr("tabActions.multiplexAndRun"),
                                    self.tr("tabActions.startupCommand"),
                                    cx.listener(move |this, _, window, cx| {
                                        this.select_session(
                                            multiplex_startup_session_id.clone(),
                                            cx,
                                        );
                                        this.close_tab_actions(cx);
                                        if this
                                            .active_session_busy_actions
                                            .contains_key(&multiplex_startup_session_id)
                                            || this
                                                .is_session_disconnected(&multiplex_startup_session_id)
                                        {
                                            this.terminal_status =
                                                "SSH multiplex is unavailable for this session"
                                                    .to_string();
                                            cx.notify();
                                            return;
                                        }
                                        this.open_startup_command_dialog_for(
                                            StartupCommandAction::Multiplex,
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                            })
                            .child(tab_action_button_enabled(
                                palette,
                                "tab-actions-split-horizontal",
                                self.tr("tabActions.splitHorizontalShort"),
                                self.tr("tabActions.duplicatePane"),
                                can_spawn_session,
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(split_horizontal_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    if !this
                                        .tab_action_can_spawn_session(&split_horizontal_session_id)
                                    {
                                        this.terminal_status =
                                            "active session cannot be duplicated for split"
                                                .to_string();
                                        cx.notify();
                                        return;
                                    }
                                    this.split_workspace_with_duplicate(
                                        WorkspaceSplitDirection::Horizontal,
                                        window,
                                        cx,
                                    );
                                }),
                            ))
                            .child(tab_action_button_enabled(
                                palette,
                                "tab-actions-split-vertical",
                                self.tr("tabActions.splitVerticalShort"),
                                self.tr("tabActions.duplicatePane"),
                                can_spawn_session,
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(split_vertical_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    if !this
                                        .tab_action_can_spawn_session(&split_vertical_session_id)
                                    {
                                        this.terminal_status =
                                            "active session cannot be duplicated for split"
                                                .to_string();
                                        cx.notify();
                                        return;
                                    }
                                    this.split_workspace_with_duplicate(
                                        WorkspaceSplitDirection::Vertical,
                                        window,
                                        cx,
                                    );
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "tab-actions-window-right",
                                self.tr("tabActions.windowRight"),
                                self.tr("tabActions.detachToLeaf"),
                                cx.listener(move |this, _, _, cx| {
                                    this.select_session(window_leaf_right_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    this.split_active_tab_to_new_window_leaf(
                                        WorkspaceSplitDirection::Vertical,
                                        SplitEdge::After,
                                        cx,
                                    );
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "tab-actions-window-below",
                                self.tr("tabActions.windowBelow"),
                                self.tr("tabActions.detachToLeaf"),
                                cx.listener(move |this, _, _, cx| {
                                    this.select_session(window_leaf_below_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    this.split_active_tab_to_new_window_leaf(
                                        WorkspaceSplitDirection::Horizontal,
                                        SplitEdge::After,
                                        cx,
                                    );
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "tab-actions-smart-split",
                                self.tr("tabActions.smartSplit"),
                                self.tr("tabActions.tileAllTabs"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_tab_actions(cx);
                                    this.apply_smart_split(SmartSplitMode::Auto, cx);
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "tab-actions-tile-h",
                                self.tr("tabActions.tileHorizontalShort"),
                                self.tr("tabActions.sideBySide"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_tab_actions(cx);
                                    this.apply_smart_split(SmartSplitMode::Horizontal, cx);
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "tab-actions-tile-v",
                                self.tr("tabActions.tileVerticalShort"),
                                self.tr("tabActions.stackedLeaves"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_tab_actions(cx);
                                    this.apply_smart_split(SmartSplitMode::Vertical, cx);
                                }),
                            ))
                            .when(can_merge_windows, |this| {
                                this.child(tab_action_button(
                                    palette,
                                    "tab-actions-window-merge",
                                    self.tr("tabActions.mergeWindows"),
                                    self.tr("tabActions.flatTabStrip"),
                                    cx.listener(|this, _, _, cx| {
                                        this.close_tab_actions(cx);
                                        this.close_terminal_window_layout(cx);
                                    }),
                                ))
                            })
                            .child(tab_action_button_enabled(
                                palette,
                                "tab-actions-reconnect",
                                self.tr("tabCtx.reconnect"),
                                self.tr("tabActions.restartSession"),
                                can_reconnect,
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(reconnect_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    if this.has_pending_session_start()
                                        || this
                                            .active_session_busy_actions
                                            .contains_key(&reconnect_session_id)
                                    {
                                        this.terminal_status =
                                            "session is busy; reconnect unavailable".to_string();
                                        cx.notify();
                                        return;
                                    }
                                    this.reconnect_active_session(window, cx);
                                }),
                            ))
                            .child(tab_action_button_enabled(
                                palette,
                                "tab-actions-disconnect",
                                self.tr("tabCtx.disconnect"),
                                self.tr("tabActions.keepTabDropBackend"),
                                can_disconnect,
                                cx.listener(move |this, _, _, cx| {
                                    this.close_tab_actions(cx);
                                    if this
                                        .active_session_busy_actions
                                        .contains_key(&disconnect_session_id)
                                        || this.is_session_disconnected(&disconnect_session_id)
                                    {
                                        this.terminal_status =
                                            "session is busy or already disconnected".to_string();
                                        cx.notify();
                                        return;
                                    }
                                    this.disconnect_session(disconnect_session_id.clone(), cx);
                                }),
                            ))
                            .child(tab_action_button_enabled(
                                palette,
                                "tab-actions-info",
                                self.tr("tabCtx.sessionInfo"),
                                self.tr("tabActions.connectionDetail"),
                                can_session_info,
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(info_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    if !this.tab_action_can_show_session_info(&info_session_id) {
                                        this.terminal_status =
                                            "active session has no saved connection info"
                                                .to_string();
                                        cx.notify();
                                        return;
                                    }
                                    this.open_active_session_info(window, cx);
                                }),
                            ))
                            .child(tab_action_button_enabled(
                                palette,
                                "tab-actions-ai-explain",
                                self.tr("ai.explainRecent"),
                                self.tr("tabActions.visibleOutput"),
                                can_use_ai,
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(explain_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    if this
                                        .active_session_busy_actions
                                        .contains_key(&explain_session_id)
                                        || this.is_session_disconnected(&explain_session_id)
                                    {
                                        this.ai_status =
                                            "terminal session unavailable for AI".to_string();
                                        cx.notify();
                                        return;
                                    }
                                    if visible_for_ai.trim().is_empty() {
                                        this.ai_status =
                                            "terminal visible screen is empty".to_string();
                                    } else {
                                        this.ai_prompt_draft = format!(
                                            "Explain this terminal output:\n\n{}",
                                            visible_for_ai
                                        );
                                        this.ai_status =
                                            "terminal output loaded into AI prompt".to_string();
                                        window.focus(&this.ai_chat_focus);
                                    }
                                    cx.notify();
                                }),
                            ))
                            .child(tab_action_button_enabled(
                                palette,
                                "tab-actions-ai-analyze",
                                self.tr("ai.analyzeError"),
                                self.tr("tabActions.bufferErrors"),
                                can_use_ai,
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(analyze_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    if this
                                        .active_session_busy_actions
                                        .contains_key(&analyze_session_id)
                                        || this.is_session_disconnected(&analyze_session_id)
                                    {
                                        this.ai_status =
                                            "terminal session unavailable for AI".to_string();
                                        cx.notify();
                                        return;
                                    }
                                    if buffer_for_ai.trim().is_empty() {
                                        this.ai_status = "terminal buffer is empty".to_string();
                                    } else {
                                        this.ai_prompt_draft = format!(
                                            "Analyze this terminal buffer for errors, risks, and next actions:\n\n{}",
                                            buffer_for_ai
                                        );
                                        this.ai_status =
                                            "terminal buffer loaded into AI prompt".to_string();
                                        window.focus(&this.ai_chat_focus);
                                    }
                                    cx.notify();
                                }),
                            ))
                            .when(can_unsplit, |this| {
                                this.child(tab_action_button(
                                    palette,
                                    "tab-actions-unsplit",
                                    self.tr("tabCtx.unsplit"),
                                    self.tr("tabActions.closeSplitView"),
                                    cx.listener(|this, _, _, cx| {
                                        this.close_tab_actions(cx);
                                        this.unsplit_workspace(cx);
                                    }),
                                ))
                            })
                            .child(tab_action_button(
                                palette,
                                "tab-actions-close-session",
                                self.tr("tabCtx.close"),
                                self.tr("tabActions.endThisSession"),
                                cx.listener(move |this, _, _, cx| {
                                    this.close_tab_actions(cx);
                                    this.close_session(close_session_id.clone(), cx);
                                }),
                            ))
                            .when(can_close_inactive, |this| {
                                this.child(tab_action_button(
                                    palette,
                                    "tab-actions-close-inactive",
                                    self.tr("tabCtx.closeInactive"),
                                    self.tr("tabActions.keepThisTab"),
                                    cx.listener(move |this, _, _, cx| {
                                        this.close_tab_actions(cx);
                                        this.close_inactive_sessions(inactive_anchor.clone(), cx);
                                    }),
                                ))
                            })
                            .when(can_close_right, |this| {
                                this.child(tab_action_button(
                                    palette,
                                    "tab-actions-close-right",
                                    self.tr("tabCtx.closeRight"),
                                    self.tr("tabActions.tabsAfterThis"),
                                    cx.listener(move |this, _, _, cx| {
                                        this.close_tab_actions(cx);
                                        this.close_sessions_to_right(right_anchor.clone(), cx);
                                    }),
                                ))
                            })
                            .when(session_count > 0, |this| {
                                this.child(tab_action_button(
                                    palette,
                                    "tab-actions-close-all",
                                    self.tr("tabCtx.closeAll"),
                                    self.tr("tabActions.endAllSessions"),
                                    cx.listener(|this, _, window, cx| {
                                        this.close_tab_actions(cx);
                                        this.open_close_all_sessions_confirm(window, cx);
                                    }),
                                ))
                            }),
                    ),
            )
            .into_any_element()
    }
}
