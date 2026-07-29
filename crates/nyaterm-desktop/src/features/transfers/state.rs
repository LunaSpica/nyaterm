//! Grouped transfer feature state.
//!
//! The transfer feature is really five things sharing one panel: the job
//! queue, the SFTP browser, the file operation dialogs, the remote editor
//! workspace, and external-editor sync. Splitting them apart makes each
//! lifetime visible; the flat `transfer_*` prefix did not.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc;
use std::time::Instant;

use gpui::{FocusHandle, Pixels, WindowHandle};
use nyaterm_transport::{SftpDuplicatePolicy, SftpFileEntry};

use crate::features::TransferExternalSyncWindow;
use crate::models::{
    TransferBrowserColumnResizeState, TransferBrowserColumnWidths, TransferBrowserContextMenuState,
    TransferBrowserDragSelectionState, TransferBrowserFavoritesMenuState,
    TransferBrowserNavigationSnapshot, TransferBrowserPathMenuState,
    TransferBrowserPendingRenameState, TransferBrowserSessionCacheState, TransferBrowserSortColumn,
    TransferBrowserSortDirection, TransferBrowserUploadMenuState, TransferDeleteState,
    TransferEditorWorkspaceState, TransferExternalSyncPromptState, TransferHeightResizeState,
    TransferJobDeleteState, TransferJobMenuState, TransferJobResult, TransferJobState,
    TransferMoveState, TransferNewFileState, TransferNewFolderState, TransferNewSymlinkState,
    TransferPathPromptKind, TransferPropertiesState, TransferRenameState, TransferUnknownFileState,
};

use super::super::remote_editor_window::RemoteFileEditorWindow;

pub(in crate::features) struct TransferFeatureState {
    pub queue: TransferQueueState,
    pub paths: TransferPathState,
    pub browser: TransferBrowserState,
    pub file_ops: TransferFileOpsState,
    pub editor: TransferEditorState,
    pub external_sync: TransferExternalSyncState,
    panel: TransferPanelState,
}

/// Focus handles the transfer feature needs at construction time.
pub(in crate::features) struct TransferFeatureFocus {
    pub panel: FocusHandle,
    pub queue: FocusHandle,
    pub job_delete: FocusHandle,
    pub download_path: FocusHandle,
    pub browser: FocusHandle,
    pub rename: FocusHandle,
    pub move_to: FocusHandle,
    pub delete: FocusHandle,
    pub new_folder: FocusHandle,
    pub new_file: FocusHandle,
    pub new_symlink: FocusHandle,
    pub properties: FocusHandle,
    pub unknown_file: FocusHandle,
    pub editor: FocusHandle,
    pub default_editor: FocusHandle,
    pub external_sync: FocusHandle,
}

/// Upload/download job queue.
pub(in crate::features) struct TransferQueueState {
    pub tx: mpsc::Sender<TransferJobResult>,
    pub rx: mpsc::Receiver<TransferJobResult>,
    pub jobs: Vec<TransferJobState>,
    pub selected_job_id: Option<String>,
    pub job_delete: Option<TransferJobDeleteState>,
    pub job_menu: Option<TransferJobMenuState>,
    pub focus: FocusHandle,
    pub job_delete_focus: FocusHandle,
}

/// Manual transfer endpoints and the duplicate policy that applies to them.
pub(in crate::features) struct TransferPathState {
    pub remote: String,
    pub local: String,
    pub duplicate_policy: SftpDuplicatePolicy,
    pub prompt: Option<TransferPathPromptKind>,
}

/// SFTP browser: current listing, navigation history, selection and menus.
pub(in crate::features) struct TransferBrowserState {
    pub path: String,
    pub home_dir: String,
    pub home_dir_pending: bool,
    pub path_draft: String,
    pub path_editing: bool,
    pub entries: Vec<SftpFileEntry>,
    pub loading: bool,
    pub error: Option<String>,
    pub status: String,
    pub search: String,
    pub list_offset: usize,
    pub viewport_height: f32,
    pub search_expanded: bool,
    pub history: VecDeque<String>,
    pub history_index: usize,
    pub visited_history: VecDeque<String>,
    pub session_cache: HashMap<String, TransferBrowserSessionCacheState>,
    /// Latest SFTP navigation job per session; older results must not rewind the browser.
    pub navigation_jobs: HashMap<String, String>,
    pub pending_navigations: HashMap<String, TransferBrowserNavigationSnapshot>,
    pub auto_sync_cwd_last_at: Option<Instant>,
    pub favorites: VecDeque<String>,
    pub sort_column: TransferBrowserSortColumn,
    pub sort_direction: TransferBrowserSortDirection,
    pub column_widths: TransferBrowserColumnWidths,
    pub column_resize: Option<TransferBrowserColumnResizeState>,
    pub selected_remote_path: Option<String>,
    pub selected_remote_paths: HashSet<String>,
    pub drag_selection: Option<TransferBrowserDragSelectionState>,
    pub pending_rename: Option<TransferBrowserPendingRenameState>,
    pub pending_rename_token: u64,
    pub context_menu: Option<TransferBrowserContextMenuState>,
    pub favorites_menu: Option<TransferBrowserFavoritesMenuState>,
    pub path_menu: Option<TransferBrowserPathMenuState>,
    pub upload_menu: Option<TransferBrowserUploadMenuState>,
    pub focus: FocusHandle,
}

