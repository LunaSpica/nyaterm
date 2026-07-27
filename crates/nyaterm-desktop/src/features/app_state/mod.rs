use super::*;

use crate::models::{
    ActionLinkMenuState, ActionLinkTooltipState, ActiveSessionMenuState,
    ActivityBarContextMenuState, ActivityBarLayoutState, BottomPanelMode, BottomPanelResizeState,
    CloudSyncConflictState, CloudSyncSecretDraft, CommandSuggestionState, ConfigPathPromptKind,
    CredentialSuggestionState, DiagnosticsPathPromptKind, GithubGistAuthJobEvent,
    GithubGistAuthState, HeaderStatusState, KeywordHighlightEditorField,
    KeywordHighlightPathPromptKind, MainMode, MultiLinePasteDraft, PanelResizeState,
    PanelStackResizeState, PendingCredentialAutofill, RecordingPathPromptKind,
    RecordingWritePipeline, RightFocus, SearchEngineEditorField, SessionEventBridge,
    SessionRuntimeMetadata, SettingsTab, SnapshotPasswordPromptState, StartupCommandAction,
    StartupCommandRequest, StoreStatus, SyncInputGroup, TabActionsSubmenu, TerminalFrameEvent,
    TitleMenu, TitleMenuSubmenu, TranslateInputField, TranslationDialogState,
    TranslationSecretDraft, WorkspacePaneNode, WorkspaceSplitDirection, WorkspaceSplitResizeState,
    WorkspaceSplitState,
};
use nyaterm_core::{TranslateResult, TranslationSettings};

mod construct;
mod types;

pub(in crate::features) use types::{
    FailedSessionStart, PendingSavedConnectionStart, PendingSessionStart,
    SavedConnectionStartOptions, SessionPaneState, SettingsDraftSnapshot, TerminalRuntimeUiState,
};

pub struct NyaTermApp {
    pub(in crate::features) stores: crate::entities::UiStoreHandles,
    pub(in crate::features) runtime: AppRuntime,
    pub(in crate::features) services: NativeServices,
    pub(in crate::features) inventory: MigrationInventory,
    pub(in crate::features) connections: Vec<SavedConnection>,
    pub(in crate::features) pending_saved_connection_queue: VecDeque<PendingSavedConnectionStart>,
    pub(in crate::features) connection_groups: Vec<Group>,
    pub(in crate::features) connection_state: ConnectionFeatureState,
    /// Real text inputs for the panels that have not been given their own,
    /// keyed by an id the panel picks. See `features::text_inputs`.
    pub(in crate::features) text_inputs: TextInputRegistry,
    pub(in crate::features) connection_ssh_keys: Vec<SshKey>,
    pub(in crate::features) connection_otp_entries: Vec<OtpEntry>,
    pub(in crate::features) connection_saved_passwords: Vec<SavedPassword>,
    pub(in crate::features) connection_saved_credentials: Vec<SavedCredential>,
    pub(in crate::features) connection_serial_ports: Vec<String>,
    pub(in crate::features) tunnels: Vec<TunnelConfig>,
    pub(in crate::features) tunnel_groups: Vec<TunnelGroup>,
    pub(in crate::features) proxies: Vec<ProxyConfig>,
    pub(in crate::features) proxy_groups: Vec<ProxyGroup>,
    pub(in crate::features) quick_commands: Arc<[QuickCommand]>,
    pub(in crate::features) quick_command_categories: Vec<QuickCommandCategory>,
    pub(in crate::features) quick_command_state: QuickCommandFeatureState,
    pub(in crate::features) remote_ops: RemoteOpsFeatureState,
    pub(in crate::features) security: SecurityFeatureState,
    pub(in crate::features) ai: AiFeatureState,
    pub(in crate::features) terminal: TerminalFeatureState,
    pub(in crate::features) send_command: SendCommandFeatureState,
    pub(in crate::features) transfer: TransferFeatureState,
    pub(in crate::features) command_history: Arc<[CommandHistoryEntry]>,
    pub(in crate::features) command_persistence_tx: mpsc::Sender<CommandPersistenceRequest>,
    pub(in crate::features) command_persistence_rx: mpsc::Receiver<CommandPersistenceResult>,
    pub(in crate::features) command_persistence_pending: usize,
    pub(in crate::features) session_command_history: HashMap<String, Vec<String>>,
    /// Inline terminal command suggestions (Tauri CommandSuggestions).
    pub(in crate::features) command_suggestions: Option<CommandSuggestionState>,
    /// Local terminal input tracker for inline suggestions (Tauri terminalInputTracker).
    pub(in crate::features) command_input_tracker: TerminalInputState,
    /// Hide suggestions after interactive programs (vim/htop/less/...) until Ctrl+C/q.
    pub(in crate::features) command_suggestions_suppressed: bool,
    /// Command captured from tracker at Enter, consumed by history recording.
    pub(in crate::features) pending_command_history_entry: Option<String>,
    /// Bumps to cancel in-flight 80ms suggestion search timers (Tauri searchTimer).
    pub(in crate::features) command_suggestion_search_gen: u64,
    /// Owning the debounce task lets a newer key cancel the previous timer.
    pub(in crate::features) command_suggestion_refresh_task: Option<gpui::Task<()>>,
    /// Terminal-output credential autofill panel (Tauri CredentialSuggestions).
    pub(in crate::features) credential_suggestions: Option<CredentialSuggestionState>,
    pub(in crate::features) credential_autofill_buffer: String,
    pub(in crate::features) credential_autofill_recent: HashMap<String, u64>,
    pub(in crate::features) credential_autofill_pending: Option<PendingCredentialAutofill>,
    pub(in crate::features) credential_autofill_detection_pending: bool,
    pub(in crate::features) credential_autofill_next_request_id: u64,
    pub(in crate::features) credential_autofill_pending_request:
        Option<CredentialAutofillMatchRequestKey>,
    pub(in crate::features) credential_autofill_match_pipeline: CredentialAutofillMatchPipeline,
    pub(in crate::features) credential_autofill_sending: bool,
    /// Suppress command suggestions while a password/username prompt is active.
    pub(in crate::features) credential_prompt_input_until_ms: u64,

