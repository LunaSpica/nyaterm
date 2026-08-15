use serde::{Deserialize, Serialize};

use crate::ai::RiskLevel;

pub mod credentials;
pub mod settings;
pub mod workspace;
pub use credentials::*;
pub use settings::*;
pub use workspace::*;

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

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
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

impl std::fmt::Debug for ProxyConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProxyConfig")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("protocol", &self.protocol)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("command", &self.command.as_ref().map(|_| "<redacted>"))
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .field("password_id", &self.password_id)
            .field("group_id", &self.group_id)
            .finish()
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub sort_order: i32,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct QuickCommandsConfig {
    #[serde(default)]
    pub commands: Vec<QuickCommand>,
    #[serde(default)]
    pub categories: Vec<QuickCommandCategory>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickCommandRelativePosition {
    Before,
    After,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickCommandCategoryPosition {
    Before,
    After,
    Inside,
}

impl QuickCommandsConfig {
    /// Moves a command relative to another command. Pinned and unpinned commands
    /// remain separate visual partitions; dropping across the boundary adopts the
    /// target partition before orders are normalized.
    pub fn reorder_command_relative(
        &mut self,
        source_id: &str,
        target_id: &str,
        position: QuickCommandRelativePosition,
    ) -> bool {
        if source_id == target_id {
            return false;
        }
        let Some(source_index) = self.commands.iter().position(|item| item.id == source_id) else {
            return false;
        };
        let Some(target) = self
            .commands
            .iter()
            .find(|item| item.id == target_id)
            .cloned()
        else {
            return false;
        };
        let mut source = self.commands.remove(source_index);
        source.category_id = target.category_id.clone();
        source.pinned = target.pinned;

        let mut partition = self
            .commands
            .iter()
            .filter(|item| {
                item.category_id == target.category_id
                    && item.pinned.unwrap_or_default() == target.pinned.unwrap_or_default()
            })
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        partition.sort_by(|left, right| {
            let left = self
                .commands
                .iter()
                .find(|item| item.id == *left)
                .expect("id");
            let right = self
                .commands
                .iter()
                .find(|item| item.id == *right)
                .expect("id");
            left.sort_order
                .unwrap_or(i32::MAX)
                .cmp(&right.sort_order.unwrap_or(i32::MAX))
                .then_with(|| left.id.cmp(&right.id))
        });
        let Some(target_index) = partition.iter().position(|id| id == target_id) else {
            self.commands.push(source);
            return false;
        };
        let insert_index =
            target_index + usize::from(position == QuickCommandRelativePosition::After);
        partition.insert(insert_index, source.id.clone());
        self.commands.push(source);
        self.normalize_command_partition(
            &target.category_id,
            target.pinned.unwrap_or_default(),
            &partition,
        );
        true
    }

    pub fn move_command_to_category(
        &mut self,
        source_id: &str,
        category_id: Option<String>,
    ) -> bool {
        let Some(source) = self.commands.iter_mut().find(|item| item.id == source_id) else {
            return false;
        };
        if source.category_id == category_id {
            return false;
        }
        source.category_id = category_id.clone();
        let pinned = source.pinned.unwrap_or_default();
        let next = self
            .commands
            .iter()
            .filter(|item| {
                item.id != source_id
                    && item.category_id == category_id
                    && item.pinned.unwrap_or_default() == pinned
            })
            .filter_map(|item| item.sort_order)
            .max()
            .unwrap_or(-1)
            .saturating_add(1);
        if let Some(source) = self.commands.iter_mut().find(|item| item.id == source_id) {
            source.sort_order = Some(next);
        }
        true
    }

    pub fn move_category(
        &mut self,
        source_id: &str,
        target_id: &str,
        position: QuickCommandCategoryPosition,
    ) -> bool {
        if source_id == target_id
            || !self.categories.iter().any(|item| item.id == source_id)
            || !self.categories.iter().any(|item| item.id == target_id)
            || self.category_is_descendant(target_id, source_id)
        {
            return false;
        }
        let target_parent = self
            .categories
            .iter()
            .find(|item| item.id == target_id)
            .and_then(|item| item.parent_id.clone());
        let new_parent = match position {
            QuickCommandCategoryPosition::Inside => Some(target_id.to_string()),
            QuickCommandCategoryPosition::Before | QuickCommandCategoryPosition::After => {
                target_parent
            }
        };
        if new_parent.as_deref() == Some(source_id) {
            return false;
        }

        if let Some(source) = self.categories.iter_mut().find(|item| item.id == source_id) {
            source.parent_id = new_parent.clone();
        }
        let mut siblings = self
            .categories
            .iter()
            .filter(|item| item.id != source_id && item.parent_id == new_parent)
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        siblings.sort_by(|left, right| {
            let left = self
                .categories
                .iter()
                .find(|item| item.id == *left)
                .expect("id");
            let right = self
                .categories
                .iter()
                .find(|item| item.id == *right)
                .expect("id");
            left.sort_order
                .cmp(&right.sort_order)
                .then_with(|| left.id.cmp(&right.id))
        });
        let insert_index = match position {
            QuickCommandCategoryPosition::Inside => siblings.len(),
            QuickCommandCategoryPosition::Before | QuickCommandCategoryPosition::After => {
                let Some(index) = siblings.iter().position(|id| id == target_id) else {
                    return false;
                };
                index + usize::from(position == QuickCommandCategoryPosition::After)
            }
        };
        siblings.insert(insert_index, source_id.to_string());
        for (order, id) in siblings.into_iter().enumerate() {
            if let Some(category) = self.categories.iter_mut().find(|item| item.id == id) {
                category.sort_order = i32::try_from(order).unwrap_or(i32::MAX);
            }
        }
        true
    }

    fn category_is_descendant(&self, candidate_id: &str, ancestor_id: &str) -> bool {
        let mut current = Some(candidate_id);
        let mut visited = std::collections::BTreeSet::new();
        while let Some(id) = current {
            if id == ancestor_id {
                return true;
            }
            if !visited.insert(id.to_string()) {
                return false;
            }
            current = self
                .categories
                .iter()
                .find(|item| item.id == id)
                .and_then(|item| item.parent_id.as_deref());
        }
        false
    }

    fn normalize_command_partition(
        &mut self,
        category_id: &Option<String>,
        pinned: bool,
        ordered_ids: &[String],
    ) {
        for (order, id) in ordered_ids.iter().enumerate() {
            if let Some(command) = self.commands.iter_mut().find(|item| {
                item.id == *id
                    && item.category_id == *category_id
                    && item.pinned.unwrap_or_default() == pinned
            }) {
                command.sort_order = Some(i32::try_from(order).unwrap_or(i32::MAX));
            }
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
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
                    parent_id: category.parent_id,
                    sort_order: category.sort_order,
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
                    sort_order: command.sort_order,
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
    #[serde(default, skip_serializing_if = "is_standard_ssh_profile")]
    pub ssh_profile: SshProfile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_type: Option<SshTerminalType>,
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
            ConnectionType::Vnc { .. } => "VNC",
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
            ConnectionType::Vnc { host, port, .. } => format!("{host}:{port}"),
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

fn default_vnc_port() -> u16 {
    5900
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

fn default_vnc_security_mode() -> String {
    "auto".to_string()
}

fn default_vnc_scale_mode() -> String {
    "fit".to_string()
}

fn default_vnc_reconnect_attempts() -> u32 {
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

fn is_standard_ssh_profile(value: &SshProfile) -> bool {
    *value == SshProfile::Standard
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
    fn secret_bearing_model_debug_output_is_redacted() {
        let secret = "nya-test-secret-never-log";
        let values = [
            format!(
                "{:?}",
                ConnectionAuth {
                    password: Some(secret.to_string()),
                    ..ConnectionAuth::default()
                }
            ),
            format!(
                "{:?}",
                SshKey {
                    id: "key-1".to_string(),
                    name: "Test key".to_string(),
                    key: Some(secret.to_string()),
                    cert: Some(secret.to_string()),
                    passphrase: Some(secret.to_string()),
                    key_file_path: None,
                    cert_file_path: None,
                    has_key_data: true,
                    has_cert_data: true,
                }
            ),
            format!(
                "{:?}",
                DecryptedSavedCredential {
                    id: "credential-1".to_string(),
                    sort_order: 0,
                    name: "Test credential".to_string(),
                    username: "tester".to_string(),
                    password: Some(secret.to_string()),
                    username_prompt_regex: None,
                    password_prompt_regex: None,
                    enabled: true,
                }
            ),
            format!(
                "{:?}",
                DecryptedOtpEntry {
                    id: "otp-1".to_string(),
                    otp_type: "totp".to_string(),
                    issuer: "NyaTerm".to_string(),
                    username: "tester".to_string(),
                    secret: Some(secret.to_string()),
                    algorithm: "SHA1".to_string(),
                    digits: 6,
                    period: 30,
                    counter: 0,
                }
            ),
            format!(
                "{:?}",
                ProxyConfig {
                    command: Some(secret.to_string()),
                    password: Some(secret.to_string()),
                    ..ProxyConfig::default()
                }
            ),
        ];

        for output in values {
            assert!(!output.contains(secret));
            assert!(output.contains("<redacted>"));
        }
    }

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
    fn vnc_connection_defaults_and_endpoint_match_tauri_shape() {
        let json = r#"{
            "id":"vnc-1",
            "name":"Remote X",
            "type":"vnc",
            "host":"192.168.1.30"
        }"#;

        let connection: SavedConnection = serde_json::from_str(json).expect("valid connection");
        assert_eq!(connection.kind_label(), "VNC");
        assert_eq!(connection.endpoint(), "192.168.1.30:5900");

        let ConnectionType::Vnc {
            port,
            security,
            display,
            clipboard,
            reconnect,
            shared,
            view_only,
            ..
        } = &connection.config
        else {
            panic!("expected VNC connection");
        };

        assert_eq!(*port, 5900);
        assert_eq!(security.mode, "auto");
        assert_eq!(display.scale_mode, "fit");
        assert!(clipboard.enabled);
        assert!(reconnect.enabled);
        assert_eq!(reconnect.max_attempts, 5);
        assert!(*shared);
        assert!(!*view_only);

        let round_trip: SavedConnection =
            serde_json::from_str(&serde_json::to_string(&connection).expect("serialize"))
                .expect("reload");
        assert_eq!(round_trip, connection);
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
            ssh_profile: Default::default(),
            terminal_type: None,
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

    #[test]
    fn legacy_ssh_profile_defaults_without_rewriting_sparse_json() {
        let json = r#"{
            "id":"legacy-ssh",
            "name":"Legacy SSH",
            "type":"ssh",
            "host":"example.com"
        }"#;
        let connection: SavedConnection = serde_json::from_str(json).expect("legacy SSH loads");
        assert_eq!(connection.ssh_profile, SshProfile::Standard);
        assert_eq!(connection.terminal_type, None);
        assert_eq!(
            resolve_ssh_terminal_type(connection.ssh_profile, connection.terminal_type),
            SshTerminalType::Xterm256Color
        );
        let serialized = serde_json::to_string(&connection).expect("serialize");
        assert!(!serialized.contains("ssh_profile"), "{serialized}");
        assert!(!serialized.contains("terminal_type"), "{serialized}");
        assert!(!serialized.contains("agent_endpoint"), "{serialized}");
        assert!(!serialized.contains("agent_forwarding"), "{serialized}");
    }

    #[test]
    fn ssh_agent_endpoint_and_forwarding_round_trip_without_secrets() {
        let json = r#"{
            "id":"agent-ssh","name":"Agent SSH","type":"ssh","host":"example.com",
            "auth":{"mode":"agent"},
            "agent_endpoint":{"type":"unix_socket","path":"/run/user/1000/agent.sock"},
            "agent_forwarding":true
        }"#;
        let connection: SavedConnection = serde_json::from_str(json).expect("agent SSH loads");
        assert_eq!(
            connection.auth.as_ref().map(|auth| auth.mode.as_str()),
            Some("agent")
        );
        let ConnectionType::Ssh {
            agent_endpoint,
            agent_forwarding,
            ..
        } = &connection.config
        else {
            panic!("SSH expected");
        };
        assert_eq!(
            agent_endpoint,
            &SshAgentEndpoint::UnixSocket {
                path: "/run/user/1000/agent.sock".to_string()
            }
        );
        assert!(*agent_forwarding);
        let serialized = serde_json::to_string(&connection).expect("agent SSH serializes");
        assert!(serialized.contains("unix_socket"), "{serialized}");
        assert!(serialized.contains("agent_forwarding"), "{serialized}");
    }

    #[test]
    fn network_device_profile_and_explicit_terminal_round_trip() {
        let json = r#"{
            "id":"switch",
            "name":"Core switch",
            "type":"ssh",
            "host":"10.0.0.2",
            "ssh_profile":"network_device"
        }"#;
        let mut connection: SavedConnection = serde_json::from_str(json).expect("profile loads");
        assert_eq!(connection.ssh_profile, SshProfile::NetworkDevice);
        assert_eq!(
            resolve_ssh_terminal_type(connection.ssh_profile, connection.terminal_type),
            SshTerminalType::Vt100
        );
        connection.terminal_type = Some(SshTerminalType::Ansi);
        assert_eq!(
            resolve_ssh_terminal_type(connection.ssh_profile, connection.terminal_type),
            SshTerminalType::Ansi
        );
        let reloaded: SavedConnection =
            serde_json::from_str(&serde_json::to_string(&connection).expect("serialize profile"))
                .expect("reload profile");
        assert_eq!(reloaded, connection);
    }

    fn quick_command(id: &str, category: Option<&str>, pinned: bool, order: i32) -> QuickCommand {
        QuickCommand {
            id: id.to_string(),
            label: id.to_string(),
            command: id.to_string(),
            category_id: category.map(ToString::to_string),
            description: None,
            color_tag: None,
            icon_tag: None,
            pinned: pinned.then_some(true),
            execution_mode: None,
            source: None,
            risk_level: None,
            updated_at: None,
            created_at: None,
            use_count: None,
            sort_order: Some(order),
        }
    }

    fn quick_category(id: &str, parent: Option<&str>, order: i32) -> QuickCommandCategory {
        QuickCommandCategory {
            id: id.to_string(),
            name: id.to_string(),
            parent_id: parent.map(ToString::to_string),
            sort_order: order,
        }
    }

    #[test]
    fn quick_command_reorder_adopts_target_partition_and_normalizes_order() {
        let mut config = QuickCommandsConfig {
            commands: vec![
                quick_command("a", Some("one"), false, 0),
                quick_command("b", Some("two"), true, 0),
                quick_command("c", Some("two"), true, 1),
            ],
            categories: vec![],
        };
        assert!(config.reorder_command_relative("a", "c", QuickCommandRelativePosition::Before));
        let a = config
            .commands
            .iter()
            .find(|item| item.id == "a")
            .expect("a");
        assert_eq!(a.category_id.as_deref(), Some("two"));
        assert!(a.pinned.unwrap_or_default());
        assert_eq!(a.sort_order, Some(1));
        assert_eq!(
            config
                .commands
                .iter()
                .find(|item| item.id == "c")
                .and_then(|item| item.sort_order),
            Some(2)
        );
    }

    #[test]
    fn category_move_rejects_descendant_cycles_and_normalizes_siblings() {
        let mut config = QuickCommandsConfig {
            commands: vec![],
            categories: vec![
                quick_category("root", None, 0),
                quick_category("child", Some("root"), 0),
                quick_category("peer", None, 1),
            ],
        };
        assert!(!config.move_category("root", "child", QuickCommandCategoryPosition::Inside));
        assert!(config.move_category("child", "peer", QuickCommandCategoryPosition::Before));
        let child = config
            .categories
            .iter()
            .find(|item| item.id == "child")
            .expect("child");
        assert_eq!(child.parent_id, None);
        assert_eq!(child.sort_order, 1);
        assert_eq!(
            config
                .categories
                .iter()
                .find(|item| item.id == "peer")
                .map(|item| item.sort_order),
            Some(2)
        );
    }
}
