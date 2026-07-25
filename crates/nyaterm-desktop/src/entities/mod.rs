//! GPUI entity-state boundaries for the native shell.
//!
//! During the migration, `NyaTermApp` and its feature-state structs remain the
//! default authoritative UI state. These stores mostly hold read-only snapshot
//! projections published from the app; explicitly migrated domains, such as the
//! quick switch state in `OverlayStore`, are the exceptions.

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
pub use overlay::{OverlaySnapshot, OverlayStore, QuickSwitchState};
pub use runtime::RuntimeStore;
pub use session::{SessionSnapshot, SessionStore};
pub use startup_restore::StartupRestoreStore;
pub use window_runtime::WindowRuntimeStore;
pub use workspace::{WorkspaceSnapshot, WorkspaceStore};
