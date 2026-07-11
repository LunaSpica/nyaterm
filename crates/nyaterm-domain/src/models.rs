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
    },
    Telnet {
        host: String,
        #[serde(default = "default_telnet_port")]
        port: u16,
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
    },
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
    #[serde(default)]
    pub auth: Option<ConnectionAuth>,
    #[serde(default)]
    pub network: Option<ConnectionNetwork>,
    #[serde(default)]
    pub post_login: Option<ConnectionPostLogin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used_at_ms: Option<u64>,
}

impl SavedConnection {
    pub fn kind_label(&self) -> &'static str {
        match self.config {
            ConnectionType::Ssh { .. } => "SSH",
            ConnectionType::LocalTerminal { .. } => "Local",
            ConnectionType::Telnet { .. } => "Telnet",
            ConnectionType::Serial { .. } => "Serial",
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
            show_in_menu: true,
        },
        SearchEngineConfig {
            name: "Bing".to_string(),
            url_template: "https://www.bing.com/search?q=%s".to_string(),
            show_in_menu: true,
        },
        SearchEngineConfig {
            name: "GitHub".to_string(),
            url_template: "https://github.com/search?q=%s".to_string(),
            show_in_menu: true,
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettingsSummary {
    pub theme: String,
    #[serde(default)]
    pub background_image_path: Option<String>,
    #[serde(default = "default_background_image_fit")]
    pub background_image_fit: String,
    /// Wallpaper opacity percent (5..=100).
    #[serde(default = "default_background_image_opacity")]
    pub background_image_opacity: u8,
    /// Shell chrome opacity percent when wallpaper is active (20..=100).
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
    pub x11_display: String,
    pub terminal_scrollback_lines: u32,
    pub terminal_keep_alive_interval: u32,
    pub terminal_hardware_acceleration: bool,
    pub terminal_show_workspace_padding: bool,
    pub terminal_show_line_numbers: bool,
    pub terminal_show_timestamps: bool,
    pub terminal_show_timestamp_milliseconds: bool,
    pub terminal_show_multi_line_paste_dialog: bool,
    pub terminal_paste_image_as_path: bool,
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
    pub ui_show_process_manager: bool,
    pub ui_process_manager_interval: u32,
    pub ui_show_docker_manager: bool,
    pub ui_docker_manager_interval: u32,
    #[serde(default = "default_quick_cmd_view_mode")]
    pub ui_quick_cmd_view_mode: String,
    #[serde(default = "default_quick_cmd_sort_mode")]
    pub ui_quick_cmd_sort_mode: String,
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
    pub interaction_right_click_paste: bool,
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
    pub recording_include_io_labels: bool,
    pub recording_include_timestamps: bool,
    pub recording_memory_limit_bytes: u64,
    pub diagnostics_level: String,
    pub diagnostics_retention_days: u32,
    pub startup_restore: bool,
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
            x11_display: String::new(),
            terminal_scrollback_lines: 5000,
            terminal_keep_alive_interval: 30,
            terminal_hardware_acceleration: true,
            terminal_show_workspace_padding: false,
            terminal_show_line_numbers: false,
            terminal_show_timestamps: false,
            terminal_show_timestamp_milliseconds: false,
            terminal_show_multi_line_paste_dialog: true,
            terminal_paste_image_as_path: true,
            terminal_action_links_enabled: false,
            terminal_action_links_matchers: ActionLinksMatcherSettings::default(),
            search_custom_engines: default_search_engines(),
            ui_show_remote_stats: true,
            ui_remote_stats_interval: 3,
            ui_show_process_manager: true,
            ui_process_manager_interval: 5,
            ui_show_docker_manager: true,
            ui_docker_manager_interval: 10,
            ui_quick_cmd_view_mode: default_quick_cmd_view_mode(),
            ui_quick_cmd_sort_mode: default_quick_cmd_sort_mode(),
            ui_file_explorer_auto_sync_cwd_connection_ids: Vec::new(),
            ui_file_explorer_favorite_dirs_by_connection_id: HashMap::new(),
            ui_left_panel_width: 256,
            ui_right_panel_width: 288,
            ui_transfer_height: 180,
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
            interaction_right_click_paste: false,
            interaction_command_suggestions_enabled: true,
            interaction_command_suggestion_min_chars: 2,
            interaction_command_suggestion_max_chars: 64,
            interaction_word_separators: " \t\r\n\"'`~!@#$%^&*()-=+[{]}\\|;:,<.>/?".to_string(),
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
            recording_include_io_labels: true,
            recording_include_timestamps: true,
            recording_memory_limit_bytes: 5 * 1024 * 1024,
            diagnostics_level: "info".to_string(),
            diagnostics_retention_days: 7,
            startup_restore: false,
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

fn default_quick_cmd_view_mode() -> String {
    "tile".to_string()
}

fn default_quick_cmd_sort_mode() -> String {
    "created".to_string()
}

fn default_background_image_fit() -> String {
    "cover".to_string()
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

fn default_true() -> bool {
    true
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
    fn local_terminal_endpoint_uses_shell_and_working_dir() {
        let connection = SavedConnection {
            id: "local-1".to_string(),
            name: "Local".to_string(),
            config: ConnectionType::LocalTerminal {
                shell_path: "zsh".to_string(),
                shell_args: String::new(),
                working_dir: Some("/data".to_string()),
                ai_execution_profile: AiExecutionProfile::Auto,
            },
            group_id: None,
            description: None,
            sort_order: 0,
            icon: None,
            auth: None,
            network: None,
            post_login: None,
            created_at_ms: None,
            updated_at_ms: None,
            last_used_at_ms: None,
        };

        assert_eq!(connection.endpoint(), "zsh in /data");
    }
}
