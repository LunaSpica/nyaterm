use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ai::RiskLevel;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AiExecutionProfile {
    #[default]
    Auto,
    Posix,
    Powershell,
    Cmd,
    SendOnly,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SshAlgorithmMode {
    #[default]
    Compatible,
    Secure,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SshAlgorithmPreferences {
    #[serde(default)]
    pub mode: SshAlgorithmMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kex: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ciphers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub macs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub host_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SftpCwdFollowMode {
    Off,
    #[default]
    ShellIntegration,
    RcFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SftpSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub cwd_follow_mode: SftpCwdFollowMode,
    #[serde(
        default = "default_sftp_shell_detection_timeout_ms",
        skip_serializing_if = "is_default_sftp_shell_detection_timeout_ms"
    )]
    pub shell_detection_timeout_ms: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub filename_encoding: String,
}

impl Default for SftpSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            cwd_follow_mode: SftpCwdFollowMode::ShellIntegration,
            shell_detection_timeout_ms: default_sftp_shell_detection_timeout_ms(),
            filename_encoding: String::new(),
        }
    }
}

pub const MIN_SFTP_SHELL_DETECTION_TIMEOUT_MS: u64 = 100;
pub const MAX_SFTP_SHELL_DETECTION_TIMEOUT_MS: u64 = 60_000;

