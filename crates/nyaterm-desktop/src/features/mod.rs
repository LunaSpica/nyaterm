mod ai;
mod app_state;
mod commands;
mod connections;
mod formatting;
mod icons;
mod inspector;
mod layout;
mod pages;
mod panels;
mod recording;
mod remote;
mod remote_desktop;
mod root;
mod runtime_jobs;
mod selects;
mod session;
mod settings;
mod shell;
mod sync;
mod sync_input;
mod terminal;
mod text_inputs;
mod transfers;
mod translation;
mod tunnels;
mod update;
mod view_widgets;

pub(crate) fn init(cx: &mut gpui::App) {
    terminal::init_key_bindings(cx);
}

#[allow(unused_imports)]
pub(in crate::features) use crate::action_links::{
    ActionLinkAction, ActionLinkKind, ActionLinkMatch, actions_for_match, find_action_links,
    match_at_offset,
};
pub(in crate::features) use crate::theme::ThemePalette;
pub(in crate::features) use ai::{
    AiFeatureFocus, AiFeatureInit, AiFeatureState, ai_active_profile_drafts, is_agent_command_card,
};
pub use app_state::NyaTermApp;
pub(in crate::features) use commands::{
    CommandFeatureInit, CommandFeatureState, CommandPersistencePoll, QUICK_COMMAND_COLOR_OPTIONS,
    QuickCommandDropPosition, QuickCommandDropTarget, QuickCommandFeatureFocus,
    quick_command_category_label, quick_command_sort_mode_from_setting,
    quick_command_view_mode_from_setting,
};
pub(in crate::features) use connections::{
    ConnectionDragKind, ConnectionDragPayload, ConnectionDragPreview, ConnectionDropPosition,
    ConnectionDropTarget, ConnectionEditorToggle, ConnectionFeatureFocus, ConnectionFeatureState,
};
pub(in crate::features) use formatting::{
    compact_id, configured_cloud_sync_provider, docker_compose_project_key, docker_state_color,
    docker_state_label, docker_state_rank, format_cloud_provider, format_history_timestamp_ms,
    format_last_used_ms, format_rate, format_uptime, non_empty_string, none_if_blank,
    recent_terminal_output, session_kind_label, short_id, tunnel_endpoint, tunnel_mode,
    tunnel_name,
};
pub(in crate::features) use icons::{
    CONNECTION_ICON_OPTIONS, DEFAULT_CONNECTION_ICON, IconDef, QUICK_COMMAND_ICON_OPTIONS,
    SEARCH_ENGINE_ICON_IDS, file_entry_icon, infer_connection_icon_key_from_remote_system,
    known_search_engine_icon, quick_command_icon, resolve_connection_icon, search_engine_icon,
};
pub(in crate::features) use panels::{
    SendCommandFeatureFocus, SendCommandFeatureState, SendCommandPresentationState,
};
pub(in crate::features) use recording::RecordingFeatureState;
pub(in crate::features) use remote::{RemoteOpsFeatureFocus, RemoteOpsFeatureState};
pub(in crate::features) use remote_desktop::RemoteDesktopFeatureState;
pub(in crate::features) use runtime_jobs::{
    ActivitySide, AiAgentBackgroundTarget, AiAgentLoopState, AiAgentStepStatus, AiAgentStepView,
    AiChatJobOutput, AiChatJobResult, AiChatWorkerEvent, AiDiscoveryJobResult,
    CommandPersistenceRequest, CommandPersistenceResult, DockerJobResult, GpuJobResult,
    NpuJobResult, ProcessJobResult, SessionStartResult, SessionStartSuccess, StatsJobResult,
    TunnelJobOutput, TunnelJobResult, spawn_command_persistence_worker,
};
pub(in crate::features) use selects::{FOLLOW_UI_THEME_VALUE, NO_SELECTION_VALUE, SelectRegistry};
pub(in crate::features) use session::{
    AgentPromptBroker, CredentialPromptBroker, HostKeyPromptBroker, NativeOtpProvider,
    PendingSessionStart, SavedConnectionStartOptions, SessionFeatureFocus, SessionFeatureState,
    SessionStartEventRequest, credential_prompt_target, keyboard_interactive_prompt_target,
};
pub(in crate::features) use settings::{
    SecurityCatalogState, SecurityFeatureFocus, SecurityFeatureState,
};
pub(in crate::features) use shell::{
    SessionTabDragPayload, SessionTabDragPreview, SessionTabTooltip, ShellFeatureInit,
    ShellFeatureState, TAB_MOUSE_ACTIONS, TabMouseActionTarget,
};
pub(in crate::features) use shell::{
    appearance_font_options, appearance_font_stack, gpui_code_font_family,
};
pub(in crate::features) use sync::CloudSyncFeatureState;
pub(in crate::features) use sync_input::SyncInputFeatureState;
pub(in crate::features) use terminal::{
    TerminalFeatureFocus, TerminalFeatureState, full_shell_paint_count,
    terminal_surface_paint_count,
};
pub(in crate::features) use text_inputs::{
    ORDINARY_INPUT_SHELL_PADDING_X_PX, TextInputRegistry, TextInputSetup,
    ordinary_input_focus_ring, ordinary_input_shell_border_color, secret_input_setup,
};
pub(in crate::features) use transfers::{
    TransferEditorCloseAfterSave, TransferEditorCloseOutcome, TransferEditorDiscardOutcome,
    TransferFeatureFocus, TransferFeatureState, duplicate_decision_label, duplicate_policy_label,
    format_file_size,
};
pub(in crate::features) use translation::TranslationFeatureState;
pub(in crate::features) use tunnels::{TunnelCatalogState, TunnelFeatureState};
pub(in crate::features) use update::UpdateFeatureState;
pub(in crate::features) use view_widgets::{
    bounded_dialog_width, child_window_header, child_window_titlebar, color_icon,
    connection_type_icon, dialog_action_button, horizontal_resize_handle_visual, logo_mark,
    modal_dialog_shell, mono_icon, nyaterm_app_icon, panel_header_with_actions, stats_progress_bar,
    themed_icon, transfer_entry_icon, vertical_resize_handle_visual, window_control_button,
};
