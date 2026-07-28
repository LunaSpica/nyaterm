//! Root GPUI shell boundary.

use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window,
    div,
};
use nyaterm_core::AppRuntime;

use crate::{
    entities::{OverlayStore, StartupRestoreStore, UiStoreHandles, WindowRuntimeStore},
    features::NyaTermApp,
};

#[allow(dead_code)]
pub struct AppShell {
    app: Entity<NyaTermApp>,
    window_runtime: Entity<WindowRuntimeStore>,
    startup_restore: Entity<StartupRestoreStore>,
    overlays: Entity<OverlayStore>,
    _subscriptions: Vec<Subscription>,
}

impl AppShell {
    pub fn new(runtime: AppRuntime, cx: &mut Context<Self>) -> Self {
        let startup_restore = cx.new(|_| StartupRestoreStore::default());
        let overlays = cx.new(|_| OverlayStore::default());
        let stores = UiStoreHandles {
            startup_restore: startup_restore.clone(),
            overlays: overlays.clone(),
        };
        let app = cx.new(|cx| NyaTermApp::new(runtime, stores, cx));
        // Do not observe UI stores for parent notify: AppShell only hosts the
        // NyaTermApp entity, and NyaTermApp already cx.notify()s on visual dirty.
        // Store observe → AppShell notify was amplifying every snapshot publish
        // into an extra shell paint (connect bursts, sideband heartbeats, drag).
        let subscriptions = Vec::new();

        Self {
            app,
            window_runtime: cx.new(|_| WindowRuntimeStore::default()),
            startup_restore,
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
