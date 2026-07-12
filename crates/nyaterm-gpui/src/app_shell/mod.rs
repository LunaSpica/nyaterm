//! Root GPUI shell boundary.

use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window,
    div,
};
use nyaterm_core::AppRuntime;

use crate::{
    entities::{
        AiStore, CloudSyncStore, ConnectionsStore, OverlayStore, RemoteOpsStore, RuntimeStore,
        SessionStore, SettingsStore, StartupRestoreStore, TransferStore, UiStoreHandles,
        WindowRuntimeStore, WorkspaceStore,
    },
    ui::NyaTermApp,
};

#[allow(dead_code)]
pub struct AppShell {
    app: Entity<NyaTermApp>,
    runtime: Entity<RuntimeStore>,
    window_runtime: Entity<WindowRuntimeStore>,
    startup_restore: Entity<StartupRestoreStore>,
    workspace: Entity<WorkspaceStore>,
    sessions: Entity<SessionStore>,
    settings: Entity<SettingsStore>,
    connections: Entity<ConnectionsStore>,
    transfers: Entity<TransferStore>,
    ai: Entity<AiStore>,
    cloud_sync: Entity<CloudSyncStore>,
    remote_ops: Entity<RemoteOpsStore>,
    overlays: Entity<OverlayStore>,
    _subscriptions: Vec<Subscription>,
}

impl AppShell {
    pub fn new(runtime: AppRuntime, cx: &mut Context<Self>) -> Self {
        let runtime_store = cx.new(|_| RuntimeStore::new(runtime.clone()));
        let startup_restore = cx.new(|_| StartupRestoreStore::default());
        let workspace = cx.new(|_| WorkspaceStore::default());
        let sessions = cx.new(|_| SessionStore::default());
        let overlays = cx.new(|_| OverlayStore::default());
        let stores = UiStoreHandles {
            startup_restore: startup_restore.clone(),
            workspace: workspace.clone(),
            sessions: sessions.clone(),
            overlays: overlays.clone(),
        };
        let app = cx.new(|cx| NyaTermApp::new(runtime, stores, cx));
        let subscriptions = vec![
            cx.observe(&startup_restore, |_, _, cx| cx.notify()),
            cx.observe(&workspace, |_, _, cx| cx.notify()),
            cx.observe(&sessions, |_, _, cx| cx.notify()),
            cx.observe(&overlays, |_, _, cx| cx.notify()),
        ];

        Self {
            app,
            runtime: runtime_store,
            window_runtime: cx.new(|_| WindowRuntimeStore::default()),
            startup_restore,
            workspace,
            sessions,
            settings: cx.new(|_| SettingsStore::default()),
            connections: cx.new(|_| ConnectionsStore::default()),
            transfers: cx.new(|_| TransferStore::default()),
            ai: cx.new(|_| AiStore::default()),
            cloud_sync: cx.new(|_| CloudSyncStore::default()),
            remote_ops: cx.new(|_| RemoteOpsStore::default()),
            overlays,
            _subscriptions: subscriptions,
        }
    }

    pub fn start_after_window_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let should_start_restore = self.startup_restore.update(cx, |store, cx| {
            if store.mark_started_after_window_open() {
                cx.notify();
                true
            } else {
                false
            }
        });
        if should_start_restore {
            self.app.update(cx, |app, cx| {
                app.start_after_window_open(window, cx);
            });
        }

        self.window_runtime.update(cx, |store, cx| {
            if store.ensure_started(window, cx, self.app.clone()) {
                cx.notify();
            }
        });
    }
}

impl Render for AppShell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.app.clone())
    }
}
