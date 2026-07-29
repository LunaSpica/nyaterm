//! Authoritative command catalog, history, runtime and quick-command UI state.

use std::path::PathBuf;
use std::sync::Arc;

use gpui::{FocusHandle, WindowHandle};
use nyaterm_core::{CommandHistoryEntry, QuickCommand, QuickCommandCategory};

use super::quick_command_runtime::QuickCommandWindow;
use crate::features::{CommandPersistenceRequest, CommandPersistenceResult};
use crate::models::{
    QuickCommandCategoryDeleteState, QuickCommandCategoryMenuState,
    QuickCommandCategoryRenameState, QuickCommandDeleteState, QuickCommandDetailsState,
    QuickCommandEditorField, QuickCommandEditorState, QuickCommandImportPathPromptKind,
    QuickCommandRowMenuState, QuickCommandSortMode, QuickCommandVariablePromptState,
    QuickCommandViewMode,
};

use super::runtime_state::{CommandPersistencePoll, CommandRuntimeState};

pub(in crate::features) struct CommandFeatureState {
    catalog: CommandCatalogState,
    quick: QuickCommandFeatureState,
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

struct QuickCommandFeatureState {
    list: QuickCommandListState,
    editor: QuickCommandEditorFeatureState,
    dialogs: QuickCommandDialogState,
    import: QuickCommandImportState,
    ai: QuickCommandAiState,
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

    pub(in crate::features) fn quick_search_draft(&self) -> &str {
        &self.quick.list.search_draft
    }

    pub(in crate::features) fn quick_selected_category(&self) -> &str {
        &self.quick.list.selected_category
    }

    pub(in crate::features) fn quick_sort_mode(&self) -> QuickCommandSortMode {
        self.quick.list.sort_mode
    }

    pub(in crate::features) fn quick_view_mode(&self) -> QuickCommandViewMode {
        self.quick.list.view_mode
    }

    pub(in crate::features) fn quick_sort_menu_is_open(&self) -> bool {
        self.quick.list.sort_menu_open
    }

    pub(in crate::features) fn quick_view_menu_is_open(&self) -> bool {
        self.quick.list.view_menu_open
    }

    pub(in crate::features) fn quick_row_menu(&self) -> Option<&QuickCommandRowMenuState> {
        self.quick.list.row_menu.as_ref()
    }

    pub(in crate::features) fn quick_category_menu(
        &self,
    ) -> Option<&QuickCommandCategoryMenuState> {
        self.quick.list.category_menu.as_ref()
    }

    pub(in crate::features) fn set_quick_search_draft(&mut self, text: String) {
        self.quick.list.search_draft = text;
    }

    pub(in crate::features) fn clear_quick_filters(&mut self) {
        self.quick.list.search_draft.clear();
        self.quick.list.selected_category = "all".to_string();
    }

    pub(in crate::features) fn select_quick_category(&mut self, category_id: String) {
        self.quick.list.selected_category = category_id;
        self.close_quick_menus();
    }

    pub(in crate::features) fn set_quick_view_mode(&mut self, mode: QuickCommandViewMode) {
        self.close_quick_row_menu();
        self.close_quick_toolbar_popovers();
        self.quick.list.view_mode = mode;
    }

    pub(in crate::features) fn set_quick_sort_mode(&mut self, mode: QuickCommandSortMode) {
        self.close_quick_row_menu();
        self.close_quick_toolbar_popovers();
        self.quick.list.sort_mode = mode;
    }

    pub(in crate::features) fn toggle_quick_sort_menu(&mut self) {
        let open = !self.quick.list.sort_menu_open;
        self.close_quick_toolbar_popovers();
        self.close_quick_row_menu();
        self.quick.list.sort_menu_open = open;
    }

    pub(in crate::features) fn toggle_quick_view_menu(&mut self) {
        let open = !self.quick.list.view_menu_open;
        self.close_quick_toolbar_popovers();
        self.close_quick_row_menu();
        self.quick.list.view_menu_open = open;
    }

    pub(in crate::features) fn close_quick_toolbar_popovers(&mut self) -> bool {
        let changed = self.quick.list.sort_menu_open
            || self.quick.list.view_menu_open
            || self.quick.list.category_menu.is_some()
            || self.quick.ai.popover_open;
        self.quick.close_toolbar_popovers();
        changed
    }

