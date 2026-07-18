use gpui::Pixels;
use nyaterm_core::{AiAction, AiContext, QuickCommand};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RightFocus {
    Default,
    Recording,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BottomPanelMode {
    QuickCommands,
    CommandSend,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuickCommandSortMode {
    Usage,
    Name,
    Created,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuickCommandViewMode {
    List,
    Compact,
    Tile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuickCommandEditorField {
    Label,
    Command,
    Category,
    Description,
}

#[derive(Debug, Clone)]
pub(crate) struct QuickCommandEditorState {
    pub(crate) original: Option<QuickCommand>,
    pub(crate) focused_field: QuickCommandEditorField,
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) category_id: Option<String>,
    pub(crate) category_draft: String,
    pub(crate) description: String,
    pub(crate) color_tag: Option<String>,
    pub(crate) icon_tag: Option<String>,
    pub(crate) pinned: bool,
    pub(crate) execution_mode: String,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct QuickCommandDeleteState {
    pub(crate) id: String,
    pub(crate) label: String,
}

#[derive(Debug, Clone)]
pub(crate) struct QuickCommandDetailsState {
    pub(crate) command: QuickCommand,
    pub(crate) category: String,
    pub(crate) x: Pixels,
    pub(crate) y: Pixels,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct QuickCommandRowMenuState {
    pub(crate) command_id: String,
    pub(crate) x: Pixels,
    pub(crate) y: Pixels,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct QuickCommandCategoryMenuState {
    pub(crate) category_id: String,
    pub(crate) x: Pixels,
    pub(crate) y: Pixels,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActiveSessionMenuState {
    pub(crate) session_id: String,
    pub(crate) x: Pixels,
    pub(crate) y: Pixels,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AiMessageMenuState {
    pub(crate) message_id: String,
    pub(crate) role_label: String,
    pub(crate) text: String,
    pub(crate) x: Pixels,
    pub(crate) y: Pixels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AiDetectedErrorState {
    pub(crate) session_id: String,
    pub(crate) output: String,
}

#[derive(Debug, Clone)]
pub(crate) struct QuickCommandCategoryDeleteState {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) command_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct QuickCommandCategoryRenameState {
    pub(crate) id: String,
    pub(crate) original_name: String,
    pub(crate) draft: String,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct QuickCommandVariableDef {
    pub(crate) raw: String,
    pub(crate) name: String,
    pub(crate) options: Vec<String>,
    pub(crate) value: String,
}

#[derive(Debug, Clone)]
pub(crate) struct QuickCommandVariablePromptState {
    pub(crate) command_id: String,
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) execute: bool,
    pub(crate) send_to_all: bool,
    pub(crate) variables: Vec<QuickCommandVariableDef>,
    pub(crate) focused_index: usize,
}

impl QuickCommandEditorState {
    pub(crate) fn blank() -> Self {
        Self {
            original: None,
            focused_field: QuickCommandEditorField::Label,
            label: String::new(),
            command: String::new(),
            category_id: None,
            category_draft: String::new(),
            description: String::new(),
            color_tag: None,
            icon_tag: None,
            pinned: false,
            execution_mode: "execute".to_string(),
            error: None,
        }
    }

    pub(crate) fn from_command(command: QuickCommand) -> Self {
        Self {
            focused_field: QuickCommandEditorField::Label,
            label: command.label.clone(),
            command: command.command.clone(),
            category_id: command.category_id.clone(),
            category_draft: String::new(),
            description: command.description.clone().unwrap_or_default(),
            color_tag: command.color_tag.clone(),
            icon_tag: command.icon_tag.clone(),
            pinned: command.pinned.unwrap_or_default(),
            execution_mode: command
                .execution_mode
                .clone()
                .unwrap_or_else(|| "execute".to_string()),
            error: None,
            original: Some(command),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AiPreparedRequest {
    pub(crate) action: AiAction,
    pub(crate) context: AiContext,
    pub(crate) source_label: String,
}
