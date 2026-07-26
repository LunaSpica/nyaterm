use super::*;
use crate::temporary_ssh_link::parse_temporary_ssh_link;

impl NyaTermApp {
    pub(in crate::features) fn temporary_ssh_link_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let input_entity = cx.entity();
        let draft = self.temporary_ssh_link_draft.clone();
        let marked = self.temporary_ssh_link_marked_text.clone();
        let input_display = if draft.is_empty() && marked.is_empty() {
            self.tr("temporarySsh.placeholder").to_string()
        } else {
            format!("{draft}{marked}")
        };
        let parsed = parse_temporary_ssh_link(&draft);
        let can_submit = draft.trim().len() > 0 && parsed.is_ok();
        let error_key = self.temporary_ssh_link_error.or_else(|| {
            if draft.trim().is_empty() {
                None
            } else {
                parsed.as_ref().err().map(|error| error.locale_key())
            }
        });

        div()
            .id(SharedString::from("temporary-ssh-link-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
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
                    .w(px((self.last_viewport_size.0 - 32.).clamp(280., 480.)))
                    .max_w_full()
                    .mx_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .overflow_hidden()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div().px_6().pt_6().flex().items_start().gap_3().child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(div().child(crate::features::mono_icon(
                                            "icons/conn/flash.svg",
                                            rgb(palette.link).into(),
                                            14.,
                                        )))
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight(800.))
                                                .text_color(rgb(palette.text))
                                                .child(self.tr("temporarySsh.title")),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .line_height(px(16.))
                                        .child(self.tr("temporarySsh.description")),
                                ),
                        ),
                    )
                    .child(
                        div()
                            .px_6()
                            .pt_4()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .id(SharedString::from("temporary-ssh-link-input"))
                                    .relative()
                                    .h(px(36.))
                                    .rounded_md()
                                    .border_1()
                                    .border_color(if error_key.is_some() {
                                        rgb(palette.danger)
                                    } else {
                                        rgb(palette.border)
                                    })
                                    .bg(rgb(palette.input))
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .font_family(crate::features::gpui_code_font_family())
                                    .text_sm()
                                    .text_color(if draft.is_empty() && marked.is_empty() {
                                        rgb(palette.text_muted)
                                    } else {
                                        rgb(palette.text)
                                    })
                                    .child(div().min_w_0().overflow_hidden().child(input_display))
                                    .child(
                                        gpui::canvas(
                                            |_bounds, _window, _cx| {},
                                            move |bounds, _state, window, cx| {
                                                let focus = input_entity
                                                    .read(cx)
                                                    .temporary_ssh_link_focus
                                                    .clone();
                                                window.handle_input(
                                                    &focus,
                                                    gpui::ElementInputHandler::new(
                                                        bounds,
                                                        input_entity.clone(),
                                                    ),
                                                    cx,
                                                );
                                            },
                                        )
                                        .absolute()
                                        .inset_0(),
                                    ),
                            )
                            .when_some(error_key, |this, key| {
                                this.child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.danger))
                                        .child(self.tr(key)),
                                )
                            }),
                    )
                    .child(
                        div()
                            .px_6()
                            .py_6()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "temporary-ssh-link-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_temporary_ssh_link_dialog(cx);
                                }),
                            ))
                            .child(if can_submit {
                                dialog_action_button(
                                    palette,
                                    "temporary-ssh-link-connect",
                                    self.tr("temporarySsh.connect"),
                                    false,
                                    cx.listener(|this, _, window, cx| {
                                        this.submit_temporary_ssh_link_dialog(window, cx);
                                    }),
                                )
                                .into_any_element()
                            } else {
                                div()
                                    .id("temporary-ssh-link-connect-disabled")
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
                                    .child(self.tr("temporarySsh.connect"))
                                    .into_any_element()
                            }),
                    ),
            )
    }
}
