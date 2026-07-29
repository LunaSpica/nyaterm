//! Authoritative application settings and grouped state for the settings experience.

use std::collections::HashMap;

use gpui::FocusHandle;
use nyaterm_core::{
    AppSettingsSummary, KeywordHighlightConfig, KeywordHighlightRule, SearchEngineConfig,
};

use crate::models::{
    ConfigPathPromptKind, DiagnosticsPathPromptKind, KeywordHighlightEditorField,
    KeywordHighlightPathPromptKind, SnapshotPasswordPromptState,
};

use super::catalog::{SettingsMasterPasswordState, StoreStatus};

pub(in crate::features) struct SettingsFeatureState {
    /// Compatibility-sensitive values loaded and persisted through `nyaterm-core`.
    summary: AppSettingsSummary,
    pub(super) keyword_config: KeywordHighlightConfig,
    pub(super) master_password: SettingsMasterPasswordState,
    pub(super) store_status: StoreStatus,
    search_engines: SearchEngineSettingsState,
    keyword_highlights: KeywordHighlightSettingsState,
    appearance: AppearanceSettingsState,
    keybindings: KeybindingSettingsState,
    prompts: SettingsPromptState,
}

#[derive(Default)]
struct SettingsPromptState {
    config_path: Option<ConfigPathPromptKind>,
    diagnostics_path: Option<DiagnosticsPathPromptKind>,
    keyword_highlight_path: Option<KeywordHighlightPathPromptKind>,
    snapshot_password: Option<SnapshotPasswordPromptState>,
}

pub(in crate::features) struct SettingsFeatureFocus {
    pub search_engine: FocusHandle,
    pub keyword_highlight: FocusHandle,
    pub keybindings: FocusHandle,
}

pub(in crate::features) struct StoreStatusView<'a> {
    pub path: &'a str,
    pub message: &'a str,
    pub ready: bool,
}

/// Borrowed staged master-password state. Deliberately does not implement
/// `Debug` so the secret draft cannot be exposed through aggregate logging.
pub(in crate::features) struct MasterPasswordView<'a> {
    pub enabled: bool,
    pub draft: &'a str,
}

pub(in crate::features) struct UiLayoutSettingsUpdate {
    pub left_panel_width: u32,
    pub right_panel_width: u32,
    pub transfer_height: u32,
    pub quick_command_height: u32,
    pub quick_command_visible: bool,
    pub serial_send_height: u32,
    pub serial_send_visible: bool,
    pub active_left_panel: Option<String>,
    pub active_right_panel: Option<String>,
    pub left_panel_collapsed: bool,
    pub right_panel_collapsed: bool,
    pub saved_connections_sort_mode: String,
    pub activity_bar_left_top: Vec<String>,
    pub activity_bar_left_bottom: Vec<String>,
    pub activity_bar_right_top: Vec<String>,
    pub activity_bar_right_bottom: Vec<String>,
    pub activity_bar_show_labels: bool,
    pub panel_multi_open: bool,
    pub left_open_panels: Vec<String>,
    pub right_open_panels: Vec<String>,
    pub panel_stack_sizes: HashMap<String, u32>,
}

struct SearchEngineSettingsState {
    expanded_index: Option<usize>,
    icon_picker_index: Option<usize>,
    actions_index: Option<usize>,
    focus: FocusHandle,
}

struct KeywordHighlightSettingsState {
    expanded_id: Option<String>,
    edit_id: Option<String>,
    edit_field: KeywordHighlightEditorField,
    focus: FocusHandle,
}

struct AppearanceSettingsState {
    menu_open: Option<String>,
    ui_font_options: Vec<String>,
    terminal_font_options: Vec<String>,
}

struct KeybindingSettingsState {
    recording_id: Option<String>,
    pending_keys: Option<String>,
    search_draft: String,
    focus: FocusHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::features) struct SearchEnginePresentationState {
    pub expanded_index: Option<usize>,
    pub icon_picker_index: Option<usize>,
    pub actions_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::features) struct KeywordHighlightPresentationState {
    pub expanded_id: Option<String>,
    pub edit_id: Option<String>,
    pub edit_field: KeywordHighlightEditorField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::features) struct KeybindingPresentationState {
    pub recording_id: Option<String>,
    pub pending_keys: Option<String>,
    pub search_draft: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::features) enum SearchEngineMenu {
    Icon,
    Actions,
}

impl SettingsFeatureState {
    pub(in crate::features) fn new(
        summary: AppSettingsSummary,
        keyword_config: KeywordHighlightConfig,
        store_path: String,
        store_message: String,
        store_ready: bool,
        ui_font_options: Vec<String>,
        terminal_font_options: Vec<String>,
        focus: SettingsFeatureFocus,
    ) -> Self {
        let master_password = SettingsMasterPasswordState::new(summary.has_master_password);
        Self {
            summary,
            keyword_config,
            master_password,
            store_status: StoreStatus {
                path: store_path,
                message: store_message,
                ready: store_ready,
            },
            search_engines: SearchEngineSettingsState {
                expanded_index: None,
                icon_picker_index: None,
                actions_index: None,
                focus: focus.search_engine,
            },
            keyword_highlights: KeywordHighlightSettingsState {
                expanded_id: None,
                edit_id: None,
                edit_field: KeywordHighlightEditorField::Name,
                focus: focus.keyword_highlight,
            },
            appearance: AppearanceSettingsState {
                menu_open: None,
                ui_font_options,
                terminal_font_options,
            },
            keybindings: KeybindingSettingsState {
                recording_id: None,
                pending_keys: None,
                search_draft: String::new(),
                focus: focus.keybindings,
            },
            prompts: SettingsPromptState::default(),
        }
    }

    pub(in crate::features) fn rebase_master_password(&mut self) {
        self.master_password.reset(self.summary.has_master_password);
    }

    pub(in crate::features) fn summary(&self) -> &AppSettingsSummary {
        &self.summary
    }

    pub(in crate::features) fn replace_summary(&mut self, summary: AppSettingsSummary) {
        self.summary = summary;
    }

    pub(in crate::features) fn set_language(&mut self, language: &str) {
        self.summary.language = language.to_string();
    }

    pub(in crate::features) fn toggle_startup_restore(&mut self) {
        self.summary.startup_restore = !self.summary.startup_restore;
    }

    pub(in crate::features) fn toggle_startup_restore_window_layout(&mut self) -> bool {
        self.summary.startup_restore_window_layout = !self.summary.startup_restore_window_layout;
        self.summary.startup_restore_window_layout
    }

    pub(in crate::features) fn toggle_confirm_on_close(&mut self) {
        self.summary.confirm_on_close = !self.summary.confirm_on_close;
    }

    pub(in crate::features) fn toggle_minimize_to_tray(&mut self) {
        self.summary.minimize_to_tray = !self.summary.minimize_to_tray;
    }

