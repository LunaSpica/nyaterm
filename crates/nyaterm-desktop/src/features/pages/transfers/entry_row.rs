use super::*;

pub(super) fn transfer_browser_parent_entry_row(
    palette: crate::theme::ThemePalette,
    current_path: String,
    column_widths: TransferBrowserColumnWidths,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let context_path = current_path.clone();
    div()
        .id(SharedString::from("transfer-browser-entry-parent"))
        .h(px(30.))
        .flex()
        .items_center()
        .rounded_sm()
        .bg(gpui::rgba(0x00000000))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)))
        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
            this.open_transfer_parent_directory(window, cx);
        }))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                this.open_transfer_browser_parent_context_menu(
                    context_path.clone(),
                    event,
                    window,
                    cx,
                );
            }),
        )
        .child(
            div()
                .min_w_0()
                .w(column_widths.name)
                .flex_none()
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .text_size(px(12.))
                .text_color(rgb(palette.text))
                .child(transfer_entry_icon(true, false, false))
                .child(".."),
        )
        .child(transfer_browser_text_cell(
            palette,
            column_widths.modified,
            "",
        ))
        .child(transfer_browser_text_cell(palette, column_widths.size, "-"))
        .child(transfer_browser_text_cell(
            palette,
            column_widths.permissions,
            "",
        ))
        .child(transfer_browser_text_cell(palette, column_widths.owner, ""))
        .child(transfer_browser_text_cell(palette, column_widths.group, ""))
}

fn transfer_browser_text_cell(
    palette: crate::theme::ThemePalette,
    width: gpui::Pixels,
    value: &'static str,
) -> impl IntoElement {
    div()
        .w(width)
        .flex_none()
        .px_2()
        .truncate()
        .text_xs()
        .text_color(rgb(palette.text_muted))
        .child(value)
}

