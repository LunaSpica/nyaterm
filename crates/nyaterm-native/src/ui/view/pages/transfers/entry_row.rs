use super::*;

pub(super) fn transfer_browser_parent_entry_row(palette: crate::ui::theme::ThemePalette,
    current_path: String,
    column_widths: TransferBrowserColumnWidths,
    cx: &mut Context<NyaTermApp>,) -> impl IntoElement  {
    let palette = cx.entity().read(cx).theme_palette();
    let parent_path = remote_parent_path(&current_path);
    let context_path = current_path.clone();
    div()
        .id(SharedString::from("transfer-browser-entry-parent"))
        .flex()
        .gap_2()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(0x0f1724))
        .px_2()
        .py_2()
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
                .text_sm()
                .font_weight(FontWeight(700.))
                .text_color(rgb(palette.text))
                .child(transfer_entry_icon(true, false, false))
                .child(".."),
        )
        .child(transfer_browser_text_cell(palette, column_widths.modified, ""))
        .child(transfer_browser_text_cell(palette, column_widths.size, "-"))
        .child(transfer_browser_text_cell(palette, column_widths.permissions, ""))
        .child(transfer_browser_text_cell(palette, column_widths.owner, ""))
        .child(transfer_browser_text_cell(palette, column_widths.group, ""))
        .child(
            div()
                .w(TRANSFER_BROWSER_ACTIONS_WIDTH)
                .flex_none()
                .flex()
                .items_center()
                .gap_1()
                .child(small_button(palette, 
                    "transfer-open-parent-entry",
                    "Up",
                    cx.listener(|this, _, window, cx| {
                        this.open_transfer_parent_directory(window, cx);
                    }),
                ))
                .child(
                    div()
                        .min_w_0()
                        .font_family("JetBrains Mono")
                        .text_xs()
                        .text_color(rgb(palette.text_muted))
                        .child(truncate_preview(&parent_path, 32)),
                ),
        )
}

fn transfer_browser_text_cell(palette: crate::ui::theme::ThemePalette, width: gpui::Pixels, value: &'static str) -> impl IntoElement  {
    div()
        .w(width)
        .flex_none()
        .truncate()
        .text_xs()
        .text_color(rgb(palette.text_muted))
        .child(value)
}

