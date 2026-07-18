use super::*;

impl NyaTermApp {
    pub(in crate::features) fn transfer_new_folder_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let dialog_width = transfer_dialog_width(self.last_viewport_size.0, 500.);
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
        let has_error = !name.is_empty() && !valid_remote_child_name(name);
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
                    .w(px(dialog_width))
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
                    .child(self.transfer_new_item_permission_grid(palette, state.mode, true, cx))
                    .child(
                        div()
                            .mt_2()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text_muted))
                                    .child(self.tr("fileExplorer.openAfterCreateFolder")),
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
                            .child(
                                div()
                                    .when(has_error || name.is_empty(), |this| this.opacity(0.45))
                                    .child(small_button(
                                        palette,
                                        "transfer-new-folder-create",
                                        self.tr("common.confirm"),
                                        cx.listener(|this, _, window, cx| {
                                            this.submit_transfer_new_folder(window, cx);
                                        }),
                                    )),
                            ),
                    ),
            )
    }

    pub(in crate::features) fn transfer_new_file_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let dialog_width = transfer_dialog_width(self.last_viewport_size.0, 500.);
        let state = self
            .transfer_new_file
            .clone()
            .unwrap_or(TransferNewFileState {
                parent_path: String::new(),
                value: String::new(),
                mode: 0o644,
                open_after_create: false,
            });
        let name = state.value.trim();
        let has_error = !name.is_empty() && !valid_remote_child_name(name);
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
                    .w(px(dialog_width))
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
                    .child(self.transfer_new_item_permission_grid(palette, state.mode, false, cx))
                    .child(
                        div()
                            .mt_2()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text_muted))
                                    .child(self.tr("fileExplorer.openAfterCreateFile")),
                            )
                            .child(small_button(
                                palette,
                                "transfer-new-file-open-after",
                                if state.open_after_create {
                                    self.tr("fileExplorer.on")
                                } else {
                                    self.tr("fileExplorer.off")
                                },
                                cx.listener(|this, _, _, cx| {
                                    if let Some(state) = this.transfer_new_file.as_mut() {
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
                            .child(
                                div()
                                    .when(has_error || name.is_empty(), |this| this.opacity(0.45))
                                    .child(small_button(
                                        palette,
                                        "transfer-new-file-create",
                                        self.tr("common.confirm"),
                                        cx.listener(|this, _, window, cx| {
                                            this.submit_transfer_new_file(window, cx);
                                        }),
                                    )),
                            ),
                    ),
            )
    }

    pub(in crate::features) fn transfer_new_symlink_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let dialog_width = transfer_dialog_width(self.last_viewport_size.0, 480.);
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
                    .w(px(dialog_width))
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

    fn transfer_new_item_permission_grid(
        &self,
        palette: crate::theme::ThemePalette,
        mode: u32,
        folder: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let bit = |mask: u32| mode & mask != 0;
        let toggle = |id: &str, label: &str, mask: u32| {
            permission_toggle(
                palette,
                id.to_string(),
                label.to_string(),
                bit(mask),
                cx.listener(move |this, _, _, cx| {
                    this.toggle_transfer_new_item_mode_bit(folder, mask, cx);
                }),
            )
        };

        div()
            .mt_3()
            .rounded_sm()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.input))
            .p_2()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_xs()
                    .text_color(rgb(palette.text_dimmed))
                    .child(div().w(px(48.)).flex_none().child(""))
                    .child(div().w(px(42.)).text_center().child("R"))
                    .child(div().w(px(42.)).text_center().child("W"))
                    .child(div().w(px(42.)).text_center().child("X"))
                    .child(
                        div()
                            .w(px(72.))
                            .text_center()
                            .child(self.tr("fileExplorer.special")),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .w(px(48.))
                            .flex_none()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("fileExplorer.permUser")),
                    )
                    .child(toggle("transfer-perm-user-r", "R", 0o400))
                    .child(toggle("transfer-perm-user-w", "W", 0o200))
                    .child(toggle("transfer-perm-user-x", "X", 0o100))
                    .child(toggle("transfer-perm-user-special", "UID", 0o4000)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .w(px(48.))
                            .flex_none()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("fileExplorer.permGroup")),
                    )
                    .child(toggle("transfer-perm-group-r", "R", 0o040))
                    .child(toggle("transfer-perm-group-w", "W", 0o020))
                    .child(toggle("transfer-perm-group-x", "X", 0o010))
                    .child(toggle("transfer-perm-group-special", "GID", 0o2000)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .w(px(48.))
                            .flex_none()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("fileExplorer.permOther")),
                    )
                    .child(toggle("transfer-perm-other-r", "R", 0o004))
                    .child(toggle("transfer-perm-other-w", "W", 0o002))
                    .child(toggle("transfer-perm-other-x", "X", 0o001))
                    .child(toggle(
                        "transfer-perm-other-special",
                        self.tr("fileExplorer.permSticky"),
                        0o1000,
                    )),
            )
            .child(
                div()
                    .mt_1()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .child(self.tr("fileExplorer.octal"))
                    .child(
                        div()
                            .font_family(crate::features::gpui_code_font_family())
                            .text_color(rgb(palette.text))
                            .child(format_permissions_octal(mode)),
                    ),
            )
    }

    fn toggle_transfer_new_item_mode_bit(
        &mut self,
        folder: bool,
        bit: u32,
        cx: &mut Context<Self>,
    ) {
        if folder {
            if let Some(state) = self.transfer_new_folder.as_mut() {
                state.mode ^= bit;
            }
        } else if let Some(state) = self.transfer_new_file.as_mut() {
            state.mode ^= bit;
        }
        cx.notify();
    }
}

fn permission_toggle(
    palette: crate::theme::ThemePalette,
    id: String,
    label: String,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id))
        .w(px(42.))
        .h(px(26.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(if active {
            rgb(palette.link)
        } else {
            rgb(palette.border)
        })
        .bg(if active {
            rgb(palette.hover)
        } else {
            rgb(palette.surface_elevated)
        })
        .text_size(px(10.))
        .text_color(if active {
            rgb(palette.link)
        } else {
            rgb(palette.text_muted)
        })
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)))
        .child(if active { "✓" } else { "·" })
        .child(label)
        .on_click(on_click)
}

fn transfer_dialog_width(viewport_width: f32, preferred_width: f32) -> f32 {
    preferred_width.min((viewport_width - 32.).max(240.))
}

#[cfg(test)]
mod tests {
    use super::transfer_dialog_width;

    #[test]
    fn dialog_width_uses_preferred_size_with_narrow_viewport_fallback() {
        assert_eq!(transfer_dialog_width(1280., 500.), 500.);
        assert_eq!(transfer_dialog_width(420., 500.), 388.);
        assert_eq!(transfer_dialog_width(200., 500.), 240.);
    }
}
