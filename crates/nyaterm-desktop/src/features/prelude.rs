pub(super) use gpui::{
    AnimationExt, AnyElement, App, ClickEvent, ClipboardItem, Context, Entity, FocusHandle,
    FontWeight, IntoElement, KeyDownEvent, KeyUpEvent, MouseButton, MouseDownEvent,
    PathPromptOptions, Render, ScrollDelta, ScrollHandle, ScrollWheelEvent, SharedString, Timer,
    Window, WindowControlArea, WindowHandle, div, prelude::*, px, rgb, rgba, svg,
};
pub(super) use nyaterm_core::{
    AgentCommandExecutionMode, AgentOutputCaptureProcessor, AiAction, AiCommandCard,
    AiExecutionProfile, AiMessage, AiMessageRole, AiMode, AiSettings, AppRuntime,
    AppSettingsSummary, CloudSyncError, CloudSyncHistoryEntry, CloudSyncSettings, CloudSyncState,
    CommandHistoryEntry, CommandObservation, ConnectionAuth, ConnectionStore, ConnectionType,
    CredentialPromptKind, DecryptedOtpEntry, Group, InputSelectionRange, KeywordHighlightConfig,
    KnownHostCheck, NativeServices, NativeUpdateInfo, OtpEntry, ProxyConfig, ProxyGroup,
    QuickCommand, QuickCommandCategory, RuntimeMode, SavedConnection, SavedCredential,
    SavedPassword, SshKey, TerminalInputState, TerminalWireWriteKind, TunnelConfig, TunnelGroup,
    apply_terminal_input_data, build_move_input_cursor_data, can_suggest_from_tracker,
    delete_terminal_input_range, terminal_input_fanout_status, terminal_wire_write_disposition,
    truncate_preview, uuid,
};
#[cfg(feature = "migration-dashboard")]
pub(super) use nyaterm_legacy::LegacyProject;
pub(super) use nyaterm_legacy::MigrationInventory;
pub(super) use nyaterm_terminal::{
    TerminalEffects, TerminalOutputDecoder, TerminalScreen, TerminalSnapshot,
};
pub(super) use nyaterm_transport::{
    DockerComposeService, DockerContainerDetails, DockerService, LocalSessionConfig,
    RecordingManager, RemoteDockerOverview, RemoteProcess, RemoteStats, SerialSessionConfig,
    SessionEvent, SessionInfo, SessionKind, SessionManager, SftpDuplicateDecision,
    SftpDuplicatePolicy, SftpDuplicateRequest, SftpDuplicateResolver, SftpFileEntry, SftpService,
    SftpTransferControl, SftpTransferOptions, SftpTransferProgress, SshCredentialPrompt,
    SshCredentialPromptKind, SshCredentialPromptReason, SshCredentialProvider, SshHostKey,
    SshHostKeyDecision, SshHostKeyVerifier, SshKeyAuthConfig, SshKeyboardInteractiveRequest,
    SshMultiplexHandle, SshOtpProvider, SshProcessService, SshProxyConfig, SshSessionConfig,
    SshTunnelInfo, SshTunnelManager, TelnetSessionConfig, TerminalHistorySearchRequest,
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
    TerminalFrameActionLinks, TerminalViewState, TransferJobEvent, TransferJobKind,
    TransferJobOutput, TransferJobResult, TransferJobState, TransferJobStatus, is_multi_line_paste,
    normalize_paste_newlines, panel_collapsed_from_persistence,
};
pub(super) use crate::send_command::{
    build_send_command_units_for, format_send_command_hex_display, parse_send_command_hex,
};
pub(super) use crate::shortcuts::event_to_hotkey_string;
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
