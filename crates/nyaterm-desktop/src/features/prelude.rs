pub(super) use gpui::{
    AnimationExt, AnyElement, App, ClickEvent, ClipboardItem, Context, Entity, FocusHandle,
    FontWeight, IntoElement, KeyDownEvent, KeyUpEvent, MouseButton, MouseDownEvent,
    PathPromptOptions, Render, ScrollDelta, ScrollHandle, ScrollWheelEvent, SharedString,
    Subscription, Timer, Window, WindowControlArea, WindowHandle, div, prelude::*, px, rgb, rgba,
    svg,
};
pub(super) use nyaterm_core::{
    AgentCommandExecutionMode, AgentOutputCaptureProcessor, AiAction, AiChatRequest, AiCommandCard,
    AiContext, AiExecutionProfile, AiMessage, AiMessageRole, AiMode, AiModelDiscovery,
    AiProviderCredential, AiProviderKind, AiSession, AiSettings, AppRuntime, AppSettingsSummary,
    AppendAiAuditRequest, CLOUD_SYNC_HISTORY_LIMIT, CloudSyncError, CloudSyncHistoryEntry,
    CloudSyncSettings, CloudSyncState, CommandHistoryEntry, CommandObservation, ConnectionAuth,
    ConnectionStore, ConnectionType, CredentialPromptKind, DecryptedOtpEntry,
    GiteeSnippetHttpBackend, GithubGistHttpBackend, Group, InputSelectionRange,
    KeywordHighlightConfig, KnownHostCheck, LocalCloudSyncOptions, NativeServices,
    NativeUpdateInfo, OtpEntry, ProxyConfig, ProxyGroup, QuickCommand, QuickCommandCategory,
    QuickCommandsConfig, RiskLevel, RuntimeMode, SavedConnection, SavedCredential, SavedPassword,
    SnippetRemote, SshKey, TerminalInputState, TerminalWireWriteKind, TranslateResult,
    TranslationSettings, TunnelConfig, TunnelGroup, agent_response_action,
    ai_model_id_for_credential, ai_model_id_for_provider, append_cloud_sync_history,
    apply_terminal_input_data, assess_agent_command_risk, build_agent_capture_command,
    build_move_input_cursor_data, build_observation_message, can_suggest_from_tracked_command,
    can_suggest_from_tracker, command_starts_suggestion_suppressing_program,
    decide_agent_command_execution, delete_terminal_input_range, export_diagnostics_archive,
    get_tracked_command, get_tracked_submission_command, merge_model_discoveries, now_rfc3339,
    parse_agent_model_output, parse_agent_tool_call, parse_model_output, pull_local_snapshot,
    pull_snapshot_with_remote, push_local_snapshot, push_snapshot_with_remote,
    read_cloud_sync_history, redact_context, redact_sensitive_text, resync_from_terminal_line,
    search_command_sources, terminal_input_fanout_status, terminal_input_tracker_below_min_chars,
    terminal_wire_write_disposition, truncate_preview, uuid,
};
#[cfg(feature = "migration-dashboard")]
pub(super) use nyaterm_legacy::LegacyProject;
pub(super) use nyaterm_legacy::MigrationInventory;
pub(super) use nyaterm_terminal::{
    TerminalEffects, TerminalOutputDecoder, TerminalScreen, TerminalSnapshot,
};
pub(super) use nyaterm_transport::{
    DockerComposeService, DockerContainerDetails, DockerService, LocalSessionConfig,
    RecordingManager, RemoteCommandOutput, RemoteDockerOverview, RemoteProcess, RemoteStats,
    SerialSessionConfig, SessionEvent, SessionInfo, SessionKind, SessionManager,
    SftpDuplicateDecision, SftpDuplicatePolicy, SftpDuplicateRequest, SftpDuplicateResolver,
    SftpFileEntry, SftpService, SftpTransferControl, SftpTransferOptions, SftpTransferProgress,
    SshCredentialPrompt, SshCredentialPromptKind, SshCredentialPromptReason, SshCredentialProvider,
    SshHostKey, SshHostKeyDecision, SshHostKeyVerifier, SshKeyAuthConfig,
    SshKeyboardInteractiveRequest, SshMultiplexHandle, SshOtpProvider, SshProcessService,
    SshProxyConfig, SshSessionConfig, SshTunnelInfo, SshTunnelManager, TelnetSessionConfig,
    TerminalHistorySearchRequest,
};

