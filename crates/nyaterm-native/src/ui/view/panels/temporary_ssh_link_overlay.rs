use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn temporary_ssh_link_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let input_display = if self.temporary_ssh_link_draft.is_empty() {
            "ssh://root@example.com:22 or ssh -p 2222 user@example.com".to_string()
        } else {
            self.temporary_ssh_link_draft.clone()
        };
        let can_submit = !self.temporary_ssh_link_draft.trim().is_empty();
        let error = self.temporary_ssh_link_error.clone();

        div()
            .id(SharedString::from("temporary-ssh-link-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x030508e6))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.temporary_ssh_link_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.temporary_ssh_link_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_temporary_ssh_link_key_down(event, window, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("temporary-ssh-link-dialog"))
                    .w(px(520.))
                    .max_w_full()
                    .mx_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(palette.text))
                                            .child("Temporary SSH Link"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child("Transient SSH session; password is requested securely during connect."),
                                    ),
                            )
                            .child(small_button(palette, 
                                "temporary-ssh-link-close",
                                "Close",
                                cx.listener(|this, _, _, cx| {
                                    this.close_temporary_ssh_link_dialog(cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .id(SharedString::from("temporary-ssh-link-input"))
                            .mt_4()
                            .min_h(px(42.))
                            .rounded_sm()
                            .border_1()
                            .border_color(if error.is_some() {
                                rgb(0x7f1d1d)
                            } else if can_submit {
                                rgb(0x334155)
                            } else {
                                rgb(palette.border)
                            })
                            .bg(rgb(palette.input))
                            .px_3()
                            .py_2()
                            .font_family("JetBrains Mono")
                            .text_sm()
                            .text_color(if self.temporary_ssh_link_draft.is_empty() {
                                rgb(palette.text_muted)
                            } else {
                                rgb(palette.text)
                            })
                            .child(truncate_preview(&input_display, 104)),
                    )
                    .when_some(error, |this, error| {
                        this.child(
                            div()
                                .mt_2()
                                .text_xs()
                                .text_color(rgb(0xfca5a5))
                                .child(error),
                        )
                    })
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text_muted))
                                    .child("Enter connects; Esc cancels; Ctrl/Cmd+V pastes."),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(small_button(palette, 
                                        "temporary-ssh-link-clear",
                                        "Clear",
                                        cx.listener(|this, _, _, cx| {
                                            this.temporary_ssh_link_draft.clear();
                                            this.temporary_ssh_link_error = None;
                                            cx.notify();
                                        }),
                                    ))
                                    .child(if can_submit {
                                        small_button(palette, 
                                            "temporary-ssh-link-connect",
                                            "Connect",
                                            cx.listener(|this, _, window, cx| {
                                                this.submit_temporary_ssh_link_dialog(window, cx);
                                            }),
                                        )
                                        .into_any_element()
                                    } else {
                                        div()
                                            .h(px(28.))
                                            .px_3()
                                            .flex()
                                            .items_center()
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(rgb(palette.border))
                                            .bg(rgb(palette.input))
                                            .text_color(rgb(palette.text_muted))
                                            .text_xs()
                                            .child("Connect")
                                            .into_any_element()
                                    }),
                            ),
                    ),
            )
    }
}
