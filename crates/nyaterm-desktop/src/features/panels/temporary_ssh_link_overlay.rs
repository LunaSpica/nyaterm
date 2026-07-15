use super::*;
use crate::temporary_ssh_link::parse_temporary_ssh_link;

impl NyaTermApp {
    pub(in crate::features) fn temporary_ssh_link_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let draft = self.temporary_ssh_link_draft.clone();
        let input_display = if draft.is_empty() {
            "ssh://root@example.com:22 or ssh -p 2222 user@example.com".to_string()
        } else {
            draft.clone()
        };
        let parsed = parse_temporary_ssh_link(&draft);
        let can_submit = draft.trim().len() > 0 && parsed.is_ok();
        let error = self.temporary_ssh_link_error.clone().or_else(|| {
            if draft.trim().is_empty() {
                None
            } else {
                parsed
                    .as_ref()
                    .err()
                    .map(|error| error.message().to_string())
            }
        });
        let preview = parsed.ok().map(|config| {
            format!(
                "{}@{}:{} · {}",
                config.username, config.host, config.port, config.name
            )
        });

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
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .shadow_lg()
                    .overflow_hidden()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.section_header))
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
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_size(px(14.))
                                                    .text_color(rgb(palette.accent))
                                                    .child("⚡"),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_weight(FontWeight(800.))
                                                    .text_color(rgb(palette.text))
                                                    .child("Temporary SSH Link"),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .line_height(px(16.))
                                            .child(
                                                "Paste an ssh:// URL or ssh command. Password is requested securely during connect.",
                                            ),
                                    ),
                            )
                            .child(small_button(
                                palette,
                                "temporary-ssh-link-close",
                                "Close",
                                cx.listener(|this, _, _, cx| {
                                    this.close_temporary_ssh_link_dialog(cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .p_4()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                div()
                                    .id(SharedString::from("temporary-ssh-link-input"))
                                    .min_h(px(42.))
                                    .rounded_md()
                                    .border_1()
                                    .border_color(if error.is_some() {
                                        rgb(palette.danger)
                                    } else if can_submit {
                                        rgb(palette.accent)
                                    } else {
                                        rgb(palette.border)
                                    })
                                    .bg(rgb(palette.input))
                                    .px_3()
                                    .py_2()
                                    .font_family(crate::features::gpui_code_font_family())
                                    .text_sm()
                                    .text_color(if self.temporary_ssh_link_draft.is_empty() {
                                        rgb(palette.text_muted)
                                    } else {
                                        rgb(palette.text)
                                    })
                                    .child(truncate_preview(&input_display, 104)),
                            )
                            .when_some(error.clone(), |this, message| {
                                this.child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.danger))
                                        .child(message),
                                )
                            })
                            .when_some(preview, |this, summary| {
                                this.child(
                                    div()
                                        .rounded_md()
                                        .border_1()
                                        .border_color(rgb(palette.border))
                                        .bg(rgb(palette.bg))
                                        .px_3()
                                        .py_2()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .font_weight(FontWeight(700.))
                                                .text_color(rgb(palette.success))
                                                .child("Ready to connect"),
                                        )
                                        .child(
                                            div()
                                                .font_family(crate::features::gpui_code_font_family())
                                                .text_size(px(11.))
                                                .text_color(rgb(palette.text))
                                                .child(summary),
                                        ),
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
                                            .text_size(px(10.))
                                            .text_color(rgb(palette.text_dimmed))
                                            .child("Enter connects · Esc closes"),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(small_button(
                                                palette,
                                                "temporary-ssh-link-clear",
                                                "Clear",
                                                cx.listener(|this, _, _, cx| {
                                                    this.temporary_ssh_link_draft.clear();
                                                    this.temporary_ssh_link_error = None;
                                                    cx.notify();
                                                }),
                                            ))
                                            .child(if can_submit {
                                                small_button(
                                                    palette,
                                                    "temporary-ssh-link-connect",
                                                    "Connect",
                                                    cx.listener(|this, _, window, cx| {
                                                        this.submit_temporary_ssh_link_dialog(
                                                            window, cx,
                                                        );
                                                    }),
                                                )
                                                .into_any_element()
                                            } else {
                                                div()
                                                    .h(px(28.))
                                                    .px_3()
                                                    .flex()
                                                    .items_center()
                                                    .rounded_md()
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
                    ),
            )
    }
}