    pub(in crate::features) active_sessions_search_draft: String,
    /// Root-level reconnect/disconnect overflow menu (Tauri ActiveSessions DropdownMenu).
    pub(in crate::features) active_session_menu: Option<ActiveSessionMenuState>,
    /// Per-session reconnect/disconnect busy state ("reconnect" | "disconnect").
    pub(in crate::features) active_session_busy_actions: HashMap<String, String>,
    pub(in crate::features) quick_switch_focus: FocusHandle,
    /// Which search engine row is focused for name/url editing (Settings → Search).
    pub(in crate::features) search_engine_edit_index: Option<usize>,
    pub(in crate::features) search_engine_expanded_index: Option<usize>,
    pub(in crate::features) search_engine_icon_picker_index: Option<usize>,
    pub(in crate::features) search_engine_actions_index: Option<usize>,
    pub(in crate::features) search_engine_edit_field: SearchEngineEditorField,
    pub(in crate::features) search_engine_focus: FocusHandle,
    pub(in crate::features) action_link_menu: Option<ActionLinkMenuState>,
    pub(in crate::features) action_link_tooltip: Option<ActionLinkTooltipState>,
    /// Pending action-link hover (Tauri 250ms delay before showing tooltip).
    pub(in crate::features) action_link_hover_pending:
        Option<(String, Instant, ActionLinkTooltipState)>,

