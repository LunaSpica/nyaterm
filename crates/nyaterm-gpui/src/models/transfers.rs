use gpui::{Pixels, px};
use nyaterm_transport::{
    SftpFileEntry, SftpFileProperties, SftpRemoteTextFile,
    SftpTransferControl, SftpTransferProgress, SftpTransferSummary, SftpWriteTextResult,
};
use std::collections::VecDeque;
use std::path::PathBuf;


#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TransferJobKind {
    ListDir {
        remote_path: String,
        select_after: Option<String>,
    },
    ResolveHome,
    SyncCwd,
    Download {
        remote_path: String,
        local_path: PathBuf,
    },
    Upload {
        local_path: PathBuf,
        remote_path: String,
    },
    Rename {
        old_path: String,
        new_path: String,
        parent_path: String,
    },
    Move {
        old_path: String,
        new_path: String,
        parent_path: String,
    },
    Delete {
        remote_path: String,
        parent_path: String,
    },
    Mkdir {
        remote_path: String,
        parent_path: String,
    },
    CreateFile {
        remote_path: String,
        parent_path: String,
    },
    Symlink {
        link_path: String,
        target_path: String,
        parent_path: String,
    },
    LoadProperties {
        remote_path: String,
    },
    UpdateProperties {
        remote_path: String,
        parent_path: String,
    },
    LoadEditor {
        remote_path: String,
    },
    SaveEditor {
        remote_path: String,
    },
    OpenExternal {
        remote_path: String,
        local_path: PathBuf,
    },
    AiFileAction {
        remote_path: String,
        action_id: String,
        action_name: String,
    },
    /// In-band ZMODEM upload (local files -> remote `rz`).
    ZmodemUpload {
        session_id: String,
        file_name: String,
    },
    /// In-band ZMODEM download (remote `sz` -> local directory).
    ZmodemDownload {
        session_id: String,
        file_name: String,
    },
    /// Pre-upload SFTP name conflict probe before remote `rz` (Tauri parity).
    ZmodemConflictProbe {
        session_id: String,
        remote_dir: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferJobStatus {
    Running,
    Paused,
    Cancelling,
    Cancelled,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub(crate) struct TransferJobState {
    pub(crate) id: String,
    pub(crate) kind: TransferJobKind,
    pub(crate) status: TransferJobStatus,
    pub(crate) detail: String,
    pub(crate) entries: Vec<SftpFileEntry>,
    pub(crate) summary: Option<SftpTransferSummary>,
    pub(crate) progress: Option<SftpTransferProgress>,
    pub(crate) control: Option<SftpTransferControl>,
}

#[derive(Debug)]
pub(crate) struct TransferJobResult {
    pub(crate) id: String,
    pub(crate) event: TransferJobEvent,
}

#[derive(Debug)]
pub(crate) enum TransferJobEvent {
    Started {
        detail: String,
    },
    ExternalModified {
        remote_path: String,
        local_path: PathBuf,
    },
    Progress(SftpTransferProgress),
    Finished(Result<TransferJobOutput, String>),
}

#[derive(Debug)]
pub(crate) enum TransferJobOutput {
    Entries(Vec<SftpFileEntry>),
    HomeDir(String),
    CwdSynced {
        remote_path: String,
        entries: Vec<SftpFileEntry>,
    },
    Summary(SftpTransferSummary),
    Uploaded {
        summary: SftpTransferSummary,
        parent_path: String,
        entries: Vec<SftpFileEntry>,
    },
    Renamed {
        old_path: String,
        new_path: String,
        parent_path: String,
        entries: Vec<SftpFileEntry>,
    },
    Moved {
        old_path: String,
        new_path: String,
        parent_path: String,
        entries: Vec<SftpFileEntry>,
    },
    Deleted {
        remote_path: String,
        parent_path: String,
        entries: Vec<SftpFileEntry>,
    },
    CreatedDirectory {
        remote_path: String,
        parent_path: String,
        entries: Vec<SftpFileEntry>,
        open_after_create: bool,
    },
    CreatedFile {
        remote_path: String,
        parent_path: String,
        entries: Vec<SftpFileEntry>,
    },
    CreatedSymlink {
        link_path: String,
        target_path: String,
        parent_path: String,
        entries: Vec<SftpFileEntry>,
    },
    PropertiesLoaded {
        remote_path: String,
        properties: SftpFileProperties,
    },
    PropertiesUpdated {
        remote_path: String,
        parent_path: String,
        properties: SftpFileProperties,
        entries: Vec<SftpFileEntry>,
    },
    EditorLoaded {
        remote_path: String,
        file: SftpRemoteTextFile,
    },
    EditorSaved {
        remote_path: String,
        result: SftpWriteTextResult,
    },
    ExternalOpened {
        remote_path: String,
        local_path: PathBuf,
    },
    AiFileActionLoaded {
        remote_path: String,
        action_id: String,
        action_name: String,
        prompt: String,
        file: SftpRemoteTextFile,
    },

    ZmodemProbeReady {
        session_id: String,
        files: Vec<PathBuf>,
        probe_skipped: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferInputField {
    Remote,
    Local,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TransferBrowserSessionCacheState {
    pub(crate) entries: Vec<SftpFileEntry>,
    pub(crate) current_path: String,
    pub(crate) home_dir: String,
    pub(crate) history: VecDeque<String>,
    pub(crate) history_index: usize,
    pub(crate) visited_history: VecDeque<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferBrowserSortColumn {
    Name,
    Modified,
    Size,
    Permissions,
    Owner,
    Group,
}

impl TransferBrowserSortColumn {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Modified => "Modified",
            Self::Size => "Size",
            Self::Permissions => "Perms",
            Self::Owner => "Owner",
            Self::Group => "Group",
        }
    }

    pub(crate) fn default_direction(self) -> TransferBrowserSortDirection {
        match self {
            Self::Name | Self::Permissions | Self::Owner | Self::Group => {
                TransferBrowserSortDirection::Ascending
            }
            Self::Size | Self::Modified => TransferBrowserSortDirection::Descending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferBrowserSortDirection {
    Ascending,
    Descending,
}

impl TransferBrowserSortDirection {
    pub(crate) fn marker(self) -> &'static str {
        match self {
            Self::Ascending => "up",
            Self::Descending => "down",
        }
    }

    pub(crate) fn toggled(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TransferBrowserColumnWidths {
    pub(crate) name: Pixels,
    pub(crate) modified: Pixels,
    pub(crate) size: Pixels,
    pub(crate) permissions: Pixels,
    pub(crate) owner: Pixels,
    pub(crate) group: Pixels,
}

impl Default for TransferBrowserColumnWidths {
    fn default() -> Self {
        Self {
            name: px(220.),
            modified: px(128.),
            size: px(80.),
            permissions: px(112.),
            owner: px(96.),
            group: px(96.),
        }
    }
}

impl TransferBrowserColumnWidths {
    pub(crate) fn get(self, column: TransferBrowserSortColumn) -> Pixels {
        match column {
            TransferBrowserSortColumn::Name => self.name,
            TransferBrowserSortColumn::Modified => self.modified,
            TransferBrowserSortColumn::Size => self.size,
            TransferBrowserSortColumn::Permissions => self.permissions,
            TransferBrowserSortColumn::Owner => self.owner,
            TransferBrowserSortColumn::Group => self.group,
        }
    }

    pub(crate) fn set(&mut self, column: TransferBrowserSortColumn, width: Pixels) {
        let width = if width < Self::min_width(column) {
            Self::min_width(column)
        } else {
            width
        };
        match column {
            TransferBrowserSortColumn::Name => self.name = width,
            TransferBrowserSortColumn::Modified => self.modified = width,
            TransferBrowserSortColumn::Size => self.size = width,
            TransferBrowserSortColumn::Permissions => self.permissions = width,
            TransferBrowserSortColumn::Owner => self.owner = width,
            TransferBrowserSortColumn::Group => self.group = width,
        }
    }

    pub(crate) fn min_width(column: TransferBrowserSortColumn) -> Pixels {
        match column {
            TransferBrowserSortColumn::Name => px(140.),
            TransferBrowserSortColumn::Modified => px(112.),
            TransferBrowserSortColumn::Size => px(72.),
            TransferBrowserSortColumn::Permissions => px(92.),
            TransferBrowserSortColumn::Owner => px(76.),
            TransferBrowserSortColumn::Group => px(76.),
        }
    }
}
