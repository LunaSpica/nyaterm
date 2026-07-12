use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn quick_commands_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let filtered_commands = filtered_quick_commands(
            &self.quick_commands,
            &self.quick_command_categories,
            &self.quick_command_search_draft,
            &self.quick_command_selected_category,
            self.quick_command_sort_mode,
        );
        let total_commands = self.quick_commands.len();
        let visible_commands = filtered_commands.len();
        let _pinned_commands = self
            .quick_commands
            .iter()
            .filter(|command| command.pinned.unwrap_or_default())
            .count();
        let categories =
            quick_command_category_options(&self.quick_commands, &self.quick_command_categories);
        let can_send_to_all = self
            .session_manager
            .list_sessions()
            .unwrap_or_default()
            .len()
            > 1;
        let palette = self.theme_palette();

        let mut category_sidebar = div()
            .w(px(140.))
            .flex_shrink_0()
            .pr_2()
            .border_r_1()
            .border_color(rgb(palette.border))
            .flex()
            .flex_col()
            .gap_1();
        for option in categories {
            let id = option.id.clone();
            let rename_id = option.id.clone();
            let delete_id = option.id.clone();
            let selected = self.quick_command_selected_category == option.id;
            let manageable = option.manageable;
            category_sidebar = category_sidebar
                .child(
                    div()
                        .id(SharedString::from(format!(
                            "quick-command-category-{}",
                            option.id
                        )))
                        .h(px(32.))
                        .px_2()
                        .flex()
                        .items_center()
                        .gap_2()
                        .rounded_sm()
                        .bg(if selected {
                            rgb(palette.section_header)
                        } else {
                            rgba(0x00000000)
                        })
                        .text_xs()
                        .text_color(if selected {
                            rgb(palette.accent)
                        } else {
                            rgb(palette.text)
                        })
                        .cursor_pointer()
                        .hover(move |this| this.bg(rgb(palette.hover)))
                        .child(div().size(px(6.)).rounded_full().bg(if selected {
                            rgb(palette.accent)
                        } else {
                            rgb(palette.text_dimmed)
                        }))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .overflow_hidden()
                                .font_weight(FontWeight(700.))
                                .child(option.label),
                        )
                        .child(
                            div()
                                .rounded_sm()
                                .px_2()
                                .py(px(1.))
                                .bg(if selected {
                                    rgb(0x203456)
                                } else {
                                    rgb(palette.border)
                                })
                                .text_size(px(10.))
                                .text_color(if selected {
                                    rgb(0xbfdbfe)
                                } else {
                                    rgb(palette.text_muted)
                                })
                                .child(option.count.to_string()),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.quick_command_selected_category = id.clone();
                            this.quick_command_menu_id = None;
                            cx.notify();
                        })),
                )
                .when(selected && manageable, |this| {
                    this.child(
                        div()
                            .pl_4()
                            .pb_1()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(palette, 
                                format!("quick-command-category-rename-{}", rename_id),
                                "Rename",
                                cx.listener(move |this, _, window, cx| {
                                    this.open_rename_quick_command_category(
                                        rename_id.clone(),
                                        window,
                                        cx,
                                    );
                                }),
                            ))
                            .child(small_button(palette, 
                                format!("quick-command-category-delete-{}", delete_id),
                                "Delete",
                                cx.listener(move |this, _, _, cx| {
                                    this.open_delete_quick_command_category_confirm(
                                        delete_id.clone(),
                                        cx,
                                    );
                                }),
                            )),
                    )
                });
        }

        let mut rows = div()
            .flex()
            .gap_1()
            .when(
                self.quick_command_view_mode == QuickCommandViewMode::Tile,
                |this| this.items_start().flex_wrap(),
            )
            .when(
                self.quick_command_view_mode != QuickCommandViewMode::Tile,
                |this| this.flex_col(),
            );
        if filtered_commands.is_empty() {
            rows = rows.child(
                div()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_2()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap_2()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .line_height(px(18.))
                    .child(if total_commands == 0 {
                        "No quick commands saved yet."
                    } else {
                        "No quick commands match the current filters."
                    })
                    .when(total_commands == 0, |this| {
                        this.child(small_button(palette, 
                            "quick-command-empty-add",
                            "Add Command",
                            cx.listener(|this, _, window, cx| {
                                this.open_new_quick_command_editor(window, cx);
                            }),
                        ))
                    }),
            );
        } else {
            for command in filtered_commands {
                let command_id = command.id.clone();
                let run_command_id = command.id.clone();
                let compact_click_command_id = command.id.clone();
                let list_header_command_id = command.id.clone();
                let edit_command_id = command.id.clone();
                let delete_command_id = command.id.clone();
                let detail_command_id = command.id.clone();
                let all_command_id = command.id.clone();
                let execution_mode = if command.execution_mode.as_deref() == Some("append") {
                    "append"
                } else {
                    "execute"
                };
                let menu_open = self.quick_command_menu_id.as_deref() == Some(command.id.as_str());
                let menu_command_id = command.id.clone();
                let command_item = match self.quick_command_view_mode {
                    QuickCommandViewMode::Tile => div()
                        .id(SharedString::from(format!(
                            "quick-command-tile-{command_id}"
                        )))
                        .relative()
                        .min_w(px(132.))
                        .max_w(px(220.))
                        .h(px(34.))
                        .rounded_sm()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .bg(rgb(palette.input))
                        .px_2()
                        .flex()
                        .items_center()
                        .gap_2()
                        .hover(|this| this.bg(rgb(0x151d2a)))
                        .child(
                            div()
                                .id(SharedString::from(format!(
                                    "quick-command-tile-run-{command_id}"
                                )))
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap_2()
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.quick_command_menu_id = None;
                                    this.run_quick_command_by_id(run_command_id.clone(), cx);
                                }))
                                .child(quick_command_icon_mark(
                                    palette,
                                    command.icon_tag.as_deref(),
                                    command.color_tag.as_deref(),
                                ))
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            div()
                                                .min_w_0()
                                                .text_xs()
                                                .font_weight(FontWeight(800.))
                                                .text_color(rgb(palette.text))
                                                .overflow_hidden()
                                                .child(truncate_preview(&command.label, 28)),
                                        )
                                        .when(command.pinned.unwrap_or_default(), |this| {
                                            this.child(
                                                div()
                                                    .text_size(px(9.))
                                                    .text_color(rgb(palette.warning))
                                                    .child("PIN"),
                                            )
                                        }),
                                ),
                        )
                        .child(status_pill(
                            if execution_mode == "append" {
                                "append"
                            } else {
                                "exec"
                            },
                            if execution_mode == "append" {
                                rgb(palette.warning)
                            } else {
                                rgb(palette.success)
                            },
                            if execution_mode == "append" {
                                rgb(0x32280f)
                            } else {
                                rgb(palette.hover)
                            },
                        ))
                        .child(icon_button(
                            format!("quick-command-tile-detail-{command_id}"),
                            "ⓘ",
                            self.theme_palette(),
                            cx.listener(move |this, _, window, cx| {
                                this.quick_command_menu_id = None;
                                this.open_quick_command_details(
                                    detail_command_id.clone(),
                                    window,
                                    cx,
                                );
                            }),
                        ))
                        .child(quick_command_more_menu(
                            palette,
                            &command_id,
                            menu_open,
                            can_send_to_all,
                            cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                if this.quick_command_menu_id.as_deref()
                                    == Some(menu_command_id.as_str())
                                {
                                    this.quick_command_menu_id = None;
                                } else {
                                    this.quick_command_menu_id = Some(menu_command_id.clone());
                                }
                                cx.notify();
                            }),
                            cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.quick_command_menu_id = None;
                                this.open_edit_quick_command_editor(
                                    edit_command_id.clone(),
                                    window,
                                    cx,
                                );
                            }),
                            cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.quick_command_menu_id = None;
                                this.send_quick_command_to_all_by_id(all_command_id.clone(), cx);
                            }),
                            cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.quick_command_menu_id = None;
                                this.open_delete_quick_command_confirm(
                                    delete_command_id.clone(),
                                    cx,
                                );
                            }),
                        ))
                        .into_any_element(),
                    QuickCommandViewMode::Compact => {
                        // Tauri compact: send + details + more (edit / send-all / delete).
                        let actions = quick_command_row_actions(
                            palette,
                            &command_id,
                            false,
                            execution_mode,
                            menu_open,
                            can_send_to_all,
                            cx.listener(move |this, _, _, cx| {
                                this.quick_command_menu_id = None;
                                this.run_quick_command_by_id(run_command_id.clone(), cx);
                            }),
                            cx.listener(move |this, _, window, cx| {
                                this.quick_command_menu_id = None;
                                this.open_quick_command_details(
                                    detail_command_id.clone(),
                                    window,
                                    cx,
                                );
                            }),
                            cx.listener({
                                let menu_command_id = command_id.clone();
                                move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    if this.quick_command_menu_id.as_deref()
                                        == Some(menu_command_id.as_str())
                                    {
                                        this.quick_command_menu_id = None;
                                    } else {
                                        this.quick_command_menu_id = Some(menu_command_id.clone());
                                    }
                                    cx.notify();
                                }
                            }),
                            cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.quick_command_menu_id = None;
                                this.open_edit_quick_command_editor(
                                    edit_command_id.clone(),
                                    window,
                                    cx,
                                );
                            }),
                            cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.quick_command_menu_id = None;
                                this.send_quick_command_to_all_by_id(all_command_id.clone(), cx);
                            }),
                            cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.quick_command_menu_id = None;
                                this.open_delete_quick_command_confirm(
                                    delete_command_id.clone(),
                                    cx,
                                );
                            }),
                        );

                        div()
                            .id(SharedString::from(format!(
                                "quick-command-compact-{command_id}"
                            )))
                            .relative()
                            .h(px(32.))
                            .w_full()
                            .rounded_sm()
                            .px_1()
                            .flex()
                            .items_center()
                            .gap_1()
                            .hover(|this| this.bg(rgb(0x151d2a)))
                            .child(
                                div()
                                    .id(SharedString::from(format!(
                                        "quick-command-compact-run-area-{command_id}"
                                    )))
                                    .min_w_0()
                                    .flex_1()
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.quick_command_menu_id = None;
                                        this.run_quick_command_by_id(
                                            compact_click_command_id.clone(),
                                            cx,
                                        );
                                    }))
                                    .child(quick_command_icon_mark(
                                        palette,
                                        command.icon_tag.as_deref(),
                                        command.color_tag.as_deref(),
                                    ))
                                    .when(command.pinned.unwrap_or_default(), |this| {
                                        this.child(
                                            div()
                                                .text_size(px(9.))
                                                .text_color(rgb(palette.warning))
                                                .child("📌"),
                                        )
                                    })
                                    .child(
                                        div()
                                            .min_w(px(64.))
                                            .max_w(px(140.))
                                            .text_size(px(11.))
                                            .font_weight(FontWeight(600.))
                                            .text_color(rgb(palette.text))
                                            .overflow_hidden()
                                            .child(truncate_preview(&command.label, 28)),
                                    )
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .font_family("JetBrains Mono")
                                            .text_size(px(11.))
                                            .text_color(rgb(palette.text_muted))
                                            .overflow_hidden()
                                            .child(truncate_preview(&command.command, 96)),
                                    ),
                            )
                            .child(actions)
                            .into_any_element()
                    }
                    QuickCommandViewMode::List => {
                        // Tauri list: badge + send + details + more.
                        let actions = quick_command_row_actions(
                            palette,
                            &command_id,
                            true,
                            execution_mode,
                            menu_open,
                            can_send_to_all,
                            cx.listener(move |this, _, _, cx| {
                                this.quick_command_menu_id = None;
                                this.run_quick_command_by_id(run_command_id.clone(), cx);
                            }),
                            cx.listener(move |this, _, window, cx| {
                                this.quick_command_menu_id = None;
                                this.open_quick_command_details(
                                    detail_command_id.clone(),
                                    window,
                                    cx,
                                );
                            }),
                            cx.listener({
                                let menu_command_id = command_id.clone();
                                move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    if this.quick_command_menu_id.as_deref()
                                        == Some(menu_command_id.as_str())
                                    {
                                        this.quick_command_menu_id = None;
                                    } else {
                                        this.quick_command_menu_id = Some(menu_command_id.clone());
                                    }
                                    cx.notify();
                                }
                            }),
                            cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.quick_command_menu_id = None;
                                this.open_edit_quick_command_editor(
                                    edit_command_id.clone(),
                                    window,
                                    cx,
                                );
                            }),
                            cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.quick_command_menu_id = None;
                                this.send_quick_command_to_all_by_id(all_command_id.clone(), cx);
                            }),
                            cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.quick_command_menu_id = None;
                                this.open_delete_quick_command_confirm(
                                    delete_command_id.clone(),
                                    cx,
                                );
                            }),
                        );

                        div()
                            .id(SharedString::from(format!(
                                "quick-command-list-{command_id}"
                            )))
                            .relative()
                            .min_h(px(44.))
                            .w_full()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(0x121820))
                            .px_2()
                            .py_1()
                            .flex()
                            .items_center()
                            .gap_2()
                            .hover(|this| this.bg(rgb(0x151d2a)))
                            .child(
                                div()
                                    .id(SharedString::from(format!(
                                        "quick-command-list-run-head-{command_id}"
                                    )))
                                    .min_w_0()
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.quick_command_menu_id = None;
                                        this.run_quick_command_by_id(
                                            list_header_command_id.clone(),
                                            cx,
                                        );
                                    }))
                                    .child(quick_command_icon_mark(
                                        palette,
                                        command.icon_tag.as_deref(),
                                        command.color_tag.as_deref(),
                                    ))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .min_w_0()
                                                            .flex_1()
                                                            .text_xs()
                                                            .font_weight(FontWeight(700.))
                                                            .text_color(rgb(palette.text))
                                                            .overflow_hidden()
                                                            .child(truncate_preview(
                                                                &command.label,
                                                                40,
                                                            )),
                                                    )
                                                    .when(
                                                        command.pinned.unwrap_or_default(),
                                                        |this| {
                                                            this.child(
                                                                div()
                                                                    .text_size(px(9.))
                                                                    .text_color(rgb(palette.warning))
                                                                    .child("📌"),
                                                            )
                                                        },
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .font_family("JetBrains Mono")
                                                    .text_size(px(11.))
                                                    .text_color(rgb(palette.text_muted))
                                                    .overflow_hidden()
                                                    .child(truncate_preview(
                                                        &command.command,
                                                        120,
                                                    )),
                                            ),
                                    ),
                            )
                            .child(actions)
                            .into_any_element()
                    }
                };

                rows = rows.child(command_item);
            }
        }

        // Tauri QuickCommands: PanelHeader-like strip with search + compact actions,
        // then category sidebar + command list (no page metrics cards).
        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(palette.surface))
            .child(
                div()
                    .h(px(36.))
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
                            .text_size(px(11.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text_muted))
                            .child("QUICK COMMANDS"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(if visible_commands == total_commands {
                                total_commands.to_string()
                            } else {
                                format!("{visible_commands}/{total_commands}")
                            }),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .id(SharedString::from("quick-command-search-input"))
                            .h(px(26.))
                            .w(px(144.))
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.bg))
                            .px_2()
                            .flex()
                            .items_center()
                            .gap_1()
                            .cursor_text()
                            .track_focus(&self.quick_command_search_focus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                window.focus(&this.quick_command_search_focus);
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.handle_quick_command_search_key_down(event, cx);
                            }))
                            .child(
                                svg()
                                    .size(px(14.))
                                    .flex_none()
                                    .path("icons/fe/search.svg")
                                    .text_color(rgb(palette.text_muted)),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .text_size(px(11.))
                                    .text_color(if self.quick_command_search_draft.is_empty() {
                                        rgb(palette.text_dimmed)
                                    } else {
                                        rgb(palette.text)
                                    })
                                    .child(if self.quick_command_search_draft.is_empty() {
                                        "Search".to_string()
                                    } else {
                                        truncate_preview(&self.quick_command_search_draft, 18)
                                    }),
                            ),
                    )
                    .child(mode_button(
                        "quick-command-view-list",
                        "List",
                        self.quick_command_view_mode == QuickCommandViewMode::List, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.set_quick_command_view_mode(QuickCommandViewMode::List, cx);
                        }),
                    ))
                    .child(mode_button(
                        "quick-command-view-compact",
                        "Cmp",
                        self.quick_command_view_mode == QuickCommandViewMode::Compact, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.set_quick_command_view_mode(QuickCommandViewMode::Compact, cx);
                        }),
                    ))
                    .child(mode_button(
                        "quick-command-view-tile",
                        "Tile",
                        self.quick_command_view_mode == QuickCommandViewMode::Tile, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.set_quick_command_view_mode(QuickCommandViewMode::Tile, cx);
                        }),
                    ))
                    .child(mode_button(
                        "quick-command-sort-created",
                        "New",
                        self.quick_command_sort_mode == QuickCommandSortMode::Created, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.set_quick_command_sort_mode(QuickCommandSortMode::Created, cx);
                        }),
                    ))
                    .child(mode_button(
                        "quick-command-sort-name",
                        "Name",
                        self.quick_command_sort_mode == QuickCommandSortMode::Name, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.set_quick_command_sort_mode(QuickCommandSortMode::Name, cx);
                        }),
                    ))
                    .child(mode_button(
                        "quick-command-sort-usage",
                        "Use",
                        self.quick_command_sort_mode == QuickCommandSortMode::Usage, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.set_quick_command_sort_mode(QuickCommandSortMode::Usage, cx);
                        }),
                    ))
                    .child(small_button(palette, 
                        "quick-command-add",
                        "Add",
                        cx.listener(|this, _, window, cx| {
                            this.open_new_quick_command_editor(window, cx);
                        }),
                    ))
                    .child(small_button(palette, 
                        "quick-command-import",
                        if self.quick_command_import_path_prompt.is_some() {
                            "..."
                        } else {
                            "Import"
                        },
                        cx.listener(|this, _, window, cx| {
                            this.open_quick_command_import_dialog(window, cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "quick-command-ai",
                        "AI",
                        cx.listener(|this, _, window, cx| {
                            // Tauri QuickCommands AI popover -> openAIAssistant(generate_command).
                            if this.ai_prompt_draft.trim().is_empty() {
                                this.ai_prompt_draft =
                                    "Generate a shell command for: ".to_string();
                            }
                            this.ai_response_preview =
                                "Describe the command you want, then Ask.".to_string();
                            this.ai_status = "quick command AI assist".to_string();
                            this.ensure_panel_open(NavItem::AiAssistant);
                            window.focus(&this.ai_chat_focus);
                            cx.notify();
                        }),
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p_2()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(category_sidebar)

                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .when(visible_commands > 12, |this| {
                                this.child(
                                    div()
                                        .rounded_sm()
                                        .border_1()
                                        .border_color(rgb(palette.border))
                                        .bg(rgb(palette.input))
                                        .p_2()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(format!(
                                            "Showing all {visible_commands} matching quick commands."
                                        )),
                                )
                            })
                            .child(
                                div()
                                    .id(SharedString::from("quick-command-rows-scroll"))
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_scroll()
                                    .scrollbar_width(px(6.))
                                    .child(rows),
                            ),
                    ),
            )
    }
}


fn quick_command_row_actions(
    palette: crate::ui::theme::ThemePalette,
    command_id: &str,
    show_badge: bool,
    execution_mode: &str,
    menu_open: bool,
    can_send_to_all: bool,
    on_run: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_details: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_more: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_edit: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_send_all: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_delete: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    // Tauri renderCommandActions: optional badge + Send + Details + More menu.
    div()
        .flex()
        .items_center()
        .gap_1()
        .flex_none()
        .when(show_badge, |this| {
            this.child(status_pill(
                if execution_mode == "append" {
                    "append"
                } else {
                    "exec"
                },
                if execution_mode == "append" {
                    rgb(palette.warning)
                } else {
                    rgb(palette.success)
                },
                if execution_mode == "append" {
                    rgb(0x32280f)
                } else {
                    rgb(palette.hover)
                },
            ))
        })
        .child(icon_button(
            format!("quick-command-run-{command_id}"),
            "▶",
            palette,
            on_run,
        ))
        .child(icon_button(
            format!("quick-command-detail-{command_id}"),
            "ⓘ",
            palette,
            on_details,
        ))
        .child(quick_command_more_menu(
            palette,
            command_id,
            menu_open,
            can_send_to_all,
            on_more,
            on_edit,
            on_send_all,
            on_delete,
        ))
}

fn quick_command_more_menu(
    palette: crate::ui::theme::ThemePalette,
    command_id: &str,
    menu_open: bool,
    can_send_to_all: bool,
    on_more: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_edit: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_send_all: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_delete: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .relative()
        .child(
            div()
                .id(SharedString::from(format!("quick-command-more-{command_id}")))
                .size(px(26.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_md()
                .text_color(rgb(palette.text_muted))
                .cursor_pointer()
                .hover(|this| {
                    this.bg(rgb(palette.surface_elevated))
                        .text_color(rgb(palette.text))
                })
                .child(
                    svg()
                        .size(px(14.))
                        .flex_none()
                        .path("icons/session/more.svg"),
                )
                .on_click(on_more),
        )
        .when(menu_open, move |this| {
            this.child(
                div()
                    .id(SharedString::from(format!(
                        "quick-command-more-menu-{command_id}"
                    )))
                    .absolute()
                    .top(px(28.))
                    .right(px(0.))
                    .w(px(148.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .shadow_lg()
                    .py_1()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(quick_command_menu_item(
                        palette,
                        format!("quick-command-menu-edit-{command_id}"),
                        "Edit",
                        false,
                        on_edit,
                    ))
                    .when(can_send_to_all, |this| {
                        this.child(quick_command_menu_item(
                            palette,
                            format!("quick-command-menu-all-{command_id}"),
                            "Send to all",
                            false,
                            on_send_all,
                        ))
                    })
                    .child(
                        div()
                            .mx_2()
                            .my_1()
                            .h(px(1.))
                            .bg(rgb(palette.border)),
                    )
                    .child(quick_command_menu_item(
                        palette,
                        format!("quick-command-menu-delete-{command_id}"),
                        "Delete",
                        true,
                        on_delete,
                    )),
            )
        })
}

fn quick_command_menu_item(
    palette: crate::ui::theme::ThemePalette,
    id: impl Into<String>,
    label: impl Into<SharedString>,
    destructive: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    div()
        .id(SharedString::from(id.into()))
        .px_3()
        .h(px(30.))
        .flex()
        .items_center()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)))
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(if destructive {
                    palette.danger
                } else {
                    palette.text
                }))
                .child(label),
        )
        .on_click(on_click)
}
