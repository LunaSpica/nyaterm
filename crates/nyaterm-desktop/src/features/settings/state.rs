//! Grouped transient state for the settings experience.

use gpui::FocusHandle;

use crate::models::{KeywordHighlightEditorField, SearchEngineEditorField};

pub(in crate::features) struct SettingsFeatureState {
    pub search_engines: SearchEngineSettingsState,
    pub keyword_highlights: KeywordHighlightSettingsState,
    pub appearance: AppearanceSettingsState,
    pub keybindings: KeybindingSettingsState,
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
        ui_font_options: Vec<String>,
        terminal_font_options: Vec<String>,
        focus: SettingsFeatureFocus,
    ) -> Self {
        Self {
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
        }
    }
}
