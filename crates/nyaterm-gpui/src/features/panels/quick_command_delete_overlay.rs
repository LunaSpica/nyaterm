use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn quick_command_delete_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let delete = self
            .quick_command_delete
            .clone()
            .unwrap_or(QuickCommandDeleteState {
                id: String::new(),
                label: "command".to_string(),
            });

        div()
            .id(SharedString::from("quick-command-delete-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id(SharedString::from("quick-command-delete-dialog"))
                    .w(px(380.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x7f1d1d))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0xfca5a5))
                            .child("Delete Quick Command"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .line_height(px(18.))
                            .text_color(rgb(0xcbd5e1))
                            .child(format!(
                                "Delete '{}' from Quick Commands? This cannot be undone.",
                                delete.label
                            )),
                    )
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "quick-command-delete-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_delete_quick_command(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "quick-command-delete-confirm",
                                "Delete",
                                cx.listener(|this, _, _, cx| {
                                    this.confirm_delete_quick_command(cx);
                                }),
                            )),
                    ),
            )
    }
}
