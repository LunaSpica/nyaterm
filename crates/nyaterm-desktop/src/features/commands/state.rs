//! Authoritative command catalog, history, runtime and quick-command UI state.

use std::path::PathBuf;
use std::sync::Arc;

use gpui::{FocusHandle, WindowHandle};
use nyaterm_core::{CommandHistoryEntry, QuickCommand, QuickCommandCategory};

use crate::features::{CommandPersistenceRequest, CommandPersistenceResult, QuickCommandWindow};
use crate::models::{
    QuickCommandCategoryDeleteState, QuickCommandCategoryMenuState,
    QuickCommandCategoryRenameState, QuickCommandDeleteState, QuickCommandDetailsState,
    QuickCommandEditorState, QuickCommandImportPathPromptKind, QuickCommandRowMenuState,
    QuickCommandSortMode, QuickCommandVariablePromptState, QuickCommandViewMode,
};

use super::runtime_state::{CommandPersistencePoll, CommandRuntimeState};

pub(in crate::features) struct CommandFeatureState {
    catalog: CommandCatalogState,
    pub quick: QuickCommandFeatureState,
    history: Arc<[CommandHistoryEntry]>,
    runtime: CommandRuntimeState,
}

pub(in crate::features) struct CommandFeatureInit {
    pub commands: Vec<QuickCommand>,
    pub categories: Vec<QuickCommandCategory>,
    pub history: Vec<CommandHistoryEntry>,
    pub sort_mode: QuickCommandSortMode,
    pub view_mode: QuickCommandViewMode,
    pub focus: QuickCommandFeatureFocus,
    pub config_dir: PathBuf,
    pub portable_key_path: Option<PathBuf>,
}

struct CommandCatalogState {
    commands: Arc<[QuickCommand]>,
    categories: Vec<QuickCommandCategory>,
}

pub(in crate::features) struct QuickCommandFeatureState {
    pub list: QuickCommandListState,
    pub editor: QuickCommandEditorFeatureState,
    pub dialogs: QuickCommandDialogState,
    pub import: QuickCommandImportState,
    pub ai: QuickCommandAiState,
}

impl CommandFeatureState {
    pub(in crate::features) fn new(init: CommandFeatureInit) -> Self {
        Self {
            catalog: CommandCatalogState::new(init.commands, init.categories),
            quick: QuickCommandFeatureState::new(init.sort_mode, init.view_mode, init.focus),
            history: Arc::from(init.history),
            runtime: CommandRuntimeState::new(init.config_dir, init.portable_key_path),
        }
    }

    pub(in crate::features) fn replace_loaded(
        &mut self,
        commands: Vec<QuickCommand>,
        categories: Vec<QuickCommandCategory>,
        history: Vec<CommandHistoryEntry>,
    ) {
        self.catalog.replace(commands, categories);
        self.history = Arc::from(history);
    }

    pub(in crate::features) fn clear_loaded(&mut self) {
        self.catalog.clear();
        self.history = Arc::default();
    }

    pub(in crate::features) fn quick_commands(&self) -> &[QuickCommand] {
        &self.catalog.commands
    }

    pub(in crate::features) fn quick_commands_snapshot(&self) -> Arc<[QuickCommand]> {
        self.catalog.commands.clone()
    }

    pub(in crate::features) fn quick_command_categories(&self) -> &[QuickCommandCategory] {
        &self.catalog.categories
    }

    pub(in crate::features) fn command_history(&self) -> &[CommandHistoryEntry] {
        &self.history
    }

    pub(in crate::features) fn command_history_snapshot(&self) -> Arc<[CommandHistoryEntry]> {
        self.history.clone()
    }

    pub(in crate::features) fn replace_quick_command_catalog(
        &mut self,
        commands: Vec<QuickCommand>,
        categories: Vec<QuickCommandCategory>,
    ) {
        self.catalog.replace(commands, categories);
    }

