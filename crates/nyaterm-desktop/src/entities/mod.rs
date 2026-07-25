//! GPUI entity-state boundaries for the native shell.
//!
//! During the migration, `NyaTermApp` and its feature-state structs remain the
//! authoritative UI state. These stores hold read-only snapshot projections
//! published from the app; mutation helpers are retained for focused store tests
//! until a specific domain explicitly migrates ownership to an Entity.

mod domain;
mod handles;
mod overlay;
mod runtime;
mod session;
mod startup_restore;
mod window_runtime;
mod workspace;

#[cfg(test)]
mod tests;

pub use domain::{
    AiSnapshot, AiStore, CloudSyncSnapshot, CloudSyncStore, ConnectionsSnapshot, ConnectionsStore,
    RemoteOpsSnapshot, RemoteOpsStore, SettingsSnapshot, SettingsStore, TransferSnapshot,
    TransferStore,
};
pub use handles::UiStoreHandles;
pub use overlay::{OverlaySnapshot, OverlayStore};
pub use runtime::RuntimeStore;
pub use session::{SessionSnapshot, SessionStore};
pub use startup_restore::StartupRestoreStore;
pub use window_runtime::WindowRuntimeStore;
pub use workspace::{WorkspaceSnapshot, WorkspaceStore};
