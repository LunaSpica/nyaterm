use super::*;

impl NyaTermApp {
    pub(super) fn transfer_browser_view(
        &mut self,
        can_transfer: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self
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
        let selected_single_file = selected_entries
            .first()
            .filter(|_| selected_count == 1)
            .is_some_and(|entry| entry.file_type != SftpFileType::Directory);
        let selected_ai_actions = selected_entries
            .first()
            .filter(|_| selected_count == 1)
            .map(|entry| self.enabled_transfer_file_ai_actions_for_entry(entry))
            .unwrap_or_default();
        let visible_count = visible_entries.len();
        let total_count = self.transfer_browser_entries.len();
        let search_active = !self.transfer_browser_search.trim().is_empty();
        let search_expanded = self.transfer_browser_search_expanded || search_active;
        let search_value = if self.transfer_browser_search.is_empty() {
            "Search remote files".to_string()
        } else {
            self.transfer_browser_search.clone()
        };
        let can_go_back =
            self.transfer_browser_history_index + 1 < self.transfer_browser_history.len();
        let can_go_forward = self.transfer_browser_history_index > 0;
        let current_browser_path = normalized_transfer_browser_path(&self.transfer_browser_path);
        let is_current_favorite = self
            .transfer_browser_favorites
            .iter()
            .any(|path| path == &current_browser_path);
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
                        div()
                            .text_size(px(18.))
                            .text_color(rgb(0x484f58))
                            .child("📁"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(0x8b949e))
                            .child(if self.active_session_id.is_some() {
                                "Unsupported session"
                            } else {
                                "Connect to an SSH session"
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x6e7681))
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
                            .text_color(rgb(0x8b949e))
                            .child("No remote entries loaded"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x6e7681))
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
                    .text_color(rgb(0x6e7681))
                    .child("No remote entries match the current search."),
            );
        } else {
            for entry in visible_entries.into_iter() {
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
        }

        div()
            .id(SharedString::from("transfer-browser-panel"))
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(0x161b22))
            .track_focus(&self.transfer_browser_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_transfer_browser_key_down(event, window, cx);
            }))
            .when(can_transfer, |this| {
                this.child(
                    div()
                        .h(px(30.))
                        .px_1()
                        .border_b_1()
                        .border_color(rgb(0x30363d))
                        .bg(rgb(0x12171f))
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .child(compact_transfer_toolbar_button(
                            "transfer-browser-new-file",
                            "＋F",
                            cx.listener(|this, _, window, cx| {
                                this.open_transfer_new_file_dialog(window, cx);
                            }),
                        ))
                        .child(compact_transfer_toolbar_button(
                            "transfer-browser-new-folder",
                            "＋D",
                            cx.listener(|this, _, window, cx| {
                                this.open_transfer_new_folder_dialog(window, cx);
                            }),
                        ))
                        .child(transfer_toolbar_divider())
                        .child(compact_transfer_toolbar_button(
                            "transfer-browser-upload-file",
                            "⬆",
                            cx.listener(|this, _, _, cx| {
                                this.prompt_transfer_browser_upload_path(
                                    TransferPathPromptKind::UploadFile,
                                    cx,
                                );
                            }),
                        ))
                        .child(compact_transfer_toolbar_button(
                            "transfer-browser-upload-folder",
                            "⬆D",
                            cx.listener(|this, _, _, cx| {
                                this.prompt_transfer_browser_upload_path(
                                    TransferPathPromptKind::UploadDirectory,
                                    cx,
                                );
                            }),
                        ))
                        .child(
                            div()
                                .when(selected_count == 0, |this| this.opacity(0.45))
                                .child(compact_transfer_toolbar_button(
                                    "transfer-browser-download-selected",
                                    "⬇",
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
                                    "🗑",
                                    cx.listener(|this, _, window, cx| {
                                        this.open_selected_transfer_delete_dialog(window, cx);
                                    }),
                                )),
                        )
                        .child(transfer_toolbar_divider())
                        .child(compact_transfer_toolbar_button(
                            "transfer-browser-go-up",
                            "⤴",
                            cx.listener(|this, _, window, cx| {
                                this.open_transfer_parent_directory(window, cx);
                            }),
                        ))
                        .child(
                            div()
                                .when(!can_go_back, |this| this.opacity(0.4))
                                .child(compact_transfer_toolbar_button(
                                    "transfer-browser-back",
                                    "◀",
                                    cx.listener(|this, _, window, cx| {
                                        this.open_transfer_browser_history(1, window, cx);
                                    }),
                                )),
                        )
                        .child(
                            div()
                                .when(!can_go_forward, |this| this.opacity(0.4))
                                .child(compact_transfer_toolbar_button(
                                    "transfer-browser-forward",
                                    "▶",
                                    cx.listener(|this, _, window, cx| {
                                        this.open_transfer_browser_history(-1, window, cx);
                                    }),
                                )),
                        )
                        .child(compact_transfer_toolbar_button(
                            "transfer-browser-refresh",
                            "↻",
                            cx.listener(|this, _, window, cx| {
                                this.refresh_transfer_browser(window, cx);
                            }),
                        ))
                        .child(div().flex_1())
                        .child(status_pill(
                            if can_transfer { "SSH" } else { "off" },
                            if can_transfer { rgb(0x34d399) } else { rgb(0x94a3b8) },
                            if can_transfer { rgb(0x10251d) } else { rgb(0x202633) },
                        ))
                        .child(compact_transfer_toolbar_button(
                            "transfer-browser-sync-cwd",
                            "CWD",
                            cx.listener(|this, _, window, cx| {
                                this.start_transfer_sync_cwd_job(window, cx);
                            }),
                        ))
                        .child(compact_transfer_toolbar_button(
                            "transfer-browser-auto-sync-cwd",
                            if auto_sync_cwd { "Auto*" } else { "Auto" },
                            cx.listener(|this, _, window, cx| {
                                this.toggle_transfer_browser_auto_sync_cwd(window, cx);
                            }),
                        ))
                        .child(compact_transfer_toolbar_button(
                            "transfer-browser-toggle-favorite",
                            if is_current_favorite { "★" } else { "☆" },
                            cx.listener(|this, _, _, cx| {
                                this.toggle_current_transfer_browser_favorite(cx);
                            }),
                        ))
                        .child(compact_transfer_toolbar_button(
                            "transfer-browser-expand-search",
                            if search_active { "Find*" } else { "Find" },
                            cx.listener(|this, _, window, cx| {
                                this.transfer_browser_search_expanded = true;
                                this.transfer_browser_status = "file search focused".to_string();
                                window.focus(&this.transfer_browser_search_focus);
                                cx.notify();
                            }),
                        )),
                )
                .child(self.transfer_browser_path_row(current_browser_path.clone(), cx))
            })
            .when(can_transfer && search_expanded, |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .border_b_1()
                        .border_color(rgb(0x30363d))
                        .bg(rgb(0x12171f))
                        .px_2()
                        .py_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x98a3b8))
                                .child(format!("{selected_count} marked for batch actions")),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(small_button(
                                    "transfer-browser-select-all",
                                    "Select All",
                                    cx.listener(|this, _, _, cx| {
                                        this.select_all_visible_transfer_entries(cx);
                                    }),
                                ))
                                .child(small_button(
                                    "transfer-browser-clear-selection",
                                    "Clear",
                                    cx.listener(|this, _, _, cx| {
                                        this.clear_transfer_browser_selection(cx);
                                    }),
                                ))
                                .child(
                                    div()
                                        .when(selected_count == 0, |this| this.opacity(0.45))
                                        .child(small_button(
                                            "transfer-browser-download-marked",
                                            "Download Marked",
                                            cx.listener(|this, _, window, cx| {
                                                this.start_selected_sftp_download_jobs(window, cx);
                                            }),
                                        )),
                                )
                                .child(
                                    div()
                                        .when(selected_count == 0, |this| this.opacity(0.45))
                                        .child(small_button(
                                            "transfer-browser-delete-marked",
                                            "Delete Marked",
                                            cx.listener(|this, _, window, cx| {
                                                this.open_selected_transfer_delete_dialog(
                                                    window, cx,
                                                );
                                            }),
                                        )),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .id(SharedString::from("transfer-browser-search"))
                                .h(px(34.))
                                .flex_1()
                                .min_w_0()
                                .rounded_sm()
                                .border_1()
                                .border_color(if search_active {
                                    rgb(0x256d3f)
                                } else {
                                    rgb(0x303848)
                                })
                                .bg(rgb(0x0d1320))
                                .px_3()
                                .flex()
                                .items_center()
                                .gap_2()
                                .cursor_pointer()
                                .track_focus(&self.transfer_browser_search_focus)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    window.focus(&this.transfer_browser_search_focus);
                                    cx.notify();
                                }))
                                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.handle_transfer_browser_search_key_down(event, cx);
                                }))
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight(800.))
                                        .text_color(if search_active {
                                            rgb(0x93c5fd)
                                        } else {
                                            rgb(0x64748b)
                                        })
                                        .child("Search"),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .font_family("JetBrains Mono")
                                        .text_xs()
                                        .text_color(if search_active {
                                            rgb(0xe5edf7)
                                        } else {
                                            rgb(0x64748b)
                                        })
                                        .child(truncate_preview(&search_value, 96)),
                                ),
                        )
                        .child(small_button(
                            "transfer-browser-clear-search",
                            if search_active { "Clear" } else { "Close" },
                            cx.listener(|this, _, _, cx| {
                                if this.transfer_browser_search.is_empty() {
                                    this.transfer_browser_search_expanded = false;
                                    this.transfer_browser_status = "file search closed".to_string();
                                } else {
                                    this.transfer_browser_search.clear();
                                    this.transfer_browser_status =
                                        "file search cleared".to_string();
                                }
                                cx.notify();
                            }),
                        )),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .rounded_sm()
                    .bg(rgb(0x10151e))
                    .px_3()
                    .py_2()
                    .child(
                        div()
                            .min_w_0()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
                            .child(format!(
                                "{} / {} item(s) · selected {} · marked {}",
                                visible_count, total_count, selected, selected_count
                            )),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x8f98aa))
                            .child(truncate_preview(&self.transfer_browser_status, 48)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .when(self.transfer_selected_remote_path.is_none(), |this| {
                                this.opacity(0.45)
                            })
                            .child(small_button(
                                "transfer-copy-path",
                                "Copy Path",
                                cx.listener(|this, _, _, cx| {
                                    this.copy_selected_transfer_path(TransferPathPart::Full, cx);
                                }),
                            ))
                            .child(small_button(
                                "transfer-copy-name",
                                "Copy Name",
                                cx.listener(|this, _, _, cx| {
                                    this.copy_selected_transfer_path(TransferPathPart::Name, cx);
                                }),
                            ))
                            .child(small_button(
                                "transfer-copy-dir",
                                "Copy Dir",
                                cx.listener(|this, _, _, cx| {
                                    this.copy_selected_transfer_path(
                                        TransferPathPart::Directory,
                                        cx,
                                    );
                                }),
                            ))
                            .child(small_button(
                                "transfer-send-path",
                                "Send Path",
                                cx.listener(|this, _, _, cx| {
                                    this.send_selected_transfer_path_to_terminal(
                                        TransferPathPart::Full,
                                        cx,
                                    );
                                }),
                            ))
                            .child(small_button(
                                "transfer-send-name",
                                "Send Name",
                                cx.listener(|this, _, _, cx| {
                                    this.send_selected_transfer_path_to_terminal(
                                        TransferPathPart::Name,
                                        cx,
                                    );
                                }),
                            ))
                            .child(small_button(
                                "transfer-send-dir",
                                "Send Dir",
                                cx.listener(|this, _, _, cx| {
                                    this.send_selected_transfer_path_to_terminal(
                                        TransferPathPart::Directory,
                                        cx,
                                    );
                                }),
                            ))
                            .child(small_button(
                                "transfer-properties",
                                "Properties",
                                cx.listener(|this, _, window, cx| {
                                    this.open_selected_transfer_properties(window, cx);
                                }),
                            ))
                            .child(small_button(
                                "transfer-open-editor",
                                "Open",
                                cx.listener(|this, _, window, cx| {
                                    this.open_selected_transfer_default(window, cx);
                                }),
                            ))
                            .when(selected_single_file, |this| {
                                this.child(small_button(
                                    "transfer-open-internal-editor",
                                    "Edit",
                                    cx.listener(|this, _, window, cx| {
                                        this.open_selected_transfer_editor(window, cx);
                                    }),
                                ))
                                .child(small_button(
                                    "transfer-open-external-editor",
                                    "Ext",
                                    cx.listener(|this, _, window, cx| {
                                        this.open_selected_transfer_external(window, cx);
                                    }),
                                ))
                            })
                            .when(!selected_ai_actions.is_empty(), |this| {
                                let mut actions =
                                    div().flex().items_center().gap_1().flex_wrap().child(
                                        div()
                                            .h(px(28.))
                                            .px_2()
                                            .flex()
                                            .items_center()
                                            .rounded_sm()
                                            .bg(rgb(0x1b2433))
                                            .text_color(rgb(0x93c5fd))
                                            .text_size(px(10.))
                                            .font_weight(FontWeight(800.))
                                            .child("AI"),
                                    );
                                for action in selected_ai_actions.into_iter() {
                                    let action_id = action.id.clone();
                                    let label = truncate_preview(&action.name, 12);
                                    actions = actions.child(transfer_dynamic_toolbar_button(
                                        format!("transfer-selected-ai-file-{action_id}"),
                                        label,
                                        cx.listener(move |this, _, window, cx| {
                                            let Some(entry) = this.selected_transfer_entry() else {
                                                this.transfer_browser_status =
                                                    "select a remote file first".to_string();
                                                cx.notify();
                                                return;
                                            };
                                            this.start_transfer_file_ai_action(
                                                entry,
                                                action.clone(),
                                                window,
                                                cx,
                                            );
                                        }),
                                    ));
                                }
                                this.child(actions)
                            })
                            .child(small_button(
                                "transfer-move",
                                "Move",
                                cx.listener(|this, _, window, cx| {
                                    this.open_selected_transfer_move_dialog(window, cx);
                                }),
                            )),
                    ),
            )
            .child(
                div()
                    .id(SharedString::from("transfer-browser-table-scroll"))
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_scroll()
                    .scrollbar_width(px(8.))
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
                                            .bg(rgb(0x10151e))
                                            .px_2()
                                            .text_size(px(10.))
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(0x64748b))
                                            .child("ACTIONS"),
                                    ),
                            )
                            .child(rows),
                    ),
            )
    }
}

fn compact_transfer_toolbar_button(
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // Tauri FileExplorerToolbar: h-7 icon buttons, muted until hover.
    div()
        .id(SharedString::from(id.into()))
        .size(px(26.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_size(px(12.))
        .text_color(rgb(0x8b949e))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xc9d1d9)))
        .child(label)
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