    pub(in crate::features) fn close_quick_row_menu(&mut self) {
        self.quick.list.row_menu = None;
    }

    pub(in crate::features) fn close_quick_menus(&mut self) {
        self.quick.list.row_menu = None;
        self.quick.list.category_menu = None;
    }

    pub(in crate::features) fn toggle_quick_row_menu(&mut self, menu: QuickCommandRowMenuState) {
        if self
            .quick
            .list
            .row_menu
            .as_ref()
            .is_some_and(|current| current.command_id == menu.command_id)
        {
            self.quick.list.row_menu = None;
        } else {
            self.quick.list.row_menu = Some(menu);
            self.quick.list.category_menu = None;
        }
    }

    pub(in crate::features) fn open_quick_row_menu(&mut self, menu: QuickCommandRowMenuState) {
        self.quick.list.row_menu = Some(menu);
        self.quick.list.category_menu = None;
    }

    pub(in crate::features) fn open_quick_category_menu(
        &mut self,
        menu: QuickCommandCategoryMenuState,
    ) {
        self.quick.list.row_menu = None;
        self.quick.list.category_menu = Some(menu);
    }

    pub(in crate::features) fn quick_editor(&self) -> Option<&QuickCommandEditorState> {
        self.quick.editor.draft.as_ref()
    }

    pub(in crate::features) fn quick_editor_snapshot(&self) -> Option<QuickCommandEditorState> {
        self.quick.editor.draft.clone()
    }

    pub(in crate::features) fn quick_editor_focus(&self) -> &FocusHandle {
        &self.quick.editor.focus
    }

    pub(in crate::features) fn open_quick_editor(&mut self, editor: QuickCommandEditorState) {
        self.close_quick_row_menu();
        self.quick.editor.draft = Some(editor);
    }

    pub(in crate::features) fn close_quick_editor(&mut self) {
        self.quick.editor.draft = None;
        self.quick.editor.window = None;
        self.quick.editor.window_open_pending = false;
    }

    pub(in crate::features) fn apply_quick_editor_input(
        &mut self,
        field: QuickCommandEditorField,
        text: String,
    ) -> bool {
        let Some(editor) = self.quick.editor.draft.as_mut() else {
            return false;
        };
        editor.focused_field = field;
        match field {
            QuickCommandEditorField::Label => editor.label = text,
            QuickCommandEditorField::Command => editor.command = text,
            QuickCommandEditorField::Description => editor.description = text,
            QuickCommandEditorField::Category => {
                editor.category_draft = text;
                editor.category_id = None;
            }
        }
        editor.error = None;
        true
    }

    pub(in crate::features) fn focus_quick_editor_field(
        &mut self,
        field: QuickCommandEditorField,
    ) -> bool {
        let Some(editor) = self.quick.editor.draft.as_mut() else {
            return false;
        };
        editor.focused_field = field;
        editor.error = None;
        true
    }

    pub(in crate::features) fn set_quick_editor_category(
        &mut self,
        category_id: Option<String>,
        category_draft: String,
    ) -> bool {
        let Some(editor) = self.quick.editor.draft.as_mut() else {
            return false;
        };
        editor.category_id = category_id;
        editor.category_draft = category_draft;
        editor.error = None;
        true
    }

    pub(in crate::features) fn set_quick_editor_color(
        &mut self,
        color_tag: Option<String>,
    ) -> bool {
        let Some(editor) = self.quick.editor.draft.as_mut() else {
            return false;
        };
        editor.color_tag = color_tag;
        editor.icon_tag = None;
        editor.error = None;
        true
    }

    pub(in crate::features) fn set_quick_editor_icon(&mut self, icon_tag: Option<String>) -> bool {
        let Some(editor) = self.quick.editor.draft.as_mut() else {
            return false;
        };
        editor.icon_tag = icon_tag;
        if editor.icon_tag.is_some() {
            editor.color_tag = None;
        }
        editor.error = None;
        true
    }

    pub(in crate::features) fn toggle_quick_editor_pinned(&mut self) -> bool {
        let Some(editor) = self.quick.editor.draft.as_mut() else {
            return false;
        };
        editor.pinned = !editor.pinned;
        editor.error = None;
        true
    }

