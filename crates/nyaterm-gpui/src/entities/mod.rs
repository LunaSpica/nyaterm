//! GPUI entity-state boundaries for the native shell.

mod runtime;
mod window_runtime;
mod startup_restore;
mod workspace;
mod session;
mod overlay;
mod domain;
mod handles;

#[cfg(test)]
mod tests;

pub use runtime::RuntimeStore;
pub use window_runtime::WindowRuntimeStore;
pub use startup_restore::StartupRestoreStore;
pub use workspace::{WorkspaceSnapshot, WorkspaceStore};
pub use session::{SessionSnapshot, SessionStore};
pub use overlay::{OverlaySnapshot, OverlayStore};
pub use domain::{
    AiSnapshot, AiStore, CloudSyncSnapshot, CloudSyncStore, ConnectionsSnapshot, ConnectionsStore,
    RemoteOpsSnapshot, RemoteOpsStore, SettingsSnapshot, SettingsStore, TransferSnapshot,
    TransferStore,
};
pub use handles::UiStoreHandles;