pub(super) fn transfer_browser_entry_row(palette: crate::ui::theme::ThemePalette,
    entry: SftpFileEntry,
    selected_remote_path: Option<String>,
    selected_remote_paths: &HashSet<String>,
    column_widths: TransferBrowserColumnWidths,
    rename_state: Option<TransferRenameState>,
    rename_focus: gpui::FocusHandle,
    ai_actions: Vec<AiCustomActionConfig>,
    cx: &mut Context<NyaTermApp>,) -> impl IntoElement  {
    let palette = cx.entity().read(cx).theme_palette();
    let entry_path = entry.path.clone();
    let mouse_down_path = entry.path.clone();
    let mouse_move_path = entry.path.clone();
    let context_path = entry.path.clone();
    let mark_path = entry.path.clone();
    let open_path = entry.path.clone();
    let favorite_path = entry.path.clone();
    let download_path = entry.path.clone();
    let move_path = entry.path.clone();
    let delete_path = entry.path.clone();
    let default_open_entry = entry.clone();
    let first_ai_action = ai_actions.first().cloned();
    let first_ai_entry = entry.clone();
    let rename_path = entry.path.clone();
    let properties_entry = entry.clone();
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
    div()
        .id(SharedString::from(format!(
            "transfer-browser-entry-{entry_path}"
        )))
        .h(px(30.))
        .flex()
        .gap_1()
        .items_center()
        .border_b_1()
        .border_color(rgb(palette.surface_elevated))
        .bg(if is_selected {
            rgb(0x10251d)
        } else if is_marked {
            rgb(0x0f1f18)
        } else {
            rgb(palette.surface)
        })
        .px_2()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)))
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
                .text_size(px(12.))
                .font_weight(FontWeight(600.))
                .text_color(if is_selected {
                    rgb(0xdcfce7)
                } else {
                    rgb(palette.text)
                })
                .child(transfer_entry_icon(
                    is_directory,
                    entry.file_type == SftpFileType::Symlink,
                    is_selected || is_marked,
                ))
                .when(is_marked, |this| {
                    this.child(
                        div()
                            .text_size(px(9.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.success))
                            .child("●"),
                    )
                })
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
                            .font_family("JetBrains Mono")
                            .text_xs()
                            .text_color(if rename_value.is_empty() {
                                rgb(palette.text_muted)
                            } else {
                                rgb(palette.text)
                            })
                            .track_focus(&rename_focus)
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                cx.stop_propagation();
                                window.focus(&this.transfer_rename_focus);
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
                .truncate()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(format_sftp_modified(entry.modified_at)),
        )
        .child(
            div()
                .w(column_widths.size)
                .flex_none()
                .truncate()
                .text_right()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(format_file_size(entry.size)),
        )
        .child(
            div()
                .w(column_widths.permissions)
                .flex_none()
                .truncate()
                .text_xs()
                .font_family("JetBrains Mono")
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
                .truncate()
                .text_xs()
                .font_family("JetBrains Mono")
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
                .truncate()
                .text_xs()
                .font_family("JetBrains Mono")
                .text_color(rgb(palette.text_muted))
                .child(if entry.group.is_empty() {
                    "-".to_string()
                } else {
                    entry.group.clone()
                }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .w(TRANSFER_BROWSER_ACTIONS_WIDTH)
                .flex_none()
                .gap_1()
                .flex_wrap()
                .child(small_button(palette, 
                    format!("transfer-mark-entry-{mark_path}"),
                    if is_marked { "Marked" } else { "Mark" },
                    cx.listener(move |this, _, _, cx| {
                        this.toggle_transfer_browser_entry_marked(mark_path.clone(), cx);
                    }),
                ))
                .when(is_directory, |this| {
                    this.child(small_button(palette, 
                        format!("transfer-open-{open_path}"),
                        "Open",
                        cx.listener(move |this, _, window, cx| {
                            this.open_transfer_browser_directory(open_path.clone(), window, cx);
                        }),
                    ))
                    .child(small_button(palette, 
                        format!("transfer-favorite-entry-{favorite_path}"),
                        "Fav",
                        cx.listener(move |this, _, _, cx| {
                            this.add_transfer_browser_favorite_path(favorite_path.clone(), cx);
                        }),
                    ))
                })
                .when(!is_directory, |this| {
                    this.child(small_button(palette, 
                        format!("transfer-open-default-entry-{}", default_open_entry.path),
                        "Open",
                        cx.listener(move |this, _, window, cx| {
                            this.open_transfer_default(default_open_entry.clone(), window, cx);
                        }),
                    ))
                    .when(first_ai_action.is_some(), |this| {
                        let action = first_ai_action
                            .clone()
                            .expect("first AI action exists after is_some check");
                        this.child(small_button(palette, 
                            format!(
                                "transfer-ai-file-entry-{}-{}",
                                first_ai_entry.path, action.id
                            ),
                            "AI",
                            cx.listener(move |this, _, window, cx| {
                                this.start_transfer_file_ai_action(
                                    first_ai_entry.clone(),
                                    action.clone(),
                                    window,
                                    cx,
                                );
                            }),
                        ))
                    })
                })
                .child(small_button(palette, 
                    format!("transfer-download-entry-{download_path}"),
                    "DL",
                    cx.listener(move |this, _, window, cx| {
                        this.select_transfer_browser_entry_from_context(
                            download_path.clone(),
                            window,
                            cx,
                        );
                        this.start_selected_sftp_download_jobs(window, cx);
                    }),
                ))
                .child(small_button(palette, 
                    format!("transfer-move-entry-{move_path}"),
                    "Move",
                    cx.listener(move |this, _, window, cx| {
                        this.select_transfer_browser_entry(move_path.clone(), cx);
                        this.open_transfer_move_dialog(move_path.clone(), window, cx);
                    }),
                ))
                .child(small_button(palette, 
                    format!("transfer-delete-entry-{delete_path}"),
                    "Del",
                    cx.listener(move |this, _, window, cx| {
                        this.select_transfer_browser_entry_from_context(
                            delete_path.clone(),
                            window,
                            cx,
                        );
                        this.open_selected_transfer_delete_dialog(window, cx);
                    }),
                ))
                .child(small_button(palette, 
                    format!("transfer-rename-entry-{rename_path}"),
                    "Rename",
                    cx.listener(move |this, _, window, cx| {
                        this.select_transfer_browser_entry(rename_path.clone(), cx);
                        this.open_transfer_rename_dialog(window, cx);
                    }),
                ))
                .child(small_button(palette, 
                    format!("transfer-properties-entry-{}", properties_entry.path),
                    "Props",
                    cx.listener(move |this, _, window, cx| {
                        this.open_transfer_properties(properties_entry.clone(), window, cx);
                    }),
                )),
        )
}
