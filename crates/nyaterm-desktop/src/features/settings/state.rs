//! Authoritative application settings and grouped state for the settings experience.

use gpui::FocusHandle;
use nyaterm_core::{AppSettingsSummary, KeywordHighlightConfig};

use crate::models::{
    ConfigPathPromptKind, DiagnosticsPathPromptKind, KeywordHighlightEditorField,
    KeywordHighlightPathPromptKind, SearchEngineEditorField, SnapshotPasswordPromptState,
};

use super::catalog::{SettingsMasterPasswordState, StoreStatus};

pub(in crate::features) struct SettingsFeatureState {
    /// Compatibility-sensitive values loaded and persisted through `nyaterm-core`.
    pub summary: AppSettingsSummary,
    pub keyword_config: KeywordHighlightConfig,
    pub master_password: SettingsMasterPasswordState,
    pub store_status: StoreStatus,
    pub search_engines: SearchEngineSettingsState,
    pub keyword_highlights: KeywordHighlightSettingsState,
    pub appearance: AppearanceSettingsState,
    pub keybindings: KeybindingSettingsState,
    pub prompts: SettingsPromptState,
}

#[derive(Default)]
pub(in crate::features) struct SettingsPromptState {
    pub config_path: Option<ConfigPathPromptKind>,
    pub diagnostics_path: Option<DiagnosticsPathPromptKind>,
    pub keyword_highlight_path: Option<KeywordHighlightPathPromptKind>,
    pub snapshot_password: Option<SnapshotPasswordPromptState>,
}

pub(in crate::features) struct SettingsFeatureFocus {
    pub search_engine: FocusHandle,
    pub keyword_highlight: FocusHandle,
    pub keybindings: FocusHandle,
}

pub(in crate::features) struct SearchEngineSettingsState {
    pub edit_index: Option<usize>,
    pub expanded_index: Option<usize>,
    pub icon_picker_index: Option<usize>,
    pub actions_index: Option<usize>,
    pub edit_field: SearchEngineEditorField,
    pub focus: FocusHandle,
}

pub(in crate::features) struct KeywordHighlightSettingsState {
    pub expanded_id: Option<String>,
    pub edit_id: Option<String>,
    pub edit_field: KeywordHighlightEditorField,
    pub focus: FocusHandle,
}

pub(in crate::features) struct AppearanceSettingsState {
    pub menu_open: Option<String>,
    pub ui_font_options: Vec<String>,
    pub terminal_font_options: Vec<String>,
}

pub(in crate::features) struct KeybindingSettingsState {
    pub recording_id: Option<String>,
    pub pending_keys: Option<String>,
    pub search_draft: String,
    pub focus: FocusHandle,
}

impl SettingsFeatureState {
    pub(in crate::features) fn new(
        summary: AppSettingsSummary,
        keyword_config: KeywordHighlightConfig,
        store_status: StoreStatus,
        ui_font_options: Vec<String>,
        terminal_font_options: Vec<String>,
        focus: SettingsFeatureFocus,
    ) -> Self {
        let master_password = SettingsMasterPasswordState::new(summary.has_master_password);
        Self {
            summary,
            keyword_config,
            master_password,
            store_status,
            search_engines: SearchEngineSettingsState {
                edit_index: None,
                expanded_index: None,
                icon_picker_index: None,
                actions_index: None,
                edit_field: SearchEngineEditorField::Name,
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

    pub fn rebase_master_password(&mut self) {
        self.master_password.reset(self.summary.has_master_password);
    }
}