pub(super) use crate::http::ai::{
    complete_native_chat, discover_openai_compatible_models, stream_native_chat,
};
pub(super) use crate::http::cloud_sync::{
    NativeAliyunDriveRemote, NativeGoogleDriveRemote, NativeOneDriveRemote, NativeS3Remote,
    NativeSnippetHttpClient, NativeWebdavRemote, run_github_gist_device_flow,
};
pub(super) use std::collections::{HashMap, HashSet, VecDeque};
pub(super) use std::path::PathBuf;
pub(super) use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};
pub(super) use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(super) use crate::models::{
    ActionLinkMenuAction, ActionLinkMenuState, ActionLinkTooltipState, ActiveSessionMenuState,
    ActivityBarContextMenuState, ActivityBarEntry, ActivityBarLayoutState, ActivityBarZone,
    AiActionEditorField, AiActionListKind, AiCredentialEditorField, AiDetectedErrorState,
    AiInputField, AiMessageMenuState, AiPreparedRequest, BottomPanelMode, BottomPanelResizeState,
    CloudSyncConflictState, CloudSyncInputField, CloudSyncSecretDraft, CommandSuggestionItem,
    CommandSuggestionState, ConfigPathPromptKind, ConfigPathPromptResult,
    CredentialAutofillMatchEvent, CredentialAutofillMatchOutcome, CredentialAutofillMatchPipeline,
    CredentialAutofillMatchRequest, CredentialAutofillMatchRequestKey, CredentialSuggestionState,
    DiagnosticsPathPromptKind, DiagnosticsPathPromptResult, DockerConfirmAction,
    DockerConfirmState, DockerTab, EffectiveTerminalPaintPolicy, GithubGistAuthEvent,
    GithubGistAuthJobEvent, GithubGistAuthState, KeywordHighlightEditorField,
    KeywordHighlightPathPromptKind, KeywordHighlightPathPromptResult, MainMode,
    MultiLinePasteDraft, NavItem, PanelResizeSide, PanelResizeState, PanelSide,
    PanelStackResizeState, PendingCredentialAutofill, QuickCommandCategoryDeleteState,
    QuickCommandCategoryMenuState, QuickCommandCategoryRenameState, QuickCommandDeleteState,
    QuickCommandDetailsState, QuickCommandEditorField, QuickCommandEditorState,
    QuickCommandImportPathPromptKind, QuickCommandImportPathPromptResult, QuickCommandRowMenuState,
    QuickCommandSortMode, QuickCommandVariableDef, QuickCommandVariablePromptState,
    QuickCommandViewMode, QuickSwitchItem, RecordingHistorySearchEvent, RecordingHistorySearchKey,
    RecordingPathPromptKind, RecordingPathPromptResult, RecordingWriteEvent,
    RecordingWritePipeline, RemoteProcessSignalConfirmState, RemoteProcessSortDirection,
    RemoteProcessSortKey, RightFocus, SearchEngineEditorField, SecurityAuthTab,
    SecurityCredentialEditorField, SecurityCredentialEditorState, SecurityDeleteConfirmState,
    SecurityKeyEditorField, SecurityKeyEditorState, SecurityOtpEditorField, SecurityOtpEditorState,
    SecurityPasswordEditorField, SecurityPasswordEditorState, SecurityUnlockAction,
    SessionEventBridge, SessionLaunchConfig, SessionRuntimeMetadata, SettingsTab, SmartSplitMode,
    SnapshotPasswordPromptKind, SnapshotPasswordPromptState, SplitEdge, StartupCommandAction,
    StartupCommandRequest, StoreStatus, SyncInputGroup, TERMINAL_UI_OUTPUT_TAIL_CAP,
    TabActionsSubmenu, TabDockZone, TerminalCellPos, TerminalContextMenuState,
    TerminalContextSubmenu, TerminalFrameActionLinks, TerminalFrameBufferTextEvent,
    TerminalFrameEvent, TerminalFrameOutputEvent, TerminalFrameOutputSubmission,
    TerminalFramePipeline, TerminalFrameSearchEvent, TerminalFrameSearchKey,
    TerminalFrameSnapshotEvent, TerminalPerformanceMode, TerminalPerformanceOverlay,
    TerminalProtocolState, TerminalSearchMode, TerminalSelection, TerminalViewState,
    TerminalWindowNode, TitleMenu, TitleMenuSubmenu, TransferBrowserColumnResizeState,
    TransferBrowserColumnWidths, TransferBrowserContextMenuState,
    TransferBrowserDragSelectionState, TransferBrowserFavoritesMenuState,
    TransferBrowserPendingRenameState, TransferBrowserSessionCacheState, TransferBrowserSortColumn,
    TransferBrowserSortDirection, TransferBrowserUploadMenuState, TransferDeleteState,
    TransferEditorWorkspaceState, TransferExternalSyncPromptState, TransferHeightResizeState,
    TransferInputField, TransferJobDeleteState, TransferJobEvent, TransferJobKind,
    TransferJobMenuState, TransferJobOutput, TransferJobResult, TransferJobState,
    TransferJobStatus, TransferMoveState, TransferNewFileState, TransferNewFolderState,
    TransferNewSymlinkState, TransferPathPromptKind, TransferPathPromptResult,
    TransferPropertiesState, TransferRenameState, TransferUnknownFileState, TranslateInputField,
    TranslationDialogState, TranslationSecretDraft, WorkspacePaneNode, WorkspaceSplitDirection,
    WorkspaceSplitResizeState, WorkspaceSplitState, is_multi_line_paste, normalize_paste_newlines,
    panel_collapsed_from_persistence, terminal_action_link_matcher_key,
    terminal_expensive_interactions_enabled, terminal_frame_search_result_is_current,
    terminal_snapshot_matches_grid_geometry,
};
pub(super) use crate::send_command::{
    SendCommandControlFocus, SendCommandDataType, SendCommandLineEnding, SendCommandMode,
    SendCommandTarget, build_send_command_units_for, format_send_command_hex_display,
    parse_send_command_hex,
};
pub(super) use crate::shortcuts::{event_to_hotkey_string, shortcut_matches};
pub(super) use crate::terminal::{
    NyaTerminalElement, NyaTerminalLayoutCache, TerminalBufferMatch, TerminalKeyMode,
    TerminalKeywordHighlightSnapshot, TerminalKeywordHighlighter, TerminalLineDecorations,
    TerminalTextCell, compile_terminal_keyword_highlighter, initial_terminal_screen,
    precompute_terminal_keyword_highlights_for_rows, terminal_byte_index_for_cell_col,
    terminal_is_zero_width_mark, terminal_key_bytes_with_mode,
    terminal_key_release_bytes_with_mode, terminal_keyword_highlight_expanded_rows,
    terminal_keyword_rules_key, terminal_text_cell_slice, terminal_text_cells,
};
pub(super) use crate::widgets::{
    capability_line, empty_panel, mode_button, session_info_row, small_button, status_pill,
    svg_icon_button,
};
