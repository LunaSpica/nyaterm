mod prelude;
use prelude::*;

#[path = "shell/activity_bar_runtime.rs"]
mod activity_bar_runtime;
#[path = "ai/ai_agent_runtime.rs"]
mod ai_agent_runtime;
#[path = "ai/ai_jobs.rs"]
mod ai_jobs;
#[path = "ai/ai_runtime.rs"]
mod ai_runtime;
mod app_state;
#[path = "shell/appearance.rs"]
mod appearance;
#[path = "session/auth_runtime.rs"]
mod auth_runtime;
#[path = "sync/cloud_sync_provider.rs"]
mod cloud_sync_provider;
#[path = "sync/cloud_sync_runtime.rs"]
mod cloud_sync_runtime;
#[path = "commands/command_runtime.rs"]
mod command_runtime;
#[path = "settings/config_runtime.rs"]
mod config_runtime;
mod connection_editor_window;
#[path = "connections/connection_import_runtime.rs"]
mod connection_import_runtime;
#[path = "connections/connection_runtime.rs"]
mod connection_runtime;
#[path = "connections/connections.rs"]
mod connections;
#[path = "session/credential_autofill_runtime.rs"]
mod credential_autofill_runtime;
#[path = "shell/event_pump.rs"]
mod event_pump;
mod formatting;
#[path = "shell/global_shortcut_runtime.rs"]
mod global_shortcut_runtime;
#[path = "terminal/input_runtime.rs"]
mod input_runtime;
mod inspector;
#[path = "shell/keybinding_runtime.rs"]
mod keybinding_runtime;
mod layout;
#[path = "settings/lock_diagnostics_runtime.rs"]
mod lock_diagnostics_runtime;
#[path = "shell/navigation_runtime.rs"]
mod navigation_runtime;
mod pages;
#[path = "shell/panel_resize_runtime.rs"]
mod panel_resize_runtime;
#[path = "shell/panel_stack_runtime.rs"]
mod panel_stack_runtime;
mod panels;
#[path = "session/prompt_runtime.rs"]
mod prompt_runtime;
#[path = "commands/quick_command_runtime.rs"]
mod quick_command_runtime;
mod quick_command_window;
#[path = "shell/quick_switch_runtime.rs"]
mod quick_switch_runtime;
#[path = "session/recording_runtime.rs"]
mod recording_runtime;
mod remote_editor_window;
#[path = "remote/remote_runtime.rs"]
mod remote_runtime;
mod remote_text_editor;
mod root;
mod runtime_jobs;
#[path = "settings/security_runtime.rs"]
mod security_runtime;
#[path = "terminal/send_command_runtime.rs"]
mod send_command_runtime;
#[path = "session/session_dialog_runtime.rs"]
mod session_dialog_runtime;
#[path = "session/session_lifecycle.rs"]
mod session_lifecycle;
#[path = "session/session_order.rs"]
mod session_order;
#[path = "session/session_runtime.rs"]
mod session_runtime;
#[path = "session/session_state.rs"]
mod session_state;
#[path = "settings/settings_runtime.rs"]
mod settings_runtime;
mod settings_window;
#[path = "session/startup_restore_runtime.rs"]
mod startup_restore_runtime;
mod sync_input;
#[path = "shell/tab_mouse.rs"]
mod tab_mouse;
#[path = "shell/tab_windows_runtime.rs"]
mod tab_windows_runtime;
#[path = "session/temporary_ssh_link.rs"]
mod temporary_ssh_link;
#[path = "terminal/terminal_context_menu_runtime.rs"]
mod terminal_context_menu_runtime;
#[path = "terminal/terminal_runtime.rs"]
mod terminal_runtime;
#[path = "terminal/terminal_search_runtime.rs"]
mod terminal_search_runtime;
#[path = "terminal/terminal_selection_runtime.rs"]
mod terminal_selection_runtime;
pub(in crate::features) use terminal_selection_runtime::terminal_bounds_tracker;
#[path = "terminal/terminal_surface.rs"]
mod terminal_surface;
pub(in crate::features) use terminal_surface::terminal_snapshot_absolute_range;
#[path = "terminal/terminal_surface_entity.rs"]
mod terminal_surface_entity;
mod transfer_external_sync_window;
pub(in crate::features) use terminal_surface_entity::{
    FULL_SHELL_PAINT_COUNT, TerminalSurface, TerminalSurfaceHitTestScrollGeometry,
    full_shell_paint_count, terminal_effective_visual_scroll_offset_px,
    terminal_snapshot_anchor_row_for_display_offset, terminal_snapshot_covers_display_offset,
    terminal_surface_paint_count,
};
#[path = "transfers/transfer_events.rs"]
mod transfer_events;
#[path = "transfers/transfer_jobs.rs"]
mod transfer_jobs;
#[path = "transfers/transfer_options.rs"]
mod transfer_options;
#[path = "transfers/transfer_paths.rs"]
mod transfer_paths;
#[path = "transfers/transfer_widgets.rs"]
mod transfer_widgets;
#[path = "translation/translation_runtime.rs"]
mod translation_runtime;
#[path = "session/trzsz_runtime.rs"]
mod trzsz_runtime;
#[path = "tunnels/tunnel_runtime.rs"]
mod tunnel_runtime;
#[path = "settings/update_runtime.rs"]
mod update_runtime;
mod view_widgets;
#[path = "shell/workspace_runtime.rs"]
mod workspace_runtime;
#[path = "session/zmodem_runtime.rs"]
mod zmodem_runtime;