/// Rename/move/delete/create/properties dialogs over browser entries.
pub(in crate::features) struct TransferFileOpsState {
    pub rename: Option<TransferRenameState>,
    pub rename_focus_pending: bool,
    pub rename_focus: FocusHandle,
    pub move_to: Option<TransferMoveState>,
    pub move_focus: FocusHandle,
    pub delete: Option<TransferDeleteState>,
    pub delete_focus: FocusHandle,
    pub new_folder: Option<TransferNewFolderState>,
    pub new_folder_focus: FocusHandle,
    pub new_file: Option<TransferNewFileState>,
    pub new_file_focus: FocusHandle,
    pub new_symlink: Option<TransferNewSymlinkState>,
    pub new_symlink_focus: FocusHandle,
    pub properties: Option<TransferPropertiesState>,
    pub properties_focus: FocusHandle,
    pub unknown_file: Option<TransferUnknownFileState>,
    pub unknown_file_focus: FocusHandle,
}

/// Built-in remote file editor workspace.
pub(in crate::features) struct TransferEditorState {
    pub workspace: Option<TransferEditorWorkspaceState>,
    pub tabs_menu_open: bool,
    pub focus: FocusHandle,
    pub window: Option<WindowHandle<RemoteFileEditorWindow>>,
    pub window_open_pending: bool,
}

/// Handing a remote file to an external editor and syncing it back.
pub(in crate::features) struct TransferExternalSyncState {
    pub prompts: HashMap<String, TransferExternalSyncPromptState>,
    pub windows: HashMap<String, WindowHandle<TransferExternalSyncWindow>>,
    pub window_open_pending: HashSet<String>,
    pub always_uploads: HashSet<String>,
    pub focus: FocusHandle,
}

/// Panel chrome: focus routing and height.
struct TransferPanelState {
    focus: FocusHandle,
    height: f32,
    height_resize: Option<TransferHeightResizeState>,
}

impl TransferFeatureState {
    pub(in crate::features) fn new(
        remote_path: String,
        local_path: String,
        duplicate_policy: SftpDuplicatePolicy,
        panel_height: f32,
        focus: TransferFeatureFocus,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            queue: TransferQueueState {
                tx,
                rx,
                jobs: Vec::new(),
                selected_job_id: None,
                job_delete: None,
                job_menu: None,
                focus: focus.queue,
                job_delete_focus: focus.job_delete,
            },
            paths: TransferPathState {
                remote: remote_path,
                local: local_path,
                duplicate_policy,
                prompt: None,
            },
            browser: TransferBrowserState {
                path: ".".to_string(),
                home_dir: String::new(),
                home_dir_pending: false,
                path_draft: String::new(),
                path_editing: false,
                entries: Vec::new(),
                loading: false,
                error: None,
                status: "List a remote directory to browse files.".to_string(),
                search: String::new(),
                list_offset: 0,
                viewport_height: 0.,
                search_expanded: false,
                history: VecDeque::new(),
                history_index: 0,
                visited_history: VecDeque::new(),
                session_cache: HashMap::new(),
                navigation_jobs: HashMap::new(),
                pending_navigations: HashMap::new(),
                auto_sync_cwd_last_at: None,
                favorites: VecDeque::new(),
                sort_column: TransferBrowserSortColumn::Name,
                sort_direction: TransferBrowserSortDirection::Ascending,
                column_widths: TransferBrowserColumnWidths::default(),
                column_resize: None,
                selected_remote_path: None,
                selected_remote_paths: HashSet::new(),
                drag_selection: None,
                pending_rename: None,
                pending_rename_token: 0,
                context_menu: None,
                favorites_menu: None,
                path_menu: None,
                upload_menu: None,
                focus: focus.browser,
            },
            file_ops: TransferFileOpsState {
                rename: None,
                rename_focus_pending: false,
                rename_focus: focus.rename,
                move_to: None,
                move_focus: focus.move_to,
                delete: None,
                delete_focus: focus.delete,
                new_folder: None,
                new_folder_focus: focus.new_folder,
                new_file: None,
                new_file_focus: focus.new_file,
                new_symlink: None,
                new_symlink_focus: focus.new_symlink,
                properties: None,
                properties_focus: focus.properties,
                unknown_file: None,
                unknown_file_focus: focus.unknown_file,
            },
            editor: TransferEditorState {
                workspace: None,
                tabs_menu_open: false,
                focus: focus.editor,
                window: None,
                window_open_pending: false,
            },
            external_sync: TransferExternalSyncState {
                prompts: HashMap::new(),
                windows: HashMap::new(),
                window_open_pending: HashSet::new(),
                always_uploads: HashSet::new(),
                focus: focus.external_sync,
            },
            panel: TransferPanelState {
                focus: focus.panel,
                height: panel_height,
                height_resize: None,
            },
        }
    }

    pub(in crate::features) fn panel_focus(&self) -> &FocusHandle {
        &self.panel.focus
    }

    pub(in crate::features) fn panel_height(&self) -> f32 {
        self.panel.height
    }

    pub(in crate::features) fn set_panel_height(&mut self, height: f32) {
        self.panel.height = height;
    }

    pub(in crate::features) fn start_panel_height_resize(&mut self, start_y: Pixels) {
        self.panel.start_height_resize(start_y);
    }

    pub(in crate::features) fn update_panel_height_resize(
        &mut self,
        current_y: Pixels,
    ) -> Option<f32> {
        self.panel.update_height_resize(current_y)
    }

    pub(in crate::features) fn finish_panel_height_resize(&mut self) -> bool {
        self.panel.finish_height_resize()
    }
}