pub fn validate_sftp_settings(settings: &SftpSettings) -> Result<(), String> {
    if !(MIN_SFTP_SHELL_DETECTION_TIMEOUT_MS..=MAX_SFTP_SHELL_DETECTION_TIMEOUT_MS)
        .contains(&settings.shell_detection_timeout_ms)
    {
        return Err(format!(
            "SFTP shell detection timeout must be between {} and {} ms",
            MIN_SFTP_SHELL_DETECTION_TIMEOUT_MS, MAX_SFTP_SHELL_DETECTION_TIMEOUT_MS
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelnetAutoLoginConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub send_wake_enter: bool,
    #[serde(default = "default_telnet_auto_login_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username_prompt_regex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password_prompt_regex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_prompt_regex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_prompt_regex: Option<String>,
    #[serde(default)]
    pub max_retries: u8,
}

impl Default for TelnetAutoLoginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            send_wake_enter: true,
            timeout_ms: default_telnet_auto_login_timeout_ms(),
            username_prompt_regex: None,
            password_prompt_regex: None,
            success_prompt_regex: None,
            failure_prompt_regex: None,
            max_retries: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConnectionType {
    Ssh {
        host: String,
        #[serde(default = "default_ssh_port")]
        port: u16,
        #[serde(default = "default_ssh_user")]
        username: String,
        #[serde(default = "default_backspace_mode_ssh")]
        backspace_mode: String,
        #[serde(default)]
        ai_execution_profile: AiExecutionProfile,
        #[serde(default)]
        x11_forwarding: bool,
        #[serde(default)]
        encoding: String,
    },
    LocalTerminal {
        #[serde(default)]
        shell_path: String,
        #[serde(default)]
        shell_args: String,
        #[serde(default)]
        working_dir: Option<String>,
        #[serde(default)]
        ai_execution_profile: AiExecutionProfile,
        #[serde(default)]
        encoding: String,
    },
    Telnet {
        host: String,
        #[serde(default = "default_telnet_port")]
        port: u16,
        #[serde(default)]
        username: String,
        #[serde(default)]
        ai_execution_profile: AiExecutionProfile,
        #[serde(default = "default_backspace_mode_telnet")]
        backspace_mode: String,
        #[serde(default)]
        raw_tcp_cli: bool,
        #[serde(default = "default_telnet_enter_mode")]
        enter_mode: String,
        #[serde(default)]
        local_echo: bool,
        #[serde(default)]
        local_line_edit: bool,
        #[serde(default)]
        force_character_at_a_time: bool,
        #[serde(default = "default_true")]
        send_naws: bool,
        #[serde(default = "default_true")]
        send_sga: bool,
        #[serde(default, skip_serializing_if = "is_default_telnet_auto_login_config")]
        auto_login: TelnetAutoLoginConfig,
        #[serde(default)]
        encoding: String,
    },
    Serial {
        port_name: String,
        #[serde(default = "default_baud_rate")]
        baud_rate: u32,
        #[serde(default = "default_data_bits")]
        data_bits: u8,
        #[serde(default = "default_parity")]
        parity: String,
        #[serde(default = "default_stop_bits")]
        stop_bits: String,
        #[serde(default)]
        ai_execution_profile: AiExecutionProfile,
        #[serde(default = "default_backspace_mode_serial")]
        backspace_mode: String,
        #[serde(default)]
        encoding: String,
    },
    Rdp {
        host: String,
        #[serde(default = "default_rdp_port")]
        port: u16,
        #[serde(default)]
        username: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        domain: String,
        #[serde(default)]
        security: RdpSecuritySettings,
        #[serde(default)]
        display: RdpDisplaySettings,
        #[serde(default)]
        clipboard: RdpClipboardSettings,
        #[serde(default)]
        reconnect: RdpReconnectSettings,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RdpSecuritySettings {
    #[serde(default = "default_true")]
    pub use_nla: bool,
    #[serde(default = "default_rdp_certificate_policy")]
    pub certificate_policy: String,
}

impl Default for RdpSecuritySettings {
    fn default() -> Self {
        Self {
            use_nla: true,
            certificate_policy: default_rdp_certificate_policy(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RdpDisplaySettings {
    #[serde(default = "default_rdp_display_mode")]
    pub mode: String,
    #[serde(default = "default_rdp_width")]
    pub width: u32,
    #[serde(default = "default_rdp_height")]
    pub height: u32,
    #[serde(default = "default_rdp_color_depth")]
    pub color_depth: u8,
}

impl Default for RdpDisplaySettings {
    fn default() -> Self {
        Self {
            mode: default_rdp_display_mode(),
            width: default_rdp_width(),
            height: default_rdp_height(),
            color_depth: default_rdp_color_depth(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RdpClipboardSettings {
    #[serde(default = "default_rdp_clipboard_mode")]
    pub mode: String,
}

impl Default for RdpClipboardSettings {
    fn default() -> Self {
        Self {
            mode: default_rdp_clipboard_mode(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RdpReconnectSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_rdp_reconnect_attempts")]
    pub max_attempts: u32,
}

impl Default for RdpReconnectSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: default_rdp_reconnect_attempts(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ConnectionAuth {
    #[serde(default = "default_auth_mode")]
    pub mode: String,
    #[serde(default)]
    pub password_id: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub key_id: Option<String>,
    #[serde(default)]
    pub otp_id: Option<String>,
    #[serde(default)]
    pub auto_fill_otp: bool,
    #[serde(default)]
    pub has_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SshKey {
    #[serde(default = "uuid_v4")]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub cert: Option<String>,
    #[serde(default)]
    pub passphrase: Option<String>,
    #[serde(default, skip_serializing)]
    pub key_file_path: Option<String>,
    #[serde(default, skip_serializing)]
    pub cert_file_path: Option<String>,
    #[serde(default, skip_serializing)]
    pub has_key_data: bool,
    #[serde(default, skip_serializing)]
    pub has_cert_data: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecryptedSshKey {
    pub id: String,
    pub name: String,
    pub key_data: Option<String>,
    pub cert_data: Option<String>,
    pub passphrase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedPassword {
    #[serde(default = "uuid_v4")]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default, skip_serializing)]
    pub has_password: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecryptedSavedPassword {
    pub id: String,
    pub name: String,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedCredential {
    #[serde(default = "uuid_v4")]
    pub id: String,
    pub name: String,
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub username_prompt_regex: Option<String>,
    #[serde(default)]
    pub password_prompt_regex: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing)]
    pub has_password: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecryptedSavedCredential {
    pub id: String,
    pub name: String,
    pub username: String,
    pub password: Option<String>,
    pub username_prompt_regex: Option<String>,
    pub password_prompt_regex: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OtpEntry {
    #[serde(default = "uuid_v4")]
    pub id: String,
    #[serde(default = "default_otp_type")]
    pub otp_type: String,
    #[serde(default)]
    pub issuer: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub secret: Option<String>,
    #[serde(default = "default_otp_algorithm")]
    pub algorithm: String,
    #[serde(default = "default_otp_digits")]
    pub digits: u8,
    #[serde(default = "default_otp_period")]
    pub period: u64,
    #[serde(default)]
    pub counter: u64,
    #[serde(default, skip_serializing)]
    pub has_secret: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecryptedOtpEntry {
    pub id: String,
    pub otp_type: String,
    pub issuer: String,
    pub username: String,
    pub secret: Option<String>,
    pub algorithm: String,
    pub digits: u8,
    pub period: u64,
    pub counter: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ConnectionNetwork {
    #[serde(default)]
    pub proxy_id: Option<String>,
    #[serde(default)]
    pub proxy_jump_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TunnelConfig {
    #[serde(default = "uuid_v4")]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_tunnel_type")]
    pub tunnel_type: String,
    #[serde(default)]
    pub connection_id: Option<String>,
    #[serde(default)]
    pub listen_port: u16,
    #[serde(default = "default_tunnel_target_host")]
    pub target_host: String,
    #[serde(default)]
    pub target_port: u16,
    #[serde(default)]
    pub is_open: bool,
    #[serde(default)]
    pub auto_open: bool,
    #[serde(default = "default_true")]
    pub bind_localhost: bool,
    #[serde(default)]
    pub group_id: Option<String>,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            id: uuid_v4(),
            name: String::new(),
            tunnel_type: default_tunnel_type(),
            connection_id: None,
            listen_port: 0,
            target_host: default_tunnel_target_host(),
            target_port: 0,
            is_open: false,
            auto_open: false,
            bind_localhost: true,
            group_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TunnelGroup {
    #[serde(default = "uuid_v4")]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub sort_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TunnelsConfig {
    #[serde(default)]
    pub tunnels: Vec<TunnelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TunnelGroupsConfig {
    #[serde(default)]
    pub groups: Vec<TunnelGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyConfig {
    #[serde(default = "uuid_v4")]
    pub id: String,
    pub name: String,
    #[serde(default = "default_proxy_protocol")]
    pub protocol: String,
    #[serde(default = "default_proxy_host")]
    pub host: String,
    #[serde(default = "default_proxy_port")]
    pub port: u16,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub password_id: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            id: uuid_v4(),
            name: String::new(),
            protocol: default_proxy_protocol(),
            host: default_proxy_host(),
            port: default_proxy_port(),
            command: None,
            username: None,
            password: None,
            password_id: None,
            group_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyGroup {
    #[serde(default = "uuid_v4")]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub sort_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ProxyGroupsConfig {
    #[serde(default)]
    pub groups: Vec<ProxyGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuickCommandCategory {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuickCommand {
    pub id: String,
    pub label: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_level: Option<RiskLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct QuickCommandsConfig {
    #[serde(default)]
    pub commands: Vec<QuickCommand>,
    #[serde(default)]
    pub categories: Vec<QuickCommandCategory>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct QuickCommandsExportConfig {
    pub categories: Vec<QuickCommandCategoryExport>,
    pub commands: Vec<QuickCommandExport>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct QuickCommandCategoryExport {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct QuickCommandExport {
    pub id: String,
    pub label: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_tag: Option<String>,
    pub pinned: bool,
    pub execution_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_level: Option<String>,
}

impl From<QuickCommandsConfig> for QuickCommandsExportConfig {
    fn from(config: QuickCommandsConfig) -> Self {
        Self {
            categories: config
                .categories
                .into_iter()
                .map(|category| QuickCommandCategoryExport {
                    id: category.id,
                    name: category.name,
                    parent_id: None,
                    sort_order: 0,
                })
                .collect(),
            commands: config
                .commands
                .into_iter()
                .map(|command| QuickCommandExport {
                    id: command.id,
                    label: command.label,
                    command: command.command,
                    category_id: command.category_id,
                    description: command.description,
                    color_tag: command.color_tag,
                    icon_tag: command.icon_tag,
                    pinned: command.pinned.unwrap_or_default(),
                    execution_mode: command
                        .execution_mode
                        .unwrap_or_else(|| "execute".to_string()),
                    source: command.source,
                    risk_level: command
                        .risk_level
                        .map(|risk| quick_command_export_risk_label(&risk).to_string()),
                })
                .collect(),
        }
    }
}

pub fn export_quick_commands_json(
    config: QuickCommandsConfig,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&QuickCommandsExportConfig::from(config))
}

fn quick_command_export_risk_label(risk: &RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Low => "low",
        RiskLevel::Medium => "medium",
        RiskLevel::High => "high",
        RiskLevel::Critical => "critical",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandHistoryEntry {
    pub command: String,
    pub last_used_at_ms: u64,
    pub use_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FuzzyResult {
    pub command: String,
    pub score: u32,
    pub indices: Vec<u32>,
    pub source: String,
    pub display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionPostLogin {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub command: String,
    #[serde(default = "default_post_login_delay_ms")]
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ConnectionRecordingSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_start: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<RecordingMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_template: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_timestamps: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<RecordingRotationPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedConnection {
    #[serde(default = "uuid_v4")]
    pub id: String,
    pub name: String,
    #[serde(flatten)]
    pub config: ConnectionType,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub icon: Option<String>,
    /// Whether `icon` may be replaced by one detected from the remote system.
    ///
    /// `None` means "not configured", which reads as enabled only while no icon
    /// has been chosen — see [`SavedConnection::icon_auto_detect_enabled`]. Kept
    /// as an `Option` and skipped when empty so files round-trip unchanged
    /// through builds that predate the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_auto_detect: Option<bool>,
    #[serde(default)]
    pub auth: Option<ConnectionAuth>,
    #[serde(default)]
    pub network: Option<ConnectionNetwork>,
    #[serde(default)]
    pub post_login: Option<ConnectionPostLogin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recording: Option<ConnectionRecordingSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh_algorithms: Option<SshAlgorithmPreferences>,
    #[serde(default, skip_serializing_if = "is_default_sftp_settings")]
    pub sftp: SftpSettings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used_at_ms: Option<u64>,
}

impl SavedConnection {
    /// Whether the icon may be replaced by one inferred from the remote system.
    ///
    /// An unset flag defaults to "yes, until the user picks something", which is
    /// what keeps auto-detection from ever overwriting a deliberate choice made
    /// before this field existed.
    pub fn icon_auto_detect_enabled(&self) -> bool {
        self.icon_auto_detect
            .unwrap_or_else(|| self.icon.as_deref().is_none_or(str::is_empty))
    }

    pub fn kind_label(&self) -> &'static str {
        match self.config {
            ConnectionType::Ssh { .. } => "SSH",
            ConnectionType::LocalTerminal { .. } => "Local",
            ConnectionType::Telnet { .. } => "Telnet",
            ConnectionType::Serial { .. } => "Serial",
            ConnectionType::Rdp { .. } => "RDP",
        }
    }

    pub fn endpoint(&self) -> String {
        match &self.config {
            ConnectionType::Ssh {
                host,
                port,
                username,
                ..
            } => format!("{username}@{host}:{port}"),
            ConnectionType::LocalTerminal {
                shell_path,
                working_dir,
                ..
            } => {
                let shell = if shell_path.is_empty() {
                    "system shell"
                } else {
                    shell_path
                };
                match working_dir {
                    Some(dir) if !dir.is_empty() => format!("{shell} in {dir}"),
                    _ => shell.to_string(),
                }
            }
            ConnectionType::Telnet { host, port, .. } => format!("{host}:{port}"),
            ConnectionType::Serial {
                port_name,
                baud_rate,
                ..
            } => format!("{port_name} @ {baud_rate}"),
            ConnectionType::Rdp {
                host,
                port,
                username,
                domain,
                ..
            } => {
                let account = if username.is_empty() {
                    String::new()
                } else if domain.is_empty() {
                    format!("{username}@")
                } else {
                    format!("{domain}\\{username}@")
                };
                format!("{account}{host}:{port}")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Group {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SessionsConfig {
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    #[serde(alias = "sessions")]
    pub connections: Vec<SavedConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionLinksMatcherSettings {
    #[serde(default = "default_true_action_link")]
    pub ipv4: bool,
    #[serde(default = "default_true_action_link")]
    pub archive: bool,
    #[serde(default = "default_true_action_link")]
    pub host_port: bool,
}

fn default_true_action_link() -> bool {
    true
}

impl Default for ActionLinksMatcherSettings {
    fn default() -> Self {
        Self {
            ipv4: true,
            archive: true,
            host_port: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchEngineConfig {
    pub name: String,
    pub url_template: String,
    /// Optional icon key (Tauri SEARCH_ICONS: google/bing/github/...).
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default = "default_true_search_menu")]
    pub show_in_menu: bool,
}

fn default_true_search_menu() -> bool {
    true
}

pub fn default_search_engines() -> Vec<SearchEngineConfig> {
    vec![
        SearchEngineConfig {
            name: "Google".to_string(),
            url_template: "https://www.google.com/search?q=%s".to_string(),
            icon: Some("google".to_string()),
            show_in_menu: true,
        },
        SearchEngineConfig {
            name: "Bing".to_string(),
            url_template: "https://www.bing.com/search?q=%s".to_string(),
            icon: Some("bing".to_string()),
            show_in_menu: true,
        },
        SearchEngineConfig {
            name: "GitHub".to_string(),
            url_template: "https://github.com/search?q=%s".to_string(),
            icon: Some("github".to_string()),
            show_in_menu: true,
        },
    ]
}

/// Tauri per-tab pane tree node (`RestorablePaneNode` in ui.open_tabs[].root).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum RestorablePaneNode {
    #[serde(rename = "leaf")]
    Leaf {
        #[serde(default)]
        id: String,
        #[serde(default)]
        title: String,
        #[serde(default)]
        session_type: String,
        #[serde(default)]
        connection_id: Option<String>,
    },
    #[serde(rename = "split")]
    Split {
        #[serde(default)]
        id: String,
        direction: String,
        #[serde(default = "default_restorable_split_ratio")]
        ratio: f64,
        first: Box<RestorablePaneNode>,
        second: Box<RestorablePaneNode>,
    },
}

impl RestorablePaneNode {
    /// Flatten session leaves in left-to-right / top-to-bottom order.
    pub fn collect_leaves(&self) -> Vec<RestorablePaneLeaf> {
        let mut out = Vec::new();
        self.collect_leaves_into(&mut out);
        out
    }

    fn collect_leaves_into(&self, out: &mut Vec<RestorablePaneLeaf>) {
        match self {
            Self::Leaf {
                id,
                title,
                session_type,
                connection_id,
            } => out.push(RestorablePaneLeaf {
                id: id.clone(),
                title: title.clone(),
                session_type: session_type.clone(),
                connection_id: connection_id.clone(),
            }),
            Self::Split { first, second, .. } => {
                first.collect_leaves_into(out);
                second.collect_leaves_into(out);
            }
        }
    }

    /// Map this pane tree onto ordered open-tab indexes starting at `base_index`.
    pub fn to_workspace_pane_layout(
        &self,
        base_index: usize,
    ) -> Option<RestorableWorkspacePaneNode> {
        let mut next = base_index;
        self.to_workspace_pane_layout_inner(&mut next)
    }

    fn to_workspace_pane_layout_inner(
        &self,
        next_index: &mut usize,
    ) -> Option<RestorableWorkspacePaneNode> {
        match self {
            Self::Leaf { .. } => {
                let tab_index = *next_index;
                *next_index += 1;
                Some(RestorableWorkspacePaneNode::Leaf { tab_index })
            }
            Self::Split {
                id,
                direction,
                ratio,
                first,
                second,
            } => {
                let first = first.to_workspace_pane_layout_inner(next_index);
                let second = second.to_workspace_pane_layout_inner(next_index);
                match (first, second) {
                    (None, None) => None,
                    (Some(only), None) | (None, Some(only)) => Some(only),
                    (Some(first), Some(second)) => Some(RestorableWorkspacePaneNode::Split {
                        id: if id.trim().is_empty() {
                            format!("pane-{}", *next_index)
                        } else {
                            id.clone()
                        },
                        direction: direction.clone(),
                        ratio: (*ratio).clamp(0.2, 0.8),
                        first: Box::new(first),
                        second: Box::new(second),
                    }),
                }
            }
        }
    }

    pub fn leaf_session(
        title: impl Into<String>,
        session_type: impl Into<String>,
        connection_id: Option<String>,
    ) -> Self {
        Self::Leaf {
            id: String::new(),
            title: title.into(),
            session_type: session_type.into(),
            connection_id,
        }
    }
}

/// One restorable session leaf extracted from a tab pane tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestorablePaneLeaf {
    pub id: String,
    pub title: String,
    pub session_type: String,
    pub connection_id: Option<String>,
}

/// Tauri `ui.open_tabs` entry (native restores connection/local leaf sessions).
/// Optional `root` preserves Tauri per-tab pane trees for interop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RestorableOpenTab {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub session_type: String,
    #[serde(default)]
    pub connection_id: Option<String>,
    #[serde(default)]
    pub custom_name: Option<String>,
    #[serde(default)]
    pub tab_color: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub locked: bool,
    #[serde(default)]
    pub active_pane_id: Option<String>,
    #[serde(default)]
    pub root: Option<RestorablePaneNode>,
}

impl RestorableOpenTab {
    /// Expand this tab into one or more session restore descriptors.
    /// Split roots become multiple leaves (native global sessions).
    pub fn expanded_sessions(&self) -> Vec<RestorableOpenTabSession> {
        if let Some(root) = &self.root {
            let leaves = root.collect_leaves();
            if !leaves.is_empty() {
                return leaves
                    .into_iter()
                    .map(|leaf| {
                        let title = if leaf.title.trim().is_empty() {
                            self.title.clone()
                        } else {
                            leaf.title
                        };
                        let session_type = if leaf.session_type.trim().is_empty() {
                            self.session_type.clone()
                        } else {
                            leaf.session_type
                        };
                        let connection_id = leaf
                            .connection_id
                            .filter(|id| !id.is_empty())
                            .or_else(|| self.connection_id.clone());
                        RestorableOpenTabSession {
                            title,
                            session_type,
                            connection_id,
                            custom_name: self.custom_name.clone(),
                            tab_color: self.tab_color.clone(),
                            locked: self.locked,
                        }
                    })
                    .collect();
            }
        }

        vec![RestorableOpenTabSession {
            title: self.title.clone(),
            session_type: self.session_type.clone(),
            connection_id: self.connection_id.clone(),
            custom_name: self.custom_name.clone(),
            tab_color: self.tab_color.clone(),
            locked: self.locked,
        }]
    }

    /// If this tab carries a multi-pane root, map it onto ordered tab indexes
    /// starting at `base_index` (index of the first expanded leaf).
    pub fn workspace_pane_layout_from_root(
        &self,
        base_index: usize,
    ) -> Option<RestorableWorkspacePaneNode> {
        let root = self.root.as_ref()?;
        if matches!(root, RestorablePaneNode::Leaf { .. }) {
            return None;
        }
        let layout = root.to_workspace_pane_layout(base_index)?;
        match &layout {
            RestorableWorkspacePaneNode::Split { .. } => Some(layout),
            RestorableWorkspacePaneNode::Leaf { .. } => None,
        }
    }

    pub fn with_leaf_root(
        title: impl Into<String>,
        session_type: impl Into<String>,
        connection_id: Option<String>,
        custom_name: Option<String>,
        tab_color: Option<String>,
    ) -> Self {
        let title = title.into();
        let session_type = session_type.into();
        let root = RestorablePaneNode::leaf_session(
            title.clone(),
            session_type.clone(),
            connection_id.clone(),
        );
        Self {
            title,
            session_type,
            connection_id,
            custom_name,
            tab_color,
            locked: false,
            active_pane_id: None,
            root: Some(root),
        }
    }
}

/// Flattened session restore unit (one native tab / session).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestorableOpenTabSession {
    pub title: String,
    pub session_type: String,
    pub connection_id: Option<String>,
    pub custom_name: Option<String>,
    pub tab_color: Option<String>,
    pub locked: bool,
}

/// Native workspace pane split tree (indexes into ordered open tabs).
/// Distinct from Tauri per-tab pane trees: native H/V splits arrange sessions globally.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum RestorableWorkspacePaneNode {
    #[serde(rename = "leaf")]
    Leaf {
        #[serde(default)]
        tab_index: usize,
    },
    #[serde(rename = "split")]
    Split {
        #[serde(default)]
        id: String,
        direction: String,
        #[serde(default = "default_restorable_split_ratio")]
        ratio: f64,
        first: Box<RestorableWorkspacePaneNode>,
        second: Box<RestorableWorkspacePaneNode>,
    },
}

/// Tauri `ui.terminal_window_layout` node (indexes into ordered open tabs).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum RestorableTerminalWindowNode {
    #[serde(rename = "leaf")]
    Leaf {
        #[serde(default)]
        tab_indexes: Vec<usize>,
        #[serde(default)]
        active_tab_index: Option<usize>,
    },
    #[serde(rename = "split")]
    Split {
        direction: String,
        #[serde(default = "default_restorable_split_ratio")]
        ratio: f64,
        first: Box<RestorableTerminalWindowNode>,
        second: Box<RestorableTerminalWindowNode>,
    },
}

fn default_restorable_split_ratio() -> f64 {
    0.5
}

pub const DEFAULT_RECORDING_PATH_TEMPLATE: &str =
    "{group}/{session}/{yyyy}-{MM}-{dd}/{HH}-{mm}-{ss}-{SSS}-{session_short_id}.log";
pub const DEFAULT_TERMINAL_TIMESTAMP_FORMAT: &str = "[HH:mm:ss]";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RecordingMode {
    #[default]
    Transcript,
    Raw,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExistingFileBehavior {
    #[default]
    Unique,
    Append,
    Overwrite,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RecordingRotationPolicy {
    #[default]
    Session,
    Daily,
    Size {
        max_bytes: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettingsSummary {
    pub theme: String,
    #[serde(default)]
    pub background_image_path: Option<String>,
    #[serde(default = "default_background_image_fit")]
    pub background_image_fit: String,
    /// Wallpaper opacity percent (0..=100).
    #[serde(default = "default_background_image_opacity")]
    pub background_image_opacity: u8,
    /// Shell chrome opacity percent when wallpaper is active (0..=100).
    #[serde(default = "default_background_content_opacity")]
    pub background_content_opacity: u8,
    pub language: String,
    pub terminal_font_family: String,
    pub terminal_font_size: u16,
    /// Terminal cursor style: block | underline | bar (Tauri appearance.cursor_style).
    #[serde(default = "default_cursor_style")]
    pub cursor_style: String,
    /// Whether the terminal caret blinks (Tauri appearance.cursor_blink).
    #[serde(default = "default_cursor_blink")]
    pub cursor_blink: bool,
    /// Optional terminal color theme id; None / empty follows UI theme (Tauri appearance.terminal_theme).
    #[serde(default)]
    pub terminal_theme: Option<String>,
    /// Minimum contrast ratio for terminal colors (Tauri appearance.minimum_contrast_ratio).
    /// Stored as a display string: "1", "3", "4.5", "7", or "21".
    #[serde(default = "default_minimum_contrast_ratio")]
    pub minimum_contrast_ratio: String,
    /// UI chrome font family (Tauri appearance.ui_font_family).
    #[serde(default = "default_ui_font_family")]
    pub ui_font_family: String,
    /// UI chrome font size in px (Tauri appearance.ui_font_size).
    #[serde(default = "default_ui_font_size")]
    pub ui_font_size: u16,
    /// Terminal regular font weight (Tauri appearance.font_weight).
    #[serde(default = "default_terminal_font_weight")]
    pub terminal_font_weight: u16,
    /// Terminal bold font weight (Tauri appearance.font_weight_bold).
    #[serde(default = "default_terminal_font_weight_bold")]
    pub terminal_font_weight_bold: u16,
    pub x11_display: String,
    pub terminal_scrollback_lines: u32,
    #[serde(default = "default_terminal_keep_alive_mode")]
    pub terminal_keep_alive_mode: String,
    pub terminal_keep_alive_interval: u32,
    #[serde(default = "default_terminal_timestamp_format")]
    pub terminal_timestamp_format: String,
    pub terminal_hardware_acceleration: bool,
    pub terminal_show_workspace_padding: bool,
    pub terminal_show_line_numbers: bool,
    pub terminal_show_timestamps: bool,
    pub terminal_show_timestamp_milliseconds: bool,
    pub terminal_show_multi_line_paste_dialog: bool,
    pub terminal_paste_image_as_path: bool,
    #[serde(default)]
    pub terminal_low_latency_mode: bool,
    /// Detect clickable entities (IP/host:port/archive) in terminal output (Tauri action_links_enabled).
    #[serde(default)]
    pub terminal_action_links_enabled: bool,
    #[serde(default)]
    pub terminal_action_links_matchers: ActionLinksMatcherSettings,
    /// Online search engines for terminal selection context menu (Tauri search.custom_engines).
    #[serde(default = "default_search_engines")]
    pub search_custom_engines: Vec<SearchEngineConfig>,
    pub ui_show_remote_stats: bool,
    pub ui_remote_stats_interval: u32,
    #[serde(default)]
    pub ui_show_gpu_monitor: bool,
    #[serde(default = "default_hardware_monitor_interval")]
    pub ui_gpu_monitor_interval: u32,
    #[serde(default)]
    pub ui_show_ascend_npu_monitor: bool,
    #[serde(default = "default_hardware_monitor_interval")]
    pub ui_ascend_npu_monitor_interval: u32,
    pub ui_show_process_manager: bool,
    pub ui_process_manager_interval: u32,
    pub ui_show_docker_manager: bool,
    pub ui_docker_manager_interval: u32,
    #[serde(default = "default_quick_cmd_view_mode")]
    pub ui_quick_cmd_view_mode: String,
    #[serde(default = "default_quick_cmd_sort_mode")]
    pub ui_quick_cmd_sort_mode: String,
    #[serde(default = "default_saved_connections_sort_mode")]
    pub ui_saved_connections_sort_mode: String,
    #[serde(default)]
    pub ui_saved_connections_expanded_group_ids: Vec<String>,
    /// Which reading the title bar's centre shows: `session`, `resources`,
    /// `host` or `datetime`.
    #[serde(default = "default_header_status_mode")]
    pub ui_header_status_mode: String,
    #[serde(default = "default_true")]
    pub ui_header_status_visible: bool,
    #[serde(default = "default_true")]
    pub ui_file_explorer_show_hidden_files: bool,
    #[serde(default)]
    pub ui_file_explorer_auto_sync_cwd_connection_ids: Vec<String>,
    #[serde(default)]
    pub ui_file_explorer_favorite_dirs_by_connection_id: HashMap<String, Vec<String>>,
    #[serde(default = "default_left_panel_width")]
    pub ui_left_panel_width: u32,
    #[serde(default = "default_right_panel_width")]
    pub ui_right_panel_width: u32,
    #[serde(default = "default_transfer_height")]
    pub ui_transfer_height: u32,
    #[serde(default = "default_quick_cmd_height")]
    pub ui_quick_cmd_height: u32,
    /// Whether the Tauri-compatible Quick Commands bottom panel is visible.
    #[serde(default = "default_true")]
    pub ui_quick_cmd_visible: bool,
    #[serde(default = "default_serial_send_height")]
    pub ui_serial_send_height: u32,
    /// Whether the Tauri-compatible Command Send bottom panel is visible.
    #[serde(default)]
    pub ui_serial_send_visible: bool,
    #[serde(default)]
    pub ui_active_left_panel: Option<String>,
    #[serde(default)]
    pub ui_active_right_panel: Option<String>,
    #[serde(default)]
    pub ui_left_panel_collapsed: bool,
    #[serde(default)]
    pub ui_right_panel_collapsed: bool,
    #[serde(default = "default_activity_left_top")]
    pub ui_activity_bar_left_top: Vec<String>,
    #[serde(default = "default_activity_left_bottom")]
    pub ui_activity_bar_left_bottom: Vec<String>,
    #[serde(default = "default_activity_right_top")]
    pub ui_activity_bar_right_top: Vec<String>,
    #[serde(default = "default_activity_right_bottom")]
    pub ui_activity_bar_right_bottom: Vec<String>,
    #[serde(default)]
    pub ui_activity_bar_show_labels: bool,
    #[serde(default)]
    pub ui_panel_multi_open: bool,
    #[serde(default)]
    pub ui_left_open_panels: Vec<String>,
    #[serde(default)]
    pub ui_right_open_panels: Vec<String>,
    #[serde(default)]
    pub ui_panel_stack_sizes: HashMap<String, u32>,
    pub interaction_copy_on_select: bool,
    #[serde(default)]
    pub interaction_allow_osc52_clipboard_write: bool,
    pub interaction_right_click_paste: bool,
    #[serde(default = "default_true")]
    pub interaction_terminal_zoom_enabled: bool,
    pub interaction_command_suggestions_enabled: bool,
    pub interaction_command_suggestion_min_chars: u32,
    pub interaction_command_suggestion_max_chars: u32,
    pub interaction_word_separators: String,
    pub interaction_duplicate_session_command_delay_ms: u32,
    pub interaction_alt_as_meta: bool,
    pub interaction_mac_ime_compatibility: bool,
    pub interaction_tab_double_click_action: String,
    pub interaction_tab_middle_click_action: String,
    pub interaction_tab_right_click_action: String,
    pub interaction_default_encoding: String,
    pub host_key_policy: String,
    pub transfer_download_path: String,
    pub transfer_ask_save_location: bool,
    pub transfer_duplicate_strategy: String,
    pub transfer_editor_type: String,
    pub transfer_default_editor: String,
    pub transfer_download_threads: u32,
    pub transfer_upload_threads: u32,
    pub transfer_max_retries: u32,
    pub transfer_buffer_size: u32,
    pub transfer_default_file_permissions: String,
    pub transfer_preserve_timestamps: bool,
    pub transfer_resume_broken_transfer: bool,
    pub recording_path: String,
    pub recording_auto_start: bool,
    #[serde(default)]
    pub recording_default_mode: RecordingMode,
    #[serde(default = "default_recording_path_template")]
    pub recording_path_template: String,
    pub recording_include_io_labels: bool,
    pub recording_include_timestamps: bool,
    #[serde(default = "default_true")]
    pub recording_include_session_metadata: bool,
    #[serde(default)]
    pub recording_rotation: RecordingRotationPolicy,
    #[serde(default)]
    pub recording_existing_file_behavior: ExistingFileBehavior,
    #[serde(default)]
    pub recording_include_binary_transfer_payloads: bool,
    pub recording_memory_limit_bytes: u64,
    pub diagnostics_level: String,
    pub diagnostics_retention_days: u32,
    pub startup_restore: bool,
    /// When true (default), restore multi-leaf tab window layout with sessions.
    #[serde(default = "default_true")]
    pub startup_restore_window_layout: bool,
    /// When true, minimize hides the main window to the system tray (platform-dependent).
    #[serde(default)]
    pub minimize_to_tray: bool,
    pub confirm_on_close: bool,
    pub enable_screen_lock: bool,
    pub idle_lock_minutes: u32,
    pub has_master_password: bool,
    pub keybindings: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeywordHighlightRule {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default = "default_highlight_color_dark")]
    pub color_dark: String,
    #[serde(default = "default_highlight_color_light")]
    pub color_light: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for KeywordHighlightRule {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            patterns: Vec::new(),
            color_dark: default_highlight_color_dark(),
            color_light: default_highlight_color_light(),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct KeywordHighlightConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub across_wrapped_lines: bool,
    /// Per built-in rule enable map (Tauri `keyword_highlight_builtin_rules`).
    #[serde(default)]
    pub builtin_rules: std::collections::HashMap<String, bool>,
    #[serde(default)]
    pub rules: Vec<KeywordHighlightRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordHighlightImportResult {
    pub imported_rules: usize,
    pub updated_rules: usize,
    pub total_rules: usize,
}

impl Default for AppSettingsSummary {
    fn default() -> Self {
        Self {
            theme: "github-dark".to_string(),
            background_image_path: None,
            background_image_fit: default_background_image_fit(),
            background_image_opacity: default_background_image_opacity(),
            background_content_opacity: default_background_content_opacity(),
            language: "zh-CN".to_string(),
            terminal_font_family: "JetBrains Mono".to_string(),
            terminal_font_size: 16,
            cursor_style: default_cursor_style(),
            cursor_blink: default_cursor_blink(),
            terminal_theme: None,
            minimum_contrast_ratio: default_minimum_contrast_ratio(),
            ui_font_family: default_ui_font_family(),
            ui_font_size: default_ui_font_size(),
            terminal_font_weight: default_terminal_font_weight(),
            terminal_font_weight_bold: default_terminal_font_weight_bold(),
            x11_display: String::new(),
            terminal_scrollback_lines: 5000,
            terminal_keep_alive_mode: default_terminal_keep_alive_mode(),
            terminal_keep_alive_interval: 30,
            terminal_timestamp_format: default_terminal_timestamp_format(),
            terminal_hardware_acceleration: true,
            terminal_show_workspace_padding: false,
            terminal_show_line_numbers: false,
            terminal_show_timestamps: false,
            terminal_show_timestamp_milliseconds: false,
            terminal_show_multi_line_paste_dialog: true,
            terminal_paste_image_as_path: true,
            terminal_low_latency_mode: false,
            terminal_action_links_enabled: false,
            terminal_action_links_matchers: ActionLinksMatcherSettings::default(),
            search_custom_engines: default_search_engines(),
            ui_show_remote_stats: true,
            ui_remote_stats_interval: 3,
            ui_show_gpu_monitor: false,
            ui_gpu_monitor_interval: 3,
            ui_show_ascend_npu_monitor: false,
            ui_ascend_npu_monitor_interval: 3,
            ui_show_process_manager: true,
            ui_process_manager_interval: 5,
            ui_show_docker_manager: true,
            ui_docker_manager_interval: 10,
            ui_quick_cmd_view_mode: default_quick_cmd_view_mode(),
            ui_quick_cmd_sort_mode: default_quick_cmd_sort_mode(),
            ui_saved_connections_sort_mode: default_saved_connections_sort_mode(),
            ui_saved_connections_expanded_group_ids: Vec::new(),
            ui_header_status_mode: default_header_status_mode(),
            ui_header_status_visible: true,
            ui_file_explorer_show_hidden_files: true,
            ui_file_explorer_auto_sync_cwd_connection_ids: Vec::new(),
            ui_file_explorer_favorite_dirs_by_connection_id: HashMap::new(),
            ui_left_panel_width: 256,
            ui_right_panel_width: 288,
            ui_transfer_height: 180,
            ui_quick_cmd_height: 180,
            ui_quick_cmd_visible: true,
            ui_serial_send_height: 180,
            ui_serial_send_visible: false,
            ui_active_left_panel: Some("fileExplorer".to_string()),
            ui_active_right_panel: Some("savedConnections".to_string()),
            ui_left_panel_collapsed: false,
            ui_right_panel_collapsed: false,
            ui_activity_bar_left_top: default_activity_left_top(),
            ui_activity_bar_left_bottom: default_activity_left_bottom(),
            ui_activity_bar_right_top: default_activity_right_top(),
            ui_activity_bar_right_bottom: default_activity_right_bottom(),
            ui_activity_bar_show_labels: false,
            ui_panel_multi_open: false,
            ui_left_open_panels: Vec::new(),
            ui_right_open_panels: Vec::new(),
            ui_panel_stack_sizes: HashMap::new(),
            interaction_copy_on_select: false,
            interaction_allow_osc52_clipboard_write: false,
            interaction_right_click_paste: false,
            interaction_terminal_zoom_enabled: true,
            interaction_command_suggestions_enabled: true,
            interaction_command_suggestion_min_chars: 2,
            interaction_command_suggestion_max_chars: 64,
            interaction_word_separators: " \t\r\n()[]{}\"':=,;|&<>".to_string(),
            interaction_duplicate_session_command_delay_ms: 1000,
            interaction_alt_as_meta: false,
            interaction_mac_ime_compatibility: true,
            interaction_tab_double_click_action: "disconnect_session".to_string(),
            interaction_tab_middle_click_action: "rename_tab".to_string(),
            interaction_tab_right_click_action: "none".to_string(),
            interaction_default_encoding: "UTF-8".to_string(),
            host_key_policy: "prompt".to_string(),
            transfer_download_path: String::new(),
            transfer_ask_save_location: false,
            transfer_duplicate_strategy: "ask".to_string(),
            transfer_editor_type: "external".to_string(),
            transfer_default_editor: String::new(),
            transfer_download_threads: 3,
            transfer_upload_threads: 3,
            transfer_max_retries: 2,
            transfer_buffer_size: 32,
            transfer_default_file_permissions: "644".to_string(),
            transfer_preserve_timestamps: true,
            transfer_resume_broken_transfer: true,
            recording_path: String::new(),
            recording_auto_start: false,
            recording_default_mode: RecordingMode::Transcript,
            recording_path_template: default_recording_path_template(),
            recording_include_io_labels: true,
            recording_include_timestamps: true,
            recording_include_session_metadata: true,
            recording_rotation: RecordingRotationPolicy::Session,
            recording_existing_file_behavior: ExistingFileBehavior::Unique,
            recording_include_binary_transfer_payloads: false,
            recording_memory_limit_bytes: 5 * 1024 * 1024,
            diagnostics_level: "info".to_string(),
            diagnostics_retention_days: 7,
            startup_restore: false,
            startup_restore_window_layout: true,
            minimize_to_tray: false,
            confirm_on_close: true,
            enable_screen_lock: false,
            idle_lock_minutes: 0,
            has_master_password: false,
            keybindings: HashMap::new(),
        }
    }
}

fn default_activity_left_top() -> Vec<String> {
    vec![
        "fileExplorer".to_string(),
        "network".to_string(),
        "securityAuth".to_string(),
    ]
}

fn default_activity_left_bottom() -> Vec<String> {
    vec!["syncBackupHistory".to_string(), "settings".to_string()]
}

fn default_activity_right_top() -> Vec<String> {
    vec![
        "savedConnections".to_string(),
        "aiAssistant".to_string(),
        "activeSessions".to_string(),
        "commandHistory".to_string(),
        "resourceMonitor".to_string(),
        "gpuMonitor".to_string(),
        "ascendNpuMonitor".to_string(),
        "processManager".to_string(),
        "dockerManager".to_string(),
    ]
}

fn default_activity_right_bottom() -> Vec<String> {
    vec![
        "quickCmdBar".to_string(),
        "serialSend".to_string(),
        "recording".to_string(),
        "lock".to_string(),
    ]
}

fn default_left_panel_width() -> u32 {
    256
}

fn default_right_panel_width() -> u32 {
    288
}

fn default_transfer_height() -> u32 {
    180
}

fn default_quick_cmd_height() -> u32 {
    180
}

fn default_serial_send_height() -> u32 {
    180
}

fn default_quick_cmd_view_mode() -> String {
    "tile".to_string()
}

fn default_quick_cmd_sort_mode() -> String {
    "created".to_string()
}

fn default_header_status_mode() -> String {
    "session".to_string()
}

fn default_terminal_keep_alive_mode() -> String {
    "compatible".to_string()
}

fn default_terminal_timestamp_format() -> String {
    DEFAULT_TERMINAL_TIMESTAMP_FORMAT.to_string()
}

fn default_hardware_monitor_interval() -> u32 {
    3
}

fn default_recording_path_template() -> String {
    DEFAULT_RECORDING_PATH_TEMPLATE.to_string()
}

fn default_saved_connections_sort_mode() -> String {
    "default".to_string()
}

fn default_background_image_fit() -> String {
    "cover".to_string()
}

fn default_minimum_contrast_ratio() -> String {
    "1".to_string()
}

fn default_ui_font_family() -> String {
    "Inter".to_string()
}

fn default_ui_font_size() -> u16 {
    16
}

fn default_terminal_font_weight() -> u16 {
    400
}

fn default_terminal_font_weight_bold() -> u16 {
    700
}

fn default_cursor_style() -> String {
    "block".to_string()
}

fn default_cursor_blink() -> bool {
    true
}

fn default_background_image_opacity() -> u8 {
    45
}

fn default_background_content_opacity() -> u8 {
    82
}

fn default_highlight_color_dark() -> String {
    "#79c0ff".to_string()
}

fn default_highlight_color_light() -> String {
    "#0969da".to_string()
}

fn default_ssh_port() -> u16 {
    22
}

fn default_ssh_user() -> String {
    "root".to_string()
}

fn default_backspace_mode_ssh() -> String {
    "del".to_string()
}

fn default_telnet_port() -> u16 {
    23
}

fn default_rdp_port() -> u16 {
    3389
}

fn default_baud_rate() -> u32 {
    115_200
}

fn default_data_bits() -> u8 {
    8
}

fn default_parity() -> String {
    "none".to_string()
}

fn default_stop_bits() -> String {
    "1".to_string()
}

fn default_backspace_mode_serial() -> String {
    "ctrl_h".to_string()
}

fn default_tunnel_type() -> String {
    "local".to_string()
}

fn default_tunnel_target_host() -> String {
    "127.0.0.1".to_string()
}

fn default_proxy_protocol() -> String {
    "socks5".to_string()
}

fn default_proxy_host() -> String {
    "127.0.0.1".to_string()
}

fn default_proxy_port() -> u16 {
    1080
}

fn default_backspace_mode_telnet() -> String {
    "del".to_string()
}

fn default_telnet_enter_mode() -> String {
    "cr".to_string()
}

fn default_telnet_auto_login_timeout_ms() -> u64 {
    60_000
}

fn default_true() -> bool {
    true
}

fn default_rdp_certificate_policy() -> String {
    "prompt".to_string()
}

fn default_rdp_display_mode() -> String {
    "fit-window".to_string()
}

fn default_rdp_width() -> u32 {
    1920
}

fn default_rdp_height() -> u32 {
    1080
}

fn default_rdp_color_depth() -> u8 {
    32
}

fn default_rdp_clipboard_mode() -> String {
    "text-only".to_string()
}

fn default_rdp_reconnect_attempts() -> u32 {
    5
}

pub fn default_sftp_shell_detection_timeout_ms() -> u64 {
    3000
}

fn is_default_sftp_shell_detection_timeout_ms(value: &u64) -> bool {
    *value == default_sftp_shell_detection_timeout_ms()
}

fn is_default_sftp_settings(value: &SftpSettings) -> bool {
    value == &SftpSettings::default()
}

fn is_default_telnet_auto_login_config(value: &TelnetAutoLoginConfig) -> bool {
    value == &TelnetAutoLoginConfig::default()
}

fn default_auth_mode() -> String {
    "password".to_string()
}

fn default_otp_type() -> String {
    "totp".to_string()
}

fn default_otp_algorithm() -> String {
    "SHA1".to_string()
}

fn default_otp_digits() -> u8 {
    6
}

fn default_otp_period() -> u64 {
    30
}

fn default_post_login_delay_ms() -> u64 {
    1000
}

fn uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_legacy_ssh_connection_shape() {
        let json = r#"{
            "sessions": [{
                "id": "conn-1",
                "name": "Production",
                "type": "ssh",
                "host": "10.0.0.8",
                "port": 2222,
                "username": "deploy",
                "auth": {
                    "mode": "password",
                    "password_id": "pw-1",
                    "has_password": true
                },
                "post_login": {
                    "enabled": true,
                    "command": "uptime",
                    "delay_ms": 1500
                }
            }],
            "groups": [{
                "id": "group-1",
                "name": "Servers"
            }]
        }"#;

        let config: SessionsConfig = serde_json::from_str(json).expect("valid sessions config");

        assert_eq!(config.groups.len(), 1);
        assert_eq!(config.connections.len(), 1);
        assert_eq!(config.connections[0].kind_label(), "SSH");
        assert_eq!(config.connections[0].endpoint(), "deploy@10.0.0.8:2222");
        match &config.connections[0].config {
            ConnectionType::Ssh {
                ai_execution_profile,
                ..
            } => assert_eq!(*ai_execution_profile, AiExecutionProfile::Auto),
            other => panic!("expected SSH connection, got {other:?}"),
        }
    }

    #[test]
    fn legacy_connections_default_new_tauri_compatibility_fields() {
        let json = r#"{
            "sessions": [
                {"id":"ssh","name":"SSH","type":"ssh","host":"10.0.0.8"},
                {"id":"local","name":"Local","type":"local_terminal"},
                {"id":"telnet","name":"Telnet","type":"telnet","host":"10.0.0.9"},
                {"id":"serial","name":"Serial","type":"serial","port_name":"COM1"}
            ],
            "groups": []
        }"#;

        let config: SessionsConfig = serde_json::from_str(json).expect("valid sessions config");

        assert_eq!(config.connections.len(), 4);
        for connection in &config.connections {
            assert!(connection.recording.is_none());
            assert!(connection.ssh_algorithms.is_none());
            assert_eq!(connection.sftp, SftpSettings::default());
        }
        match &config.connections[0].config {
            ConnectionType::Ssh { encoding, .. } => assert_eq!(encoding, ""),
            other => panic!("expected SSH connection, got {other:?}"),
        }
        match &config.connections[1].config {
            ConnectionType::LocalTerminal { encoding, .. } => assert_eq!(encoding, ""),
            other => panic!("expected local connection, got {other:?}"),
        }
        match &config.connections[2].config {
            ConnectionType::Telnet {
                username, encoding, ..
            } => {
                assert_eq!(username, "");
                assert_eq!(encoding, "");
            }
            other => panic!("expected Telnet connection, got {other:?}"),
        }
        match &config.connections[3].config {
            ConnectionType::Serial { encoding, .. } => assert_eq!(encoding, ""),
            other => panic!("expected serial connection, got {other:?}"),
        }
    }

    #[test]
    fn saved_connection_recording_override_round_trips() {
        let json = r#"{
            "id":"recording-override",
            "name":"Recorded SSH",
            "type":"ssh",
            "host":"example.com",
            "recording":{
                "auto_start":true,
                "mode":"raw",
                "path_template":"{session}.raw",
                "include_timestamps":false,
                "rotation":{"type":"size","max_bytes":1048576}
            }
        }"#;

        let connection: SavedConnection = serde_json::from_str(json).expect("valid connection");
        let recording = connection.recording.as_ref().expect("recording override");
        assert_eq!(recording.auto_start, Some(true));
        assert_eq!(recording.mode, Some(RecordingMode::Raw));
        assert_eq!(recording.path_template.as_deref(), Some("{session}.raw"));
        assert_eq!(recording.include_timestamps, Some(false));
        assert_eq!(
            recording.rotation,
            Some(RecordingRotationPolicy::Size {
                max_bytes: 1_048_576
            })
        );

        let round_trip: SavedConnection =
            serde_json::from_str(&serde_json::to_string(&connection).expect("serialize"))
                .expect("reload");
        assert_eq!(round_trip, connection);
    }

    #[test]
    fn rdp_connection_defaults_and_endpoint_match_tauri_shape() {
        let json = r#"{
            "id":"rdp-1",
            "name":"Windows",
            "type":"rdp",
            "host":"192.168.1.20",
            "username":"Administrator"
        }"#;

        let connection: SavedConnection = serde_json::from_str(json).expect("valid connection");
        assert_eq!(connection.kind_label(), "RDP");
        assert_eq!(connection.endpoint(), "Administrator@192.168.1.20:3389");

        let ConnectionType::Rdp {
            port,
            domain,
            security,
            display,
            clipboard,
            reconnect,
            ..
        } = &connection.config
        else {
            panic!("expected RDP connection");
        };

        assert_eq!(*port, 3389);
        assert!(domain.is_empty());
        assert!(security.use_nla);
        assert_eq!(security.certificate_policy, "prompt");
        assert_eq!(display.mode, "fit-window");
        assert_eq!(display.width, 1920);
        assert_eq!(display.height, 1080);
        assert_eq!(display.color_depth, 32);
        assert_eq!(clipboard.mode, "text-only");
        assert!(reconnect.enabled);
        assert_eq!(reconnect.max_attempts, 5);
    }

    #[test]
    fn tauri_ssh_algorithm_sftp_and_encoding_fields_round_trip() {
        let json = r#"{
            "id":"ssh-tauri",
            "name":"SSH",
            "type":"ssh",
            "host":"example.com",
            "port":2222,
            "username":"deploy",
            "encoding":"GBK",
            "ssh_algorithms":{
                "mode":"custom",
                "kex":["curve25519-sha256"],
                "ciphers":["aes128-ctr"],
                "macs":["hmac-sha2-256"],
                "host_keys":["ssh-ed25519"]
            },
            "sftp":{
                "enabled":false,
                "cwd_follow_mode":"rc_file",
                "shell_detection_timeout_ms":5000,
                "filename_encoding":"GB18030"
            }
        }"#;

        let connection: SavedConnection = serde_json::from_str(json).expect("valid connection");

        match &connection.config {
            ConnectionType::Ssh { encoding, .. } => assert_eq!(encoding, "GBK"),
            other => panic!("expected SSH connection, got {other:?}"),
        }
        assert_eq!(
            connection.ssh_algorithms,
            Some(SshAlgorithmPreferences {
                mode: SshAlgorithmMode::Custom,
                kex: vec!["curve25519-sha256".to_string()],
                ciphers: vec!["aes128-ctr".to_string()],
                macs: vec!["hmac-sha2-256".to_string()],
                host_keys: vec!["ssh-ed25519".to_string()],
            })
        );
        assert_eq!(
            connection.sftp,
            SftpSettings {
                enabled: false,
                cwd_follow_mode: SftpCwdFollowMode::RcFile,
                shell_detection_timeout_ms: 5000,
                filename_encoding: "GB18030".to_string(),
            }
        );

        let round_trip: SavedConnection =
            serde_json::from_str(&serde_json::to_string(&connection).expect("serialize"))
                .expect("reload");
        assert_eq!(round_trip, connection);
    }

    #[test]
    fn tauri_telnet_username_auth_and_encoding_fields_round_trip() {
        let json = r#"{
            "id":"telnet-tauri",
            "name":"Telnet",
            "type":"telnet",
            "host":"10.0.0.9",
            "port":23,
            "username":"operator",
            "encoding":"GB18030",
            "local_echo":true,
            "local_line_edit":true,
            "auth":{
                "mode":"password",
                "password":"secret"
            }
        }"#;

        let connection: SavedConnection = serde_json::from_str(json).expect("valid connection");

        match &connection.config {
            ConnectionType::Telnet {
                username,
                encoding,
                local_echo,
                local_line_edit,
                ..
            } => {
                assert_eq!(username, "operator");
                assert_eq!(encoding, "GB18030");
                assert!(*local_echo);
                assert!(*local_line_edit);
            }
            other => panic!("expected Telnet connection, got {other:?}"),
        }
        assert_eq!(
            connection
                .auth
                .as_ref()
                .and_then(|auth| auth.password.as_deref()),
            Some("secret")
        );

        let round_trip: SavedConnection =
            serde_json::from_str(&serde_json::to_string(&connection).expect("serialize"))
                .expect("reload");
        assert_eq!(round_trip, connection);
    }

    #[test]
    fn validates_sftp_shell_detection_timeout_range() {
        for value in [
            MIN_SFTP_SHELL_DETECTION_TIMEOUT_MS,
            default_sftp_shell_detection_timeout_ms(),
            MAX_SFTP_SHELL_DETECTION_TIMEOUT_MS,
        ] {
            let settings = SftpSettings {
                shell_detection_timeout_ms: value,
                ..Default::default()
            };
            assert!(validate_sftp_settings(&settings).is_ok());
        }

        for value in [
            MIN_SFTP_SHELL_DETECTION_TIMEOUT_MS - 1,
            MAX_SFTP_SHELL_DETECTION_TIMEOUT_MS + 1,
        ] {
            let settings = SftpSettings {
                shell_detection_timeout_ms: value,
                ..Default::default()
            };
            assert!(validate_sftp_settings(&settings).is_err());
        }
    }

    #[test]
    fn local_terminal_endpoint_uses_shell_and_working_dir() {
        let connection = SavedConnection {
            id: "local-1".to_string(),
            name: "Local".to_string(),
            config: ConnectionType::LocalTerminal {
                shell_path: "zsh".to_string(),
                shell_args: String::new(),
                working_dir: Some("/data".to_string()),
                ai_execution_profile: AiExecutionProfile::Auto,
                encoding: String::new(),
            },
            group_id: None,
            description: None,
            sort_order: 0,
            icon: None,
            icon_auto_detect: None,
            auth: None,
            ssh_algorithms: None,
            sftp: Default::default(),
            network: None,
            post_login: None,
            recording: None,
            created_at_ms: None,
            updated_at_ms: None,
            last_used_at_ms: None,
        };

        assert_eq!(connection.endpoint(), "zsh in /data");
    }

    #[test]
    fn icon_auto_detect_defaults_to_filling_in_a_blank_only() {
        let mut connection: SavedConnection = serde_json::from_str(
            r#"{"id":"c1","name":"Box","type":"ssh","host":"h","port":22,"username":"root"}"#,
        )
        .expect("valid connection");

        // Unset flag, no icon: detection may fill one in.
        assert!(connection.icon_auto_detect_enabled());

        // Unset flag but an icon chosen by an older build: leave it alone.
        connection.icon = Some("ubuntu".to_string());
        assert!(!connection.icon_auto_detect_enabled());

        // An explicit flag always wins over the heuristic, either way.
        connection.icon_auto_detect = Some(true);
        assert!(connection.icon_auto_detect_enabled());
        connection.icon = None;
        connection.icon_auto_detect = Some(false);
        assert!(!connection.icon_auto_detect_enabled());
    }

    #[test]
    fn icon_auto_detect_round_trips_and_stays_absent_when_unset() {
        let connection: SavedConnection = serde_json::from_str(
            r#"{"id":"c1","name":"Box","type":"ssh","host":"h","port":22,"username":"root"}"#,
        )
        .expect("valid connection");

        // Files written by builds predating the field must round-trip byte-for-
        // byte, so an unset flag is never serialized.
        let json = serde_json::to_string(&connection).expect("serializes");
        assert!(!json.contains("icon_auto_detect"), "{json}");

        let explicit = SavedConnection {
            icon_auto_detect: Some(false),
            ..connection
        };
        let reloaded: SavedConnection =
            serde_json::from_str(&serde_json::to_string(&explicit).expect("serializes"))
                .expect("reloads");
        assert_eq!(reloaded.icon_auto_detect, Some(false));
    }

    #[test]
    fn restorable_open_tab_lock_is_backward_compatible_and_sparse() {
        let legacy = r#"{
            "title":"Local",
            "session_type":"Local",
            "connection_id":null,
            "custom_name":null,
            "tab_color":null,
            "active_pane_id":null,
            "root":null
        }"#;
        let tab: RestorableOpenTab = serde_json::from_str(legacy).expect("legacy tab loads");
        assert!(!tab.locked);
        let json = serde_json::to_string(&tab).expect("tab serializes");
        assert!(!json.contains("locked"), "{json}");

        let locked = RestorableOpenTab {
            locked: true,
            ..tab
        };
        let json = serde_json::to_string(&locked).expect("locked tab serializes");
        assert!(json.contains(r#""locked":true"#), "{json}");
        let reloaded: RestorableOpenTab = serde_json::from_str(&json).expect("locked tab reloads");
        assert!(reloaded.locked);
    }
}