#[allow(unused_imports)]
pub(in crate::features) use crate::action_links::{
    ActionLinkAction, ActionLinkKind, ActionLinkMatch, actions_for_match, find_action_links,
    match_at_offset,
};
pub(in crate::features) use crate::theme::ThemePalette;
pub(in crate::features) use activity_bar_runtime::{
    ActivityBarDragPayload, ActivityBarDragPreview,
};
pub(in crate::features) use ai_jobs::{
    ai_active_profile_drafts, ai_job_cancelled, ai_usage_counts, is_agent_command_card,
    observation_summary, remote_command_observation, run_ai_ask_job,
};
pub use app_state::NyaTermApp;
pub(in crate::features) use app_state::{
    FailedSessionStart, PendingSavedConnectionStart, PendingSessionStart,
    SavedConnectionStartOptions, SessionPaneState,
};
pub(in crate::features) use appearance::{
    appearance_font_options, appearance_font_stack, gpui_code_font_family,
};
pub(in crate::features) use auth_runtime::{
    CredentialPromptBroker, CredentialPromptRequest, CredentialPromptState, HostKeyPromptBroker,
    HostKeyPromptChoice, HostKeyPromptIssue, HostKeyPromptRequest, KeyboardInteractivePromptState,
    NativeHostKeyVerifier, NativeOtpCodePreview, NativeOtpProvider, SftpDuplicatePromptBroker,
    SftpDuplicatePromptState, unix_seconds_now,
};
pub(in crate::features) use cloud_sync_provider::{
    pull_provider_snapshot, push_provider_snapshot, test_provider_connection,
};
pub(in crate::features) use connection_editor_window::ConnectionEditorWindow;
pub(in crate::features) use connection_runtime::ConnectionEditorToggle;
pub(in crate::features) use connections::{
    ConnectionDragKind, ConnectionDragPayload, ConnectionDragPreview, ConnectionDropPosition,
    ConnectionDropTarget,
};
pub(in crate::features) use formatting::*;
pub(in crate::features) use prompt_runtime::{
    credential_prompt_id, credential_prompt_target, keyboard_interactive_prompt_id,
    keyboard_interactive_prompt_target, sftp_duplicate_prompt_id, uuid_like_prompt_id,
};
pub(in crate::features) use quick_command_runtime::{
    QUICK_COMMAND_COLOR_OPTIONS, QUICK_COMMAND_ICON_OPTIONS, quick_command_category_label,
    quick_command_sort_mode_from_setting, quick_command_view_mode_from_setting,
    sorted_quick_commands,
};
pub(in crate::features) use quick_command_window::QuickCommandWindow;
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
pub(in crate::features) use settings_window::SettingsWindow;
pub(in crate::features) use tab_mouse::{
    ChromeTooltip, SessionTabDragPayload, SessionTabDragPreview, SessionTabTooltip,
    TAB_MOUSE_ACTIONS, TabMouseActionTarget,
};
pub(in crate::features) use transfer_external_sync_window::TransferExternalSyncWindow;
pub(in crate::features) use transfer_widgets::{
    compact_transfer_job_row, duplicate_decision_label, duplicate_policy_label, format_file_size,
    format_transfer_progress, transfer_input, transfer_job_title, transfer_status_label,
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
