use gpui::{Pixels, px};
use nyaterm_domain::{
    AiAction, AiContext, AiExecutionProfile, ConfigBackupInfo, DiagnosticsExportInfo, QuickCommand,
    SavedConnection,
};
use nyaterm_session::{
    LocalSessionConfig, SerialSessionConfig, SftpFileEntry, SftpFileProperties, SftpRemoteTextFile,
    SftpTransferControl, SftpTransferProgress, SftpTransferSummary, SftpWriteTextResult,
    SshSessionConfig, TelnetSessionConfig,
};
use nyaterm_terminal::TerminalScreen;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

use super::terminal::{terminal_screen_from_output, trim_terminal_output};

pub(super) struct TerminalViewState {
    pub(super) output: String,
    pub(super) screen: TerminalScreen,
    pub(super) has_unread: bool,
}

impl TerminalViewState {
    pub(super) fn new() -> Self {
        Self {
            output: String::new(),
            screen: TerminalScreen::default(),
            has_unread: false,
        }
    }

    pub(super) fn from_output(output: String) -> Self {
        let screen = terminal_screen_from_output(&output);
        Self {
            output,
            screen,
            has_unread: false,
        }
    }

    pub(super) fn append_text(&mut self, text: &str) {
        self.output.push_str(text);
        self.screen.advance(text.as_bytes());
        trim_terminal_output(&mut self.output);
    }

    pub(super) fn append_bytes(&mut self, data: &[u8]) {
        self.screen.advance(data);
        self.output.push_str(&String::from_utf8_lossy(data));
        trim_terminal_output(&mut self.output);
    }

    pub(super) fn clear(&mut self) {
        self.output.clear();
        self.screen.clear();
        self.has_unread = false;
    }
}

#[derive(Clone)]
pub(super) struct SessionRuntimeMetadata {
    pub(super) ssh_config: Option<SshSessionConfig>,
    pub(super) ssh_multiplex_key: Option<String>,
    pub(super) source_connection_id: Option<String>,
    pub(super) ai_execution_profile: AiExecutionProfile,
    pub(super) launch_config: SessionLaunchConfig,
}

#[derive(Clone)]
pub(super) struct StartupCommandRequest {
    pub(super) command: String,
    pub(super) delay_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SyncInputGroup {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) color: u32,
    pub(super) session_ids: Vec<String>,
    pub(super) paused_session_ids: Vec<String>,
    pub(super) enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StartupCommandAction {
    Duplicate,
    Multiplex,
}

impl StartupCommandAction {
    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Duplicate => "Duplicate and Run Command",
            Self::Multiplex => "Multiplex and Run Command",
        }
    }

    pub(super) fn placeholder(self) -> &'static str {
        match self {
            Self::Duplicate => "Command to run after duplicate",
            Self::Multiplex => "Command to run after multiplex",
        }
    }

    pub(super) fn submit_label(self) -> &'static str {
        match self {
            Self::Duplicate => "Duplicate",
            Self::Multiplex => "Multiplex",
        }
    }

    pub(super) fn status_opened(self) -> &'static str {
        match self {
            Self::Duplicate => "duplicate and run command opened",
            Self::Multiplex => "multiplex and run command opened",
        }
    }

    pub(super) fn status_cancelled(self) -> &'static str {
        match self {
            Self::Duplicate => "duplicate and run command cancelled",
            Self::Multiplex => "multiplex and run command cancelled",
        }
    }
}

#[derive(Clone)]
pub(super) enum SessionLaunchConfig {
    Local(LocalSessionConfig),
    Ssh(SshSessionConfig),
    Telnet(TelnetSessionConfig),
    Serial(SerialSessionConfig),
}

#[derive(Clone)]
pub(super) enum QuickSwitchItem {
    Session {
        id: String,
        title: String,
        subtitle: String,
        active: bool,
        unread: bool,
    },
    Connection {
        connection: SavedConnection,
        title: String,
        subtitle: String,
    },
    Pending {
        title: String,
        subtitle: String,
    },
}