    pub(in crate::features) fn set_diagnostics_level(&mut self, level: &str) -> bool {
        let level = match level {
            "warn" | "debug" => level,
            _ => "info",
        };
        if self.summary.diagnostics_level == level {
            return false;
        }
        self.summary.diagnostics_level = level.to_string();
        true
    }

    pub(in crate::features) fn set_diagnostics_retention_days(&mut self, days: u32) -> bool {
        let days = match days {
            3 | 7 | 14 | 30 => days,
            _ => 7,
        };
        if self.summary.diagnostics_retention_days == days {
            return false;
        }
        self.summary.diagnostics_retention_days = days;
        true
    }

    pub(in crate::features) fn toggle_interaction_copy_on_select(&mut self) {
        self.summary.interaction_copy_on_select = !self.summary.interaction_copy_on_select;
    }

    pub(in crate::features) fn toggle_interaction_right_click_paste(&mut self) {
        self.summary.interaction_right_click_paste = !self.summary.interaction_right_click_paste;
    }

    pub(in crate::features) fn toggle_command_suggestions(&mut self) -> bool {
        self.summary.interaction_command_suggestions_enabled =
            !self.summary.interaction_command_suggestions_enabled;
        self.summary.interaction_command_suggestions_enabled
    }

    pub(in crate::features) fn adjust_command_suggestion_min_chars(&mut self, delta: i32) {
        let max_chars = self.summary.interaction_command_suggestion_max_chars;
        self.summary.interaction_command_suggestion_min_chars =
            (self.summary.interaction_command_suggestion_min_chars as i32 + delta)
                .clamp(1, max_chars as i32) as u32;
    }

    pub(in crate::features) fn adjust_command_suggestion_max_chars(&mut self, delta: i32) {
        let min_chars = self.summary.interaction_command_suggestion_min_chars;
        self.summary.interaction_command_suggestion_max_chars =
            (self.summary.interaction_command_suggestion_max_chars as i32 + delta)
                .clamp(min_chars as i32, 500) as u32;
    }

    pub(in crate::features) fn adjust_duplicate_session_command_delay(&mut self, delta_ms: i32) {
        self.summary.interaction_duplicate_session_command_delay_ms =
            (self.summary.interaction_duplicate_session_command_delay_ms as i32 + delta_ms)
                .clamp(0, 60_000) as u32;
    }

    pub(in crate::features) fn toggle_alt_as_meta(&mut self) {
        self.summary.interaction_alt_as_meta = !self.summary.interaction_alt_as_meta;
    }

    pub(in crate::features) fn toggle_mac_ime_compatibility(&mut self) {
        self.summary.interaction_mac_ime_compatibility =
            !self.summary.interaction_mac_ime_compatibility;
    }

    pub(in crate::features) fn set_interaction_encoding(&mut self, encoding: &str) {
        self.summary.interaction_default_encoding = encoding.to_string();
    }

    pub(in crate::features) fn set_interaction_word_separators(&mut self, text: String) {
        self.summary.interaction_word_separators = text;
    }

    pub(in crate::features) fn toggle_screen_lock_enabled(&mut self) {
        self.summary.enable_screen_lock = !self.summary.enable_screen_lock;
    }

    pub(in crate::features) fn adjust_idle_lock_minutes(&mut self, delta: i32) {
        self.summary.idle_lock_minutes =
            (self.summary.idle_lock_minutes as i32 + delta).clamp(0, 1440) as u32;
    }

    pub(in crate::features) fn set_terminal_x11_display(&mut self, text: String) {
        self.summary.x11_display = text;
    }

    pub(in crate::features) fn toggle_terminal_hardware_acceleration(&mut self) {
        self.summary.terminal_hardware_acceleration = !self.summary.terminal_hardware_acceleration;
    }

    pub(in crate::features) fn toggle_terminal_low_latency_mode(&mut self) -> bool {
        self.summary.terminal_low_latency_mode = !self.summary.terminal_low_latency_mode;
        self.summary.terminal_low_latency_mode
    }

    pub(in crate::features) fn adjust_terminal_scrollback_lines(&mut self, delta: i32) {
        self.summary.terminal_scrollback_lines =
            (self.summary.terminal_scrollback_lines as i32 + delta).clamp(100, 100_000) as u32;
    }

    pub(in crate::features) fn adjust_terminal_keep_alive_interval(&mut self, delta: i32) {
        self.summary.terminal_keep_alive_interval =
            (self.summary.terminal_keep_alive_interval as i32 + delta).clamp(0, 600) as u32;
    }

    pub(in crate::features) fn toggle_terminal_workspace_padding(&mut self) {
        self.summary.terminal_show_workspace_padding =
            !self.summary.terminal_show_workspace_padding;
    }

    pub(in crate::features) fn toggle_terminal_line_numbers(&mut self) {
        self.summary.terminal_show_line_numbers = !self.summary.terminal_show_line_numbers;
    }

    pub(in crate::features) fn toggle_terminal_timestamps(&mut self) {
        self.summary.terminal_show_timestamps = !self.summary.terminal_show_timestamps;
    }

    pub(in crate::features) fn toggle_terminal_timestamp_milliseconds(&mut self) {
        self.summary.terminal_show_timestamp_milliseconds =
            !self.summary.terminal_show_timestamp_milliseconds;
    }

    pub(in crate::features) fn toggle_multi_line_paste_dialog(&mut self) {
        self.summary.terminal_show_multi_line_paste_dialog =
            !self.summary.terminal_show_multi_line_paste_dialog;
    }

    pub(in crate::features) fn toggle_paste_image_as_path(&mut self) {
        self.summary.terminal_paste_image_as_path = !self.summary.terminal_paste_image_as_path;
    }

    pub(in crate::features) fn toggle_remote_stats_panel(&mut self) {
        self.summary.ui_show_remote_stats = !self.summary.ui_show_remote_stats;
    }

    pub(in crate::features) fn adjust_remote_stats_interval(&mut self, delta: i32) {
        self.summary.ui_remote_stats_interval =
            (self.summary.ui_remote_stats_interval as i32 + delta).clamp(1, 60) as u32;
    }

    pub(in crate::features) fn toggle_process_manager_panel(&mut self) {
        self.summary.ui_show_process_manager = !self.summary.ui_show_process_manager;
    }

    pub(in crate::features) fn adjust_process_manager_interval(&mut self, delta: i32) {
        self.summary.ui_process_manager_interval =
            (self.summary.ui_process_manager_interval as i32 + delta).clamp(3, 120) as u32;
    }

    pub(in crate::features) fn toggle_docker_manager_panel(&mut self) {
        self.summary.ui_show_docker_manager = !self.summary.ui_show_docker_manager;
    }

    pub(in crate::features) fn adjust_docker_manager_interval(&mut self, delta: i32) {
        self.summary.ui_docker_manager_interval =
            (self.summary.ui_docker_manager_interval as i32 + delta).clamp(3, 120) as u32;
    }

    pub(in crate::features) fn toggle_terminal_action_links(&mut self) {
        self.summary.terminal_action_links_enabled = !self.summary.terminal_action_links_enabled;
    }

