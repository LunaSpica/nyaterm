use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn transfer_new_folder_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let state = self
            .transfer_new_folder
            .clone()
            .unwrap_or(TransferNewFolderState {
                parent_path: String::new(),
                value: String::new(),
                mode: 0o755,
                open_after_create: false,
            });
        let name = state.value.trim();
        let has_error = !valid_remote_child_name(name);
        let input_display = if state.value.is_empty() {
            "Folder name".to_string()
        } else {
            state.value.clone()
        };

        div()
            .id(SharedString::from("transfer-new-folder-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer_new_folder_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer_new_folder_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_transfer_new_folder_key_down(event, window, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("transfer-new-folder-dialog"))
                    .w(px(420.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0xe5edf7))
                            .child("New Remote Folder"),
                    )
                    .child(
                        div()
                            .mt_2()
                            .font_family("JetBrains Mono")
                            .text_xs()
                            .text_color(rgb(0x8f98aa))
                            .child(truncate_preview(&state.parent_path, 92)),
                    )
                    .child(
                        div()
                            .id(SharedString::from("transfer-new-folder-input"))
                            .mt_3()
                            .h(px(36.))
                            .rounded_sm()
                            .border_1()
                            .border_color(if has_error {
                                rgb(0x7f1d1d)
                            } else {
                                rgb(0x256d3f)
                            })
                            .bg(rgb(0x0d1320))
                            .px_3()
                            .flex()
                            .items_center()
                            .font_family("JetBrains Mono")
                            .text_sm()
                            .text_color(if state.value.is_empty() {
                                rgb(0x64748b)
                            } else {
                                rgb(0xe5edf7)
                            })
                            .child(truncate_preview(&input_display, 80)),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(small_button(
                                        "transfer-new-folder-mode-755",
                                        "0755",
                                        cx.listener(|this, _, _, cx| {
                                            if let Some(state) = this.transfer_new_folder.as_mut() {
                                                state.mode = 0o755;
                                            }
                                            cx.notify();
                                        }),
                                    ))
                                    .child(small_button(
                                        "transfer-new-folder-mode-700",
                                        "0700",
                                        cx.listener(|this, _, _, cx| {
                                            if let Some(state) = this.transfer_new_folder.as_mut() {
                                                state.mode = 0o700;
                                            }
                                            cx.notify();
                                        }),
                                    ))
                                    .child(status_pill(
                                        if state.mode == 0o700 { "0700" } else { "0755" },
                                        rgb(0x93c5fd),
                                        rgb(0x17253b),
                                    )),
                            )
                            .child(small_button(
                                "transfer-new-folder-open-after",
                                if state.open_after_create {
                                    "Open: On"
                                } else {
                                    "Open: Off"
                                },
                                cx.listener(|this, _, _, cx| {
                                    if let Some(state) = this.transfer_new_folder.as_mut() {
                                        state.open_after_create = !state.open_after_create;
                                    }
                                    cx.notify();
                                }),
                            )),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(if has_error {
                                rgb(0xfca5a5)
                            } else {
                                rgb(0x8f98aa)
                            })
                            .child(if has_error {
                                "Use a non-empty single folder name."
                            } else {
                                "Enter creates / Esc cancels."
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
                                "transfer-new-folder-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.close_transfer_new_folder_dialog(cx);
                                }),
                            ))
                            .child(div().when(has_error, |this| this.opacity(0.45)).child(
                                small_button(
                                    "transfer-new-folder-create",
                                    "Create",
                                    cx.listener(|this, _, window, cx| {
                                        this.submit_transfer_new_folder(window, cx);
                                    }),
                                ),
                            )),
                    ),
            )
    }

    pub(in crate::ui::view) fn transfer_new_file_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let state = self
            .transfer_new_file
            .clone()
            .unwrap_or(TransferNewFileState {
                parent_path: String::new(),
                value: String::new(),
                mode: 0o644,
            });
        let name = state.value.trim();
        let has_error = !valid_remote_child_name(name);
        let input_display = if state.value.is_empty() {
            "File name".to_string()
        } else {
            state.value.clone()
        };

        div()
            .id(SharedString::from("transfer-new-file-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer_new_file_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer_new_file_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_transfer_new_file_key_down(event, window, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("transfer-new-file-dialog"))
                    .w(px(420.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0xe5edf7))
                            .child("New Remote File"),
                    )
                    .child(
                        div()
                            .mt_2()
                            .font_family("JetBrains Mono")
                            .text_xs()
                            .text_color(rgb(0x8f98aa))
                            .child(truncate_preview(&state.parent_path, 92)),
                    )
                    .child(
                        div()
                            .id(SharedString::from("transfer-new-file-input"))
                            .mt_3()
                            .h(px(36.))
                            .rounded_sm()
                            .border_1()
                            .border_color(if has_error {
                                rgb(0x7f1d1d)
                            } else {
                                rgb(0x256d3f)
                            })
                            .bg(rgb(0x0d1320))
                            .px_3()
                            .flex()
                            .items_center()
                            .font_family("JetBrains Mono")
                            .text_sm()
                            .text_color(if state.value.is_empty() {
                                rgb(0x64748b)
                            } else {
                                rgb(0xe5edf7)
                            })
                            .child(truncate_preview(&input_display, 80)),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(small_button(
                                        "transfer-new-file-mode-644",
                                        "0644",
                                        cx.listener(|this, _, _, cx| {
                                            if let Some(state) = this.transfer_new_file.as_mut() {
                                                state.mode = 0o644;
                                            }
                                            cx.notify();
                                        }),
                                    ))
                                    .child(small_button(
                                        "transfer-new-file-mode-600",
                                        "0600",
                                        cx.listener(|this, _, _, cx| {
                                            if let Some(state) = this.transfer_new_file.as_mut() {
                                                state.mode = 0o600;
                                            }
                                            cx.notify();
                                        }),
                                    ))
                                    .child(status_pill(
                                        if state.mode == 0o600 { "0600" } else { "0644" },
                                        rgb(0x93c5fd),
                                        rgb(0x17253b),
                                    )),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x8f98aa))
                                    .child("Creates an empty remote file."),
                            ),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(if has_error {
                                rgb(0xfca5a5)
                            } else {
                                rgb(0x8f98aa)
                            })
                            .child(if has_error {
                                "Use a non-empty single file name."
                            } else {
                                "Enter creates / Esc cancels."
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
                                "transfer-new-file-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.close_transfer_new_file_dialog(cx);
                                }),
                            ))
                            .child(div().when(has_error, |this| this.opacity(0.45)).child(
                                small_button(
                                    "transfer-new-file-create",
                                    "Create",
                                    cx.listener(|this, _, window, cx| {
                                        this.submit_transfer_new_file(window, cx);
                                    }),
                                ),
                            )),
                    ),
            )
    }

    pub(in crate::ui::view) fn transfer_new_symlink_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let state = self
            .transfer_new_symlink
            .clone()
            .unwrap_or(TransferNewSymlinkState {
                parent_path: String::new(),
                name: String::new(),
                target: String::new(),
                focused_field: TransferSymlinkField::Name,
            });
        let name = state.name.trim();
        let target = state.target.trim();
        let has_error = !valid_remote_child_name(name) || target.is_empty();

        div()
            .id(SharedString::from("transfer-new-symlink-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer_new_symlink_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer_new_symlink_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_transfer_new_symlink_key_down(event, window, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("transfer-new-symlink-dialog"))
                    .w(px(460.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0xe5edf7))
                            .child("New Remote Symlink"),
                    )
                    .child(
                        div()
                            .mt_2()
                            .font_family("JetBrains Mono")
                            .text_xs()
                            .text_color(rgb(0x8f98aa))
                            .child(truncate_preview(&state.parent_path, 92)),
                    )
                    .child(symlink_input_row(
                        "transfer-new-symlink-name",
                        "Name",
                        if state.name.is_empty() {
                            "Symlink name"
                        } else {
                            &state.name
                        },
                        state.focused_field == TransferSymlinkField::Name,
                        has_error && !valid_remote_child_name(name),
                        cx.listener(|this, _, window, cx| {
                            if let Some(state) = this.transfer_new_symlink.as_mut() {
                                state.focused_field = TransferSymlinkField::Name;
                            }
                            window.focus(&this.transfer_new_symlink_focus);
                            cx.notify();
                        }),
                    ))
                    .child(symlink_input_row(
                        "transfer-new-symlink-target",
                        "Target",
                        if state.target.is_empty() {
                            "/path/to/target"
                        } else {
                            &state.target
                        },
                        state.focused_field == TransferSymlinkField::Target,
                        has_error && target.is_empty(),
                        cx.listener(|this, _, window, cx| {
                            if let Some(state) = this.transfer_new_symlink.as_mut() {
                                state.focused_field = TransferSymlinkField::Target;
                            }
                            window.focus(&this.transfer_new_symlink_focus);
                            cx.notify();
                        }),
                    ))
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(if has_error {
                                rgb(0xfca5a5)
                            } else {
                                rgb(0x8f98aa)
                            })
                            .child(if has_error {
                                "Name and target are required; name must not contain '/'."
                            } else {
                                "Tab switches fields / Enter creates / Esc cancels."
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
                                "transfer-new-symlink-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.close_transfer_new_symlink_dialog(cx);
                                }),
                            ))
                            .child(div().when(has_error, |this| this.opacity(0.45)).child(
                                small_button(
                                    "transfer-new-symlink-create",
                                    "Create",
                                    cx.listener(|this, _, window, cx| {
                                        this.submit_transfer_new_symlink(window, cx);
                                    }),
                                ),
                            )),
                    ),
            )
    }
}
