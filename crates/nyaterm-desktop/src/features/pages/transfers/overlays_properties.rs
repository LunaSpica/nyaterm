use gpui::{
    Context, FontWeight, InteractiveElement as _, IntoElement, KeyDownEvent, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, div, prelude::FluentBuilder as _,
    px, rgb, rgba,
};
use nyaterm_core::truncate_preview;
use nyaterm_transport::{SftpFileEntry, SftpFileType};

use crate::features::{NyaTermApp, TextInputSetup, dialog_action_button, format_file_size};
use crate::models::{TransferPermissionTarget, TransferPropertiesField, TransferPropertiesState};
use crate::widgets::small_button;

use super::{
    format_owner_group, format_sftp_modified, parse_transfer_mode, property_input_row,
    property_row, property_section_heading, remote_parent_path,
};

impl NyaTermApp {
    pub(in crate::features) fn transfer_properties_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let state = self
            .transfer
            .file_ops
            .properties
            .clone()
            .unwrap_or_else(|| TransferPropertiesState {
                session_id: None,
                entry: SftpFileEntry {
                    name: String::new(),
                    path: String::new(),
                    file_type: SftpFileType::Other,
                    size: None,
                    permissions: None,
                    owner: String::new(),
                    group: String::new(),
                    modified_at: None,
                },
                properties: None,
                mode_value: String::new(),
                owner_value: String::new(),
                group_value: String::new(),
                recursive: false,
                saving: false,
                error: None,
                focused_field: TransferPropertiesField::Mode,
            });
        let entry = state.entry.clone();
        let entry_type_label = match entry.file_type {
            SftpFileType::Directory => self.tr("fileExplorer.folder"),
            SftpFileType::File => self.tr("fileExplorer.file"),
            SftpFileType::Symlink => self.tr("fileExplorer.newSymlink"),
            SftpFileType::Other => self.tr("fileExplorer.special"),
        };
        let properties = state.properties.clone();
        let location = remote_parent_path(&entry.path);
        let size = properties
            .as_ref()
            .and_then(|properties| properties.size)
            .or(entry.size);
        let modified_at = properties
            .as_ref()
            .and_then(|properties| properties.modified_at)
            .or(entry.modified_at);
        let accessed_at = properties
            .as_ref()
            .and_then(|properties| properties.accessed_at);
        let owner = properties
            .as_ref()
            .map(|properties| format_owner_group(&properties.owner, properties.uid))
            .unwrap_or_else(|| "-".to_string());
        let group = properties
            .as_ref()
            .map(|properties| format_owner_group(&properties.group, properties.gid))
            .unwrap_or_else(|| "-".to_string());
        let loading = properties.is_none();
        let can_save = !loading && !state.saving;
        let property_mode = parse_transfer_mode(&state.mode_value)
            .or(entry.permissions)
            .unwrap_or(0o644);
        let dialog_width = (self.last_viewport_size.0 - 32.).min(460.).max(280.);
        let dialog_max_height = (self.last_viewport_size.1 * 0.75).clamp(320., 720.);
        let owner_input = self
            .text_input_box(
                "transfer.properties.owner",
                &state.owner_value,
                TextInputSetup::placeholder(self.tr("fileExplorer.owner")),
                cx,
            )
            .into_any_element();
        let group_input = self
            .text_input_box(
                "transfer.properties.group",
                &state.group_value,
                TextInputSetup::placeholder(self.tr("fileExplorer.group")),
                cx,
            )
            .into_any_element();
        let mode_input = self
            .text_input_box(
                "transfer.properties.mode",
                &state.mode_value,
                TextInputSetup::placeholder("0644"),
                cx,
            )
            .into_any_element();

