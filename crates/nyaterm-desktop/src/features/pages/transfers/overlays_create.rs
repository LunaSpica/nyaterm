use gpui::{
    App, ClickEvent, Context, FontWeight, InteractiveElement as _, IntoElement, KeyDownEvent,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, px, rgb, rgba, svg,
};

use crate::features::{NyaTermApp, TextInputSetup, dialog_action_button, gpui_code_font_family};
use crate::models::{
    TransferNewFileState, TransferNewFolderState, TransferNewSymlinkState,
    TransferPermissionTarget, TransferSymlinkField,
};
use crate::theme::ThemePalette;
use crate::widgets::small_button;

use super::{
    format_permissions_octal, parse_transfer_mode, symlink_input_row, transfer_dialog_width,
    valid_remote_child_name,
};

impl NyaTermApp {
    pub(in crate::features) fn transfer_new_folder_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let dialog_width = transfer_dialog_width(self.last_viewport_size.0, 500.);
        let state = self
            .transfer
            .file_ops
            .new_folder
            .clone()
            .unwrap_or(TransferNewFolderState {
                parent_path: String::new(),
                value: String::new(),
                mode: 0o755,
                open_after_create: false,
            });
        let name = state.value.trim();
        let has_error = !name.is_empty() && !valid_remote_child_name(name);
        let name_input = self
            .text_input_box(
                "transfer.new-folder.name",
                &state.value,
                TextInputSetup::placeholder(self.tr("fileExplorer.newFolderName")),
                cx,
            )
            .into_any_element();

