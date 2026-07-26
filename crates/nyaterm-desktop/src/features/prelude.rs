pub(super) use gpui::{
    AnimationExt, AnyElement, App, ClickEvent, ClipboardItem, Context, Entity, FocusHandle,
    FontWeight, IntoElement, KeyDownEvent, KeyUpEvent, MouseButton, MouseDownEvent,
    PathPromptOptions, Render, ScrollDelta, ScrollHandle, ScrollWheelEvent, SharedString,
    Subscription, Timer, Window, WindowControlArea, WindowHandle, div, prelude::*, px, rgb, rgba,
    svg,
};
pub(super) use nyaterm_core::{
    AgentCommandExecutionMode, AgentOutputCaptureProcessor, AiAction, AiChatRequest, AiCommandCard,
    AiExecutionProfile, AiMessage, AiMessageRole, AiMode, AiProviderCredential, AiProviderKind,
    AiSettings, AppRuntime, AppSettingsSummary, CloudSyncError, CloudSyncHistoryEntry,
    CloudSyncSettings, CloudSyncState, CommandHistoryEntry, CommandObservation, ConnectionAuth,
    ConnectionStore, ConnectionType, CredentialPromptKind, DecryptedOtpEntry, Group,
    InputSelectionRange, KeywordHighlightConfig, KnownHostCheck, NativeServices, NativeUpdateInfo,
    OtpEntry, ProxyConfig, ProxyGroup, QuickCommand, QuickCommandCategory, QuickCommandsConfig,
    RiskLevel, RuntimeMode, SavedConnection, SavedCredential, SavedPassword, SshKey,
    TerminalInputState, TerminalWireWriteKind, TunnelConfig, TunnelGroup,
    apply_terminal_input_data, build_move_input_cursor_data, can_suggest_from_tracked_command,
    can_suggest_from_tracker, command_starts_suggestion_suppressing_program,
    delete_terminal_input_range, export_diagnostics_archive, get_tracked_command,
    get_tracked_submission_command, now_rfc3339, resync_from_terminal_line, search_command_sources,
    terminal_input_fanout_status, terminal_input_tracker_below_min_chars,
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

pub(super) use std::collections::{HashMap, HashSet, VecDeque};
pub(super) use std::path::PathBuf;
pub(super) use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};
pub(super) use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(super) use crate::models::{
    CloudSyncInputField, CredentialAutofillMatchEvent, CredentialAutofillMatchOutcome,
    CredentialAutofillMatchPipeline, CredentialAutofillMatchRequest,
    CredentialAutofillMatchRequestKey, NavItem, PanelSide, SessionLaunchConfig,
    TERMINAL_UI_OUTPUT_TAIL_CAP, TerminalFrameActionLinks, TerminalFrameEvent,
    TerminalFramePipeline, TerminalFrameSearchKey, TerminalFrameSnapshotEvent, TerminalViewState,
    TransferJobEvent, TransferJobKind, TransferJobOutput, TransferJobResult, TransferJobState,
    TransferJobStatus, is_multi_line_paste, normalize_paste_newlines,
    panel_collapsed_from_persistence, terminal_action_link_matcher_key,
    terminal_expensive_interactions_enabled, terminal_frame_search_result_is_current,
    terminal_snapshot_matches_grid_geometry,
};
pub(super) use crate::send_command::{
    build_send_command_units_for, format_send_command_hex_display, parse_send_command_hex,
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
