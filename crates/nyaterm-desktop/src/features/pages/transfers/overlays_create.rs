use super::*;

impl NyaTermApp {
    pub(in crate::features) fn transfer_new_folder_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
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
            self.tr("fileExplorer.newFolderName").to_string()
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
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("fileExplorer.newFolder")),
                    )
                    .child(
                        div()
                            .mt_2()
                            .font_family(crate::features::gpui_code_font_family())
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
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
                                rgb(palette.danger)
                            } else {
                                rgb(0x256d3f)
                            })
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
                                        palette,
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
                                        palette,
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
                                        rgb(palette.link),
                                        rgb(palette.hover),
                                    )),
                            )
                            .child(small_button(
                                palette,
                                "transfer-new-folder-open-after",
                                if state.open_after_create {
                                    self.tr("fileExplorer.on")
                                } else {
                                    self.tr("fileExplorer.off")
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
                                rgb(palette.danger)
                            } else {
                                rgb(palette.text_muted)
                            })
                            .child(if has_error {
                                self.tr("fileExplorer.invalidFolderName")
                            } else {
                                ""
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
                                "transfer-new-folder-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_transfer_new_folder_dialog(cx);
                                }),
                            ))
                            .child(div().when(has_error, |this| this.opacity(0.45)).child(
                                small_button(
                                    palette,
                                    "transfer-new-folder-create",
                                    self.tr("common.confirm"),
                                    cx.listener(|this, _, window, cx| {
                                        this.submit_transfer_new_folder(window, cx);
                                    }),
                                ),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn transfer_new_file_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
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
            self.tr("fileExplorer.newFileName").to_string()
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
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("fileExplorer.newFile")),
                    )
                    .child(
                        div()
                            .mt_2()
                            .font_family(crate::features::gpui_code_font_family())
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
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
                                rgb(palette.danger)
                            } else {
                                rgb(0x256d3f)
                            })
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
                                        palette,
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
                                        palette,
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
                                        rgb(palette.link),
                                        rgb(palette.hover),
                                    )),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text_muted))
                                    .child(self.tr("fileExplorer.file")),
                            ),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(if has_error {
                                rgb(palette.danger)
                            } else {
                                rgb(palette.text_muted)
                            })
                            .child(if has_error {
                                self.tr("fileExplorer.invalidFileName")
                            } else {
                                ""
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
                                "transfer-new-file-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_transfer_new_file_dialog(cx);
                                }),
                            ))
                            .child(div().when(has_error, |this| this.opacity(0.45)).child(
                                small_button(
                                    palette,
                                    "transfer-new-file-create",
                                    self.tr("common.confirm"),
                                    cx.listener(|this, _, window, cx| {
                                        this.submit_transfer_new_file(window, cx);
                                    }),
                                ),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn transfer_new_symlink_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
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
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("fileExplorer.newSymlink")),
                    )
                    .child(
                        div()
                            .mt_2()
                            .font_family(crate::features::gpui_code_font_family())
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(truncate_preview(&state.parent_path, 92)),
                    )
                    .child(symlink_input_row(
                        palette,
                        "transfer-new-symlink-name",
                        self.tr("fileExplorer.symlinkName"),
                        if state.name.is_empty() {
                            self.tr("fileExplorer.symlinkName")
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
                        palette,
                        "transfer-new-symlink-target",
                        self.tr("fileExplorer.symlinkTarget"),
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
                                rgb(palette.danger)
                            } else {
                                rgb(palette.text_muted)
                            })
                            .child(if has_error {
                                self.tr("fileExplorer.invalidSymlink")
                            } else {
                                ""
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
                                "transfer-new-symlink-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_transfer_new_symlink_dialog(cx);
                                }),
                            ))
                            .child(div().when(has_error, |this| this.opacity(0.45)).child(
                                small_button(
                                    palette,
                                    "transfer-new-symlink-create",
                                    self.tr("common.confirm"),
                                    cx.listener(|this, _, window, cx| {
                                        this.submit_transfer_new_symlink(window, cx);
                                    }),
                                ),
                            )),
                    ),
            )
    }
}
