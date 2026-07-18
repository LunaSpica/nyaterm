#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteProcessSortKey {
    Cpu,
    Memory,
    Pid,
    User,
    Command,
}

impl RemoteProcessSortKey {
    pub(crate) fn label(self) -> &'static str {
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
pub(crate) enum RemoteProcessSortDirection {
    Ascending,
    Descending,
}

impl RemoteProcessSortDirection {
    pub(crate) fn marker(self) -> &'static str {
        match self {
            Self::Ascending => "↑",
            Self::Descending => "↓",
        }
    }

    pub(crate) fn reversed(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteProcessSignalConfirmState {
    pub(crate) pid: u32,
    pub(crate) signal: &'static str,
    pub(crate) command: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DockerConfirmAction {
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
pub(crate) struct DockerConfirmState {
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) action: DockerConfirmAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DockerTab {
    Containers,
    Images,
    Volumes,
    Networks,
    Compose,
}

impl DockerTab {
    pub(crate) fn label(self) -> &'static str {
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
pub(crate) enum MainMode {
    Workspace,
    Page,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsTab {
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
    pub(crate) fn label(self) -> &'static str {
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

    pub(crate) fn group_label(self) -> &'static str {
        match self {
            Self::General | Self::Appearance | Self::Interaction | Self::Keybindings => "Workspace",
            Self::TerminalGeneral | Self::Search | Self::Translation => "Terminal Session",
            Self::AiGeneral | Self::AiModels | Self::AiRules => "AI",
            Self::Transfer => "Transfer",
            Self::Security => "Security",
            Self::SyncBackup => "Sync Backup",
        }
    }

    pub(crate) fn expandable_group_id(self) -> Option<&'static str> {
        match self {
            Self::General | Self::Appearance | Self::Interaction | Self::Keybindings => {
                Some("workspace")
            }
            Self::TerminalGeneral | Self::Search | Self::Translation => Some("terminal_session"),
            Self::AiGeneral | Self::AiModels | Self::AiRules => Some("ai_group"),
            Self::Transfer | Self::Security | Self::SyncBackup => None,
        }
    }
}
