use gpui::{
    App, ClickEvent, ClipboardItem, Context, FontWeight, Hsla, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, SharedString, Timer, Window, div,
    prelude::*, px, rgb,
};
use nyaterm_session::{
    SftpAttributeUpdate, SftpDuplicatePolicy, SftpFileEntry, SftpFileType, SftpService,
    SftpTransferProgress,
};

use std::cmp::Ordering;
use std::collections::{HashSet, VecDeque};
use std::time::Duration;

use crate::ui::components::{section_header, small_button, status_pill};
use crate::ui::models::{
    TransferBrowserColumnResizeState, TransferBrowserColumnWidths, TransferBrowserContextMenuState,
    TransferBrowserDragSelectionState, TransferBrowserFavoritesMenuState,
    TransferBrowserPendingRenameState, TransferBrowserSessionCacheState, TransferBrowserSortColumn,
    TransferBrowserSortDirection, TransferDeleteState, TransferEditorField, TransferEditorState,
    TransferInputField, TransferJobEvent, TransferJobKind, TransferJobOutput, TransferJobResult,
    TransferJobState, TransferJobStatus, TransferMoveState, TransferNewFileState,
    TransferNewFolderState, TransferNewSymlinkState, TransferPathPromptKind,
    TransferPropertiesField, TransferPropertiesState, TransferRenameState, TransferSymlinkField,
    TransferUnknownFileState,
};
use nyaterm_domain::{AiCustomActionConfig, ConnectionStore};

use super::super::{
    NyaTermApp, entry_kind_label, format_file_size, policy_button, transfer_input,
    transfer_job_title, transfer_progress_bar, transfer_status_label, truncate_preview,
};

#[path = "transfers/browser.rs"]
mod browser;
#[path = "transfers/browser_columns.rs"]
mod browser_columns;
#[path = "transfers/browser_filter.rs"]
mod browser_filter;
#[path = "transfers/browser_keys.rs"]
mod browser_keys;
#[path = "transfers/browser_navigation.rs"]
mod browser_navigation;
#[path = "transfers/browser_selection.rs"]
mod browser_selection;
#[path = "transfers/editor.rs"]
mod editor;
#[path = "transfers/entry_row.rs"]
mod entry_row;
#[path = "transfers/file_ops.rs"]
mod file_ops;
#[path = "transfers/helpers.rs"]
mod helpers;
#[path = "transfers/overlays.rs"]
mod overlays;
#[path = "transfers/overlays_context.rs"]
mod overlays_context;
#[path = "transfers/overlays_create.rs"]
mod overlays_create;
#[path = "transfers/overlays_delete_move.rs"]
mod overlays_delete_move;
#[path = "transfers/overlays_editor.rs"]
mod overlays_editor;
#[path = "transfers/overlays_favorites.rs"]
mod overlays_favorites;
#[path = "transfers/overlays_properties.rs"]
mod overlays_properties;
#[path = "transfers/overlays_unknown.rs"]
mod overlays_unknown;
#[path = "transfers/path_bar.rs"]
mod path_bar;
#[path = "transfers/properties.rs"]
mod properties;
#[path = "transfers/queue.rs"]
mod queue;

use entry_row::*;
use helpers::*;

const NATIVE_EDITOR_MAX_BYTES: u64 = 512 * 1024;

impl NyaTermApp {
    pub(in crate::ui::view) fn transfers_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let ssh_status = if self.active_ssh_config.is_some() {
            "SSH session ready"
        } else if self.pending_ssh_config.is_some() {
            "SSH session connecting"
        } else {
            "No SSH session"
        };
        let can_transfer = self.active_ssh_config.is_some();
        let remote_value = if self.transfer_remote_path.is_empty() {
            " ".to_string()
        } else {
            self.transfer_remote_path.clone()
        };
        let local_value = if self.transfer_local_path.is_empty() {
            " ".to_string()
        } else {
            self.transfer_local_path.clone()
        };

        let mut view = div().size_full().p_5().child(section_header(
            "Transfers",
            "SFTP queue for the active SSH session.",
        ));
        if let Some(prompt) = self.active_duplicate_prompt.clone() {
            view = view.child(self.duplicate_prompt_banner(prompt, cx));
        }

        view = view
            .child(self.transfer_control_panel(
                ssh_status,
                can_transfer,
                remote_value,
                local_value,
                cx,
            ))
            .child(self.transfer_browser_view(can_transfer, cx))
            .child(self.transfer_queue_view(cx));

