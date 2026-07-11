use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn tab_actions_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let palette = self.theme_palette();
        let Some(session_id) = self.tab_actions_session_id.clone() else {
            return div().into_any_element();
        };
        let sessions = self.ordered_sessions();
        let Some(session) = sessions
            .iter()
            .find(|session| session.id == session_id)
            .cloned()
        else {
            self.tab_actions_session_id = None;
            self.tab_actions_anchor = None;
            return div().into_any_element();
        };

        let display_name = self.session_display_name_by_info(&session);
        let active_color = self.session_tab_colors.get(&session_id).copied();
        let can_copy_ssh = self.session_ssh_address(&session_id).is_some();
        let can_multiplex = session.kind == SessionKind::Ssh;
        let can_close_inactive = sessions.len() > 1;
        let can_close_right = sessions
            .iter()
            .position(|session| session.id == session_id)
            .is_some_and(|index| index + 1 < sessions.len());
        let can_unsplit = self.workspace_split.is_some();
        let visible_for_ai = terminal_action_prompt_text(
            &self
                .terminal_views
                .get(&session_id)
                .map(|view| view.screen.lines().join("\n"))
                .unwrap_or_default(),
            2_800,
        );
        let buffer_for_ai = terminal_action_prompt_text(
            &self
                .terminal_views
                .get(&session_id)
                .map(|view| view.output.clone())
                .unwrap_or_default(),
            4_000,
        );

        let compact = self.tab_actions_anchor.is_some();
        if compact {
            return self.compact_tab_actions_menu(
                palette,
                session_id,
                &session,
                &display_name,
                active_color,
                can_copy_ssh,
                can_multiplex,
                can_close_inactive,
                can_close_right,
                can_unsplit,
                visible_for_ai,
                buffer_for_ai,
                sessions.len(),
                cx,
            );
        }

        self.expanded_tab_actions_dialog(
            palette,
            session_id,
            &session,
            &display_name,
            active_color,
            can_copy_ssh,
            can_multiplex,
            can_close_inactive,
            can_close_right,
            can_unsplit,
            visible_for_ai,
            buffer_for_ai,
            sessions.len(),
            cx,
        )
    }

    fn compact_tab_actions_menu(
        &mut self,
        palette: ThemePalette,
        session_id: String,
        session: &SessionInfo,
        display_name: &str,
        active_color: Option<u32>,
        can_copy_ssh: bool,
        can_multiplex: bool,
        can_close_inactive: bool,
        can_close_right: bool,
        can_unsplit: bool,
        visible_for_ai: String,
        buffer_for_ai: String,
        _session_count: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let (viewport_w, viewport_h) = self.last_viewport_size;
        let (menu_x, menu_y) = if let Some((x, y)) = self.tab_actions_anchor {
            clamp_tab_actions_position(x, y, 240., 560., viewport_w, viewport_h)
        } else {
            (24.0, 74.0)
        };

        let mut color_row = div().px_3().py_1().flex().flex_wrap().gap_1().items_center();
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
                .child("Reset")
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
        let reconnect_session_id = session_id.clone();
        let info_session_id = session_id.clone();
        let close_session_id = session_id.clone();
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
                            .child("Color"),
                    )
                    .child(color_row)
                    .child(tab_menu_separator(palette))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-rename",
                        "Rename",
                        cx.listener(move |this, _, window, cx| {
                            this.close_tab_actions(cx);
                            this.open_rename_session(rename_session_id.clone(), window, cx);
                        }),
                    ))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-copy-name",
                        "Copy Name",
                        cx.listener(move |this, _, _, cx| {
                            this.select_session(copy_name_session_id.clone(), cx);
                            this.close_tab_actions(cx);
                            this.copy_active_session_name(cx);
                        }),
                    ))
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-copy-ip",
                        "Copy IP",
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
                            "Copy SSH Address",
                            cx.listener(move |this, _, _, cx| {
                                this.select_session(copy_ssh_session_id.clone(), cx);
                                this.close_tab_actions(cx);
                                this.copy_active_session_ssh_address(cx);
                            }),
                        ))
                    })
                    .child(tab_menu_separator(palette))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-duplicate",
                        "Duplicate",
                        cx.listener(move |this, _, window, cx| {
                            this.select_session(duplicate_session_id.clone(), cx);
                            this.close_tab_actions(cx);
                            this.duplicate_active_session(window, cx);
                        }),
                    ))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-duplicate-run",
                        "Duplicate with Command",
                        cx.listener(move |this, _, window, cx| {
                            this.select_session(startup_session_id.clone(), cx);
                            this.close_tab_actions(cx);
                            this.open_startup_command_dialog(window, cx);
                        }),
                    ))
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-multiplex",
                        "Multiplex SSH",
                        can_multiplex,
                        cx.listener(move |this, _, window, cx| {
                            this.select_session(multiplex_session_id.clone(), cx);
                            this.close_tab_actions(cx);
                            this.multiplex_active_ssh_session(window, cx);
                        }),
                    ))
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-multiplex-run",
                        "Multiplex with Command",
                        can_multiplex,
                        cx.listener(move |this, _, window, cx| {
                            this.select_session(multiplex_startup_session_id.clone(), cx);
                            this.close_tab_actions(cx);
                            this.open_startup_command_dialog_for(
                                StartupCommandAction::Multiplex,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-reconnect",
                        "Reconnect",
                        cx.listener(move |this, _, window, cx| {
                            this.select_session(reconnect_session_id.clone(), cx);
                            this.close_tab_actions(cx);
                            this.reconnect_active_session(window, cx);
                        }),
                    ))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-disconnect",
                        "Disconnect",
                        cx.listener(move |this, _, _, cx| {
                            this.close_tab_actions(cx);
                            this.disconnect_session(close_session_id.clone(), cx);
                        }),
                    ))
                    .child(tab_menu_separator(palette))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-ai-explain",
                        "AI · Explain Recent",
                        cx.listener(move |this, _, window, cx| {
                            this.select_session(explain_session_id.clone(), cx);
                            this.close_tab_actions(cx);
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
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-ai-analyze",
                        "AI · Analyze Errors",
                        cx.listener(move |this, _, window, cx| {
                            this.select_session(analyze_session_id.clone(), cx);
                            this.close_tab_actions(cx);
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
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-split-h",
                        "Split Horizontal",
                        cx.listener(move |this, _, window, cx| {
                            this.select_session(split_horizontal_session_id.clone(), cx);
                            this.close_tab_actions(cx);
                            this.split_workspace_with_duplicate(
                                WorkspaceSplitDirection::Horizontal,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-split-v",
                        "Split Vertical",
                        cx.listener(move |this, _, window, cx| {
                            this.select_session(split_vertical_session_id.clone(), cx);
                            this.close_tab_actions(cx);
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
                            "Unsplit",
                            cx.listener(|this, _, _, cx| {
                                this.close_tab_actions(cx);
                                this.unsplit_workspace(cx);
                            }),
                        ))
                    })
                    .child(tab_menu_separator(palette))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-close",
                        "Close",
                        cx.listener(move |this, _, _, cx| {
                            this.close_tab_actions(cx);
                            this.close_session(session_id.clone(), cx);
                        }),
                    ))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-close-all",
                        "Close All",
                        cx.listener(|this, _, window, cx| {
                            this.close_tab_actions(cx);
                            this.open_close_all_sessions_confirm(window, cx);
                        }),
                    ))
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-close-others",
                        "Close Others",
                        can_close_inactive,
                        cx.listener(move |this, _, _, cx| {
                            this.close_tab_actions(cx);
                            this.close_inactive_sessions(inactive_anchor.clone(), cx);
                        }),
                    ))
                    .child(tab_menu_item_enabled(
                        palette,
                        "tab-ctx-close-right",
                        "Close Right",
                        can_close_right,
                        cx.listener(move |this, _, _, cx| {
                            this.close_tab_actions(cx);
                            this.close_sessions_to_right(right_anchor.clone(), cx);
                        }),
                    ))
                    .child(tab_menu_item(
                        palette,
                        "tab-ctx-info",
                        "Session Info",
                        cx.listener(move |this, _, window, cx| {
                            this.select_session(info_session_id.clone(), cx);
                            this.close_tab_actions(cx);
                            this.open_active_session_info(window, cx);
                        }),
                    )),
            )
            .into_any_element()
    }

    fn expanded_tab_actions_dialog(
        &mut self,
        palette: ThemePalette,
        session_id: String,
        session: &SessionInfo,
        display_name: &str,
        active_color: Option<u32>,
        can_copy_ssh: bool,
        can_multiplex: bool,
        can_close_inactive: bool,
        can_close_right: bool,
        can_unsplit: bool,
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
        let reconnect_session_id = session_id.clone();
        let info_session_id = session_id.clone();
        let close_session_id = session_id.clone();
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
                                            .child("Tab Actions"),
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
                                            .child("Color"),
                                    )
                                    .child(div().mt_3().child(swatches))
                                    .child(div().mt_3().child(small_button(
                                        palette,
                                        "tab-actions-color-reset",
                                        "Reset Color",
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
                                            .child("Identity"),
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
                                                "Rename",
                                                "Edit tab title",
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
                                                "Copy Name",
                                                "Clipboard title",
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
                                                "Copy Endpoint",
                                                "Host or shell",
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
                                                    "Copy IP",
                                                    "SSH host",
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
                                                    "Copy SSH",
                                                    "user@host",
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
                            .child(tab_action_button(
                                palette,
                                "tab-actions-duplicate",
                                "Duplicate",
                                "New same session",
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(duplicate_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    this.duplicate_active_session(window, cx);
                                }),
                            ))
                            .when(can_multiplex, |this| {
                                this.child(tab_action_button(
                                    palette,
                                    "tab-actions-multiplex",
                                    "Multiplex",
                                    "Reuse SSH channel",
                                    cx.listener(move |this, _, window, cx| {
                                        this.select_session(multiplex_session_id.clone(), cx);
                                        this.close_tab_actions(cx);
                                        this.multiplex_active_ssh_session(window, cx);
                                    }),
                                ))
                            })
                            .child(tab_action_button(
                                palette,
                                "tab-actions-duplicate-command",
                                "Duplicate + Run",
                                "Startup command",
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(startup_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    this.open_startup_command_dialog(window, cx);
                                }),
                            ))
                            .when(can_multiplex, |this| {
                                this.child(tab_action_button(
                                    palette,
                                    "tab-actions-multiplex-command",
                                    "Multiplex + Run",
                                    "Startup command",
                                    cx.listener(move |this, _, window, cx| {
                                        this.select_session(
                                            multiplex_startup_session_id.clone(),
                                            cx,
                                        );
                                        this.close_tab_actions(cx);
                                        this.open_startup_command_dialog_for(
                                            StartupCommandAction::Multiplex,
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                            })
                            .child(tab_action_button(
                                palette,
                                "tab-actions-split-horizontal",
                                "Split H",
                                "Duplicate pane",
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(split_horizontal_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    this.split_workspace_with_duplicate(
                                        WorkspaceSplitDirection::Horizontal,
                                        window,
                                        cx,
                                    );
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "tab-actions-split-vertical",
                                "Split V",
                                "Duplicate pane",
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(split_vertical_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    this.split_workspace_with_duplicate(
                                        WorkspaceSplitDirection::Vertical,
                                        window,
                                        cx,
                                    );
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "tab-actions-reconnect",
                                "Reconnect",
                                "Restart session",
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(reconnect_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    this.reconnect_active_session(window, cx);
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "tab-actions-info",
                                "Info",
                                "Connection detail",
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(info_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
                                    this.open_active_session_info(window, cx);
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "tab-actions-ai-explain",
                                "AI Explain",
                                "visible output",
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(explain_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
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
                            .child(tab_action_button(
                                palette,
                                "tab-actions-ai-analyze",
                                "AI Analyze",
                                "buffer errors",
                                cx.listener(move |this, _, window, cx| {
                                    this.select_session(analyze_session_id.clone(), cx);
                                    this.close_tab_actions(cx);
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
                                    "Unsplit",
                                    "Close split view",
                                    cx.listener(|this, _, _, cx| {
                                        this.close_tab_actions(cx);
                                        this.unsplit_workspace(cx);
                                    }),
                                ))
                            })
                            .child(tab_action_button(
                                palette,
                                "tab-actions-close-session",
                                "Close Tab",
                                "End this session",
                                cx.listener(move |this, _, _, cx| {
                                    this.close_tab_actions(cx);
                                    this.close_session(close_session_id.clone(), cx);
                                }),
                            ))
                            .when(can_close_inactive, |this| {
                                this.child(tab_action_button(
                                    palette,
                                    "tab-actions-close-inactive",
                                    "Close Others",
                                    "Keep this tab",
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
                                    "Close Right",
                                    "Tabs after this",
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
                                    "Close All",
                                    "End all sessions",
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

fn clamp_tab_actions_position(
    x: f32,
    y: f32,
    menu_w: f32,
    menu_h: f32,
    viewport_w: f32,
    viewport_h: f32,
) -> (f32, f32) {
    let max_x = (viewport_w - menu_w - 8.0).max(8.0);
    let max_y = (viewport_h - menu_h - 8.0).max(8.0);
    (x.clamp(8.0, max_x), y.clamp(8.0, max_y))
}
