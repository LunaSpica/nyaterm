use gpui::Context;
use nyaterm_transport::SftpFileEntry;
use nyaterm_ui::NyaMenuItem;

use crate::features::NyaTermApp;
use crate::models::{TransferBrowserContextTarget, TransferPathPromptKind};

use super::TransferPathPart;

impl NyaTermApp {
    pub(in crate::features::pages::transfers) fn transfer_browser_context_menu_items(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Vec<NyaMenuItem> {
        match self.transfer.browser_view().context_target.clone() {
            TransferBrowserContextTarget::CurrentDirectory => {
                self.transfer_browser_current_context_menu_items(cx)
            }
            TransferBrowserContextTarget::ParentDirectory => {
                self.transfer_browser_parent_context_menu_items(cx)
            }
            TransferBrowserContextTarget::Entry(path) => {
                if self.transfer.rename_dialog_is_open() {
                    return Vec::new();
                }
                let Some(entry) = self
                    .transfer
                    .browser_view()
                    .entries
                    .iter()
                    .find(|entry| entry.matches_identity(&path))
                    .cloned()
                else {
                    return Vec::new();
                };
                self.transfer_browser_entry_context_menu_items(entry, cx)
            }
            TransferBrowserContextTarget::Suppressed => Vec::new(),
        }
    }

    pub(in crate::features::pages::transfers) fn transfer_browser_current_context_menu_items(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Vec<NyaMenuItem> {
        vec![
            NyaMenuItem::action(self.tr("fileExplorer.cmRefresh"))
                .icon("icons/fe/refresh.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.refresh_transfer_browser(window, cx);
                })),
            NyaMenuItem::submenu(
                self.tr("fileExplorer.upload"),
                vec![
                    NyaMenuItem::action(self.tr("fileExplorer.cmUpload"))
                        .icon("icons/fe/upload.svg")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.prompt_transfer_browser_upload_path(
                                TransferPathPromptKind::UploadFile,
                                cx,
                            );
                        })),
                    NyaMenuItem::action(self.tr("fileExplorer.uploadFolder"))
                        .icon("icons/fe/upload-folder.svg")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.prompt_transfer_browser_upload_path(
                                TransferPathPromptKind::UploadDirectory,
                                cx,
                            );
                        })),
                ],
            )
            .icon("icons/fe/upload.svg"),
            NyaMenuItem::separator(),
            NyaMenuItem::action(self.tr("fileExplorer.newFile"))
                .icon("icons/fe/new-file.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_transfer_new_file_dialog(window, cx);
                })),
            NyaMenuItem::action(self.tr("fileExplorer.newFolder"))
                .icon("icons/fe/new-folder.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_transfer_new_folder_dialog(window, cx);
                })),
            NyaMenuItem::action(self.tr("fileExplorer.newSymlink"))
                .icon("icons/conn/symlink.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_transfer_new_symlink_dialog(window, cx);
                })),
            NyaMenuItem::separator(),
            NyaMenuItem::action(self.tr("fileExplorer.cmCopyDirPath"))
                .icon("icons/copy.svg")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.copy_current_transfer_browser_path(cx);
                })),
            NyaMenuItem::action(self.tr("fileExplorer.cmTerminalDirPath"))
                .icon("icons/send.svg")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.send_current_transfer_browser_path_to_terminal(cx);
                })),
            NyaMenuItem::separator(),
            NyaMenuItem::action(self.tr("fileExplorer.cmProperties"))
                .icon("icons/menu/info.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_current_transfer_browser_properties(window, cx);
                })),
        ]
    }

    pub(in crate::features::pages::transfers) fn transfer_browser_parent_context_menu_items(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Vec<NyaMenuItem> {
        vec![
            NyaMenuItem::action(self.tr("fileExplorer.goUp"))
                .icon("icons/fe/up.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_transfer_parent_directory(window, cx);
                })),
            NyaMenuItem::action(self.tr("fileExplorer.cmRefresh"))
                .icon("icons/fe/refresh.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.refresh_transfer_browser(window, cx);
                })),
        ]
    }

    pub(in crate::features::pages::transfers) fn transfer_browser_entry_context_menu_items(
        &mut self,
        entry: SftpFileEntry,
        cx: &mut Context<Self>,
    ) -> Vec<NyaMenuItem> {
        let show_open_internal = self.show_transfer_open_internal_menu_entry(&entry);
        let show_open_external = self.show_transfer_open_external_menu_entry(&entry);
        let is_directory = entry.is_directory();

        let mut items = vec![
            NyaMenuItem::action(self.tr("fileExplorer.cmRefresh"))
                .icon("icons/fe/refresh.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.refresh_transfer_browser(window, cx);
                })),
            NyaMenuItem::submenu(
                self.tr("fileExplorer.upload"),
                vec![
                    NyaMenuItem::action(self.tr("fileExplorer.cmUpload"))
                        .icon("icons/fe/upload.svg")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.prompt_transfer_browser_upload_path(
                                TransferPathPromptKind::UploadFile,
                                cx,
                            );
                        })),
                    NyaMenuItem::action(self.tr("fileExplorer.uploadFolder"))
                        .icon("icons/fe/upload-folder.svg")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.prompt_transfer_browser_upload_path(
                                TransferPathPromptKind::UploadDirectory,
                                cx,
                            );
                        })),
                ],
            )
            .icon("icons/fe/upload.svg"),
            NyaMenuItem::action(self.tr("fileExplorer.cmDownload"))
                .icon("icons/fe/download.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.start_selected_sftp_download_jobs(window, cx);
                })),
            NyaMenuItem::separator(),
            NyaMenuItem::action(self.tr("fileExplorer.cmOpen"))
                .icon("icons/conn/folder.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_selected_transfer_default(window, cx);
                })),
        ];
        if show_open_internal {
            items.push(
                NyaMenuItem::action(self.tr("fileExplorer.cmOpenInternalEditor"))
                    .icon("icons/net/edit.svg")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_selected_transfer_editor(window, cx);
                    })),
            );
        }
        if show_open_external {
            items.push(
                NyaMenuItem::action(self.tr("fileExplorer.cmOpenExternalEditor"))
                    .icon("icons/net/edit.svg")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_selected_transfer_external(window, cx);
                    })),
            );
        }
        items.extend([
            NyaMenuItem::action(self.tr("fileExplorer.cmRename"))
                .icon("icons/session/rename.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_transfer_rename_dialog(window, cx);
                })),
            NyaMenuItem::action(self.tr("fileExplorer.cmMove"))
                .icon("icons/net/move.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    let Some(path) = this.transfer.browser_view().selected_remote_path.clone()
                    else {
                        return;
                    };
                    this.open_transfer_move_dialog(path, window, cx);
                })),
        ]);
        if is_directory {
            let favorite_path = entry.path.clone();
            items.push(
                NyaMenuItem::action(self.tr("fileExplorer.addToFavorites"))
                    .icon("icons/fe/star.svg")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.add_transfer_browser_favorite_path(favorite_path.clone(), cx);
                    })),
            );
        }

        items.extend([
            NyaMenuItem::separator(),
            NyaMenuItem::submenu(
                self.tr("common.copyToClipboard"),
                vec![
                    NyaMenuItem::action(self.tr("fileExplorer.cmCopyPath")).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.copy_selected_transfer_path(TransferPathPart::Full, cx);
                        },
                    )),
                    NyaMenuItem::action(self.tr("fileExplorer.cmCopyName")).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.copy_selected_transfer_path(TransferPathPart::Name, cx);
                        },
                    )),
                    NyaMenuItem::action(self.tr("fileExplorer.cmCopyDirPath")).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.copy_selected_transfer_path(TransferPathPart::Directory, cx);
                        }),
                    ),
                ],
            )
            .icon("icons/copy.svg"),
            NyaMenuItem::submenu(
                self.tr("fileExplorer.sendToTerminal"),
                vec![
                    NyaMenuItem::action(self.tr("fileExplorer.cmTerminalPath")).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.send_selected_transfer_path_to_terminal(
                                TransferPathPart::Full,
                                cx,
                            );
                        }),
                    ),
                    NyaMenuItem::action(self.tr("fileExplorer.cmTerminalName")).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.send_selected_transfer_path_to_terminal(
                                TransferPathPart::Name,
                                cx,
                            );
                        }),
                    ),
                    NyaMenuItem::action(self.tr("fileExplorer.cmTerminalDirPath")).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.send_selected_transfer_path_to_terminal(
                                TransferPathPart::Directory,
                                cx,
                            );
                        }),
                    ),
                ],
            )
            .icon("icons/send.svg"),
            NyaMenuItem::action(self.tr("fileExplorer.cmProperties"))
                .icon("icons/menu/info.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_selected_transfer_properties(window, cx);
                })),
        ]);

        let ai_items = self
            .enabled_transfer_file_ai_actions_for_entry(&entry)
            .into_iter()
            .map(|action| {
                let ai_entry = entry.clone();
                NyaMenuItem::action(action.name.clone()).on_click(cx.listener(
                    move |this, _, window, cx| {
                        this.start_transfer_file_ai_action(
                            ai_entry.clone(),
                            action.clone(),
                            window,
                            cx,
                        );
                    },
                ))
            })
            .collect::<Vec<_>>();
        if !ai_items.is_empty() {
            items.push(NyaMenuItem::separator());
            items.push(NyaMenuItem::submenu("AI", ai_items).icon("icons/ai.svg"));
        }
        items.push(NyaMenuItem::separator());
        items.push(
            NyaMenuItem::action(self.tr("fileExplorer.cmDelete"))
                .icon("icons/net/delete.svg")
                .danger()
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_selected_transfer_delete_dialog(window, cx);
                })),
        );
        items
    }
}
