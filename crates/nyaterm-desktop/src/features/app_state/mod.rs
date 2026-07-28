use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use gpui::WindowHandle;
use nyaterm_core::{
    AppRuntime, AppSettingsSummary, CommandHistoryEntry, Group, KeywordHighlightConfig,
    NativeServices, OtpEntry, ProxyConfig, ProxyGroup, QuickCommand, QuickCommandCategory,
    SavedConnection, SavedCredential, SavedPassword, SshKey, TunnelConfig, TunnelGroup,
};
use nyaterm_legacy::MigrationInventory;
use nyaterm_transport::SessionEvent;

use super::ai::AiFeatureState;
use super::commands::QuickCommandFeatureState;
use super::connections::ConnectionFeatureState;
use super::panels::SendCommandFeatureState;
use super::recording::RecordingFeatureState;
use super::remote::RemoteOpsFeatureState;
use super::remote_editor_window::RemoteFileEditorWindow;
use super::runtime_jobs::{CommandPersistenceRequest, CommandPersistenceResult};
use super::session::SessionFeatureState;
use super::settings::{SecurityFeatureState, SettingsFeatureState};
use super::shell::ShellFeatureState;
use super::sync::CloudSyncFeatureState;
use super::sync_input::SyncInputFeatureState;
use super::terminal::TerminalFeatureState;
use super::text_inputs::TextInputRegistry;
use super::transfers::TransferFeatureState;
use super::translation::TranslationFeatureState;
use super::tunnels::TunnelFeatureState;
use super::update::UpdateFeatureState;
use crate::models::{
    ConfigPathPromptKind, DiagnosticsPathPromptKind, KeywordHighlightPathPromptKind,
    SnapshotPasswordPromptState, StoreStatus,
};

mod construct;
mod types;

pub(in crate::features) use types::{
    PendingSavedConnectionStart, SavedConnectionStartOptions, SettingsDraftSnapshot,
    TerminalRuntimeUiState,
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
    pub(in crate::features) settings_state: SettingsFeatureState,
    pub(in crate::features) ai: AiFeatureState,
    pub(in crate::features) terminal: TerminalFeatureState,
    pub(in crate::features) send_command: SendCommandFeatureState,
    pub(in crate::features) transfer: TransferFeatureState,
    pub(in crate::features) translation: TranslationFeatureState,
    pub(in crate::features) update: UpdateFeatureState,
    pub(in crate::features) cloud_sync: CloudSyncFeatureState,
    pub(in crate::features) command_history: Arc<[CommandHistoryEntry]>,
    pub(in crate::features) command_persistence_tx: mpsc::Sender<CommandPersistenceRequest>,
    pub(in crate::features) command_persistence_rx: mpsc::Receiver<CommandPersistenceResult>,
    pub(in crate::features) command_persistence_pending: usize,
    pub(in crate::features) session: SessionFeatureState,
    pub(in crate::features) shell: ShellFeatureState,
    pub(in crate::features) sync_input: SyncInputFeatureState,
    pub(in crate::features) keyword_highlights: KeywordHighlightConfig,
    pub(in crate::features) settings: AppSettingsSummary,
    pub(in crate::features) settings_master_password_enabled: bool,
    pub(in crate::features) settings_master_password_draft: String,
    pub(in crate::features) store_status: StoreStatus,
    pub(in crate::features) recording: RecordingFeatureState,
    pub(in crate::features) tunnel_runtime: TunnelFeatureState,
    pub(in crate::features) about_open: bool,
    pub(in crate::features) remote_editor_window: Option<WindowHandle<RemoteFileEditorWindow>>,
    pub(in crate::features) remote_editor_window_open_pending: bool,
    pub(in crate::features) config_path_prompt: Option<ConfigPathPromptKind>,
    pub(in crate::features) diagnostics_path_prompt: Option<DiagnosticsPathPromptKind>,
    pub(in crate::features) keyword_highlight_path_prompt: Option<KeywordHighlightPathPromptKind>,
    pub(in crate::features) active_snapshot_password_prompt: Option<SnapshotPasswordPromptState>,
    pub(in crate::features) pending_session_events: VecDeque<SessionEvent>,
    pub(in crate::features) diagnostic_log_last_at: HashMap<&'static str, Instant>,
    pub(in crate::features) startup_restore_complete: bool,
}