    pub(in crate::features) fn replace_command_history(
        &mut self,
        history: Vec<CommandHistoryEntry>,
    ) {
        self.history = Arc::from(history);
    }

    pub(in crate::features) fn queue_command_history(&mut self, commands: Vec<String>) -> bool {
        self.runtime
            .queue(CommandPersistenceRequest::AppendHistory(commands))
    }

    pub(in crate::features) fn queue_quick_command_use_count(
        &mut self,
        command_id: String,
    ) -> bool {
        if !self
            .runtime
            .queue(CommandPersistenceRequest::IncrementQuickCommand(
                command_id.clone(),
            ))
        {
            return false;
        }
        self.catalog.increment_use_count(&command_id);
        true
    }

    pub(in crate::features) fn poll_persistence(&mut self) -> CommandPersistencePoll {
        self.runtime.poll()
    }

    pub(in crate::features) fn persistence_is_idle(&self) -> bool {
        self.runtime.is_idle()
    }

    pub(in crate::features) fn apply_persistence_result(
        &mut self,
        event: CommandPersistenceResult,
    ) -> Result<(), String> {
        match event {
            CommandPersistenceResult::History(Ok(history)) => {
                self.replace_command_history(history);
                Ok(())
            }
            CommandPersistenceResult::History(Err(error)) => {
                Err(format!("command history save failed: {error}"))
            }
            CommandPersistenceResult::QuickCommandUseCount { command_id, result } => result
                .map_err(|error| {
                    self.catalog.rollback_use_count(&command_id);
                    format!("quick command use count update failed: {error}")
                }),
        }
    }
}

impl CommandCatalogState {
    fn new(commands: Vec<QuickCommand>, categories: Vec<QuickCommandCategory>) -> Self {
        Self {
            commands: Arc::from(commands),
            categories,
        }
    }

    pub(in crate::features) fn replace(
        &mut self,
        commands: Vec<QuickCommand>,
        categories: Vec<QuickCommandCategory>,
    ) {
        self.commands = Arc::from(commands);
        self.categories = categories;
    }

    fn clear(&mut self) {
        self.commands = Arc::default();
        self.categories.clear();
    }

    fn increment_use_count(&mut self, command_id: &str) {
        if let Some(command) = Arc::make_mut(&mut self.commands)
            .iter_mut()
            .find(|command| command.id == command_id)
        {
            command.use_count = Some(command.use_count.unwrap_or_default().saturating_add(1));
        }
    }

    fn rollback_use_count(&mut self, command_id: &str) {
        if let Some(command) = Arc::make_mut(&mut self.commands)
            .iter_mut()
            .find(|command| command.id == command_id)
        {
            command.use_count = Some(command.use_count.unwrap_or_default().saturating_sub(1));
        }
    }
}

/// Focus handles the quick command state needs at construction time.
pub(in crate::features) struct QuickCommandFeatureFocus {
    pub editor: FocusHandle,
    pub details: FocusHandle,
    pub category_rename: FocusHandle,
    pub variable: FocusHandle,
    pub import: FocusHandle,
}

/// Panel list state: search, category filter, sort/view mode and their menus.
pub(in crate::features) struct QuickCommandListState {
    pub search_draft: String,
    pub selected_category: String,
    pub sort_mode: QuickCommandSortMode,
    pub view_mode: QuickCommandViewMode,
    pub sort_menu_open: bool,
    pub view_menu_open: bool,
    /// Open row overflow/context menu.
    pub row_menu: Option<QuickCommandRowMenuState>,
    /// Open category context menu.
    pub category_menu: Option<QuickCommandCategoryMenuState>,
}

/// Quick command editor draft and its optional detached window.
pub(in crate::features) struct QuickCommandEditorFeatureState {
    pub draft: Option<QuickCommandEditorState>,
    pub focus: FocusHandle,
    pub window: Option<WindowHandle<QuickCommandWindow>>,
    pub window_open_pending: bool,
}

