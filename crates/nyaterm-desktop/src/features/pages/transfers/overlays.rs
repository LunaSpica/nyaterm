use super::*;
use crate::features::TransferJobDeleteState;

impl NyaTermApp {
    pub(in crate::features) fn transfer_job_delete_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let state = self
            .transfer_job_delete
            .clone()
            .unwrap_or(TransferJobDeleteState {
                job_id: String::new(),
                title: String::new(),
            });

        div()
            .id(SharedString::from("transfer-job-delete-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer_job_delete_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer_job_delete_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                match event.keystroke.key.as_str() {
                    "escape" => this.cancel_delete_transfer_job(cx),
                    "enter" => this.confirm_delete_transfer_job(cx),
                    _ => {}
                }
            }))
            .child(
                div()
                    .w(px(460.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(800.))
                                    .child("Delete Transfer"),
                            )
                            .child(status_pill("queue item", rgb(0xfca5a5), rgb(0x3a1717))),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0xaeb7c8))
                            .child("Remove this transfer from the queue history."),
                    )
                    .child(
                        div()
                            .rounded_sm()
                            .bg(rgb(palette.input))
                            .p_3()
                            .font_family("JetBrains Mono")
                            .text_xs()
                            .text_color(rgb(palette.text))
                            .child(truncate_preview(&state.title, 64)),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "transfer-job-delete-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_delete_transfer_job(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "transfer-job-delete-confirm",
                                "Delete",
                                cx.listener(|this, _, _, cx| {
                                    this.confirm_delete_transfer_job(cx);
                                }),
                            )),
                    ),
            )
    }
}