    pub(in crate::features) fn toggle_terminal_action_link_matcher(&mut self, which: &str) -> bool {
        let matcher = &mut self.summary.terminal_action_links_matchers;
        match which {
            "ipv4" => matcher.ipv4 = !matcher.ipv4,
            "archive" => matcher.archive = !matcher.archive,
            "host_port" => matcher.host_port = !matcher.host_port,
            _ => return false,
        }
        true
    }

    pub(in crate::features) fn set_host_key_policy(&mut self, policy: &str) {
        self.summary.host_key_policy = policy.to_string();
    }

    pub(in crate::features) fn toggle_recording_auto_start(&mut self) {
        self.summary.recording_auto_start = !self.summary.recording_auto_start;
    }

    pub(in crate::features) fn toggle_recording_io_labels(&mut self) {
        self.summary.recording_include_io_labels = !self.summary.recording_include_io_labels;
    }

    pub(in crate::features) fn toggle_recording_timestamps(&mut self) {
        self.summary.recording_include_timestamps = !self.summary.recording_include_timestamps;
    }

    pub(in crate::features) fn adjust_recording_memory_limit(&mut self, delta_mib: i64) {
        let current_mib = (self.summary.recording_memory_limit_bytes / (1024 * 1024)).max(1);
        let next_mib = if delta_mib.is_negative() {
            current_mib.saturating_sub(delta_mib.unsigned_abs()).max(1)
        } else {
            current_mib.saturating_add(delta_mib as u64).min(512)
        };
        self.summary.recording_memory_limit_bytes = next_mib * 1024 * 1024;
    }

    pub(in crate::features) fn set_transfer_duplicate_strategy(&mut self, strategy: String) {
        self.summary.transfer_duplicate_strategy = strategy;
    }

    pub(in crate::features) fn set_transfer_editor_type(&mut self, editor_type: &str) {
        self.summary.transfer_editor_type = editor_type.to_string();
    }

    pub(in crate::features) fn toggle_transfer_ask_save_location(&mut self) {
        self.summary.transfer_ask_save_location = !self.summary.transfer_ask_save_location;
    }

    pub(in crate::features) fn toggle_transfer_preserve_timestamps(&mut self) {
        self.summary.transfer_preserve_timestamps = !self.summary.transfer_preserve_timestamps;
    }

    pub(in crate::features) fn toggle_transfer_resume_broken(&mut self) {
        self.summary.transfer_resume_broken_transfer =
            !self.summary.transfer_resume_broken_transfer;
    }

    pub(in crate::features) fn adjust_transfer_download_threads(&mut self, delta: i32) {
        self.summary.transfer_download_threads =
            adjust_u32_setting(self.summary.transfer_download_threads, delta, 1, 10);
    }

    pub(in crate::features) fn adjust_transfer_upload_threads(&mut self, delta: i32) {
        self.summary.transfer_upload_threads =
            adjust_u32_setting(self.summary.transfer_upload_threads, delta, 1, 10);
    }

    pub(in crate::features) fn adjust_transfer_max_retries(&mut self, delta: i32) {
        self.summary.transfer_max_retries =
            adjust_u32_setting(self.summary.transfer_max_retries, delta, 0, 10);
    }

    pub(in crate::features) fn adjust_transfer_buffer_size(&mut self, delta: i32) {
        self.summary.transfer_buffer_size = adjust_u32_setting(
            self.summary.transfer_buffer_size,
            delta.saturating_mul(8),
            8,
            256,
        );
    }

    pub(in crate::features) fn set_transfer_file_permissions(&mut self, permissions: &str) {
        self.summary.transfer_default_file_permissions = permissions.to_string();
    }

    pub(in crate::features) fn keyword_config(&self) -> &KeywordHighlightConfig {
        &self.keyword_config
    }

    pub(in crate::features) fn replace_keyword_config(&mut self, config: KeywordHighlightConfig) {
        self.keyword_config = config;
    }

    pub(in crate::features) fn toggle_keyword_highlights(&mut self) -> bool {
        self.keyword_config.enabled = !self.keyword_config.enabled;
        self.keyword_config.enabled
    }

    pub(in crate::features) fn toggle_keyword_highlights_wrapped(&mut self) -> bool {
        self.keyword_config.across_wrapped_lines = !self.keyword_config.across_wrapped_lines;
        self.keyword_config.across_wrapped_lines
    }

    pub(in crate::features) fn toggle_keyword_highlight_builtin(&mut self, rule_id: String) {
        let enabled = self
            .keyword_config
            .builtin_rules
            .get(&rule_id)
            .copied()
            .unwrap_or(true);
        self.keyword_config.builtin_rules.insert(rule_id, !enabled);
    }

    pub(in crate::features) fn toggle_keyword_highlight_rule(&mut self, rule_id: &str) -> bool {
        let Some(rule) = self
            .keyword_config
            .rules
            .iter_mut()
            .find(|rule| rule.id == rule_id)
        else {
            return false;
        };
        rule.enabled = !rule.enabled;
        true
    }

    pub(in crate::features) fn set_keyword_highlight_rule_color(
        &mut self,
        rule_id: &str,
        dark: bool,
        color: String,
    ) -> bool {
        let Some(rule) = self
            .keyword_config
            .rules
            .iter_mut()
            .find(|rule| rule.id == rule_id)
        else {
            return false;
        };
        if dark {
            rule.color_dark = color;
        } else {
            rule.color_light = color;
        }
        true
    }

    pub(in crate::features) fn apply_keyword_highlight_rule_input(
        &mut self,
        rule_id: &str,
        field: KeywordHighlightEditorField,
        text: String,
    ) -> bool {
        let Some(rule) = self
            .keyword_config
            .rules
            .iter_mut()
            .find(|rule| rule.id == rule_id)
        else {
            return false;
        };
        match field {
            KeywordHighlightEditorField::Name => rule.name = text,
            KeywordHighlightEditorField::Patterns => {
                rule.patterns = text.split('\n').map(ToOwned::to_owned).collect();
            }
            KeywordHighlightEditorField::ColorDark => rule.color_dark = text,
            KeywordHighlightEditorField::ColorLight => rule.color_light = text,
        }
        self.begin_keyword_highlight_edit(rule_id.to_string(), field);
        true
    }