/// Delete/details/rename confirmations and the variable prompt.
pub(in crate::features) struct QuickCommandDialogState {
    pub delete: Option<QuickCommandDeleteState>,
    pub details: Option<QuickCommandDetailsState>,
    pub details_focus: FocusHandle,
    pub category_delete: Option<QuickCommandCategoryDeleteState>,
    pub category_rename: Option<QuickCommandCategoryRenameState>,
    pub category_rename_focus: FocusHandle,
    pub variable_prompt: Option<QuickCommandVariablePromptState>,
    pub variable_focus: FocusHandle,
}

/// Import source picker and its path prompt.
pub(in crate::features) struct QuickCommandImportState {
    pub dialog_open: bool,
    pub path_prompt: Option<QuickCommandImportPathPromptKind>,
    pub focus: FocusHandle,
}

/// AI-assisted quick command popover.
pub(in crate::features) struct QuickCommandAiState {
    pub popover_open: bool,
    pub prompt_draft: String,
}

impl QuickCommandFeatureState {
    pub(in crate::features) fn new(
        sort_mode: QuickCommandSortMode,
        view_mode: QuickCommandViewMode,
        focus: QuickCommandFeatureFocus,
    ) -> Self {
        Self {
            list: QuickCommandListState {
                search_draft: String::new(),
                selected_category: "all".to_string(),
                sort_mode,
                view_mode,
                sort_menu_open: false,
                view_menu_open: false,
                row_menu: None,
                category_menu: None,
            },
            editor: QuickCommandEditorFeatureState {
                draft: None,
                focus: focus.editor,
                window: None,
                window_open_pending: false,
            },
            dialogs: QuickCommandDialogState {
                delete: None,
                details: None,
                details_focus: focus.details,
                category_delete: None,
                category_rename: None,
                category_rename_focus: focus.category_rename,
                variable_prompt: None,
                variable_focus: focus.variable,
            },
            import: QuickCommandImportState {
                dialog_open: false,
                path_prompt: None,
                focus: focus.import,
            },
            ai: QuickCommandAiState {
                popover_open: false,
                prompt_draft: String::new(),
            },
        }
    }
}

impl QuickCommandFeatureState {
    /// Closes every toolbar popover at once; they are mutually exclusive.
    pub(in crate::features) fn close_toolbar_popovers(&mut self) {
        self.list.sort_menu_open = false;
        self.list.view_menu_open = false;
        self.list.category_menu = None;
        self.ai.popover_open = false;
    }
}

#[cfg(test)]
mod tests {
    use nyaterm_core::{QuickCommand, QuickCommandCategory};

    use super::CommandCatalogState;

    fn command(id: &str) -> QuickCommand {
        QuickCommand {
            id: id.to_string(),
            label: id.to_string(),
            command: "pwd".to_string(),
            category_id: Some("category-1".to_string()),
            description: None,
            color_tag: None,
            icon_tag: None,
            pinned: None,
            execution_mode: None,
            source: None,
            risk_level: None,
            updated_at: None,
            created_at: None,
            use_count: None,
        }
    }

    #[test]
    fn command_catalog_replaces_and_clears_commands_with_categories() {
        let mut catalog = CommandCatalogState::new(Vec::new(), Vec::new());
        catalog.replace(
            vec![command("command-1")],
            vec![QuickCommandCategory {
                id: "category-1".to_string(),
                name: "Common".to_string(),
            }],
        );

        assert_eq!(catalog.commands.len(), 1);
        assert_eq!(catalog.categories.len(), 1);

        catalog.clear();
        assert!(catalog.commands.is_empty());
        assert!(catalog.categories.is_empty());
    }

    #[test]
    fn command_catalog_use_count_increment_can_be_rolled_back() {
        let mut catalog = CommandCatalogState::new(vec![command("command-1")], Vec::new());

        catalog.increment_use_count("command-1");
        assert_eq!(catalog.commands[0].use_count, Some(1));

        catalog.rollback_use_count("command-1");
        assert_eq!(catalog.commands[0].use_count, Some(0));
    }
}