    pub(in crate::features) fn set_quick_editor_execution_mode(&mut self, mode: &str) -> bool {
        let Some(editor) = self.quick.editor.draft.as_mut() else {
            return false;
        };
        editor.execution_mode = if mode == "append" {
            "append"
        } else {
            "execute"
        }
        .to_string();
        editor.error = None;
        true
    }

    pub(in crate::features) fn set_quick_editor_error(
        &mut self,
        error: String,
        field: Option<QuickCommandEditorField>,
    ) {
        if let Some(editor) = self.quick.editor.draft.as_mut() {
            editor.error = Some(error);
            if let Some(field) = field {
                editor.focused_field = field;
            }
        }
    }

    pub(in crate::features::commands) fn quick_editor_window(
        &self,
    ) -> Option<WindowHandle<QuickCommandWindow>> {
        self.quick.editor.window
    }

    pub(in crate::features) fn quick_editor_window_is_open(&self) -> bool {
        self.quick.editor.window.is_some()
    }

    pub(in crate::features) fn quick_editor_window_is_pending(&self) -> bool {
        self.quick.editor.window_open_pending
    }

    pub(in crate::features) fn quick_editor_is_inline(&self) -> bool {
        self.quick.editor.draft.is_some()
            && self.quick.editor.window.is_none()
            && !self.quick.editor.window_open_pending
    }

    pub(in crate::features) fn quick_editor_window_is_open_or_pending(&self) -> bool {
        self.quick.editor.window.is_some() || self.quick.editor.window_open_pending
    }

    pub(in crate::features) fn request_quick_editor_window(&mut self) -> bool {
        if self.quick.editor.draft.is_none()
            || self.quick.editor.window.is_some()
            || self.quick.editor.window_open_pending
        {
            return false;
        }
        self.quick.editor.window_open_pending = true;
        true
    }

    pub(in crate::features::commands) fn finish_quick_editor_window_open(
        &mut self,
        window: Option<WindowHandle<QuickCommandWindow>>,
    ) {
        self.quick.editor.window = window;
        self.quick.editor.window_open_pending = false;
    }

    pub(in crate::features::commands) fn clear_quick_editor_window_if(
        &mut self,
        expected: WindowHandle<QuickCommandWindow>,
    ) -> bool {
        if self.quick.editor.window == Some(expected) {
            self.quick.editor.window = None;
            return true;
        }
        false
    }

    pub(in crate::features) fn cancel_quick_editor_window_request(&mut self) {
        self.quick.editor.window_open_pending = false;
    }

    pub(in crate::features) fn quick_delete(&self) -> Option<&QuickCommandDeleteState> {
        self.quick.dialogs.delete.as_ref()
    }

    pub(in crate::features) fn request_quick_delete(&mut self, state: QuickCommandDeleteState) {
        self.close_quick_row_menu();
        self.quick.dialogs.delete = Some(state);
    }

    pub(in crate::features) fn clear_quick_delete(&mut self) {
        self.quick.dialogs.delete = None;
    }

    pub(in crate::features) fn quick_details(&self) -> Option<&QuickCommandDetailsState> {
        self.quick.dialogs.details.as_ref()
    }

    pub(in crate::features) fn quick_details_focus(&self) -> &FocusHandle {
        &self.quick.dialogs.details_focus
    }

    pub(in crate::features) fn request_quick_details(&mut self, state: QuickCommandDetailsState) {
        self.close_quick_row_menu();
        self.quick.dialogs.details = Some(state);
    }

    pub(in crate::features) fn clear_quick_details(&mut self) {
        self.quick.dialogs.details = None;
    }

    pub(in crate::features) fn quick_category_delete(
        &self,
    ) -> Option<&QuickCommandCategoryDeleteState> {
        self.quick.dialogs.category_delete.as_ref()
    }

    pub(in crate::features) fn request_quick_category_delete(
        &mut self,
        state: QuickCommandCategoryDeleteState,
    ) {
        self.close_quick_menus();
        self.quick.dialogs.category_delete = Some(state);
    }

    pub(in crate::features) fn clear_quick_category_delete(&mut self) {
        self.quick.dialogs.category_delete = None;
    }

