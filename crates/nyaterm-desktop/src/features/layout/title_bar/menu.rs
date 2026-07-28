use gpui::{Context, IntoElement, SharedString, div, prelude::*, px, rgb};

use crate::features::NyaTermApp;
use crate::models::{SmartSplitMode, TitleMenu, TitleMenuSubmenu};

use super::super::title_menu_helpers::{
    title_menu_item, title_menu_separator, title_menu_submenu_trigger,
};

impl NyaTermApp {
    pub(in crate::features) fn title_menu_dropdown(
        &self,
        menu: TitleMenu,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let shortcut = |id: &str, fallback: &str| self.display_shortcut_for(id, fallback);
        let tr = |key: &'static str| self.tr(key);
        let palette = self.theme_palette();
        let mut items = div()
            .id(SharedString::from(format!("title-menu-{}", menu.label())))
            .absolute()
            .top(px(36.))
            .left_0()
            .w(px(220.))
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(self.shell_surface_color(palette.surface))
            .shadow_lg()
            .py_1()
            .flex()
            .flex_col();

        match menu {
            TitleMenu::File => {
                items = items
                    .child(title_menu_item(
                        palette,
                        "title-file-new-session",
                        Some("icons/conn/add.svg"),
                        false,
                        tr("menu.newSession"),
                        Some(shortcut("tab.newSession", "Ctrl+Shift+N")),
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.shell.chrome.open_tabs_menu_open = false;
                            this.shell.chrome.new_session_menu_open = false;
                            this.shell.chrome.new_session_all_sessions_open = false;
                            this.shell.chrome.new_session_group_menu_path.clear();
                            this.open_connection_editor(None, None, false, window, cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-file-import",
                        Some("icons/import.svg"),
                        false,
                        tr("settings.importConfig"),
                        None,
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.open_connection_import_dialog(window, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-file-export",
                        Some("icons/menu/export.svg"),
                        false,
                        tr("settings.exportConfig"),
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.prompt_config_export(cx);
                        }),
                    ));
            }
            TitleMenu::View => {
                items = items
                    .child(self.title_menu_submenu_trigger(
                        TitleMenuSubmenu::Theme,
                        "title-view-theme",
                        Some("icons/menu/palette.svg"),
                        tr("menu.theme"),
                        cx,
                    ))
                    .child(self.title_menu_submenu_trigger(
                        TitleMenuSubmenu::Language,
                        "title-view-language",
                        Some("icons/translation.svg"),
                        tr("menu.language"),
                        cx,
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-view-zoom-in",
                        Some("icons/menu/zoom-in.svg"),
                        false,
                        tr("menu.zoomIn"),
                        Some(shortcut("view.zoomIn", "Ctrl+=")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.zoom_terminal_in(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-zoom-out",
                        Some("icons/menu/zoom-out.svg"),
                        false,
                        tr("menu.zoomOut"),
                        Some(shortcut("view.zoomOut", "Ctrl+-")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.zoom_terminal_out(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-reset-zoom",
                        Some("icons/menu/reset.svg"),
                        false,
                        tr("menu.resetZoom"),
                        Some(shortcut("view.resetZoom", "Ctrl+0")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.reset_terminal_font_size(cx);
                        }),
                    ));
            }
            TitleMenu::Terminal => {
                items = items
                    .child(title_menu_item(
                        palette,
                        "title-term-command-palette",
                        Some("icons/fe/search.svg"),
                        false,
                        tr("menu.commandPalette"),
                        Some(shortcut("tab.quickSwitch", "Ctrl+Shift+S")),
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.open_quick_switch(window, cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(self.title_menu_submenu_trigger(
                        TitleMenuSubmenu::SmartSplit,
                        "title-term-smart-split",
                        Some("icons/menu/split.svg"),
                        tr("menu.smartSplit"),
                        cx,
                    ))
                    .child(title_menu_separator(palette))
                    .child(self.title_menu_submenu_trigger(
                        TitleMenuSubmenu::SyncInput,
                        "title-term-sync-input",
                        Some("icons/sync.svg"),
                        tr("menu.syncInput"),
                        cx,
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-term-broadcast",
                        Some("icons/menu/broadcast.svg"),
                        self.sync_input.broadcast_to_all,
                        tr("menu.broadcastToAll"),
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.toggle_broadcast_to_all(cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-term-clear",
                        Some("icons/fe/delete.svg"),
                        false,
                        tr("menu.clearTerminal"),
                        Some(shortcut("terminal.clear", "Ctrl+L")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.clear_terminal(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-reset-size",
                        Some("icons/menu/fit.svg"),
                        false,
                        tr("menu.resetTerminalSize"),
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            let changed = this.resize_all_known_terminal_surfaces();
                            this.terminal.view.status = if changed {
                                "terminal sizes reset".to_string()
                            } else {
                                "terminal sizes already current".to_string()
                            };
                            cx.notify();
                        }),
                    ));
            }
            TitleMenu::Help => {
                let update_label = if self.update.pending {
                    tr("updater.checking")
                } else if self.update.info.as_ref().is_some_and(|info| info.available) {
                    tr("updater.newVersionAvailable")
                } else {
                    tr("menu.checkForUpdates")
                };
                items = items
                    .child(title_menu_item(
                        palette,
                        "title-help-docs",
                        Some("icons/menu/book.svg"),
                        false,
                        tr("menu.documentation"),
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.open_documentation(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-help-update",
                        Some("icons/menu/update.svg"),
                        false,
                        update_label,
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.open_update_dialog(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-help-logs",
                        Some("icons/menu/article.svg"),
                        false,
                        tr("menu.viewLogs"),
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.reveal_log_dir(cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-help-about",
                        Some("icons/menu/info.svg"),
                        false,
                        format!("{} NyaTerm", tr("menu.about")),
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.open_about(cx);
                        }),
                    ));
            }
        }

        items.when_some(self.shell.chrome.title_menu_submenu, |this, submenu| {
            this.child(self.title_menu_submenu(submenu, cx))
        })
    }

    fn title_menu_submenu_trigger(
        &self,
        submenu: TitleMenuSubmenu,
        id: &'static str,
        icon: Option<&'static str>,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let open = self.shell.chrome.title_menu_submenu == Some(submenu);
        title_menu_submenu_trigger(
            palette,
            id,
            icon,
            label,
            open,
            cx.listener(move |this, hovered: &bool, _, cx| {
                if *hovered {
                    this.open_title_submenu(submenu, cx);
                }
            }),
            cx.listener(move |this, _, _, cx| {
                if this.shell.chrome.title_menu_submenu == Some(submenu) {
                    this.shell.chrome.title_menu_submenu = None;
                } else {
                    this.shell.chrome.title_menu_submenu = Some(submenu);
                }
                cx.notify();
            }),
        )
    }

    fn title_menu_submenu(
        &self,
        submenu: TitleMenuSubmenu,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let top = match submenu {
            TitleMenuSubmenu::Theme => 4.,
            TitleMenuSubmenu::Language => 34.,
            TitleMenuSubmenu::SmartSplit => 44.,
            TitleMenuSubmenu::SyncInput => 84.,
        };
        let mut menu = div()
            .id(SharedString::from(format!("title-submenu-{submenu:?}")))
            .absolute()
            .top(px(top))
            .left(px(224.))
            .w(px(220.))
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(self.shell_surface_color(palette.surface))
            .shadow_lg()
            .py_1()
            .flex()
            .flex_col();

        match submenu {
            TitleMenuSubmenu::Theme => {
                for &theme in crate::theme::APPEARANCE_THEME_IDS {
                    let current = self.settings.theme.as_str();
                    let selected = current == theme
                        || (current == "catppuccin" && theme == "catppuccin-mocha");
                    let label = crate::theme::appearance_theme_label(theme);
                    menu = menu.child(title_menu_item(
                        palette,
                        format!("title-theme-{theme}"),
                        None,
                        selected,
                        label,
                        None,
                        cx.listener(move |this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme(theme, cx);
                        }),
                    ));
                }
            }
            TitleMenuSubmenu::Language => {
                let english = matches!(self.settings.language.as_str(), "en" | "en-US");
                let chinese = matches!(self.settings.language.as_str(), "zh" | "zh-CN");
                menu = menu
                    .child(title_menu_item(
                        palette,
                        "title-language-en",
                        None,
                        english,
                        "English",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_ui_language("en", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-language-zh",
                        None,
                        chinese,
                        "中文",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_ui_language("zh-CN", cx);
                        }),
                    ));
            }
            TitleMenuSubmenu::SmartSplit => {
                menu = menu
                    .child(title_menu_item(
                        palette,
                        "title-smart-split-auto",
                        Some("icons/view-grid.svg"),
                        false,
                        self.tr("menu.autoTile"),
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.apply_smart_split(SmartSplitMode::Auto, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-smart-split-horizontal",
                        Some("icons/menu/horizontal.svg"),
                        false,
                        self.tr("menu.tileHorizontally"),
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.apply_smart_split(SmartSplitMode::Horizontal, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-smart-split-vertical",
                        Some("icons/menu/vertical.svg"),
                        false,
                        self.tr("menu.tileVertically"),
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.apply_smart_split(SmartSplitMode::Vertical, cx);
                        }),
                    ));
            }
            TitleMenuSubmenu::SyncInput => {
                menu = menu.child(title_menu_item(
                    palette,
                    "title-sync-manage-groups",
                    Some("icons/settings.svg"),
                    false,
                    self.tr("menu.manageGroups"),
                    Some(self.display_shortcut_for("terminal.manageSyncGroups", "Ctrl+Shift+G")),
                    cx.listener(|this, _, window, cx| {
                        this.close_title_menu(cx);
                        this.open_sync_groups(window, cx);
                    }),
                ));
            }
        }

        menu
    }
}