    pub(in crate::features) fn master_password(&self) -> MasterPasswordView<'_> {
        MasterPasswordView {
            enabled: self.master_password.enabled,
            draft: &self.master_password.draft,
        }
    }

    pub(in crate::features) fn restore_master_password_draft(
        &mut self,
        enabled: bool,
        draft: String,
    ) {
        self.master_password.enabled = enabled;
        self.master_password.draft = draft;
    }

    pub(in crate::features) fn toggle_master_password(
        &mut self,
        cloud_sync_enabled: bool,
    ) -> Result<bool, &'static str> {
        self.master_password.toggle(cloud_sync_enabled)
    }

    pub(in crate::features) fn edit_master_password_draft(&mut self, text: String) -> bool {
        self.master_password.edit_draft(text)
    }

    pub(in crate::features) fn set_quick_command_view_mode(&mut self, mode: String) {
        self.summary.ui_quick_cmd_view_mode = mode;
    }

    pub(in crate::features) fn set_quick_command_sort_mode(&mut self, mode: String) {
        self.summary.ui_quick_cmd_sort_mode = mode;
    }

    pub(in crate::features) fn set_saved_connections_sort_mode(&mut self, mode: String) {
        self.summary.ui_saved_connections_sort_mode = mode;
    }

    pub(in crate::features) fn set_header_status_mode(&mut self, mode: String) {
        self.summary.ui_header_status_mode = mode;
        self.summary.ui_header_status_visible = true;
    }

    pub(in crate::features) fn set_header_status_visible(&mut self, visible: bool) {
        self.summary.ui_header_status_visible = visible;
    }

    pub(in crate::features) fn set_keybindings(&mut self, keybindings: HashMap<String, String>) {
        self.summary.keybindings = keybindings;
    }

    pub(in crate::features) fn set_transfer_download_path(&mut self, path: String) {
        self.summary.transfer_download_path = path;
    }

    pub(in crate::features) fn set_recording_path(&mut self, path: String) {
        self.summary.recording_path = path;
    }

    pub(in crate::features) fn set_transfer_default_editor(&mut self, path: String) {
        self.summary.transfer_default_editor = path;
    }

    pub(in crate::features) fn set_tab_double_click_action(&mut self, action: String) {
        self.summary.interaction_tab_double_click_action = action;
    }

    pub(in crate::features) fn set_tab_middle_click_action(&mut self, action: String) {
        self.summary.interaction_tab_middle_click_action = action;
    }

    pub(in crate::features) fn set_tab_right_click_action(&mut self, action: String) {
        self.summary.interaction_tab_right_click_action = action;
    }

    pub(in crate::features) fn apply_ui_layout(&mut self, update: UiLayoutSettingsUpdate) {
        self.summary.ui_left_panel_width = update.left_panel_width;
        self.summary.ui_right_panel_width = update.right_panel_width;
        self.summary.ui_transfer_height = update.transfer_height;
        self.summary.ui_quick_cmd_height = update.quick_command_height;
        self.summary.ui_quick_cmd_visible = update.quick_command_visible;
        self.summary.ui_serial_send_height = update.serial_send_height;
        self.summary.ui_serial_send_visible = update.serial_send_visible;
        self.summary.ui_active_left_panel = update.active_left_panel;
        self.summary.ui_active_right_panel = update.active_right_panel;
        self.summary.ui_left_panel_collapsed = update.left_panel_collapsed;
        self.summary.ui_right_panel_collapsed = update.right_panel_collapsed;
        self.summary.ui_saved_connections_sort_mode = update.saved_connections_sort_mode;
        self.summary.ui_activity_bar_left_top = update.activity_bar_left_top;
        self.summary.ui_activity_bar_left_bottom = update.activity_bar_left_bottom;
        self.summary.ui_activity_bar_right_top = update.activity_bar_right_top;
        self.summary.ui_activity_bar_right_bottom = update.activity_bar_right_bottom;
        self.summary.ui_activity_bar_show_labels = update.activity_bar_show_labels;
        self.summary.ui_panel_multi_open = update.panel_multi_open;
        self.summary.ui_left_open_panels = update.left_open_panels;
        self.summary.ui_right_open_panels = update.right_open_panels;
        self.summary.ui_panel_stack_sizes = update.panel_stack_sizes;
    }

    pub(in crate::features) fn set_appearance_theme(&mut self, theme: String) {
        self.summary.theme = theme;
    }

    pub(in crate::features) fn set_terminal_font_family(&mut self, family: String) -> bool {
        if self.summary.terminal_font_family == family {
            return false;
        }
        self.summary.terminal_font_family = family;
        true
    }

    pub(in crate::features) fn set_terminal_font_size(&mut self, size: u16) -> bool {
        if self.summary.terminal_font_size == size {
            return false;
        }
        self.summary.terminal_font_size = size;
        true
    }

    pub(in crate::features) fn set_cursor_style(&mut self, style: String) {
        self.summary.cursor_style = style;
    }

    pub(in crate::features) fn toggle_cursor_blink(&mut self) -> bool {
        self.summary.cursor_blink = !self.summary.cursor_blink;
        self.summary.cursor_blink
    }

    pub(in crate::features) fn set_terminal_theme(&mut self, theme: Option<String>) {
        self.summary.terminal_theme = theme;
    }

    pub(in crate::features) fn set_minimum_contrast_ratio(&mut self, ratio: String) -> bool {
        if self.summary.minimum_contrast_ratio == ratio {
            return false;
        }
        self.summary.minimum_contrast_ratio = ratio;
        true
    }

    pub(in crate::features) fn set_ui_font_family(&mut self, family: String) {
        self.summary.ui_font_family = family;
    }

    pub(in crate::features) fn set_ui_font_size(&mut self, size: u16) {
        self.summary.ui_font_size = size;
    }

    pub(in crate::features) fn set_terminal_font_weight(&mut self, weight: u16) -> bool {
        if self.summary.terminal_font_weight == weight {
            return false;
        }
        self.summary.terminal_font_weight = weight;
        true
    }

    pub(in crate::features) fn set_terminal_font_weight_bold(&mut self, weight: u16) -> bool {
        if self.summary.terminal_font_weight_bold == weight {
            return false;
        }
        self.summary.terminal_font_weight_bold = weight;
        true
    }

    pub(in crate::features) fn select_background_image(&mut self, path: String) {
        self.summary.background_image_path = Some(path);
        if self.summary.background_image_fit.trim().is_empty() {
            self.summary.background_image_fit = "cover".to_string();
        }
    }

    pub(in crate::features) fn clear_background_image(&mut self) {
        self.summary.background_image_path = None;
    }

    pub(in crate::features) fn set_background_image_fit(&mut self, fit: String) {
        self.summary.background_image_fit = fit;
    }

    pub(in crate::features) fn set_background_image_opacity(&mut self, opacity: u8) -> bool {
        if self.summary.background_image_opacity == opacity {
            return false;
        }
        self.summary.background_image_opacity = opacity;
        true
    }

    pub(in crate::features) fn set_background_content_opacity(&mut self, opacity: u8) -> bool {
        if self.summary.background_content_opacity == opacity {
            return false;
        }
        self.summary.background_content_opacity = opacity;
        true
    }

    pub(in crate::features) fn toggle_file_explorer_auto_sync_cwd(
        &mut self,
        connection_id: String,
    ) -> bool {
        let ids = &mut self.summary.ui_file_explorer_auto_sync_cwd_connection_ids;
        let was_enabled = ids.iter().any(|id| id == &connection_id);
        ids.retain(|id| id != &connection_id);
        if !was_enabled {
            ids.push(connection_id);
        }
        !was_enabled
    }

    pub(in crate::features) fn toggle_file_explorer_hidden_files(&mut self) -> bool {
        self.summary.ui_file_explorer_show_hidden_files =
            !self.summary.ui_file_explorer_show_hidden_files;
        self.summary.ui_file_explorer_show_hidden_files
    }

    pub(in crate::features) fn set_file_explorer_favorites(
        &mut self,
        connection_id: String,
        favorites: Vec<String>,
    ) {
        if favorites.is_empty() {
            self.summary
                .ui_file_explorer_favorite_dirs_by_connection_id
                .remove(&connection_id);
        } else {
            self.summary
                .ui_file_explorer_favorite_dirs_by_connection_id
                .insert(connection_id, favorites);
        }
    }

    pub(in crate::features) fn store_status(&self) -> StoreStatusView<'_> {
        StoreStatusView {
            path: &self.store_status.path,
            message: &self.store_status.message,
            ready: self.store_status.ready,
        }
    }

    pub(in crate::features) fn set_store_message(&mut self, message: impl Into<String>) {
        self.store_status.message = message.into();
    }

    pub(in crate::features) fn set_store_ready(&mut self, ready: bool) {
        self.store_status.ready = ready;
    }

    pub(in crate::features) fn replace_store_status(
        &mut self,
        path: String,
        message: String,
        ready: bool,
    ) {
        self.store_status = StoreStatus {
            path,
            message,
            ready,
        };
    }

    pub(in crate::features) fn search_engine_presentation(&self) -> SearchEnginePresentationState {
        SearchEnginePresentationState {
            expanded_index: self.search_engines.expanded_index,
            icon_picker_index: self.search_engines.icon_picker_index,
            actions_index: self.search_engines.actions_index,
        }
    }

    pub(in crate::features) fn search_engine_focus(&self) -> &FocusHandle {
        &self.search_engines.focus
    }

    pub(in crate::features) fn apply_search_engine_input(
        &mut self,
        rest: &str,
        text: String,
    ) -> bool {
        let Some((index, field)) = rest.split_once('.') else {
            return false;
        };
        let Ok(index) = index.parse::<usize>() else {
            return false;
        };
        let Some(engine) = self.summary.search_custom_engines.get_mut(index) else {
            return false;
        };
        match field {
            "name" => engine.name = text,
            "url" => engine.url_template = text,
            _ => return false,
        }
        true
    }

    pub(in crate::features) fn add_search_engine(&mut self, engine: SearchEngineConfig) {
        self.summary.search_custom_engines.insert(0, engine);
        self.search_engines.expanded_index = Some(0);
        self.search_engines.close_menus();
    }

    pub(in crate::features) fn remove_search_engine(&mut self, index: usize) -> bool {
        if index >= self.summary.search_custom_engines.len() {
            return false;
        }
        self.summary.search_custom_engines.remove(index);
        self.search_engines.close_menus();
        self.search_engines.expanded_index =
            adjusted_index_after_remove(self.search_engines.expanded_index, index);
        true
    }

    pub(in crate::features) fn set_search_engine_icon(
        &mut self,
        index: usize,
        icon: Option<&str>,
    ) -> bool {
        let Some(engine) = self.summary.search_custom_engines.get_mut(index) else {
            return false;
        };
        engine.icon = icon.map(str::to_string);
        self.search_engines.icon_picker_index = None;
        true
    }

    pub(in crate::features) fn toggle_search_engine_in_menu(&mut self, index: usize) -> bool {
        let Some(engine) = self.summary.search_custom_engines.get_mut(index) else {
            return false;
        };
        engine.show_in_menu = !engine.show_in_menu;
        true
    }

    /// Returns whether collapsing the row requires normalized values to be persisted.
    pub(in crate::features) fn toggle_search_engine_expanded(
        &mut self,
        index: usize,
    ) -> Option<bool> {
        if index >= self.summary.search_custom_engines.len() {
            return None;
        }
        let collapsed = self.search_engines.expanded_index == Some(index);
        if collapsed {
            self.search_engines.expanded_index = None;
            self.normalize_search_engines();
        } else {
            self.search_engines.expanded_index = Some(index);
        }
        self.search_engines.close_menus();
        Some(collapsed)
    }

    pub(in crate::features) fn toggle_search_engine_menu(
        &mut self,
        menu: SearchEngineMenu,
        index: usize,
    ) -> bool {
        if index >= self.summary.search_custom_engines.len() {
            return false;
        }
        let next = match menu {
            SearchEngineMenu::Icon => self.search_engines.icon_picker_index != Some(index),
            SearchEngineMenu::Actions => self.search_engines.actions_index != Some(index),
        };
        self.search_engines.close_menus();
        if next {
            match menu {
                SearchEngineMenu::Icon => self.search_engines.icon_picker_index = Some(index),
                SearchEngineMenu::Actions => self.search_engines.actions_index = Some(index),
            }
        }
        true
    }

    pub(in crate::features) fn close_search_engine_menus(&mut self) {
        self.search_engines.close_menus();
    }

    pub(in crate::features) fn normalize_search_engines(&mut self) {
        for engine in &mut self.summary.search_custom_engines {
            engine.name = engine.name.trim().to_string();
            engine.url_template = engine.url_template.trim().to_string();
        }
    }

    pub(in crate::features) fn keyword_highlight_presentation(
        &self,
    ) -> KeywordHighlightPresentationState {
        KeywordHighlightPresentationState {
            expanded_id: self.keyword_highlights.expanded_id.clone(),
            edit_id: self.keyword_highlights.edit_id.clone(),
            edit_field: self.keyword_highlights.edit_field,
        }
    }

    pub(in crate::features) fn keyword_highlight_focus(&self) -> &FocusHandle {
        &self.keyword_highlights.focus
    }

    pub(in crate::features) fn clear_keyword_highlight_edit(&mut self) {
        self.keyword_highlights.edit_id = None;
    }

    /// Returns ids whose registry-backed inputs should be discarded.
    pub(in crate::features) fn toggle_keyword_highlight_expanded(
        &mut self,
        rule_id: String,
    ) -> Vec<String> {
        let mut forgotten = Vec::new();
        if self.keyword_highlights.expanded_id.as_deref() == Some(rule_id.as_str()) {
            self.keyword_highlights.expanded_id = None;
            self.keyword_highlights.edit_id = None;
            forgotten.push(rule_id);
        } else {
            if let Some(previous_id) = self.keyword_highlights.expanded_id.replace(rule_id) {
                forgotten.push(previous_id);
            }
            self.keyword_highlights.edit_id = None;
        }
        forgotten
    }

    pub(in crate::features) fn begin_keyword_highlight_edit(
        &mut self,
        rule_id: String,
        field: KeywordHighlightEditorField,
    ) {
        self.keyword_highlights.expanded_id = Some(rule_id.clone());
        self.keyword_highlights.edit_id = Some(rule_id);
        self.keyword_highlights.edit_field = field;
    }

    fn remove_keyword_highlight_rule_reference(&mut self, rule_id: &str) {
        if self.keyword_highlights.expanded_id.as_deref() == Some(rule_id) {
            self.keyword_highlights.expanded_id = None;
        }
        if self.keyword_highlights.edit_id.as_deref() == Some(rule_id) {
            self.keyword_highlights.edit_id = None;
        }
    }

    pub(in crate::features) fn remove_keyword_highlight_rule(&mut self, rule_id: &str) -> bool {
        let previous_len = self.keyword_config.rules.len();
        self.keyword_config.rules.retain(|rule| rule.id != rule_id);
        if self.keyword_config.rules.len() == previous_len {
            return false;
        }
        self.remove_keyword_highlight_rule_reference(rule_id);
        true
    }

    pub(in crate::features) fn add_keyword_highlight_rule(&mut self, rule: KeywordHighlightRule) {
        let id = rule.id.clone();
        self.keyword_config.rules.push(rule);
        self.begin_keyword_highlight_edit(id, KeywordHighlightEditorField::Name);
    }

    pub(in crate::features) fn keybinding_presentation(&self) -> KeybindingPresentationState {
        KeybindingPresentationState {
            recording_id: self.keybindings.recording_id.clone(),
            pending_keys: self.keybindings.pending_keys.clone(),
            search_draft: self.keybindings.search_draft.clone(),
        }
    }

    pub(in crate::features) fn keybinding_focus(&self) -> &FocusHandle {
        &self.keybindings.focus
    }

    pub(in crate::features) fn begin_keybinding_recording(&mut self, shortcut_id: String) {
        self.keybindings.recording_id = Some(shortcut_id);
        self.keybindings.pending_keys = None;
    }

    pub(in crate::features) fn cancel_keybinding_recording(&mut self) {
        self.keybindings.recording_id = None;
        self.keybindings.pending_keys = None;
    }

    pub(in crate::features) fn keybinding_recording_id(&self) -> Option<&str> {
        self.keybindings.recording_id.as_deref()
    }

    pub(in crate::features) fn pending_keybinding(&self) -> Option<&str> {
        self.keybindings.pending_keys.as_deref()
    }

    pub(in crate::features) fn set_pending_keybinding(&mut self, keys: Option<String>) {
        self.keybindings.pending_keys = keys;
    }

    pub(in crate::features) fn finish_keybinding_recording(&mut self) {
        self.cancel_keybinding_recording();
    }

    pub(in crate::features) fn set_keybinding_search(&mut self, text: String) {
        self.keybindings.search_draft = text;
    }

    pub(in crate::features) fn clear_keybinding_search(&mut self) {
        self.keybindings.search_draft.clear();
    }

    pub(in crate::features) fn appearance_menu_open(&self, id: &str) -> bool {
        self.appearance.menu_open.as_deref() == Some(id)
    }

    pub(in crate::features) fn toggle_appearance_menu(&mut self, id: &str) {
        if self.appearance_menu_open(id) {
            self.appearance.menu_open = None;
        } else {
            self.appearance.menu_open = Some(id.to_string());
        }
    }

    pub(in crate::features) fn close_appearance_menu(&mut self) {
        self.appearance.menu_open = None;
    }

    pub(in crate::features) fn ui_font_options(&self) -> &[String] {
        &self.appearance.ui_font_options
    }

    pub(in crate::features) fn terminal_font_options(&self) -> &[String] {
        &self.appearance.terminal_font_options
    }

    pub(in crate::features) fn config_path_prompt_active(&self) -> bool {
        self.prompts.config_path.is_some()
    }

    pub(in crate::features) fn begin_config_path_prompt(
        &mut self,
        kind: ConfigPathPromptKind,
    ) -> bool {
        if self.prompts.config_path.is_some() {
            return false;
        }
        self.prompts.config_path = Some(kind);
        true
    }

    pub(in crate::features) fn finish_config_path_prompt(
        &mut self,
        kind: ConfigPathPromptKind,
    ) -> bool {
        if self.prompts.config_path != Some(kind) {
            return false;
        }
        self.prompts.config_path = None;
        true
    }

    pub(in crate::features) fn begin_diagnostics_path_prompt(&mut self) -> bool {
        if self.prompts.diagnostics_path.is_some() {
            return false;
        }
        self.prompts.diagnostics_path = Some(DiagnosticsPathPromptKind::Export);
        true
    }

    pub(in crate::features) fn finish_diagnostics_path_prompt(&mut self) -> bool {
        if self.prompts.diagnostics_path != Some(DiagnosticsPathPromptKind::Export) {
            return false;
        }
        self.prompts.diagnostics_path = None;
        true
    }

    pub(in crate::features) fn begin_keyword_highlight_path_prompt(&mut self) -> bool {
        if self.prompts.keyword_highlight_path.is_some() {
            return false;
        }
        self.prompts.keyword_highlight_path = Some(KeywordHighlightPathPromptKind::Import);
        true
    }

    pub(in crate::features) fn finish_keyword_highlight_path_prompt(&mut self) -> bool {
        if self.prompts.keyword_highlight_path != Some(KeywordHighlightPathPromptKind::Import) {
            return false;
        }
        self.prompts.keyword_highlight_path = None;
        true
    }

    pub(in crate::features) fn snapshot_password_prompt(
        &self,
    ) -> Option<SnapshotPasswordPromptState> {
        self.prompts.snapshot_password.clone()
    }

    pub(in crate::features) fn snapshot_password_prompt_active(&self) -> bool {
        self.prompts.snapshot_password.is_some()
    }

    pub(in crate::features) fn begin_snapshot_password_prompt(
        &mut self,
        kind: crate::models::SnapshotPasswordPromptKind,
    ) -> bool {
        if self.prompts.config_path.is_some() {
            return false;
        }
        self.prompts.snapshot_password = Some(SnapshotPasswordPromptState {
            kind,
            value: String::new(),
        });
        true
    }

    pub(in crate::features) fn take_snapshot_password_prompt(
        &mut self,
    ) -> Option<SnapshotPasswordPromptState> {
        self.prompts.snapshot_password.take()
    }

    pub(in crate::features) fn restore_snapshot_password_prompt(
        &mut self,
        kind: crate::models::SnapshotPasswordPromptKind,
    ) {
        self.prompts.snapshot_password = Some(SnapshotPasswordPromptState {
            kind,
            value: String::new(),
        });
    }

    pub(in crate::features) fn apply_snapshot_password_input(&mut self, text: String) -> bool {
        let Some(state) = self.prompts.snapshot_password.as_mut() else {
            return false;
        };
        state.value = text;
        true
    }
}

