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
use nyaterm_transport::{
    SftpDuplicatePolicy, SftpFileEntry, SftpFileProperties, SftpRemoteTextFile, SftpWriteTextResult,
};

use crate::features::TransferExternalSyncWindow;
use crate::models::{
    TransferBrowserColumnResizeState, TransferBrowserColumnWidths, TransferBrowserContextMenuState,
    TransferBrowserDragSelectionState, TransferBrowserFavoritesMenuState,
    TransferBrowserNavigationSnapshot, TransferBrowserPathMenuState,
    TransferBrowserPendingRenameState, TransferBrowserSessionCacheState, TransferBrowserSortColumn,
    TransferBrowserSortDirection, TransferBrowserUploadMenuState, TransferDeleteState,
    TransferEditorState, TransferEditorWorkspaceState, TransferExternalSyncPromptState,
    TransferHeightResizeState, TransferJobDeleteState, TransferJobMenuState, TransferJobResult,
    TransferJobState, TransferJobStatus, TransferMoveState, TransferNewFileState,
    TransferNewFolderState, TransferNewSymlinkState, TransferPathPromptKind,
    TransferPropertiesState, TransferRenameState, TransferUnknownFileState,
};

use super::super::remote_editor_window::RemoteFileEditorWindow;

pub(in crate::features) struct TransferFeatureState {
    queue: TransferQueueState,
    paths: TransferPathState,
    pub(super) browser: TransferBrowserState,
    file_ops: TransferFileOpsState,
    editor: TransferEditorFeatureState,
    external_sync: TransferExternalSyncState,
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
struct TransferQueueState {
    tx: mpsc::Sender<TransferJobResult>,
    rx: mpsc::Receiver<TransferJobResult>,
    jobs: Vec<TransferJobState>,
    next_job_sequence: u64,
    selected_job_id: Option<String>,
    job_delete: Option<TransferJobDeleteState>,
    job_menu: Option<TransferJobMenuState>,
    focus: FocusHandle,
    job_delete_focus: FocusHandle,
}

/// Manual transfer endpoints and the duplicate policy that applies to them.
struct TransferPathState {
    remote: String,
    local: String,
    duplicate_policy: SftpDuplicatePolicy,
    prompt: Option<TransferPathPromptKind>,
}

/// Borrowed presentation state for the SFTP browser.
///
/// Mutations stay on `TransferFeatureState`; renderers and app-level adapters
/// can inspect browser state without receiving the authoritative child.
pub(in crate::features) struct TransferBrowserView<'a> {
    pub path: &'a String,
    pub home_dir: &'a String,
    pub home_dir_pending: bool,
    pub path_draft: &'a String,
    pub path_editing: bool,
    pub entries: &'a Vec<SftpFileEntry>,
    pub loading: bool,
    pub error: &'a Option<String>,
    pub search: &'a String,
    pub list_offset: usize,
    pub viewport_height: f32,
    pub search_expanded: bool,
    pub history: &'a VecDeque<String>,
    pub history_index: usize,
    pub visited_history: &'a VecDeque<String>,
    pub favorites: &'a VecDeque<String>,
    pub sort_column: TransferBrowserSortColumn,
    pub sort_direction: TransferBrowserSortDirection,
    pub column_widths: TransferBrowserColumnWidths,
    pub column_resize: &'a Option<TransferBrowserColumnResizeState>,
    pub selected_remote_path: &'a Option<String>,
    pub selected_remote_paths: &'a HashSet<String>,
    pub drag_selection: &'a Option<TransferBrowserDragSelectionState>,
    pub pending_rename: &'a Option<TransferBrowserPendingRenameState>,
    pub context_menu: &'a Option<TransferBrowserContextMenuState>,
    pub favorites_menu: &'a Option<TransferBrowserFavoritesMenuState>,
    pub path_menu: &'a Option<TransferBrowserPathMenuState>,
    pub upload_menu: &'a Option<TransferBrowserUploadMenuState>,
    pub focus: &'a FocusHandle,
}

/// SFTP browser: current listing, navigation history, selection and menus.
pub(super) struct TransferBrowserState {
    pub(super) path: String,
    pub(super) home_dir: String,
    pub(super) home_dir_pending: bool,
    pub(super) path_draft: String,
    pub(super) path_editing: bool,
    pub(super) entries: Vec<SftpFileEntry>,
    pub(super) loading: bool,
    pub(super) error: Option<String>,
    pub(super) status: String,
    pub(super) search: String,
    pub(super) list_offset: usize,
    pub(super) viewport_height: f32,
    pub(super) search_expanded: bool,
    pub(super) history: VecDeque<String>,
    pub(super) history_index: usize,
    pub(super) visited_history: VecDeque<String>,
    pub(super) session_cache: HashMap<String, TransferBrowserSessionCacheState>,
    /// Latest SFTP navigation job per session; older results must not rewind the browser.
    pub(super) navigation_jobs: HashMap<String, String>,
    pub(super) pending_navigations: HashMap<String, TransferBrowserNavigationSnapshot>,
    pub(super) auto_sync_cwd_last_at: Option<Instant>,
    pub(super) favorites: VecDeque<String>,
    pub(super) sort_column: TransferBrowserSortColumn,
    pub(super) sort_direction: TransferBrowserSortDirection,
    pub(super) column_widths: TransferBrowserColumnWidths,
    pub(super) column_resize: Option<TransferBrowserColumnResizeState>,
    pub(super) selected_remote_path: Option<String>,
    pub(super) selected_remote_paths: HashSet<String>,
    pub(super) drag_selection: Option<TransferBrowserDragSelectionState>,
    pub(super) pending_rename: Option<TransferBrowserPendingRenameState>,
    pub(super) pending_rename_token: u64,
    pub(super) context_menu: Option<TransferBrowserContextMenuState>,
    pub(super) favorites_menu: Option<TransferBrowserFavoritesMenuState>,
    pub(super) path_menu: Option<TransferBrowserPathMenuState>,
    pub(super) upload_menu: Option<TransferBrowserUploadMenuState>,
    pub(super) focus: FocusHandle,
}

/// Rename/move/delete/create/properties dialogs over browser entries.
struct TransferFileOpsState {
    rename: Option<TransferRenameState>,
    rename_focus_pending: bool,
    rename_focus: FocusHandle,
    move_to: Option<TransferMoveState>,
    move_focus: FocusHandle,
    delete: Option<TransferDeleteState>,
    delete_focus: FocusHandle,
    new_folder: Option<TransferNewFolderState>,
    new_folder_focus: FocusHandle,
    new_file: Option<TransferNewFileState>,
    new_file_focus: FocusHandle,
    new_symlink: Option<TransferNewSymlinkState>,
    new_symlink_focus: FocusHandle,
    properties: Option<TransferPropertiesState>,
    properties_focus: FocusHandle,
    unknown_file: Option<TransferUnknownFileState>,
    unknown_file_focus: FocusHandle,
}

/// Built-in remote file editor workspace.
struct TransferEditorFeatureState {
    workspace: Option<TransferEditorWorkspaceState>,
    tabs_menu_open: bool,
    focus: FocusHandle,
    window: Option<WindowHandle<RemoteFileEditorWindow>>,
    window_open_pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::features) enum TransferEditorCloseOutcome {
    Missing,
    ConfirmationRequired,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::features) enum TransferEditorDiscardOutcome {
    Missing,
    TabDiscarded,
    WorkspaceDiscarded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::features) enum TransferEditorSaveOutcome {
    Saved,
    Conflict,
    SavedAndClosed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::features) enum TransferEditorCloseAfterSave {
    Missing,
    Loading,
    Saving,
    Ready(String),
    All,
}

