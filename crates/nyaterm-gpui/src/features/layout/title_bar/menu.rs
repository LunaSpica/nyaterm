use super::*;

impl NyaTermApp {
    pub(in crate::features) fn title_menu_dropdown(&self, menu: TitleMenu, cx: &mut Context<Self>) -> impl IntoElement {
        let shortcut = |id: &str, fallback: &str| self.display_shortcut_for(id, fallback);
        let palette = self.theme_palette();
        let mut items = div()
            .id(SharedString::from(format!("title-menu-{}", menu.label())))
            .absolute()
            .top(px(30.))
            .left_0()
            .w(px(260.))
            .max_h(px(480.))
            .overflow_y_scroll()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
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
                        "New Session",
                        Some(shortcut("tab.newSession", "Ctrl+Shift+N")),
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.start_local_session(window, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-file-temp-ssh",
                        "Temporary SSH Link",
                        Some(shortcut("tab.temporarySshLink", "Ctrl+Alt+N")),
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.open_temporary_ssh_link_dialog(window, cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-file-import",
                        "Import Config",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.prompt_config_import(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-file-export",
                        "Export Config",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.prompt_config_export(cx);
                        }),
                    ));
            }
            TitleMenu::View => {
                items = items
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .text_size(px(10.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text_dimmed))
                            .child("Theme"),
                    )
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-github-dark",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "github-dark"
                                || (current == "catppuccin" && "github-dark" == "catppuccin-mocha");
                            let label = crate::theme::appearance_theme_label("github-dark");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("github-dark", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-dracula",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "dracula"
                                || (current == "catppuccin" && "dracula" == "catppuccin-mocha");
                            let label = crate::theme::appearance_theme_label("dracula");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("dracula", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-nord",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "nord"
                                || (current == "catppuccin" && "nord" == "catppuccin-mocha");
                            let label = crate::theme::appearance_theme_label("nord");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("nord", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-monokai-pro",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "monokai-pro"
                                || (current == "catppuccin" && "monokai-pro" == "catppuccin-mocha");
                            let label = crate::theme::appearance_theme_label("monokai-pro");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("monokai-pro", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-solarized-light",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "solarized-light"
                                || (current == "catppuccin"
                                    && "solarized-light" == "catppuccin-mocha");
                            let label = crate::theme::appearance_theme_label("solarized-light");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("solarized-light", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-catppuccin-mocha",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "catppuccin-mocha"
                                || (current == "catppuccin"
                                    && "catppuccin-mocha" == "catppuccin-mocha");
                            let label =
                                crate::theme::appearance_theme_label("catppuccin-mocha");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("catppuccin-mocha", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-tokyo-night",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "tokyo-night"
                                || (current == "catppuccin" && "tokyo-night" == "catppuccin-mocha");
                            let label = crate::theme::appearance_theme_label("tokyo-night");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("tokyo-night", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-one-dark-pro",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "one-dark-pro"
                                || (current == "catppuccin"
                                    && "one-dark-pro" == "catppuccin-mocha");
                            let label = crate::theme::appearance_theme_label("one-dark-pro");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("one-dark-pro", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-rose-pine",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "rose-pine"
                                || (current == "catppuccin" && "rose-pine" == "catppuccin-mocha");
                            let label = crate::theme::appearance_theme_label("rose-pine");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("rose-pine", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-gruvbox-dark",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "gruvbox-dark"
                                || (current == "catppuccin"
                                    && "gruvbox-dark" == "catppuccin-mocha");
                            let label = crate::theme::appearance_theme_label("gruvbox-dark");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("gruvbox-dark", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-github-light",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "github-light"
                                || (current == "catppuccin"
                                    && "github-light" == "catppuccin-mocha");
                            let label = crate::theme::appearance_theme_label("github-light");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("github-light", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-catppuccin-latte",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "catppuccin-latte"
                                || (current == "catppuccin"
                                    && "catppuccin-latte" == "catppuccin-mocha");
                            let label =
                                crate::theme::appearance_theme_label("catppuccin-latte");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("catppuccin-latte", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-one-light",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "one-light"
                                || (current == "catppuccin" && "one-light" == "catppuccin-mocha");
                            let label = crate::theme::appearance_theme_label("one-light");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("one-light", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-nya-high-contrast",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "nya-high-contrast"
                                || (current == "catppuccin"
                                    && "nya-high-contrast" == "catppuccin-mocha");
                            let label =
                                crate::theme::appearance_theme_label("nya-high-contrast");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("nya-high-contrast", cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .text_size(px(10.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text_dimmed))
                            .child("Language"),
                    )
                    .child(title_menu_item(
                        palette,
                        "title-view-lang-en",
                        if matches!(self.settings.language.as_str(), "en" | "en-US") {
                            "✓ English"
                        } else {
                            "English"
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_ui_language("en", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-lang-zh",
                        if matches!(self.settings.language.as_str(), "zh-CN" | "zh") {
                            "✓ 中文"
                        } else {
                            "中文"
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_ui_language("zh-CN", cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-view-zoom-in",
                        "Zoom In",
                        Some(shortcut("view.zoomIn", "Ctrl+=")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.zoom_terminal_in(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-zoom-out",
                        "Zoom Out",
                        Some(shortcut("view.zoomOut", "Ctrl+-")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.zoom_terminal_out(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-reset-zoom",
                        "Reset Zoom",
                        Some(shortcut("view.resetZoom", "Ctrl+0")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.reset_terminal_font_size(cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-view-toggle-left",
                        "Toggle Left Sidebar",
                        Some(shortcut("view.toggleLeftSidebar", "Ctrl+Shift+E")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.toggle_left_sidebar(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-toggle-right",
                        "Toggle Right Sidebar",
                        Some(shortcut("view.toggleRightSidebar", "Ctrl+Shift+B")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.toggle_right_inspector(cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-view-smart-split",
                        "Smart Split",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.apply_smart_split(SmartSplitMode::Auto, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-tile-h",
                        "Tile Horizontally",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.apply_smart_split(SmartSplitMode::Horizontal, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-tile-v",
                        "Tile Vertically",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.apply_smart_split(SmartSplitMode::Vertical, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-merge-windows",
                        "Merge Windows",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.close_terminal_window_layout(cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-view-settings",
                        "Settings",
                        Some(shortcut("view.openSettings", "Ctrl+,")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.open_page(NavItem::Settings, cx);
                        }),
                    ));
            }
            TitleMenu::Terminal => {
                items = items
                    .child(title_menu_item(
                        palette,
                        "title-term-copy",
                        "Copy",
                        Some(shortcut("terminal.copy", "Ctrl+Shift+C")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.copy_terminal_selection_or_visible(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-paste",
                        "Paste",
                        Some(shortcut("terminal.paste", "Ctrl+Shift+V")),
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.paste_from_clipboard(window, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-select-all",
                        "Select All",
                        Some(shortcut("terminal.selectAll", "Ctrl+Shift+A")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.select_all_terminal_visible(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-find",
                        "Find",
                        Some(shortcut("terminal.find", "Ctrl+Shift+F")),
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.open_terminal_search(window, cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-term-quick-switch",
                        "Command Palette",
                        Some(shortcut("tab.quickSwitch", "Ctrl+Shift+S")),
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.open_quick_switch(window, cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-term-split-h",
                        "Split Horizontal",
                        None,
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.split_workspace_with_duplicate(
                                WorkspaceSplitDirection::Horizontal,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-split-v",
                        "Split Vertical",
                        None,
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.split_workspace_with_duplicate(
                                WorkspaceSplitDirection::Vertical,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-unsplit",
                        "Unsplit",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.unsplit_workspace(cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-term-sync-groups",
                        "Manage Sync Groups",
                        Some(shortcut("terminal.manageSyncGroups", "Ctrl+Shift+G")),
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.open_sync_groups(window, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-broadcast",
                        if self.broadcast_to_all {
                            "Broadcast to All ✓"
                        } else {
                            "Broadcast to All"
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.toggle_broadcast_to_all(cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-term-smart-split",
                        "Smart Split",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.apply_smart_split(SmartSplitMode::Auto, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-tile-h",
                        "Tile Horizontally",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.apply_smart_split(SmartSplitMode::Horizontal, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-tile-v",
                        "Tile Vertically",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.apply_smart_split(SmartSplitMode::Vertical, cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-term-clear",
                        "Clear Terminal",
                        Some(shortcut("terminal.clear", "Ctrl+L")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.clear_terminal(cx);
                        }),
                    ));
            }
            TitleMenu::Help => {
                let update_label = if self.update_pending {
                    "Checking Updates…"
                } else if self.update_info.as_ref().is_some_and(|info| info.available) {
                    "Update Available"
                } else {
                    "Check for Updates"
                };
                items = items
                    .child(title_menu_item(
                        palette,
                        "title-help-docs",
                        "Documentation",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.terminal_status =
                                "docs: https://github.com/nyaterm/nyaterm".to_string();
                            cx.notify();
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-help-update",
                        update_label,
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.start_update_check(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-help-migration",
                        "Migration Status",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.open_page(NavItem::Migration, cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-help-about",
                        "About NyaTerm",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.terminal_status =
                                format!("NyaTerm native {}", env!("CARGO_PKG_VERSION"));
                            cx.notify();
                        }),
                    ));
            }
        }

        items
    }

}
