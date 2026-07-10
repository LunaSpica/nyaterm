use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn close_all_sessions_confirm_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let sessions = self.ordered_sessions();
        let session_count = sessions.len();
        let mut session_list = div().mt_3().flex().flex_col().gap_1();
        for session in sessions.into_iter().take(6) {
            let name = self.session_display_name_by_info(&session);
            let short_session_id = short_id(&session.id).to_string();
            session_list = session_list.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .rounded_sm()
                    .bg(rgb(0x121821))
                    .px_2()
                    .py_1()
                    .text_xs()
                    .child(
                        div()
                            .min_w_0()
                            .overflow_hidden()
                            .text_color(rgb(0xe5edf7))
                            .child(truncate_preview(&name, 44)),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(10.))
                            .text_color(rgb(0x8f98aa))
                            .child(short_session_id),
                    ),
            );
        }
        if session_count > 6 {
            session_list = session_list.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x8f98aa))
                    .child(format!("and {} more session(s)", session_count - 6)),
            );
        }

        div()
            .id("close-all-sessions-confirm-overlay")
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x030508d8))
            .flex()
            .items_start()
            .justify_center()
            .pt(px(116.))
            .track_focus(&self.close_all_sessions_confirm_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.close_all_sessions_confirm_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                match event.keystroke.key.as_str() {
                    "escape" => this.cancel_close_all_sessions_confirm(cx),
                    "enter" => this.confirm_close_all_sessions(cx),
                    _ => {}
                }
            }))
            .child(
                div()
                    .id("close-all-sessions-confirm-dialog")
                    .w(px(430.))
                    .max_w_full()
                    .mx_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x3f1f27))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(0xfca5a5))
                                    .child("Close All Sessions"),
                            )
                            .child(div().text_xs().text_color(rgb(0x98a3b8)).child(format!(
                                "This will close {session_count} active session(s)."
                            ))),
                    )
                    .child(session_list)
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                "close-all-sessions-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_close_all_sessions_confirm(cx);
                                }),
                            ))
                            .child(
                                div()
                                    .id("close-all-sessions-confirm")
                                    .h(px(28.))
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_sm()
                                    .bg(rgb(0x7f1d1d))
                                    .text_xs()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(0xfee2e2))
                                    .cursor_pointer()
                                    .hover(|this| this.bg(rgb(0x991b1b)))
                                    .child("Close All")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.confirm_close_all_sessions(cx);
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }
}