    pub(in crate::features) translation_dialog: Option<TranslationDialogState>,
    pub(in crate::features) bottom_panel: BottomPanelMode,
    pub(in crate::features) quick_cmd_height: f32,
    pub(in crate::features) serial_send_height: f32,
    pub(in crate::features) bottom_panel_resize: Option<BottomPanelResizeState>,
    pub(in crate::features) sync_groups: Vec<SyncInputGroup>,
    pub(in crate::features) sync_groups_open: bool,
    pub(in crate::features) sync_groups_focus: FocusHandle,
    pub(in crate::features) sync_groups_search_draft: String,
    pub(in crate::features) sync_groups_selected_id: Option<String>,
    pub(in crate::features) sync_groups_delete_pending: Option<String>,
    /// Broadcast keyboard input to every live session (Tauri broadcastToAll).
    pub(in crate::features) broadcast_to_all: bool,
    pub(in crate::features) keyword_highlights: KeywordHighlightConfig,
    pub(in crate::features) keyword_highlight_expanded_id: Option<String>,
    pub(in crate::features) keyword_highlight_edit_id: Option<String>,
    pub(in crate::features) keyword_highlight_edit_field: KeywordHighlightEditorField,
    pub(in crate::features) keyword_highlight_focus: FocusHandle,
    pub(in crate::features) settings: AppSettingsSummary,
    pub(in crate::features) settings_master_password_enabled: bool,
    pub(in crate::features) settings_master_password_draft: String,
    pub(in crate::features) appearance_menu_open: Option<String>,
    pub(in crate::features) appearance_ui_font_options: Vec<String>,
    pub(in crate::features) appearance_terminal_font_options: Vec<String>,
    pub(in crate::features) keybinding_recording_id: Option<String>,
    pub(in crate::features) keybinding_pending_keys: Option<String>,
    pub(in crate::features) keybinding_search_draft: String,
    pub(in crate::features) keybindings_focus: FocusHandle,
    pub(in crate::features) store_status: StoreStatus,
    pub(in crate::features) session_manager: Arc<SessionManager>,
    pub(in crate::features) session_event_bridge: SessionEventBridge,
    pub(in crate::features) recording_manager: Arc<RecordingManager>,
    /// Cached count of sessions with active file recording (paint-safe).
    pub(in crate::features) recording_active_count: usize,
    /// Deferred auto-start recording after connect (avoid file open on success arm).
    pub(in crate::features) pending_auto_recording_session: Option<(String, String)>,
    pub(in crate::features) recording_write_pipeline: RecordingWritePipeline,
    pub(in crate::features) recording_search_draft: String,
    /// Per-session recording panel busy state ("record" | "save").
    pub(in crate::features) recording_busy_actions: HashMap<String, String>,
    pub(in crate::features) session_start_tx: mpsc::Sender<SessionStartResult>,
    pub(in crate::features) session_start_rx: mpsc::Receiver<SessionStartResult>,
    pub(in crate::features) tunnel_manager: Arc<SshTunnelManager>,
    pub(in crate::features) tunnel_tx: mpsc::Sender<TunnelJobResult>,
    pub(in crate::features) tunnel_rx: mpsc::Receiver<TunnelJobResult>,
    pub(in crate::features) pending_tunnels: Vec<String>,
    pub(in crate::features) translate_tx: mpsc::Sender<TranslateJobResult>,
    pub(in crate::features) translate_rx: mpsc::Receiver<TranslateJobResult>,
    pub(in crate::features) translate_provider: String,
    pub(in crate::features) translation_settings: TranslationSettings,
    pub(in crate::features) translation_secret_draft: TranslationSecretDraft,
    pub(in crate::features) translate_target_language: String,
    pub(in crate::features) translate_input: String,
    pub(in crate::features) translate_result: Option<TranslateResult>,
    pub(in crate::features) translate_status: String,
    pub(in crate::features) translate_pending: bool,
    pub(in crate::features) translate_focused_field: TranslateInputField,
    pub(in crate::features) update_tx: mpsc::Sender<UpdateJobResult>,
    pub(in crate::features) update_rx: mpsc::Receiver<UpdateJobResult>,
    pub(in crate::features) update_status: String,
    pub(in crate::features) update_info: Option<NativeUpdateInfo>,
    pub(in crate::features) update_pending: bool,
    pub(in crate::features) update_dialog_open: bool,
    pub(in crate::features) about_open: bool,
    pub(in crate::features) remote_editor_window: Option<WindowHandle<RemoteFileEditorWindow>>,
    pub(in crate::features) remote_editor_window_open_pending: bool,
    pub(in crate::features) recording_path_prompt: Option<RecordingPathPromptKind>,
    pub(in crate::features) config_path_prompt: Option<ConfigPathPromptKind>,
    pub(in crate::features) diagnostics_path_prompt: Option<DiagnosticsPathPromptKind>,
    pub(in crate::features) keyword_highlight_path_prompt: Option<KeywordHighlightPathPromptKind>,
    pub(in crate::features) active_snapshot_password_prompt: Option<SnapshotPasswordPromptState>,
    pub(in crate::features) cloud_sync_settings: CloudSyncSettings,
    pub(in crate::features) cloud_sync_state: CloudSyncState,
    pub(in crate::features) cloud_sync_history: Vec<CloudSyncHistoryEntry>,
    pub(in crate::features) cloud_sync_history_expanded: HashSet<String>,
    pub(in crate::features) cloud_sync_conflict: Option<CloudSyncConflictState>,
    pub(in crate::features) cloud_sync_secret_draft: CloudSyncSecretDraft,
    pub(in crate::features) cloud_sync_status: String,
    /// Prevent overlapping network jobs from applying cloud state out of order.
    pub(in crate::features) cloud_sync_job_running: bool,
    pub(in crate::features) cloud_sync_focused_field: CloudSyncInputField,
    pub(in crate::features) cloud_sync_provider_menu_open: bool,
    pub(in crate::features) github_gist_auth: GithubGistAuthState,
    pub(in crate::features) github_gist_auth_tx: mpsc::Sender<GithubGistAuthJobEvent>,
    pub(in crate::features) github_gist_auth_rx: mpsc::Receiver<GithubGistAuthJobEvent>,
    pub(in crate::features) github_gist_auth_job_id: u64,
    pub(in crate::features) github_gist_auth_cancel: Option<Arc<AtomicBool>>,
    pub(in crate::features) duplicate_prompts: Arc<SftpDuplicatePromptBroker>,
    pub(in crate::features) active_duplicate_prompt: Option<SftpDuplicatePromptState>,
    pub(in crate::features) pending_session_starts: HashMap<String, PendingSessionStart>,
    pub(in crate::features) active_pending_session_start: Option<String>,
    pub(in crate::features) failed_session_starts: HashMap<String, FailedSessionStart>,
    pub(in crate::features) active_failed_session_start: Option<String>,
    /// Session starts removed from the UI while their worker may still finish.
    pub(in crate::features) cancelled_session_start_requests: HashSet<String>,
    pub(in crate::features) session_pane_states: HashMap<String, SessionPaneState>,
    /// Disconnected session id being replaced by an in-flight reconnect.
    pub(in crate::features) pending_reconnect_replace_id: Option<String>,
    /// Reconnect failures rendered in their original workspace panes.
    pub(in crate::features) reconnect_session_failures: HashMap<String, String>,
    pub(in crate::features) pending_workspace_split: Option<(WorkspaceSplitDirection, String)>,
    pub(in crate::features) host_key_prompts: Arc<HostKeyPromptBroker>,
    pub(in crate::features) active_host_key_prompt: Option<HostKeyPromptRequest>,
    pub(in crate::features) credential_prompts: Arc<CredentialPromptBroker>,
    pub(in crate::features) active_credential_prompt: Option<CredentialPromptState>,
    pub(in crate::features) active_keyboard_interactive_prompt:
        Option<KeyboardInteractivePromptState>,
    pub(in crate::features) credential_prompt_focus_pending: bool,
    pub(in crate::features) credential_focus: FocusHandle,
    pub(in crate::features) snapshot_password_focus: FocusHandle,
    pub(in crate::features) otp_provider: Arc<NativeOtpProvider>,
    pub(in crate::features) active_session_id: Option<String>,
    pub(in crate::features) active_ssh_config: Option<SshSessionConfig>,
    pub(in crate::features) active_ai_execution_profile: AiExecutionProfile,
    pub(in crate::features) session_order: Vec<String>,
    pub(in crate::features) session_metadata: HashMap<String, SessionRuntimeMetadata>,
    pub(in crate::features) session_custom_names: HashMap<String, String>,
    /// OSC 0/2 titles from the session PTY (fall back when no custom rename).
    pub(in crate::features) session_dynamic_titles: HashMap<String, String>,
    /// Latest OSC 7 working directories per session.
    pub(in crate::features) session_cwds: HashMap<String, String>,
    /// Per-session ZMODEM detector / transfer state (UI-layer interception).
    pub(in crate::features) zmodem_sessions:
        HashMap<String, crate::features::session::ZmodemSessionState>,
    /// Per-session trzsz trigger detector state (pre-parser protocol slot).
    pub(in crate::features) trzsz_sessions:
        HashMap<String, crate::features::session::TrzszSessionState>,
    pub(in crate::features) session_tab_colors: HashMap<String, u32>,
    pub(in crate::features) ssh_multiplex_handles: HashMap<String, SshMultiplexHandle>,
    pub(in crate::features) tab_actions_session_id: Option<String>,
    pub(in crate::features) tab_actions_anchor: Option<(f32, f32)>,
    pub(in crate::features) tab_actions_submenu: Option<TabActionsSubmenu>,
    pub(in crate::features) tab_actions_focus: FocusHandle,
    pub(in crate::features) close_all_sessions_confirm_open: bool,
    pub(in crate::features) pending_quit_after_close_all: bool,
    pub(in crate::features) pending_window_quit: bool,
    pub(in crate::features) close_all_sessions_confirm_focus: FocusHandle,
    pub(in crate::features) rename_session_id: Option<String>,
    pub(in crate::features) rename_draft: String,
    pub(in crate::features) rename_focus: FocusHandle,
    pub(in crate::features) color_picker_open: bool,
    pub(in crate::features) color_picker_focus: FocusHandle,
    pub(in crate::features) session_info_open: bool,
    pub(in crate::features) session_info_focus: FocusHandle,
    pub(in crate::features) startup_command_open: bool,
    pub(in crate::features) startup_command_action: StartupCommandAction,
    pub(in crate::features) startup_command_draft: String,
    pub(in crate::features) startup_command_delay_ms: u64,
    pub(in crate::features) startup_command_focus: FocusHandle,
    pub(in crate::features) temporary_ssh_link_open: bool,
    pub(in crate::features) temporary_ssh_link_draft: String,
    pub(in crate::features) temporary_ssh_link_error: Option<&'static str>,
    pub(in crate::features) temporary_ssh_link_focus: FocusHandle,
    pub(in crate::features) multi_line_paste: Option<MultiLinePasteDraft>,
    pub(in crate::features) multi_line_paste_marked_text: String,
    pub(in crate::features) multi_line_paste_marked_range: Option<std::ops::Range<usize>>,
    pub(in crate::features) multi_line_paste_cursor: usize,
    pub(in crate::features) multi_line_paste_anchor: Option<usize>,
    pub(in crate::features) multi_line_paste_focus: FocusHandle,
    pub(in crate::features) lock_focus: FocusHandle,
    pub(in crate::features) lock_password_draft: String,
    pub(in crate::features) lock_status: String,
    pub(in crate::features) pending_terminal_frame_events: VecDeque<TerminalFrameEvent>,
    pub(in crate::features) pending_session_events: VecDeque<SessionEvent>,
    pub(in crate::features) diagnostic_log_last_at: HashMap<&'static str, Instant>,
    /// Cached terminal surface palette (theme + contrast fingerprint).
    pub(in crate::features) cached_terminal_theme_palette:
        Option<(String, String, String, crate::theme::ThemePalette)>,
    /// Cached keyword highlight rules for paint (invalidated on settings change).
    pub(in crate::features) cached_keyword_highlight_rules:
        Option<std::sync::Arc<Vec<nyaterm_core::ResolvedKeywordHighlightRule>>>,

