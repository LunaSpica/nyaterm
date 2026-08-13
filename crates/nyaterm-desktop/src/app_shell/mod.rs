//! Root GPUI shell boundary.

use gpui::{
    AppContext, Context, Entity, InteractiveElement, IntoElement, Menu, MenuItem, OsAction,
    ParentElement, Render, Styled, Subscription, SystemMenuType, WeakEntity, Window, actions, div,
    px,
};
use nyaterm_core::AppRuntime;
use nyaterm_ui::{NyaAppMenu, NyaAppMenuBar, NyaCut, NyaRedo, NyaUndo};

use crate::{
    entities::{OverlayStore, StartupRestoreStore, UiStoreHandles, WindowRuntimeStore},
    features::NyaTermApp,
};

actions!(
    nyaterm_native_menu,
    [
        NativeNewSession,
        NativeNewLocalTerminal,
        NativeQuickSwitch,
        NativeOpenSettings,
        NativeToggleLeftSidebar,
        NativeToggleRightSidebar,
        NativeZoomIn,
        NativeZoomOut,
        NativeResetZoom,
        NativeTerminalCopy,
        NativeTerminalPaste,
        NativeTerminalFind,
        NativeTerminalClear,
        NativeTerminalSelectAll,
        NativeManageSyncGroups,
        NativeQuit
    ]
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeMenuCommand {
    NewSession,
    NewLocalTerminal,
    QuickSwitch,
    OpenSettings,
    ToggleLeftSidebar,
    ToggleRightSidebar,
    ZoomIn,
    ZoomOut,
    ResetZoom,
    TerminalCopy,
    TerminalPaste,
    TerminalFind,
    TerminalClear,
    TerminalSelectAll,
    ManageSyncGroups,
}

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
        install_native_app_menus(cx);
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

    fn perform_native_menu_command(
        &mut self,
        command: NativeMenuCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.app.update(cx, |app, cx| {
            app.perform_native_menu_command(command, window, cx);
        });
    }
}

fn install_native_app_menus(cx: &mut Context<AppShell>) {
    if !cfg!(target_os = "macos") {
        return;
    }
    cx.set_menus([
        Menu::new("NyaTerm").items([
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Quit NyaTerm", NativeQuit),
        ]),
        Menu::new("File").items([
            MenuItem::action("New Session", NativeNewSession),
            MenuItem::action("New Local Terminal", NativeNewLocalTerminal),
        ]),
        Menu::new("Edit").items([
            MenuItem::os_action("Undo", NyaUndo, OsAction::Undo),
            MenuItem::os_action("Redo", NyaRedo, OsAction::Redo),
            MenuItem::separator(),
            MenuItem::os_action("Cut", NyaCut, OsAction::Cut),
            // Copy/Paste/Select All retain terminal-aware handlers while also
            // advertising their standard macOS selectors to focused controls.
            MenuItem::os_action("Copy", NativeTerminalCopy, OsAction::Copy),
            MenuItem::os_action("Paste", NativeTerminalPaste, OsAction::Paste),
            MenuItem::os_action("Select All", NativeTerminalSelectAll, OsAction::SelectAll),
        ]),
        Menu::new("View").items([
            MenuItem::action("Settings", NativeOpenSettings),
            MenuItem::separator(),
            MenuItem::action("Toggle Left Sidebar", NativeToggleLeftSidebar),
            MenuItem::action("Toggle Right Sidebar", NativeToggleRightSidebar),
            MenuItem::separator(),
            MenuItem::action("Zoom In", NativeZoomIn),
            MenuItem::action("Zoom Out", NativeZoomOut),
            MenuItem::action("Reset Zoom", NativeResetZoom),
        ]),
        Menu::new("Terminal").items([
            MenuItem::action("Command Palette", NativeQuickSwitch),
            MenuItem::separator(),
            MenuItem::action("Copy", NativeTerminalCopy),
            MenuItem::action("Paste", NativeTerminalPaste),
            MenuItem::action("Find", NativeTerminalFind),
            MenuItem::action("Clear", NativeTerminalClear),
            MenuItem::action("Select All", NativeTerminalSelectAll),
            MenuItem::separator(),
            MenuItem::action("Manage Sync Groups", NativeManageSyncGroups),
        ]),
    ]);
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .on_action(cx.listener(|this, _: &NativeNewSession, window, cx| {
                this.perform_native_menu_command(NativeMenuCommand::NewSession, window, cx);
            }))
            .on_action(cx.listener(|this, _: &NativeNewLocalTerminal, window, cx| {
                this.perform_native_menu_command(NativeMenuCommand::NewLocalTerminal, window, cx);
            }))
            .on_action(cx.listener(|this, _: &NativeQuickSwitch, window, cx| {
                this.perform_native_menu_command(NativeMenuCommand::QuickSwitch, window, cx);
            }))
            .on_action(cx.listener(|this, _: &NativeOpenSettings, window, cx| {
                this.perform_native_menu_command(NativeMenuCommand::OpenSettings, window, cx);
            }))
            .on_action(
                cx.listener(|this, _: &NativeToggleLeftSidebar, window, cx| {
                    this.perform_native_menu_command(
                        NativeMenuCommand::ToggleLeftSidebar,
                        window,
                        cx,
                    );
                }),
            )
            .on_action(
                cx.listener(|this, _: &NativeToggleRightSidebar, window, cx| {
                    this.perform_native_menu_command(
                        NativeMenuCommand::ToggleRightSidebar,
                        window,
                        cx,
                    );
                }),
            )
            .on_action(cx.listener(|this, _: &NativeZoomIn, window, cx| {
                this.perform_native_menu_command(NativeMenuCommand::ZoomIn, window, cx);
            }))
            .on_action(cx.listener(|this, _: &NativeZoomOut, window, cx| {
                this.perform_native_menu_command(NativeMenuCommand::ZoomOut, window, cx);
            }))
            .on_action(cx.listener(|this, _: &NativeResetZoom, window, cx| {
                this.perform_native_menu_command(NativeMenuCommand::ResetZoom, window, cx);
            }))
            .on_action(cx.listener(|this, _: &NativeTerminalCopy, window, cx| {
                this.perform_native_menu_command(NativeMenuCommand::TerminalCopy, window, cx);
            }))
            .on_action(cx.listener(|this, _: &NativeTerminalPaste, window, cx| {
                this.perform_native_menu_command(NativeMenuCommand::TerminalPaste, window, cx);
            }))
            .on_action(cx.listener(|this, _: &NativeTerminalFind, window, cx| {
                this.perform_native_menu_command(NativeMenuCommand::TerminalFind, window, cx);
            }))
            .on_action(cx.listener(|this, _: &NativeTerminalClear, window, cx| {
                this.perform_native_menu_command(NativeMenuCommand::TerminalClear, window, cx);
            }))
            .on_action(
                cx.listener(|this, _: &NativeTerminalSelectAll, window, cx| {
                    this.perform_native_menu_command(
                        NativeMenuCommand::TerminalSelectAll,
                        window,
                        cx,
                    );
                }),
            )
            .on_action(cx.listener(|this, _: &NativeManageSyncGroups, window, cx| {
                this.perform_native_menu_command(NativeMenuCommand::ManageSyncGroups, window, cx);
            }))
            .on_action(|_: &NativeQuit, _window, cx| {
                cx.quit();
            })
            .child(self.app.clone())
    }
}