        div()
            .id(SharedString::from("transfer-new-folder-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer.file_ops.new_folder_focus)
            // No blanket focus grab: click follows mouse-down, so it would take
            // focus straight back off the box the pointer landed on.
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
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex_none()
                            .px_5()
                            .py_3()
                            .border_b_1()
                            .border_color(rgb(palette.border))
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("fileExplorer.newFolder")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .p_5()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .w(px(64.))
                                            .flex_none()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child(self.tr("fileExplorer.newFolderName")),
                                    )
                                    .child(div().flex_1().min_w_0().child(name_input)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .gap_3()
                                    .child(
                                        div()
                                            .w(px(64.))
                                            .flex_none()
                                            .mt_1()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child(self.tr("fileExplorer.permissions")),
                                    )
                                    .child(div().flex_1().min_w_0().child(
                                        self.transfer_permission_grid(
                                            palette,
                                            state.mode,
                                            TransferPermissionTarget::NewFolder,
                                            cx,
                                        ),
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .flex_none()
                            .border_t_1()
                            .border_color(rgb(palette.border))
                            .px_5()
                            .py_3()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .id("transfer-new-folder-open-after")
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(state) =
                                            this.transfer.file_ops.new_folder.as_mut()
                                        {
                                            state.open_after_create = !state.open_after_create;
                                        }
                                        cx.notify();
                                    }))
                                    .child(
                                        div()
                                            .size(px(14.))
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(if state.open_after_create {
                                                rgb(palette.link)
                                            } else {
                                                rgb(palette.border)
                                            })
                                            .bg(if state.open_after_create {
                                                rgb(palette.link)
                                            } else {
                                                rgb(palette.input)
                                            })
                                            .when(state.open_after_create, |this| {
                                                this.child(
                                                    svg()
                                                        .size(px(11.))
                                                        .path("icons/check.svg")
                                                        .text_color(rgb(palette.bg)),
                                                )
                                            }),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child(self.tr("fileExplorer.openAfterCreateFolder")),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
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
                                            .when(has_error || name.is_empty(), |this| {
                                                this.opacity(0.45)
                                            })
                                            .child(dialog_action_button(
                                                palette,
                                                "transfer-new-folder-create",
                                                self.tr("common.confirm"),
                                                false,
                                                cx.listener(|this, _, window, cx| {
                                                    this.submit_transfer_new_folder(window, cx);
                                                }),
                                            )),
                                    ),
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
            .transfer
            .file_ops
            .new_file
            .clone()
            .unwrap_or(TransferNewFileState {
                parent_path: String::new(),
                value: String::new(),
                mode: 0o644,
                open_after_create: false,
            });
        let name = state.value.trim();
        let has_error = !name.is_empty() && !valid_remote_child_name(name);
        let name_input = self
            .text_input_box(
                "transfer.new-file.name",
                &state.value,
                TextInputSetup::placeholder(self.tr("fileExplorer.newFileName")),
                cx,
            )
            .into_any_element();

        div()
            .id(SharedString::from("transfer-new-file-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer.file_ops.new_file_focus)
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
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex_none()
                            .px_5()
                            .py_3()
                            .border_b_1()
                            .border_color(rgb(palette.border))
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("fileExplorer.newFile")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .p_5()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .w(px(64.))
                                            .flex_none()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child(self.tr("fileExplorer.newFileName")),
                                    )
                                    .child(div().flex_1().min_w_0().child(name_input)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .gap_3()
                                    .child(
                                        div()
                                            .w(px(64.))
                                            .flex_none()
                                            .mt_1()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child(self.tr("fileExplorer.permissions")),
                                    )
                                    .child(div().flex_1().min_w_0().child(
                                        self.transfer_permission_grid(
                                            palette,
                                            state.mode,
                                            TransferPermissionTarget::NewFile,
                                            cx,
                                        ),
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .flex_none()
                            .border_t_1()
                            .border_color(rgb(palette.border))
                            .px_5()
                            .py_3()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .id("transfer-new-file-open-after")
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(state) =
                                            this.transfer.file_ops.new_file.as_mut()
                                        {
                                            state.open_after_create = !state.open_after_create;
                                        }
                                        cx.notify();
                                    }))
                                    .child(
                                        div()
                                            .size(px(14.))
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(if state.open_after_create {
                                                rgb(palette.link)
                                            } else {
                                                rgb(palette.border)
                                            })
                                            .bg(if state.open_after_create {
                                                rgb(palette.link)
                                            } else {
                                                rgb(palette.input)
                                            })
                                            .when(state.open_after_create, |this| {
                                                this.child(
                                                    svg()
                                                        .size(px(11.))
                                                        .path("icons/check.svg")
                                                        .text_color(rgb(palette.bg)),
                                                )
                                            }),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child(self.tr("fileExplorer.openAfterCreateFile")),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
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
                                            .when(has_error || name.is_empty(), |this| {
                                                this.opacity(0.45)
                                            })
                                            .child(dialog_action_button(
                                                palette,
                                                "transfer-new-file-create",
                                                self.tr("common.confirm"),
                                                false,
                                                cx.listener(|this, _, window, cx| {
                                                    this.submit_transfer_new_file(window, cx);
                                                }),
                                            )),
                                    ),
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
            .transfer
            .file_ops
            .new_symlink
            .clone()
            .unwrap_or(TransferNewSymlinkState {
                parent_path: String::new(),
                name: String::new(),
                target: String::new(),
                focused_field: TransferSymlinkField::Name,
            });
        let name = state.name.trim();
        let target = state.target.trim();
        let name_invalid = !name.is_empty() && !valid_remote_child_name(name);
        let has_error = name.is_empty() || target.is_empty() || name_invalid;
        let symlink_name_input = self
            .text_input_box(
                "transfer.new-symlink.name",
                &state.name,
                TextInputSetup::placeholder(self.tr("fileExplorer.symlinkName")),
                cx,
            )
            .into_any_element();
        let symlink_target_input = self
            .text_input_box(
                "transfer.new-symlink.target",
                &state.target,
                TextInputSetup::placeholder("/path/to/target"),
                cx,
            )
            .into_any_element();

        div()
            .id(SharedString::from("transfer-new-symlink-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer.file_ops.new_symlink_focus)
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
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex_none()
                            .px_5()
                            .py_3()
                            .border_b_1()
                            .border_color(rgb(palette.border))
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("fileExplorer.newSymlink")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .p_5()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(symlink_input_row(
                                palette,
                                self.tr("fileExplorer.symlinkName"),
                                name_invalid,
                                symlink_name_input,
                            ))
                            .child(symlink_input_row(
                                palette,
                                self.tr("fileExplorer.symlinkTarget"),
                                false,
                                symlink_target_input,
                            )),
                    )
                    .child(
                        div()
                            .flex_none()
                            .border_t_1()
                            .border_color(rgb(palette.border))
                            .px_5()
                            .py_3()
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
                                dialog_action_button(
                                    palette,
                                    "transfer-new-symlink-create",
                                    self.tr("common.confirm"),
                                    false,
                                    cx.listener(|this, _, window, cx| {
                                        this.submit_transfer_new_symlink(window, cx);
                                    }),
                                ),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn transfer_permission_grid(
        &self,
        palette: ThemePalette,
        mode: u32,
        target: TransferPermissionTarget,
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
                    this.toggle_transfer_permission_bit(target, mask, cx);
                }),
            )
        };

        div()
            .flex()
            .flex_col()
            .gap_2()
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
                    .child(toggle("transfer-perm-user-r", "", 0o400))
                    .child(toggle("transfer-perm-user-w", "", 0o200))
                    .child(toggle("transfer-perm-user-x", "", 0o100))
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
                    .child(toggle("transfer-perm-group-r", "", 0o040))
                    .child(toggle("transfer-perm-group-w", "", 0o020))
                    .child(toggle("transfer-perm-group-x", "", 0o010))
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
                    .child(toggle("transfer-perm-other-r", "", 0o004))
                    .child(toggle("transfer-perm-other-w", "", 0o002))
                    .child(toggle("transfer-perm-other-x", "", 0o001))
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
                            .font_family(gpui_code_font_family())
                            .text_color(rgb(palette.text))
                            .child(format_permissions_octal(mode)),
                    ),
            )
    }

    fn toggle_transfer_permission_bit(
        &mut self,
        target: TransferPermissionTarget,
        bit: u32,
        cx: &mut Context<Self>,
    ) {
        match target {
            TransferPermissionTarget::NewFolder => {
                if let Some(state) = self.transfer.file_ops.new_folder.as_mut() {
                    state.mode ^= bit;
                }
            }
            TransferPermissionTarget::NewFile => {
                if let Some(state) = self.transfer.file_ops.new_file.as_mut() {
                    state.mode ^= bit;
                }
            }
            TransferPermissionTarget::Properties => {
                if let Some(state) = self.transfer.file_ops.properties.as_mut() {
                    let current = parse_transfer_mode(&state.mode_value)
                        .or(state.entry.permissions)
                        .unwrap_or(0o644);
                    state.mode_value = format_permissions_octal(current ^ bit);
                }
                self.sync_transfer_properties_inputs(cx);
            }
        }
        cx.notify();
    }
}

fn permission_toggle(
    palette: ThemePalette,
    id: String,
    label: String,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let show_label = !label.is_empty();
    div()
        .id(SharedString::from(id))
        .w(px(42.))
        .h(px(26.))
        .flex_none()
        .flex()
        .items_center()
        .justify_start()
        .when(!show_label, |this| this.justify_center())
        .gap_1()
        .text_size(px(11.))
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| this.text_color(rgb(palette.text)))
        .child(
            div()
                .size(px(14.))
                .flex_none()
                .rounded_sm()
                .border_1()
                .border_color(if active {
                    rgb(palette.link)
                } else {
                    rgb(palette.border)
                })
                .bg(if active {
                    rgb(palette.link)
                } else {
                    rgb(palette.input)
                })
                .flex()
                .items_center()
                .justify_center()
                .when(active, |this| {
                    this.child(
                        svg()
                            .size(px(11.))
                            .path("icons/check.svg")
                            .text_color(rgb(palette.bg)),
                    )
                }),
        )
        .when(show_label, |this| this.child(label))
        .on_click(on_click)
}
