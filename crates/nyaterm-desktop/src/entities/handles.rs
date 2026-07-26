use gpui::Entity;

use super::{OverlayStore, SessionStore, StartupRestoreStore, WorkspaceStore};

#[derive(Clone)]
pub struct UiStoreHandles {
    pub startup_restore: Entity<StartupRestoreStore>,
    pub workspace: Entity<WorkspaceStore>,
    pub sessions: Entity<SessionStore>,
    pub overlays: Entity<OverlayStore>,
}
