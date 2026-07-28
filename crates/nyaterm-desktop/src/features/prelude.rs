pub(super) use gpui::{
    App, ClickEvent, ClipboardItem, Context, Entity, FocusHandle, FontWeight, IntoElement,
    KeyDownEvent, KeyUpEvent, MouseButton, Render, ScrollDelta, ScrollHandle, ScrollWheelEvent,
    SharedString, Timer, Window, WindowHandle, div, prelude::*, px, rgb, rgba, svg,
};
pub(super) use nyaterm_core::{
    AgentOutputCaptureProcessor, AiCommandCard, AiExecutionProfile, AiMode, AiSettings, AppRuntime,
    AppSettingsSummary, CloudSyncHistoryEntry, CloudSyncSettings, CloudSyncState,
    CommandHistoryEntry, CommandObservation, ConnectionStore, Group, InputSelectionRange,
    KeywordHighlightConfig, NativeServices, NativeUpdateInfo, OtpEntry, ProxyConfig, ProxyGroup,
    QuickCommand, QuickCommandCategory, SavedConnection, SavedCredential, SavedPassword, SshKey,
    TerminalInputState, TerminalWireWriteKind, TunnelConfig, TunnelGroup,
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
    DockerComposeService, DockerContainerDetails, RecordingManager, RemoteDockerOverview,
    RemoteProcess, RemoteStats, SessionEvent, SessionInfo, SessionKind, SessionManager,
    SftpDuplicatePolicy, SshMultiplexHandle, SshSessionConfig, SshTunnelInfo, SshTunnelManager,
};

pub(super) use std::collections::{HashMap, HashSet, VecDeque};
pub(super) use std::path::PathBuf;
pub(super) use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};
pub(super) use std::time::{Duration, Instant};

pub(super) use crate::models::{
    CloudSyncInputField, CredentialAutofillMatchPipeline, CredentialAutofillMatchRequestKey,
    NavItem, PanelSide, SessionLaunchConfig, TerminalFrameActionLinks, TerminalViewState,
    TransferJobResult, is_multi_line_paste, normalize_paste_newlines,
    panel_collapsed_from_persistence,
};
pub(super) use crate::terminal::{
    NyaTerminalElement, NyaTerminalLayoutCache, TerminalBufferMatch, TerminalKeyMode,
    TerminalKeywordHighlightSnapshot, TerminalKeywordHighlighter, TerminalLineDecorations,
    TerminalTextCell, compile_terminal_keyword_highlighter, initial_terminal_screen,
    precompute_terminal_keyword_highlights_for_rows, terminal_byte_index_for_cell_col,
    terminal_is_zero_width_mark, terminal_key_bytes_with_mode,
    terminal_key_release_bytes_with_mode, terminal_keyword_highlight_expanded_rows,
    terminal_keyword_rules_key, terminal_text_cell_slice, terminal_text_cells,
};
pub(super) use crate::widgets::small_button;
