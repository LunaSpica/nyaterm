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
    features::NyaTermApp,
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
        let settings = cx.new(|_| SettingsStore::default());
        let connections = cx.new(|_| ConnectionsStore::default());
        let transfers = cx.new(|_| TransferStore::default());
        let ai = cx.new(|_| AiStore::default());
        let cloud_sync = cx.new(|_| CloudSyncStore::default());
        let remote_ops = cx.new(|_| RemoteOpsStore::default());
        let stores = UiStoreHandles {
            startup_restore: startup_restore.clone(),
            workspace: workspace.clone(),
            sessions: sessions.clone(),
            overlays: overlays.clone(),
            settings: settings.clone(),
            connections: connections.clone(),
            transfers: transfers.clone(),
            ai: ai.clone(),
            cloud_sync: cloud_sync.clone(),
            remote_ops: remote_ops.clone(),
        };
        let app = cx.new(|cx| NyaTermApp::new(runtime, stores, cx));
        // Do not observe UI stores for parent notify: AppShell only hosts the
        // NyaTermApp entity, and NyaTermApp already cx.notify()s on visual dirty.
        // Store observe → AppShell notify was amplifying every snapshot publish
        // into an extra shell paint (connect bursts, sideband heartbeats, drag).
        let subscriptions = Vec::new();

        Self {
            app,
            runtime: runtime_store,
            window_runtime: cx.new(|_| WindowRuntimeStore::default()),
            startup_restore,
            workspace,
            sessions,
            settings,
            connections,
            transfers,
            ai,
            cloud_sync,
            remote_ops,
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
