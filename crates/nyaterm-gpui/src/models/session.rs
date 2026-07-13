use nyaterm_core::{
    AiExecutionProfile,
    SavedConnection,
};
use nyaterm_transport::{
    LocalSessionConfig, SerialSessionConfig,
    SshSessionConfig, TelnetSessionConfig,
};

#[derive(Clone)]
pub(crate) struct SessionRuntimeMetadata {
    pub(crate) ssh_config: Option<SshSessionConfig>,
    pub(crate) ssh_multiplex_key: Option<String>,
    pub(crate) source_connection_id: Option<String>,
    pub(crate) ai_execution_profile: AiExecutionProfile,
    pub(crate) launch_config: SessionLaunchConfig,
    /// Backend closed while the tab is kept for reconnect (Tauri disconnected pane).
    pub(crate) disconnected: bool,
}

#[derive(Clone)]
pub(crate) struct StartupCommandRequest {
    pub(crate) command: String,
    pub(crate) delay_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SyncInputGroup {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) color: u32,
    pub(crate) session_ids: Vec<String>,
    pub(crate) paused_session_ids: Vec<String>,
    pub(crate) enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartupCommandAction {
    Duplicate,
    Multiplex,
}

impl StartupCommandAction {
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Duplicate => "Duplicate and Run Command",
            Self::Multiplex => "Multiplex and Run Command",
        }
    }

    pub(crate) fn placeholder(self) -> &'static str {
        match self {
            Self::Duplicate => "Command to run after duplicate",
            Self::Multiplex => "Command to run after multiplex",
        }
    }

    pub(crate) fn submit_label(self) -> &'static str {
        match self {
            Self::Duplicate => "Duplicate",
            Self::Multiplex => "Multiplex",
        }
    }

    pub(crate) fn status_opened(self) -> &'static str {
        match self {
            Self::Duplicate => "duplicate and run command opened",
            Self::Multiplex => "multiplex and run command opened",
        }
    }

    pub(crate) fn status_cancelled(self) -> &'static str {
        match self {
            Self::Duplicate => "duplicate and run command cancelled",
            Self::Multiplex => "multiplex and run command cancelled",
        }
    }
}

#[derive(Clone)]
pub(crate) enum SessionLaunchConfig {
    Local(LocalSessionConfig),
    Ssh(SshSessionConfig),
    Telnet(TelnetSessionConfig),
    Serial(SerialSessionConfig),
}

#[derive(Clone)]
pub(crate) enum QuickSwitchItem {
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
    pub(crate) fn id(&self) -> String {
        match self {
            Self::Session { id, .. } => format!("session:{id}"),
            Self::Connection { connection, .. } => format!("connection:{}", connection.id),
            Self::Pending { title, .. } => format!("pending:{title}"),
        }
    }

    pub(crate) fn title(&self) -> &str {
        match self {
            Self::Session { title, .. }
            | Self::Connection { title, .. }
            | Self::Pending { title, .. } => title,
        }
    }

    pub(crate) fn subtitle(&self) -> &str {
        match self {
            Self::Session { subtitle, .. }
            | Self::Connection { subtitle, .. }
            | Self::Pending { subtitle, .. } => subtitle,
        }
    }

    pub(crate) fn search_text(&self) -> String {
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
pub(crate) enum TerminalSearchMode {
    Buffer,
    History,
}

impl TerminalSearchMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Buffer => "Buffer",
            Self::History => "History",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StoreStatus {
    pub(crate) path: String,
    pub(crate) message: String,
    pub(crate) ready: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct MultiLinePasteDraft {
    pub(crate) text: String,
}

impl MultiLinePasteDraft {
    pub(crate) fn new(text: String) -> Self {
        Self { text }
    }

    pub(crate) fn normalized_text(&self) -> String {
        normalize_paste_newlines(&self.text)
    }

    pub(crate) fn line_count(&self) -> usize {
        count_paste_lines(&self.text)
    }

    pub(crate) fn character_count(&self) -> usize {
        self.text.chars().count()
    }
}

pub(crate) fn normalize_paste_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

pub(crate) fn is_multi_line_paste(text: &str) -> bool {
    normalize_paste_newlines(text).contains('\n')
}

pub(crate) fn count_paste_lines(text: &str) -> usize {
    normalize_paste_newlines(text).split('\n').count()
}
