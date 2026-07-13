use super::*;

impl NyaTermApp {
    pub(in crate::features) fn transfer_delete_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let state = self.transfer_delete.clone().unwrap_or(TransferDeleteState {
            remote_path: String::new(),
            name: String::new(),
            paths: Vec::new(),
        });
        let delete_count = state.paths.len().max(1);
        let preview = if delete_count == 1 {
            state.remote_path.clone()
        } else {
            state
                .paths
                .iter()
                .take(4)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        };

        div()
            .id(SharedString::from("transfer-delete-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer_delete_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer_delete_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_transfer_delete_key_down(event, window, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("transfer-delete-dialog"))
                    .w(px(380.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x7f1d1d))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0xfca5a5))
                            .child(if delete_count == 1 {
                                "Delete Remote Item"
                            } else {
                                "Delete Remote Items"
                            }),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text))
                            .child(truncate_preview(&state.name, 72)),
                    )
                    .child(
                        div()
                            .mt_2()
                            .font_family("JetBrains Mono")
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(truncate_preview(&preview, 160)),
                    )
                    .child(
                        div().mt_3().text_xs().text_color(rgb(0xfca5a5)).child(
                            if delete_count == 1 {
                                "Enter deletes recursively when this is a directory; Esc cancels."
                            } else {
                                "Enter deletes all marked items recursively when directories are included; Esc cancels."
                            },
                        ),
                    )
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(palette, 
                                "transfer-delete-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.close_transfer_delete_dialog(cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "transfer-delete-confirm",
                                "Delete",
                                cx.listener(|this, _, window, cx| {
                                    this.submit_transfer_delete(window, cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn transfer_move_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let state = self.transfer_move.clone().unwrap_or(TransferMoveState {
            old_path: String::new(),
            name: String::new(),
            value: String::new(),
        });
        let target = state.value.trim();
        let has_error = target.is_empty();
        let unchanged = target == state.old_path;
        let input_display = if state.value.is_empty() {
            "Target path".to_string()
        } else {
            state.value.clone()
        };

        div()
            .id(SharedString::from("transfer-move-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer_move_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer_move_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_transfer_move_key_down(event, window, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("transfer-move-dialog"))
                    .w(px(430.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(format!("Move {}", truncate_preview(&state.name, 48))),
                    )
                    .child(
                        div()
                            .mt_2()
                            .font_family("JetBrains Mono")
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(truncate_preview(&state.old_path, 92)),
                    )
                    .child(
                        div()
                            .id(SharedString::from("transfer-move-input"))
                            .mt_3()
                            .h(px(36.))
                            .rounded_sm()
                            .border_1()
                            .border_color(if has_error {
                                rgb(0x7f1d1d)
                            } else if unchanged {
                                rgb(palette.border)
                            } else {
                                rgb(0x256d3f)
                            })
                            .bg(rgb(palette.input))
                            .px_3()
                            .flex()
                            .items_center()
                            .font_family("JetBrains Mono")
                            .text_sm()
                            .text_color(if state.value.is_empty() {
                                rgb(palette.text_muted)
                            } else {
                                rgb(palette.text)
                            })
                            .child(truncate_preview(&input_display, 92)),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(if has_error {
                                rgb(0xfca5a5)
                            } else {
                                rgb(palette.text_muted)
                            })
                            .child(if has_error {
                                "Target path is required."
                            } else {
                                "Enter saves / Esc cancels."
                            }),
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
                                "transfer-move-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.close_transfer_move_dialog(cx);
                                }),
                            ))
                            .child(div().when(has_error, |this| this.opacity(0.45)).child(
                                small_button(
                                    palette,
                                    "transfer-move-save",
                                    "Save",
                                    cx.listener(|this, _, window, cx| {
                                        this.submit_transfer_move(window, cx);
                                    }),
                                ),
                            )),
                    ),
            )
    }
}