    pub(in crate::features) last_viewport_size: (f32, f32),
    /// Cached intrinsic dimensions for the current tiled wallpaper path.
    pub(in crate::features) wallpaper_tile_dimensions: Option<(String, u32, u32)>,
    /// When the window viewport last changed (resize/drag geometry).
    pub(in crate::features) last_viewport_change_at: Option<Instant>,
    /// Deadline for treating title-bar window dragging as active.
    pub(in crate::features) title_drag_active_until: Option<Instant>,
    pub(in crate::features) selected_nav: NavItem,
    pub(in crate::features) main_mode: MainMode,
    pub(in crate::features) settings_active_tab: SettingsTab,
    /// Expanded multi-item groups in the settings sidebar (Tauri keeps this local to the page).
    pub(in crate::features) settings_expanded_groups: HashSet<String>,
    /// Committed values captured when the in-window settings page opens.
    pub(in crate::features) settings_draft_snapshot: Option<SettingsDraftSnapshot>,
    pub(in crate::features) settings_window: Option<WindowHandle<SettingsWindow>>,
    pub(in crate::features) settings_window_open_pending: bool,
    /// Main workspace panel state to restore after leaving the in-window settings page.
    pub(in crate::features) settings_previous_left_collapsed: Option<bool>,
    pub(in crate::features) settings_previous_right_collapsed: Option<bool>,
    pub(in crate::features) active_left_panel: Option<NavItem>,
    pub(in crate::features) active_right_panel: Option<NavItem>,
    pub(in crate::features) left_open_panels: Vec<String>,
    pub(in crate::features) right_open_panels: Vec<String>,
    pub(in crate::features) panel_stack_sizes: HashMap<String, f32>,
    pub(in crate::features) panel_multi_open: bool,
    pub(in crate::features) right_focus: RightFocus,
    pub(in crate::features) left_sidebar_collapsed: bool,
    pub(in crate::features) right_inspector_collapsed: bool,
    pub(in crate::features) mobile_left_open: bool,
    pub(in crate::features) mobile_right_open: bool,
    pub(in crate::features) left_panel_width: f32,
    pub(in crate::features) right_panel_width: f32,
    pub(in crate::features) panel_resize: Option<PanelResizeState>,
    pub(in crate::features) panel_stack_resize: Option<PanelStackResizeState>,
    pub(in crate::features) activity_bar_layout: ActivityBarLayoutState,
    pub(in crate::features) activity_bar_context_menu: Option<ActivityBarContextMenuState>,
    pub(in crate::features) title_menu_open: Option<TitleMenu>,
    pub(in crate::features) title_menu_submenu: Option<TitleMenuSubmenu>,
    pub(in crate::features) header_status: HeaderStatusState,
    /// Open-tabs overflow menu (Tauri TabBar expand-more when many tabs).
    pub(in crate::features) open_tabs_menu_open: bool,
    /// New-session menu next to the tab strip + control.
    pub(in crate::features) new_session_menu_open: bool,
    /// Whether the Tauri-style "All sessions" submenu is expanded.
    pub(in crate::features) new_session_all_sessions_open: bool,
    /// Hovered group ids from the root submenu through the deepest open child menu.
    pub(in crate::features) new_session_group_menu_path: Vec<String>,
    /// Horizontal scroll handle for the global session tab strip (scroll-into-view).
    pub(in crate::features) session_tab_strip_scroll: ScrollHandle,
    /// Request scroll-into-view of the active tab on next paint (Tauri TabBar).
    pub(in crate::features) session_tab_scroll_into_view_pending: bool,
    /// Last failed connect name (shown as ephemeral failed tab chrome).
    pub(in crate::features) last_connect_failure_name: Option<String>,
    /// Last failed connect error text.
    pub(in crate::features) last_connect_failure_error: Option<String>,
    /// Legacy/global active pane tree view: mirrors the active tab's per-tab root when split.
    pub(in crate::features) workspace_split: Option<WorkspaceSplitState>,
    pub(in crate::features) workspace_split_resize: Option<WorkspaceSplitResizeState>,
    /// Per-tab pane trees keyed by tab-root session id (Tauri `Tab.root`).
    pub(in crate::features) session_pane_roots: HashMap<String, WorkspacePaneNode>,
    /// Leaf session id → owning tab-root session id (hidden from tab strip when secondary).
    pub(in crate::features) session_tab_owner: HashMap<String, String>,
    pub(in crate::features) focused_terminal_window_leaf_id: Option<String>,
    /// Whether we already attempted startup restore of global workspace pane splits.
    pub(in crate::features) workspace_pane_layout_restored: bool,
    pub(in crate::features) startup_restore_complete: bool,
    pub(in crate::features) is_locked: bool,
    pub(in crate::features) last_user_activity_at: Instant,
}