    pub(in crate::features) fn finish_quick_category_delete(&mut self, category_id: &str) {
        self.quick.dialogs.category_delete = None;
        if self.quick.list.selected_category == category_id {
            self.quick.list.selected_category = "all".to_string();
        }
        if let Some(editor) = self.quick.editor.draft.as_mut()
            && editor.category_id.as_deref() == Some(category_id)
        {
            editor.category_id = None;
            editor.category_draft.clear();
        }
    }

    pub(in crate::features) fn quick_category_rename(
        &self,
    ) -> Option<&QuickCommandCategoryRenameState> {
        self.quick.dialogs.category_rename.as_ref()
    }

    pub(in crate::features) fn quick_category_rename_focus(&self) -> &FocusHandle {
        &self.quick.dialogs.category_rename_focus
    }

    pub(in crate::features) fn request_quick_category_rename(
        &mut self,
        state: QuickCommandCategoryRenameState,
    ) {
        self.close_quick_menus();
        self.quick.dialogs.category_rename = Some(state);
    }

    pub(in crate::features) fn clear_quick_category_rename(&mut self) {
        self.quick.dialogs.category_rename = None;
    }

    pub(in crate::features) fn apply_quick_category_rename(&mut self, text: String) -> bool {
        let Some(rename) = self.quick.dialogs.category_rename.as_mut() else {
            return false;
        };
        rename.draft = text;
        rename.error = None;
        true
    }

    pub(in crate::features) fn set_quick_category_rename_error(&mut self, error: String) {
        if let Some(rename) = self.quick.dialogs.category_rename.as_mut() {
            rename.error = Some(error);
        }
    }

    pub(in crate::features) fn quick_variable_prompt(
        &self,
    ) -> Option<&QuickCommandVariablePromptState> {
        self.quick.dialogs.variable_prompt.as_ref()
    }

    pub(in crate::features) fn quick_variable_focus(&self) -> &FocusHandle {
        &self.quick.dialogs.variable_focus
    }

    pub(in crate::features) fn request_quick_variable_prompt(
        &mut self,
        prompt: QuickCommandVariablePromptState,
    ) {
        self.quick.dialogs.variable_prompt = Some(prompt);
    }

    pub(in crate::features) fn take_quick_variable_prompt(
        &mut self,
    ) -> Option<QuickCommandVariablePromptState> {
        self.quick.dialogs.variable_prompt.take()
    }

    pub(in crate::features) fn clear_quick_variable_prompt(&mut self) {
        self.quick.dialogs.variable_prompt = None;
    }

    pub(in crate::features) fn focus_quick_variable(&mut self, index: usize) -> bool {
        let Some(prompt) = self.quick.dialogs.variable_prompt.as_mut() else {
            return false;
        };
        if index >= prompt.variables.len() {
            return false;
        }
        prompt.focused_index = index;
        true
    }

    pub(in crate::features) fn set_quick_variable_value(
        &mut self,
        index: usize,
        value: String,
    ) -> bool {
        let Some(prompt) = self.quick.dialogs.variable_prompt.as_mut() else {
            return false;
        };
        let Some(variable) = prompt.variables.get(index) else {
            return false;
        };
        let name = variable.name.clone();
        for variable in &mut prompt.variables {
            if variable.name == name {
                variable.value = value.clone();
            }
        }
        prompt.focused_index = index;
        true
    }

    pub(in crate::features) fn cycle_quick_variable_option(
        &mut self,
        index: usize,
        delta: isize,
    ) -> bool {
        let Some(prompt) = self.quick.dialogs.variable_prompt.as_mut() else {
            return false;
        };
        let Some(variable) = prompt.variables.get(index) else {
            return false;
        };
        if variable.options.is_empty() {
            return false;
        }
        let current = variable
            .options
            .iter()
            .position(|option| option == &variable.value)
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(variable.options.len() as isize) as usize;
        let value = variable.options[next].clone();
        let name = variable.name.clone();
        for variable in &mut prompt.variables {
            if variable.name == name {
                variable.value = value.clone();
            }
        }
        prompt.focused_index = index;
        true
    }

    pub(in crate::features) fn quick_import_dialog_is_open(&self) -> bool {
        self.quick.import.dialog_open
    }

