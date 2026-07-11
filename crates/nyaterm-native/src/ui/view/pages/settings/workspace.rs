use super::*;
use crate::ui::shortcuts::{
    SHORTCUT_CATEGORIES, SHORTCUT_REGISTRY, ShortcutCategory, ShortcutDefinition,
    ShortcutNativeStatus, format_hotkey_for_display, shortcut_keys_for,
};

impl NyaTermApp {
    pub(super) fn general_settings_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        // Tauri GeneralTab: SettingSection + SettingRow switches (no metric cards).
        let language = self.settings.language.clone();
        let language_label = match language.as_str() {
            "zh-CN" | "zh" => "简体中文",
            "zh-TW" => "繁體中文",
            "en" | "en-US" => "English",
            "ja" => "日本語",
            other => other,
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                None,
                None,
                settings_form_row(
                    "Language",
                    Some(SharedString::from("UI language preference for labels and dialogs.")),
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(settings_choice_chip(
                            "general-lang-en",
                            "English",
                            matches!(language.as_str(), "en" | "en-US"),
                            cx.listener(|this, _, _, cx| {
                                this.update_ui_language("en", cx);
                            }),
                        ))
                        .child(settings_choice_chip(
                            "general-lang-zh",
                            "中文",
                            matches!(language.as_str(), "zh-CN" | "zh"),
                            cx.listener(|this, _, _, cx| {
                                this.update_ui_language("zh-CN", cx);
                            }),
                        ))
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(0x6e7681))
                                .child(language_label.to_string()),
                        ),
                ),
            ))
            .child(settings_form_section(
                Some("Startup & window"),
                Some("Restore sessions and confirm before closing the app."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        "Restore sessions on startup",
                        Some(SharedString::from("Reopen the previous workspace tabs when NyaTerm starts.")),
                        settings_switch(
                            "general-startup-restore",
                            self.settings.startup_restore,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_startup_restore(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        "Confirm on close",
                        Some(SharedString::from("Ask before quitting when sessions are still open.")),
                        settings_switch(
                            "general-confirm-close",
                            self.settings.confirm_on_close,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_confirm_on_close(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                Some("Status"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(settings_form_row(
                        "Connection store",
                        Some(SharedString::from("Native redb store readiness.")),
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight(600.))
                            .text_color(if self.store_status.ready {
                                rgb(0x3fb950)
                            } else {
                                rgb(0xff7b72)
                            })
                            .child(if self.store_status.ready {
                                "Ready"
                            } else {
                                "Offline"
                            }),
                    ))
                    .child(settings_form_row(
                        "Theme / font",
                        Some(SharedString::from("Current appearance snapshot.")),
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x8b949e))
                            .child(format!(
                                "{} · {} {}",
                                self.settings.theme,
                                self.settings.terminal_font_family,
                                self.settings.terminal_font_size
                            )),
                    )),
            ))
    }

    pub(super) fn appearance_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let font_size_label = format!("{} px", self.settings.terminal_font_size);
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                Some("Theme"),
                Some("Color scheme used by the native shell chrome."),
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(settings_choice_chip(
                        "appearance-theme-dark",
                        "GitHub Dark",
                        self.settings.theme == "github-dark",
                        cx.listener(|this, _, _, cx| {
                            this.update_appearance_theme("github-dark", cx);
                        }),
                    ))
                    .child(settings_choice_chip(
                        "appearance-theme-light",
                        "GitHub Light",
                        self.settings.theme == "github-light",
                        cx.listener(|this, _, _, cx| {
                            this.update_appearance_theme("github-light", cx);
                        }),
                    ))
                    .child(settings_choice_chip(
                        "appearance-theme-catppuccin",
                        "Catppuccin",
                        self.settings.theme == "catppuccin",
                        cx.listener(|this, _, _, cx| {
                            this.update_appearance_theme("catppuccin", cx);
                        }),
                    )),
            ))
            .child(settings_form_section(
                Some("Terminal font"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        "Font family",
                        Some(SharedString::from("Monospace face used by the GPUI terminal surface.")),
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(
                                "appearance-font-jetbrains",
                                "JetBrains Mono",
                                self.settings.terminal_font_family == "JetBrains Mono",
                                cx.listener(|this, _, _, cx| {
                                    this.update_terminal_font_family("JetBrains Mono", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "appearance-font-iosevka",
                                "Iosevka",
                                self.settings.terminal_font_family == "Iosevka",
                                cx.listener(|this, _, _, cx| {
                                    this.update_terminal_font_family("Iosevka", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "appearance-font-monospace",
                                "monospace",
                                self.settings.terminal_font_family == "monospace",
                                cx.listener(|this, _, _, cx| {
                                    this.update_terminal_font_family("monospace", cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        "Font size",
                        Some(SharedString::from("Zoom the terminal text without leaving Settings.")),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                "appearance-font-minus",
                                "−",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_terminal_font_size(-1, cx);
                                }),
                            ))
                            .child(
                                div()
                                    .min_w(px(48.))
                                    .text_center()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(600.))
                                    .text_color(rgb(0xc9d1d9))
                                    .child(font_size_label),
                            )
                            .child(small_button(
                                "appearance-font-plus",
                                "+",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_terminal_font_size(1, cx);
                                }),
                            ))
                            .child(small_button(
                                "appearance-font-reset",
                                "Reset",
                                cx.listener(|this, _, _, cx| {
                                    this.reset_terminal_font_size(cx);
                                }),
                            )),
                    )),
            ))
            .child(settings_form_section(
                Some("X11 display"),
                Some("Used when launching remote X11-forwarded tools."),
                settings_form_row(
                    "Display",
                    Some(SharedString::from("Forwarded X11 DISPLAY preference.")),
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_1()
                        .child(settings_choice_chip(
                            "appearance-x11-auto",
                            "Auto",
                            self.settings.x11_display.trim().is_empty(),
                            cx.listener(|this, _, _, cx| {
                                this.update_x11_display("", cx);
                            }),
                        ))
                        .child(settings_choice_chip(
                            "appearance-x11-localhost0",
                            "localhost:0",
                            self.settings.x11_display == "localhost:0",
                            cx.listener(|this, _, _, cx| {
                                this.update_x11_display("localhost:0", cx);
                            }),
                        ))
                        .child(settings_choice_chip(
                            "appearance-x11-localhost1",
                            "localhost:1",
                            self.settings.x11_display == "localhost:1",
                            cx.listener(|this, _, _, cx| {
                                this.update_x11_display("localhost:1", cx);
                            }),
                        )),
                ),
            ))
    }

    pub(super) fn interaction_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Tauri InteractionTab density: section cards + switch/choice rows.
        let encoding = self.settings.interaction_default_encoding.clone();
        let word_sep = self.settings.interaction_word_separators.clone();
        let double_action = tab_mouse_action_label(&self.settings.interaction_tab_double_click_action);
        let middle_action = tab_mouse_action_label(&self.settings.interaction_tab_middle_click_action);
        let right_action = tab_mouse_action_label(&self.settings.interaction_tab_right_click_action);
        let delay_ms = self
            .settings
            .interaction_duplicate_session_command_delay_ms
            .to_string();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                Some("Side panels"),
                Some("Allow multiple side panels stacked on each edge."),
                settings_form_row(
                    "Multi-open panels",
                    Some(SharedString::from("Stack several left/right panels instead of replacing the active one.")),
                    settings_switch(
                        "settings-panel-multi-open",
                        self.panel_multi_open,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_panel_multi_open(cx);
                        }),
                    ),
                ),
            ))
            .child(settings_form_section(
                Some("Clipboard and mouse"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        "Copy on select",
                        Some(SharedString::from("Copy selected terminal text to the clipboard automatically.")),
                        settings_switch(
                            "interaction-copy-select",
                            self.settings.interaction_copy_on_select,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_interaction_copy_on_select(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        "Right-click paste",
                        Some(SharedString::from("Paste from clipboard on right-click instead of opening a menu.")),
                        settings_switch(
                            "interaction-right-paste",
                            self.settings.interaction_right_click_paste,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_interaction_right_click_paste(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        "Alt as Meta",
                        Some(SharedString::from("Treat Alt as Meta for terminal key bindings.")),
                        settings_switch(
                            "interaction-alt-meta",
                            self.settings.interaction_alt_as_meta,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_alt_as_meta(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        "Mac IME compatibility",
                        Some(SharedString::from("Improve input method editor handling on macOS.")),
                        settings_switch(
                            "interaction-mac-ime",
                            self.settings.interaction_mac_ime_compatibility,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_mac_ime_compatibility(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                Some("Command input"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        "Command suggestions",
                        Some(SharedString::from("Offer history-based suggestions while typing commands.")),
                        settings_switch(
                            "interaction-cmd-suggestions",
                            self.settings.interaction_command_suggestions_enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_command_suggestions(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        "Suggestion length",
                        Some(SharedString::from("Minimum and maximum characters before suggestions appear.")),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(11.))
                                    .text_color(rgb(0xc9d1d9))
                                    .child(format!(
                                        "{}–{}",
                                        self.settings.interaction_command_suggestion_min_chars,
                                        self.settings.interaction_command_suggestion_max_chars
                                    )),
                            )
                            .child(small_button(
                                "interaction-suggest-min-minus",
                                "Min −",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_min_chars(-1, cx);
                                }),
                            ))
                            .child(small_button(
                                "interaction-suggest-min-plus",
                                "Min +",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_min_chars(1, cx);
                                }),
                            ))
                            .child(small_button(
                                "interaction-suggest-max-minus",
                                "Max −",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_max_chars(-1, cx);
                                }),
                            ))
                            .child(small_button(
                                "interaction-suggest-max-plus",
                                "Max +",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_max_chars(1, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        "Default encoding",
                        Some(SharedString::from("Fallback character encoding for session I/O.")),
                        div()
                            .flex()
                            .gap_1()
                            .child(settings_choice_chip(
                                "interaction-encoding-utf8",
                                "UTF-8",
                                encoding == "UTF-8",
                                cx.listener(|this, _, _, cx| {
                                    this.set_interaction_encoding("UTF-8", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "interaction-encoding-gbk",
                                "GBK",
                                encoding == "GBK",
                                cx.listener(|this, _, _, cx| {
                                    this.set_interaction_encoding("GBK", cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        "Word separators",
                        Some(SharedString::from("Characters that split double-click word selection.")),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(10.))
                                    .text_color(rgb(0x8b949e))
                                    .child(truncate_preview(&word_sep, 24)),
                            )
                            .child(settings_choice_chip(
                                "interaction-word-sep-shell",
                                "Shell",
                                word_sep.contains('/') && word_sep.contains(':'),
                                cx.listener(|this, _, _, cx| {
                                    this.set_interaction_word_separators(
                                        " `\"'()[]{}<>|&;/",
                                        cx,
                                    );
                                }),
                            ))
                            .child(settings_choice_chip(
                                "interaction-word-sep-basic",
                                "Basic",
                                word_sep == " \t\r\n",
                                cx.listener(|this, _, _, cx| {
                                    this.set_interaction_word_separators(" \t\r\n", cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        "Duplicate session delay",
                        Some(SharedString::from("Delay before replaying the startup command on a duplicated tab.")),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(11.))
                                    .text_color(rgb(0xc9d1d9))
                                    .child(format!("{delay_ms} ms")),
                            )
                            .child(small_button(
                                "interaction-dup-delay-minus",
                                "−100",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_duplicate_session_command_delay(-100, cx);
                                }),
                            ))
                            .child(small_button(
                                "interaction-dup-delay-plus",
                                "+100",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_duplicate_session_command_delay(100, cx);
                                }),
                            )),
                    )),
            ))
            .child(settings_form_section(
                Some("Tab mouse actions"),
                Some("What happens when clicking session tabs."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        "Double-click",
                        Some(SharedString::from(double_action)),
                        small_button(
                            "interaction-cycle-double",
                            "Cycle",
                            cx.listener(|this, _, _, cx| {
                                this.cycle_tab_mouse_action(TabMouseActionTarget::Double, cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        "Middle-click",
                        Some(SharedString::from(middle_action)),
                        small_button(
                            "interaction-cycle-middle",
                            "Cycle",
                            cx.listener(|this, _, _, cx| {
                                this.cycle_tab_mouse_action(TabMouseActionTarget::Middle, cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        "Right-click",
                        Some(SharedString::from(right_action)),
                        small_button(
                            "interaction-cycle-right",
                            "Cycle",
                            cx.listener(|this, _, _, cx| {
                                this.cycle_tab_mouse_action(TabMouseActionTarget::Right, cx);
                            }),
                        ),
                    )),
            ))
    }

    pub(super) fn keybindings_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let supported = SHORTCUT_REGISTRY
            .iter()
            .filter(|shortcut| shortcut.native_status == ShortcutNativeStatus::Supported)
            .count();
        let pending = SHORTCUT_REGISTRY
            .iter()
            .filter(|shortcut| shortcut.native_status == ShortcutNativeStatus::Pending)
            .count();
        let overrides = self.settings.keybindings.len();
        let mut groups = div().mt_4().flex().flex_col().gap_3();
        for category in SHORTCUT_CATEGORIES {
            groups = groups.child(self.shortcut_category_group(category, cx));
        }

        div()
            .id("settings-keybindings-panel")
            .flex()
            .flex_col()
            .gap_4()
            .track_focus(&self.keybindings_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.keybindings_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_keybinding_key_down(event, cx);
            }))
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Keyboard Shortcuts"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(4)
                            .gap_3()
                            .child(metric("Registry", SHORTCUT_REGISTRY.len().to_string()))
                            .child(metric("Native", supported.to_string()))
                            .child(metric("Pending", pending.to_string()))
                            .child(metric("Overrides", overrides.to_string())),
                    )
                    .child(
                        div()
                            .mt_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(0x263142))
                            .bg(rgb(0x0d1320))
                            .p_3()
                            .text_xs()
                            .line_height(px(18.))
                            .text_color(rgb(0x98a3b8))
                            .child("Press Record, type a shortcut, then Save or Enter. Overrides are stored in the same keybindings object used by the Tauri app."),
                    )
                    .when(overrides > 0, |this| {
                        this.child(
                            div()
                                .mt_3()
                                .flex()
                                .justify_end()
                                .child(small_button(
                                    "keybindings-reset-all",
                                    "Reset All",
                                    cx.listener(|this, _, _, cx| {
                                        this.reset_all_keybindings(cx);
                                    }),
                                )),
                        )
                    })
                    .child(groups),
            )
    }

    fn shortcut_category_group(
        &mut self,
        category: ShortcutCategory,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut rows = div().mt_2().flex().flex_col().gap_2();
        for shortcut in SHORTCUT_REGISTRY
            .iter()
            .filter(|shortcut| shortcut.category == category)
        {
            rows = rows.child(self.shortcut_registry_row(shortcut, cx));
        }

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x263142))
            .bg(rgb(0x10151e))
            .p_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0xe5edf7))
                            .child(category.label()),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(0x64748b))
                            .child(format!(
                                "{} shortcut(s)",
                                SHORTCUT_REGISTRY
                                    .iter()
                                    .filter(|shortcut| shortcut.category == category)
                                    .count()
                            )),
                    ),
            )
            .child(rows)
    }

    fn shortcut_registry_row(
        &mut self,
        shortcut: &'static ShortcutDefinition,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (badge_fg, badge_bg) = match shortcut.native_status {
            ShortcutNativeStatus::Supported => (rgb(0x6ee7b7), rgb(0x12342a)),
            ShortcutNativeStatus::Partial => (rgb(0x93c5fd), rgb(0x17253b)),
            ShortcutNativeStatus::Pending => (rgb(0xfca5a5), rgb(0x3b1518)),
            ShortcutNativeStatus::Contextual => (rgb(0xfacc15), rgb(0x33270c)),
        };
        let is_custom = self.settings.keybindings.contains_key(shortcut.id);
        let is_recording = self.keybinding_recording_id.as_deref() == Some(shortcut.id);
        let effective_keys = shortcut_keys_for(shortcut.id, &self.settings.keybindings)
            .unwrap_or_else(|| shortcut.default_keys.to_string());
        let key_display = if is_recording {
            self.keybinding_pending_keys
                .as_deref()
                .map(format_hotkey_for_display)
                .unwrap_or_else(|| "Press key combination...".to_string())
        } else {
            format_hotkey_for_display(&effective_keys)
        };
        let shortcut_id = shortcut.id.to_string();
        let reset_shortcut_id = shortcut.id.to_string();

        div()
            .rounded_sm()
            .border_1()
            .border_color(if is_recording {
                rgb(0x3b82f6)
            } else {
                rgb(0x202633)
            })
            .bg(if is_recording {
                rgb(0x0f1b2d)
            } else {
                rgb(0x0d1320)
            })
            .p_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(0xe5edf7))
                                            .child(shortcut.label),
                                    )
                                    .when(is_custom, |this| {
                                        this.child(status_pill(
                                            "custom",
                                            rgb(0xc4b5fd),
                                            rgb(0x2b1b45),
                                        ))
                                    }),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(0x64748b))
                                    .child(shortcut.id),
                            ),
                    )
                    .child(status_pill(
                        shortcut.native_status.label(),
                        badge_fg,
                        badge_bg,
                    )),
            )
            .child(
                div()
                    .mt_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .min_w_0()
                            .rounded_sm()
                            .border_1()
                            .border_color(if is_recording {
                                rgb(0x60a5fa)
                            } else {
                                rgb(0x303848)
                            })
                            .bg(rgb(0x151b27))
                            .px_2()
                            .py_1()
                            .font_family("JetBrains Mono")
                            .text_size(px(10.))
                            .font_weight(FontWeight(800.))
                            .text_color(if is_recording {
                                rgb(0xbfdbfe)
                            } else {
                                rgb(0xdbeafe)
                            })
                            .child(key_display),
                    )
                    .child(
                        div()
                            .min_w(px(180.))
                            .max_w(px(300.))
                            .text_size(px(10.))
                            .text_color(rgb(0x8f98aa))
                            .line_height(px(14.))
                            .child(shortcut.note),
                    ),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .when(is_recording, |this| {
                        this.child(small_button(
                            format!("keybinding-save-{}", shortcut.id),
                            "Save",
                            cx.listener(|this, _, _, cx| {
                                this.confirm_keybinding_recording(cx);
                            }),
                        ))
                        .child(small_button(
                            format!("keybinding-cancel-{}", shortcut.id),
                            "Cancel",
                            cx.listener(|this, _, _, cx| {
                                this.cancel_keybinding_recording(cx);
                            }),
                        ))
                    })
                    .when(!is_recording, |this| {
                        this.child(small_button(
                            format!("keybinding-record-{}", shortcut.id),
                            "Record",
                            cx.listener(move |this, _, window, cx| {
                                this.start_keybinding_recording(shortcut_id.clone(), window, cx);
                            }),
                        ))
                    })
                    .when(is_custom && !is_recording, |this| {
                        this.child(small_button(
                            format!("keybinding-reset-{}", shortcut.id),
                            "Reset",
                            cx.listener(move |this, _, _, cx| {
                                this.reset_keybinding(reset_shortcut_id.clone(), cx);
                            }),
                        ))
                    }),
            )
    }
}
