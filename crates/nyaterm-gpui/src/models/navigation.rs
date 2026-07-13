use std::hash::Hash;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum NavItem {
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
    pub(crate) fn label(self) -> &'static str {
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

    pub(crate) fn short_label(self) -> &'static str {
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
    pub(crate) fn panel_title(self) -> &'static str {
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
    pub(crate) fn glyph(self) -> &'static str {
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
    pub(crate) fn icon_path(self) -> Option<&'static str> {
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

    pub(crate) fn is_left_panel(self) -> bool {
        matches!(
            self,
            NavItem::Transfers
                | NavItem::Tunnels
                | NavItem::SecurityAuth
                | NavItem::SyncBackupHistory
                | NavItem::Migration
        )
    }

    pub(crate) fn is_right_panel(self) -> bool {
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

    pub(crate) fn opens_settings(self) -> bool {
        matches!(self, NavItem::Settings)
    }

    /// Stable id compatible with Tauri `UiConfig` panel ids.
    pub(crate) fn persistence_id(self) -> &'static str {
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

    pub(crate) fn from_persistence_id(id: &str) -> Option<Self> {
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
pub(crate) enum ActivityBarZone {
    LeftTop,
    LeftBottom,
    RightTop,
    RightBottom,
}

impl ActivityBarZone {
    pub(crate) fn persistence_key(self) -> &'static str {
        match self {
            Self::LeftTop => "left_top",
            Self::LeftBottom => "left_bottom",
            Self::RightTop => "right_top",
            Self::RightBottom => "right_bottom",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::LeftTop => "Left Top",
            Self::LeftBottom => "Left Bottom",
            Self::RightTop => "Right Top",
            Self::RightBottom => "Right Bottom",
        }
    }

    pub(crate) fn all() -> [Self; 4] {
        [
            Self::LeftTop,
            Self::LeftBottom,
            Self::RightTop,
            Self::RightBottom,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivityBarEntry {
    Panel(NavItem),
    QuickCommands,
    CommandSend,
    Recording,
    Lock,
}

impl ActivityBarEntry {
    pub(crate) fn persistence_id(self) -> &'static str {
        match self {
            Self::Panel(item) => item.persistence_id(),
            Self::QuickCommands => "quickCmdBar",
            Self::CommandSend => "serialSend",
            Self::Recording => "recording",
            Self::Lock => "lock",
        }
    }

    pub(crate) fn from_persistence_id(id: &str) -> Option<Self> {
        match id.trim() {
            "quickCmdBar" => Some(Self::QuickCommands),
            "serialSend" => Some(Self::CommandSend),
            "recording" => Some(Self::Recording),
            "lock" => Some(Self::Lock),
            other => NavItem::from_persistence_id(other).map(Self::Panel),
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Panel(item) => item.label(),
            Self::QuickCommands => "Quick Commands",
            Self::CommandSend => "Command Send",
            Self::Recording => "Recording",
            Self::Lock => "Lock",
        }
    }

    pub(crate) fn short_label(self) -> &'static str {
        match self {
            Self::Panel(item) => item.short_label(),
            Self::QuickCommands => "Cmd",
            Self::CommandSend => "Send",
            Self::Recording => "Rec",
            Self::Lock => "Lock",
        }
    }

    pub(crate) fn glyph(self) -> &'static str {
        match self {
            Self::Panel(item) => item.glyph(),
            Self::QuickCommands => "⚡",
            Self::CommandSend => "⏎",
            Self::Recording => "●",
            Self::Lock => "🔒",
        }
    }

    pub(crate) fn icon_path(self) -> Option<&'static str> {
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
pub(crate) struct ActivityBarLayoutState {
    pub(crate) left_top: Vec<String>,
    pub(crate) left_bottom: Vec<String>,
    pub(crate) right_top: Vec<String>,
    pub(crate) right_bottom: Vec<String>,
    pub(crate) show_labels: bool,
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
    pub(crate) fn zone_mut(&mut self, zone: ActivityBarZone) -> &mut Vec<String> {
        match zone {
            ActivityBarZone::LeftTop => &mut self.left_top,
            ActivityBarZone::LeftBottom => &mut self.left_bottom,
            ActivityBarZone::RightTop => &mut self.right_top,
            ActivityBarZone::RightBottom => &mut self.right_bottom,
        }
    }

    pub(crate) fn zone(&self, zone: ActivityBarZone) -> &[String] {
        match zone {
            ActivityBarZone::LeftTop => &self.left_top,
            ActivityBarZone::LeftBottom => &self.left_bottom,
            ActivityBarZone::RightTop => &self.right_top,
            ActivityBarZone::RightBottom => &self.right_bottom,
        }
    }

    pub(crate) fn find_entry(&self, entry_id: &str) -> Option<(ActivityBarZone, usize)> {
        for zone in ActivityBarZone::all() {
            if let Some(index) = self.zone(zone).iter().position(|id| id == entry_id) {
                return Some((zone, index));
            }
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivityBarContextMenuState {
    pub(crate) entry_id: String,
    pub(crate) zone: ActivityBarZone,
    pub(crate) index: usize,
}

/// Top menubar dropdown (Tauri Header File/View/Terminal/Help).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TitleMenu {
    File,
    View,
    Terminal,
    Help,
}

impl TitleMenu {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::View => "View",
            Self::Terminal => "Terminal",
            Self::Help => "Help",
        }
    }

    pub(crate) fn all() -> [Self; 4] {
        [Self::File, Self::View, Self::Terminal, Self::Help]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanelSide {
    Left,
    Right,
}