pub(super) fn transfer_browser_entry_row(
    palette: crate::theme::ThemePalette,
    entry: SftpFileEntry,
    selected_remote_path: Option<String>,
    selected_remote_paths: &HashSet<String>,
    column_widths: TransferBrowserColumnWidths,
    rename_state: Option<TransferRenameState>,
    rename_focus: gpui::FocusHandle,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let entry_path = entry.path.clone();
    let mouse_down_path = entry.path.clone();
    let mouse_move_path = entry.path.clone();
    let context_path = entry.path.clone();
    let is_selected = selected_remote_path.as_deref() == Some(entry.path.as_str());
    let is_marked = selected_remote_paths.contains(&entry.path);
    let inline_rename = rename_state.filter(|state| state.old_path == entry.path);
    let is_renaming = inline_rename.is_some();
    let was_single_selected_on_mouse_down = is_selected
        && selected_remote_paths.len() == 1
        && selected_remote_paths.contains(&entry.path);
    let name_click_path = entry.path.clone();
    let rename_value = inline_rename
        .as_ref()
        .map(|state| state.value.clone())
        .unwrap_or_default();
    let rename_display = if rename_value.is_empty() {
        "Remote name".to_string()
    } else {
        format!("{rename_value}|")
    };
    let rename_has_error = inline_rename.as_ref().is_some_and(|state| {
        let trimmed = state.value.trim();
        trimmed.is_empty() || trimmed.contains('/') || trimmed == "." || trimmed == ".."
    });
    let is_directory = entry.file_type == SftpFileType::Directory;
    let is_marked_or_selected = is_selected || is_marked;
    let size_display = if is_directory {
        "-".to_string()
    } else {
        format_file_size(entry.size)
    };
    div()
        .id(SharedString::from(format!(
            "transfer-browser-entry-{entry_path}"
        )))
        .h(px(30.))
        .flex()
        .items_center()
        .bg(if is_marked_or_selected {
            gpui::rgba((palette.primary << 8) | 0x1a)
        } else {
            gpui::rgba(0x00000000)
        })
        .cursor_pointer()
        .when(!is_marked_or_selected, |this| {
            this.hover(|this| this.bg(rgb(palette.hover)))
        })
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                if !is_renaming {
                    this.handle_transfer_browser_entry_mouse_down(
                        mouse_down_path.clone(),
                        event,
                        window,
                        cx,
                    );
                }
            }),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                if !is_renaming {
                    this.open_transfer_browser_context_menu(
                        context_path.clone(),
                        event,
                        window,
                        cx,
                    );
                }
            }),
        )
        .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
            if !is_renaming {
                this.handle_transfer_browser_entry_mouse_move(mouse_move_path.clone(), event, cx);
            }
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, event: &MouseUpEvent, _, cx| {
                this.finish_transfer_browser_selection_drag(event, cx);
            }),
        )
        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            this.select_transfer_browser_entry_from_click(entry_path.clone(), event, window, cx);
        }))
        .child(
            div()
                .min_w_0()
                .w(column_widths.name)
                .flex_none()
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .text_size(px(12.))
                .text_color(if is_marked_or_selected {
                    rgb(palette.primary)
                } else {
                    rgb(palette.text)
                })
                .child(transfer_entry_icon(
                    is_directory,
                    entry.file_type == SftpFileType::Symlink,
                    is_marked_or_selected,
                ))
                .when(is_renaming, |this| {
                    this.child(
                        div()
                            .id(SharedString::from(format!(
                                "transfer-inline-rename-{}",
                                entry.path
                            )))
                            .h(px(28.))
                            .min_w_0()
                            .flex_1()
                            .rounded_sm()
                            .border_1()
                            .border_color(if rename_has_error {
                                rgb(0x7f1d1d)
                            } else {
                                rgb(0x256d3f)
                            })
                            .bg(rgb(palette.input))
                            .px_2()
                            .flex()
                            .items_center()
                            .font_family(crate::features::gpui_code_font_family())
                            .text_xs()
                            .text_color(if rename_value.is_empty() {
                                rgb(palette.text_muted)
                            } else {
                                rgb(palette.text)
                            })
                            .track_focus(&rename_focus)
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                cx.stop_propagation();
                                window.focus(&this.transfer.file_ops.rename_focus);
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                                cx.stop_propagation();
                                this.handle_transfer_rename_key_down(event, window, cx);
                            }))
                            .child(truncate_preview(&rename_display, 42)),
                    )
                })
                .when(!is_renaming, |this| {
                    this.child(
                        div()
                            .id(SharedString::from(format!(
                                "transfer-browser-entry-name-{name_click_path}"
                            )))
                            .min_w_0()
                            .flex_1()
                            .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                                this.schedule_transfer_browser_name_rename(
                                    name_click_path.clone(),
                                    was_single_selected_on_mouse_down,
                                    event,
                                    cx,
                                );
                            }))
                            .truncate()
                            .child(truncate_preview(&entry.name, 42)),
                    )
                }),
        )
        .child(
            div()
                .w(column_widths.modified)
                .flex_none()
                .px_2()
                .truncate()
                .text_xs()
                .font_family(crate::features::gpui_code_font_family())
                .text_color(rgb(palette.text_muted))
                .child(format_sftp_modified(entry.modified_at)),
        )
        .child(
            div()
                .w(column_widths.size)
                .flex_none()
                .px_2()
                .truncate()
                .text_right()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(size_display),
        )
        .child(
            div()
                .w(column_widths.permissions)
                .flex_none()
                .px_2()
                .truncate()
                .text_xs()
                .font_family(crate::features::gpui_code_font_family())
                .text_color(rgb(palette.text_muted))
                .child(
                    entry
                        .permissions
                        .map(format_permissions_octal)
                        .unwrap_or_else(|| "-".to_string()),
                ),
        )
        .child(
            div()
                .w(column_widths.owner)
                .flex_none()
                .px_2()
                .truncate()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(if entry.owner.is_empty() {
                    "-".to_string()
                } else {
                    entry.owner.clone()
                }),
        )
        .child(
            div()
                .w(column_widths.group)
                .flex_none()
                .px_2()
                .truncate()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(if entry.group.is_empty() {
                    "-".to_string()
                } else {
                    entry.group.clone()
                }),
        )
}
