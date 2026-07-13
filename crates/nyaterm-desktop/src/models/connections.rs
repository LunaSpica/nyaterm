use gpui::Pixels;
use nyaterm_core::{CredentialPromptKind, SavedCredential};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionKindTab {
    Ssh,
    Local,
    Telnet,
    Serial,
}

impl ConnectionKindTab {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Ssh => "SSH",
            Self::Local => "Local",
            Self::Telnet => "Telnet",
            Self::Serial => "Serial",
        }
    }

    pub(crate) fn next(self) -> Self {
        match self {
            Self::Ssh => Self::Local,
            Self::Local => Self::Telnet,
            Self::Telnet => Self::Serial,
            Self::Serial => Self::Ssh,
        }
    }

    pub(crate) fn from_connection_type(config: &nyaterm_core::ConnectionType) -> Self {
        match config {
            nyaterm_core::ConnectionType::Ssh { .. } => Self::Ssh,
            nyaterm_core::ConnectionType::LocalTerminal { .. } => Self::Local,
            nyaterm_core::ConnectionType::Telnet { .. } => Self::Telnet,
            nyaterm_core::ConnectionType::Serial { .. } => Self::Serial,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionEditorField {
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
    pub(crate) fn next(self, kind: ConnectionKindTab, auth_mode: &str) -> Self {
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
pub(crate) struct ConnectionEditorState {
    pub(crate) id: Option<String>,
    pub(crate) kind: ConnectionKindTab,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) group_id: Option<String>,
    pub(crate) host: String,
    pub(crate) port: String,
    pub(crate) username: String,
    pub(crate) auth_mode: String,
    pub(crate) password: String,
    pub(crate) existing_password: Option<String>,
    pub(crate) key_id: Option<String>,
    pub(crate) otp_id: Option<String>,
    pub(crate) auto_fill_otp: bool,
    pub(crate) proxy_id: Option<String>,
    pub(crate) proxy_jump_id: Option<String>,
    pub(crate) x11_forwarding: bool,
    pub(crate) backspace_mode: String,
    pub(crate) shell_path: String,
    pub(crate) shell_args: String,
    pub(crate) working_dir: String,
    pub(crate) serial_port: String,
    pub(crate) baud_rate: String,
    pub(crate) data_bits: String,
    pub(crate) parity: String,
    pub(crate) stop_bits: String,
    pub(crate) raw_tcp_cli: bool,
    pub(crate) local_echo: bool,
    pub(crate) post_login_enabled: bool,
    pub(crate) post_login_command: String,
    pub(crate) post_login_delay_ms: String,
    pub(crate) connect_after_save: bool,
    pub(crate) focused_field: ConnectionEditorField,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectionGroupEditorState {
    pub(crate) id: Option<String>,
    pub(crate) name: String,
    pub(crate) parent_id: Option<String>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectionDeleteConfirmState {
    pub(crate) connection_id: String,
    pub(crate) label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConnectionContextMenuState {
    pub(crate) connection_id: String,
    pub(crate) x: Pixels,
    pub(crate) y: Pixels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionLinkMenuAction {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) command: Option<String>,
    pub(crate) open_url: Option<String>,
    pub(crate) is_default: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActionLinkMenuState {
    pub(crate) x: Pixels,
    pub(crate) y: Pixels,
    pub(crate) kind_label: String,
    pub(crate) value: String,
    pub(crate) actions: Vec<ActionLinkMenuAction>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActionLinkTooltipState {
    pub(crate) x: Pixels,
    pub(crate) y: Pixels,
    pub(crate) kind_label: String,
    pub(crate) value: String,
    pub(crate) default_action_label: String,
    pub(crate) default_action_preview: String,
    pub(crate) has_more_actions: bool,
    /// Identity key for hover stability (kind|value|start|end).
    pub(crate) match_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TranslationDialogState {
    pub(crate) source_text: String,
    pub(crate) provider: String,
    pub(crate) provider_label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CommandSuggestionItem {
    pub(crate) command: String,
    pub(crate) display: String,
    pub(crate) source: String,
    pub(crate) score: u32,
    pub(crate) indices: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CommandSuggestionState {
    pub(crate) draft: String,
    pub(crate) items: Vec<CommandSuggestionItem>,
    pub(crate) selected_index: usize,
    pub(crate) cursor_row: usize,
    pub(crate) cursor_col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CredentialSuggestionState {
    pub(crate) kind: CredentialPromptKind,
    pub(crate) matches: Vec<SavedCredential>,
    pub(crate) prompt_text: String,
    pub(crate) selected_index: usize,
    pub(crate) cursor_row: usize,
    pub(crate) cursor_col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingCredentialAutofill {
    pub(crate) credential_id: String,
    pub(crate) expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TerminalContextMenuState {
    pub(crate) x: Pixels,
    pub(crate) y: Pixels,
    /// Snapshot of selected text when the menu opened (Tauri caches selection).
    pub(crate) selected_text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConnectionGroupContextMenuState {
    pub(crate) group_id: String,
    pub(crate) x: Pixels,
    pub(crate) y: Pixels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectionGroupDeleteConfirmState {
    pub(crate) group_id: String,
    pub(crate) label: String,
    pub(crate) connection_count: usize,
    pub(crate) child_group_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionSortMode {
    Default,
    NameAsc,
    NameDesc,
    Recent,
}

impl ConnectionSortMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::NameAsc => "Name A-Z",
            Self::NameDesc => "Name Z-A",
            Self::Recent => "Recent",
        }
    }

    pub(crate) fn next(self) -> Self {
        match self {
            Self::Default => Self::NameAsc,
            Self::NameAsc => Self::NameDesc,
            Self::NameDesc => Self::Recent,
            Self::Recent => Self::Default,
        }
    }
}
