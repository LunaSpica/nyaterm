use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn transfer_properties_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let state = self
            .transfer_properties
            .clone()
            .unwrap_or_else(|| TransferPropertiesState {
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
                    .w(px(500.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0b0f16))
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
                                    .text_color(rgb(0xe5edf7))
                                    .child(format!(
                                        "Properties: {}",
                                        truncate_preview(&entry.name, 42)
                                    )),
                            )
                            .child(status_pill(
                                entry_kind_label(entry.file_type),
                                rgb(0x93c5fd),
                                rgb(0x17253b),
                            )),
                    )
                    .child(
                        div()
                            .mt_4()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x202633))
                            .bg(rgb(0x10151e))
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(property_row("Name", truncate_preview(&entry.name, 76)))
                            .child(property_row("Type", entry_kind_label(entry.file_type)))
                            .child(property_row("Location", truncate_preview(&location, 76)))
                            .child(property_row("Path", truncate_preview(&entry.path, 82)))
                            .child(property_row("Size", format_file_size(size)))
                            .child(property_row("Modified", format_sftp_modified(modified_at)))
                            .child(property_row("Accessed", format_sftp_modified(accessed_at)))
                            .child(property_row("Owner", owner))
                            .child(property_row("Group", group))
                            .child(property_row("Permissions", permissions))
                            .child(property_row("Mode", symbolic)),
                    )
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .text_color(rgb(0x8f98aa))
                            .child(if loading {
                                "Loading full remote metadata..."
                            } else {
                                "Tab switches fields / Enter saves / Esc cancels."
                            }),
                    )
                    .child(
                        div()
                            .mt_4()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x202633))
                            .bg(rgb(0x10151e))
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(property_input_row(
                                "transfer-properties-mode-input",
                                "Mode",
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
                            .child(property_input_row(
                                "transfer-properties-owner-input",
                                "Owner",
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
                                "transfer-properties-group-input",
                                "Group",
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
                                        .child("Apply recursively")
                                        .child(small_button(palette, 
                                            "transfer-properties-recursive-toggle",
                                            if state.recursive { "On" } else { "Off" },
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
                            .child(small_button(palette, 
                                "transfer-properties-copy-path",
                                "Copy Path",
                                {
                                    let path = entry.path.clone();
                                    cx.listener(move |this, _, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            path.clone(),
                                        ));
                                        this.terminal_status = "copied remote path".to_string();
                                        cx.notify();
                                    })
                                },
                            ))
                            .child(small_button(palette, 
                                "transfer-properties-save",
                                if state.saving { "Saving" } else { "Save" },
                                cx.listener(move |this, _, window, cx| {
                                    if can_save {
                                        this.submit_transfer_properties(window, cx);
                                    }
                                }),
                            ))
                            .child(small_button(palette, 
                                "transfer-properties-close",
                                "Close",
                                cx.listener(|this, _, _, cx| {
                                    this.close_transfer_properties(cx);
                                }),
                            )),
                    ),
            )
    }
}
