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
        let delete_title = if delete_count == 1 {
            self.tr("fileExplorer.sureDelete")
                .replace("{{name}}", &state.name)
        } else {
            self.tr("fileExplorer.sureDeleteMultiple")
                .replace("{{count}}", &delete_count.to_string())
        };
        let preview_items = state
            .paths
            .iter()
            .take(6)
            .map(|path| remote_file_name(path))
            .collect::<Vec<_>>();
        let remaining_items = delete_count.saturating_sub(preview_items.len());
        let dialog_width = transfer_dialog_width(self.last_viewport_size.0, 320.);

        div()
            .id(SharedString::from("transfer-delete-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(gpui::rgba(0x00000080))
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
                    .w(px(dialog_width))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(delete_title),
                    )
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("fileExplorer.deleteConfirmHint")),
                    )
                    .when(delete_count > 1, |this| {
                        let mut items = div()
                            .id(SharedString::from("transfer-delete-preview"))
                            .mt_3()
                            .max_h(px(160.))
                            .overflow_y_scroll()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .px_2()
                            .py_1()
                            .text_xs()
                            .text_color(rgb(palette.text_muted));
                        for item in preview_items {
                            items = items.child(
                                div()
                                    .py(px(2.))
                                    .font_family(crate::features::gpui_code_font_family())
                                    .child(truncate_preview(&item, 72)),
                            );
                        }
                        if remaining_items > 0 {
                            items = items.child(
                                div().pt_1().text_color(rgb(palette.text)).child(
                                    self.tr("fileExplorer.moreItems")
                                        .replace("{{count}}", &remaining_items.to_string()),
                                ),
                            );
                        }
                        this.child(items)
                    })
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "transfer-delete-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_transfer_delete_dialog(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "transfer-delete-confirm",
                                self.tr("fileExplorer.delete"),
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
        let input_display = if state.value.is_empty() {
            self.tr("fileExplorer.location").to_string()
        } else {
            state.value.clone()
        };
        let dialog_width = transfer_dialog_width(self.last_viewport_size.0, 384.);

        div()
            .id(SharedString::from("transfer-move-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(gpui::rgba(0x00000080))
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
                    .w(px(dialog_width))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(
                                self.tr("fileExplorer.moveTo")
                                    .replace("{{name}}", &truncate_preview(&state.name, 48)),
                            ),
                    )
                    .child(
                        div()
                            .id(SharedString::from("transfer-move-input"))
                            .mt_3()
                            .h(px(36.))
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.input))
                            .px_3()
                            .flex()
                            .items_center()
                            .font_family(crate::features::gpui_code_font_family())
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
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "transfer-move-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_transfer_move_dialog(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "transfer-move-save",
                                self.tr("common.save"),
                                cx.listener(|this, _, window, cx| {
                                    this.submit_transfer_move(window, cx);
                                }),
                            )),
                    ),
            )
    }
}
