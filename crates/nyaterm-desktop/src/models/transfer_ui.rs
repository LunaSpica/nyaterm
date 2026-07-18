use gpui::Pixels;
use nyaterm_transport::{SftpFileEntry, SftpFileProperties};
use std::collections::HashSet;
use std::path::PathBuf;

use super::TransferBrowserSortColumn;

#[derive(Debug, Clone, Copy)]
pub(crate) struct TransferBrowserColumnResizeState {
    pub(crate) column: TransferBrowserSortColumn,
    pub(crate) start_x: Pixels,
    pub(crate) start_width: Pixels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransferBrowserDragSelectionState {
    pub(crate) anchor_path: String,
    pub(crate) base_selection: HashSet<String>,
    pub(crate) additive: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TransferBrowserContextMenuState {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) is_parent: bool,
    pub(crate) is_current_directory: bool,
    pub(crate) is_directory: bool,
    pub(crate) x: Pixels,
    pub(crate) y: Pixels,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TransferBrowserFavoritesMenuState {
    pub(crate) x: Pixels,
    pub(crate) y: Pixels,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TransferBrowserUploadMenuState {
    pub(crate) x: Pixels,
    pub(crate) y: Pixels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransferBrowserPendingRenameState {
    pub(crate) path: String,
    pub(crate) token: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TransferUnknownFileState {
    pub(crate) entry: SftpFileEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransferRenameState {
    pub(crate) old_path: String,
    pub(crate) initial_name: String,
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransferMoveState {
    pub(crate) old_path: String,
    pub(crate) name: String,
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransferDeleteState {
    pub(crate) remote_path: String,
    pub(crate) name: String,
    pub(crate) paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransferJobDeleteState {
    pub(crate) job_id: String,
    pub(crate) title: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TransferJobMenuState {
    pub(crate) job_id: String,
    pub(crate) x: Pixels,
    pub(crate) y: Pixels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransferNewFolderState {
    pub(crate) parent_path: String,
    pub(crate) value: String,
    pub(crate) mode: u32,
    pub(crate) open_after_create: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransferNewFileState {
    pub(crate) parent_path: String,
    pub(crate) value: String,
    pub(crate) mode: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferSymlinkField {
    Name,
    Target,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransferNewSymlinkState {
    pub(crate) parent_path: String,
    pub(crate) name: String,
    pub(crate) target: String,
    pub(crate) focused_field: TransferSymlinkField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferPropertiesField {
    Mode,
    Owner,
    Group,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransferPropertiesState {
    pub(crate) session_id: Option<String>,
    pub(crate) entry: SftpFileEntry,
    pub(crate) properties: Option<SftpFileProperties>,
    pub(crate) mode_value: String,
    pub(crate) owner_value: String,
    pub(crate) group_value: String,
    pub(crate) recursive: bool,
    pub(crate) saving: bool,
    pub(crate) error: Option<String>,
    pub(crate) focused_field: TransferPropertiesField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransferEditorState {
    pub(crate) session_id: Option<String>,
    pub(crate) remote_path: String,
    pub(crate) name: String,
    pub(crate) content: String,
    pub(crate) search_query: String,
    pub(crate) active_match: usize,
    pub(crate) base_size: Option<u64>,
    pub(crate) base_modified_at: Option<u64>,
    pub(crate) loading: bool,
    pub(crate) saving: bool,
    pub(crate) dirty: bool,
    pub(crate) conflict: bool,
    pub(crate) close_confirm: bool,
    pub(crate) close_after_save: bool,
    pub(crate) reload_confirm: bool,
    pub(crate) error: Option<String>,
    pub(crate) focused_field: TransferEditorField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransferExternalSyncPromptState {
    pub(crate) session_id: Option<String>,
    pub(crate) job_id: String,
    pub(crate) remote_path: String,
    pub(crate) local_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferEditorField {
    Content,
    Search,
}
