use gpui::{Pixels, px};
use nyaterm_domain::{
    AiAction, AiContext, AiExecutionProfile, ConfigBackupInfo, ConnectionType, DiagnosticsExportInfo,
    QuickCommand, SavedConnection,
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
    /// Viewport offset from the live bottom (0 = follow output).
    pub(super) scroll_offset: usize,
}

impl TerminalViewState {
    pub(super) fn new() -> Self {
        Self {
            output: String::new(),
            screen: TerminalScreen::default(),
            has_unread: false,
            scroll_offset: 0,
        }
    }

    pub(super) fn from_output(output: String) -> Self {
        let screen = terminal_screen_from_output(&output);
        Self {
            output,
            screen,
            has_unread: false,
            scroll_offset: 0,
        }
    }

    pub(super) fn append_text(&mut self, text: &str) {
        self.output.push_str(text);
        self.screen.advance(text.as_bytes());
        trim_terminal_output(&mut self.output);
        // Keep following the bottom while pinned.
        if self.scroll_offset == 0 {
            // no-op
        }
        self.clamp_scroll_offset();
    }

    pub(super) fn append_bytes(&mut self, data: &[u8]) {
        self.screen.advance(data);
        self.output.push_str(&String::from_utf8_lossy(data));
        trim_terminal_output(&mut self.output);
        self.clamp_scroll_offset();
    }

    pub(super) fn clear(&mut self) {
        self.output.clear();
        self.screen.clear();
        self.has_unread = false;
        self.scroll_offset = 0;
    }

    pub(super) fn clamp_scroll_offset(&mut self) {
        let max = self.screen.scrollback_len();
        if self.scroll_offset > max {
            self.scroll_offset = max;
        }
    }
}

/// Inclusive cell coordinate inside the visible terminal grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TerminalCellPos {
    pub(super) row: usize,
    pub(super) col: usize,
}

impl TerminalCellPos {
    pub(super) fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

/// Visible-grid text selection (start/end are inclusive cell positions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TerminalSelection {
    pub(super) anchor: TerminalCellPos,
    pub(super) head: TerminalCellPos,
}

impl TerminalSelection {
    pub(super) fn new(anchor: TerminalCellPos) -> Self {
        Self {
            anchor,
            head: anchor,
        }
    }

    pub(super) fn ordered(&self) -> (TerminalCellPos, TerminalCellPos) {
        let a = self.anchor;
        let b = self.head;
        if (a.row, a.col) <= (b.row, b.col) {
            (a, b)
        } else {
            (b, a)
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Column range [start, end) for a painted line, if any cells are selected.
    /// Endpoints are inclusive cell positions; returned range is half-open for slicing.
    pub(super) fn cols_for_row(&self, row: usize) -> Option<(usize, usize)> {
        if self.is_empty() {
            return None;
        }
        let (start, end) = self.ordered();
        if row < start.row || row > end.row {
            return None;
        }
        if start.row == end.row {
            return Some((start.col, end.col.saturating_add(1)));
        }
        if row == start.row {
            return Some((start.col, usize::MAX));
        }
        if row == end.row {
            return Some((0, end.col.saturating_add(1)));
        }
        Some((0, usize::MAX))
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
    AiAssistant,
    ActiveSessions,
    CommandHistory,
    SecurityAuth,
    SyncBackupHistory,
    Recording,
}

impl NavItem {
    pub(super) fn label(self) -> &'static str {
        match self {
            NavItem::Workspace => "Workspace",
            NavItem::Connections => "Saved Connections",
            NavItem::Tunnels => "Network",
            NavItem::Stats => "Resource Monitor",
            NavItem::Processes => "Process Manager",
            NavItem::Docker => "Docker",
            NavItem::Translation => "Translation",
            NavItem::Transfers => "File Explorer",
            NavItem::Settings => "Settings",
            NavItem::Migration => "Migration",
            NavItem::AiAssistant => "AI Assistant",
            NavItem::ActiveSessions => "Active Sessions",
            NavItem::CommandHistory => "Command History",
            NavItem::SecurityAuth => "Security / Auth",
            NavItem::SyncBackupHistory => "Sync / Backup",
            NavItem::Recording => "Recording",
        }
    }

    pub(super) fn short_label(self) -> &'static str {
        // Tauri panel.* short titles used under activity icons when labels are shown.
        match self {
            NavItem::Workspace => "Work",
            NavItem::Connections => "Conn",
            NavItem::Tunnels => "Net",
            NavItem::Stats => "Res",
            NavItem::Processes => "Proc",
            NavItem::Docker => "Dock",
            NavItem::Translation => "Trans",
            NavItem::Transfers => "Files",
            NavItem::Settings => "Set",
            NavItem::Migration => "Mig",
            NavItem::AiAssistant => "AI",
            NavItem::ActiveSessions => "Sess",
            NavItem::CommandHistory => "Hist",
            NavItem::SecurityAuth => "Auth",
            NavItem::SyncBackupHistory => "Sync",
            NavItem::Recording => "Rec",
        }
    }

    /// Compact panel title used in side PanelHeader (Tauri panel.* keys).
    pub(super) fn panel_title(self) -> &'static str {
        match self {
            NavItem::Transfers => "Files",
            NavItem::Tunnels => "Network",
            NavItem::Connections => "Connections",
            NavItem::AiAssistant => "AI Assistant",
            NavItem::ActiveSessions => "Sessions",
            NavItem::CommandHistory => "History",
            NavItem::Stats => "Resources",
            NavItem::Processes => "Processes",
            NavItem::Docker => "Docker",
            NavItem::SyncBackupHistory => "Cloud Sync",
            NavItem::SecurityAuth => "Security",
            NavItem::Recording => "Recording",
            NavItem::Translation => "Translation",
            NavItem::Migration => "Migration",
            NavItem::Settings => "Settings",
            NavItem::Workspace => "Workspace",
        }
    }

