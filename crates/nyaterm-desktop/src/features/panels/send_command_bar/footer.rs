use super::state::SendCommandBarViewState;
use super::*;

impl NyaTermApp {
    pub(super) fn send_command_bar_progress(
        &self,
        state: &SendCommandBarViewState,
    ) -> gpui::AnyElement {
        let palette = state.palette;
        let progress_label = state.progress_label.clone();
        let progress_ratio = state.progress_ratio;
        div()
            .flex_none()
            .px_1()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x1f6feb))
                    .bg(rgb(palette.bg))
                    .px_2()
                    .py_1()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight(600.))
                                    .text_color(rgb(palette.text))
                                    .child(progress_label.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text_dimmed))
                                    .child(format!("{:.0}%", progress_ratio * 100.0)),
                            ),
                    )
                    .child(
                        div()
                            .h(px(6.))
                            .w_full()
                            .rounded_full()
                            .bg(rgb(palette.surface_elevated))
                            .overflow_hidden()
                            .child(
                                div()
                                    .h_full()
                                    .w(px((280.0 * progress_ratio).max(2.0)))
                                    .rounded_full()
                                    .bg(rgb(0x1f6feb)),
                            ),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn send_command_bar_footer(
        &mut self,
        state: &SendCommandBarViewState,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let palette = state.palette;
        let validation_error = state.validation_error;
        let is_sending = state.is_sending;
        let progress_label = state.progress_label.clone();
        let validation_text = state.validation_text.clone();
        let preview = state.preview.clone();
        div()
            .flex_none()
            .h(px(34.))
            .pt_1()
            .border_t_1()
            .border_color(rgb(palette.border))
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap_0()
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(if validation_error && !is_sending {
                                rgb(palette.danger)
                            } else if is_sending {
                                rgb(palette.accent)
                            } else {
                                rgb(palette.text_muted)
                            })
                            .overflow_hidden()
                            .child(if is_sending {
                                progress_label
                            } else {
                                validation_text
                            }),
                    )
                    .child(
                        div()
                            .font_family("JetBrains Mono")
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_dimmed))
                            .overflow_hidden()
                            .child(if preview.trim().is_empty() {
                                "preview empty".to_string()
                            } else {
                                preview
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(small_button(
                        palette,
                        "bottom-command-send-clear",
                        "Clear",
                        cx.listener(|this, _, _, cx| {
                            this.send_command_draft.clear();
                            this.terminal_status = "command send cleared".to_string();
                            cx.notify();
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "bottom-command-send-now",
                        if is_sending { "Stop" } else { "Send" },
                        cx.listener(|this, _, _, cx| {
                            this.send_bottom_command(false, cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "bottom-command-send-enter",
                        if is_sending { "Stop" } else { "Send ↵" },
                        cx.listener(|this, _, _, cx| {
                            if this.send_command_sending {
                                this.stop_send_command(cx);
                            } else {
                                this.send_bottom_command(true, cx);
                            }
                        }),
                    )),
            )
            .into_any_element()
    }
}
