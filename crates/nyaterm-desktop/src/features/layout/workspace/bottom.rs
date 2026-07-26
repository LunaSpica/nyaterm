use super::*;

use crate::models::BottomPanelMode;

impl NyaTermApp {
    pub(in crate::features) fn bottom_panel_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match self.bottom_panel {
            BottomPanelMode::QuickCommands => {
                let palette = self.theme_palette();
                div()
                    .h(px(self.quick_cmd_height.clamp(36., 520.)))
                    .flex_none()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.surface))
                    .child(self.quick_commands_panel(cx))
                    .into_any_element()
            }
            BottomPanelMode::CommandSend => self.bottom_command_send_bar(cx).into_any_element(),
            BottomPanelMode::Hidden => div().into_any_element(),
        }
    }

    pub(in crate::features) fn bottom_quick_commands_bar(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();

        let commands = sorted_quick_commands(&self.quick_commands);
        let mut command_row = div()
            .flex()
            .items_center()
            .gap_2()
            .min_w_0()
            .overflow_hidden();
        if commands.is_empty() {
            command_row = command_row.child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.text_dimmed))
                    .child("No quick commands saved."),
            );
        } else {
            for (index, command) in commands.into_iter().take(5).enumerate() {
                command_row = command_row.child(self.bottom_quick_command_chip(index, command, cx));
            }
        }

        div()
            .h(px(112.))
            .flex_none()
            .border_t_1()
            .border_color(rgb(palette.border))
            .bg(self.shell_surface_color(palette.surface))
            .p_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text_muted))
                                    .child("Quick Commands"),
                            )
                            .child(status_pill(
                                "bottom panel",
                                rgb(palette.link),
                                rgb(palette.hover),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "bottom-command-search",
                                "Search",
                                cx.listener(|this, _, window, cx| {
                                    window.focus(&this.command_search_focus);
                                    this.terminal.view.status =
                                        "command search focused".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "bottom-command-refresh",
                                "Refresh",
                                cx.listener(|this, _, _, cx| {
                                    this.refresh_quick_commands();
                                    this.terminal.view.status =
                                        "quick commands refreshed".to_string();
                                    cx.notify();
                                }),
                            )),
                    ),
            )
            .child(div().mt_3().child(command_row))
    }

    pub(in crate::features) fn bottom_quick_command_chip(
        &self,
        index: usize,
        command: QuickCommand,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let title = truncate_preview(&command.label, 22);
        let preview = truncate_preview(&command.command, 38);
        let category = quick_command_category_label(&self.quick_command_categories, &command);
        let execute_label = if command.execution_mode.as_deref() == Some("append") {
            "Insert"
        } else {
            "Run"
        };
        let palette = self.theme_palette();

        div()
            .id(SharedString::from(format!("bottom-quick-command-{index}")))
            .w(px(176.))
            .h(px(56.))
            .flex_none()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(self.shell_surface_color(palette.surface))
            .p_2()
            .cursor_pointer()
            .hover(move |this| this.bg(rgb(palette.hover)))
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
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(palette.success))
                            .child(execute_label),
                    ),
            )
            .child(
                div()
                    .mt_1()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .child(format!("{category} / {preview}")),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if execute_label == "Insert" {
                    this.insert_quick_command(index, cx);
                } else {
                    this.run_quick_command(index, cx);
                }
            }))
    }
}