impl TransferPanelState {
    const HEIGHT_MIN: f32 = 60.;
    const HEIGHT_MAX: f32 = 600.;

    fn start_height_resize(&mut self, start_y: Pixels) {
        self.height_resize = Some(TransferHeightResizeState {
            start_y,
            start_height: gpui::px(self.height),
        });
    }

    fn update_height_resize(&mut self, current_y: Pixels) -> Option<f32> {
        let state = self.height_resize?;
        let delta = f32::from(current_y - state.start_y);
        self.height =
            (f32::from(state.start_height) - delta).clamp(Self::HEIGHT_MIN, Self::HEIGHT_MAX);
        Some(self.height)
    }

    fn finish_height_resize(&mut self) -> bool {
        self.height_resize.take().is_some()
    }
}

/// Column resize is self-contained: it only reads and writes browser geometry.
///
/// Keeping it here rather than on `NyaTermApp` means a drag cannot reach any
/// other app state; the page-level handlers are forwarders that own the redraw.
impl TransferBrowserState {
    pub(in crate::features) fn cancel_path_edit(&mut self) {
        self.path_draft.clear();
        self.path_editing = false;
        self.status = "remote directory path edit cancelled".to_string();
    }

    pub(in crate::features) fn start_column_resize(
        &mut self,
        column: TransferBrowserSortColumn,
        position_x: Pixels,
    ) {
        self.column_resize = Some(TransferBrowserColumnResizeState {
            column,
            start_x: position_x,
            start_width: self.column_widths.get(column),
        });
        self.status = format!("resizing {} column", column.label().to_lowercase());
    }

    /// Returns false when no resize is in flight, so the caller can skip the redraw.
    pub(in crate::features) fn update_column_resize(&mut self, position_x: Pixels) -> bool {
        let Some(state) = self.column_resize else {
            return false;
        };
        let next_width = state.start_width + (position_x - state.start_x);
        self.column_widths.set(state.column, next_width);
        let width = f32::from(self.column_widths.get(state.column)).round();
        self.status = format!("{} column: {width}px", state.column.label().to_lowercase());
        true
    }

    /// Returns false when no resize was in flight, so the caller can skip the redraw.
    pub(in crate::features) fn finish_column_resize(&mut self) -> bool {
        if self.column_resize.take().is_none() {
            return false;
        }
        self.status = "file column width updated".to_string();
        true
    }
}

impl TransferQueueState {
    pub(in crate::features) fn close_job_menu(&mut self) {
        self.job_menu = None;
    }
}

#[cfg(test)]
mod tests {
    use gpui::{TestAppContext, px};

    use super::TransferPanelState;

    #[test]
    fn transfer_panel_owns_focus_height_and_resize_lifecycle() {
        let cx = TestAppContext::single();
        let mut panel = TransferPanelState {
            focus: cx.update(|cx| cx.focus_handle()),
            height: 120.,
            height_resize: None,
        };

        panel.start_height_resize(px(400.));
        assert_eq!(panel.update_height_resize(px(450.)), Some(70.));
        assert_eq!(panel.update_height_resize(px(800.)), Some(60.));
        assert!(panel.finish_height_resize());
        assert!(!panel.finish_height_resize());
        assert!(panel.update_height_resize(px(300.)).is_none());

        panel.start_height_resize(px(400.));
        assert_eq!(panel.update_height_resize(px(-200.)), Some(600.));
        assert!(panel.finish_height_resize());
    }
}
