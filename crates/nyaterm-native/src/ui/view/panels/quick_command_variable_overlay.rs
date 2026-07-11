use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn quick_command_variable_prompt_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let prompt =
            self.quick_command_variable_prompt
                .clone()
                .unwrap_or(QuickCommandVariablePromptState {
                    command_id: String::new(),
                    label: "command".to_string(),
                    command: String::new(),
                    execute: true,
                    send_to_all: false,
                    variables: Vec::new(),
                    focused_index: 0,
                });
        let mut preview = prompt.command.clone();
        for variable in &prompt.variables {
            preview = preview.replace(&variable.raw, &variable.value);
        }

        let mut rows = div().mt_3().flex().flex_col().gap_2();
        for (index, variable) in prompt.variables.iter().cloned().enumerate() {
            let focused = prompt.focused_index == index;
            let variable_name = variable.name.clone();
            let field_id = format!("quick-command-variable-{index}");
            rows = rows.child(
                div()
                    .id(SharedString::from(field_id.clone()))
                    .rounded_sm()
                    .border_1()
                    .border_color(if focused {
                        rgb(0x4ade80)
                    } else {
                        rgb(0x263142)
                    })
                    .bg(if focused {
                        rgb(0x0f1f18)
                    } else {
                        rgb(0x0d1320)
                    })
                    .p_2()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.focus_quick_command_variable(index, cx);
                        window.focus(&this.quick_command_variable_focus);
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
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
                                            .text_size(px(10.))
                                            .text_color(rgb(0x64748b))
                                            .child(truncate_preview(&variable_name, 48)),
                                    )
                                    .child(if variable.options.is_empty() {
                                        transfer_input(
                                            field_id,
                                            "Value",
                                            variable.value.clone(),
                                            focused,
                    self.theme_palette(),
                )
                                        .track_focus(&self.quick_command_variable_focus)
                                        .into_any_element()
                                    } else {
                                        div()
                                            .font_family("JetBrains Mono")
                                            .text_xs()
                                            .text_color(rgb(0xe5edf7))
                                            .child(if variable.value.is_empty() {
                                                "-".to_string()
                                            } else {
                                                variable.value.clone()
                                            })
                                            .into_any_element()
                                    }),
                            )
                            .when(!variable.options.is_empty(), |this| {
                                this.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(small_button(
                                            format!("quick-command-variable-prev-{index}"),
                                            "Prev",
                                            cx.listener(move |this, _, _, cx| {
                                                this.cycle_quick_command_variable_option(
                                                    index, -1, cx,
                                                );
                                            }),
                                        ))
                                        .child(small_button(
                                            format!("quick-command-variable-next-{index}"),
                                            "Next",
                                            cx.listener(move |this, _, _, cx| {
                                                this.cycle_quick_command_variable_option(
                                                    index, 1, cx,
                                                );
                                            }),
                                        )),
                                )
                            }),
                    ),
            );
        }

        div()
            .id(SharedString::from("quick-command-variable-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.quick_command_variable_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.quick_command_variable_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                this.handle_quick_command_variable_key_down(event, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("quick-command-variable-dialog"))
                    .w(px(460.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(0xe5edf7))
                                            .child("Fill Quick Command Variables"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0x98a3b8))
                                            .child(truncate_preview(&prompt.label, 64)),
                                    ),
                            )
                            .child(status_pill(
                                if prompt.send_to_all {
                                    "all"
                                } else if prompt.execute {
                                    "run"
                                } else {
                                    "insert"
                                },
                                if prompt.send_to_all || prompt.execute {
                                    rgb(0x6ee7b7)
                                } else {
                                    rgb(0xfacc15)
                                },
                                if prompt.send_to_all || prompt.execute {
                                    rgb(0x12342a)
                                } else {
                                    rgb(0x32280f)
                                },
                            )),
                    )
                    .child(rows)
                    .child(
                        div()
                            .mt_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(0x263142))
                            .bg(rgb(0x0d1320))
                            .p_2()
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(0x64748b))
                                    .child("Preview"),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .font_family("JetBrains Mono")
                                    .text_xs()
                                    .line_height(px(18.))
                                    .text_color(rgb(0xaeb7c8))
                                    .child(if preview.trim().is_empty() {
                                        "Empty command".to_string()
                                    } else {
                                        truncate_preview(&preview, 280)
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(div().text_size(px(10.)).text_color(rgb(0x64748b)).child(
                                "Tab switches fields. Enter submits. Arrow keys cycle options.",
                            ))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(small_button(
                                        "quick-command-variable-cancel",
                                        "Cancel",
                                        cx.listener(|this, _, _, cx| {
                                            this.cancel_quick_command_variable_prompt(cx);
                                        }),
                                    ))
                                    .child(small_button(
                                        "quick-command-variable-submit",
                                        if prompt.execute { "Run" } else { "Insert" },
                                        cx.listener(|this, _, _, cx| {
                                            this.submit_quick_command_variable_prompt(cx);
                                        }),
                                    )),
                            ),
                    ),
            )
    }
}
