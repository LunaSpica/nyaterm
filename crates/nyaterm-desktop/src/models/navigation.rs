use gpui::Pixels;
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum NavItem {
    Workspace,
    Connections,
    Tunnels,
    Stats,
    Processes,
    Docker,
    Transfers,
    Settings,
    AiAssistant,
    ActiveSessions,
    CommandHistory,
    SecurityAuth,
    SyncBackupHistory,
    Recording,
}

impl NavItem {
    pub(crate) fn i18n_key(self) -> Option<&'static str> {
        Some(match self {
            NavItem::Connections => "panel.savedConnections",
            NavItem::Tunnels => "panel.network",
            NavItem::Stats => "panel.resourceMonitor",
            NavItem::Processes => "panel.processManager",
            NavItem::Docker => "panel.dockerManager",
            NavItem::Transfers => "panel.fileExplorer",
            NavItem::Settings => "settings.title",
            NavItem::AiAssistant => "ai.title",
            NavItem::ActiveSessions => "panel.activeSessions",
            NavItem::CommandHistory => "panel.commandHistory",
            NavItem::SecurityAuth => "securityAuth.title",
            NavItem::SyncBackupHistory => "panel.syncBackupHistory",
            NavItem::Recording => "recording.panelTitle",
            NavItem::Workspace => return None,
        })
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            NavItem::Workspace => "Workspace",
            NavItem::Connections => "Saved Connections",
            NavItem::Tunnels => "Network",
            NavItem::Stats => "Resource Monitor",
            NavItem::Processes => "Process Manager",
            NavItem::Docker => "Docker",
            NavItem::Transfers => "File Explorer",
            NavItem::Settings => "Settings",
            NavItem::AiAssistant => "AI Assistant",
            NavItem::ActiveSessions => "Active Sessions",
            NavItem::CommandHistory => "Command History",
            NavItem::SecurityAuth => "Security / Auth",
            NavItem::SyncBackupHistory => "Sync / Backup",
            NavItem::Recording => "Recording",
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
            NavItem::Settings => "Settings",
            NavItem::Workspace => "Workspace",
        }
    }

    /// Compact monochrome glyph used as text fallback for the activity bar.
    /// Bundled SVG path for activity-bar / toolbar icons.
    pub(crate) fn icon_path(self) -> &'static str {
        match self {
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
            NavItem::Workspace => "icons/workspace.svg",
        }
    }

    pub(crate) fn is_left_panel(self) -> bool {
        matches!(
            self,
            NavItem::Transfers
                | NavItem::Tunnels
                | NavItem::SecurityAuth
                | NavItem::SyncBackupHistory
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
            NavItem::Transfers => "fileExplorer",
            NavItem::Settings => "settings",
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
            "fileExplorer" | "fileTransfer" | "transfers" => Some(NavItem::Transfers),
            "settings" => Some(NavItem::Settings),
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
    pub(crate) fn i18n_key(self) -> &'static str {
        match self {
            Self::LeftTop => "activityBar.leftTop",
            Self::LeftBottom => "activityBar.leftBottom",
            Self::RightTop => "activityBar.rightTop",
            Self::RightBottom => "activityBar.rightBottom",
        }
    }

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
    pub(crate) fn i18n_key(self) -> Option<&'static str> {
        match self {
            Self::Panel(item) => item.i18n_key(),
            Self::QuickCommands => Some("panel.quickCommands"),
            Self::CommandSend => Some("panel.serialSend"),
            Self::Recording => Some("recording.panelTitle"),
            Self::Lock => Some("statusBar.lock"),
        }
    }

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

    pub(crate) fn icon_path(self) -> &'static str {
        match self {
            Self::Panel(item) => item.icon_path(),
            Self::QuickCommands => "icons/commands.svg",
            Self::CommandSend => "icons/send.svg",
            Self::Recording => "icons/record.svg",
            Self::Lock => "icons/lock.svg",
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

    pub(crate) fn side_for_entry(&self, entry_id: &str) -> Option<PanelSide> {
        self.find_entry(entry_id).map(|(zone, _)| match zone {
            ActivityBarZone::LeftTop | ActivityBarZone::LeftBottom => PanelSide::Left,
            ActivityBarZone::RightTop | ActivityBarZone::RightBottom => PanelSide::Right,
        })
    }

    pub(crate) fn first_panel_on_side(&self, side: PanelSide) -> Option<NavItem> {
        let zones = match side {
            PanelSide::Left => [ActivityBarZone::LeftTop, ActivityBarZone::LeftBottom],
            PanelSide::Right => [ActivityBarZone::RightTop, ActivityBarZone::RightBottom],
        };
        zones
            .into_iter()
            .flat_map(|zone| self.zone(zone))
            .find_map(|id| NavItem::from_persistence_id(id).filter(|item| !item.opens_settings()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActivityBarContextMenuState {
    pub(crate) entry_id: String,
    pub(crate) zone: ActivityBarZone,
    pub(crate) index: usize,
    pub(crate) x: Pixels,
    pub(crate) y: Pixels,
    pub(crate) move_submenu_open: bool,
}

/// Top menubar dropdown (Tauri Header File/View/Terminal/Help).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TitleMenu {
    File,
    View,
    Terminal,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TabActionsSubmenu {
    Color,
    SshAdvanced,
    Ai,
}

impl TitleMenu {
    pub(crate) fn i18n_key(self) -> &'static str {
        match self {
            Self::File => "menu.file",
            Self::View => "menu.view",
            Self::Terminal => "menu.terminal",
            Self::Help => "menu.help",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::View => "View",
            Self::Terminal => "Terminal",
            Self::Help => "Help",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanelSide {
    Left,
    Right,
}

pub(crate) fn panel_collapsed_from_persistence(
    configured_collapsed: bool,
    multi_open: bool,
    has_active_panel: bool,
    has_open_stack: bool,
) -> bool {
    configured_collapsed || (!has_active_panel && (!multi_open || !has_open_stack))
}

#[cfg(test)]
mod tests {
    use super::{
        ActivityBarEntry, ActivityBarLayoutState, NavItem, PanelSide,
        panel_collapsed_from_persistence,
    };

    #[test]
    fn retired_migration_panel_is_ignored_when_loading_persisted_layouts() {
        assert_eq!(NavItem::from_persistence_id("migration"), None);
        assert_eq!(ActivityBarEntry::from_persistence_id("migration"), None);
    }

    #[test]
    fn activity_bar_entry_side_follows_current_layout() {
        let mut layout = ActivityBarLayoutState::default();
        assert_eq!(layout.side_for_entry("fileExplorer"), Some(PanelSide::Left));

        layout.left_top.retain(|id| id != "fileExplorer");
        layout.right_bottom.push("fileExplorer".to_string());

        assert_eq!(
            layout.side_for_entry("fileExplorer"),
            Some(PanelSide::Right)
        );
        assert_eq!(
            layout.first_panel_on_side(PanelSide::Right),
            Some(NavItem::Connections)
        );
        assert_eq!(layout.side_for_entry("missing"), None);
    }

    #[test]
    fn persisted_null_panel_closes_only_an_empty_side() {
        assert!(panel_collapsed_from_persistence(false, false, false, false));
        assert!(!panel_collapsed_from_persistence(false, false, true, false));
        assert!(!panel_collapsed_from_persistence(false, true, false, true));
        assert!(panel_collapsed_from_persistence(false, true, false, false));
        assert!(panel_collapsed_from_persistence(true, true, true, true));
    }
}