impl SearchEngineSettingsState {
    fn close_menus(&mut self) {
        self.icon_picker_index = None;
        self.actions_index = None;
    }
}

fn adjusted_index_after_remove(value: Option<usize>, removed: usize) -> Option<usize> {
    match value {
        Some(index) if index == removed => None,
        Some(index) if index > removed => Some(index - 1),
        other => other,
    }
}

fn adjust_u32_setting(current: u32, delta: i32, min: u32, max: u32) -> u32 {
    let next = if delta.is_negative() {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current.saturating_add(delta as u32)
    };
    next.clamp(min, max)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use gpui::TestAppContext;
    use nyaterm_core::{
        AppSettingsSummary, KeywordHighlightConfig, KeywordHighlightRule, SearchEngineConfig,
    };

    use super::{
        SearchEngineMenu, SettingsFeatureFocus, SettingsFeatureState, UiLayoutSettingsUpdate,
    };
    use crate::models::{
        ConfigPathPromptKind, KeywordHighlightEditorField, SnapshotPasswordPromptKind,
    };

    fn settings_state() -> SettingsFeatureState {
        let cx = TestAppContext::single();
        let focus = || cx.update(|cx| cx.focus_handle());
        SettingsFeatureState::new(
            AppSettingsSummary::default(),
            KeywordHighlightConfig::default(),
            String::new(),
            String::new(),
            true,
            vec!["Inter".to_string()],
            vec!["JetBrains Mono".to_string()],
            SettingsFeatureFocus {
                search_engine: focus(),
                keyword_highlight: focus(),
                keybindings: focus(),
            },
        )
    }

    #[test]
    fn settings_owner_normalizes_general_and_interaction_transitions() {
        let mut state = settings_state();
        state.summary.diagnostics_level = "info".to_string();
        state.summary.diagnostics_retention_days = 7;
        state.summary.interaction_command_suggestion_min_chars = 5;
        state.summary.interaction_command_suggestion_max_chars = 10;
        state.summary.interaction_duplicate_session_command_delay_ms = 100;

        assert!(state.set_diagnostics_level("debug"));
        assert!(!state.set_diagnostics_level("debug"));
        assert!(state.set_diagnostics_level("trace"));
        assert_eq!(state.summary().diagnostics_level, "info");
        assert!(state.set_diagnostics_retention_days(30));
        assert!(state.set_diagnostics_retention_days(99));
        assert_eq!(state.summary().diagnostics_retention_days, 7);

        state.adjust_command_suggestion_min_chars(50);
        assert_eq!(state.summary().interaction_command_suggestion_min_chars, 10);
        state.adjust_command_suggestion_max_chars(-50);
        assert_eq!(state.summary().interaction_command_suggestion_max_chars, 10);
        state.adjust_duplicate_session_command_delay(-500);
        assert_eq!(
            state
                .summary()
                .interaction_duplicate_session_command_delay_ms,
            0
        );

        let restore_layout = state.summary().startup_restore_window_layout;
        assert_eq!(
            state.toggle_startup_restore_window_layout(),
            !restore_layout
        );
        let suggestions_enabled = state.summary().interaction_command_suggestions_enabled;
        assert_eq!(state.toggle_command_suggestions(), !suggestions_enabled);
    }

    #[test]
    fn settings_owner_clamps_terminal_and_remote_transitions() {
        let mut state = settings_state();
        state.summary.terminal_scrollback_lines = 100;
        state.summary.terminal_keep_alive_interval = 0;
        state.summary.ui_remote_stats_interval = 1;
        state.summary.ui_process_manager_interval = 3;
        state.summary.ui_docker_manager_interval = 120;

        state.adjust_terminal_scrollback_lines(-1000);
        state.adjust_terminal_keep_alive_interval(-10);
        state.adjust_remote_stats_interval(-5);
        state.adjust_process_manager_interval(-5);
        state.adjust_docker_manager_interval(5);

        let summary = state.summary();
        assert_eq!(summary.terminal_scrollback_lines, 100);
        assert_eq!(summary.terminal_keep_alive_interval, 0);
        assert_eq!(summary.ui_remote_stats_interval, 1);
        assert_eq!(summary.ui_process_manager_interval, 3);
        assert_eq!(summary.ui_docker_manager_interval, 120);

        let low_latency = summary.terminal_low_latency_mode;
        assert_eq!(state.toggle_terminal_low_latency_mode(), !low_latency);
        assert!(state.toggle_terminal_action_link_matcher("ipv4"));
        assert!(!state.toggle_terminal_action_link_matcher("unknown"));
    }

    #[test]
    fn settings_owner_clamps_recording_and_transfer_transitions() {
        let mut state = settings_state();
        state.summary.recording_memory_limit_bytes = 1024 * 1024;
        state.summary.transfer_download_threads = 1;
        state.summary.transfer_upload_threads = 10;
        state.summary.transfer_max_retries = 0;
        state.summary.transfer_buffer_size = 8;

        state.adjust_recording_memory_limit(-1);
        assert_eq!(state.summary().recording_memory_limit_bytes, 1024 * 1024);
        state.adjust_recording_memory_limit(1_000);
        assert_eq!(
            state.summary().recording_memory_limit_bytes,
            512 * 1024 * 1024
        );

        state.adjust_transfer_download_threads(-1);
        state.adjust_transfer_upload_threads(1);
        state.adjust_transfer_max_retries(-1);
        state.adjust_transfer_buffer_size(-1);
        let summary = state.summary();
        assert_eq!(summary.transfer_download_threads, 1);
        assert_eq!(summary.transfer_upload_threads, 10);
        assert_eq!(summary.transfer_max_retries, 0);
        assert_eq!(summary.transfer_buffer_size, 8);

        state.set_transfer_duplicate_strategy("rename".to_string());
        state.set_transfer_editor_type("internal");
        state.set_transfer_file_permissions("640");
        let summary = state.summary();
        assert_eq!(summary.transfer_duplicate_strategy, "rename");
        assert_eq!(summary.transfer_editor_type, "internal");
        assert_eq!(summary.transfer_default_file_permissions, "640");
    }

    #[test]
    fn settings_owner_keeps_search_engine_rows_and_menus_consistent() {
        let mut state = settings_state();
        for name in ["one", "two", "three"] {
            state
                .summary
                .search_custom_engines
                .push(SearchEngineConfig {
                    name: name.to_string(),
                    url_template: format!("https://{name}.example/?q=%s"),
                    icon: None,
                    show_in_menu: true,
                });
        }

        assert_eq!(state.toggle_search_engine_expanded(2), Some(false));
        assert!(state.toggle_search_engine_menu(SearchEngineMenu::Icon, 2));
        assert!(state.toggle_search_engine_menu(SearchEngineMenu::Actions, 2));
        let interaction = state.search_engine_presentation();
        assert_eq!(interaction.expanded_index, Some(2));
        assert_eq!(interaction.icon_picker_index, None);
        assert_eq!(interaction.actions_index, Some(2));

        assert!(state.remove_search_engine(0));
        assert_eq!(state.search_engine_presentation().expanded_index, Some(1));
        assert!(state.remove_search_engine(1));
        assert_eq!(state.search_engine_presentation().expanded_index, None);
    }

    #[test]
    fn settings_owner_reconciles_keyword_highlight_edit_lifecycle() {
        let mut state = settings_state();
        state.add_keyword_highlight_rule(KeywordHighlightRule {
            id: "first".to_string(),
            name: "first".to_string(),
            patterns: Vec::new(),
            color_dark: "#ffffff".to_string(),
            color_light: "#000000".to_string(),
            enabled: true,
        });
        state.begin_keyword_highlight_edit(
            "first".to_string(),
            KeywordHighlightEditorField::Patterns,
        );
        let interaction = state.keyword_highlight_presentation();
        assert_eq!(interaction.expanded_id.as_deref(), Some("first"));
        assert_eq!(interaction.edit_id.as_deref(), Some("first"));
        assert_eq!(
            interaction.edit_field,
            KeywordHighlightEditorField::Patterns
        );

        assert_eq!(
            state.toggle_keyword_highlight_expanded("second".to_string()),
            vec!["first".to_string()]
        );
        let interaction = state.keyword_highlight_presentation();
        assert_eq!(interaction.expanded_id.as_deref(), Some("second"));
        assert_eq!(interaction.edit_id, None);
    }

    #[test]
    fn settings_owner_admits_and_finishes_prompts_by_identity() {
        let mut state = settings_state();
        assert!(state.begin_config_path_prompt(ConfigPathPromptKind::Export));
        assert!(!state.begin_config_path_prompt(ConfigPathPromptKind::PortableImport));
        assert!(!state.finish_config_path_prompt(ConfigPathPromptKind::PortableImport));
        assert!(state.config_path_prompt_active());
        assert!(state.finish_config_path_prompt(ConfigPathPromptKind::Export));

        assert!(state.begin_snapshot_password_prompt(SnapshotPasswordPromptKind::CloudForcePush));
        assert!(state.apply_snapshot_password_input("secret".to_string()));
        let prompt = state.take_snapshot_password_prompt().expect("prompt");
        assert_eq!(prompt.kind, SnapshotPasswordPromptKind::CloudForcePush);
        assert_eq!(prompt.value, "secret");
        assert!(!state.snapshot_password_prompt_active());
    }

    #[test]
    fn settings_owner_keeps_keybinding_recording_and_search_atomic() {
        let mut state = settings_state();
        state.begin_keybinding_recording("terminal.copy".to_string());
        state.set_pending_keybinding(Some("ctrl-shift-c".to_string()));
        state.set_keybinding_search("copy".to_string());
        let interaction = state.keybinding_presentation();
        assert_eq!(interaction.recording_id.as_deref(), Some("terminal.copy"));
        assert_eq!(interaction.pending_keys.as_deref(), Some("ctrl-shift-c"));
        assert_eq!(interaction.search_draft, "copy");

        state.cancel_keybinding_recording();
        state.clear_keybinding_search();
        let interaction = state.keybinding_presentation();
        assert_eq!(interaction.recording_id, None);
        assert_eq!(interaction.pending_keys, None);
        assert!(interaction.search_draft.is_empty());
    }

    #[test]
    fn settings_owner_controls_store_status_updates_and_replacement() {
        let mut state = settings_state();

        state.set_store_message("saving settings");
        state.set_store_ready(false);
        let status = state.store_status();
        assert_eq!(status.path, "");
        assert_eq!(status.message, "saving settings");
        assert!(!status.ready);

        state.replace_store_status(
            "/tmp/nyaterm.redb".to_string(),
            "store reopened".to_string(),
            true,
        );
        let status = state.store_status();
        assert_eq!(status.path, "/tmp/nyaterm.redb");
        assert_eq!(status.message, "store reopened");
        assert!(status.ready);
    }

    #[test]
    fn settings_owner_applies_ui_layout_as_one_transition() {
        let mut state = settings_state();

        state.apply_ui_layout(UiLayoutSettingsUpdate {
            left_panel_width: 320,
            right_panel_width: 360,
            transfer_height: 240,
            quick_command_height: 180,
            quick_command_visible: false,
            serial_send_height: 160,
            serial_send_visible: true,
            active_left_panel: Some("connections".to_string()),
            active_right_panel: Some("sftp".to_string()),
            left_panel_collapsed: false,
            right_panel_collapsed: true,
            saved_connections_sort_mode: "recent".to_string(),
            activity_bar_left_top: vec!["connections".to_string()],
            activity_bar_left_bottom: vec!["settings".to_string()],
            activity_bar_right_top: vec!["sftp".to_string()],
            activity_bar_right_bottom: vec!["ai".to_string()],
            activity_bar_show_labels: true,
            panel_multi_open: true,
            left_open_panels: vec!["connections".to_string()],
            right_open_panels: vec!["sftp".to_string()],
            panel_stack_sizes: HashMap::from([("left:connections".to_string(), 600)]),
        });

        let summary = state.summary();
        assert_eq!(summary.ui_left_panel_width, 320);
        assert_eq!(summary.ui_right_panel_width, 360);
        assert!(!summary.ui_quick_cmd_visible);
        assert!(summary.ui_serial_send_visible);
        assert_eq!(summary.ui_active_left_panel.as_deref(), Some("connections"));
        assert!(summary.ui_right_panel_collapsed);
        assert_eq!(summary.ui_activity_bar_right_bottom, ["ai"]);
        assert_eq!(summary.ui_panel_stack_sizes["left:connections"], 600);
    }

    #[test]
    fn settings_owner_controls_keyword_catalog_transitions() {
        let mut state = settings_state();
        state.add_keyword_highlight_rule(KeywordHighlightRule {
            id: "warning".to_string(),
            name: "warning".to_string(),
            patterns: vec!["WARN".to_string()],
            color_dark: "#ffffff".to_string(),
            color_light: "#000000".to_string(),
            enabled: true,
        });

        assert!(state.toggle_keyword_highlights());
        assert!(state.toggle_keyword_highlights_wrapped());
        state.toggle_keyword_highlight_builtin("builtin-error".to_string());
        assert!(state.toggle_keyword_highlight_rule("warning"));
        assert!(state.set_keyword_highlight_rule_color("warning", true, "#123456".to_string()));
        assert!(state.apply_keyword_highlight_rule_input(
            "warning",
            KeywordHighlightEditorField::Patterns,
            "WARN\nERROR".to_string(),
        ));

        let config = state.keyword_config();
        assert!(config.enabled);
        assert!(config.across_wrapped_lines);
        assert!(!config.builtin_rules["builtin-error"]);
        assert!(!config.rules[0].enabled);
        assert_eq!(config.rules[0].color_dark, "#123456");
        assert_eq!(config.rules[0].patterns, ["WARN", "ERROR"]);
    }

    #[test]
    fn settings_owner_exposes_master_password_as_a_borrowed_view() {
        let mut state = settings_state();
        assert_eq!(state.master_password.toggle(false), Ok(true));
        assert!(
            state
                .master_password
                .edit_draft("replacement secret".to_string())
        );

        let view = state.master_password();
        assert!(view.enabled);
        assert_eq!(view.draft, "replacement secret");

        state.summary.has_master_password = false;
        state.rebase_master_password();
        let view = state.master_password();
        assert!(!view.enabled);
        assert!(view.draft.is_empty());
    }
}
