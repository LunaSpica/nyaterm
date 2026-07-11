mod prelude;
use prelude::*;

mod ai_agent_runtime;
mod ai_jobs;
mod ai_runtime;
mod app_state;
mod appearance;
mod activity_bar_runtime;
mod auth_runtime;
mod cloud_sync_provider;
mod cloud_sync_runtime;
mod command_runtime;
mod credential_autofill_runtime;
mod config_runtime;
mod connection_runtime;
mod connections;
mod event_pump;
mod formatting;
mod global_shortcut_runtime;
mod input_runtime;
mod inspector;
mod keybinding_runtime;
mod layout;
mod lock_diagnostics_runtime;
mod navigation_runtime;
mod security_runtime;
mod panel_resize_runtime;
mod panel_stack_runtime;
mod pages;
mod panels;
mod prompt_runtime;
mod quick_command_runtime;
mod quick_switch_runtime;
mod recording_runtime;
mod remote_runtime;
mod root;
mod runtime_jobs;
mod send_command_runtime;
mod session_dialog_runtime;
mod session_lifecycle;
mod session_order;
mod startup_restore_runtime;
mod session_runtime;
mod session_state;
mod settings_runtime;
mod sync_input;
mod tab_mouse;
mod tab_windows_runtime;
mod temporary_ssh_link;
mod terminal_runtime;
mod terminal_search_runtime;
mod terminal_selection_runtime;
mod terminal_context_menu_runtime;
pub(in crate::ui::view) use terminal_selection_runtime::terminal_bounds_tracker;
mod terminal_surface;
mod transfer_events;
mod transfer_jobs;
mod transfer_options;
mod transfer_paths;
mod transfer_widgets;
mod translation_runtime;
mod tunnel_runtime;
mod update_runtime;
mod view_widgets;
mod workspace_runtime;
mod zmodem_runtime;

pub(in crate::ui::view) use ai_jobs::{
    ai_active_profile_api_key, ai_active_profile_drafts, ai_job_cancelled, ai_usage_counts,
    is_agent_command_card, observation_summary, remote_command_observation, run_ai_ask_job,
};
pub use app_state::NyaTermApp;
pub(in crate::ui::view) use connection_runtime::ConnectionEditorToggle;
pub(in crate::ui::view) use auth_runtime::{
    CredentialPromptBroker, CredentialPromptState, HostKeyPromptBroker, HostKeyPromptChoice,
    HostKeyPromptIssue, HostKeyPromptRequest, NativeHostKeyVerifier, NativeOtpProvider,
    SftpDuplicatePromptBroker, SftpDuplicatePromptState,
};
pub(in crate::ui::view) use cloud_sync_provider::{pull_provider_snapshot, push_provider_snapshot};
pub(in crate::ui::view) use formatting::*;
pub(in crate::ui::view) use prompt_runtime::{
    credential_prompt_id, credential_prompt_target, sftp_duplicate_prompt_id, uuid_like_prompt_id,
};
pub(in crate::ui::view) use quick_command_runtime::{
    QUICK_COMMAND_COLOR_OPTIONS, QUICK_COMMAND_ICON_OPTIONS, quick_command_category_label,
    quick_command_sort_mode_from_setting, quick_command_view_mode_from_setting,
    sorted_quick_commands,
};
pub(in crate::ui::view) use runtime_jobs::{
    ActivitySide, AiAgentBackgroundTarget, AiAgentLoopState, AiAgentStepStatus, AiAgentStepView,
    AiChatJobOutput, AiChatJobResult, AiChatWorkerEvent, AiDiscoveryJobResult, DockerJobOutput,
    DockerJobResult, ProcessJobOutput, ProcessJobResult, SessionStartResult, SessionStartSuccess,
    StatsJobResult, TranslateJobResult, TunnelJobOutput, TunnelJobResult, UpdateJobResult,
};
pub(in crate::ui::view) use activity_bar_runtime::{
    ActivityBarDragPayload, ActivityBarDragPreview,
};
pub(in crate::ui::view) use connections::{
    ConnectionDragKind, ConnectionDragPayload, ConnectionDragPreview, ConnectionDropPosition, ConnectionDropTarget,
};
pub(in crate::ui::view) use tab_mouse::{
    SessionTabDragPayload, SessionTabDragPreview, SessionTabTooltip, TabMouseActionTarget, tab_mouse_action_label,
};
pub(in crate::ui::view) use transfer_widgets::{
    compact_transfer_job_row, duplicate_decision_label, duplicate_policy_label, entry_kind_label,
    format_file_size, format_transfer_progress, transfer_input, transfer_job_title,
    transfer_progress_bar, transfer_status_label,
};
#[allow(unused_imports)]
pub(in crate::ui::view) use crate::ui::action_links::{ActionLinkAction, ActionLinkKind, ActionLinkMatch, actions_for_match, find_action_links, match_at_offset};
pub(in crate::ui::view) use crate::ui::theme::{ThemePalette, theme_palette};
pub(in crate::ui::view) use view_widgets::*;

const LEGACY_ROOT: &str = "nyaterm-tauri";
pub(super) const INITIAL_TERMINAL_BANNER: &str = "$ nyaterm --native\nGPUI shell initialized.\nStart a local terminal or open a saved connection.\n";
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
