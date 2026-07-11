use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn transfer_unknown_file_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let state = self
            .transfer_unknown_file
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

        div()
            .id(SharedString::from("transfer-unknown-file-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer_unknown_file_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer_unknown_file_focus);
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
                    .w(px(460.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .on_click(|_, _, cx| {
                        cx.stop_propagation();
                    })
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
                                    .text_color(rgb(0xe5edf7))
                                    .child("Unknown File Type"),
                            )
                            .child(status_pill("choose", rgb(0x93c5fd), rgb(0x17253b))),
                    )
                    .child(div().text_xs().text_color(rgb(0xaeb7c8)).child(
                        "This file type is not known as text or binary. Choose how to open it.",
                    ))
                    .child(
                        div()
                            .rounded_sm()
                            .bg(rgb(0x10151e))
                            .p_3()
                            .font_family("JetBrains Mono")
                            .text_xs()
                            .text_color(rgb(0xe5edf7))
                            .child(truncate_preview(&name, 72)),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(small_button(palette, 
                                "transfer-unknown-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_transfer_unknown_file(cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "transfer-unknown-internal",
                                "Open Internal",
                                cx.listener(|this, _, window, cx| {
                                    this.open_unknown_transfer_file_internal(window, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "transfer-unknown-external",
                                "Open External",
                                cx.listener(|this, _, window, cx| {
                                    this.open_unknown_transfer_file_external(window, cx);
                                }),
                            )),
                    ),
            )
    }
}
