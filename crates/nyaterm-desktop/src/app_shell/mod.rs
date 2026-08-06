//! Root GPUI shell boundary.

use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription,
    WeakEntity, Window, div, px,
};
use nyaterm_core::AppRuntime;
use nyaterm_ui::{NyaAppMenu, NyaAppMenuBar};

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
        let title_menu_bar = build_title_menu_bar(app.downgrade(), cx);
        app.update(cx, |app, _| app.set_title_menu_bar(title_menu_bar));
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

fn build_title_menu_bar(
    app: WeakEntity<NyaTermApp>,
    cx: &mut Context<AppShell>,
) -> Entity<NyaAppMenuBar> {
    use crate::models::TitleMenu;

    let menus = [
        TitleMenu::File,
        TitleMenu::View,
        TitleMenu::Terminal,
        TitleMenu::Help,
    ]
    .into_iter()
    .map(|menu| {
        let label_app = app.clone();
        let items_app = app.clone();
        let open_app = app.clone();
        NyaAppMenu::new(
            menu.label(),
            move |cx| {
                label_app
                    .read_with(cx, |app, _| app.title_menu_label(menu).into())
                    .unwrap_or_else(|_| menu.label().into())
            },
            move |_, cx| {
                items_app
                    .update(cx, |app, cx| app.build_title_menu_items(menu, cx))
                    .unwrap_or_default()
            },
        )
        .min_width(px(220.))
        .on_open(move |_, cx| {
            _ = open_app.update(cx, |app, cx| app.prepare_title_menu(cx));
        })
    })
    .collect::<Vec<_>>();
    NyaAppMenuBar::new(menus, cx)
}

impl Render for AppShell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.app.clone())
    }
}
