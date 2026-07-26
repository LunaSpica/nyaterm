use super::*;

impl NyaTermApp {
    pub(in crate::features) fn transfer_browser_context_menu_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let state =
            self.transfer
                .browser
                .context_menu
                .clone()
                .unwrap_or(TransferBrowserContextMenuState {
                    path: String::new(),
                    name: String::new(),
                    is_parent: false,
                    is_current_directory: false,
                    is_directory: false,
                    x: px(24.),
                    y: px(24.),
                });
        let selected_entry = self.selected_transfer_entry();
        let show_open_internal = selected_entry
            .as_ref()
            .is_some_and(|entry| self.show_transfer_open_internal_menu_entry(entry));
        let show_open_external = selected_entry
            .as_ref()
            .is_some_and(|entry| self.show_transfer_open_external_menu_entry(entry));
        let selected_file_ai_actions = selected_entry
            .as_ref()
            .filter(|entry| entry.file_type != SftpFileType::Directory)
            .map(|entry| self.enabled_transfer_file_ai_actions_for_entry(entry))
            .unwrap_or_default();
        let has_ai_actions = !selected_file_ai_actions.is_empty();
        let (viewport_w, viewport_h) = self.last_viewport_size;
        let preferred_height = if state.is_current_directory {
            380.
        } else if state.is_parent {
            150.
        } else {
            560. + if has_ai_actions { 96. } else { 0. }
        };
        let (menu_x, menu_y, menu_max_height) = transfer_menu_position(
            f32::from(state.x),
            f32::from(state.y),
            268.,
            preferred_height,
            viewport_w,
            viewport_h,
        );

        let mut ai_actions = div().flex().flex_col().gap_1();
        for action in selected_file_ai_actions {
            let action_id = action.id.clone();
            let label = truncate_preview(&action.name, 28);
            ai_actions = ai_actions.child(context_menu_button(
                palette,
                format!("transfer-context-ai-{action_id}"),
                label,
                cx.listener(move |this, _, window, cx| {
                    let Some(entry) = this.selected_transfer_entry() else {
                        this.transfer.browser.status = "select a remote file first".to_string();
                        this.close_transfer_browser_context_menu(cx);
                        return;
                    };
                    this.close_transfer_browser_context_menu(cx);
                    this.start_transfer_file_ai_action(entry, action.clone(), window, cx);
                }),
            ));
        }

