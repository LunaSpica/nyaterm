use super::*;

impl NyaTermApp {
    pub(in crate::features) fn compact_tab_actions_menu(
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
        _session_count: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let (viewport_w, viewport_h) = self.last_viewport_size;
        let (menu_x, menu_y) = if let Some((x, y)) = self.tab_actions_anchor {
            clamp_tab_actions_position(x, y, 240., 560., viewport_w, viewport_h)
        } else {
            (((viewport_w - 240.).max(16.) * 0.5).max(8.), 74.0)
        };

        let mut color_row = div()
            .px_3()
            .py_1()
            .flex()
            .flex_wrap()
            .gap_1()
            .items_center();
        for (name, color) in TAB_PRESET_COLORS {
            let selected = active_color == Some(color);
            let color_session_id = session_id.clone();
            color_row = color_row.child(
                div()
                    .id(SharedString::from(format!("tab-ctx-color-{name}")))
                    .size(px(16.))
                    .rounded_full()
                    .border_1()
                    .border_color(if selected {
                        rgb(0xffffff)
                    } else {
                        rgb(palette.border)
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
        let reset_color_session_id = session_id.clone();
        color_row = color_row.child(
            div()
                .id(SharedString::from("tab-ctx-color-reset"))
                .h(px(18.))
                .px_2()
                .rounded_sm()
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
                .cursor_pointer()
                .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
                .child(self.tr("tabCtx.resetColor"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.select_session(reset_color_session_id.clone(), cx);
                    this.close_tab_actions(cx);
                    this.set_active_session_tab_color(None, cx);
                })),
        );

        let rename_session_id = session_id.clone();
        let copy_name_session_id = session_id.clone();
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
        let disconnect_session_id = session_id.clone();
        let info_session_id = session_id.clone();
        let inactive_anchor = session_id.clone();
        let right_anchor = session_id.clone();
        let explain_session_id = session_id.clone();
        let analyze_session_id = session_id.clone();

        div()
            .id(SharedString::from("tab-actions-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000000))
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
                    .id(SharedString::from("tab-actions-menu"))
                    .absolute()
                    .left(px(menu_x))
                    .top(px(menu_y))
                    .w(px(240.))
                    .max_h(px(560.))
                    .overflow_y_scroll()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .shadow_lg()
                    .py_1()
                    .flex()
                    .flex_col()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .px_3()
                            .pb_1()
                            .pt_1()
                            .border_b_1()
                            .border_color(rgb(palette.border))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(palette.text))
                                    .child(truncate_preview(display_name, 28)),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text_dimmed))
                                    .child(format!(
                                        "{} · {}",
                                        session_kind_label(session.kind),
                                        short_id(&session.id)
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .px_3()
                            .pt_1()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("tabCtx.setColor")),
                    )
                    .child(color_row)
                    .child(tab_menu_separator(palette))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-rename",
                        self.tr("tabCtx.rename"),
                        cx.listener(move |this, _, window, cx| {
                            this.close_tab_actions(cx);
                            this.open_rename_session(rename_session_id.clone(), window, cx);
                        }),
                    ))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-copy-name",
                        self.tr("tabCtx.copyName"),
                        cx.listener(move |this, _, _, cx| {
                            this.select_session(copy_name_session_id.clone(), cx);
                            this.close_tab_actions(cx);
                            this.copy_active_session_name(cx);
                        }),
                    ))
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-copy-ip",
                        self.tr("tabCtx.copyIp"),
                        can_copy_ssh,
                        cx.listener(move |this, _, _, cx| {
                            this.select_session(copy_host_session_id.clone(), cx);
                            this.close_tab_actions(cx);
                            this.copy_active_session_ssh_host(cx);
                        }),
                    ))
                    .when(can_copy_ssh, |this| {
                        this.child(tab_menu_item(
                            palette,
                            "tab-ctx-copy-ssh",
                            self.tr("tabCtx.copySshAddress"),
                            cx.listener(move |this, _, _, cx| {
                                this.select_session(copy_ssh_session_id.clone(), cx);
                                this.close_tab_actions(cx);
                                this.copy_active_session_ssh_address(cx);
                            }),
                        ))
                    })
                    .child(tab_menu_separator(palette))
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-duplicate",
                        self.tr("tabCtx.duplicate"),
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
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-duplicate-run",
                        self.tr("tabCtx.duplicateWithCommand"),
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
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-multiplex",
                        self.tr("tabCtx.multiplexSsh"),
                        can_multiplex,
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
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-multiplex-run",
                        self.tr("tabCtx.multiplexSshWithCommand"),
                        can_multiplex,
                        cx.listener(move |this, _, window, cx| {
                            this.select_session(multiplex_startup_session_id.clone(), cx);
                            this.close_tab_actions(cx);
                            if this
                                .active_session_busy_actions
                                .contains_key(&multiplex_startup_session_id)
                                || this.is_session_disconnected(&multiplex_startup_session_id)
                            {
                                this.terminal_status =
                                    "SSH multiplex is unavailable for this session".to_string();
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
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-reconnect",
                        self.tr("tabCtx.reconnect"),
                        can_reconnect,
                        cx.listener(move |this, _, window, cx| {
                            this.select_session(reconnect_session_id.clone(), cx);
                            this.close_tab_actions(cx);
                            if this
                                .active_session_busy_actions
                                .contains_key(&reconnect_session_id)
                                || this.has_pending_session_start()
                            {
                                cx.notify();
                                return;
                            }
                            this.reconnect_active_session(window, cx);
                        }),
                    ))
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-disconnect",
                        self.tr("tabCtx.disconnect"),
                        can_disconnect,
                        cx.listener(move |this, _, _, cx| {
                            this.close_tab_actions(cx);
                            if this
                                .active_session_busy_actions
                                .contains_key(&disconnect_session_id)
                                || this.is_session_disconnected(&disconnect_session_id)
                            {
                                cx.notify();
                                return;
                            }
                            this.disconnect_session(disconnect_session_id.clone(), cx);
                        }),
                    ))
                    .child(tab_menu_separator(palette))
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-ai-explain",
                        self.tr("ai.explainRecent"),
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
                                this.ai_status = "terminal visible screen is empty".to_string();
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
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-ai-analyze",
                        self.tr("ai.analyzeError"),
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
                    .child(tab_menu_separator(palette))
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-split-h",
                        self.tr("tabCtx.splitHorizontal"),
                        can_spawn_session,
                        cx.listener(move |this, _, window, cx| {
                            this.select_session(split_horizontal_session_id.clone(), cx);
                            this.close_tab_actions(cx);
                            if !this.tab_action_can_spawn_session(&split_horizontal_session_id) {
                                this.terminal_status =
                                    "active session cannot be duplicated for split".to_string();
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
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-split-v",
                        self.tr("tabCtx.splitVertical"),
                        can_spawn_session,
                        cx.listener(move |this, _, window, cx| {
                            this.select_session(split_vertical_session_id.clone(), cx);
                            this.close_tab_actions(cx);
                            if !this.tab_action_can_spawn_session(&split_vertical_session_id) {
                                this.terminal_status =
                                    "active session cannot be duplicated for split".to_string();
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
                    .when(can_unsplit, |this| {
                        this.child(tab_menu_item(
                            palette,
                            "tab-ctx-unsplit",
                            self.tr("tabCtx.unsplit"),
                            cx.listener(|this, _, _, cx| {
                                this.close_tab_actions(cx);
                                this.unsplit_workspace(cx);
                            }),
                        ))
                    })
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-window-right",
                        self.tr("tabActions.windowRight"),
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
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-window-below",
                        self.tr("tabActions.windowBelow"),
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
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-smart-split",
                        self.tr("tabActions.smartSplit"),
                        cx.listener(|this, _, _, cx| {
                            this.close_tab_actions(cx);
                            this.apply_smart_split(SmartSplitMode::Auto, cx);
                        }),
                    ))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-tile-h",
                        self.tr("tabActions.tileHorizontal"),
                        cx.listener(|this, _, _, cx| {
                            this.close_tab_actions(cx);
                            this.apply_smart_split(SmartSplitMode::Horizontal, cx);
                        }),
                    ))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-tile-v",
                        self.tr("tabActions.tileVertical"),
                        cx.listener(|this, _, _, cx| {
                            this.close_tab_actions(cx);
                            this.apply_smart_split(SmartSplitMode::Vertical, cx);
                        }),
                    ))
                    .when(can_merge_windows, |this| {
                        this.child(tab_menu_item(
                            palette,
                            "tab-ctx-window-flat",
                            self.tr("tabActions.mergeWindows"),
                            cx.listener(|this, _, _, cx| {
                                this.close_tab_actions(cx);
                                this.close_terminal_window_layout(cx);
                            }),
                        ))
                    })
                    .child(tab_menu_separator(palette))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-close",
                        self.tr("tabCtx.close"),
                        cx.listener(move |this, _, _, cx| {
                            this.close_tab_actions(cx);
                            this.close_session(session_id.clone(), cx);
                        }),
                    ))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-close-all",
                        self.tr("tabCtx.closeAll"),
                        cx.listener(|this, _, window, cx| {
                            this.close_tab_actions(cx);
                            this.open_close_all_sessions_confirm(window, cx);
                        }),
                    ))
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-close-others",
                        self.tr("tabCtx.closeInactive"),
                        can_close_inactive,
                        cx.listener(move |this, _, _, cx| {
                            this.close_tab_actions(cx);
                            this.close_inactive_sessions(inactive_anchor.clone(), cx);
                        }),
                    ))
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-close-right",
                        self.tr("tabCtx.closeRight"),
                        can_close_right,
                        cx.listener(move |this, _, _, cx| {
                            this.close_tab_actions(cx);
                            this.close_sessions_to_right(right_anchor.clone(), cx);
                        }),
                    ))
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-info",
                        self.tr("tabCtx.sessionInfo"),
                        can_session_info,
                        cx.listener(move |this, _, window, cx| {
                            this.select_session(info_session_id.clone(), cx);
                            this.close_tab_actions(cx);
                            if !this.tab_action_can_show_session_info(&info_session_id) {
                                this.terminal_status =
                                    "active session has no saved connection info".to_string();
                                cx.notify();
                                return;
                            }
                            this.open_active_session_info(window, cx);
                        }),
                    )),
            )
            .into_any_element()
    }
}
