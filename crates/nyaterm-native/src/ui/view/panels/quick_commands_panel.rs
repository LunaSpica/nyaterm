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

        let mut category_sidebar = div()
            .w(px(140.))
            .flex_shrink_0()
            .pr_2()
            .border_r_1()
            .border_color(rgb(0x30363d))
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
                            rgb(0x12171f)
                        } else {
                            rgba(0x00000000)
                        })
                        .text_xs()
                        .text_color(if selected {
                            rgb(0x58a6ff)
                        } else {
                            rgb(0xc9d1d9)
                        })
                        .cursor_pointer()
                        .hover(|this| this.bg(rgb(0x1c2128)))
                        .child(div().size(px(6.)).rounded_full().bg(if selected {
                            rgb(0x58a6ff)
                        } else {
                            rgb(0x6e7681)
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
                                    rgb(0x202633)
                                })
                                .text_size(px(10.))
                                .text_color(if selected {
                                    rgb(0xbfdbfe)
                                } else {
                                    rgb(0x98a3b8)
                                })
                                .child(option.count.to_string()),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.quick_command_selected_category = id.clone();
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
                            .child(small_button(
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
                            .child(small_button(
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
            .gap_2()
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
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x0d1320))
                    .p_3()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap_3()
                    .text_xs()
                    .text_color(rgb(0x98a3b8))
                    .line_height(px(18.))
                    .child(if total_commands == 0 {
                        "No quick commands saved yet."
                    } else {
                        "No quick commands match the current filters."
                    })
                    .when(total_commands == 0, |this| {
                        this.child(small_button(
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
                let insert_command_id = command.id.clone();
                let run_command_id = command.id.clone();
                let compact_click_command_id = command.id.clone();
                let list_header_command_id = command.id.clone();
                let list_body_command_id = command.id.clone();
                let edit_command_id = command.id.clone();
                let delete_command_id = command.id.clone();
                let detail_command_id = command.id.clone();
                let all_command_id = command.id.clone();
                let category =
                    quick_command_category_label(&self.quick_command_categories, &command);
                let execution_mode = if command.execution_mode.as_deref() == Some("append") {
                    "append"
                } else {
                    "execute"
                };
                let risk = risk_label(command.risk_level.as_ref());
                let meta = format!(
                    "{} · used {} · {}",
                    category,
                    command.use_count.unwrap_or_default(),
                    risk
                );
                let command_item = match self.quick_command_view_mode {
                    QuickCommandViewMode::Tile => div()
                        .id(SharedString::from(format!(
                            "quick-command-tile-{command_id}"
                        )))
                        .min_w(px(132.))
                        .max_w(px(220.))
                        .h(px(34.))
                        .rounded_sm()
                        .border_1()
                        .border_color(rgb(0x30363d))
                        .bg(rgb(0x0d1320))
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
                                    this.run_quick_command_by_id(run_command_id.clone(), cx);
                                }))
                                .child(quick_command_icon_mark(
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
                                                .text_color(rgb(0xe5edf7))
                                                .overflow_hidden()
                                                .child(truncate_preview(&command.label, 28)),
                                        )
                                        .when(command.pinned.unwrap_or_default(), |this| {
                                            this.child(
                                                div()
                                                    .text_size(px(9.))
                                                    .text_color(rgb(0xfacc15))
                                                    .child("PIN"),
                                            )
                                        }),
                                ),
                        )
                        .child(status_pill(
                            if execution_mode == "append" { "+" } else { ">" },
                            if execution_mode == "append" {
                                rgb(0xfacc15)
                            } else {
                                rgb(0x6ee7b7)
                            },
                            if execution_mode == "append" {
                                rgb(0x32280f)
                            } else {
                                rgb(0x12342a)
                            },
                        ))
                        .child(small_button(
                            format!("quick-command-tile-detail-{command_id}"),
                            "Info",
                            cx.listener(move |this, _, window, cx| {
                                this.open_quick_command_details(
                                    detail_command_id.clone(),
                                    window,
                                    cx,
                                );
                            }),
                        ))
                        .when(can_send_to_all, |this| {
                            this.child(small_button(
                                format!("quick-command-tile-all-{command_id}"),
                                "All",
                                cx.listener(move |this, _, _, cx| {
                                    this.send_quick_command_to_all_by_id(
                                        all_command_id.clone(),
                                        cx,
                                    );
                                }),
                            ))
                        })
                        .into_any_element(),
                    QuickCommandViewMode::Compact => div()
                        .id(SharedString::from(format!(
                            "quick-command-compact-{command_id}"
                        )))
                        .min_h(px(38.))
                        .rounded_sm()
                        .border_1()
                        .border_color(rgb(0x30363d))
                        .bg(rgb(0x0d1320))
                        .px_2()
                        .py_1()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(quick_command_icon_mark(
                            command.icon_tag.as_deref(),
                            command.color_tag.as_deref(),
                        ))
                        .child(
                            div()
                                .id(SharedString::from(format!(
                                    "quick-command-compact-run-area-{command_id}"
                                )))
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap_2()
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.run_quick_command_by_id(
                                        compact_click_command_id.clone(),
                                        cx,
                                    );
                                }))
                                .child(
                                    div()
                                        .min_w(px(88.))
                                        .max_w(px(180.))
                                        .text_xs()
                                        .font_weight(FontWeight(800.))
                                        .text_color(rgb(0xe5edf7))
                                        .overflow_hidden()
                                        .child(truncate_preview(&command.label, 30)),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .font_family("JetBrains Mono")
                                        .text_xs()
                                        .text_color(rgb(0xaeb7c8))
                                        .overflow_hidden()
                                        .child(truncate_preview(&command.command, 96)),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(small_button(
                                    format!("quick-command-compact-detail-{command_id}"),
                                    "View",
                                    cx.listener(move |this, _, window, cx| {
                                        this.open_quick_command_details(
                                            detail_command_id.clone(),
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                                .when(can_send_to_all, |this| {
                                    this.child(small_button(
                                        format!("quick-command-compact-all-{command_id}"),
                                        "All",
                                        cx.listener(move |this, _, _, cx| {
                                            this.send_quick_command_to_all_by_id(
                                                all_command_id.clone(),
                                                cx,
                                            );
                                        }),
                                    ))
                                })
                                .child(small_button(
                                    format!("quick-command-compact-edit-{command_id}"),
                                    "Edit",
                                    cx.listener(move |this, _, window, cx| {
                                        this.open_edit_quick_command_editor(
                                            edit_command_id.clone(),
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(small_button(
                                    format!("quick-command-compact-insert-{command_id}"),
                                    "Insert",
                                    cx.listener(move |this, _, _, cx| {
                                        this.insert_quick_command_by_id(
                                            insert_command_id.clone(),
                                            cx,
                                        );
                                    }),
                                ))
                                .child(small_button(
                                    format!("quick-command-compact-run-{command_id}"),
                                    "Run",
                                    cx.listener(move |this, _, _, cx| {
                                        this.run_quick_command_by_id(run_command_id.clone(), cx);
                                    }),
                                ))
                                .child(small_button(
                                    format!("quick-command-compact-delete-{command_id}"),
                                    "Delete",
                                    cx.listener(move |this, _, _, cx| {
                                        this.open_delete_quick_command_confirm(
                                            delete_command_id.clone(),
                                            cx,
                                        );
                                    }),
                                )),
                        )
                        .into_any_element(),
                    QuickCommandViewMode::List => div()
                        .id(SharedString::from(format!(
                            "quick-command-list-{command_id}"
                        )))
                        .rounded_sm()
                        .border_1()
                        .border_color(rgb(0x30363d))
                        .bg(rgb(0x0d1320))
                        .p_3()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_3()
                                .child(
                                    div()
                                        .id(SharedString::from(format!(
                                            "quick-command-list-run-head-{command_id}"
                                        )))
                                        .min_w_0()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .cursor_pointer()
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.run_quick_command_by_id(
                                                list_header_command_id.clone(),
                                                cx,
                                            );
                                        }))
                                        .child(quick_command_icon_mark(
                                            command.icon_tag.as_deref(),
                                            command.color_tag.as_deref(),
                                        ))
                                        .child(
                                            div()
                                                .min_w_0()
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .min_w_0()
                                                        .text_xs()
                                                        .font_weight(FontWeight(800.))
                                                        .text_color(rgb(0xe5edf7))
                                                        .child(truncate_preview(
                                                            &command.label,
                                                            44,
                                                        )),
                                                )
                                                .when(command.pinned.unwrap_or_default(), |this| {
                                                    this.child(
                                                        div()
                                                            .text_size(px(10.))
                                                            .text_color(rgb(0xfacc15))
                                                            .child("PIN"),
                                                    )
                                                }),
                                        ),
                                )
                                .child(status_pill(
                                    execution_mode,
                                    if execution_mode == "append" {
                                        rgb(0xfacc15)
                                    } else {
                                        rgb(0x6ee7b7)
                                    },
                                    if execution_mode == "append" {
                                        rgb(0x32280f)
                                    } else {
                                        rgb(0x12342a)
                                    },
                                )),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!(
                                    "quick-command-list-run-body-{command_id}"
                                )))
                                .font_family("JetBrains Mono")
                                .text_xs()
                                .text_color(rgb(0xaeb7c8))
                                .line_height(px(18.))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.run_quick_command_by_id(list_body_command_id.clone(), cx);
                                }))
                                .child(truncate_preview(&command.command, 140)),
                        )
                        .when(command.description.is_some(), |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x98a3b8))
                                    .line_height(px(18.))
                                    .child(truncate_preview(
                                        command.description.as_deref().unwrap_or_default(),
                                        120,
                                    )),
                            )
                        })
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
                                        .text_color(rgb(0x6e7681))
                                        .child(meta),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(small_button(
                                            format!("quick-command-detail-{command_id}"),
                                            "View",
                                            cx.listener(move |this, _, window, cx| {
                                                this.open_quick_command_details(
                                                    detail_command_id.clone(),
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .when(can_send_to_all, |this| {
                                            this.child(small_button(
                                                format!("quick-command-all-{command_id}"),
                                                "All",
                                                cx.listener(move |this, _, _, cx| {
                                                    this.send_quick_command_to_all_by_id(
                                                        all_command_id.clone(),
                                                        cx,
                                                    );
                                                }),
                                            ))
                                        })
                                        .child(small_button(
                                            format!("quick-command-edit-{command_id}"),
                                            "Edit",
                                            cx.listener(move |this, _, window, cx| {
                                                this.open_edit_quick_command_editor(
                                                    edit_command_id.clone(),
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
                                            format!("quick-command-insert-{command_id}"),
                                            "Insert",
                                            cx.listener(move |this, _, _, cx| {
                                                this.insert_quick_command_by_id(
                                                    insert_command_id.clone(),
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
                                            format!("quick-command-run-{command_id}"),
                                            "Run",
                                            cx.listener(move |this, _, _, cx| {
                                                this.run_quick_command_by_id(
                                                    run_command_id.clone(),
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
                                            format!("quick-command-delete-{command_id}"),
                                            "Delete",
                                            cx.listener(move |this, _, _, cx| {
                                                this.open_delete_quick_command_confirm(
                                                    delete_command_id.clone(),
                                                    cx,
                                                );
                                            }),
                                        )),
                                ),
                        )
                        .into_any_element(),
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
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(0x8b949e))
                            .child("QUICK COMMANDS"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x6e7681))
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
                            .border_color(rgb(0x30363d))
                            .bg(rgb(0x0d1117))
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
                                    .text_color(rgb(0x8b949e)),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .text_size(px(11.))
                                    .text_color(if self.quick_command_search_draft.is_empty() {
                                        rgb(0x6e7681)
                                    } else {
                                        rgb(0xc9d1d9)
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
                        self.quick_command_view_mode == QuickCommandViewMode::List,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_view_mode(QuickCommandViewMode::List, cx);
                        }),
                    ))
                    .child(mode_button(
                        "quick-command-view-compact",
                        "Cmp",
                        self.quick_command_view_mode == QuickCommandViewMode::Compact,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_view_mode(QuickCommandViewMode::Compact, cx);
                        }),
                    ))
                    .child(mode_button(
                        "quick-command-view-tile",
                        "Tile",
                        self.quick_command_view_mode == QuickCommandViewMode::Tile,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_view_mode(QuickCommandViewMode::Tile, cx);
                        }),
                    ))
                    .child(mode_button(
                        "quick-command-sort-created",
                        "New",
                        self.quick_command_sort_mode == QuickCommandSortMode::Created,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_sort_mode(QuickCommandSortMode::Created, cx);
                        }),
                    ))
                    .child(mode_button(
                        "quick-command-sort-name",
                        "Name",
                        self.quick_command_sort_mode == QuickCommandSortMode::Name,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_sort_mode(QuickCommandSortMode::Name, cx);
                        }),
                    ))
                    .child(mode_button(
                        "quick-command-sort-usage",
                        "Use",
                        self.quick_command_sort_mode == QuickCommandSortMode::Usage,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_sort_mode(QuickCommandSortMode::Usage, cx);
                        }),
                    ))
                    .child(small_button(
                        "quick-command-add",
                        "Add",
                        cx.listener(|this, _, window, cx| {
                            this.open_new_quick_command_editor(window, cx);
                        }),
                    ))
                    .child(small_button(
                        "quick-command-import",
                        if self.quick_command_import_path_prompt.is_some() {
                            "..."
                        } else {
                            "Import"
                        },
                        cx.listener(|this, _, window, cx| {
                            this.open_quick_command_import_dialog(window, cx);
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
                            .gap_3()
                            .when(visible_commands > 12, |this| {
                                this.child(
                                    div()
                                        .rounded_sm()
                                        .border_1()
                                        .border_color(rgb(0x30363d))
                                        .bg(rgb(0x0d1320))
                                        .p_2()
                                        .text_xs()
                                        .text_color(rgb(0x98a3b8))
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
