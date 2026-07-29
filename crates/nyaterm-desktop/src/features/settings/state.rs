//! Authoritative application settings and grouped state for the settings experience.

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
    pub summary: AppSettingsSummary,
    pub keyword_config: KeywordHighlightConfig,
    pub master_password: SettingsMasterPasswordState,
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

    pub fn rebase_master_password(&mut self) {
        self.master_password.reset(self.summary.has_master_password);
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

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use nyaterm_core::{
        AppSettingsSummary, KeywordHighlightConfig, KeywordHighlightRule, SearchEngineConfig,
    };

    use super::{SearchEngineMenu, SettingsFeatureFocus, SettingsFeatureState};
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
}