impl QuickSwitchItem {
    pub(super) fn id(&self) -> String {
        match self {
            Self::Session { id, .. } => format!("session:{id}"),
            Self::Connection { connection, .. } => format!("connection:{}", connection.id),
            Self::Pending { title, .. } => format!("pending:{title}"),
        }
    }

    pub(super) fn title(&self) -> &str {
        match self {
            Self::Session { title, .. }
            | Self::Connection { title, .. }
            | Self::Pending { title, .. } => title,
        }
    }

    pub(super) fn subtitle(&self) -> &str {
        match self {
            Self::Session { subtitle, .. }
            | Self::Connection { subtitle, .. }
            | Self::Pending { subtitle, .. } => subtitle,
        }
    }

    pub(super) fn search_text(&self) -> String {
        match self {
            Self::Session {
                id,
                title,
                subtitle,
                ..
            } => format!("{title} {subtitle} {id}"),
            Self::Connection {
                connection,
                title,
                subtitle,
            } => format!(
                "{} {} {} {}",
                title,
                subtitle,
                connection.description.clone().unwrap_or_default(),
                connection.endpoint()
            ),
            Self::Pending { title, subtitle } => format!("{title} {subtitle}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TerminalSearchMode {
    Buffer,
    History,
}

impl TerminalSearchMode {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Buffer => "Buffer",
            Self::History => "History",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct StoreStatus {
    pub(super) path: String,
    pub(super) message: String,
    pub(super) ready: bool,
}

#[derive(Debug, Clone)]
pub(super) struct MultiLinePasteDraft {
    pub(super) text: String,
}

impl MultiLinePasteDraft {
    pub(super) fn new(text: String) -> Self {
        Self { text }
    }

    pub(super) fn normalized_text(&self) -> String {
        normalize_paste_newlines(&self.text)
    }

    pub(super) fn line_count(&self) -> usize {
        count_paste_lines(&self.text)
    }

    pub(super) fn character_count(&self) -> usize {
        self.text.chars().count()
    }
}

pub(super) fn normalize_paste_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

pub(super) fn is_multi_line_paste(text: &str) -> bool {
    normalize_paste_newlines(text).contains('\n')
}

pub(super) fn count_paste_lines(text: &str) -> usize {
    normalize_paste_newlines(text).split('\n').count()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum NavItem {
    Workspace,
    Connections,
    Tunnels,
    Stats,
    Processes,
    Docker,
    Translation,
    Transfers,
    Settings,
    Migration,
}

impl NavItem {
    pub(super) fn label(self) -> &'static str {
        match self {
            NavItem::Workspace => "Workspace",
            NavItem::Connections => "Connections",
            NavItem::Tunnels => "Tunnels",
            NavItem::Stats => "Stats",
            NavItem::Processes => "Processes",
            NavItem::Docker => "Docker",
            NavItem::Translation => "Translation",
            NavItem::Transfers => "Transfers",
            NavItem::Settings => "Settings",
            NavItem::Migration => "Migration",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NetworkTab {
    Tunnels,
    Proxies,
}

impl NetworkTab {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Tunnels => "Tunnels",
            Self::Proxies => "Proxies",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NetworkDeleteConfirmState {
    pub(super) tab: NetworkTab,
    pub(super) id: String,
    pub(super) label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NetworkGroupEditorState {
    pub(super) tab: NetworkTab,
    pub(super) id: Option<String>,
    pub(super) name: String,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NetworkGroupDeleteConfirmState {
    pub(super) tab: NetworkTab,
    pub(super) id: String,
    pub(super) label: String,
    pub(super) item_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NetworkMovePickerState {
    pub(super) tab: NetworkTab,
    pub(super) id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NetworkTunnelEditorField {
    Name,
    ListenPort,
    TargetHost,
    TargetPort,
}

impl NetworkTunnelEditorField {
    pub(super) fn next(self, dynamic: bool) -> Self {
        match self {
            Self::Name => Self::ListenPort,
            Self::ListenPort if dynamic => Self::Name,
            Self::ListenPort => Self::TargetHost,
            Self::TargetHost => Self::TargetPort,
            Self::TargetPort => Self::Name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NetworkTunnelEditorState {
    pub(super) id: Option<String>,
    pub(super) is_open: bool,
    pub(super) name: String,
    pub(super) tunnel_type: String,
    pub(super) connection_id: Option<String>,
    pub(super) listen_port: String,
    pub(super) target_host: String,
    pub(super) target_port: String,
    pub(super) auto_open: bool,
    pub(super) bind_localhost: bool,
    pub(super) group_id: Option<String>,
    pub(super) focused_field: NetworkTunnelEditorField,
    pub(super) error: Option<String>,
}

impl NetworkTunnelEditorState {
    pub(super) fn is_dynamic(&self) -> bool {
        self.tunnel_type == "dynamic"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NetworkProxyEditorField {
    Name,
    Host,
    Port,
    Command,
    Username,
    Password,
}

impl NetworkProxyEditorField {
    pub(super) fn next(self, command: bool) -> Self {
        if command {
            return match self {
                Self::Name => Self::Command,
                Self::Command => Self::Name,
                _ => Self::Name,
            };
        }

        match self {
            Self::Name => Self::Host,
            Self::Host => Self::Port,
            Self::Port => Self::Username,
            Self::Username => Self::Password,
            Self::Password => Self::Name,
            Self::Command => Self::Name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NetworkProxyEditorState {
    pub(super) id: Option<String>,
    pub(super) name: String,
    pub(super) protocol: String,
    pub(super) host: String,
    pub(super) port: String,
    pub(super) command: String,
    pub(super) username: String,
    pub(super) password: String,
    pub(super) existing_password: Option<String>,
    pub(super) password_id: Option<String>,
    pub(super) group_id: Option<String>,
    pub(super) focused_field: NetworkProxyEditorField,
    pub(super) error: Option<String>,
}

impl NetworkProxyEditorState {
    pub(super) fn is_proxy_command(&self) -> bool {
        self.protocol == "proxycommand"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RemoteProcessSortKey {
    Cpu,
    Memory,
    Pid,
    User,
    Command,
}

impl RemoteProcessSortKey {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Memory => "Memory",
            Self::Pid => "PID",
            Self::User => "User",
            Self::Command => "Command",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RemoteProcessSortDirection {
    Ascending,
    Descending,
}

impl RemoteProcessSortDirection {
    pub(super) fn marker(self) -> &'static str {
        match self {
            Self::Ascending => "↑",
            Self::Descending => "↓",
        }
    }

    pub(super) fn reversed(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RemoteProcessSignalConfirmState {
    pub(super) pid: u32,
    pub(super) signal: &'static str,
    pub(super) command: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DockerConfirmAction {
    ContainerAction {
        container_id: String,
        action: &'static str,
    },
    ImageRemove {
        image_id: String,
        force: bool,
    },
    VolumeRemove {
        volume_name: String,
        force: bool,
    },
    NetworkRemove {
        network_id: String,
    },
    ComposeAction {
        project_name: String,
        config_files: Option<String>,
        action: &'static str,
    },
    Prune {
        volumes: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DockerConfirmState {
    pub(super) title: String,
    pub(super) detail: String,
    pub(super) action: DockerConfirmAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DockerTab {
    Containers,
    Images,
    Volumes,
    Networks,
    Compose,
}

impl DockerTab {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Containers => "Containers",
            Self::Images => "Images",
            Self::Volumes => "Volumes",
            Self::Networks => "Networks",
            Self::Compose => "Compose",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MainMode {
    Workspace,
    Page,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingsTab {
    General,
    Appearance,
    Interaction,
    Keybindings,
    TerminalGeneral,
    Search,
    Translation,
    AiGeneral,
    AiModels,
    AiRules,
    Transfer,
    Security,
    SyncBackup,
}

impl SettingsTab {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Appearance => "Appearance",
            Self::Interaction => "Interaction",
            Self::Keybindings => "Keybindings",
            Self::TerminalGeneral => "General",
            Self::Search => "Search",
            Self::Translation => "Translation",
            Self::AiGeneral => "General",
            Self::AiModels => "Models",
            Self::AiRules => "Rules",
            Self::Transfer => "Transfer",
            Self::Security => "Security",
            Self::SyncBackup => "Sync Backup",
        }
    }

    pub(super) fn group_label(self) -> &'static str {
        match self {
            Self::General | Self::Appearance | Self::Interaction | Self::Keybindings => "Workspace",
            Self::TerminalGeneral | Self::Search | Self::Translation => "Terminal Session",
            Self::AiGeneral | Self::AiModels | Self::AiRules => "AI",
            Self::Transfer => "Transfer",
            Self::Security => "Security",
            Self::SyncBackup => "Sync Backup",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkspaceSplitDirection {
    Horizontal,
    Vertical,
}

impl WorkspaceSplitDirection {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Horizontal => "Horizontal",
            Self::Vertical => "Vertical",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkspaceSplitState {
    pub(super) direction: WorkspaceSplitDirection,
    pub(super) primary_session_id: String,
    pub(super) secondary_session_id: String,
    pub(super) ratio_percent: u8,
}

impl WorkspaceSplitState {
    pub(super) const DEFAULT_RATIO_PERCENT: u8 = 50;
    pub(super) const MIN_RATIO_PERCENT: u8 = 20;
    pub(super) const MAX_RATIO_PERCENT: u8 = 80;

    pub(super) fn clamped_ratio_percent(value: u8) -> u8 {
        value.clamp(Self::MIN_RATIO_PERCENT, Self::MAX_RATIO_PERCENT)
    }

    pub(super) fn primary_weight(&self) -> f32 {
        Self::clamped_ratio_percent(self.ratio_percent) as f32
    }

    pub(super) fn secondary_weight(&self) -> f32 {
        (100 - Self::clamped_ratio_percent(self.ratio_percent)) as f32
    }

    pub(super) fn adjust_ratio(&mut self, delta: i8) {
        let next = (self.ratio_percent as i16 + delta as i16).clamp(
            Self::MIN_RATIO_PERCENT as i16,
            Self::MAX_RATIO_PERCENT as i16,
        );
        self.ratio_percent = next as u8;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RightFocus {
    Default,
    Recording,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BottomPanelMode {
    QuickCommands,
    CommandSend,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QuickCommandSortMode {
    Usage,
    Name,
    Created,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QuickCommandViewMode {
    List,
    Compact,
    Tile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QuickCommandEditorField {
    Label,
    Command,
    Category,
    Description,
}

#[derive(Debug, Clone)]
pub(super) struct QuickCommandEditorState {
    pub(super) original: Option<QuickCommand>,
    pub(super) focused_field: QuickCommandEditorField,
    pub(super) label: String,
    pub(super) command: String,
    pub(super) category_id: Option<String>,
    pub(super) category_draft: String,
    pub(super) description: String,
    pub(super) color_tag: Option<String>,
    pub(super) icon_tag: Option<String>,
    pub(super) pinned: bool,
    pub(super) execution_mode: String,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct QuickCommandDeleteState {
    pub(super) id: String,
    pub(super) label: String,
}

#[derive(Debug, Clone)]
pub(super) struct QuickCommandDetailsState {
    pub(super) command: QuickCommand,
    pub(super) category: String,
    pub(super) risk: String,
}

#[derive(Debug, Clone)]
pub(super) struct QuickCommandCategoryDeleteState {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) command_count: usize,
}

#[derive(Debug, Clone)]
pub(super) struct QuickCommandCategoryRenameState {
    pub(super) id: String,
    pub(super) original_name: String,
    pub(super) draft: String,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct QuickCommandVariableDef {
    pub(super) raw: String,
    pub(super) name: String,
    pub(super) options: Vec<String>,
    pub(super) value: String,
}

#[derive(Debug, Clone)]
pub(super) struct QuickCommandVariablePromptState {
    pub(super) command_id: String,
    pub(super) label: String,
    pub(super) command: String,
    pub(super) execute: bool,
    pub(super) send_to_all: bool,
    pub(super) variables: Vec<QuickCommandVariableDef>,
    pub(super) focused_index: usize,
}

impl QuickCommandEditorState {
    pub(super) fn blank() -> Self {
        Self {
            original: None,
            focused_field: QuickCommandEditorField::Label,
            label: String::new(),
            command: String::new(),
            category_id: None,
            category_draft: String::new(),
            description: String::new(),
            color_tag: None,
            icon_tag: None,
            pinned: false,
            execution_mode: "execute".to_string(),
            error: None,
        }
    }

    pub(super) fn from_command(command: QuickCommand) -> Self {
        Self {
            focused_field: QuickCommandEditorField::Label,
            label: command.label.clone(),
            command: command.command.clone(),
            category_id: command.category_id.clone(),
            category_draft: String::new(),
            description: command.description.clone().unwrap_or_default(),
            color_tag: command.color_tag.clone(),
            icon_tag: command.icon_tag.clone(),
            pinned: command.pinned.unwrap_or_default(),
            execution_mode: command
                .execution_mode
                .clone()
                .unwrap_or_else(|| "execute".to_string()),
            error: None,
            original: Some(command),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AiPreparedRequest {
    pub(super) action: AiAction,
    pub(super) context: AiContext,
    pub(super) source_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TransferJobKind {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransferJobStatus {
    Running,
    Paused,
    Cancelling,
    Cancelled,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub(super) struct TransferJobState {
    pub(super) id: String,
    pub(super) kind: TransferJobKind,
    pub(super) status: TransferJobStatus,
    pub(super) detail: String,
    pub(super) entries: Vec<SftpFileEntry>,
    pub(super) summary: Option<SftpTransferSummary>,
    pub(super) progress: Option<SftpTransferProgress>,
    pub(super) control: Option<SftpTransferControl>,
}

#[derive(Debug)]
pub(super) struct TransferJobResult {
    pub(super) id: String,
    pub(super) event: TransferJobEvent,
}

#[derive(Debug)]
pub(super) enum TransferJobEvent {
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
pub(super) enum TransferJobOutput {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransferInputField {
    Remote,
    Local,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TransferBrowserSessionCacheState {
    pub(super) entries: Vec<SftpFileEntry>,
    pub(super) current_path: String,
    pub(super) home_dir: String,
    pub(super) history: VecDeque<String>,
    pub(super) history_index: usize,
    pub(super) visited_history: VecDeque<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransferBrowserSortColumn {
    Name,
    Modified,
    Size,
    Permissions,
    Owner,
    Group,
}

impl TransferBrowserSortColumn {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Modified => "Modified",
            Self::Size => "Size",
            Self::Permissions => "Perms",
            Self::Owner => "Owner",
            Self::Group => "Group",
        }
    }

    pub(super) fn default_direction(self) -> TransferBrowserSortDirection {
        match self {
            Self::Name | Self::Permissions | Self::Owner | Self::Group => {
                TransferBrowserSortDirection::Ascending
            }
            Self::Size | Self::Modified => TransferBrowserSortDirection::Descending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransferBrowserSortDirection {
    Ascending,
    Descending,
}

impl TransferBrowserSortDirection {
    pub(super) fn marker(self) -> &'static str {
        match self {
            Self::Ascending => "up",
            Self::Descending => "down",
        }
    }

    pub(super) fn toggled(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TransferBrowserColumnWidths {
    pub(super) name: Pixels,
    pub(super) modified: Pixels,
    pub(super) size: Pixels,
    pub(super) permissions: Pixels,
    pub(super) owner: Pixels,
    pub(super) group: Pixels,
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
    pub(super) fn get(self, column: TransferBrowserSortColumn) -> Pixels {
        match column {
            TransferBrowserSortColumn::Name => self.name,
            TransferBrowserSortColumn::Modified => self.modified,
            TransferBrowserSortColumn::Size => self.size,
            TransferBrowserSortColumn::Permissions => self.permissions,
            TransferBrowserSortColumn::Owner => self.owner,
            TransferBrowserSortColumn::Group => self.group,
        }
    }

    pub(super) fn set(&mut self, column: TransferBrowserSortColumn, width: Pixels) {
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

    pub(super) fn min_width(column: TransferBrowserSortColumn) -> Pixels {
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TransferBrowserColumnResizeState {
    pub(super) column: TransferBrowserSortColumn,
    pub(super) start_x: Pixels,
    pub(super) start_width: Pixels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransferBrowserDragSelectionState {
    pub(super) anchor_path: String,
    pub(super) base_selection: HashSet<String>,
    pub(super) additive: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TransferBrowserContextMenuState {
    pub(super) path: String,
    pub(super) name: String,
    pub(super) is_parent: bool,
    pub(super) is_current_directory: bool,
    pub(super) is_directory: bool,
    pub(super) x: Pixels,
    pub(super) y: Pixels,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TransferBrowserFavoritesMenuState {
    pub(super) x: Pixels,
    pub(super) y: Pixels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransferBrowserPendingRenameState {
    pub(super) path: String,
    pub(super) token: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TransferUnknownFileState {
    pub(super) entry: SftpFileEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransferRenameState {
    pub(super) old_path: String,
    pub(super) initial_name: String,
    pub(super) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransferMoveState {
    pub(super) old_path: String,
    pub(super) name: String,
    pub(super) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransferDeleteState {
    pub(super) remote_path: String,
    pub(super) name: String,
    pub(super) paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransferJobDeleteState {
    pub(super) job_id: String,
    pub(super) title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransferNewFolderState {
    pub(super) parent_path: String,
    pub(super) value: String,
    pub(super) mode: u32,
    pub(super) open_after_create: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransferNewFileState {
    pub(super) parent_path: String,
    pub(super) value: String,
    pub(super) mode: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransferSymlinkField {
    Name,
    Target,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransferNewSymlinkState {
    pub(super) parent_path: String,
    pub(super) name: String,
    pub(super) target: String,
    pub(super) focused_field: TransferSymlinkField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransferPropertiesField {
    Mode,
    Owner,
    Group,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransferPropertiesState {
    pub(super) entry: SftpFileEntry,
    pub(super) properties: Option<SftpFileProperties>,
    pub(super) mode_value: String,
    pub(super) owner_value: String,
    pub(super) group_value: String,
    pub(super) recursive: bool,
    pub(super) saving: bool,
    pub(super) error: Option<String>,
    pub(super) focused_field: TransferPropertiesField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransferEditorState {
    pub(super) remote_path: String,
    pub(super) name: String,
    pub(super) content: String,
    pub(super) search_query: String,
    pub(super) active_match: usize,
    pub(super) base_size: Option<u64>,
    pub(super) base_modified_at: Option<u64>,
    pub(super) loading: bool,
    pub(super) saving: bool,
    pub(super) dirty: bool,
    pub(super) conflict: bool,
    pub(super) close_confirm: bool,
    pub(super) close_after_save: bool,
    pub(super) reload_confirm: bool,
    pub(super) error: Option<String>,
    pub(super) focused_field: TransferEditorField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransferExternalSyncPromptState {
    pub(super) job_id: String,
    pub(super) remote_path: String,
    pub(super) local_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransferEditorField {
    Content,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CloudSyncInputField {
    RemoteRoot,
    WebdavEndpoint,
    WebdavRoot,
    WebdavUsername,
    WebdavPassword,
    S3Endpoint,
    S3Bucket,
    S3Region,
    S3Root,
    S3AccessKeyId,
    S3SecretAccessKey,
    S3SessionToken,
    GoogleDriveRoot,
    GoogleDriveAccessToken,
    GoogleDriveRefreshToken,
    GoogleDriveClientId,
    GoogleDriveClientSecret,
    OneDriveRoot,
    OneDriveAccessToken,
    OneDriveRefreshToken,
    OneDriveClientId,
    OneDriveClientSecret,
    AliyunDriveRoot,
    AliyunDriveType,
    AliyunDriveAccessToken,
    AliyunDriveRefreshToken,
    AliyunDriveClientId,
    AliyunDriveClientSecret,
    GiteeEndpoint,
    GiteeGistId,
    GiteeToken,
    GithubGistId,
    GithubToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AiInputField {
    Model,
    BaseUrl,
    ApiKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TranslateInputField {
    TargetLanguage,
    Text,
    SettingsTargetLanguage,
    DeeplApiKey,
    BaiduAppId,
    BaiduAppKey,
    AliAppId,
    AliAppKey,
    YoudaoAppId,
    YoudaoAppKey,
}

impl TranslateInputField {
    pub(super) fn is_settings_field(self) -> bool {
        matches!(
            self,
            Self::SettingsTargetLanguage
                | Self::DeeplApiKey
                | Self::BaiduAppId
                | Self::BaiduAppKey
                | Self::AliAppId
                | Self::AliAppKey
                | Self::YoudaoAppId
                | Self::YoudaoAppKey
        )
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct CloudSyncSecretDraft {
    pub(super) webdav_password: String,
    pub(super) s3_access_key_id: String,
    pub(super) s3_secret_access_key: String,
    pub(super) s3_session_token: String,
    pub(super) google_drive_access_token: String,
    pub(super) google_drive_refresh_token: String,
    pub(super) google_drive_client_secret: String,
    pub(super) onedrive_access_token: String,
    pub(super) onedrive_refresh_token: String,
    pub(super) onedrive_client_secret: String,
    pub(super) aliyun_drive_access_token: String,
    pub(super) aliyun_drive_refresh_token: String,
    pub(super) aliyun_drive_client_secret: String,
    pub(super) gitee_token: String,
    pub(super) github_token: String,
}

#[derive(Debug, Clone, Default)]
pub(super) struct TranslationSecretDraft {
    pub(super) deepl_api_key: String,
    pub(super) baidu_app_key: String,
    pub(super) ali_app_key: String,
    pub(super) youdao_app_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransferPathPromptKind {
    UploadFile,
    UploadDirectory,
    DownloadDirectory,
}

#[derive(Debug)]
pub(super) enum TransferPathPromptResult {
    Selected(Vec<PathBuf>),
    Cancelled,
    Failed(String),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RecordingPathPromptKind {
    Start,
    SaveTranscript,
}

#[derive(Debug)]
pub(super) enum RecordingPathPromptResult {
    Selected(PathBuf),
    Cancelled,
    Failed(String),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConfigPathPromptKind {
    Export,
    Import,
    PortableExport,
    PortableImport,
    EncryptedPortableExport,
    EncryptedPortableImport,
}

#[derive(Debug)]
pub(super) enum ConfigPathPromptResult {
    Exported(ConfigBackupInfo),
    Imported(ConfigBackupInfo),
    Cancelled,
    Failed(String),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SnapshotPasswordPromptKind {
    Export,
    Import,
    CloudPush,
    CloudPull,
    CloudForcePush,
    CloudForcePull,
    CloudProviderPush,
    CloudProviderPull,
    CloudProviderForcePush,
    CloudProviderForcePull,
}

#[derive(Debug, Clone)]
pub(super) struct SnapshotPasswordPromptState {
    pub(super) kind: SnapshotPasswordPromptKind,
    pub(super) value: String,
}

#[derive(Debug, Clone)]
pub(super) struct CloudSyncConflictState {
    pub(super) provider: String,
    pub(super) message: String,
    pub(super) provider_action: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DiagnosticsPathPromptKind {
    Export,
}

#[derive(Debug)]
pub(super) enum DiagnosticsPathPromptResult {
    Exported(DiagnosticsExportInfo),
    Cancelled,
    Failed(String),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeywordHighlightPathPromptKind {
    Import,
}

#[derive(Debug)]
pub(super) enum KeywordHighlightPathPromptResult {
    Imported {
        imported_rules: usize,
        updated_rules: usize,
        total_rules: usize,
    },
    Cancelled,
    Failed(String),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QuickCommandImportPathPromptKind {
    NyatermJson,
    WindTermQuickbar,
    XshellXts,
}

#[derive(Debug)]
pub(super) enum QuickCommandImportPathPromptResult {
    Imported {
        imported_commands: usize,
        imported_categories: usize,
        updated_commands: usize,
        total_commands: usize,
        total_categories: usize,
    },
    Cancelled,
    Failed(String),
    Closed,
}
