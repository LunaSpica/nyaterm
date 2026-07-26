mod prelude;
use prelude::*;

mod ai;
mod app_state;
mod commands;
mod connection_editor_window;
mod connections;
mod formatting;
mod inspector;
mod layout;
mod pages;
mod panels;
mod quick_command_window;
mod remote;
mod remote_editor_window;
mod remote_text_editor;
mod root;
mod runtime_jobs;
mod session;
mod settings;
mod settings_window;
mod shell;
mod sync;
mod sync_input;
mod terminal;
mod transfer_external_sync_window;
mod transfers;
mod translation;
mod tunnels;
mod view_widgets;

#[allow(unused_imports)]
pub(in crate::features) use crate::action_links::{
    ActionLinkAction, ActionLinkKind, ActionLinkMatch, actions_for_match, find_action_links,
    match_at_offset,
};
pub(in crate::features) use crate::theme::ThemePalette;
pub(in crate::features) use ai::{
    AiFeatureFocus, AiFeatureState, ai_active_profile_drafts, ai_usage_counts,
    is_agent_command_card,
};
pub use app_state::NyaTermApp;
pub(in crate::features) use app_state::{
    FailedSessionStart, PendingSavedConnectionStart, PendingSessionStart,
    SavedConnectionStartOptions, SessionPaneState,
};
pub(in crate::features) use commands::{
    QUICK_COMMAND_COLOR_OPTIONS, QUICK_COMMAND_ICON_OPTIONS, QuickCommandFeatureFocus,
    QuickCommandFeatureState, quick_command_category_label, quick_command_sort_mode_from_setting,
    quick_command_view_mode_from_setting, sorted_quick_commands,
};
pub(in crate::features) use connection_editor_window::ConnectionEditorWindow;
pub(in crate::features) use connections::{
    ConnectionDragKind, ConnectionDragPayload, ConnectionDragPreview, ConnectionDropPosition,
    ConnectionDropTarget, ConnectionEditorToggle, ConnectionFeatureFocus, ConnectionFeatureState,
};
pub(in crate::features) use formatting::*;
pub(in crate::features) use quick_command_window::QuickCommandWindow;
pub(in crate::features) use remote::{RemoteOpsFeatureFocus, RemoteOpsFeatureState};
pub(in crate::features) use remote_editor_window::RemoteFileEditorWindow;
pub(in crate::features) use remote_text_editor::RemoteTextEditor;
pub(in crate::features) use runtime_jobs::{
    ActivitySide, AiAgentBackgroundTarget, AiAgentLoopState, AiAgentStepStatus, AiAgentStepView,
    AiChatJobOutput, AiChatJobResult, AiChatWorkerEvent, AiDiscoveryJobResult,
    CommandPersistenceRequest, CommandPersistenceResult, DockerJobOutput, DockerJobResult,
    ProcessJobOutput, ProcessJobResult, SessionStartResult, SessionStartSuccess, StatsJobResult,
    TranslateJobResult, TunnelJobOutput, TunnelJobResult, UpdateJobResult,
    remote_job_event_matches, spawn_command_persistence_worker,
};
pub(in crate::features) use session::{
    CredentialPromptBroker, CredentialPromptState, HostKeyPromptBroker, HostKeyPromptChoice,
    HostKeyPromptIssue, HostKeyPromptRequest, KeyboardInteractivePromptState, NativeOtpProvider,
    SftpDuplicatePromptBroker, SftpDuplicatePromptState, credential_prompt_target,
    keyboard_interactive_prompt_target, unix_seconds_now,
};
pub(in crate::features) use settings::{SecurityFeatureFocus, SecurityFeatureState};
pub(in crate::features) use settings_window::SettingsWindow;
pub(in crate::features) use shell::{ActivityBarDragPayload, ActivityBarDragPreview};
pub(in crate::features) use shell::{
    ChromeTooltip, SessionTabDragPayload, SessionTabDragPreview, SessionTabTooltip,
    TAB_MOUSE_ACTIONS, TabMouseActionTarget,
};
pub(in crate::features) use shell::{
    appearance_font_options, appearance_font_stack, gpui_code_font_family,
};
pub(in crate::features) use terminal::{
    FULL_SHELL_PAINT_COUNT, TerminalSurface, full_shell_paint_count, terminal_surface_paint_count,
};
pub(in crate::features) use transfer_external_sync_window::TransferExternalSyncWindow;
pub(in crate::features) use transfers::{
    TransferFeatureFocus, TransferFeatureState, compact_transfer_job_row, duplicate_decision_label,
    duplicate_policy_label, format_file_size, transfer_input, transfer_job_title,
    transfer_status_label,
};
pub(in crate::features) use view_widgets::*;

const LEGACY_ROOT: &str = "./temp/nyaterm-tauri";
pub(crate) const INITIAL_TERMINAL_BANNER: &str = "$ nyaterm --native\nGPUI shell initialized.\nStart a local terminal or open a saved connection.\n";
const AI_AGENT_OBSERVATION_MIN_WAIT: Duration = Duration::from_millis(700);
const AI_AGENT_OBSERVATION_QUIET: Duration = Duration::from_millis(900);
const AI_AGENT_DEFAULT_STEP_TIMEOUT: Duration = Duration::from_millis(30_000);
const SESSION_COMMAND_HISTORY_LIMIT: usize = 128;
const DEFAULT_DUPLICATE_STARTUP_DELAY_MS: u64 = 500;
const SYNC_GROUP_COLORS: [u32; 8] = [
    0x3b82f6, 0xef4444, 0x22c55e, 0xf59e0b, 0x8b5cf6, 0xec4899, 0x06b6d4, 0xf97316,
];
const TAB_PRESET_COLORS: [(&str, u32); 11] = [
    ("Red", 0xef4444),
    ("Orange", 0xf97316),
    ("Amber", 0xf59e0b),
    ("Yellow", 0xeab308),
    ("Green", 0x22c55e),
    ("Emerald", 0x10b981),
    ("Cyan", 0x06b6d4),
    ("Blue", 0x3b82f6),
    ("Indigo", 0x6366f1),
    ("Purple", 0xa855f7),
    ("Pink", 0xec4899),
];
