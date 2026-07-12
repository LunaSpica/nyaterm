use gpui::{
    App, ClickEvent, ClipboardItem, Context, FontWeight, Hsla, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ScrollDelta, ScrollWheelEvent,
    SharedString, Timer, Window, div, prelude::*, px, rgb, svg,
};
use nyaterm_transport::{
    SftpAttributeUpdate, SftpDuplicatePolicy, SftpFileEntry, SftpFileType, SftpService,
    SftpTransferProgress,
};

use std::cmp::Ordering;
use std::collections::{HashSet, VecDeque};
use std::time::Duration;

use crate::ui::components::{small_button, status_pill};
use crate::ui::models::{
    TransferBrowserColumnResizeState, TransferBrowserColumnWidths, TransferBrowserContextMenuState,
    TransferBrowserDragSelectionState, TransferBrowserFavoritesMenuState,
    TransferBrowserPendingRenameState, TransferBrowserSessionCacheState, TransferBrowserSortColumn,
    TransferBrowserSortDirection, TransferBrowserUploadMenuState, TransferDeleteState,
    TransferEditorField, TransferEditorState, TransferInputField, TransferJobEvent,
    TransferJobKind, TransferJobOutput, TransferJobResult, TransferJobState, TransferJobStatus,
    TransferMoveState, TransferNewFileState, TransferNewFolderState, TransferNewSymlinkState,
    TransferPathPromptKind, TransferPropertiesField, TransferPropertiesState, TransferRenameState,
    TransferSymlinkField, TransferUnknownFileState,
};
use nyaterm_core::{AiCustomActionConfig, ConnectionStore};

use super::super::{
    NyaTermApp, entry_kind_label, format_file_size, transfer_entry_icon, transfer_job_title,
    transfer_progress_bar, transfer_status_label, truncate_preview,
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
#[path = "transfers/overlays_upload.rs"]
mod overlays_upload;
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
        let can_transfer = self.active_ssh_config.is_some();
        let transfer_height = self.transfer_panel_height.clamp(60., 600.);
        let duplicate_prompt = self.active_duplicate_prompt.clone();

        // Tauri AppPanelContent: FileExplorer (flex-1) + vertical resize + FileTransfer fixed height.
        let palette = self.theme_palette();
        let mut view = div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(palette.surface));

        if let Some(prompt) = duplicate_prompt {
            view = view.child(self.duplicate_prompt_banner(prompt, cx));
        }

        view.child(
            div()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .child(self.transfer_browser_view(can_transfer, cx)),
        )
        .child(self.transfer_height_resize_handle(cx))
        .child(
            div()
                .h(px(transfer_height))
                .flex_none()
                .overflow_hidden()
                .child(self.transfer_queue_view(cx)),
        )
    }
}
