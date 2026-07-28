pub(super) use gpui::{
    Context, FocusHandle, ScrollHandle, Window, WindowHandle, prelude::*, px, svg,
};
pub(super) use nyaterm_core::{
    AiCommandCard, AiExecutionProfile, AiMode, AiSettings, AppRuntime, AppSettingsSummary,
    CloudSyncHistoryEntry, CloudSyncSettings, CloudSyncState, CommandHistoryEntry,
    CommandObservation, ConnectionStore, Group, KeywordHighlightConfig, NativeServices,
    NativeUpdateInfo, OtpEntry, ProxyConfig, ProxyGroup, QuickCommand, QuickCommandCategory,
    SavedConnection, SavedCredential, SavedPassword, SshKey, TerminalInputState, TunnelConfig,
    TunnelGroup, truncate_preview, uuid,
};
#[cfg(feature = "migration-dashboard")]
pub(super) use nyaterm_legacy::LegacyProject;
pub(super) use nyaterm_legacy::MigrationInventory;
pub(super) use nyaterm_terminal::TerminalOutputDecoder;
pub(super) use nyaterm_transport::{
    DockerComposeService, DockerContainerDetails, RecordingManager, RemoteDockerOverview,
    RemoteProcess, RemoteStats, SessionEvent, SessionInfo, SessionKind, SessionManager,
    SftpDuplicatePolicy, SshMultiplexHandle, SshSessionConfig, SshTunnelInfo, SshTunnelManager,
};

pub(super) use std::collections::{HashMap, HashSet, VecDeque};
pub(super) use std::path::PathBuf;
pub(super) use std::sync::{Arc, atomic::AtomicBool, mpsc};
pub(super) use std::time::{Duration, Instant};

pub(super) use crate::models::{
    CloudSyncInputField, CredentialAutofillMatchPipeline, CredentialAutofillMatchRequestKey,
    NavItem, PanelSide, SessionLaunchConfig, TransferJobResult, panel_collapsed_from_persistence,
};
pub(super) use crate::terminal::initial_terminal_screen;
pub(super) use crate::widgets::small_button;
