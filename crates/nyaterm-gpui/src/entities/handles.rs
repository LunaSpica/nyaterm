use gpui::Entity;

use super::{
    AiStore, CloudSyncStore, ConnectionsStore, OverlayStore, RemoteOpsStore, SessionStore,
    SettingsStore, StartupRestoreStore, TransferStore, WorkspaceStore,
};

#[derive(Clone)]
pub struct UiStoreHandles {
    pub startup_restore: Entity<StartupRestoreStore>,
    pub workspace: Entity<WorkspaceStore>,
    pub sessions: Entity<SessionStore>,
    pub overlays: Entity<OverlayStore>,
    pub settings: Entity<SettingsStore>,
    pub connections: Entity<ConnectionsStore>,
    pub transfers: Entity<TransferStore>,
    pub ai: Entity<AiStore>,
    pub cloud_sync: Entity<CloudSyncStore>,
    pub remote_ops: Entity<RemoteOpsStore>,
}