        div()
            .id(SharedString::from("transfer-browser-context-menu-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_transfer_browser_context_menu(cx);
            }))
            .child(
                div()
                    .id(SharedString::from("transfer-browser-context-menu"))
                    .absolute()
                    .top(px(menu_y))
                    .left(px(menu_x))
                    .w(px(268.))
                    .max_h(px(menu_max_height))
                    .overflow_y_scroll()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .on_click(|_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .min_w_0()
                                    .font_family(crate::features::gpui_code_font_family())
                                    .text_xs()
                                    .text_color(rgb(palette.text))
                                    .child(truncate_preview(&state.name, 34)),
                            )
                            .child(status_pill(
                                if state.is_parent {
                                    "parent"
                                } else if state.is_current_directory {
                                    "current"
                                } else if state.is_directory {
                                    "dir"
                                } else {
                                    "file"
                                },
                                if state.is_directory {
                                    rgb(0x93c5fd)
                                } else {
                                    rgb(0x34d399)
                                },
                                rgb(palette.hover),
                            )),
                    )
                    .when(state.is_current_directory, |this| {
                        this.child(
                            context_menu_group(palette)
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-current-refresh",
                                    self.tr("fileExplorer.cmRefresh"),
                                    cx.listener(|this, _, window, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.refresh_transfer_browser(window, cx);
                                    }),
                                ))
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-current-upload-file",
                                    self.tr("fileExplorer.cmUpload"),
                                    cx.listener(|this, _, _, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.prompt_transfer_browser_upload_path(
                                            TransferPathPromptKind::UploadFile,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-current-upload-folder",
                                    self.tr("fileExplorer.uploadFolder"),
                                    cx.listener(|this, _, _, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.prompt_transfer_browser_upload_path(
                                            TransferPathPromptKind::UploadDirectory,
                                            cx,
                                        );
                                    }),
                                )),
                        )
                        .child(
                            context_menu_group(palette)
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-current-new-file",
                                    self.tr("fileExplorer.newFile"),
                                    cx.listener(|this, _, window, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.open_transfer_new_file_dialog(window, cx);
                                    }),
                                ))
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-current-new-folder",
                                    self.tr("fileExplorer.newFolder"),
                                    cx.listener(|this, _, window, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.open_transfer_new_folder_dialog(window, cx);
                                    }),
                                ))
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-current-new-symlink",
                                    self.tr("fileExplorer.newSymlink"),
                                    cx.listener(|this, _, window, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.open_transfer_new_symlink_dialog(window, cx);
                                    }),
                                )),
                        )
                        .child(
                            context_menu_group(palette)
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-current-copy-path",
                                    self.tr("fileExplorer.cmCopyDirPath"),
                                    cx.listener(|this, _, _, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.copy_current_transfer_browser_path(cx);
                                    }),
                                ))
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-current-send-path",
                                    self.tr("fileExplorer.cmTerminalDirPath"),
                                    cx.listener(|this, _, _, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.send_current_transfer_browser_path_to_terminal(cx);
                                    }),
                                )),
                        )
                        .child(context_menu_group(palette).child(context_menu_button(
                            palette,
                            "transfer-context-current-properties",
                            self.tr("fileExplorer.cmProperties"),
                            cx.listener(|this, _, window, cx| {
                                this.close_transfer_browser_context_menu(cx);
                                this.open_current_transfer_browser_properties(window, cx);
                            }),
                        )))
                    })
                    .when(state.is_parent, |this| {
                        this.child(
                            context_menu_group(palette)
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-parent-up",
                                    self.tr("fileExplorer.goUp"),
                                    cx.listener(|this, _, window, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.open_transfer_parent_directory(window, cx);
                                    }),
                                ))
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-parent-refresh",
                                    self.tr("fileExplorer.cmRefresh"),
                                    cx.listener(|this, _, window, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.refresh_transfer_browser(window, cx);
                                    }),
                                )),
                        )
                    })
                    .when(!state.is_parent && !state.is_current_directory, |this| {
                        this.child(
                            context_menu_group(palette)
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-refresh",
                                    self.tr("fileExplorer.cmRefresh"),
                                    cx.listener(|this, _, window, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.refresh_transfer_browser(window, cx);
                                    }),
                                ))
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-upload-file",
                                    self.tr("fileExplorer.cmUpload"),
                                    cx.listener(|this, _, _, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.prompt_transfer_browser_upload_path(
                                            TransferPathPromptKind::UploadFile,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-upload-folder",
                                    self.tr("fileExplorer.uploadFolder"),
                                    cx.listener(|this, _, _, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.prompt_transfer_browser_upload_path(
                                            TransferPathPromptKind::UploadDirectory,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-download",
                                    self.tr("fileExplorer.cmDownload"),
                                    cx.listener(|this, _, window, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.start_selected_sftp_download_jobs(window, cx);
                                    }),
                                )),
                        )
                    })
                    .when(!state.is_parent && !state.is_current_directory, |this| {
                        this.child(
                            context_menu_group(palette)
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-open",
                                    self.tr("fileExplorer.cmOpen"),
                                    cx.listener(|this, _, window, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.open_selected_transfer_default(window, cx);
                                    }),
                                ))
                                .when(show_open_internal, |this| {
                                    this.child(context_menu_button(
                                        palette,
                                        "transfer-context-open-internal",
                                        self.tr("fileExplorer.cmOpenInternalEditor"),
                                        cx.listener(|this, _, window, cx| {
                                            this.close_transfer_browser_context_menu(cx);
                                            this.open_selected_transfer_editor(window, cx);
                                        }),
                                    ))
                                })
                                .when(show_open_external, |this| {
                                    this.child(context_menu_button(
                                        palette,
                                        "transfer-context-open-external",
                                        self.tr("fileExplorer.cmOpenExternalEditor"),
                                        cx.listener(|this, _, window, cx| {
                                            this.close_transfer_browser_context_menu(cx);
                                            this.open_selected_transfer_external(window, cx);
                                        }),
                                    ))
                                })
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-rename",
                                    self.tr("fileExplorer.cmRename"),
                                    cx.listener(|this, _, window, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.open_transfer_rename_dialog(window, cx);
                                    }),
                                ))
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-move",
                                    self.tr("fileExplorer.cmMove"),
                                    cx.listener(|this, _, window, cx| {
                                        let Some(path) =
                                            this.transfer.browser.selected_remote_path.clone()
                                        else {
                                            this.close_transfer_browser_context_menu(cx);
                                            return;
                                        };
                                        this.close_transfer_browser_context_menu(cx);
                                        this.open_transfer_move_dialog(path, window, cx);
                                    }),
                                ))
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-delete",
                                    self.tr("fileExplorer.cmDelete"),
                                    cx.listener(|this, _, window, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.open_selected_transfer_delete_dialog(window, cx);
                                    }),
                                )),
                        )
                    })
                    .when(
                        state.is_directory && !state.is_parent && !state.is_current_directory,
                        |this| {
                            let favorite_path = state.path.clone();
                            this.child(context_menu_group(palette).child(context_menu_button(
                                palette,
                                "transfer-context-favorite",
                                self.tr("fileExplorer.addToFavorites"),
                                cx.listener(move |this, _, _, cx| {
                                    this.close_transfer_browser_context_menu(cx);
                                    this.add_transfer_browser_favorite_path(
                                        favorite_path.clone(),
                                        cx,
                                    );
                                }),
                            )))
                        },
                    )
                    .when(!state.is_parent && !state.is_current_directory, |this| {
                        this.child(
                            context_menu_group(palette)
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-copy-path",
                                    self.tr("fileExplorer.cmCopyPath"),
                                    cx.listener(|this, _, _, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.copy_selected_transfer_path(
                                            TransferPathPart::Full,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-copy-name",
                                    self.tr("fileExplorer.cmCopyName"),
                                    cx.listener(|this, _, _, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.copy_selected_transfer_path(
                                            TransferPathPart::Name,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-copy-dir",
                                    self.tr("fileExplorer.cmCopyDirPath"),
                                    cx.listener(|this, _, _, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.copy_selected_transfer_path(
                                            TransferPathPart::Directory,
                                            cx,
                                        );
                                    }),
                                )),
                        )
                    })
                    .when(!state.is_parent && !state.is_current_directory, |this| {
                        this.child(
                            context_menu_group(palette)
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-send-path",
                                    self.tr("fileExplorer.cmTerminalPath"),
                                    cx.listener(|this, _, _, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.send_selected_transfer_path_to_terminal(
                                            TransferPathPart::Full,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-send-name",
                                    self.tr("fileExplorer.cmTerminalName"),
                                    cx.listener(|this, _, _, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.send_selected_transfer_path_to_terminal(
                                            TransferPathPart::Name,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(context_menu_button(
                                    palette,
                                    "transfer-context-send-dir",
                                    self.tr("fileExplorer.cmTerminalDirPath"),
                                    cx.listener(|this, _, _, cx| {
                                        this.close_transfer_browser_context_menu(cx);
                                        this.send_selected_transfer_path_to_terminal(
                                            TransferPathPart::Directory,
                                            cx,
                                        );
                                    }),
                                )),
                        )
                    })
                    .when(!state.is_parent && !state.is_current_directory, |this| {
                        this.child(context_menu_group(palette).child(context_menu_button(
                            palette,
                            "transfer-context-properties",
                            self.tr("fileExplorer.cmProperties"),
                            cx.listener(|this, _, window, cx| {
                                this.close_transfer_browser_context_menu(cx);
                                this.open_selected_transfer_properties(window, cx);
                            }),
                        )))
                    })
                    .when(has_ai_actions, |this| {
                        this.child(
                            context_menu_group(palette)
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .font_weight(FontWeight(800.))
                                        .text_color(rgb(0x93c5fd))
                                        .child("AI"),
                                )
                                .child(ai_actions),
                        )
                    }),
            )
    }
}

fn context_menu_group(palette: crate::theme::ThemePalette) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .border_t_1()
        .border_color(rgb(palette.border))
        .pt_2()
}

fn context_menu_button(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: impl Into<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface))
        .text_color(rgb(palette.text))
        .text_xs()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)))
        .child(label.into())
        .on_click(on_click)
}
