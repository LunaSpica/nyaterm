use super::*;

impl NyaTermApp {
    pub(super) fn transfer_browser_view(
        &mut self,
        can_transfer: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let _selected = self
            .transfer_selected_remote_path
            .as_deref()
            .map(|path| truncate_preview(path, 56))
            .unwrap_or_else(|| "none".to_string());
        let visible_entries = self.visible_transfer_browser_entries();
        let column_widths = self.transfer_browser_column_widths;
        let table_width = transfer_browser_table_width(column_widths);
        let resizing_column = self
            .transfer_browser_column_resize
            .map(|state| state.column);
        let selected_entries = self.selected_transfer_entries();
        let selected_count = selected_entries.len();
        let total_count = self.transfer_browser_entries.len();
        let files_total_size: u64 = self
            .transfer_browser_entries
            .iter()
            .filter(|entry| entry.file_type != SftpFileType::Directory)
            .map(|entry| entry.size.unwrap_or(0))
            .sum();
        let search_active = !self.transfer_browser_search.trim().is_empty();
        let search_expanded = self.transfer_browser_search_expanded || search_active;
        let search_value = if self.transfer_browser_search.is_empty() {
            "Search remote files".to_string()
        } else {
            self.transfer_browser_search.clone()
        };
        let current_browser_path = normalized_transfer_browser_path(&self.transfer_browser_path);
        let auto_sync_cwd = self.transfer_browser_auto_sync_cwd_enabled();
        let mut rows = div().flex().flex_col();
        if can_transfer && current_browser_path != "/" && current_browser_path != "." {
            rows = rows.child(transfer_browser_parent_entry_row(
                current_browser_path.clone(),
                column_widths,
                cx,
            ));
        }
        if !can_transfer {
            rows = rows.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .px_4()
                    .py_8()
                    .gap_1()
                    .child(
                        svg()
                            .size(px(28.))
                            .flex_none()
                            .path("icons/conn/folder.svg")
                            .text_color(rgb(palette.border)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette.text_muted))
                            .child(if self.active_session_id.is_some() {
                                "Unsupported session"
                            } else {
                                "Connect to an SSH session"
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(if self.active_session_id.is_some() {
                                "File Explorer requires an active SSH session."
                            } else {
                                "Open an SSH connection to browse remote files."
                            }),
                    ),
            );
        } else if self.transfer_browser_entries.is_empty() {
            rows = rows.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .px_4()
                    .py_8()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette.text_muted))
                            .child("No remote entries loaded"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(truncate_preview(&self.transfer_browser_status, 64)),
                    ),
            );
        } else if visible_entries.is_empty() {
            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px_4()
                    .py_8()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child("No remote entries match the current search."),
            );
        } else {
            // Tauri File Explorer virtual list (30px rows, overscan, spacer padding).
            const FILE_ROW_PX: f32 = 30.;
            const FILE_VIEWPORT_ROWS: usize = 36;
            const FILE_OVERSCAN: usize = 8;
            let total_entries = visible_entries.len();
            let window_capacity = FILE_VIEWPORT_ROWS + FILE_OVERSCAN * 2;
            let max_offset = total_entries.saturating_sub(FILE_VIEWPORT_ROWS.min(total_entries));
            if self.transfer_browser_list_offset > max_offset {
                self.transfer_browser_list_offset = max_offset;
            }
            let scroll_row = self.transfer_browser_list_offset.min(max_offset);
            let window_start = scroll_row.saturating_sub(FILE_OVERSCAN);
            let window_end = (window_start + window_capacity).min(total_entries);
            let pad_top = (window_start as f32) * FILE_ROW_PX;
            let pad_bottom = ((total_entries.saturating_sub(window_end)) as f32) * FILE_ROW_PX;
            if pad_top > 0. {
                rows = rows.child(div().h(px(pad_top)).w_full().flex_none());
            }
            for entry in visible_entries
                .get(window_start..window_end)
                .unwrap_or(&[])
                .iter()
                .cloned()
            {
                let ai_actions = self.enabled_transfer_file_ai_actions_for_entry(&entry);
                rows = rows.child(transfer_browser_entry_row(
                    entry,
                    self.transfer_selected_remote_path.clone(),
                    &self.transfer_selected_remote_paths,
                    column_widths,
                    self.transfer_rename.clone(),
                    self.transfer_rename_focus.clone(),
                    ai_actions,
                    cx,
                ));
            }
            if pad_bottom > 0. {
                rows = rows.child(div().h(px(pad_bottom)).w_full().flex_none());
            }
        }

        div()
            .id(SharedString::from("transfer-browser-panel"))
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(palette.surface))
            .track_focus(&self.transfer_browser_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_transfer_browser_key_down(event, window, cx);
            }))
            .when(can_transfer, |this| {
                this.child(
                    div()
                        .relative()
                        .h(px(30.))
                        .px_1()
                        .border_b_1()
                        .border_color(rgb(palette.border))
                        .bg(rgb(palette.section_header))
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .child(compact_transfer_toolbar_button(
                            "transfer-browser-new-file",
                            "icons/fe/new-file.svg",
                            cx.listener(|this, _, window, cx| {
                                this.open_transfer_new_file_dialog(window, cx);
                            }),
                        ))
                        .child(compact_transfer_toolbar_button(
                            "transfer-browser-new-folder",
                            "icons/fe/new-folder.svg",
                            cx.listener(|this, _, window, cx| {
                                this.open_transfer_new_folder_dialog(window, cx);
                            }),
                        ))
                        .child(transfer_toolbar_divider())
                        .child(compact_transfer_upload_menu_button(cx))
                        .child(
                            div()
                                .when(selected_count == 0, |this| this.opacity(0.45))
                                .child(compact_transfer_toolbar_button(
                                    "transfer-browser-download-selected",
                                    "icons/fe/download.svg",
                                    cx.listener(|this, _, window, cx| {
                                        this.start_selected_sftp_download_jobs(window, cx);
                                    }),
                                )),
                        )
                        .child(
                            div()
                                .when(selected_count == 0, |this| this.opacity(0.45))
                                .child(compact_transfer_toolbar_button(
                                    "transfer-browser-delete-selected",
                                    "icons/fe/delete.svg",
                                    cx.listener(|this, _, window, cx| {
                                        this.open_selected_transfer_delete_dialog(window, cx);
                                    }),
                                )),
                        )
                        .child(transfer_toolbar_divider())
                        .child(compact_transfer_toolbar_button(
                            "transfer-browser-go-up",
                            "icons/fe/up.svg",
                            cx.listener(|this, _, window, cx| {
                                this.open_transfer_parent_directory(window, cx);
                            }),
                        ))
                        .child(compact_transfer_toolbar_button(
                            "transfer-browser-refresh",
                            "icons/fe/refresh.svg",
                            cx.listener(|this, _, window, cx| {
                                this.refresh_transfer_browser(window, cx);
                            }),
                        ))
                        .child(div().flex_1())
                        .child(compact_transfer_toolbar_button_active(
                            "transfer-browser-expand-search",
                            "icons/fe/search.svg",
                            search_active || search_expanded,
                            cx.listener(|this, _, window, cx| {
                                this.transfer_browser_search_expanded = true;
                                this.transfer_browser_status = "file search focused".to_string();
                                window.focus(&this.transfer_browser_search_focus);
                                cx.notify();
                            }),
                        ))
                        .when(search_expanded, |toolbar| {
                            toolbar.child(
                                div()
                                    .id(SharedString::from("transfer-browser-search-overlay"))
                                    .absolute()
                                    .top(px(2.))
                                    .bottom(px(2.))
                                    .left(px(4.))
                                    .right(px(4.))
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(0x388bfd))
                                    .bg(rgb(palette.section_header))
                                    .px_1()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        svg()
                                            .size(px(16.))
                                            .flex_none()
                                            .path("icons/fe/search.svg")
                                            .text_color(rgb(palette.accent)),
                                    )
                                    .child(
                                        div()
                                            .id(SharedString::from("transfer-browser-search"))
                                            .h_full()
                                            .flex_1()
                                            .min_w_0()
                                            .px_1()
                                            .flex()
                                            .items_center()
                                            .cursor_text()
                                            .track_focus(&self.transfer_browser_search_focus)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                window.focus(&this.transfer_browser_search_focus);
                                                cx.notify();
                                            }))
                                            .on_key_down(cx.listener(
                                                |this, event: &KeyDownEvent, _, cx| {
                                                    cx.stop_propagation();
                                                    this.handle_transfer_browser_search_key_down(
                                                        event, cx,
                                                    );
                                                },
                                            ))
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .flex_1()
                                                    .font_family("JetBrains Mono")
                                                    .text_size(px(12.))
                                                    .text_color(if search_active {
                                                        rgb(palette.text)
                                                    } else {
                                                        rgb(palette.text_dimmed)
                                                    })
                                                    .child(truncate_preview(&search_value, 96)),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id(SharedString::from(
                                                "transfer-browser-clear-search",
                                            ))
                                            .size(px(20.))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_sm()
                                            .text_size(px(12.))
                                            .text_color(rgb(palette.text_muted))
                                            .cursor_pointer()
                                            .hover(|this| {
                                                this.bg(rgb(palette.surface_elevated)).text_color(rgb(palette.text))
                                            })
                                            .child("×")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                if this.transfer_browser_search.is_empty() {
                                                    this.transfer_browser_search_expanded = false;
                                                    this.transfer_browser_status =
                                                        "file search closed".to_string();
                                                } else {
                                                    this.transfer_browser_search.clear();
                                                    this.transfer_browser_list_offset = 0;
                                                    this.transfer_browser_status =
                                                        "file search cleared".to_string();
                                                }
                                                cx.notify();
                                            })),
                                    ),
                            )
                        }),
                )
                .child(self.transfer_browser_path_row(current_browser_path.clone(), cx))
            })
                        .child(
                div()
                    .id(SharedString::from("transfer-browser-table-scroll"))
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_x_scroll()
                    .overflow_y_hidden()
                    .scrollbar_width(px(8.))
                    .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                        const FILE_ROW_PX: f32 = 30.;
                        const FILE_VIEWPORT_ROWS: usize = 36;
                        let total = this.visible_transfer_browser_entries().len();
                        let max_offset =
                            total.saturating_sub(FILE_VIEWPORT_ROWS.min(total));
                        if max_offset == 0 {
                            return;
                        }
                        let delta_rows = match event.delta {
                            ScrollDelta::Lines(delta) => delta.y,
                            ScrollDelta::Pixels(delta) => f32::from(delta.y) / FILE_ROW_PX,
                        };
                        let next = (this.transfer_browser_list_offset as f32 - delta_rows)
                            .round()
                            .clamp(0., max_offset as f32) as usize;
                        if next != this.transfer_browser_list_offset {
                            this.transfer_browser_list_offset = next;
                            cx.stop_propagation();
                            cx.notify();
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, event: &MouseDownEvent, window, cx| {
                            this.open_transfer_browser_current_context_menu(event, window, cx);
                        }),
                    )
                    .child(
                        div()
                            .min_w(table_width)
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_2()
                                    .child(sort_header_cell(
                                        TransferBrowserSortColumn::Name,
                                        column_widths.name,
                                        self.transfer_browser_sort_column,
                                        self.transfer_browser_sort_direction,
                                        resizing_column,
                                        cx,
                                    ))
                                    .child(sort_header_cell(
                                        TransferBrowserSortColumn::Modified,
                                        column_widths.modified,
                                        self.transfer_browser_sort_column,
                                        self.transfer_browser_sort_direction,
                                        resizing_column,
                                        cx,
                                    ))
                                    .child(sort_header_cell(
                                        TransferBrowserSortColumn::Size,
                                        column_widths.size,
                                        self.transfer_browser_sort_column,
                                        self.transfer_browser_sort_direction,
                                        resizing_column,
                                        cx,
                                    ))
                                    .child(sort_header_cell(
                                        TransferBrowserSortColumn::Permissions,
                                        column_widths.permissions,
                                        self.transfer_browser_sort_column,
                                        self.transfer_browser_sort_direction,
                                        resizing_column,
                                        cx,
                                    ))
                                    .child(sort_header_cell(
                                        TransferBrowserSortColumn::Owner,
                                        column_widths.owner,
                                        self.transfer_browser_sort_column,
                                        self.transfer_browser_sort_direction,
                                        resizing_column,
                                        cx,
                                    ))
                                    .child(sort_header_cell(
                                        TransferBrowserSortColumn::Group,
                                        column_widths.group,
                                        self.transfer_browser_sort_column,
                                        self.transfer_browser_sort_direction,
                                        resizing_column,
                                        cx,
                                    ))
                                    .child(
                                        div()
                                            .h(px(24.))
                                            .w(TRANSFER_BROWSER_ACTIONS_WIDTH)
                                            .flex_none()
                                            .flex()
                                            .items_center()
                                            .rounded_sm()
                                            .bg(rgb(palette.input))
                                            .px_2()
                                            .text_size(px(10.))
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(palette.text_muted))
                                            .child("ACTIONS"),
                                    ),
                            )
                            .child(rows),
                    ),
            )
            // Tauri FileExplorer footer: totals left, cwd sync / send icons right.
            .child(
                div()
                    .h(px(28.))
                    .flex_none()
                    .px_2()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_3()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .child(format!("{total_count} item(s)"))
                            .when(files_total_size > 0, |this| {
                                this.child(format_file_size(Some(files_total_size)))
                            })
                            .when(selected_count > 0, |this| {
                                this.child(format!("{selected_count} marked"))
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_0()
                            .child(compact_transfer_footer_button(
                                "transfer-browser-footer-sync-cwd",
                                "icons/fe/sync.svg",
                                cx.listener(|this, _, window, cx| {
                                    this.start_transfer_sync_cwd_job(window, cx);
                                }),
                            ))
                            .child(compact_transfer_footer_button_active(
                                "transfer-browser-footer-auto-sync",
                                "icons/fe/sync.svg",
                                auto_sync_cwd,
                                cx.listener(|this, _, window, cx| {
                                    this.toggle_transfer_browser_auto_sync_cwd(window, cx);
                                }),
                            ))
                            .child(compact_transfer_footer_button(
                                "transfer-browser-footer-send-path",
                                "icons/fe/paste.svg",
                                cx.listener(|this, _, _, cx| {
                                    this.send_current_transfer_browser_path_to_terminal(cx);
                                }),
                            ))
                            .when(self.transfer_selected_remote_path.is_some(), |this| {
                                this.child(compact_transfer_footer_button(
                                    "transfer-footer-copy-path",
                                    "icons/fe/paste.svg",
                                    cx.listener(|this, _, _, cx| {
                                        this.copy_selected_transfer_path(TransferPathPart::Full, cx);
                                    }),
                                ))
                                .child(compact_transfer_footer_button(
                                    "transfer-footer-props",
                                    "icons/fe/search.svg",
                                    cx.listener(|this, _, window, cx| {
                                        this.open_selected_transfer_properties(window, cx);
                                    }),
                                ))
                            }),
                    ),
            )
    }
}