    pub(in crate::features) fn quick_import_path_prompt(
        &self,
    ) -> Option<QuickCommandImportPathPromptKind> {
        self.quick.import.path_prompt
    }

    pub(in crate::features) fn quick_import_focus(&self) -> &FocusHandle {
        &self.quick.import.focus
    }

    pub(in crate::features) fn open_quick_import_dialog(&mut self) -> bool {
        if self.quick.import.path_prompt.is_some() {
            return false;
        }
        self.quick.import.dialog_open = true;
        true
    }

    pub(in crate::features) fn close_quick_import_dialog(&mut self) {
        self.quick.import.dialog_open = false;
    }

    pub(in crate::features) fn request_quick_import_path(
        &mut self,
        kind: QuickCommandImportPathPromptKind,
    ) -> bool {
        if self.quick.import.path_prompt.is_some() {
            return false;
        }
        self.quick.import.dialog_open = false;
        self.quick.import.path_prompt = Some(kind);
        true
    }

    pub(in crate::features) fn finish_quick_import_path(&mut self) {
        self.quick.import.path_prompt = None;
    }

    pub(in crate::features) fn quick_ai_popover_is_open(&self) -> bool {
        self.quick.ai.popover_open
    }

    pub(in crate::features) fn quick_ai_prompt_draft(&self) -> &str {
        &self.quick.ai.prompt_draft
    }

    pub(in crate::features) fn toggle_quick_ai_popover(&mut self) -> bool {
        let open = !self.quick.ai.popover_open;
        self.close_quick_toolbar_popovers();
        self.close_quick_row_menu();
        self.quick.ai.popover_open = open;
        open
    }

    pub(in crate::features) fn close_quick_ai_popover(&mut self) {
        self.quick.ai.popover_open = false;
    }

    pub(in crate::features) fn set_quick_ai_prompt_draft(&mut self, text: String) {
        self.quick.ai.prompt_draft = text;
    }

