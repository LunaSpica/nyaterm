use super::*;

impl NyaTermApp {
    pub(super) fn quick_command_rows(
        &mut self,
        filtered_commands: Vec<QuickCommand>,
        total_commands: usize,
        palette: crate::theme::ThemePalette,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let mut rows = div()
            .flex()
            .gap(px(6.))
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
                    .mt_8()
                    .mx_auto()
                    .w_full()
                    .max_w(px(384.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .p_4()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .line_height(px(18.))
                    .opacity(0.72)
                    .child(
                        svg()
                            .size(px(24.))
                            .flex_none()
                            .text_color(rgb(palette.text_muted))
                            .path("icons/conn/terminal.svg"),
                    )
                    .child(self.tr("quickCommands.noCommandsFound"))
                    .when(total_commands == 0, |this| {
                        this.child(small_button(
                            palette,
                            "quick-command-empty-add",
                            self.tr("quickCommands.addCommand"),
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
                let detail_command_id = command.id.clone();
                let execution_mode = if command.execution_mode.as_deref() == Some("append") {
                    "append"
                } else {
                    "execute"
                };
                let menu_open = self
                    .quick_command_menu
                    .as_ref()
                    .is_some_and(|menu| menu.command_id == command.id);
                let command_item = match self.quick_command_view_mode {
                    QuickCommandViewMode::Tile => div()
                        .id(SharedString::from(format!(
                            "quick-command-tile-{command_id}"
                        )))
                        .relative()
                        .max_w(px(220.))
                        .h(px(26.))
                        .rounded_md()
                        .border_1()
                        .border_color(rgba((palette.border << 8) | 0x59))
                        .bg(rgba((palette.surface_elevated << 8) | 0x33))
                        .px_2()
                        .flex()
                        .items_center()
                        .gap_1()
                        .cursor_pointer()
                        .hover(move |this| {
                            this.bg(rgba((palette.surface_elevated << 8) | 0x80))
                        })
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener({
                                let menu_command_id = command_id.clone();
                                move |this, event: &gpui::MouseDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.quick_command_menu = Some(QuickCommandRowMenuState {
                                        command_id: menu_command_id.clone(),
                                        x: event.position.x,
                                        y: event.position.y,
                                    });
                                    cx.notify();
                                }
                            }),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.quick_command_menu = None;
                            this.run_quick_command_by_id(run_command_id.clone(), cx);
                        }))
                        .child(quick_command_icon_mark(
                            palette,
                            command.icon_tag.as_deref(),
                            command.color_tag.as_deref(),
                        ))
                        .when(command.pinned.unwrap_or_default(), |this| {
                            this.child(quick_command_pin_mark(palette))
                        })
                        .child(
                            div()
                                .min_w_0()
                                .text_size(px(11.))
                                .font_weight(FontWeight(500.))
                                .text_color(rgb(palette.text))
                                .overflow_hidden()
                                .child(truncate_preview(&command.label, 28)),
                        )
                        .tooltip({
                            let label = command.label.clone();
                            let command_text = command.command.clone();
                            move |_, cx| {
                                cx.new(|_| {
                                    ChromeTooltip::new(format!(
                                        "{}\n{}",
                                        label,
                                        truncate_preview(&command_text, 120)
                                    ))
                                })
                                .into()
                            }
                        })
                        .into_any_element(),
                    QuickCommandViewMode::Compact => {
                        // Tauri compact: send + details + more (edit / send-all / delete).
                        let actions = quick_command_row_actions(
                            palette,
                            &command_id,
                            false,
                            execution_mode,
                            menu_open,
                            cx.listener(move |this, _, _, cx| {
                                this.quick_command_menu = None;
                                this.run_quick_command_by_id(run_command_id.clone(), cx);
                            }),
                            cx.listener(move |this, event: &ClickEvent, window, cx| {
                                this.quick_command_menu = None;
                                let position = event.position();
                                this.open_quick_command_details(
                                    detail_command_id.clone(),
                                    position.x,
                                    position.y,
                                    window,
                                    cx,
                                );
                            }),
                            cx.listener({
                                let menu_command_id = command_id.clone();
                                move |this, event: &ClickEvent, _, cx| {
                                    cx.stop_propagation();
                                    if this
                                        .quick_command_menu
                                        .as_ref()
                                        .is_some_and(|menu| menu.command_id == menu_command_id)
                                    {
                                        this.quick_command_menu = None;
                                    } else {
                                        let position = event.position();
                                        this.quick_command_menu = Some(QuickCommandRowMenuState {
                                            command_id: menu_command_id.clone(),
                                            x: position.x,
                                            y: position.y,
                                        });
                                    }
                                    cx.notify();
                                }
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
                            .hover(move |this| {
                                this.bg(rgba((palette.surface_elevated << 8) | 0x73))
                            })
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener({
                                    let menu_command_id = command_id.clone();
                                    move |this, event: &gpui::MouseDownEvent, _, cx| {
                                        cx.stop_propagation();
                                        this.quick_command_menu = Some(QuickCommandRowMenuState {
                                            command_id: menu_command_id.clone(),
                                            x: event.position.x,
                                            y: event.position.y,
                                        });
                                        cx.notify();
                                    }
                                }),
                            )
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
                                        this.quick_command_menu = None;
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
                                        this.child(quick_command_pin_mark(palette))
                                    })
                                    .child(
                                        div()
                                            .min_w(px(64.))
                                            .max_w(px(140.))
                                            .text_size(px(11.))
                                            .font_weight(FontWeight(500.))
                                            .text_color(rgb(palette.text))
                                            .overflow_hidden()
                                            .child(truncate_preview(&command.label, 28)),
                                    )
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .font_family(crate::features::gpui_code_font_family())
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
                            cx.listener(move |this, _, _, cx| {
                                this.quick_command_menu = None;
                                this.run_quick_command_by_id(run_command_id.clone(), cx);
                            }),
                            cx.listener(move |this, event: &ClickEvent, window, cx| {
                                this.quick_command_menu = None;
                                let position = event.position();
                                this.open_quick_command_details(
                                    detail_command_id.clone(),
                                    position.x,
                                    position.y,
                                    window,
                                    cx,
                                );
                            }),
                            cx.listener({
                                let menu_command_id = command_id.clone();
                                move |this, event: &ClickEvent, _, cx| {
                                    cx.stop_propagation();
                                    if this
                                        .quick_command_menu
                                        .as_ref()
                                        .is_some_and(|menu| menu.command_id == menu_command_id)
                                    {
                                        this.quick_command_menu = None;
                                    } else {
                                        let position = event.position();
                                        this.quick_command_menu = Some(QuickCommandRowMenuState {
                                            command_id: menu_command_id.clone(),
                                            x: position.x,
                                            y: position.y,
                                        });
                                    }
                                    cx.notify();
                                }
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
                            .border_color(rgba((palette.border << 8) | 0x59))
                            .bg(rgba((palette.surface_elevated << 8) | 0x26))
                            .px_2()
                            .py_1()
                            .flex()
                            .items_center()
                            .gap_2()
                            .hover(move |this| {
                                this.bg(rgba((palette.surface_elevated << 8) | 0x73))
                            })
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener({
                                    let menu_command_id = command_id.clone();
                                    move |this, event: &gpui::MouseDownEvent, _, cx| {
                                        cx.stop_propagation();
                                        this.quick_command_menu = Some(QuickCommandRowMenuState {
                                            command_id: menu_command_id.clone(),
                                            x: event.position.x,
                                            y: event.position.y,
                                        });
                                        cx.notify();
                                    }
                                }),
                            )
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
                                        this.quick_command_menu = None;
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
                                                            .font_weight(FontWeight(500.))
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
                                                            this.child(quick_command_pin_mark(
                                                                palette,
                                                            ))
                                                        },
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .font_family(
                                                        crate::features::gpui_code_font_family(),
                                                    )
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
