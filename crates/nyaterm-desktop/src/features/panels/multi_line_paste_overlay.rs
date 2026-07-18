use super::*;

impl NyaTermApp {
    pub(in crate::features) fn multi_line_paste_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let draft = self
            .multi_line_paste
            .clone()
            .unwrap_or_else(|| MultiLinePasteDraft::new(String::new()));
        let input_entity = cx.entity();
        let draft_text = format!("{}{}", draft.text, self.multi_line_paste_marked_text);
        let normalized = normalize_paste_newlines(&draft_text);
        let stats = self
            .tr("terminal.multiLinePasteStats")
            .replace("{{lines}}", &normalized.split('\n').count().to_string())
            .replace("{{chars}}", &draft_text.chars().count().to_string());
        let can_send = !draft_text.is_empty();
        let mut preview = div()
            .id(SharedString::from("multi-line-paste-text"))
            .mt_3()
            .h(px(190.))
            .overflow_hidden()
            .rounded_sm()
            .border_1()
            .border_color(if can_send {
                rgb(palette.border)
            } else {
                rgb(0x7f1d1d)
            })
            .bg(rgb(palette.input))
            .p_3()
            .font_family(crate::features::gpui_code_font_family())
            .text_xs()
            .line_height(px(18.))
            .text_color(if can_send {
                rgb(palette.text)
            } else {
                rgb(palette.text_muted)
            });
        let display_lines = normalized
            .lines()
            .map(ToString::to_string)
            .chain(
                normalized
                    .ends_with('\n')
                    .then_some(String::new())
                    .into_iter(),
            )
            .take(10)
            .collect::<Vec<_>>();
        if display_lines.is_empty() {
            preview = preview.child(self.tr("terminal.multiLinePasteTextPlaceholder"));
        } else {
            for line in display_lines {
                let line_preview = if line.is_empty() {
                    " ".to_string()
                } else {
                    truncate_preview(&line, 92)
                };
                preview = preview.child(div().whitespace_nowrap().child(line_preview));
            }
        }

        div()
            .id(SharedString::from("multi-line-paste-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x000000d9))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.multi_line_paste_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.multi_line_paste_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                this.handle_multi_line_paste_key_down(event, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("multi-line-paste-dialog"))
                    .w(px(620.))
                    .max_w_full()
                    .mx_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("terminal.multiLinePasteTitle")),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(stats),
                    )
                    .child(
                        preview
                            .relative()
                            .track_focus(&self.multi_line_paste_focus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                window.focus(&this.multi_line_paste_focus);
                                cx.notify();
                            }))
                            .child(
                                gpui::canvas(
                                    |_bounds, _window, _cx| {},
                                    move |bounds, _state, window, cx| {
                                        let focus =
                                            input_entity.read(cx).multi_line_paste_focus.clone();
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
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("terminal.multiLinePasteDescription")),
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
                                "multi-line-paste-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_multi_line_paste(cx);
                                }),
                            ))
                            .child(div().when(!can_send, |this| this.opacity(0.45)).child(
                                small_button(
                                    palette,
                                    "multi-line-paste-direct",
                                    self.tr("terminal.multiLinePasteDirect"),
                                    cx.listener(|this, _, _, cx| {
                                        this.direct_multi_line_paste(cx);
                                    }),
                                ),
                            ))
                            .child(div().when(!can_send, |this| this.opacity(0.45)).child(
                                small_button(
                                    palette,
                                    "multi-line-paste-line",
                                    self.tr("terminal.multiLinePasteSendLineByLine"),
                                    cx.listener(|this, _, _, cx| {
                                        this.send_multi_line_paste_by_line(cx);
                                    }),
                                ),
                            )),
                    ),
            )
    }
}