    pub(in crate::features) fn take_quick_ai_prompt(&mut self) -> Option<String> {
        let prompt = self.quick.ai.prompt_draft.trim().to_string();
        if prompt.is_empty() {
            return None;
        }
        self.quick.ai.prompt_draft.clear();
        Some(prompt)
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
struct QuickCommandListState {
    search_draft: String,
    selected_category: String,
    sort_mode: QuickCommandSortMode,
    view_mode: QuickCommandViewMode,
    sort_menu_open: bool,
    view_menu_open: bool,
    /// Open row overflow/context menu.
    row_menu: Option<QuickCommandRowMenuState>,
    /// Open category context menu.
    category_menu: Option<QuickCommandCategoryMenuState>,
}

/// Quick command editor draft and its optional detached window.
struct QuickCommandEditorFeatureState {
    draft: Option<QuickCommandEditorState>,
    focus: FocusHandle,
    window: Option<WindowHandle<QuickCommandWindow>>,
    window_open_pending: bool,
}

/// Delete/details/rename confirmations and the variable prompt.
struct QuickCommandDialogState {
    delete: Option<QuickCommandDeleteState>,
    details: Option<QuickCommandDetailsState>,
    details_focus: FocusHandle,
    category_delete: Option<QuickCommandCategoryDeleteState>,
    category_rename: Option<QuickCommandCategoryRenameState>,
    category_rename_focus: FocusHandle,
    variable_prompt: Option<QuickCommandVariablePromptState>,
    variable_focus: FocusHandle,
}

/// Import source picker and its path prompt.
struct QuickCommandImportState {
    dialog_open: bool,
    path_prompt: Option<QuickCommandImportPathPromptKind>,
    focus: FocusHandle,
}

/// AI-assisted quick command popover.
struct QuickCommandAiState {
    popover_open: bool,
    prompt_draft: String,
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
    use std::path::PathBuf;

    use gpui::{TestAppContext, px};
    use nyaterm_core::{QuickCommand, QuickCommandCategory};

    use crate::models::{
        QuickCommandCategoryMenuState, QuickCommandEditorState, QuickCommandImportPathPromptKind,
        QuickCommandRowMenuState, QuickCommandSortMode, QuickCommandVariableDef,
        QuickCommandVariablePromptState, QuickCommandViewMode,
    };

    use super::{
        CommandCatalogState, CommandFeatureInit, CommandFeatureState, QuickCommandFeatureFocus,
    };

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

    fn command_state() -> CommandFeatureState {
        let cx = TestAppContext::single();
        let focus = || cx.update(|cx| cx.focus_handle());
        CommandFeatureState::new(CommandFeatureInit {
            commands: Vec::new(),
            categories: Vec::new(),
            history: Vec::new(),
            sort_mode: QuickCommandSortMode::Usage,
            view_mode: QuickCommandViewMode::List,
            focus: QuickCommandFeatureFocus {
                editor: focus(),
                details: focus(),
                category_rename: focus(),
                variable: focus(),
                import: focus(),
            },
            config_dir: PathBuf::new(),
            portable_key_path: None,
        })
    }

    #[test]
    fn quick_toolbar_popovers_and_context_menus_are_exclusive() {
        let mut state = command_state();

        state.toggle_quick_sort_menu();
        assert!(state.quick_sort_menu_is_open());
        state.toggle_quick_ai_popover();
        assert!(!state.quick_sort_menu_is_open());
        assert!(state.quick_ai_popover_is_open());

        state.open_quick_category_menu(QuickCommandCategoryMenuState {
            category_id: "category-1".to_string(),
            x: px(10.),
            y: px(12.),
        });
        assert!(state.quick_category_menu().is_some());
        assert!(state.quick_row_menu().is_none());
        state.open_quick_row_menu(QuickCommandRowMenuState {
            command_id: "command-1".to_string(),
            x: px(14.),
            y: px(16.),
        });
        assert!(state.quick_category_menu().is_none());
        assert!(state.quick_row_menu().is_some());
    }

    #[test]
    fn category_deletion_clears_filter_and_matching_editor_category() {
        let mut state = command_state();
        state.open_quick_editor(QuickCommandEditorState::blank());
        assert!(state.set_quick_editor_category(Some("category-1".to_string()), String::new(),));
        state.select_quick_category("category-1".to_string());
        state.request_quick_category_delete(crate::models::QuickCommandCategoryDeleteState {
            id: "category-1".to_string(),
            name: "Common".to_string(),
            command_count: 1,
        });

        state.finish_quick_category_delete("category-1");

        assert_eq!(state.quick_selected_category(), "all");
        assert_eq!(
            state
                .quick_editor()
                .and_then(|editor| editor.category_id.as_deref()),
            None
        );
        assert!(state.quick_category_delete().is_none());
    }

    #[test]
    fn variable_values_are_synchronized_by_name_at_the_owner_boundary() {
        let mut state = command_state();
        state.request_quick_variable_prompt(QuickCommandVariablePromptState {
            command_id: "command-1".to_string(),
            label: "Command".to_string(),
            command: "{{host}} {{host}}".to_string(),
            execute: true,
            send_to_all: false,
            variables: vec![
                QuickCommandVariableDef {
                    raw: "{{host}}".to_string(),
                    name: "host".to_string(),
                    options: Vec::new(),
                    value: String::new(),
                },
                QuickCommandVariableDef {
                    raw: "{{host}}".to_string(),
                    name: "host".to_string(),
                    options: Vec::new(),
                    value: String::new(),
                },
            ],
            focused_index: 0,
        });

        assert!(state.set_quick_variable_value(1, "prod".to_string()));
        let prompt = state
            .quick_variable_prompt()
            .expect("prompt should remain open");
        assert_eq!(prompt.variables[0].value, "prod");
        assert_eq!(prompt.variables[1].value, "prod");
    }

    #[test]
    fn import_and_detached_editor_lifecycles_clear_pending_state_atomically() {
        let mut state = command_state();
        assert!(state.open_quick_import_dialog());
        assert!(state.request_quick_import_path(QuickCommandImportPathPromptKind::NyatermJson));
        assert!(!state.quick_import_dialog_is_open());
        assert!(!state.open_quick_import_dialog());
        state.finish_quick_import_path();
        assert!(state.open_quick_import_dialog());

        state.open_quick_editor(QuickCommandEditorState::blank());
        assert!(state.request_quick_editor_window());
        assert!(state.quick_editor_window_is_pending());
        state.close_quick_editor();
        assert!(state.quick_editor().is_none());
        assert!(!state.quick_editor_window_is_open_or_pending());
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
