use serde::{Deserialize, Serialize};

use super::{
    default_backspace_mode_serial, default_backspace_mode_ssh, default_backspace_mode_telnet,
    default_baud_rate, default_data_bits, default_parity, default_rdp_certificate_policy,
    default_rdp_clipboard_mode, default_rdp_color_depth, default_rdp_display_mode,
    default_rdp_height, default_rdp_port, default_rdp_reconnect_attempts, default_rdp_width,
    default_sftp_shell_detection_timeout_ms, default_ssh_port, default_ssh_user, default_stop_bits,
    default_telnet_auto_login_timeout_ms, default_telnet_enter_mode, default_telnet_port,
    default_true, default_vnc_port, default_vnc_reconnect_attempts, default_vnc_scale_mode,
    default_vnc_security_mode, is_default_sftp_shell_detection_timeout_ms,
    is_default_telnet_auto_login_config,
};

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SshProfile {
    #[default]
    Standard,
    NetworkDevice,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SshTerminalType {
    #[default]
    #[serde(rename = "xterm-256color")]
    Xterm256Color,
    #[serde(rename = "xterm")]
    Xterm,
    #[serde(rename = "vt100")]
    Vt100,
    #[serde(rename = "vt220")]
    Vt220,
    #[serde(rename = "ansi")]
    Ansi,
    #[serde(rename = "linux")]
    Linux,
}

impl SshTerminalType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Xterm256Color => "xterm-256color",
            Self::Xterm => "xterm",
            Self::Vt100 => "vt100",
            Self::Vt220 => "vt220",
            Self::Ansi => "ansi",
            Self::Linux => "linux",
        }
    }
}

pub fn default_terminal_type_for_profile(profile: SshProfile) -> SshTerminalType {
    match profile {
        SshProfile::Standard => SshTerminalType::Xterm256Color,
        SshProfile::NetworkDevice => SshTerminalType::Vt100,
    }
}

pub fn resolve_ssh_terminal_type(
    profile: SshProfile,
    terminal_type: Option<SshTerminalType>,
) -> SshTerminalType {
    terminal_type.unwrap_or_else(|| default_terminal_type_for_profile(profile))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SftpCwdFollowMode {
    Off,
    #[default]
    ShellIntegration,
    RcFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SshAgentEndpoint {
    #[default]
    Auto,
    Environment {
        variable: String,
    },
    UnixSocket {
        path: String,
    },
    Pageant,
    WindowsOpenSsh,
}

fn is_default_ssh_agent_endpoint(value: &SshAgentEndpoint) -> bool {
    matches!(value, SshAgentEndpoint::Auto)
}

fn is_false(value: &bool) -> bool {
    !*value
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
        #[serde(default, skip_serializing_if = "is_default_ssh_agent_endpoint")]
        agent_endpoint: SshAgentEndpoint,
        #[serde(default, skip_serializing_if = "is_false")]
        agent_forwarding: bool,
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
    Vnc {
        host: String,
        #[serde(default = "default_vnc_port")]
        port: u16,
        #[serde(default)]
        security: VncSecuritySettings,
        #[serde(default)]
        display: VncDisplaySettings,
        #[serde(default)]
        clipboard: VncClipboardSettings,
        #[serde(default)]
        reconnect: VncReconnectSettings,
        #[serde(default = "default_true")]
        shared: bool,
        #[serde(default)]
        view_only: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VncSecuritySettings {
    #[serde(default = "default_vnc_security_mode")]
    pub mode: String,
}

impl Default for VncSecuritySettings {
    fn default() -> Self {
        Self {
            mode: default_vnc_security_mode(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VncDisplaySettings {
    #[serde(default = "default_vnc_scale_mode")]
    pub scale_mode: String,
}

impl Default for VncDisplaySettings {
    fn default() -> Self {
        Self {
            scale_mode: default_vnc_scale_mode(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VncClipboardSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for VncClipboardSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VncReconnectSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_vnc_reconnect_attempts")]
    pub max_attempts: u32,
}

impl Default for VncReconnectSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: default_vnc_reconnect_attempts(),
        }
    }
}
