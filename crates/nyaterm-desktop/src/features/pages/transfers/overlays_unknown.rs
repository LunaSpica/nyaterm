use super::*;

impl NyaTermApp {
    pub(in crate::features) fn transfer_unknown_file_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let state =
            self.transfer
                .file_ops
                .unknown_file
                .clone()
                .unwrap_or(TransferUnknownFileState {
                    entry: SftpFileEntry {
                        name: String::new(),
                        path: String::new(),
                        file_type: SftpFileType::File,
                        size: None,
                        permissions: None,
                        owner: String::new(),
                        group: String::new(),
                        modified_at: None,
                    },
                });
        let name = if state.entry.name.trim().is_empty() {
            remote_file_name(&state.entry.path)
        } else {
            state.entry.name.clone()
        };
        let dialog_width = transfer_dialog_width(self.last_viewport_size.0, 512.);
        let description = self
            .tr("fileExplorer.unknownFileTypeDesc")
            .replace("{{name}}", &name);

        div()
            .id(SharedString::from("transfer-unknown-file-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(gpui::rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer.file_ops.unknown_file_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer.file_ops.unknown_file_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                match event.keystroke.key.as_str() {
                    "escape" => this.cancel_transfer_unknown_file(cx),
                    "enter" => this.open_unknown_transfer_file_external(window, cx),
                    _ => {}
                }
            }))
            .child(
                div()
                    .id(SharedString::from("transfer-unknown-file-dialog"))
                    .w(px(dialog_width))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .on_click(|_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(
                        div().flex().items_center().justify_between().gap_3().child(
                            div()
                                .text_size(px(18.))
                                .font_weight(FontWeight(800.))
                                .text_color(rgb(palette.text))
                                .child(self.tr("fileExplorer.unknownFileTypeTitle")),
                        ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(palette.text_muted))
                            .child(description),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "transfer-unknown-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_transfer_unknown_file(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "transfer-unknown-internal",
                                self.tr("fileExplorer.unknownFileTypeOpenInternal"),
                                cx.listener(|this, _, window, cx| {
                                    this.open_unknown_transfer_file_internal(window, cx);
                                }),
                            ))
                            .child(dialog_action_button(
                                palette,
                                "transfer-unknown-external",
                                self.tr("fileExplorer.unknownFileTypeOpenExternal"),
                                false,
                                cx.listener(|this, _, window, cx| {
                                    this.open_unknown_transfer_file_external(window, cx);
                                }),
                            )),
                    ),
            )
    }
}
