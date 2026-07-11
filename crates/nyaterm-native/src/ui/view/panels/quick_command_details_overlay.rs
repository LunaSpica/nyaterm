use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn quick_command_details_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let details =
            self.quick_command_details
                .clone()
                .unwrap_or_else(|| QuickCommandDetailsState {
                    command: QuickCommand {
                        id: String::new(),
                        label: "Quick Command".to_string(),
                        command: String::new(),
                        description: None,
                        category_id: None,
                        color_tag: None,
                        icon_tag: None,
                        pinned: None,
                        execution_mode: None,
                        source: None,
                        risk_level: None,
                        use_count: None,
                        created_at: None,
                        updated_at: None,
                    },
                    category: "Unsorted".to_string(),
                    risk: "unknown".to_string(),
                });
        let command = details.command;
        let command_id = command.id.clone();
        let edit_command_id = command.id.clone();
        let execution_mode = if command.execution_mode.as_deref() == Some("append") {
            "append only"
        } else {
            "execute immediately"
        };
        let description = command
            .description
            .as_deref()
            .map(str::trim)
            .filter(|description| !description.is_empty())
            .unwrap_or("No description.");

        div()
            .id(SharedString::from("quick-command-details-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.quick_command_details_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.quick_command_details_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                if event.keystroke.key == "escape" {
                    this.close_quick_command_details(cx);
                }
            }))
            .child(
                div()
                    .id(SharedString::from("quick-command-details-dialog"))
                    .w(px(560.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .items_start()
                                    .gap_3()
                                    .child(quick_command_icon_mark(
                                        command.icon_tag.as_deref(),
                                        command.color_tag.as_deref(),
                                    ))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_weight(FontWeight(800.))
                                                    .text_color(rgb(palette.text))
                                                    .child(truncate_preview(&command.label, 64)),
                                            )
                                            .child(
                                                div()
                                                    .mt_1()
                                                    .text_xs()
                                                    .text_color(rgb(palette.text_muted))
                                                    .child(format!(
                                                        "{} / used {} / {}",
                                                        details.category,
                                                        command.use_count.unwrap_or_default(),
                                                        details.risk
                                                    )),
                                            ),
                                    ),
                            )
                            .child(status_pill(
                                execution_mode,
                                if command.execution_mode.as_deref() == Some("append") {
                                    rgb(0xfacc15)
                                } else {
                                    rgb(palette.success)
                                },
                                if command.execution_mode.as_deref() == Some("append") {
                                    rgb(0x32280f)
                                } else {
                                    rgb(palette.hover)
                                },
                            )),
                    )
                    .child(
                        div()
                            .mt_4()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.input))
                            .p_3()
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text_muted))
                                    .child("Command"),
                            )
                            .child(
                                div()
                                    .mt_2()
                                    .max_h(px(180.))
                                    .overflow_hidden()
                                    .font_family("JetBrains Mono")
                                    .text_xs()
                                    .line_height(px(18.))
                                    .text_color(rgb(0xdbe4f0))
                                    .child(if command.command.trim().is_empty() {
                                        "Empty command".to_string()
                                    } else {
                                        command.command.clone()
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .mt_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.input))
                            .p_3()
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text_muted))
                                    .child("Description"),
                            )
                            .child(
                                div()
                                    .mt_2()
                                    .text_xs()
                                    .line_height(px(18.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(description.to_string()),
                            ),
                    )
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(format!("ID {}", truncate_preview(&command_id, 42))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(small_button(palette, 
                                        "quick-command-details-edit",
                                        "Edit",
                                        cx.listener(move |this, _, window, cx| {
                                            this.close_quick_command_details(cx);
                                            this.open_edit_quick_command_editor(
                                                edit_command_id.clone(),
                                                window,
                                                cx,
                                            );
                                        }),
                                    ))
                                    .child(small_button(palette, 
                                        "quick-command-details-copy",
                                        "Copy",
                                        cx.listener(|this, _, _, cx| {
                                            this.copy_quick_command_details(cx);
                                        }),
                                    ))
                                    .child(small_button(palette, 
                                        "quick-command-details-close",
                                        "Close",
                                        cx.listener(|this, _, _, cx| {
                                            this.close_quick_command_details(cx);
                                        }),
                                    )),
                            ),
                    ),
            )
    }
}