fn compact_transfer_footer_button(
    id: impl Into<String>,
    icon_path: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let palette = crate::ui::theme::theme_palette("github-dark");
    // Tauri footer icons: h-6 w-6 (24px)
    div()
        .id(SharedString::from(id.into()))
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)).text_color(rgb(palette.text)))
        .child(
            svg()
                .size(px(14.))
                .flex_none()
                .path(icon_path),
        )
        .on_click(on_click)
}

fn compact_transfer_footer_button_active(
    id: impl Into<String>,
    icon_path: &'static str,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let palette = crate::ui::theme::theme_palette("github-dark");
    let color = if active { rgb(palette.accent) } else { rgb(palette.text_muted) };
    div()
        .id(SharedString::from(id.into()))
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .bg(if active { rgb(palette.hover) } else { rgb(palette.surface) })
        .text_color(color)
        .cursor_pointer()
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(if active { rgb(0x79b8ff) } else { rgb(palette.text) })
        })
        .child(
            svg()
                .size(px(14.))
                .flex_none()
                .path(icon_path),
        )
        .on_click(on_click)
}

fn compact_transfer_upload_menu_button(cx: &mut Context<NyaTermApp>) -> impl IntoElement {
        let palette = cx.entity().read(cx).theme_palette();
    // Tauri: single Upload icon opens DropdownMenu (Upload Files / Upload Folder).
    div()
        .id(SharedString::from("transfer-browser-upload"))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)).text_color(rgb(palette.text)))
        .child(
            svg()
                .size(px(16.))
                .flex_none()
                .path("icons/fe/upload.svg"),
        )
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, event: &MouseDownEvent, _, cx| {
                this.open_transfer_browser_upload_menu(event, cx);
            }),
        )
}