        div()
            .id(SharedString::from("transfer-properties-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer.file_ops.properties_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_transfer_properties_key_down(event, window, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("transfer-properties-dialog"))
                    .w(px(dialog_width))
                    .max_h(px(dialog_max_height))
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
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .min_w_0()
                                    .text_sm()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text))
                                    .child(
                                        self.tr("fileExplorer.propertiesOf").replace(
                                            "{{name}}",
                                            &truncate_preview(&entry.name, 42),
                                        ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id(SharedString::from("transfer-properties-body"))
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .scrollbar_width(px(8.))
                            .p(px(20.))
                            .when(!loading, |this| {
                                this.child(property_section_heading(
                                    palette,
                                    self.tr("fileExplorer.general"),
                                ))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(10.))
                                        .child(property_row(
                                            palette,
                                            self.tr("fileExplorer.type"),
                                            entry_type_label,
                                        ))
                                        .child(property_row(
                                            palette,
                                            self.tr("fileExplorer.location"),
                                            truncate_preview(&location, 76),
                                        ))
                                        .child(property_row(
                                            palette,
                                            self.tr("fileExplorer.size"),
                                            format_file_size(size),
                                        ))
                                        .child(property_row(
                                            palette,
                                            self.tr("fileExplorer.mtime"),
                                            format_sftp_modified(modified_at),
                                        ))
                                        .child(property_row(
                                            palette,
                                            self.tr("fileExplorer.atime"),
                                            format_sftp_modified(accessed_at),
                                        ))
                                        .child(property_row(
                                            palette,
                                            self.tr("fileExplorer.owner"),
                                            owner,
                                        ))
                                        .child(property_row(
                                            palette,
                                            self.tr("fileExplorer.group"),
                                            group,
                                        )),
                                )
                                .child(
                                    div()
                                        .mt_5()
                                        .pt_5()
                                        .border_t_1()
                                        .border_color(rgb(palette.border))
                                        .child(property_section_heading(
                                            palette,
                                            self.tr("fileExplorer.ownership"),
                                        )),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_3()
                                        .child(property_input_row(
                                            palette,
                                            "transfer-properties-owner-row",
                                            self.tr("fileExplorer.owner"),
                                            owner_input,
                                            loading || state.saving,
                                            cx.listener(|this, _, window, cx| {
                                                this.focus_transfer_properties_field(
                                                    TransferPropertiesField::Owner,
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(property_input_row(
                                            palette,
                                            "transfer-properties-group-row",
                                            self.tr("fileExplorer.group"),
                                            group_input,
                                            loading || state.saving,
                                            cx.listener(|this, _, window, cx| {
                                                this.focus_transfer_properties_field(
                                                    TransferPropertiesField::Group,
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(
                                            div()
                                                .mt_1()
                                                .pt_4()
                                                .border_t_1()
                                                .border_color(rgb(palette.border))
                                                .child(property_section_heading(
                                                    palette,
                                                    self.tr("fileExplorer.permissions"),
                                                )),
                                        )
                                        .child(self.transfer_permission_grid(
                                            palette,
                                            property_mode,
                                            TransferPermissionTarget::Properties,
                                            cx,
                                        ))
                                        .child(property_input_row(
                                            palette,
                                            "transfer-properties-mode-row",
                                            self.tr("fileExplorer.octal"),
                                            mode_input,
                                            loading || state.saving,
                                            cx.listener(|this, _, window, cx| {
                                                this.focus_transfer_properties_field(
                                                    TransferPropertiesField::Mode,
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .when(entry.file_type == SftpFileType::Directory, |this| {
                                            this.child(
                                                div()
                                                    .mt_1()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .gap_3()
                                                    .text_xs()
                                                    .text_color(rgb(0xaeb7c8))
                                                    .child(self.tr("fileExplorer.applyRecursively"))
                                                    .child(small_button(
                                                        palette,
                                                        "transfer-properties-recursive-toggle",
                                                        if state.recursive {
                                                            self.tr("fileExplorer.on")
                                                        } else {
                                                            self.tr("fileExplorer.off")
                                                        },
                                                        cx.listener(|this, _, _, cx| {
                                                            if let Some(state) = this
                                                                .transfer
                                                                .file_ops
                                                                .properties
                                                                .as_mut()
                                                            {
                                                                state.recursive = !state.recursive;
                                                            }
                                                            cx.notify();
                                                        }),
                                                    )),
                                            )
                                        }),
                                )
                                .when_some(
                                    state.error.clone(),
                                    |this, error| {
                                        this.child(
                                            div()
                                                .mt_3()
                                                .rounded_sm()
                                                .bg(rgb(0x351216))
                                                .px_3()
                                                .py_2()
                                                .text_xs()
                                                .text_color(rgb(0xfca5a5))
                                                .child(error),
                                        )
                                    },
                                )
                            })
                            .when(loading, |this| {
                                this.child(if let Some(error) = state.error.clone() {
                                    div()
                                        .min_h(px(250.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .px_4()
                                        .text_xs()
                                        .text_color(rgb(palette.danger))
                                        .child(error)
                                } else {
                                    div()
                                        .min_h(px(250.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(self.tr("fileExplorer.loading"))
                                })
                            }),
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
                                "transfer-properties-close",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_transfer_properties(cx);
                                }),
                            ))
                            .child(dialog_action_button(
                                palette,
                                "transfer-properties-save",
                                if state.saving {
                                    self.tr("common.saving")
                                } else {
                                    self.tr("common.save")
                                },
                                false,
                                cx.listener(move |this, _, window, cx| {
                                    if can_save {
                                        this.submit_transfer_properties(window, cx);
                                    }
                                }),
                            )),
                    ),
            )
    }
}
