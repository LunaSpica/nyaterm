//! Grouped quick command UI state.
//!
//! The persisted collections (`quick_commands`, `quick_command_categories`)
//! stay on `NyaTermApp`; everything here is transient panel, overlay and editor
//! state that only the quick command feature owns.

use gpui::{FocusHandle, WindowHandle};

use crate::features::QuickCommandWindow;
use crate::models::{
    QuickCommandCategoryDeleteState, QuickCommandCategoryMenuState,
    QuickCommandCategoryRenameState, QuickCommandDeleteState, QuickCommandDetailsState,
    QuickCommandEditorState, QuickCommandImportPathPromptKind, QuickCommandRowMenuState,
    QuickCommandSortMode, QuickCommandVariablePromptState, QuickCommandViewMode,
};

pub(in crate::features) struct QuickCommandFeatureState {
    pub list: QuickCommandListState,
    pub editor: QuickCommandEditorFeatureState,
    pub dialogs: QuickCommandDialogState,
    pub import: QuickCommandImportState,
    pub ai: QuickCommandAiState,
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
