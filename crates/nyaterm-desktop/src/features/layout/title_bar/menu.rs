use gpui::Context;
use nyaterm_ui::{NyaDialogWindowExt as _, NyaMenuItem};

use crate::features::NyaTermApp;
use crate::models::{SmartSplitMode, TitleMenu};

impl NyaTermApp {
    pub(in crate::features) fn title_menu_items(
        &self,
        menu: TitleMenu,
        cx: &mut Context<Self>,
    ) -> Vec<NyaMenuItem> {
        match menu {
            TitleMenu::File => self.title_file_menu_items(cx),
            TitleMenu::View => self.title_view_menu_items(cx),
            TitleMenu::Terminal => self.title_terminal_menu_items(cx),
            TitleMenu::Help => self.title_help_menu_items(cx),
        }
    }

    fn title_file_menu_items(&self, cx: &mut Context<Self>) -> Vec<NyaMenuItem> {
        vec![
            NyaMenuItem::action(self.tr("menu.newSession"))
                .icon("icons/conn/add.svg")
                .shortcut(self.display_shortcut_for("tab.newSession", "Ctrl+Shift+N"))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.shell.close_open_tabs_menu();
                    this.shell.close_new_session_menu();
                    this.open_connection_editor(None, None, false, window, cx);
                })),
            NyaMenuItem::separator(),
            NyaMenuItem::action(self.tr("settings.importConfig"))
                .icon("icons/import.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    window.close_nya_dialog(cx);
                    this.open_connection_import_dialog(window, cx);
                })),
            NyaMenuItem::action(self.tr("settings.exportConfig"))
                .icon("icons/menu/export.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    window.close_nya_dialog(cx);
                    this.prompt_encrypted_portable_snapshot_export(window, cx);
                })),
        ]
    }

    fn title_view_menu_items(&self, cx: &mut Context<Self>) -> Vec<NyaMenuItem> {
        vec![
            NyaMenuItem::submenu(self.tr("menu.theme"), self.title_theme_menu_items(cx))
                .icon("icons/menu/palette.svg"),
            NyaMenuItem::submenu(self.tr("menu.language"), self.title_language_menu_items(cx))
                .icon("icons/translation.svg"),
            NyaMenuItem::separator(),
            NyaMenuItem::action(self.tr("menu.zoomIn"))
                .icon("icons/menu/zoom-in.svg")
                .shortcut(self.display_shortcut_for("view.zoomIn", "Ctrl+="))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.zoom_terminal_in(cx);
                })),
            NyaMenuItem::action(self.tr("menu.zoomOut"))
                .icon("icons/menu/zoom-out.svg")
                .shortcut(self.display_shortcut_for("view.zoomOut", "Ctrl+-"))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.zoom_terminal_out(cx);
                })),
            NyaMenuItem::action(self.tr("menu.resetZoom"))
                .icon("icons/menu/reset.svg")
                .shortcut(self.display_shortcut_for("view.resetZoom", "Ctrl+0"))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.reset_terminal_font_size(cx);
                })),
        ]
    }

    fn title_terminal_menu_items(&self, cx: &mut Context<Self>) -> Vec<NyaMenuItem> {
        vec![
            NyaMenuItem::action(self.tr("menu.commandPalette"))
                .icon("icons/fe/search.svg")
                .shortcut(self.display_shortcut_for("tab.quickSwitch", "Ctrl+Shift+S"))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_quick_switch(window, cx);
                })),
            NyaMenuItem::separator(),
            NyaMenuItem::submenu(
                self.tr("menu.smartSplit"),
                self.title_smart_split_menu_items(cx),
            )
            .icon("icons/menu/split.svg"),
            NyaMenuItem::separator(),
            NyaMenuItem::submenu(
                self.tr("menu.syncInput"),
                self.title_sync_input_menu_items(cx),
            )
            .icon("icons/sync.svg"),
            NyaMenuItem::separator(),
            NyaMenuItem::action(self.tr("menu.broadcastToAll"))
                .icon("icons/menu/broadcast.svg")
                .checked(self.sync_input.broadcast_to_all())
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_broadcast_to_all(cx);
                })),
            NyaMenuItem::separator(),
            NyaMenuItem::action(self.tr("menu.clearTerminal"))
                .icon("icons/fe/delete.svg")
                .shortcut(self.display_shortcut_for("terminal.clear", "Ctrl+L"))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.clear_terminal(cx);
                })),
            NyaMenuItem::action(self.tr("menu.resetTerminalSize"))
                .icon("icons/menu/fit.svg")
                .on_click(cx.listener(|this, _, _, cx| {
                    let changed = this.resize_all_known_terminal_surfaces();
                    this.shell.set_status(if changed {
                        "terminal sizes reset".to_string()
                    } else {
                        "terminal sizes already current".to_string()
                    });
                    cx.notify();
                })),
        ]
    }

    fn title_help_menu_items(&self, cx: &mut Context<Self>) -> Vec<NyaMenuItem> {
        let update_label = if self.update.is_pending() {
            self.tr("updater.checking")
        } else if self.update.info().is_some_and(|info| info.available) {
            self.tr("updater.newVersionAvailable")
        } else {
            self.tr("menu.checkForUpdates")
        };

        vec![
            NyaMenuItem::action(self.tr("menu.documentation"))
                .icon("icons/menu/book.svg")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.open_documentation(cx);
                })),
            NyaMenuItem::action(update_label)
                .icon("icons/menu/update.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_update_dialog(window, cx);
                })),
            NyaMenuItem::action(self.tr("menu.viewLogs"))
                .icon("icons/menu/article.svg")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.reveal_log_dir(cx);
                })),
            NyaMenuItem::separator(),
            NyaMenuItem::action(format!("{} NyaTerm", self.tr("menu.about")))
                .icon("icons/menu/info.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_about(window, cx);
                })),
        ]
    }

    fn title_theme_menu_items(&self, cx: &mut Context<Self>) -> Vec<NyaMenuItem> {
        let current = self.settings.summary().theme.as_str();
        crate::theme::APPEARANCE_THEME_IDS
            .iter()
            .map(|&theme| {
                let selected =
                    current == theme || (current == "catppuccin" && theme == "catppuccin-mocha");
                NyaMenuItem::action(crate::theme::appearance_theme_label(theme))
                    .checked(selected)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.update_appearance_theme(theme, cx);
                    }))
            })
            .collect()
    }

    fn title_language_menu_items(&self, cx: &mut Context<Self>) -> Vec<NyaMenuItem> {
        let language = self.settings.summary().language.as_str();
        vec![
            NyaMenuItem::action("English")
                .checked(matches!(language, "en" | "en-US"))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.update_ui_language("en", cx);
                })),
            NyaMenuItem::action("中文")
                .checked(matches!(language, "zh" | "zh-CN"))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.update_ui_language("zh-CN", cx);
                })),
        ]
    }

    fn title_smart_split_menu_items(&self, cx: &mut Context<Self>) -> Vec<NyaMenuItem> {
        vec![
            NyaMenuItem::action(self.tr("menu.autoTile"))
                .icon("icons/view-grid.svg")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.apply_smart_split(SmartSplitMode::Auto, cx);
                })),
            NyaMenuItem::action(self.tr("menu.tileHorizontally"))
                .icon("icons/menu/horizontal.svg")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.apply_smart_split(SmartSplitMode::Horizontal, cx);
                })),
            NyaMenuItem::action(self.tr("menu.tileVertically"))
                .icon("icons/menu/vertical.svg")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.apply_smart_split(SmartSplitMode::Vertical, cx);
                })),
        ]
    }

    fn title_sync_input_menu_items(&self, cx: &mut Context<Self>) -> Vec<NyaMenuItem> {
        vec![
            NyaMenuItem::action(self.tr("menu.manageGroups"))
                .icon("icons/settings.svg")
                .shortcut(self.display_shortcut_for("terminal.manageSyncGroups", "Ctrl+Shift+G"))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_sync_groups(window, cx);
                })),
        ]
    }
}
