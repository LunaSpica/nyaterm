use super::*;

impl NyaTermApp {
    pub(super) fn quick_command_rows(
        &mut self,
        filtered_commands: Vec<QuickCommand>,
        total_commands: usize,
        can_send_to_all: bool,
        palette: crate::theme::ThemePalette,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
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
                        this.child(small_button(
                            palette,
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
                                                                    .text_color(
                                                                        rgb(palette.warning),
                                                                    )
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
                                                    .child(truncate_preview(&command.command, 120)),
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
        rows
    }
}