fn compact_transfer_toolbar_button(
    id: impl Into<String>,
    icon_path: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let palette = crate::ui::theme::theme_palette("github-dark");
    // Tauri FileExplorerToolbar: h-7 ghost icon buttons, muted until hover.
    div()
        .id(SharedString::from(id.into()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)).text_color(rgb(palette.text)))
        .child(
            svg()
                .size(px(16.))
                .flex_none()
                .path(icon_path),
        )
        .on_click(on_click)
}

fn compact_transfer_toolbar_button_active(
    id: impl Into<String>,
    icon_path: &'static str,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let palette = crate::ui::theme::theme_palette("github-dark");
    let color = if active { rgb(palette.accent) } else { rgb(palette.text_muted) };
    div()
        .id(SharedString::from(id.into()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .bg(if active { rgb(palette.hover) } else { rgb(palette.surface) })
        .text_color(color)
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)).text_color(if active { rgb(0x79b8ff) } else { rgb(palette.text) }))
        .child(
            svg()
                .size(px(16.))
                .flex_none()
                .path(icon_path),
        )
        .on_click(on_click)
}

fn transfer_dynamic_toolbar_button(
    id: impl Into<String>,
    label: impl Into<String>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .max_w(px(116.))
        .px_3()
        .flex()
        .items_center()
        .overflow_hidden()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x303848))
        .bg(rgb(0x151b27))
        .text_color(rgb(0xdbeafe))
        .text_xs()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x223047)))
        .child(label.into())
        .on_click(on_click)
}

fn transfer_toolbar_divider() -> impl IntoElement {
    div()
        .h(px(16.))
        .w(px(1.))
        .mx_1()
        .rounded_sm()
        .bg(rgb(0x303848))
}
