//! GPUI entity-state boundaries for the native shell.
//!
//! `NyaTermApp` and its feature-state structs are the authoritative UI state.
//! Every store here owns something the app does not: the app runtime and native
//! services, the window runtime pump, the startup-restore queue, and the quick
//! switch overlay state. Read-only snapshot projections used to live here too;
//! they were removed once it turned out nothing consumed them.

mod handles;
mod overlay;
mod runtime;
mod startup_restore;
mod window_runtime;

#[cfg(test)]
mod tests;

pub use handles::UiStoreHandles;
pub use overlay::{OverlayStore, QuickSwitchState};
pub use runtime::RuntimeStore;
pub use startup_restore::StartupRestoreStore;
pub use window_runtime::WindowRuntimeStore;