/// Handing a remote file to an external editor and syncing it back.
struct TransferExternalSyncState {
    prompts: HashMap<String, TransferExternalSyncPromptState>,
    windows: HashMap<String, WindowHandle<TransferExternalSyncWindow>>,
    window_open_pending: HashSet<String>,
    always_uploads: HashSet<String>,
    focus: FocusHandle,
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
            file_ops: TransferFileOpsState::new(&focus),
            queue: TransferQueueState::new(tx, rx, focus.queue, focus.job_delete),
            paths: TransferPathState::new(remote_path, local_path, duplicate_policy),
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
            editor: TransferEditorFeatureState::new(focus.editor),
            external_sync: TransferExternalSyncState::new(focus.external_sync),
            panel: TransferPanelState {
                focus: focus.panel,
                height: panel_height,
                height_resize: None,
            },
        }
    }

    pub(in crate::features) fn browser_view(&self) -> TransferBrowserView<'_> {
        TransferBrowserView {
            path: &self.browser.path,
            home_dir: &self.browser.home_dir,
            home_dir_pending: self.browser.home_dir_pending,
            path_draft: &self.browser.path_draft,
            path_editing: self.browser.path_editing,
            entries: &self.browser.entries,
            loading: self.browser.loading,
            error: &self.browser.error,
            search: &self.browser.search,
            list_offset: self.browser.list_offset,
            viewport_height: self.browser.viewport_height,
            search_expanded: self.browser.search_expanded,
            history: &self.browser.history,
            history_index: self.browser.history_index,
            visited_history: &self.browser.visited_history,
            favorites: &self.browser.favorites,
            sort_column: self.browser.sort_column,
            sort_direction: self.browser.sort_direction,
            column_widths: self.browser.column_widths,
            column_resize: &self.browser.column_resize,
            selected_remote_path: &self.browser.selected_remote_path,
            selected_remote_paths: &self.browser.selected_remote_paths,
            drag_selection: &self.browser.drag_selection,
            pending_rename: &self.browser.pending_rename,
            context_menu: &self.browser.context_menu,
            favorites_menu: &self.browser.favorites_menu,
            path_menu: &self.browser.path_menu,
            upload_menu: &self.browser.upload_menu,
            focus: &self.browser.focus,
        }
    }

    pub(in crate::features) fn set_browser_status(&mut self, status: impl Into<String>) {
        self.browser.status = status.into();
    }

    pub(in crate::features) fn select_browser_path(&mut self, path: impl Into<String>) {
        self.browser.selected_remote_path = Some(path.into());
    }

    pub(in crate::features) fn browser_entries_are_empty(&self) -> bool {
        self.browser.entries.is_empty()
    }

    pub(in crate::features) fn apply_terminal_cwd_to_browser(&mut self, cwd: String) -> bool {
        if self.browser.path_editing || self.browser.path == cwd {
            return false;
        }
        self.browser.path = cwd.clone();
        self.browser.path_draft = cwd.clone();
        self.browser.status = format!("cwd synced: {cwd}");
        true
    }

    pub(in crate::features) fn browser_auto_sync_cwd_last_at(&self) -> Option<Instant> {
        self.browser.auto_sync_cwd_last_at
    }

    pub(in crate::features) fn mark_browser_auto_sync_cwd(&mut self, now: Instant) {
        self.browser.auto_sync_cwd_last_at = Some(now);
    }

    pub(in crate::features) fn reset_browser_auto_sync_cwd(&mut self) {
        self.browser.auto_sync_cwd_last_at = None;
    }

    pub(in crate::features) fn remove_browser_session_cache(&mut self, session_id: &str) {
        self.browser.session_cache.remove(session_id);
    }

    pub(in crate::features) fn has_browser_session_cache(&self, session_id: &str) -> bool {
        self.browser.session_cache.contains_key(session_id)
    }

    pub(in crate::features) fn set_browser_list_offset(&mut self, offset: usize) {
        self.browser.list_offset = offset;
    }

    pub(in crate::features) fn set_browser_viewport_height(&mut self, height: f32) -> bool {
        if (self.browser.viewport_height - height).abs() < 0.5 {
            return false;
        }
        self.browser.viewport_height = height;
        true
    }

    pub(in crate::features) fn start_browser_column_resize(
        &mut self,
        column: TransferBrowserSortColumn,
        position_x: Pixels,
    ) {
        self.browser.start_column_resize(column, position_x);
    }

    pub(in crate::features) fn update_browser_column_resize(&mut self, position_x: Pixels) -> bool {
        self.browser.update_column_resize(position_x)
    }

    pub(in crate::features) fn finish_browser_column_resize(&mut self) -> bool {
        self.browser.finish_column_resize()
    }

    pub(in crate::features) fn cancel_browser_path_edit(&mut self) {
        self.browser.cancel_path_edit();
    }

    pub(in crate::features) fn set_browser_search(&mut self, search: String) {
        self.browser.search = search;
        self.browser.list_offset = 0;
    }

    pub(in crate::features) fn expand_browser_search(&mut self) {
        self.browser.search_expanded = true;
    }

    pub(in crate::features) fn close_browser_search(&mut self) {
        self.browser.search_expanded = false;
    }

    pub(in crate::features) fn clear_browser_search(&mut self) {
        self.browser.search.clear();
        self.browser.list_offset = 0;
    }

    pub(in crate::features) fn toggle_browser_sort(
        &mut self,
        column: TransferBrowserSortColumn,
    ) -> String {
        if self.browser.sort_column == column {
            self.browser.sort_direction = self.browser.sort_direction.toggled();
        } else {
            self.browser.sort_column = column;
            self.browser.sort_direction = column.default_direction();
        }
        self.browser.list_offset = 0;
        let status = format!(
            "sorted by {} {}",
            self.browser.sort_column.label().to_lowercase(),
            self.browser.sort_direction.marker()
        );
        self.browser.status = status.clone();
        status
    }

    pub(in crate::features) fn begin_browser_path_edit(&mut self, path: String) {
        self.browser.path_draft = path;
        self.browser.path_editing = true;
        self.browser.status = "editing remote directory path".to_string();
    }

    pub(in crate::features) fn update_browser_path_draft(&mut self, path: String) {
        self.browser.path_draft = path;
        self.browser.status = "editing remote directory path".to_string();
    }

    pub(in crate::features) fn finish_browser_path_edit(&mut self) {
        self.browser.path_draft.clear();
        self.browser.path_editing = false;
    }

    pub(in crate::features) fn dismiss_browser_path_edit(&mut self) {
        self.browser.path_editing = false;
    }

    pub(in crate::features) fn open_browser_context_menu(
        &mut self,
        menu: TransferBrowserContextMenuState,
        status: impl Into<String>,
    ) {
        self.browser.path_menu = None;
        self.browser.drag_selection = None;
        self.browser.context_menu = Some(menu);
        self.browser.status = status.into();
    }

    pub(in crate::features) fn close_browser_context_menu(&mut self) {
        self.browser.context_menu = None;
    }

    pub(in crate::features) fn open_browser_favorites_menu(
        &mut self,
        menu: TransferBrowserFavoritesMenuState,
        status: impl Into<String>,
    ) {
        self.browser.upload_menu = None;
        self.browser.path_menu = None;
        self.browser.context_menu = None;
        self.browser.favorites_menu = Some(menu);
        self.browser.status = status.into();
    }

    pub(in crate::features) fn close_browser_favorites_menu(&mut self) {
        self.browser.favorites_menu = None;
    }

    pub(in crate::features) fn open_browser_upload_menu(
        &mut self,
        menu: TransferBrowserUploadMenuState,
    ) {
        self.browser.favorites_menu = None;
        self.browser.path_menu = None;
        self.browser.context_menu = None;
        self.browser.upload_menu = Some(menu);
        self.browser.status = "upload menu opened".to_string();
    }

    pub(in crate::features) fn close_browser_upload_menu(&mut self) {
        self.browser.upload_menu = None;
    }

    pub(in crate::features) fn open_browser_path_menu(
        &mut self,
        menu: TransferBrowserPathMenuState,
    ) {
        self.browser.context_menu = None;
        self.browser.favorites_menu = None;
        self.browser.upload_menu = None;
        self.browser.path_menu = Some(menu);
    }

    pub(in crate::features) fn close_browser_path_menu(&mut self) {
        self.browser.path_menu = None;
    }

    pub(in crate::features) fn store_browser_session_cache(
        &mut self,
        session_id: String,
        cache: TransferBrowserSessionCacheState,
    ) {
        self.browser.session_cache.insert(session_id, cache);
    }

    pub(in crate::features) fn restore_browser_session_cache(
        &mut self,
        session_id: &str,
    ) -> Option<String> {
        let cache = self.browser.session_cache.get(session_id)?.clone();
        let remote_path = cache.current_path.clone();
        self.browser.path = cache.current_path;
        self.browser.home_dir = cache.home_dir;
        self.browser.home_dir_pending = false;
        self.browser.path_draft.clear();
        self.browser.path_editing = false;
        self.browser.entries = cache.entries;
        self.browser.loading = false;
        self.browser.error = None;
        self.browser.status = format!(
            "restored cached directory · {} item(s)",
            self.browser.entries.len()
        );
        self.browser.history = cache.history;
        self.browser.history_index = cache
            .history_index
            .min(self.browser.history.len().saturating_sub(1));
        self.browser.visited_history = cache.visited_history;
        self.browser.clear_interaction();
        Some(remote_path)
    }

    pub(in crate::features) fn reset_browser_for_session(&mut self, ssh_active: bool) {
        self.browser.path = ".".to_string();
        self.browser.home_dir.clear();
        self.browser.home_dir_pending = false;
        self.browser.path_draft.clear();
        self.browser.path_editing = false;
        self.browser.entries.clear();
        self.browser.loading = false;
        self.browser.error = None;
        self.browser.status = if ssh_active {
            "List a remote directory to browse files.".to_string()
        } else {
            "Start an SSH session to browse remote files.".to_string()
        };
        self.browser.history.clear();
        self.browser.history_index = 0;
        self.browser.visited_history.clear();
        self.browser.clear_interaction();
    }

    pub(in crate::features) fn begin_browser_directory_load(&mut self, path: String) {
        self.browser.list_offset = 0;
        self.browser.path = path;
        self.browser.path_draft.clear();
        self.browser.path_editing = false;
        self.browser.path_menu = None;
        self.browser.selected_remote_path = None;
        self.browser.status = "Loading remote directory...".to_string();
        self.browser.loading = true;
        self.browser.error = None;
    }

    pub(in crate::features) fn begin_browser_parent_load(&mut self, path: String) {
        self.browser.path = path;
        self.browser.selected_remote_path = None;
        self.browser.selected_remote_paths.clear();
        self.browser.status = "Loading parent directory...".to_string();
        self.browser.loading = true;
        self.browser.error = None;
    }

    pub(in crate::features) fn browser_history_destination(
        &mut self,
        delta: isize,
    ) -> Result<String, &'static str> {
        if self.browser.history.is_empty() {
            return Err("directory history is empty");
        }
        let next = self.browser.history_index as isize + delta;
        if next < 0 || next as usize >= self.browser.history.len() {
            return Err(if delta > 0 {
                "no older directory history"
            } else {
                "no newer directory history"
            });
        }
        self.browser.history_index = next as usize;
        self.browser
            .history
            .get(self.browser.history_index)
            .cloned()
            .ok_or("directory history entry is unavailable")
    }

    pub(in crate::features) fn record_browser_history(&mut self, path: String) {
        self.browser.record_history(path);
    }

    pub(in crate::features) fn record_browser_visited_history(&mut self, path: String) {
        self.browser.record_visited_history(path);
    }

    pub(in crate::features) fn add_browser_favorite(&mut self, path: String) -> bool {
        let existed = self
            .browser
            .favorites
            .iter()
            .any(|existing| existing == &path);
        self.browser.favorites.retain(|existing| existing != &path);
        self.browser.favorites.push_front(path);
        self.browser.favorites.truncate(12);
        existed
    }

    pub(in crate::features) fn remove_browser_favorite(&mut self, path: &str) -> bool {
        let previous_len = self.browser.favorites.len();
        self.browser.favorites.retain(|existing| existing != path);
        self.browser.favorites.len() < previous_len
    }

    pub(in crate::features) fn replace_browser_favorites(&mut self, favorites: VecDeque<String>) {
        self.browser.favorites = favorites;
        self.browser.favorites.truncate(12);
    }

    pub(in crate::features) fn browser_favorites_owned(&self) -> Vec<String> {
        self.browser.favorites.iter().cloned().collect()
    }

    pub(in crate::features) fn clear_browser_favorites(&mut self) {
        self.browser.favorites.clear();
    }

    pub(in crate::features) fn retain_browser_selection(
        &mut self,
        mut retain: impl FnMut(&str) -> bool,
    ) {
        self.browser
            .selected_remote_paths
            .retain(|path| retain(path));
        if self
            .browser
            .selected_remote_path
            .as_deref()
            .is_some_and(|path| !retain(path))
        {
            self.browser.selected_remote_path = None;
        }
    }

    pub(in crate::features) fn select_browser_entry(&mut self, path: String) {
        self.browser.selected_remote_path = Some(path.clone());
        self.browser.selected_remote_paths.clear();
        self.browser.selected_remote_paths.insert(path);
    }

    pub(in crate::features) fn replace_browser_selection(
        &mut self,
        paths: HashSet<String>,
        active_path: Option<String>,
    ) -> usize {
        self.browser.selected_remote_paths = paths;
        self.browser.selected_remote_path = active_path;
        self.browser.selected_remote_paths.len()
    }

    pub(in crate::features) fn clear_browser_selection(&mut self) {
        self.browser.selected_remote_path = None;
        self.browser.selected_remote_paths.clear();
    }

    pub(in crate::features) fn activate_marked_browser_path(
        &mut self,
        path: &str,
    ) -> Option<usize> {
        self.browser.drag_selection = None;
        if !self.browser.selected_remote_paths.contains(path) {
            return None;
        }
        self.browser.selected_remote_path = Some(path.to_string());
        Some(self.browser.selected_remote_paths.len())
    }

    pub(in crate::features) fn toggle_browser_path_mark(&mut self, path: String) -> usize {
        if !self.browser.selected_remote_paths.remove(&path) {
            self.browser.selected_remote_paths.insert(path.clone());
        }
        self.browser.selected_remote_path = Some(path);
        self.browser.selected_remote_paths.len()
    }

    pub(in crate::features) fn set_browser_drag_selection(
        &mut self,
        selection: TransferBrowserDragSelectionState,
    ) {
        self.browser.drag_selection = Some(selection);
    }

    pub(in crate::features) fn clear_browser_drag_selection(&mut self) {
        self.browser.drag_selection = None;
    }

    pub(in crate::features) fn finish_browser_drag_selection(&mut self) -> bool {
        self.browser.drag_selection.take().is_some()
    }

    pub(in crate::features) fn schedule_browser_pending_rename(
        &mut self,
        path: &str,
    ) -> Option<u64> {
        if self.browser.selected_remote_path.as_deref() != Some(path)
            || self.browser.selected_remote_paths.len() != 1
            || !self.browser.selected_remote_paths.contains(path)
        {
            return None;
        }
        self.browser.pending_rename_token = self.browser.pending_rename_token.wrapping_add(1);
        let token = self.browser.pending_rename_token;
        self.browser.pending_rename = Some(TransferBrowserPendingRenameState {
            path: path.to_string(),
            token,
        });
        Some(token)
    }

    pub(in crate::features) fn resolve_browser_pending_rename(
        &mut self,
        path: &str,
        token: u64,
        rename_dialog_open: bool,
    ) -> bool {
        let should_rename = self
            .browser
            .pending_rename
            .as_ref()
            .is_some_and(|pending| pending.path == path && pending.token == token)
            && self.browser.selected_remote_path.as_deref() == Some(path)
            && self.browser.selected_remote_paths.len() == 1
            && self.browser.selected_remote_paths.contains(path)
            && !rename_dialog_open;
        self.browser.pending_rename = None;
        should_rename
    }

    pub(in crate::features) fn cancel_browser_pending_rename(&mut self) -> bool {
        self.browser.cancel_pending_rename()
    }

    pub(in crate::features) fn prepare_browser_navigation(
        &mut self,
        session_key: &str,
        remote_path: String,
    ) -> TransferBrowserNavigationSnapshot {
        let pending_job_id = self.browser.navigation_jobs.remove(session_key);
        if let Some(snapshot) =
            pending_job_id.and_then(|job_id| self.browser.pending_navigations.remove(&job_id))
        {
            self.browser.restore_navigation(snapshot.clone());
            return snapshot;
        }
        self.browser.capture_navigation(remote_path)
    }

    pub(in crate::features) fn restore_browser_navigation(
        &mut self,
        snapshot: TransferBrowserNavigationSnapshot,
    ) -> String {
        let remote_path = snapshot.remote_path.clone();
        self.browser.restore_navigation(snapshot);
        remote_path
    }

    pub(in crate::features) fn panel_focus(&self) -> &FocusHandle {
        &self.panel.focus
    }

    pub(in crate::features) fn queue_focus(&self) -> &FocusHandle {
        self.queue.focus()
    }

    pub(in crate::features) fn queue_delete_focus(&self) -> &FocusHandle {
        self.queue.delete_focus()
    }

    pub(in crate::features) fn transfer_jobs(&self) -> &[TransferJobState] {
        self.queue.jobs()
    }

    pub(in crate::features) fn transfer_jobs_are_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub(in crate::features) fn transfer_job(&self, job_id: &str) -> Option<&TransferJobState> {
        self.queue.job(job_id)
    }

    pub(in crate::features) fn transfer_job_mut(
        &mut self,
        job_id: &str,
    ) -> Option<&mut TransferJobState> {
        self.queue.job_mut(job_id)
    }

    pub(in crate::features) fn visit_transfer_jobs_mut(
        &mut self,
        visit: impl FnMut(&mut TransferJobState),
    ) {
        self.queue.visit_jobs_mut(visit);
    }

    pub(in crate::features) fn enqueue_transfer_job(&mut self, job: TransferJobState) {
        self.queue.enqueue(job);
    }

    pub(in crate::features) fn transfer_event_sender(&self) -> mpsc::Sender<TransferJobResult> {
        self.queue.event_sender()
    }

    pub(in crate::features) fn try_recv_transfer_event(
        &self,
    ) -> Result<TransferJobResult, mpsc::TryRecvError> {
        self.queue.try_recv_event()
    }

    pub(in crate::features) fn take_transfer_job_for_event(
        &mut self,
        job_id: &str,
    ) -> Option<(usize, TransferJobState)> {
        self.queue.take_job(job_id)
    }

    pub(in crate::features) fn restore_transfer_job_after_event(
        &mut self,
        queued: (usize, TransferJobState),
    ) {
        self.queue.restore_job(queued);
    }

    pub(in crate::features) fn next_transfer_job_id(&mut self, prefix: &str) -> String {
        self.queue.next_job_id(prefix)
    }

    pub(in crate::features) fn selected_transfer_job_id(&self) -> Option<&str> {
        self.queue.selected_job_id()
    }

    pub(in crate::features) fn select_transfer_job_id(&mut self, job_id: &str) -> bool {
        self.queue.select_job(job_id)
    }

    pub(in crate::features) fn selected_or_latest_visible_transfer_job_id(
        &self,
        session_id: Option<&str>,
    ) -> Option<String> {
        self.queue.selected_or_latest_visible_job_id(session_id)
    }

    pub(in crate::features) fn transfer_job_delete(&self) -> Option<&TransferJobDeleteState> {
        self.queue.job_delete()
    }

    pub(in crate::features) fn request_transfer_job_delete(
        &mut self,
        job_id: &str,
        title: String,
    ) -> bool {
        self.queue.request_job_delete(job_id, title)
    }

    pub(in crate::features) fn confirm_transfer_job_delete(&mut self) -> Option<(String, bool)> {
        self.queue.confirm_job_delete()
    }

    pub(in crate::features) fn cancel_transfer_job_delete(&mut self) {
        self.queue.cancel_job_delete();
    }

    pub(in crate::features) fn transfer_job_menu(&self) -> Option<&TransferJobMenuState> {
        self.queue.job_menu()
    }

    pub(in crate::features) fn open_transfer_job_menu_at(
        &mut self,
        job_id: &str,
        x: Pixels,
        y: Pixels,
    ) -> bool {
        self.queue.open_job_menu(job_id, x, y)
    }

    pub(in crate::features) fn close_transfer_job_menu(&mut self) {
        self.queue.close_job_menu();
    }

    pub(in crate::features) fn reset_transfer_queue_interaction(&mut self) {
        self.queue.reset_interaction();
    }

    pub(in crate::features) fn transfer_job_can_be_deleted(
        &self,
        job_id: &str,
        session_id: Option<&str>,
    ) -> bool {
        self.queue.can_delete_job(job_id, session_id)
    }

    pub(in crate::features) fn pause_visible_transfer_jobs(
        &mut self,
        session_id: Option<&str>,
    ) -> usize {
        self.queue.pause_visible_jobs(session_id)
    }

    pub(in crate::features) fn resume_visible_transfer_jobs(
        &mut self,
        session_id: Option<&str>,
    ) -> usize {
        self.queue.resume_visible_jobs(session_id)
    }

    pub(in crate::features) fn cancel_visible_transfer_jobs(
        &mut self,
        session_id: Option<&str>,
    ) -> usize {
        self.queue.cancel_visible_jobs(session_id)
    }

    pub(in crate::features) fn clear_completed_transfer_jobs_for_session(
        &mut self,
        session_id: Option<&str>,
    ) -> usize {
        self.queue.clear_completed_jobs(session_id)
    }

    pub(in crate::features) fn clear_stopped_transfer_jobs_for_session(
        &mut self,
        session_id: Option<&str>,
    ) -> usize {
        self.queue.clear_stopped_jobs(session_id)
    }

    pub(in crate::features) fn rename_dialog(&self) -> Option<&TransferRenameState> {
        self.file_ops.rename()
    }

    pub(in crate::features) fn rename_dialog_is_open(&self) -> bool {
        self.file_ops.rename().is_some()
    }

    pub(in crate::features) fn open_rename_dialog(&mut self, state: TransferRenameState) {
        self.file_ops.open_rename(state);
    }

    pub(in crate::features) fn close_rename_dialog(&mut self) {
        self.file_ops.close_rename();
    }

    pub(in crate::features) fn set_rename_value(&mut self, value: String) -> bool {
        self.file_ops.set_rename_value(value)
    }

    pub(in crate::features) fn rename_focus(&self) -> &FocusHandle {
        &self.file_ops.rename_focus
    }

    pub(in crate::features) fn schedule_rename_focus(&mut self) {
        self.file_ops.schedule_rename_focus();
    }

    pub(in crate::features) fn rename_focus_is_pending(&self) -> bool {
        self.file_ops.rename_focus_pending
    }

    pub(in crate::features) fn take_pending_rename_focus(&mut self) -> Option<FocusHandle> {
        self.file_ops.take_pending_rename_focus()
    }

    pub(in crate::features) fn move_dialog(&self) -> Option<&TransferMoveState> {
        self.file_ops.move_to.as_ref()
    }

    pub(in crate::features) fn open_move_dialog(&mut self, state: TransferMoveState) {
        self.file_ops.move_to = Some(state);
    }

    pub(in crate::features) fn close_move_dialog(&mut self) {
        self.file_ops.move_to = None;
    }

    pub(in crate::features) fn set_move_value(&mut self, value: String) -> bool {
        let Some(state) = self.file_ops.move_to.as_mut() else {
            return false;
        };
        state.value = value;
        true
    }

    pub(in crate::features) fn move_focus(&self) -> &FocusHandle {
        &self.file_ops.move_focus
    }

    pub(in crate::features) fn delete_dialog(&self) -> Option<&TransferDeleteState> {
        self.file_ops.delete.as_ref()
    }

    pub(in crate::features) fn open_delete_dialog(&mut self, state: TransferDeleteState) {
        self.file_ops.delete = Some(state);
    }

    pub(in crate::features) fn close_delete_dialog(&mut self) {
        self.file_ops.delete = None;
    }

    pub(in crate::features) fn take_delete_dialog(&mut self) -> Option<TransferDeleteState> {
        self.file_ops.delete.take()
    }

    pub(in crate::features) fn delete_focus(&self) -> &FocusHandle {
        &self.file_ops.delete_focus
    }

    pub(in crate::features) fn new_folder_dialog(&self) -> Option<&TransferNewFolderState> {
        self.file_ops.new_folder.as_ref()
    }

    pub(in crate::features) fn open_new_folder_dialog(&mut self, state: TransferNewFolderState) {
        self.file_ops.new_folder = Some(state);
    }

    pub(in crate::features) fn close_new_folder_dialog(&mut self) {
        self.file_ops.new_folder = None;
    }

    pub(in crate::features) fn set_new_folder_name(&mut self, value: String) -> bool {
        let Some(state) = self.file_ops.new_folder.as_mut() else {
            return false;
        };
        state.value = value;
        true
    }

    pub(in crate::features) fn toggle_new_folder_open_after_create(&mut self) -> bool {
        let Some(state) = self.file_ops.new_folder.as_mut() else {
            return false;
        };
        state.open_after_create = !state.open_after_create;
        true
    }

    pub(in crate::features) fn toggle_new_folder_mode_bit(&mut self, bit: u32) -> bool {
        let Some(state) = self.file_ops.new_folder.as_mut() else {
            return false;
        };
        state.mode ^= bit;
        true
    }

    pub(in crate::features) fn new_folder_focus(&self) -> &FocusHandle {
        &self.file_ops.new_folder_focus
    }

    pub(in crate::features) fn new_file_dialog(&self) -> Option<&TransferNewFileState> {
        self.file_ops.new_file.as_ref()
    }

    pub(in crate::features) fn open_new_file_dialog(&mut self, state: TransferNewFileState) {
        self.file_ops.new_file = Some(state);
    }

    pub(in crate::features) fn close_new_file_dialog(&mut self) {
        self.file_ops.new_file = None;
    }

    pub(in crate::features) fn set_new_file_name(&mut self, value: String) -> bool {
        let Some(state) = self.file_ops.new_file.as_mut() else {
            return false;
        };
        state.value = value;
        true
    }

    pub(in crate::features) fn toggle_new_file_open_after_create(&mut self) -> bool {
        let Some(state) = self.file_ops.new_file.as_mut() else {
            return false;
        };
        state.open_after_create = !state.open_after_create;
        true
    }

    pub(in crate::features) fn toggle_new_file_mode_bit(&mut self, bit: u32) -> bool {
        let Some(state) = self.file_ops.new_file.as_mut() else {
            return false;
        };
        state.mode ^= bit;
        true
    }

    pub(in crate::features) fn new_file_focus(&self) -> &FocusHandle {
        &self.file_ops.new_file_focus
    }

    pub(in crate::features) fn new_symlink_dialog(&self) -> Option<&TransferNewSymlinkState> {
        self.file_ops.new_symlink.as_ref()
    }

    pub(in crate::features) fn open_new_symlink_dialog(&mut self, state: TransferNewSymlinkState) {
        self.file_ops.new_symlink = Some(state);
    }

    pub(in crate::features) fn close_new_symlink_dialog(&mut self) {
        self.file_ops.new_symlink = None;
    }

    pub(in crate::features) fn set_new_symlink_input(
        &mut self,
        field: crate::models::TransferSymlinkField,
        value: String,
    ) -> bool {
        let Some(state) = self.file_ops.new_symlink.as_mut() else {
            return false;
        };
        state.focused_field = field;
        match field {
            crate::models::TransferSymlinkField::Name => state.name = value,
            crate::models::TransferSymlinkField::Target => state.target = value,
        }
        true
    }

    pub(in crate::features) fn new_symlink_focus(&self) -> &FocusHandle {
        &self.file_ops.new_symlink_focus
    }

    pub(in crate::features) fn properties_dialog(&self) -> Option<&TransferPropertiesState> {
        self.file_ops.properties.as_ref()
    }

    pub(in crate::features) fn properties_dialog_is_open_for_session(
        &self,
        session_id: Option<&str>,
    ) -> bool {
        self.file_ops.properties_matches(session_id, None)
    }

    pub(in crate::features) fn open_properties_dialog(&mut self, state: TransferPropertiesState) {
        self.file_ops.properties = Some(state);
    }

    pub(in crate::features) fn close_properties_dialog(&mut self) {
        self.file_ops.properties = None;
    }

    pub(in crate::features) fn close_properties_dialog_for_session(
        &mut self,
        session_id: &str,
    ) -> bool {
        if !self.file_ops.properties_matches(Some(session_id), None) {
            return false;
        }
        self.file_ops.properties = None;
        true
    }

    pub(in crate::features) fn set_properties_focused_field(
        &mut self,
        field: crate::models::TransferPropertiesField,
    ) -> Option<String> {
        let state = self.file_ops.properties.as_mut()?;
        state.focused_field = field;
        Some(match field {
            crate::models::TransferPropertiesField::Mode => state.mode_value.clone(),
            crate::models::TransferPropertiesField::Owner => state.owner_value.clone(),
            crate::models::TransferPropertiesField::Group => state.group_value.clone(),
        })
    }

    pub(in crate::features) fn next_properties_field(
        &self,
    ) -> Option<crate::models::TransferPropertiesField> {
        self.file_ops
            .properties
            .as_ref()
            .map(|state| match state.focused_field {
                crate::models::TransferPropertiesField::Mode => {
                    crate::models::TransferPropertiesField::Owner
                }
                crate::models::TransferPropertiesField::Owner => {
                    crate::models::TransferPropertiesField::Group
                }
                crate::models::TransferPropertiesField::Group => {
                    crate::models::TransferPropertiesField::Mode
                }
            })
    }

    pub(in crate::features) fn set_properties_input(
        &mut self,
        field: crate::models::TransferPropertiesField,
        value: String,
    ) -> bool {
        let Some(state) = self.file_ops.properties.as_mut() else {
            return false;
        };
        match field {
            crate::models::TransferPropertiesField::Mode => state.mode_value = value,
            crate::models::TransferPropertiesField::Owner => state.owner_value = value,
            crate::models::TransferPropertiesField::Group => state.group_value = value,
        }
        state.focused_field = field;
        state.error = None;
        true
    }

    pub(in crate::features) fn properties_input_values(&self) -> Option<(String, String, String)> {
        self.file_ops.properties.as_ref().map(|state| {
            (
                state.mode_value.clone(),
                state.owner_value.clone(),
                state.group_value.clone(),
            )
        })
    }

    pub(in crate::features) fn set_properties_mode_value(&mut self, value: String) -> bool {
        let Some(state) = self.file_ops.properties.as_mut() else {
            return false;
        };
        state.mode_value = value;
        true
    }

    pub(in crate::features) fn toggle_properties_recursive(&mut self) -> bool {
        let Some(state) = self.file_ops.properties.as_mut() else {
            return false;
        };
        state.recursive = !state.recursive;
        true
    }

    pub(in crate::features) fn set_properties_error(&mut self, error: String) -> bool {
        let Some(state) = self.file_ops.properties.as_mut() else {
            return false;
        };
        state.saving = false;
        state.error = Some(error);
        true
    }

    pub(in crate::features) fn begin_properties_save(&mut self) -> bool {
        let Some(state) = self.file_ops.properties.as_mut() else {
            return false;
        };
        state.saving = true;
        state.error = None;
        true
    }

    pub(in crate::features) fn complete_properties_load(
        &mut self,
        session_id: Option<&str>,
        remote_path: &str,
        properties: SftpFileProperties,
        mode_value: String,
        owner_value: String,
        group_value: String,
    ) -> bool {
        let Some(state) = self
            .file_ops
            .matching_properties_mut(session_id, remote_path)
        else {
            return false;
        };
        state.mode_value = mode_value;
        state.owner_value = owner_value;
        state.group_value = group_value;
        state.properties = Some(properties);
        state.error = None;
        true
    }

    pub(in crate::features) fn complete_properties_update(
        &mut self,
        session_id: Option<&str>,
        remote_path: &str,
        properties: SftpFileProperties,
    ) -> bool {
        let Some(state) = self
            .file_ops
            .matching_properties_mut(session_id, remote_path)
        else {
            return false;
        };
        state.properties = Some(properties);
        state.saving = false;
        state.error = None;
        self.file_ops.properties = None;
        true
    }

    pub(in crate::features) fn fail_properties_operation(
        &mut self,
        session_id: Option<&str>,
        remote_path: &str,
        error: String,
    ) -> bool {
        let Some(state) = self
            .file_ops
            .matching_properties_mut(session_id, remote_path)
        else {
            return false;
        };
        state.saving = false;
        state.error = Some(error);
        true
    }

    pub(in crate::features) fn properties_focus(&self) -> &FocusHandle {
        &self.file_ops.properties_focus
    }

    pub(in crate::features) fn unknown_file_dialog(&self) -> Option<&TransferUnknownFileState> {
        self.file_ops.unknown_file.as_ref()
    }

    pub(in crate::features) fn open_unknown_file_dialog(
        &mut self,
        state: TransferUnknownFileState,
    ) {
        self.file_ops.unknown_file = Some(state);
    }

    pub(in crate::features) fn close_unknown_file_dialog(&mut self) {
        self.file_ops.unknown_file = None;
    }

    pub(in crate::features) fn take_unknown_file_dialog(
        &mut self,
    ) -> Option<TransferUnknownFileState> {
        self.file_ops.unknown_file.take()
    }

    pub(in crate::features) fn unknown_file_focus(&self) -> &FocusHandle {
        &self.file_ops.unknown_file_focus
    }

    pub(in crate::features) fn external_sync_focus(&self) -> &FocusHandle {
        &self.external_sync.focus
    }

    pub(in crate::features) fn editor_focus(&self) -> &FocusHandle {
        &self.editor.focus
    }

    pub(in crate::features) fn editor_workspace(&self) -> Option<&TransferEditorWorkspaceState> {
        self.editor.workspace.as_ref()
    }

    pub(in crate::features) fn editor_workspace_snapshot(
        &self,
    ) -> Option<TransferEditorWorkspaceState> {
        self.editor.workspace.clone()
    }

    pub(in crate::features) fn editor_has_workspace(&self) -> bool {
        self.editor.workspace.is_some()
    }

    pub(in crate::features) fn editor_inline_overlay_is_open(&self) -> bool {
        self.editor.workspace.is_some()
            && self.editor.window.is_none()
            && !self.editor.window_open_pending
    }

    pub(in crate::features) fn active_editor_tab(&self) -> Option<&TransferEditorState> {
        self.editor
            .workspace
            .as_ref()
            .and_then(TransferEditorWorkspaceState::active_tab)
    }

    pub(in crate::features) fn active_editor_tab_mut(
        &mut self,
    ) -> Option<&mut TransferEditorState> {
        self.editor
            .workspace
            .as_mut()
            .and_then(TransferEditorWorkspaceState::active_tab_mut)
    }

    pub(in crate::features) fn editor_tab_snapshot(
        &self,
        tab_id: &str,
    ) -> Option<TransferEditorState> {
        self.editor
            .workspace
            .as_ref()?
            .tabs
            .iter()
            .find(|tab| tab.id == tab_id)
            .cloned()
    }

    pub(in crate::features) fn open_editor_tab(&mut self, tab: TransferEditorState) -> bool {
        let tab_id = tab.id.clone();
        if let Some(workspace) = self.editor.workspace.as_mut() {
            let already_open = workspace.tabs.iter().any(|current| current.id == tab_id);
            if !already_open {
                workspace.tabs.push(tab);
            }
            workspace.active_tab_id = tab_id;
            Self::clear_editor_close_state(workspace);
            self.editor.tabs_menu_open = false;
            already_open
        } else {
            self.editor.workspace = Some(TransferEditorWorkspaceState::new(tab));
            self.editor.tabs_menu_open = false;
            false
        }
    }

    pub(in crate::features) fn activate_editor_tab(&mut self, tab_id: &str) -> bool {
        self.editor.tabs_menu_open = false;
        let Some(workspace) = self.editor.workspace.as_mut() else {
            return false;
        };
        if !workspace.tabs.iter().any(|tab| tab.id == tab_id) {
            return false;
        }
        workspace.active_tab_id = tab_id.to_string();
        Self::clear_editor_close_state(workspace);
        true
    }

    pub(in crate::features) fn request_editor_tab_close(
        &mut self,
        tab_id: &str,
    ) -> TransferEditorCloseOutcome {
        self.editor.tabs_menu_open = false;
        let Some(workspace) = self.editor.workspace.as_mut() else {
            return TransferEditorCloseOutcome::Missing;
        };
        let Some(tab) = workspace.tabs.iter().find(|tab| tab.id == tab_id) else {
            return TransferEditorCloseOutcome::Missing;
        };
        if tab.dirty || tab.saving {
            workspace.active_tab_id = tab_id.to_string();
            workspace.close_confirm = true;
            workspace.pending_close_tab_id = Some(tab_id.to_string());
            workspace.close_after_save_all = false;
            return TransferEditorCloseOutcome::ConfirmationRequired;
        }
        workspace.remove_tab(tab_id);
        if workspace.tabs.is_empty() {
            self.editor.workspace = None;
            self.editor.window_open_pending = false;
        }
        TransferEditorCloseOutcome::Closed
    }

    pub(in crate::features) fn request_editor_close(&mut self) -> TransferEditorCloseOutcome {
        let Some(workspace) = self.editor.workspace.as_mut() else {
            return TransferEditorCloseOutcome::Missing;
        };
        if let Some(dirty_tab_id) = workspace
            .tabs
            .iter()
            .find(|tab| tab.dirty || tab.saving)
            .map(|tab| tab.id.clone())
        {
            workspace.active_tab_id = dirty_tab_id;
            workspace.close_confirm = true;
            workspace.pending_close_tab_id = None;
            workspace.close_after_save_all = false;
            return TransferEditorCloseOutcome::ConfirmationRequired;
        }
        self.editor.workspace = None;
        self.editor.tabs_menu_open = false;
        self.editor.window_open_pending = false;
        TransferEditorCloseOutcome::Closed
    }

    pub(in crate::features) fn discard_editor(&mut self) -> TransferEditorDiscardOutcome {
        let Some(workspace) = self.editor.workspace.as_mut() else {
            return TransferEditorDiscardOutcome::Missing;
        };
        if let Some(tab_id) = workspace.pending_close_tab_id.clone() {
            workspace.remove_tab(&tab_id);
            Self::clear_editor_close_state(workspace);
            if workspace.tabs.is_empty() {
                self.editor.workspace = None;
                self.editor.window_open_pending = false;
            }
            TransferEditorDiscardOutcome::TabDiscarded
        } else {
            self.editor.workspace = None;
            self.editor.tabs_menu_open = false;
            self.editor.window_open_pending = false;
            TransferEditorDiscardOutcome::WorkspaceDiscarded
        }
    }

    pub(in crate::features) fn cancel_editor_close(&mut self) -> bool {
        let Some(workspace) = self.editor.workspace.as_mut() else {
            return false;
        };
        Self::clear_editor_close_state(workspace);
        for tab in &mut workspace.tabs {
            tab.close_after_save = false;
        }
        true
    }

    pub(in crate::features) fn cancel_editor_reload(&mut self) -> bool {
        let Some(tab) = self.active_editor_tab_mut() else {
            return false;
        };
        tab.reload_confirm = false;
        if tab
            .error
            .as_deref()
            .is_some_and(|error| error.contains("Reload will discard"))
        {
            tab.error = None;
        }
        true
    }

    pub(in crate::features) fn cancel_editor_conflict(&mut self) -> bool {
        let Some(tab) = self.active_editor_tab_mut() else {
            return false;
        };
        tab.conflict = false;
        true
    }

    pub(in crate::features) fn clear_editor_close_request(&mut self) -> bool {
        let Some(workspace) = self.editor.workspace.as_mut() else {
            return false;
        };
        let changed = workspace.close_confirm
            || workspace.pending_close_tab_id.is_some()
            || workspace.close_after_save_all;
        Self::clear_editor_close_state(workspace);
        changed
    }

    pub(in crate::features) fn editor_close_confirmation_is_open(&self) -> bool {
        self.editor
            .workspace
            .as_ref()
            .is_some_and(|workspace| workspace.close_confirm)
    }

    pub(in crate::features) fn dirty_editor_tab_ids(&self) -> Vec<String> {
        self.editor
            .workspace
            .as_ref()
            .map(|workspace| {
                workspace
                    .tabs
                    .iter()
                    .filter(|tab| tab.dirty && !tab.loading && !tab.saving)
                    .map(|tab| tab.id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(in crate::features) fn set_editor_tab_error(
        &mut self,
        session_id: Option<&str>,
        remote_path: &str,
        error: String,
    ) -> bool {
        let Some(tab) = self
            .editor
            .workspace
            .as_mut()
            .and_then(|workspace| workspace.tab_mut(session_id, remote_path))
        else {
            return false;
        };
        tab.error = Some(error);
        true
    }

    pub(in crate::features) fn fail_editor_load(
        &mut self,
        session_id: Option<&str>,
        remote_path: &str,
        error: String,
    ) -> bool {
        let Some(tab) = self
            .editor
            .workspace
            .as_mut()
            .and_then(|workspace| workspace.tab_mut(session_id, remote_path))
        else {
            return false;
        };
        tab.loading = false;
        tab.error = Some(error);
        true
    }

    pub(in crate::features) fn begin_editor_tab_save(&mut self, tab_id: &str) -> bool {
        let Some(tab) = self
            .editor
            .workspace
            .as_mut()
            .and_then(|workspace| workspace.tabs.iter_mut().find(|tab| tab.id == tab_id))
        else {
            return false;
        };
        if tab.loading || tab.saving {
            return false;
        }
        tab.saving = true;
        tab.error = None;
        tab.conflict = false;
        tab.reload_confirm = false;
        true
    }

    pub(in crate::features) fn prepare_editor_close_after_save(
        &mut self,
    ) -> TransferEditorCloseAfterSave {
        let Some(workspace) = self.editor.workspace.as_mut() else {
            return TransferEditorCloseAfterSave::Missing;
        };
        if let Some(tab_id) = workspace.pending_close_tab_id.clone() {
            let Some(tab) = workspace.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
                return TransferEditorCloseAfterSave::Missing;
            };
            if tab.loading {
                return TransferEditorCloseAfterSave::Loading;
            }
            tab.close_after_save = true;
            if tab.saving {
                TransferEditorCloseAfterSave::Saving
            } else {
                TransferEditorCloseAfterSave::Ready(tab_id)
            }
        } else {
            workspace.close_after_save_all = true;
            workspace.close_confirm = false;
            TransferEditorCloseAfterSave::All
        }
    }

    pub(in crate::features) fn complete_editor_load(
        &mut self,
        session_id: Option<&str>,
        remote_path: &str,
        file: SftpRemoteTextFile,
    ) -> bool {
        let Some(tab) = self
            .editor
            .workspace
            .as_mut()
            .and_then(|workspace| workspace.tab_mut(session_id, remote_path))
        else {
            return false;
        };
        tab.content = file.content;
        tab.base_size = Some(file.size);
        tab.base_modified_at = Some(file.modified_at);
        tab.loading = false;
        tab.saving = false;
        tab.dirty = false;
        tab.conflict = false;
        tab.close_after_save = false;
        tab.reload_confirm = false;
        tab.error = None;
        true
    }

    pub(in crate::features) fn complete_editor_save(
        &mut self,
        session_id: Option<&str>,
        remote_path: &str,
        result: SftpWriteTextResult,
    ) -> Option<TransferEditorSaveOutcome> {
        let workspace = self.editor.workspace.as_mut()?;
        let tab = workspace.tab_mut(session_id, remote_path)?;
        let mut remove_tab_id = None;
        let outcome = match result {
            SftpWriteTextResult::Saved { modified_at, size } => {
                if tab.close_after_save {
                    remove_tab_id = Some(tab.id.clone());
                }
                tab.base_size = Some(size);
                tab.base_modified_at = Some(modified_at);
                tab.saving = false;
                tab.dirty = false;
                tab.conflict = false;
                tab.close_after_save = false;
                tab.reload_confirm = false;
                tab.error = None;
                TransferEditorSaveOutcome::Saved
            }
            SftpWriteTextResult::Conflict { modified_at, size } => {
                tab.base_size = Some(size);
                tab.base_modified_at = Some(modified_at);
                tab.saving = false;
                tab.conflict = true;
                tab.close_after_save = false;
                tab.error = Some("Remote file changed before save.".to_string());
                workspace.close_after_save_all = false;
                workspace.close_confirm = true;
                TransferEditorSaveOutcome::Conflict
            }
        };
        if let Some(tab_id) = remove_tab_id.as_deref() {
            workspace.remove_tab(tab_id);
            workspace.pending_close_tab_id = None;
            workspace.close_confirm = false;
        }
        let close_workspace = workspace.tabs.is_empty()
            || (workspace.close_after_save_all
                && workspace.tabs.iter().all(|tab| !tab.dirty && !tab.saving));
        if close_workspace {
            self.editor.workspace = None;
            self.editor.tabs_menu_open = false;
            self.editor.window_open_pending = false;
            Some(TransferEditorSaveOutcome::SavedAndClosed)
        } else {
            Some(outcome)
        }
    }

    pub(in crate::features) fn fail_editor_operation(
        &mut self,
        session_id: Option<&str>,
        remote_path: &str,
        error: String,
    ) -> bool {
        let Some(workspace) = self.editor.workspace.as_mut() else {
            return false;
        };
        let Some(tab) = workspace.tab_mut(session_id, remote_path) else {
            return false;
        };
        tab.loading = false;
        tab.saving = false;
        tab.close_after_save = false;
        tab.error = Some(error);
        workspace.close_after_save_all = false;
        true
    }

    pub(in crate::features) fn remove_editor_tabs_for_session(
        &mut self,
        session_id: &str,
    ) -> usize {
        let Some(workspace) = self.editor.workspace.as_mut() else {
            return 0;
        };
        let before = workspace.tabs.len();
        let active_removed = workspace
            .active_tab()
            .is_some_and(|tab| tab.session_id.as_deref() == Some(session_id));
        workspace
            .tabs
            .retain(|tab| tab.session_id.as_deref() != Some(session_id));
        let removed = before.saturating_sub(workspace.tabs.len());
        if active_removed {
            workspace.active_tab_id = workspace
                .tabs
                .first()
                .map(|tab| tab.id.clone())
                .unwrap_or_default();
        }
        if workspace.tabs.is_empty() {
            self.editor.workspace = None;
            self.editor.tabs_menu_open = false;
            self.editor.window_open_pending = false;
        } else if removed > 0 {
            Self::clear_editor_close_state(workspace);
        }
        removed
    }

    pub(in crate::features) fn sync_editor_content(
        &mut self,
        tab_id: &str,
        content: String,
    ) -> bool {
        let Some(workspace) = self.editor.workspace.as_mut() else {
            return false;
        };
        Self::clear_editor_close_state(workspace);
        let Some(tab) = workspace.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
            return false;
        };
        tab.focused_field = crate::models::TransferEditorField::Content;
        if tab.content == content {
            return false;
        }
        tab.content = content;
        tab.dirty = true;
        tab.conflict = false;
        tab.close_after_save = false;
        tab.reload_confirm = false;
        tab.error = None;
        true
    }

    pub(in crate::features) fn editor_tabs_menu_is_open(&self) -> bool {
        self.editor.tabs_menu_open
    }

    pub(in crate::features) fn toggle_editor_tabs_menu(&mut self) -> bool {
        self.editor.tabs_menu_open = !self.editor.tabs_menu_open;
        self.editor.tabs_menu_open
    }

    pub(in crate::features) fn close_editor_tabs_menu(&mut self) -> bool {
        std::mem::take(&mut self.editor.tabs_menu_open)
    }

    pub(in crate::features) fn editor_window(
        &self,
    ) -> Option<WindowHandle<RemoteFileEditorWindow>> {
        self.editor.window
    }

    pub(in crate::features) fn editor_window_open_is_pending(&self) -> bool {
        self.editor.window_open_pending
    }

    pub(in crate::features) fn begin_editor_window_open(&mut self) -> bool {
        if self.editor.workspace.is_none()
            || self.editor.window.is_some()
            || self.editor.window_open_pending
        {
            return false;
        }
        self.editor.window_open_pending = true;
        true
    }

    pub(in crate::features) fn finish_editor_window_open(
        &mut self,
        handle: WindowHandle<RemoteFileEditorWindow>,
    ) {
        self.editor.window = Some(handle);
        self.editor.window_open_pending = false;
    }

    pub(in crate::features) fn finish_editor_window_activation(
        &mut self,
        handle: WindowHandle<RemoteFileEditorWindow>,
        activated: bool,
    ) -> bool {
        self.editor.window_open_pending = false;
        if !activated && self.editor.window.is_some_and(|current| current == handle) {
            self.editor.window = None;
            return true;
        }
        false
    }

    pub(in crate::features) fn clear_editor_window_if(
        &mut self,
        handle: WindowHandle<RemoteFileEditorWindow>,
    ) -> bool {
        if self.editor.window.is_some_and(|current| current == handle) {
            self.editor.window = None;
            true
        } else {
            false
        }
    }

    pub(in crate::features) fn clear_editor_window_tracking(&mut self) -> bool {
        let changed = self.editor.window.take().is_some() || self.editor.window_open_pending;
        self.editor.window_open_pending = false;
        changed
    }

    fn clear_editor_close_state(workspace: &mut TransferEditorWorkspaceState) {
        workspace.close_confirm = false;
        workspace.pending_close_tab_id = None;
        workspace.close_after_save_all = false;
    }

    pub(in crate::features) fn external_sync_prompt(
        &self,
        prompt_id: &str,
    ) -> Option<&TransferExternalSyncPromptState> {
        self.external_sync.prompts.get(prompt_id)
    }

    pub(in crate::features) fn insert_external_sync_prompt(
        &mut self,
        prompt_id: String,
        prompt: TransferExternalSyncPromptState,
    ) {
        self.external_sync.prompts.insert(prompt_id, prompt);
    }

    pub(in crate::features) fn active_external_sync_prompt(
        &self,
        session_id: &str,
    ) -> Option<(String, TransferExternalSyncPromptState)> {
        self.external_sync
            .prompts
            .iter()
            .find(|(prompt_id, prompt)| {
                prompt.session_id.as_deref() == Some(session_id)
                    && !self.external_sync.windows.contains_key(*prompt_id)
                    && !self.external_sync.window_open_pending.contains(*prompt_id)
            })
            .map(|(prompt_id, prompt)| (prompt_id.clone(), prompt.clone()))
    }

    pub(in crate::features) fn external_sync_always_uploads(&self, watch_key: &str) -> bool {
        self.external_sync.always_uploads.contains(watch_key)
    }

    pub(in crate::features) fn take_external_sync_prompt_for_upload(
        &mut self,
        prompt_id: &str,
        always_watch_key: Option<String>,
    ) -> Option<TransferExternalSyncPromptState> {
        let prompt = self.external_sync.prompts.remove(prompt_id)?;
        self.external_sync.windows.remove(prompt_id);
        self.external_sync.window_open_pending.remove(prompt_id);
        if let Some(watch_key) = always_watch_key {
            self.external_sync.always_uploads.insert(watch_key);
        }
        Some(prompt)
    }

    pub(in crate::features) fn dismiss_external_sync_prompt(&mut self, prompt_id: &str) -> bool {
        let removed = self.external_sync.prompts.remove(prompt_id).is_some();
        self.external_sync.windows.remove(prompt_id);
        self.external_sync.window_open_pending.remove(prompt_id);
        removed
    }

    pub(in crate::features) fn clear_external_sync_for_session(
        &mut self,
        session_id: &str,
    ) -> usize {
        let before = self.external_sync.prompts.len();
        self.external_sync
            .prompts
            .retain(|_, prompt| prompt.session_id.as_deref() != Some(session_id));
        let prompts = &self.external_sync.prompts;
        self.external_sync
            .windows
            .retain(|prompt_id, _| prompts.contains_key(prompt_id));
        self.external_sync
            .window_open_pending
            .retain(|prompt_id| prompts.contains_key(prompt_id));
        before.saturating_sub(self.external_sync.prompts.len())
    }

    pub(in crate::features) fn external_sync_has_window(&self) -> bool {
        !self.external_sync.windows.is_empty()
    }

    pub(in crate::features) fn external_sync_has_pending_window(&self) -> bool {
        !self.external_sync.window_open_pending.is_empty()
    }

    pub(in crate::features) fn first_external_sync_window(
        &self,
    ) -> Option<(String, WindowHandle<TransferExternalSyncWindow>)> {
        self.external_sync
            .windows
            .iter()
            .next()
            .map(|(prompt_id, handle)| (prompt_id.clone(), *handle))
    }

    pub(in crate::features) fn external_sync_window(
        &self,
        prompt_id: &str,
    ) -> Option<WindowHandle<TransferExternalSyncWindow>> {
        self.external_sync.windows.get(prompt_id).copied()
    }

    pub(in crate::features) fn external_sync_window_open_is_pending(
        &self,
        prompt_id: &str,
    ) -> bool {
        self.external_sync.window_open_pending.contains(prompt_id)
    }

    pub(in crate::features) fn begin_external_sync_window_open(&mut self, prompt_id: &str) -> bool {
        if !self.external_sync.prompts.contains_key(prompt_id)
            || self.external_sync.windows.contains_key(prompt_id)
            || self.external_sync.window_open_pending.contains(prompt_id)
        {
            return false;
        }
        self.external_sync
            .window_open_pending
            .insert(prompt_id.to_string());
        true
    }

    pub(in crate::features) fn finish_external_sync_window_open(
        &mut self,
        prompt_id: String,
        handle: WindowHandle<TransferExternalSyncWindow>,
    ) {
        self.external_sync.windows.insert(prompt_id.clone(), handle);
        self.external_sync.window_open_pending.remove(&prompt_id);
    }

    pub(in crate::features) fn clear_external_sync_window_tracking(
        &mut self,
        prompt_id: &str,
    ) -> bool {
        let removed_window = self.external_sync.windows.remove(prompt_id).is_some();
        let removed_pending = self.external_sync.window_open_pending.remove(prompt_id);
        removed_window || removed_pending
    }

    pub(in crate::features) fn remote_path(&self) -> &str {
        self.paths.remote_path()
    }

    pub(in crate::features) fn set_remote_path(&mut self, path: impl Into<String>) {
        self.paths.set_remote_path(path);
    }

    pub(in crate::features) fn normalized_remote_path(&self) -> String {
        self.paths.normalized_remote_path()
    }

    pub(in crate::features) fn local_path(&self) -> &str {
        self.paths.local_path()
    }

    pub(in crate::features) fn set_local_path(&mut self, path: impl Into<String>) {
        self.paths.set_local_path(path);
    }

    pub(in crate::features) fn duplicate_policy(&self) -> SftpDuplicatePolicy {
        self.paths.duplicate_policy()
    }

    pub(in crate::features) fn set_duplicate_policy(&mut self, policy: SftpDuplicatePolicy) {
        self.paths.set_duplicate_policy(policy);
    }

    pub(in crate::features) fn path_prompt_is_open(&self) -> bool {
        self.paths.prompt.is_some()
    }

    pub(in crate::features) fn begin_path_prompt(&mut self, kind: TransferPathPromptKind) -> bool {
        self.paths.begin_prompt(kind)
    }

    pub(in crate::features) fn finish_path_prompt(&mut self, kind: TransferPathPromptKind) -> bool {
        self.paths.finish_prompt(kind)
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

impl TransferFileOpsState {
    fn new(focus: &TransferFeatureFocus) -> Self {
        Self {
            rename: None,
            rename_focus_pending: false,
            rename_focus: focus.rename.clone(),
            move_to: None,
            move_focus: focus.move_to.clone(),
            delete: None,
            delete_focus: focus.delete.clone(),
            new_folder: None,
            new_folder_focus: focus.new_folder.clone(),
            new_file: None,
            new_file_focus: focus.new_file.clone(),
            new_symlink: None,
            new_symlink_focus: focus.new_symlink.clone(),
            properties: None,
            properties_focus: focus.properties.clone(),
            unknown_file: None,
            unknown_file_focus: focus.unknown_file.clone(),
        }
    }

    fn rename(&self) -> Option<&TransferRenameState> {
        self.rename.as_ref()
    }

    fn open_rename(&mut self, state: TransferRenameState) {
        self.rename = Some(state);
        self.rename_focus_pending = false;
    }

    fn close_rename(&mut self) {
        self.rename = None;
        self.rename_focus_pending = false;
    }

    fn set_rename_value(&mut self, value: String) -> bool {
        let Some(state) = self.rename.as_mut() else {
            return false;
        };
        state.value = value;
        true
    }

    fn schedule_rename_focus(&mut self) {
        self.rename_focus_pending = self.rename.is_some();
    }

    fn take_pending_rename_focus(&mut self) -> Option<FocusHandle> {
        if !self.rename_focus_pending || self.rename.is_none() {
            self.rename_focus_pending = false;
            return None;
        }
        self.rename_focus_pending = false;
        Some(self.rename_focus.clone())
    }

    fn properties_matches(&self, session_id: Option<&str>, remote_path: Option<&str>) -> bool {
        self.properties.as_ref().is_some_and(|state| {
            state.session_id.as_deref() == session_id
                && remote_path.is_none_or(|path| state.entry.path == path)
        })
    }

    fn matching_properties_mut(
        &mut self,
        session_id: Option<&str>,
        remote_path: &str,
    ) -> Option<&mut TransferPropertiesState> {
        self.properties.as_mut().filter(|state| {
            state.session_id.as_deref() == session_id && state.entry.path == remote_path
        })
    }
}

impl TransferExternalSyncState {
    fn new(focus: FocusHandle) -> Self {
        Self {
            prompts: HashMap::new(),
            windows: HashMap::new(),
            window_open_pending: HashSet::new(),
            always_uploads: HashSet::new(),
            focus,
        }
    }
}

impl TransferEditorFeatureState {
    fn new(focus: FocusHandle) -> Self {
        Self {
            workspace: None,
            tabs_menu_open: false,
            focus,
            window: None,
            window_open_pending: false,
        }
    }
}

impl TransferPathState {
    fn new(remote: String, local: String, duplicate_policy: SftpDuplicatePolicy) -> Self {
        Self {
            remote,
            local,
            duplicate_policy,
            prompt: None,
        }
    }

    fn remote_path(&self) -> &str {
        &self.remote
    }

    fn set_remote_path(&mut self, path: impl Into<String>) {
        self.remote = path.into();
    }

    fn normalized_remote_path(&self) -> String {
        let path = self.remote.trim();
        if path.is_empty() {
            ".".to_string()
        } else {
            path.to_string()
        }
    }

    fn local_path(&self) -> &str {
        &self.local
    }

    fn set_local_path(&mut self, path: impl Into<String>) {
        self.local = path.into();
    }

    fn duplicate_policy(&self) -> SftpDuplicatePolicy {
        self.duplicate_policy
    }

    fn set_duplicate_policy(&mut self, policy: SftpDuplicatePolicy) {
        self.duplicate_policy = policy;
    }

    fn begin_prompt(&mut self, kind: TransferPathPromptKind) -> bool {
        if self.prompt.is_some() {
            return false;
        }
        self.prompt = Some(kind);
        true
    }

    fn finish_prompt(&mut self, kind: TransferPathPromptKind) -> bool {
        if self.prompt != Some(kind) {
            return false;
        }
        self.prompt = None;
        true
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
    fn clear_interaction(&mut self) {
        self.selected_remote_path = None;
        self.selected_remote_paths.clear();
        self.drag_selection = None;
        self.cancel_pending_rename();
        self.context_menu = None;
        self.favorites_menu = None;
        self.path_menu = None;
        self.upload_menu = None;
    }

    fn record_history(&mut self, path: String) {
        if self.history.get(self.history_index) == Some(&path) {
            return;
        }
        if !self.history.is_empty() {
            let current_index = self.history_index.min(self.history.len() - 1);
            self.history.drain(..current_index);
        }
        self.history.push_front(path.clone());
        self.history_index = 0;
        self.record_visited_history(path);
    }

    fn record_visited_history(&mut self, path: String) {
        self.visited_history.retain(|existing| existing != &path);
        self.visited_history.push_front(path);
        self.visited_history.truncate(30);
    }

    fn capture_navigation(&self, remote_path: String) -> TransferBrowserNavigationSnapshot {
        TransferBrowserNavigationSnapshot {
            remote_path,
            browser_path: self.path.clone(),
            entries: self.entries.clone(),
            loading: self.loading,
            error: self.error.clone(),
            status: self.status.clone(),
            history: self.history.clone(),
            history_index: self.history_index,
            visited_history: self.visited_history.clone(),
            selected_path: self.selected_remote_path.clone(),
            selected_paths: self.selected_remote_paths.clone(),
            list_offset: self.list_offset,
        }
    }

    fn restore_navigation(&mut self, snapshot: TransferBrowserNavigationSnapshot) {
        self.path = snapshot.browser_path;
        self.entries = snapshot.entries;
        self.loading = snapshot.loading;
        self.error = snapshot.error;
        self.status = snapshot.status;
        self.history = snapshot.history;
        self.history_index = snapshot
            .history_index
            .min(self.history.len().saturating_sub(1));
        self.visited_history = snapshot.visited_history;
        self.selected_remote_path = snapshot.selected_path;
        self.selected_remote_paths = snapshot.selected_paths;
        self.list_offset = snapshot.list_offset;
    }

    fn cancel_pending_rename(&mut self) -> bool {
        let cancelled = self.pending_rename.take().is_some();
        if cancelled {
            self.pending_rename_token = self.pending_rename_token.wrapping_add(1);
        }
        cancelled
    }

    fn cancel_path_edit(&mut self) {
        self.path_draft.clear();
        self.path_editing = false;
        self.status = "remote directory path edit cancelled".to_string();
    }

    fn start_column_resize(&mut self, column: TransferBrowserSortColumn, position_x: Pixels) {
        self.column_resize = Some(TransferBrowserColumnResizeState {
            column,
            start_x: position_x,
            start_width: self.column_widths.get(column),
        });
        self.status = format!("resizing {} column", column.label().to_lowercase());
    }

    /// Returns false when no resize is in flight, so the caller can skip the redraw.
    fn update_column_resize(&mut self, position_x: Pixels) -> bool {
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
    fn finish_column_resize(&mut self) -> bool {
        if self.column_resize.take().is_none() {
            return false;
        }
        self.status = "file column width updated".to_string();
        true
    }
}

impl TransferQueueState {
    fn new(
        tx: mpsc::Sender<TransferJobResult>,
        rx: mpsc::Receiver<TransferJobResult>,
        focus: FocusHandle,
        job_delete_focus: FocusHandle,
    ) -> Self {
        Self {
            tx,
            rx,
            jobs: Vec::new(),
            next_job_sequence: 0,
            selected_job_id: None,
            job_delete: None,
            job_menu: None,
            focus,
            job_delete_focus,
        }
    }

    fn focus(&self) -> &FocusHandle {
        &self.focus
    }

    fn delete_focus(&self) -> &FocusHandle {
        &self.job_delete_focus
    }

    fn jobs(&self) -> &[TransferJobState] {
        &self.jobs
    }

    fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    fn job(&self, job_id: &str) -> Option<&TransferJobState> {
        self.jobs.iter().find(|job| job.id == job_id)
    }

    fn job_mut(&mut self, job_id: &str) -> Option<&mut TransferJobState> {
        self.jobs.iter_mut().find(|job| job.id == job_id)
    }

    fn visit_jobs_mut(&mut self, visit: impl FnMut(&mut TransferJobState)) {
        self.jobs.iter_mut().for_each(visit);
    }

    fn enqueue(&mut self, job: TransferJobState) {
        self.jobs.push(job);
    }

    fn event_sender(&self) -> mpsc::Sender<TransferJobResult> {
        self.tx.clone()
    }

    fn try_recv_event(&self) -> Result<TransferJobResult, mpsc::TryRecvError> {
        self.rx.try_recv()
    }

    fn remove_job(&mut self, job_id: &str) -> bool {
        let before = self.jobs.len();
        self.jobs.retain(|job| job.id != job_id);
        let removed = self.jobs.len() != before;
        if removed {
            self.clear_job_interaction(job_id);
        }
        removed
    }

    fn take_job(&mut self, job_id: &str) -> Option<(usize, TransferJobState)> {
        let index = self.jobs.iter().position(|job| job.id == job_id)?;
        Some((index, self.jobs.remove(index)))
    }

    fn restore_job(&mut self, queued: (usize, TransferJobState)) {
        let (index, job) = queued;
        self.jobs.insert(index.min(self.jobs.len()), job);
    }

    fn next_job_id(&mut self, prefix: &str) -> String {
        self.next_job_sequence = self.next_job_sequence.max(self.jobs.len() as u64) + 1;
        format!("{prefix}-{}", self.next_job_sequence)
    }

    fn selected_job_id(&self) -> Option<&str> {
        self.selected_job_id.as_deref()
    }

    fn select_job(&mut self, job_id: &str) -> bool {
        if self.job(job_id).is_none() {
            return false;
        }
        self.selected_job_id = Some(job_id.to_string());
        true
    }

    fn selected_or_latest_visible_job_id(&self, session_id: Option<&str>) -> Option<String> {
        self.selected_job_id
            .as_ref()
            .filter(|job_id| {
                self.job(job_id)
                    .is_some_and(|job| job.is_visible_for_session(session_id))
            })
            .cloned()
            .or_else(|| {
                self.jobs
                    .iter()
                    .rev()
                    .find(|job| job.is_visible_for_session(session_id))
                    .map(|job| job.id.clone())
            })
    }

    fn job_delete(&self) -> Option<&TransferJobDeleteState> {
        self.job_delete.as_ref()
    }

    fn request_job_delete(&mut self, job_id: &str, title: String) -> bool {
        if self.job(job_id).is_none() {
            return false;
        }
        self.selected_job_id = Some(job_id.to_string());
        self.job_delete = Some(TransferJobDeleteState {
            job_id: job_id.to_string(),
            title,
        });
        true
    }

    fn confirm_job_delete(&mut self) -> Option<(String, bool)> {
        let state = self.job_delete.take()?;
        let removed = self.remove_job(&state.job_id);
        Some((state.job_id, removed))
    }

    fn cancel_job_delete(&mut self) {
        self.job_delete = None;
    }

    fn job_menu(&self) -> Option<&TransferJobMenuState> {
        self.job_menu.as_ref()
    }

    fn open_job_menu(&mut self, job_id: &str, x: Pixels, y: Pixels) -> bool {
        if !self.select_job(job_id) {
            self.job_menu = None;
            return false;
        }
        self.job_menu = Some(TransferJobMenuState {
            job_id: job_id.to_string(),
            x,
            y,
        });
        true
    }

    fn close_job_menu(&mut self) {
        self.job_menu = None;
    }

    fn reset_interaction(&mut self) {
        self.selected_job_id = None;
        self.job_delete = None;
        self.job_menu = None;
    }

    fn clear_job_interaction(&mut self, job_id: &str) {
        if self.selected_job_id.as_deref() == Some(job_id) {
            self.selected_job_id = None;
        }
        if self
            .job_menu
            .as_ref()
            .is_some_and(|menu| menu.job_id == job_id)
        {
            self.job_menu = None;
        }
        if self
            .job_delete
            .as_ref()
            .is_some_and(|delete| delete.job_id == job_id)
        {
            self.job_delete = None;
        }
    }

    fn prune_missing_interaction(&mut self) {
        let selected_missing = self
            .selected_job_id
            .as_deref()
            .is_some_and(|job_id| self.job(job_id).is_none());
        let menu_missing = self
            .job_menu
            .as_ref()
            .is_some_and(|menu| self.job(&menu.job_id).is_none());
        let delete_missing = self
            .job_delete
            .as_ref()
            .is_some_and(|delete| self.job(&delete.job_id).is_none());
        if selected_missing {
            self.selected_job_id = None;
        }
        if menu_missing {
            self.job_menu = None;
        }
        if delete_missing {
            self.job_delete = None;
        }
    }

    fn can_delete_job(&self, job_id: &str, session_id: Option<&str>) -> bool {
        self.job(job_id).is_some_and(|job| {
            job.is_visible_for_session(session_id)
                && !matches!(
                    job.status,
                    TransferJobStatus::Running
                        | TransferJobStatus::Paused
                        | TransferJobStatus::Cancelling
                )
        })
    }

    fn pause_visible_jobs(&mut self, session_id: Option<&str>) -> usize {
        let mut changed = 0;
        for job in &mut self.jobs {
            if job.is_visible_for_session(session_id)
                && job.status == TransferJobStatus::Running
                && let Some(control) = job.control.as_ref()
            {
                control.pause();
                job.status = TransferJobStatus::Paused;
                job.detail = "Paused".to_string();
                changed += 1;
            }
        }
        changed
    }

    fn resume_visible_jobs(&mut self, session_id: Option<&str>) -> usize {
        let mut changed = 0;
        for job in &mut self.jobs {
            if job.is_visible_for_session(session_id)
                && job.status == TransferJobStatus::Paused
                && let Some(control) = job.control.as_ref()
            {
                control.resume();
                job.status = TransferJobStatus::Running;
                job.detail = "Resuming".to_string();
                changed += 1;
            }
        }
        changed
    }

    fn cancel_visible_jobs(&mut self, session_id: Option<&str>) -> usize {
        let mut changed = 0;
        for job in &mut self.jobs {
            if job.is_visible_for_session(session_id)
                && matches!(
                    job.status,
                    TransferJobStatus::Running | TransferJobStatus::Paused
                )
                && let Some(control) = job.control.as_ref()
            {
                control.cancel();
                job.status = TransferJobStatus::Cancelling;
                job.detail = "Cancelling".to_string();
                changed += 1;
            }
        }
        changed
    }

    fn clear_completed_jobs(&mut self, session_id: Option<&str>) -> usize {
        let before = self.jobs.len();
        self.jobs.retain(|job| {
            !job.is_visible_for_session(session_id) || job.status != TransferJobStatus::Completed
        });
        let removed = before.saturating_sub(self.jobs.len());
        self.prune_missing_interaction();
        removed
    }

    fn clear_stopped_jobs(&mut self, session_id: Option<&str>) -> usize {
        let before = self.jobs.len();
        self.jobs.retain(|job| {
            !job.is_visible_for_session(session_id)
                || matches!(
                    job.status,
                    TransferJobStatus::Running
                        | TransferJobStatus::Paused
                        | TransferJobStatus::Cancelling
                )
        });
        let removed = before.saturating_sub(self.jobs.len());
        self.prune_missing_interaction();
        removed
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::sync::mpsc;

    use gpui::{TestAppContext, px};
    use nyaterm_transport::{
        SftpDuplicatePolicy, SftpFileEntry, SftpFileProperties, SftpFileType, SftpTransferControl,
        SftpWriteTextResult,
    };

    use crate::models::{
        TransferBrowserNavigationSnapshot, TransferBrowserSessionCacheState, TransferEditorField,
        TransferEditorState, TransferExternalSyncPromptState, TransferJobEvent, TransferJobKind,
        TransferJobResult, TransferJobState, TransferJobStatus, TransferNewFolderState,
        TransferPathPromptKind, TransferPropertiesField, TransferPropertiesState,
        TransferRenameState,
    };

    use super::{
        TransferEditorCloseAfterSave, TransferEditorCloseOutcome, TransferEditorSaveOutcome,
        TransferFeatureFocus, TransferFeatureState, TransferPanelState, TransferPathState,
        TransferQueueState,
    };

    fn transfer_focus(cx: &TestAppContext) -> TransferFeatureFocus {
        cx.update(|cx| TransferFeatureFocus {
            panel: cx.focus_handle(),
            queue: cx.focus_handle(),
            job_delete: cx.focus_handle(),
            download_path: cx.focus_handle(),
            browser: cx.focus_handle(),
            rename: cx.focus_handle(),
            move_to: cx.focus_handle(),
            delete: cx.focus_handle(),
            new_folder: cx.focus_handle(),
            new_file: cx.focus_handle(),
            new_symlink: cx.focus_handle(),
            properties: cx.focus_handle(),
            unknown_file: cx.focus_handle(),
            editor: cx.focus_handle(),
            default_editor: cx.focus_handle(),
            external_sync: cx.focus_handle(),
        })
    }

    fn transfer_state(cx: &TestAppContext) -> TransferFeatureState {
        TransferFeatureState::new(
            ".".to_string(),
            String::new(),
            SftpDuplicatePolicy::Ask,
            180.,
            transfer_focus(cx),
        )
    }

    fn file_entry(path: &str) -> SftpFileEntry {
        SftpFileEntry {
            name: path.rsplit('/').next().unwrap_or(path).to_string(),
            path: path.to_string(),
            file_type: SftpFileType::File,
            size: Some(12),
            permissions: Some(0o640),
            owner: "owner".to_string(),
            group: "group".to_string(),
            modified_at: Some(1),
        }
    }

    fn file_properties(path: &str) -> SftpFileProperties {
        SftpFileProperties {
            name: path.rsplit('/').next().unwrap_or(path).to_string(),
            path: path.to_string(),
            file_type: SftpFileType::File,
            size: Some(12),
            permissions: Some(0o600),
            permissions_symbolic: "rw-------".to_string(),
            owner: "updated-owner".to_string(),
            group: "updated-group".to_string(),
            uid: Some(1000),
            gid: Some(1000),
            modified_at: Some(2),
            accessed_at: Some(3),
        }
    }

    #[test]
    fn browser_history_discards_the_forward_branch_and_tracks_visits() {
        let cx = TestAppContext::single();
        let mut transfer = transfer_state(&cx);
        transfer.browser.history =
            VecDeque::from(["/three".to_string(), "/two".to_string(), "/one".to_string()]);
        transfer.browser.history_index = 1;

        transfer.record_browser_history("/four".to_string());

        assert_eq!(
            transfer.browser.history,
            VecDeque::from(["/four".to_string(), "/two".to_string(), "/one".to_string(),])
        );
        assert_eq!(transfer.browser.history_index, 0);
        assert_eq!(
            transfer.browser.visited_history.front().map(String::as_str),
            Some("/four")
        );
    }

    #[test]
    fn browser_session_restore_clamps_history_and_clears_interaction() {
        let cx = TestAppContext::single();
        let mut transfer = transfer_state(&cx);
        transfer.select_browser_entry("/stale.txt".to_string());
        assert!(
            transfer
                .schedule_browser_pending_rename("/stale.txt")
                .is_some()
        );
        transfer.store_browser_session_cache(
            "session-a".to_string(),
            TransferBrowserSessionCacheState {
                entries: vec![file_entry("/srv/current.txt")],
                current_path: "/srv".to_string(),
                home_dir: "/home/test".to_string(),
                history: VecDeque::from(["/srv".to_string()]),
                history_index: 99,
                visited_history: VecDeque::from(["/srv".to_string()]),
            },
        );

        assert_eq!(
            transfer.restore_browser_session_cache("session-a"),
            Some("/srv".to_string())
        );
        let browser = transfer.browser_view();
        assert_eq!(browser.path.as_str(), "/srv");
        assert_eq!(browser.history_index, 0);
        assert!(browser.selected_remote_paths.is_empty());
        assert!(browser.pending_rename.is_none());
        assert_eq!(browser.entries.len(), 1);
    }

    #[test]
    fn browser_navigation_restores_the_stable_pending_snapshot() {
        let cx = TestAppContext::single();
        let mut transfer = transfer_state(&cx);
        transfer.browser.path = "/optimistic".to_string();
        transfer
            .browser
            .navigation_jobs
            .insert("session-a".to_string(), "list-1".to_string());
        let stable = TransferBrowserNavigationSnapshot {
            remote_path: "/stable".to_string(),
            browser_path: "/stable".to_string(),
            entries: vec![file_entry("/stable/file.txt")],
            loading: false,
            error: None,
            status: "stable".to_string(),
            history: VecDeque::from(["/stable".to_string()]),
            history_index: 0,
            visited_history: VecDeque::from(["/stable".to_string()]),
            selected_path: None,
            selected_paths: Default::default(),
            list_offset: 3,
        };
        transfer
            .browser
            .pending_navigations
            .insert("list-1".to_string(), stable.clone());

        let rollback = transfer.prepare_browser_navigation("session-a", "/optimistic".to_string());

        assert_eq!(rollback.browser_path, "/stable");
        assert_eq!(transfer.browser.path, "/stable");
        assert_eq!(transfer.browser.list_offset, 3);
        assert!(!transfer.browser.navigation_jobs.contains_key("session-a"));
        assert!(!transfer.browser.pending_navigations.contains_key("list-1"));
    }

    #[test]
    fn browser_selection_replacement_preserves_the_explicit_active_path() {
        let cx = TestAppContext::single();
        let mut transfer = transfer_state(&cx);
        transfer.select_browser_entry("/active".to_string());

        let selected = ["/base".to_string()].into_iter().collect();
        let selected_count =
            transfer.replace_browser_selection(selected, Some("/active".to_string()));

        assert_eq!(selected_count, 1);
        assert_eq!(
            transfer.browser.selected_remote_path.as_deref(),
            Some("/active")
        );
        assert_eq!(
            transfer.browser.selected_remote_paths,
            ["/base".to_string()].into_iter().collect()
        );
    }

    fn transfer_queue(cx: &TestAppContext) -> TransferQueueState {
        let (tx, rx) = mpsc::channel();
        let (focus, delete_focus) = cx.update(|cx| (cx.focus_handle(), cx.focus_handle()));
        TransferQueueState::new(tx, rx, focus, delete_focus)
    }

    fn transfer_job(
        id: &str,
        session_id: &str,
        status: TransferJobStatus,
        controlled: bool,
    ) -> TransferJobState {
        TransferJobState {
            id: id.to_string(),
            session_id: Some(session_id.to_string()),
            kind: TransferJobKind::Download {
                remote_path: format!("/remote/{id}"),
                local_path: PathBuf::from(format!("/local/{id}")),
            },
            status,
            detail: String::new(),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: controlled.then(SftpTransferControl::new),
        }
    }

    fn external_sync_prompt(
        session_id: Option<&str>,
        job_id: &str,
    ) -> TransferExternalSyncPromptState {
        TransferExternalSyncPromptState {
            session_id: session_id.map(str::to_string),
            job_id: job_id.to_string(),
            remote_path: format!("/remote/{job_id}.txt"),
            local_path: PathBuf::from(format!("/local/{job_id}.txt")),
        }
    }

    fn editor_tab(session_id: &str, remote_path: &str) -> TransferEditorState {
        TransferEditorState {
            id: TransferEditorState::tab_id(Some(session_id), remote_path),
            session_id: Some(session_id.to_string()),
            remote_path: remote_path.to_string(),
            name: remote_path.rsplit('/').next().unwrap().to_string(),
            content: String::new(),
            search_query: String::new(),
            active_match: 0,
            base_size: Some(0),
            base_modified_at: Some(1),
            loading: false,
            saving: false,
            dirty: false,
            conflict: false,
            close_after_save: false,
            reload_confirm: false,
            error: None,
            focused_field: TransferEditorField::Content,
        }
    }

    #[test]
    fn transfer_paths_own_endpoints_policy_and_prompt_admission() {
        let mut paths = TransferPathState::new(
            "  ".to_string(),
            "/tmp/download".to_string(),
            SftpDuplicatePolicy::Ask,
        );

        assert_eq!(paths.normalized_remote_path(), ".");
        assert_eq!(paths.local_path(), "/tmp/download");
        assert_eq!(paths.duplicate_policy(), SftpDuplicatePolicy::Ask);

        paths.set_remote_path("/srv/files");
        paths.set_local_path("/tmp/upload");
        paths.set_duplicate_policy(SftpDuplicatePolicy::Overwrite);
        assert_eq!(paths.remote_path(), "/srv/files");
        assert_eq!(paths.normalized_remote_path(), "/srv/files");
        assert_eq!(paths.local_path(), "/tmp/upload");
        assert_eq!(paths.duplicate_policy(), SftpDuplicatePolicy::Overwrite);

        assert!(paths.begin_prompt(TransferPathPromptKind::UploadFile));
        assert!(!paths.begin_prompt(TransferPathPromptKind::DownloadDirectory));
        assert!(!paths.finish_prompt(TransferPathPromptKind::DownloadDirectory));
        assert!(!paths.begin_prompt(TransferPathPromptKind::UploadDirectory));
        assert!(paths.finish_prompt(TransferPathPromptKind::UploadFile));
        assert!(!paths.finish_prompt(TransferPathPromptKind::UploadFile));
    }

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

    #[test]
    fn transfer_file_ops_own_rename_focus_and_creation_options() {
        let cx = TestAppContext::single();
        let mut transfer = transfer_state(&cx);

        transfer.schedule_rename_focus();
        assert!(!transfer.rename_focus_is_pending());
        transfer.open_rename_dialog(TransferRenameState {
            old_path: "/srv/old".to_string(),
            initial_name: "old".to_string(),
            value: "old".to_string(),
        });
        transfer.schedule_rename_focus();
        assert!(transfer.rename_focus_is_pending());
        assert!(transfer.take_pending_rename_focus().is_some());
        assert!(!transfer.rename_focus_is_pending());
        transfer.schedule_rename_focus();
        transfer.close_rename_dialog();
        assert!(transfer.take_pending_rename_focus().is_none());

        transfer.open_new_folder_dialog(TransferNewFolderState {
            parent_path: "/srv".to_string(),
            value: String::new(),
            mode: 0o755,
            open_after_create: false,
        });
        assert!(transfer.set_new_folder_name("logs".to_string()));
        assert!(transfer.toggle_new_folder_open_after_create());
        assert!(transfer.toggle_new_folder_mode_bit(0o020));
        let folder = transfer
            .new_folder_dialog()
            .expect("new folder dialog should remain open");
        assert_eq!(folder.value, "logs");
        assert!(folder.open_after_create);
        assert_eq!(folder.mode, 0o775);
    }

    #[test]
    fn external_sync_prompts_are_filtered_by_session_and_window_admission() {
        let cx = TestAppContext::single();
        let mut transfer = transfer_state(&cx);
        transfer.insert_external_sync_prompt(
            "prompt-a".to_string(),
            external_sync_prompt(Some("session-a"), "job-a"),
        );
        transfer.insert_external_sync_prompt(
            "prompt-b".to_string(),
            external_sync_prompt(Some("session-b"), "job-b"),
        );

        assert_eq!(
            transfer
                .active_external_sync_prompt("session-a")
                .map(|(prompt_id, _)| prompt_id),
            Some("prompt-a".to_string())
        );
        assert!(transfer.begin_external_sync_window_open("prompt-a"));
        assert!(!transfer.begin_external_sync_window_open("prompt-a"));
        assert!(transfer.active_external_sync_prompt("session-a").is_none());
        assert_eq!(
            transfer
                .active_external_sync_prompt("session-b")
                .map(|(prompt_id, _)| prompt_id),
            Some("prompt-b".to_string())
        );
        assert!(!transfer.begin_external_sync_window_open("missing"));

        assert!(transfer.clear_external_sync_window_tracking("prompt-a"));
        assert!(!transfer.external_sync_window_open_is_pending("prompt-a"));
        assert_eq!(
            transfer
                .active_external_sync_prompt("session-a")
                .map(|(prompt_id, _)| prompt_id),
            Some("prompt-a".to_string())
        );

        assert!(transfer.begin_external_sync_window_open("prompt-b"));
        assert!(transfer.dismiss_external_sync_prompt("prompt-b"));
        assert!(transfer.external_sync_prompt("prompt-b").is_none());
        assert!(!transfer.external_sync_window_open_is_pending("prompt-b"));
        assert!(!transfer.dismiss_external_sync_prompt("prompt-b"));
    }

    #[test]
    fn external_sync_upload_resolution_cleans_tracking_and_records_policy() {
        let cx = TestAppContext::single();
        let mut transfer = transfer_state(&cx);
        transfer.insert_external_sync_prompt(
            "prompt-a".to_string(),
            external_sync_prompt(Some("session-a"), "job-a"),
        );
        assert!(transfer.begin_external_sync_window_open("prompt-a"));

        let prompt = transfer
            .take_external_sync_prompt_for_upload(
                "prompt-a",
                Some("/remote/job-a.txt\n/local/job-a.txt".to_string()),
            )
            .expect("known prompt should resolve for upload");

        assert_eq!(prompt.job_id, "job-a");
        assert!(transfer.external_sync_prompt("prompt-a").is_none());
        assert!(!transfer.external_sync_window_open_is_pending("prompt-a"));
        assert!(transfer.external_sync_always_uploads("/remote/job-a.txt\n/local/job-a.txt"));
        assert!(
            transfer
                .take_external_sync_prompt_for_upload("prompt-a", None)
                .is_none()
        );
    }

    #[test]
    fn external_sync_session_cleanup_preserves_other_sessions_and_policy() {
        let cx = TestAppContext::single();
        let mut transfer = transfer_state(&cx);
        transfer.insert_external_sync_prompt(
            "prompt-a-1".to_string(),
            external_sync_prompt(Some("session-a"), "job-a-1"),
        );
        transfer.insert_external_sync_prompt(
            "prompt-a-2".to_string(),
            external_sync_prompt(Some("session-a"), "job-a-2"),
        );
        transfer.insert_external_sync_prompt(
            "prompt-b".to_string(),
            external_sync_prompt(Some("session-b"), "job-b"),
        );
        transfer.insert_external_sync_prompt(
            "policy-source".to_string(),
            external_sync_prompt(None, "policy-source"),
        );
        transfer.take_external_sync_prompt_for_upload(
            "policy-source",
            Some("persistent-watch-key".to_string()),
        );
        assert!(transfer.begin_external_sync_window_open("prompt-a-1"));
        assert!(transfer.begin_external_sync_window_open("prompt-b"));

        assert_eq!(transfer.clear_external_sync_for_session("session-a"), 2);
        assert!(transfer.external_sync_prompt("prompt-a-1").is_none());
        assert!(transfer.external_sync_prompt("prompt-a-2").is_none());
        assert!(!transfer.external_sync_window_open_is_pending("prompt-a-1"));
        assert!(transfer.external_sync_prompt("prompt-b").is_some());
        assert!(transfer.external_sync_window_open_is_pending("prompt-b"));
        assert!(transfer.external_sync_always_uploads("persistent-watch-key"));
    }

    #[test]
    fn transfer_editor_owns_tab_activation_and_close_confirmation() {
        let cx = TestAppContext::single();
        let mut transfer = transfer_state(&cx);
        let tab_a = editor_tab("session-a", "/srv/a.txt");
        let tab_a_id = tab_a.id.clone();
        let tab_b = editor_tab("session-b", "/srv/b.txt");
        let tab_b_id = tab_b.id.clone();

        assert!(!transfer.open_editor_tab(tab_a));
        assert!(!transfer.open_editor_tab(tab_b));
        assert_eq!(
            transfer.active_editor_tab().map(|tab| tab.id.as_str()),
            Some(tab_b_id.as_str())
        );
        assert!(transfer.activate_editor_tab(&tab_a_id));
        transfer.active_editor_tab_mut().unwrap().dirty = true;

        assert_eq!(
            transfer.request_editor_tab_close(&tab_a_id),
            TransferEditorCloseOutcome::ConfirmationRequired
        );
        let workspace = transfer.editor_workspace().unwrap();
        assert!(workspace.close_confirm);
        assert_eq!(
            workspace.pending_close_tab_id.as_deref(),
            Some(tab_a_id.as_str())
        );
        assert!(transfer.cancel_editor_close());
        assert!(!transfer.editor_close_confirmation_is_open());

        transfer.active_editor_tab_mut().unwrap().dirty = false;
        assert_eq!(
            transfer.request_editor_tab_close(&tab_a_id),
            TransferEditorCloseOutcome::Closed
        );
        assert_eq!(
            transfer.active_editor_tab().map(|tab| tab.id.as_str()),
            Some(tab_b_id.as_str())
        );
    }

    #[test]
    fn transfer_editor_save_completion_closes_requested_tab_atomically() {
        let cx = TestAppContext::single();
        let mut transfer = transfer_state(&cx);
        let tab = editor_tab("session-a", "/srv/a.txt");
        let tab_id = tab.id.clone();
        transfer.open_editor_tab(tab);
        assert!(transfer.sync_editor_content(&tab_id, "updated".to_string()));
        assert_eq!(
            transfer.request_editor_tab_close(&tab_id),
            TransferEditorCloseOutcome::ConfirmationRequired
        );
        assert_eq!(
            transfer.prepare_editor_close_after_save(),
            TransferEditorCloseAfterSave::Ready(tab_id.clone())
        );
        assert!(transfer.begin_editor_tab_save(&tab_id));

        assert_eq!(
            transfer.complete_editor_save(
                Some("session-a"),
                "/srv/a.txt",
                SftpWriteTextResult::Saved {
                    modified_at: 2,
                    size: 7,
                },
            ),
            Some(TransferEditorSaveOutcome::SavedAndClosed)
        );
        assert!(!transfer.editor_has_workspace());
    }

    #[test]
    fn transfer_editor_save_all_waits_for_every_dirty_tab() {
        let cx = TestAppContext::single();
        let mut transfer = transfer_state(&cx);
        let tab_a = editor_tab("session-a", "/srv/a.txt");
        let tab_a_id = tab_a.id.clone();
        let tab_b = editor_tab("session-b", "/srv/b.txt");
        let tab_b_id = tab_b.id.clone();
        transfer.open_editor_tab(tab_a);
        transfer.open_editor_tab(tab_b);
        assert!(transfer.sync_editor_content(&tab_a_id, "updated a".to_string()));
        assert!(transfer.sync_editor_content(&tab_b_id, "updated b".to_string()));
        assert_eq!(
            transfer.request_editor_close(),
            TransferEditorCloseOutcome::ConfirmationRequired
        );
        assert_eq!(
            transfer.prepare_editor_close_after_save(),
            TransferEditorCloseAfterSave::All
        );
        assert!(transfer.begin_editor_tab_save(&tab_a_id));
        assert!(transfer.begin_editor_tab_save(&tab_b_id));

        assert_eq!(
            transfer.complete_editor_save(
                Some("session-a"),
                "/srv/a.txt",
                SftpWriteTextResult::Saved {
                    modified_at: 2,
                    size: 9,
                },
            ),
            Some(TransferEditorSaveOutcome::Saved)
        );
        assert!(transfer.editor_has_workspace());
        assert_eq!(
            transfer.complete_editor_save(
                Some("session-b"),
                "/srv/b.txt",
                SftpWriteTextResult::Saved {
                    modified_at: 3,
                    size: 9,
                },
            ),
            Some(TransferEditorSaveOutcome::SavedAndClosed)
        );
        assert!(!transfer.editor_has_workspace());
    }

    #[test]
    fn transfer_editor_conflict_and_session_cleanup_preserve_other_tabs() {
        let cx = TestAppContext::single();
        let mut transfer = transfer_state(&cx);
        let tab_a = editor_tab("session-a", "/srv/a.txt");
        let tab_a_id = tab_a.id.clone();
        let tab_b = editor_tab("session-b", "/srv/b.txt");
        let tab_b_id = tab_b.id.clone();
        transfer.open_editor_tab(tab_a);
        transfer.open_editor_tab(tab_b);
        assert!(transfer.activate_editor_tab(&tab_a_id));
        assert!(transfer.begin_editor_tab_save(&tab_a_id));
        assert_eq!(
            transfer.complete_editor_save(
                Some("session-a"),
                "/srv/a.txt",
                SftpWriteTextResult::Conflict {
                    modified_at: 3,
                    size: 9,
                },
            ),
            Some(TransferEditorSaveOutcome::Conflict)
        );
        assert!(transfer.editor_close_confirmation_is_open());

        assert_eq!(transfer.remove_editor_tabs_for_session("session-a"), 1);
        assert_eq!(
            transfer.active_editor_tab().map(|tab| tab.id.as_str()),
            Some(tab_b_id.as_str())
        );
        assert!(!transfer.editor_close_confirmation_is_open());
        assert!(transfer.begin_editor_window_open());
        assert!(!transfer.begin_editor_window_open());
        assert!(transfer.clear_editor_window_tracking());
        assert_eq!(transfer.remove_editor_tabs_for_session("session-b"), 1);
        assert!(!transfer.editor_has_workspace());
    }

    #[test]
    fn transfer_properties_ignore_stale_results_and_close_for_the_owner_session() {
        let cx = TestAppContext::single();
        let mut transfer = transfer_state(&cx);
        transfer.open_properties_dialog(TransferPropertiesState {
            session_id: Some("session-a".to_string()),
            entry: file_entry("/srv/file.txt"),
            properties: None,
            mode_value: "0640".to_string(),
            owner_value: String::new(),
            group_value: String::new(),
            recursive: false,
            saving: false,
            error: None,
            focused_field: TransferPropertiesField::Mode,
        });

        assert!(!transfer.complete_properties_load(
            Some("session-b"),
            "/srv/file.txt",
            file_properties("/srv/file.txt"),
            "0600".to_string(),
            "updated-owner".to_string(),
            "updated-group".to_string(),
        ));
        assert!(
            transfer
                .properties_dialog()
                .is_some_and(|state| state.properties.is_none())
        );

        assert!(transfer.complete_properties_load(
            Some("session-a"),
            "/srv/file.txt",
            file_properties("/srv/file.txt"),
            "0600".to_string(),
            "updated-owner".to_string(),
            "updated-group".to_string(),
        ));
        assert_eq!(
            transfer
                .properties_dialog()
                .map(|state| state.owner_value.as_str()),
            Some("updated-owner")
        );
        assert!(transfer.begin_properties_save());
        assert!(!transfer.fail_properties_operation(
            Some("session-b"),
            "/srv/file.txt",
            "stale".to_string(),
        ));
        assert!(
            transfer
                .properties_dialog()
                .is_some_and(|state| state.saving && state.error.is_none())
        );
        assert!(transfer.fail_properties_operation(
            Some("session-a"),
            "/srv/file.txt",
            "denied".to_string(),
        ));
        assert!(
            transfer
                .properties_dialog()
                .is_some_and(|state| { !state.saving && state.error.as_deref() == Some("denied") })
        );
        assert!(!transfer.close_properties_dialog_for_session("session-b"));
        assert!(transfer.close_properties_dialog_for_session("session-a"));
        assert!(transfer.properties_dialog().is_none());

        transfer.open_properties_dialog(TransferPropertiesState {
            session_id: Some("session-a".to_string()),
            entry: file_entry("/srv/file.txt"),
            properties: Some(file_properties("/srv/file.txt")),
            mode_value: "0600".to_string(),
            owner_value: "updated-owner".to_string(),
            group_value: "updated-group".to_string(),
            recursive: false,
            saving: true,
            error: None,
            focused_field: TransferPropertiesField::Mode,
        });
        assert!(!transfer.complete_properties_update(
            Some("session-a"),
            "/srv/other.txt",
            file_properties("/srv/other.txt"),
        ));
        assert!(transfer.complete_properties_update(
            Some("session-a"),
            "/srv/file.txt",
            file_properties("/srv/file.txt"),
        ));
        assert!(transfer.properties_dialog().is_none());
    }

    #[test]
    fn transfer_queue_owns_admission_events_and_delete_interaction() {
        let cx = TestAppContext::single();
        let mut queue = transfer_queue(&cx);
        queue.enqueue(transfer_job(
            "job-1",
            "session-a",
            TransferJobStatus::Completed,
            false,
        ));

        assert_eq!(queue.next_job_id("download"), "download-2");
        assert!(queue.select_job("job-1"));
        assert!(queue.open_job_menu("job-1", px(12.), px(24.)));
        assert_eq!(queue.selected_job_id(), Some("job-1"));
        assert_eq!(
            queue.job_menu().map(|menu| menu.job_id.as_str()),
            Some("job-1")
        );
        assert!(queue.can_delete_job("job-1", Some("session-a")));

        assert!(queue.request_job_delete("job-1", "Download".to_string()));
        assert_eq!(
            queue.confirm_job_delete(),
            Some(("job-1".to_string(), true))
        );
        assert!(queue.is_empty());
        assert_eq!(queue.selected_job_id(), None);
        assert_eq!(queue.next_job_id("download"), "download-3");

        let sender = queue.event_sender();
        sender
            .send(TransferJobResult {
                id: "missing-job".to_string(),
                event: TransferJobEvent::Started {
                    detail: "started".to_string(),
                },
            })
            .expect("queue receiver should remain connected");
        let event = queue
            .try_recv_event()
            .expect("queue should receive its typed event");
        assert_eq!(event.id, "missing-job");
        assert!(matches!(event.event, TransferJobEvent::Started { .. }));
    }

    #[test]
    fn transfer_queue_batches_are_scoped_to_the_visible_session() {
        let cx = TestAppContext::single();
        let mut queue = transfer_queue(&cx);
        queue.enqueue(transfer_job(
            "running-a",
            "session-a",
            TransferJobStatus::Running,
            true,
        ));
        queue.enqueue(transfer_job(
            "running-b",
            "session-b",
            TransferJobStatus::Running,
            true,
        ));
        queue.enqueue(transfer_job(
            "completed-a",
            "session-a",
            TransferJobStatus::Completed,
            false,
        ));
        assert!(queue.open_job_menu("completed-a", px(8.), px(8.)));
        assert!(queue.request_job_delete("completed-a", "Completed".to_string()));

        assert_eq!(queue.pause_visible_jobs(Some("session-a")), 1);
        assert_eq!(
            queue.job("running-a").map(|job| job.status),
            Some(TransferJobStatus::Paused)
        );
        assert_eq!(
            queue.job("running-b").map(|job| job.status),
            Some(TransferJobStatus::Running)
        );
        assert_eq!(queue.resume_visible_jobs(Some("session-a")), 1);
        assert_eq!(queue.cancel_visible_jobs(Some("session-a")), 1);
        assert_eq!(
            queue.job("running-a").map(|job| job.status),
            Some(TransferJobStatus::Cancelling)
        );
        assert_eq!(queue.clear_completed_jobs(Some("session-a")), 1);
        assert!(queue.job("completed-a").is_none());
        assert_eq!(queue.selected_job_id(), None);
        assert!(queue.job_menu().is_none());
        assert!(queue.job_delete().is_none());
        assert!(queue.job("running-b").is_some());
        assert_eq!(queue.clear_stopped_jobs(Some("session-b")), 0);
    }
}
