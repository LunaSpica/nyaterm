use super::*;

impl NyaTermApp {
    pub(in crate::features) fn transfer_properties_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let state = self
            .transfer_properties
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
        let permissions = properties
            .as_ref()
            .and_then(|properties| properties.permissions)
            .or(entry.permissions)
            .map(format_permissions_octal)
            .unwrap_or_else(|| "-".to_string());
        let symbolic = properties
            .as_ref()
            .map(|properties| properties.permissions_symbolic.clone())
            .or_else(|| {
                entry
                    .permissions
                    .map(|mode| format_permissions_symbolic(entry.file_type, mode))
            })
            .unwrap_or_else(|| "-".to_string());
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
        let dialog_width = (self.last_viewport_size.0 - 32.).min(460.).max(280.);
        let dialog_max_height = (self.last_viewport_size.1 * 0.75).clamp(320., 720.);

        div()
            .id(SharedString::from("transfer-properties-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer_properties_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer_properties_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_transfer_properties_key_down(event, window, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("transfer-properties-dialog"))
                    .w(px(dialog_width))
                    .max_h(px(dialog_max_height))
                    .overflow_y_scroll()
                    .scrollbar_width(px(8.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
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
                            )
                            .child(status_pill(
                                entry_type_label,
                                rgb(0x93c5fd),
                                rgb(palette.hover),
                            )),
                    )
                    .child(property_section_heading(
                        palette,
                        self.tr("fileExplorer.general"),
                    ))
                    .child(
                        div()
                            .mt_4()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x202633))
                            .bg(rgb(palette.input))
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(property_row(
                                palette,
                                self.tr("fileExplorer.name"),
                                truncate_preview(&entry.name, 76),
                            ))
                            .child(property_row(
                                palette,
                                self.tr("fileExplorer.type"),
                                entry_type_label,
                            ))
                            .child(property_row(
                                palette,
                                self.tr("fileExplorer.path"),
                                truncate_preview(&location, 76),
                            ))
                            .child(property_row(
                                palette,
                                self.tr("fileExplorer.location"),
                                truncate_preview(&entry.path, 82),
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
                            .child(property_row(palette, self.tr("fileExplorer.owner"), owner))
                            .child(property_row(palette, self.tr("fileExplorer.group"), group))
                            .child(property_row(
                                palette,
                                self.tr("fileExplorer.octal"),
                                permissions,
                            ))
                            .child(property_row(
                                palette,
                                self.tr("fileExplorer.permissions"),
                                symbolic,
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(if loading {
                                self.tr("fileExplorer.loading")
                            } else {
                                ""
                            }),
                    )
                    .child(property_section_heading(
                        palette,
                        self.tr("fileExplorer.ownership"),
                    ))
                    .child(
                        div()
                            .mt_4()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x202633))
                            .bg(rgb(palette.input))
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(property_input_row(
                                palette,
                                "transfer-properties-owner-input",
                                self.tr("fileExplorer.owner"),
                                &state.owner_value,
                                state.focused_field == TransferPropertiesField::Owner,
                                loading || state.saving,
                                cx.listener(|this, _, window, cx| {
                                    if let Some(state) = this.transfer_properties.as_mut() {
                                        state.focused_field = TransferPropertiesField::Owner;
                                    }
                                    window.focus(&this.transfer_properties_focus);
                                    cx.notify();
                                }),
                            ))
                            .child(property_input_row(
                                palette,
                                "transfer-properties-group-input",
                                self.tr("fileExplorer.group"),
                                &state.group_value,
                                state.focused_field == TransferPropertiesField::Group,
                                loading || state.saving,
                                cx.listener(|this, _, window, cx| {
                                    if let Some(state) = this.transfer_properties.as_mut() {
                                        state.focused_field = TransferPropertiesField::Group;
                                    }
                                    window.focus(&this.transfer_properties_focus);
                                    cx.notify();
                                }),
                            ))
                            .child(property_section_heading(
                                palette,
                                self.tr("fileExplorer.permissions"),
                            ))
                            .child(property_input_row(
                                palette,
                                "transfer-properties-mode-input",
                                self.tr("fileExplorer.octal"),
                                &state.mode_value,
                                state.focused_field == TransferPropertiesField::Mode,
                                loading || state.saving,
                                cx.listener(|this, _, window, cx| {
                                    if let Some(state) = this.transfer_properties.as_mut() {
                                        state.focused_field = TransferPropertiesField::Mode;
                                    }
                                    window.focus(&this.transfer_properties_focus);
                                    cx.notify();
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
                                                if let Some(state) =
                                                    this.transfer_properties.as_mut()
                                                {
                                                    state.recursive = !state.recursive;
                                                }
                                                cx.notify();
                                            }),
                                        )),
                                )
                            }),
                    )
                    .when_some(state.error.clone(), |this, error| {
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
                                "transfer-properties-save",
                                if state.saving {
                                    self.tr("common.saving")
                                } else {
                                    self.tr("common.save")
                                },
                                cx.listener(move |this, _, window, cx| {
                                    if can_save {
                                        this.submit_transfer_properties(window, cx);
                                    }
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "transfer-properties-close",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_transfer_properties(cx);
                                }),
                            )),
                    ),
            )
    }
}
