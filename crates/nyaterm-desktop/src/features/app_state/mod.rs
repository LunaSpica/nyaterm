use nyaterm_core::{AppRuntime, AppSettingsSummary, KeywordHighlightConfig, NativeServices};
use nyaterm_legacy::MigrationInventory;

use super::ai::AiFeatureState;
use super::commands::CommandFeatureState;
use super::connections::{ConnectionCatalogState, ConnectionFeatureState};
use super::panels::SendCommandFeatureState;
use super::recording::RecordingFeatureState;
use super::remote::RemoteOpsFeatureState;
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
use crate::models::StoreStatus;

mod construct;
mod types;

pub(in crate::features) use types::{SettingsDraftSnapshot, TerminalRuntimeUiState};

pub struct NyaTermApp {
    pub(in crate::features) stores: crate::entities::UiStoreHandles,
    pub(in crate::features) runtime: AppRuntime,
    pub(in crate::features) services: NativeServices,
    pub(in crate::features) inventory: MigrationInventory,
    pub(in crate::features) connection_catalog: ConnectionCatalogState,
    pub(in crate::features) connection_state: ConnectionFeatureState,
    /// Real text inputs for the panels that have not been given their own,
    /// keyed by an id the panel picks. See `features::text_inputs`.
    pub(in crate::features) text_inputs: TextInputRegistry,
    pub(in crate::features) commands: CommandFeatureState,
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
    pub(in crate::features) session: SessionFeatureState,
    pub(in crate::features) shell: ShellFeatureState,
    pub(in crate::features) sync_input: SyncInputFeatureState,
    pub(in crate::features) keyword_highlights: KeywordHighlightConfig,
    pub(in crate::features) settings: AppSettingsSummary,
    pub(in crate::features) settings_master_password_enabled: bool,
    pub(in crate::features) settings_master_password_draft: String,
    pub(in crate::features) store_status: StoreStatus,
    pub(in crate::features) recording: RecordingFeatureState,
    pub(in crate::features) tunnel_state: TunnelFeatureState,
}
