use gpui::{
    AnyElement, App, Bounds, ClickEvent, ClipboardItem, Context, Element, Entity, FontWeight,
    GlobalElementId, Hsla, InspectorElementId, IntoElement, KeyDownEvent, LayoutId, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, ScrollDelta, ScrollWheelEvent,
    SharedString, Timer, Window, div, prelude::*, px, rgb, svg,
};
use nyaterm_transport::{
    SftpAttributeUpdate, SftpDuplicatePolicy, SftpFileEntry, SftpFileType, SftpService,
    SftpTransferProgress,
};

use std::cmp::Ordering;
use std::collections::{HashSet, VecDeque};
use std::time::Duration;

use crate::models::{
    TransferBrowserBreadcrumbSegment, TransferBrowserChildrenMenuStatus,
    TransferBrowserColumnWidths, TransferBrowserContextMenuState,
    TransferBrowserDragSelectionState, TransferBrowserFavoritesMenuState,
    TransferBrowserNavigationSnapshot, TransferBrowserPathMenuKind, TransferBrowserPathMenuState,
    TransferBrowserPendingRenameState, TransferBrowserSessionCacheState, TransferBrowserSortColumn,
    TransferBrowserSortDirection, TransferBrowserUploadMenuState, TransferDeleteState,
    TransferEditorField, TransferEditorState, TransferEditorWorkspaceState,
    TransferExternalSyncPromptState, TransferInputField, TransferJobEvent, TransferJobKind,
    TransferJobOutput, TransferJobResult, TransferJobState, TransferJobStatus, TransferMoveState,
    TransferNewFileState, TransferNewFolderState, TransferNewSymlinkState, TransferPathPromptKind,
    TransferPermissionTarget, TransferPropertiesField, TransferPropertiesState,
    TransferRenameState, TransferSymlinkField, TransferUnknownFileState,
};
use crate::widgets::{small_button, status_pill};
use nyaterm_core::{AiCustomActionConfig, ConnectionStore};

use super::super::{
    NyaTermApp, RemoteTextEditor, TextInputSetup, dialog_action_button, format_file_size,
    panel_header_with_actions, transfer_entry_icon, transfer_job_title, transfer_status_label,
    truncate_preview,
};

mod browser;
mod browser_columns;
mod browser_filter;
mod browser_keys;
mod browser_navigation;
mod browser_selection;
mod editor;
mod entry_row;
mod file_ops;
mod helpers;
mod overlays;
mod overlays_context;
mod overlays_create;
mod overlays_delete_move;
mod overlays_editor;
mod overlays_favorites;
mod overlays_properties;
mod overlays_unknown;
mod overlays_upload;
mod path_bar;
mod properties;
mod queue;

use entry_row::*;
use helpers::*;

const NATIVE_EDITOR_MAX_BYTES: u64 = 512 * 1024;

fn transfer_dialog_width(viewport_width: f32, preferred_width: f32) -> f32 {
    preferred_width.min((viewport_width - 32.).max(240.))
}

fn transfer_menu_position(
    x: f32,
    y: f32,
    menu_width: f32,
    preferred_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> (f32, f32, f32) {
    let margin = 8.;
    let max_height = (viewport_height - margin * 2.).max(80.);
    let height = preferred_height.min(max_height);
    let max_x = (viewport_width - menu_width - margin).max(margin);
    let max_y = (viewport_height - height - margin).max(margin);
    (x.clamp(margin, max_x), y.clamp(margin, max_y), max_height)
}

impl NyaTermApp {
    pub(in crate::features) fn transfers_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let can_transfer = self.active_ssh_config.is_some()
            && self
                .active_session_id
                .as_deref()
                .is_some_and(|session_id| !self.is_session_disconnected(session_id));
        let transfer_height = self.transfer.panel.height.clamp(60., 600.);
        let duplicate_prompt = self.active_duplicate_prompt.clone();

        // Tauri AppPanelContent: FileExplorer (flex-1) + vertical resize + FileTransfer fixed height.
        let palette = self.theme_palette();
        let view = div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(self.shell_transparent_color(palette.surface));

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
        .when_some(duplicate_prompt, |this, prompt| {
            this.child(self.duplicate_prompt_banner(prompt, cx))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{transfer_dialog_width, transfer_menu_position};

    #[test]
    fn dialog_width_uses_preferred_size_with_narrow_viewport_fallback() {
        assert_eq!(transfer_dialog_width(1280., 500.), 500.);
        assert_eq!(transfer_dialog_width(420., 500.), 388.);
        assert_eq!(transfer_dialog_width(200., 500.), 240.);
    }

    #[test]
    fn menu_position_stays_inside_viewport() {
        assert_eq!(
            transfer_menu_position(1200., 760., 268., 360., 1280., 800.),
            (1004., 432., 784.)
        );
        assert_eq!(
            transfer_menu_position(500., 500., 268., 560., 300., 240.),
            (24., 8., 224.)
        );
    }
}