        view
    }

    fn transfer_control_panel(
        &mut self,
        ssh_status: &'static str,
        can_transfer: bool,
        remote_value: String,
        local_value: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (total, running, paused, completed, failed) =
            transfer_queue_counts(&self.transfer_jobs);
        let selected_remote = self
            .transfer_selected_remote_path
            .as_deref()
            .map(|path| truncate_preview(path, 48))
            .unwrap_or_else(|| "none".to_string());
        div()
            .mt_4()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x151923))
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_sm().text_color(rgb(0xe5edf7)).child("Queue"))
                            .child(status_pill(ssh_status, rgb(0x93c5fd), rgb(0x17253b))),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(queue_metric("Total", total, rgb(0x93c5fd)))
                            .child(queue_metric("Run", running, rgb(0xfacc15)))
                            .child(queue_metric("Pause", paused, rgb(0x93c5fd)))
                            .child(queue_metric("Done", completed, rgb(0x34d399)))
                            .child(queue_metric("Fail", failed, rgb(0xfb7185))),
                    ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .child(
                        transfer_input(
                            "transfer-remote-path",
                            "Remote",
                            remote_value,
                            self.transfer_focused_field == TransferInputField::Remote,
                        )
                        .track_focus(&self.transfer_focus)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.transfer_focused_field = TransferInputField::Remote;
                            window.focus(&this.transfer_focus);
                            cx.notify();
                        }))
                        .on_key_down(cx.listener(
                            |this, event: &KeyDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.handle_transfer_key_down(event, cx);
                            },
                        )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                transfer_input(
                                    "transfer-local-path",
                                    "Local",
                                    local_value,
                                    self.transfer_focused_field == TransferInputField::Local,
                                )
                                .flex_1()
                                .track_focus(&self.transfer_focus)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.transfer_focused_field = TransferInputField::Local;
                                    window.focus(&this.transfer_focus);
                                    cx.notify();
                                }))
                                .on_key_down(cx.listener(
                                    |this, event: &KeyDownEvent, _, cx| {
                                        cx.stop_propagation();
                                        this.handle_transfer_key_down(event, cx);
                                    },
                                )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(small_button(
                                        "transfer-pick-download-dir",
                                        "Save To",
                                        cx.listener(|this, _, _, cx| {
                                            this.prompt_transfer_path(
                                                TransferPathPromptKind::DownloadDirectory,
                                                cx,
                                            );
                                        }),
                                    ))
                                    .child(small_button(
                                        "transfer-open-download-dir",
                                        "Open",
                                        cx.listener(|this, _, _, cx| {
                                            this.reveal_transfer_download_dir(cx);
                                        }),
                                    ))
                                    .child(
                                        div()
                                            .flex()
                                            .gap_1()
                                            .child(small_button(
                                                "transfer-pick-upload-file",
                                                "File",
                                                cx.listener(|this, _, _, cx| {
                                                    this.prompt_transfer_path(
                                                        TransferPathPromptKind::UploadFile,
                                                        cx,
                                                    );
                                                }),
                                            ))
                                            .child(small_button(
                                                "transfer-pick-upload-dir",
                                                "Dir",
                                                cx.listener(|this, _, _, cx| {
                                                    this.prompt_transfer_path(
                                                        TransferPathPromptKind::UploadDirectory,
                                                        cx,
                                                    );
                                                }),
                                            )),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_xs().text_color(rgb(0x98a3b8)).child("Duplicate"))
                            .child(status_pill(
                                duplicate_policy_short_label(self.transfer_duplicate_policy),
                                rgb(0x93c5fd),
                                rgb(0x17253b),
                            ))
                            .child(div().text_xs().text_color(rgb(0x98a3b8)).child("Selected"))
                            .child(
                                div()
                                    .rounded_full()
                                    .bg(rgb(0x10251d))
                                    .px_2()
                                    .py_1()
                                    .text_xs()
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(0x34d399))
                                    .child(selected_remote),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(policy_button(
                                "transfer-policy-ask",
                                "Ask",
                                self.transfer_duplicate_policy == SftpDuplicatePolicy::Ask,
                                cx.listener(|this, _, _, cx| {
                                    this.transfer_duplicate_policy = SftpDuplicatePolicy::Ask;
                                    this.terminal_status =
                                        "transfer duplicate policy: ask".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(policy_button(
                                "transfer-policy-overwrite",
                                "Overwrite",
                                self.transfer_duplicate_policy == SftpDuplicatePolicy::Overwrite,
                                cx.listener(|this, _, _, cx| {
                                    this.transfer_duplicate_policy = SftpDuplicatePolicy::Overwrite;
                                    this.terminal_status =
                                        "transfer duplicate policy: overwrite".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(policy_button(
                                "transfer-policy-skip",
                                "Skip",
                                self.transfer_duplicate_policy == SftpDuplicatePolicy::Skip,
                                cx.listener(|this, _, _, cx| {
                                    this.transfer_duplicate_policy = SftpDuplicatePolicy::Skip;
                                    this.terminal_status =
                                        "transfer duplicate policy: skip".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(policy_button(
                                "transfer-policy-rename",
                                "Rename",
                                self.transfer_duplicate_policy == SftpDuplicatePolicy::Rename,
                                cx.listener(|this, _, _, cx| {
                                    this.transfer_duplicate_policy = SftpDuplicatePolicy::Rename;
                                    this.terminal_status =
                                        "transfer duplicate policy: rename".to_string();
                                    cx.notify();
                                }),
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .when(!can_transfer, |this| this.opacity(0.45))
                    .child(small_button(
                        "transfer-list",
                        "List",
                        cx.listener(|this, _, window, cx| {
                            this.start_sftp_list_job(window, cx);
                        }),
                    ))
                    .child(small_button(
                        "transfer-download",
                        "Download",
                        cx.listener(|this, _, window, cx| {
                            this.start_sftp_download_job(window, cx);
                        }),
                    ))
                    .child(small_button(
                        "transfer-upload",
                        "Upload",
                        cx.listener(|this, _, window, cx| {
                            this.start_sftp_upload_job(window, cx);
                        }),
                    ))
                    .child(small_button(
                        "transfer-rename",
                        "Rename",
                        cx.listener(|this, _, window, cx| {
                            this.open_transfer_rename_dialog(window, cx);
                        }),
                    )),
            )
    }
}