    /// Compact monochrome glyph used as text fallback for the activity bar.
    pub(super) fn glyph(self) -> &'static str {
        match self {
            NavItem::Workspace => "▣",
            NavItem::Connections => "⌂",
            NavItem::Tunnels => "⇄",
            NavItem::Stats => "◔",
            NavItem::Processes => "☰",
            NavItem::Docker => "🐋",
            NavItem::Translation => "文",
            NavItem::Transfers => "📁",
            NavItem::Settings => "⚙",
            NavItem::Migration => "⇪",
            NavItem::AiAssistant => "✦",
            NavItem::ActiveSessions => "◉",
            NavItem::CommandHistory => "⌛",
            NavItem::SecurityAuth => "⛨",
            NavItem::SyncBackupHistory => "☁",
            NavItem::Recording => "●",
        }
    }

    /// Bundled SVG path for activity-bar / toolbar icons.
    pub(super) fn icon_path(self) -> Option<&'static str> {
        Some(match self {
            NavItem::Transfers => "icons/files.svg",
            NavItem::Tunnels => "icons/network.svg",
            NavItem::SecurityAuth => "icons/auth.svg",
            NavItem::SyncBackupHistory => "icons/sync.svg",
            NavItem::Settings => "icons/settings.svg",
            NavItem::Connections => "icons/connections.svg",
            NavItem::AiAssistant => "icons/ai.svg",
            NavItem::ActiveSessions => "icons/sessions.svg",
            NavItem::CommandHistory => "icons/history.svg",
            NavItem::Stats => "icons/resources.svg",
            NavItem::Processes => "icons/processes.svg",
            NavItem::Docker => "icons/docker.svg",
            NavItem::Recording => "icons/record.svg",
            NavItem::Translation => "icons/translation.svg",
            NavItem::Migration => "icons/migration.svg",
            NavItem::Workspace => return None,
        })
    }

    pub(super) fn is_left_panel(self) -> bool {
        matches!(
            self,
            NavItem::Transfers
                | NavItem::Tunnels
                | NavItem::SecurityAuth
                | NavItem::SyncBackupHistory
                | NavItem::Migration
        )
    }

    pub(super) fn is_right_panel(self) -> bool {
        matches!(
            self,
            NavItem::Connections
                | NavItem::AiAssistant
                | NavItem::ActiveSessions
                | NavItem::CommandHistory
                | NavItem::Stats
                | NavItem::Processes
                | NavItem::Docker
                | NavItem::Translation
                | NavItem::Recording
                | NavItem::Workspace
        )
    }

    pub(super) fn opens_settings(self) -> bool {
        matches!(self, NavItem::Settings)
    }

    /// Stable id compatible with Tauri `UiConfig` panel ids.
    pub(super) fn persistence_id(self) -> &'static str {
        match self {
            NavItem::Workspace => "workspace",
            NavItem::Connections => "savedConnections",
            NavItem::Tunnels => "network",
            NavItem::Stats => "resourceMonitor",
            NavItem::Processes => "processManager",
            NavItem::Docker => "dockerManager",
            NavItem::Translation => "translation",
            NavItem::Transfers => "fileExplorer",
            NavItem::Settings => "settings",
            NavItem::Migration => "migration",
            NavItem::AiAssistant => "aiAssistant",
            NavItem::ActiveSessions => "activeSessions",
            NavItem::CommandHistory => "commandHistory",
            NavItem::SecurityAuth => "securityAuth",
            NavItem::SyncBackupHistory => "syncBackupHistory",
            NavItem::Recording => "recording",
        }
    }

    pub(super) fn from_persistence_id(id: &str) -> Option<Self> {
        match id.trim() {
            "workspace" => Some(NavItem::Workspace),
            "connections" | "savedConnections" => Some(NavItem::Connections),
            "network" | "tunnels" => Some(NavItem::Tunnels),
            "stats" | "resourceMonitor" => Some(NavItem::Stats),
            "processes" | "processManager" => Some(NavItem::Processes),
            "docker" | "dockerManager" => Some(NavItem::Docker),
            "translation" => Some(NavItem::Translation),
            "fileExplorer" | "fileTransfer" | "transfers" => Some(NavItem::Transfers),
            "settings" => Some(NavItem::Settings),
            "migration" => Some(NavItem::Migration),
            "aiAssistant" | "ai" => Some(NavItem::AiAssistant),
            "activeSessions" => Some(NavItem::ActiveSessions),
            "commandHistory" => Some(NavItem::CommandHistory),
            "securityAuth" | "security" => Some(NavItem::SecurityAuth),
            "syncBackupHistory" | "syncBackup" => Some(NavItem::SyncBackupHistory),
            "recording" => Some(NavItem::Recording),
            _ => None,
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActivityBarZone {
    LeftTop,
    LeftBottom,
    RightTop,
    RightBottom,
}

impl ActivityBarZone {
    pub(super) fn persistence_key(self) -> &'static str {
        match self {
            Self::LeftTop => "left_top",
            Self::LeftBottom => "left_bottom",
            Self::RightTop => "right_top",
            Self::RightBottom => "right_bottom",
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::LeftTop => "Left Top",
            Self::LeftBottom => "Left Bottom",
            Self::RightTop => "Right Top",
            Self::RightBottom => "Right Bottom",
        }
    }

    pub(super) fn all() -> [Self; 4] {
        [Self::LeftTop, Self::LeftBottom, Self::RightTop, Self::RightBottom]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActivityBarEntry {
    Panel(NavItem),
    QuickCommands,
    CommandSend,
    Recording,
    Lock,
}

impl ActivityBarEntry {
    pub(super) fn persistence_id(self) -> &'static str {
        match self {
            Self::Panel(item) => item.persistence_id(),
            Self::QuickCommands => "quickCmdBar",
            Self::CommandSend => "serialSend",
            Self::Recording => "recording",
            Self::Lock => "lock",
        }
    }

    pub(super) fn from_persistence_id(id: &str) -> Option<Self> {
        match id.trim() {
            "quickCmdBar" => Some(Self::QuickCommands),
            "serialSend" => Some(Self::CommandSend),
            "recording" => Some(Self::Recording),
            "lock" => Some(Self::Lock),
            other => NavItem::from_persistence_id(other).map(Self::Panel),
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Panel(item) => item.label(),
            Self::QuickCommands => "Quick Commands",
            Self::CommandSend => "Command Send",
            Self::Recording => "Recording",
            Self::Lock => "Lock",
        }
    }

    pub(super) fn short_label(self) -> &'static str {
        match self {
            Self::Panel(item) => item.short_label(),
            Self::QuickCommands => "Cmd",
            Self::CommandSend => "Send",
            Self::Recording => "Rec",
            Self::Lock => "Lock",
        }
    }

    pub(super) fn glyph(self) -> &'static str {
        match self {
            Self::Panel(item) => item.glyph(),
            Self::QuickCommands => "⚡",
            Self::CommandSend => "⏎",
            Self::Recording => "●",
            Self::Lock => "🔒",
        }
    }

    pub(super) fn icon_path(self) -> Option<&'static str> {
        match self {
            Self::Panel(item) => item.icon_path(),
            Self::QuickCommands => Some("icons/commands.svg"),
            Self::CommandSend => Some("icons/send.svg"),
            Self::Recording => Some("icons/record.svg"),
            Self::Lock => Some("icons/lock.svg"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ActivityBarLayoutState {
    pub(super) left_top: Vec<String>,
    pub(super) left_bottom: Vec<String>,
    pub(super) right_top: Vec<String>,
    pub(super) right_bottom: Vec<String>,
    pub(super) show_labels: bool,
}

impl Default for ActivityBarLayoutState {
    fn default() -> Self {
        Self {
            left_top: vec![
                "fileExplorer".to_string(),
                "network".to_string(),
                "securityAuth".to_string(),
            ],
            left_bottom: vec!["syncBackupHistory".to_string(), "settings".to_string()],
            right_top: vec![
                "savedConnections".to_string(),
                "aiAssistant".to_string(),
                "activeSessions".to_string(),
                "commandHistory".to_string(),
                "resourceMonitor".to_string(),
                "processManager".to_string(),
                "dockerManager".to_string(),
            ],
            right_bottom: vec![
                "quickCmdBar".to_string(),
                "serialSend".to_string(),
                "recording".to_string(),
                "lock".to_string(),
            ],
            show_labels: false,
        }
    }
}

impl ActivityBarLayoutState {
    pub(super) fn zone_mut(&mut self, zone: ActivityBarZone) -> &mut Vec<String> {
        match zone {
            ActivityBarZone::LeftTop => &mut self.left_top,
            ActivityBarZone::LeftBottom => &mut self.left_bottom,
            ActivityBarZone::RightTop => &mut self.right_top,
            ActivityBarZone::RightBottom => &mut self.right_bottom,
        }
    }

    pub(super) fn zone(&self, zone: ActivityBarZone) -> &[String] {
        match zone {
            ActivityBarZone::LeftTop => &self.left_top,
            ActivityBarZone::LeftBottom => &self.left_bottom,
            ActivityBarZone::RightTop => &self.right_top,
            ActivityBarZone::RightBottom => &self.right_bottom,
        }
    }

    pub(super) fn find_entry(&self, entry_id: &str) -> Option<(ActivityBarZone, usize)> {
        for zone in ActivityBarZone::all() {
            if let Some(index) = self.zone(zone).iter().position(|id| id == entry_id) {
                return Some((zone, index));
            }
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ActivityBarContextMenuState {
    pub(super) entry_id: String,
    pub(super) zone: ActivityBarZone,
    pub(super) index: usize,
}

/// Top menubar dropdown (Tauri Header File/View/Terminal/Help).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TitleMenu {
    File,
    View,
    Terminal,
    Help,
}

impl TitleMenu {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::View => "View",
            Self::Terminal => "Terminal",
            Self::Help => "Help",
        }
    }

    pub(super) fn all() -> [Self; 4] {
        [Self::File, Self::View, Self::Terminal, Self::Help]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelSide {
    Left,
    Right,
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
pub(super) enum ConnectionKindTab {
    Ssh,
    Local,
    Telnet,
    Serial,
}

impl ConnectionKindTab {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Ssh => "SSH",
            Self::Local => "Local",
            Self::Telnet => "Telnet",
            Self::Serial => "Serial",
        }
    }

    pub(super) fn next(self) -> Self {
        match self {
            Self::Ssh => Self::Local,
            Self::Local => Self::Telnet,
            Self::Telnet => Self::Serial,
            Self::Serial => Self::Ssh,
        }
    }

    pub(super) fn from_connection_type(config: &nyaterm_domain::ConnectionType) -> Self {
        match config {
            nyaterm_domain::ConnectionType::Ssh { .. } => Self::Ssh,
            nyaterm_domain::ConnectionType::LocalTerminal { .. } => Self::Local,
            nyaterm_domain::ConnectionType::Telnet { .. } => Self::Telnet,
            nyaterm_domain::ConnectionType::Serial { .. } => Self::Serial,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConnectionEditorField {
    Name,
    Description,
    Host,
    Port,
    Username,
    Password,
    ShellPath,
    ShellArgs,
    WorkingDir,
    SerialPort,
    BaudRate,
    PostLoginCommand,
    PostLoginDelay,
}

impl ConnectionEditorField {
    pub(super) fn next(self, kind: ConnectionKindTab, auth_mode: &str) -> Self {
        match kind {
            ConnectionKindTab::Ssh => match self {
                Self::Name => Self::Description,
                Self::Description => Self::Host,
                Self::Host => Self::Port,
                Self::Port => Self::Username,
                Self::Username if auth_mode == "password" => Self::Password,
                Self::Username => Self::PostLoginCommand,
                Self::Password => Self::PostLoginCommand,
                Self::PostLoginCommand => Self::PostLoginDelay,
                Self::PostLoginDelay => Self::Name,
                other => other.next_fallback(kind),
            },
            ConnectionKindTab::Local => match self {
                Self::Name => Self::Description,
                Self::Description => Self::ShellPath,
                Self::ShellPath => Self::ShellArgs,
                Self::ShellArgs => Self::WorkingDir,
                Self::WorkingDir => Self::Name,
                other => other.next_fallback(kind),
            },
            ConnectionKindTab::Telnet => match self {
                Self::Name => Self::Description,
                Self::Description => Self::Host,
                Self::Host => Self::Port,
                Self::Port => Self::Name,
                other => other.next_fallback(kind),
            },
            ConnectionKindTab::Serial => match self {
                Self::Name => Self::Description,
                Self::Description => Self::SerialPort,
                Self::SerialPort => Self::BaudRate,
                Self::BaudRate => Self::Name,
                other => other.next_fallback(kind),
            },
        }
    }

    fn next_fallback(self, kind: ConnectionKindTab) -> Self {
        match kind {
            ConnectionKindTab::Ssh => Self::Name,
            ConnectionKindTab::Local => Self::Name,
            ConnectionKindTab::Telnet => Self::Name,
            ConnectionKindTab::Serial => Self::Name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConnectionEditorState {
    pub(super) id: Option<String>,
    pub(super) kind: ConnectionKindTab,
    pub(super) name: String,
    pub(super) description: String,
    pub(super) group_id: Option<String>,
    pub(super) host: String,
    pub(super) port: String,
    pub(super) username: String,
    pub(super) auth_mode: String,
    pub(super) password: String,
    pub(super) existing_password: Option<String>,
    pub(super) key_id: Option<String>,
    pub(super) otp_id: Option<String>,
    pub(super) auto_fill_otp: bool,
    pub(super) proxy_id: Option<String>,
    pub(super) proxy_jump_id: Option<String>,
    pub(super) x11_forwarding: bool,
    pub(super) backspace_mode: String,
    pub(super) shell_path: String,
    pub(super) shell_args: String,
    pub(super) working_dir: String,
    pub(super) serial_port: String,
    pub(super) baud_rate: String,
    pub(super) data_bits: String,
    pub(super) parity: String,
    pub(super) stop_bits: String,
    pub(super) raw_tcp_cli: bool,
    pub(super) local_echo: bool,
    pub(super) post_login_enabled: bool,
    pub(super) post_login_command: String,
    pub(super) post_login_delay_ms: String,
    pub(super) connect_after_save: bool,
    pub(super) focused_field: ConnectionEditorField,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConnectionGroupEditorState {
    pub(super) id: Option<String>,
    pub(super) name: String,
    pub(super) parent_id: Option<String>,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConnectionDeleteConfirmState {
    pub(super) connection_id: String,
    pub(super) label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ConnectionContextMenuState {
    pub(super) connection_id: String,
    pub(super) x: Pixels,
    pub(super) y: Pixels,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TerminalContextMenuState {
    pub(super) x: Pixels,
    pub(super) y: Pixels,
    /// Snapshot of selected text when the menu opened (Tauri caches selection).
    pub(super) selected_text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ConnectionGroupContextMenuState {
    pub(super) group_id: String,
    pub(super) x: Pixels,
    pub(super) y: Pixels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConnectionGroupDeleteConfirmState {
    pub(super) group_id: String,
    pub(super) label: String,
    pub(super) connection_count: usize,
    pub(super) child_group_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConnectionSortMode {
    Default,
    NameAsc,
    NameDesc,
    Recent,
}

impl ConnectionSortMode {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::NameAsc => "Name A-Z",
            Self::NameDesc => "Name Z-A",
            Self::Recent => "Recent",
        }
    }

    pub(super) fn next(self) -> Self {
        match self {
            Self::Default => Self::NameAsc,
            Self::NameAsc => Self::NameDesc,
            Self::NameDesc => Self::Recent,
            Self::Recent => Self::Default,
        }
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

/// Recursive workspace pane tree (Tauri PaneNode / SplitPane).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum WorkspacePaneNode {
    Leaf {
        session_id: String,
    },
    Split {
        id: String,
        direction: WorkspaceSplitDirection,
        ratio_percent: u8,
        first: Box<WorkspacePaneNode>,
        second: Box<WorkspacePaneNode>,
    },
}

impl WorkspacePaneNode {
    pub(super) const DEFAULT_RATIO_PERCENT: u8 = 50;
    pub(super) const MIN_RATIO_PERCENT: u8 = 20;
    pub(super) const MAX_RATIO_PERCENT: u8 = 80;

    pub(super) fn leaf(session_id: impl Into<String>) -> Self {
        Self::Leaf {
            session_id: session_id.into(),
        }
    }

    pub(super) fn clamped_ratio_percent(value: u8) -> u8 {
        value.clamp(Self::MIN_RATIO_PERCENT, Self::MAX_RATIO_PERCENT)
    }

    pub(super) fn primary_weight(ratio_percent: u8) -> f32 {
        Self::clamped_ratio_percent(ratio_percent) as f32
    }

    pub(super) fn secondary_weight(ratio_percent: u8) -> f32 {
        (100 - Self::clamped_ratio_percent(ratio_percent)) as f32
    }

    pub(super) fn contains_session(&self, session_id: &str) -> bool {
        match self {
            Self::Leaf { session_id: id } => id == session_id,
            Self::Split { first, second, .. } => {
                first.contains_session(session_id) || second.contains_session(session_id)
            }
        }
    }

    pub(super) fn session_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        self.collect_session_ids(&mut ids);
        ids
    }

    fn collect_session_ids(&self, out: &mut Vec<String>) {
        match self {
            Self::Leaf { session_id } => out.push(session_id.clone()),
            Self::Split { first, second, .. } => {
                first.collect_session_ids(out);
                second.collect_session_ids(out);
            }
        }
    }

    pub(super) fn is_split(&self) -> bool {
        matches!(self, Self::Split { .. })
    }

    pub(super) fn split_count(&self) -> usize {
        match self {
            Self::Leaf { .. } => 0,
            Self::Split { first, second, .. } => 1 + first.split_count() + second.split_count(),
        }
    }

    pub(super) fn focused_split_id(&self, active_session_id: Option<&str>) -> Option<String> {
        if let Some(session_id) = active_session_id {
            if let Some(id) = self.split_id_containing(session_id) {
                return Some(id);
            }
        }
        self.first_split_id()
    }

    fn first_split_id(&self) -> Option<String> {
        match self {
            Self::Leaf { .. } => None,
            Self::Split { id, .. } => Some(id.clone()),
        }
    }

    fn split_id_containing(&self, session_id: &str) -> Option<String> {
        match self {
            Self::Leaf { .. } => None,
            Self::Split {
                id,
                first,
                second,
                ..
            } => {
                if first.contains_session(session_id) || second.contains_session(session_id) {
                    if matches!(**first, Self::Leaf { .. }) || matches!(**second, Self::Leaf { .. })
                    {
                        // Prefer the deepest split that still directly owns the leaf when possible.
                    }
                    if let Some(nested) = first
                        .split_id_containing(session_id)
                        .or_else(|| second.split_id_containing(session_id))
                    {
                        return Some(nested);
                    }
                    Some(id.clone())
                } else {
                    None
                }
            }
        }
    }

    pub(super) fn adjust_ratio_for_split(&mut self, split_id: &str, delta: i8) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split {
                id,
                ratio_percent,
                first,
                second,
                ..
            } => {
                if id == split_id {
                    let next = (*ratio_percent as i16 + delta as i16).clamp(
                        Self::MIN_RATIO_PERCENT as i16,
                        Self::MAX_RATIO_PERCENT as i16,
                    );
                    *ratio_percent = next as u8;
                    true
                } else {
                    first.adjust_ratio_for_split(split_id, delta)
                        || second.adjust_ratio_for_split(split_id, delta)
                }
            }
        }
    }

    pub(super) fn set_ratio_for_split(&mut self, split_id: &str, value: u8) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split {
                id,
                ratio_percent,
                first,
                second,
                ..
            } => {
                if id == split_id {
                    *ratio_percent = Self::clamped_ratio_percent(value);
                    true
                } else {
                    first.set_ratio_for_split(split_id, value)
                        || second.set_ratio_for_split(split_id, value)
                }
            }
        }
    }

    pub(super) fn direction_for_split(&self, split_id: &str) -> Option<WorkspaceSplitDirection> {
        match self {
            Self::Leaf { .. } => None,
            Self::Split {
                id,
                direction,
                first,
                second,
                ..
            } => {
                if id == split_id {
                    Some(*direction)
                } else {
                    first
                        .direction_for_split(split_id)
                        .or_else(|| second.direction_for_split(split_id))
                }
            }
        }
    }

    pub(super) fn ratio_for_split(&self, split_id: &str) -> Option<u8> {
        match self {
            Self::Leaf { .. } => None,
            Self::Split {
                id,
                ratio_percent,
                first,
                second,
                ..
            } => {
                if id == split_id {
                    Some(*ratio_percent)
                } else {
                    first
                        .ratio_for_split(split_id)
                        .or_else(|| second.ratio_for_split(split_id))
                }
            }
        }
    }

    /// Split the leaf holding `target_session_id` by replacing it with
    /// Split(leaf(target), leaf(new_session_id)).
    pub(super) fn split_leaf(
        &mut self,
        target_session_id: &str,
        new_session_id: String,
        direction: WorkspaceSplitDirection,
        split_id: String,
    ) -> bool {
        match self {
            Self::Leaf { session_id } if session_id == target_session_id => {
                let first = Box::new(Self::leaf(session_id.clone()));
                let second = Box::new(Self::leaf(new_session_id));
                *self = Self::Split {
                    id: split_id,
                    direction,
                    ratio_percent: Self::DEFAULT_RATIO_PERCENT,
                    first,
                    second,
                };
                true
            }
            Self::Leaf { .. } => false,
            Self::Split { first, second, .. } => {
                first.split_leaf(target_session_id, new_session_id.clone(), direction, split_id.clone())
                    || second.split_leaf(target_session_id, new_session_id, direction, split_id)
            }
        }
    }

    /// Remove a leaf session and collapse its parent split into the sibling.
    pub(super) fn remove_leaf(self, target_session_id: &str) -> Option<Self> {
        match self {
            Self::Leaf { session_id } => {
                if session_id == target_session_id {
                    None
                } else {
                    Some(Self::Leaf { session_id })
                }
            }
            Self::Split {
                id,
                direction,
                ratio_percent,
                first,
                second,
            } => {
                let first = first.remove_leaf(target_session_id);
                let second = second.remove_leaf(target_session_id);
                match (first, second) {
                    (Some(first), Some(second)) => Some(Self::Split {
                        id,
                        direction,
                        ratio_percent,
                        first: Box::new(first),
                        second: Box::new(second),
                    }),
                    (Some(only), None) | (None, Some(only)) => Some(only),
                    (None, None) => None,
                }
            }
        }
    }

    /// Remove dead sessions and collapse unnecessary nodes.
    pub(super) fn prune(self, live_ids: &HashSet<String>) -> Option<Self> {
        match self {
            Self::Leaf { session_id } => {
                if live_ids.contains(&session_id) {
                    Some(Self::Leaf { session_id })
                } else {
                    None
                }
            }
            Self::Split {
                id,
                direction,
                ratio_percent,
                first,
                second,
            } => {
                let first = first.prune(live_ids);
                let second = second.prune(live_ids);
                match (first, second) {
                    (Some(first), Some(second)) => Some(Self::Split {
                        id,
                        direction,
                        ratio_percent: Self::clamped_ratio_percent(ratio_percent),
                        first: Box::new(first),
                        second: Box::new(second),
                    }),
                    (Some(only), None) | (None, Some(only)) => Some(only),
                    (None, None) => None,
                }
            }
        }
    }
}

/// Compatibility alias used by older dual-pane helpers.
pub(super) type WorkspaceSplitState = WorkspacePaneNode;

#[derive(Debug, Clone)]
pub(super) struct WorkspaceSplitResizeState {
    pub(super) split_id: String,
    pub(super) direction: WorkspaceSplitDirection,
    pub(super) start_pos: Pixels,
    pub(super) start_ratio: u8,
    pub(super) container_size: f32,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SecurityAuthTab {
    Keys,
    Passwords,
    Credentials,
    Otp,
}

impl SecurityAuthTab {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Keys => "Keys",
            Self::Passwords => "Pwd",
            Self::Credentials => "Cred",
            Self::Otp => "OTP",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SecurityKeyEditorField {
    Name,
    KeyPath,
    CertPath,
    Passphrase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SecurityKeyEditorState {
    pub(super) id: Option<String>,
    pub(super) name: String,
    pub(super) key_file_path: String,
    pub(super) cert_file_path: String,
    pub(super) passphrase: String,
    pub(super) has_key_data: bool,
    pub(super) has_cert_data: bool,
    pub(super) focused_field: SecurityKeyEditorField,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SecurityOtpEditorField {
    Issuer,
    Username,
    Secret,
    Digits,
    Period,
    Counter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SecurityOtpEditorState {
    pub(super) id: Option<String>,
    pub(super) otp_type: String,
    pub(super) issuer: String,
    pub(super) username: String,
    pub(super) secret: String,
    pub(super) algorithm: String,
    pub(super) digits: String,
    pub(super) period: String,
    pub(super) counter: String,
    pub(super) has_secret: bool,
    pub(super) focused_field: SecurityOtpEditorField,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SecurityDeleteConfirmState {
    pub(super) kind: SecurityAuthTab,
    pub(super) id: String,
    pub(super) label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SecurityPasswordEditorField {
    Name,
    Password,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SecurityPasswordEditorState {
    pub(super) id: Option<String>,
    pub(super) name: String,
    pub(super) password: String,
    pub(super) has_password: bool,
    pub(super) focused_field: SecurityPasswordEditorField,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SecurityCredentialEditorField {
    Name,
    Username,
    Password,
    UsernameRegex,
    PasswordRegex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SecurityCredentialEditorState {
    pub(super) id: Option<String>,
    pub(super) name: String,
    pub(super) username: String,
    pub(super) password: String,
    pub(super) username_prompt_regex: String,
    pub(super) password_prompt_regex: String,
    pub(super) enabled: bool,
    pub(super) has_password: bool,
    pub(super) focused_field: SecurityCredentialEditorField,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelResizeSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PanelResizeState {
    pub(super) side: PanelResizeSide,
    pub(super) start_x: Pixels,
    pub(super) start_width: Pixels,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TransferHeightResizeState {
    pub(super) start_y: Pixels,
    pub(super) start_height: Pixels,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PanelStackResizeState {
    pub(super) side: PanelSide,
    pub(super) above_id: String,
    pub(super) below_id: String,
    pub(super) start_y: Pixels,
    pub(super) above_weight: f32,
    pub(super) below_weight: f32,
    pub(super) container_height: f32,
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TransferBrowserUploadMenuState {
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
