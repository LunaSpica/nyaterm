use super::*;
use crate::ui::shortcuts::{
    SHORTCUT_CATEGORIES, SHORTCUT_REGISTRY, ShortcutCategory, ShortcutDefinition,
    ShortcutNativeStatus, format_hotkey_for_display, shortcut_keys_for,
};

impl NyaTermApp {
    pub(super) fn general_settings_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .grid()
                    .grid_cols(3)
                    .gap_3()
                    .child(metric("Theme", self.settings.theme.clone()))
                    .child(metric("Language", self.settings.language.clone()))
                    .child(metric(
                        "Terminal Font",
                        format!(
                            "{} {}",
                            self.settings.terminal_font_family, self.settings.terminal_font_size
                        ),
                    ))
                    .child(metric(
                        "Transfer Policy",
                        self.settings.transfer_duplicate_strategy.clone(),
                    ))
                    .child(metric(
                        "X11 Display",
                        if self.settings.x11_display.trim().is_empty() {
                            "auto".to_string()
                        } else {
                            self.settings.x11_display.clone()
                        },
                    ))
                    .child(metric(
                        "Store",
                        if self.store_status.ready {
                            "ready".to_string()
                        } else {
                            "offline".to_string()
                        },
                    )),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_3()
                    .child(setting_state(
                        "Startup Restore",
                        if self.settings.startup_restore {
                            "enabled"
                        } else {
                            "disabled"
                        },
                    ))
                    .child(setting_state(
                        "Confirm On Close",
                        if self.settings.confirm_on_close {
                            "enabled"
                        } else {
                            "disabled"
                        },
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(small_button(
                        "general-startup-restore",
                        if self.settings.startup_restore {
                            "Restore On"
                        } else {
                            "Restore Off"
                        },
                        cx.listener(|this, _, _, cx| {
                            this.toggle_startup_restore(cx);
                        }),
                    ))
                    .child(small_button(
                        "general-confirm-close",
                        if self.settings.confirm_on_close {
                            "Confirm On"
                        } else {
                            "Confirm Off"
                        },
                        cx.listener(|this, _, _, cx| {
                            this.toggle_confirm_on_close(cx);
                        }),
                    )),
            )
    }

    pub(super) fn appearance_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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
                    .child("Appearance"),
            )
            .child(
                div()
                    .mt_3()
                    .grid()
                    .grid_cols(5)
                    .gap_3()
                    .child(metric("Theme", self.settings.theme.clone()))
                    .child(metric(
                        "Terminal Font",
                        self.settings.terminal_font_family.clone(),
                    ))
                    .child(metric(
                        "Font Size",
                        self.settings.terminal_font_size.to_string(),
                    ))
                    .child(metric(
                        "X11 Display",
                        if self.settings.x11_display.trim().is_empty() {
                            "auto".to_string()
                        } else {
                            self.settings.x11_display.clone()
                        },
                    ))
                    .child(metric("Language", self.settings.language.clone())),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(policy_button(
                        "appearance-theme-dark",
                        "GitHub Dark",
                        self.settings.theme == "github-dark",
                        cx.listener(|this, _, _, cx| {
                            this.update_appearance_theme("github-dark", cx);
                        }),
                    ))
                    .child(policy_button(
                        "appearance-theme-light",
                        "GitHub Light",
                        self.settings.theme == "github-light",
                        cx.listener(|this, _, _, cx| {
                            this.update_appearance_theme("github-light", cx);
                        }),
                    ))
                    .child(policy_button(
                        "appearance-theme-catppuccin",
                        "Catppuccin",
                        self.settings.theme == "catppuccin",
                        cx.listener(|this, _, _, cx| {
                            this.update_appearance_theme("catppuccin", cx);
                        }),
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(policy_button(
                        "appearance-font-jetbrains",
                        "JetBrains Mono",
                        self.settings.terminal_font_family == "JetBrains Mono",
                        cx.listener(|this, _, _, cx| {
                            this.update_terminal_font_family("JetBrains Mono", cx);
                        }),
                    ))
                    .child(policy_button(
                        "appearance-font-iosevka",
                        "Iosevka",
                        self.settings.terminal_font_family == "Iosevka",
                        cx.listener(|this, _, _, cx| {
                            this.update_terminal_font_family("Iosevka", cx);
                        }),
                    ))
                    .child(policy_button(
                        "appearance-font-monospace",
                        "monospace",
                        self.settings.terminal_font_family == "monospace",
                        cx.listener(|this, _, _, cx| {
                            this.update_terminal_font_family("monospace", cx);
                        }),
                    ))
                    .child(small_button(
                        "appearance-font-size-minus",
                        "-1 px",
                        cx.listener(|this, _, _, cx| {
                            this.adjust_terminal_font_size(-1, cx);
                        }),
                    ))
                    .child(small_button(
                        "appearance-font-size-plus",
                        "+1 px",
                        cx.listener(|this, _, _, cx| {
                            this.adjust_terminal_font_size(1, cx);
                        }),
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(policy_button(
                        "appearance-x11-auto",
                        "X11 Auto",
                        self.settings.x11_display.trim().is_empty(),
                        cx.listener(|this, _, _, cx| {
                            this.update_x11_display("", cx);
                        }),
                    ))
                    .child(policy_button(
                        "appearance-x11-localhost0",
                        "localhost:0",
                        self.settings.x11_display == "localhost:0",
                        cx.listener(|this, _, _, cx| {
                            this.update_x11_display("localhost:0", cx);
                        }),
                    ))
                    .child(policy_button(
                        "appearance-x11-localhost1",
                        "localhost:1",
                        self.settings.x11_display == "localhost:1",
                        cx.listener(|this, _, _, cx| {
                            this.update_x11_display("localhost:1", cx);
                        }),
                    )),
            )
    }

    pub(super) fn interaction_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Side Panels"),
                            )
                            .child(status_pill(
                                if self.panel_multi_open {
                                    "multi-open"
                                } else {
                                    "single"
                                },
                                if self.panel_multi_open {
                                    rgb(0x6ee7b7)
                                } else {
                                    rgb(0x98a3b8)
                                },
                                if self.panel_multi_open {
                                    rgb(0x12342a)
                                } else {
                                    rgb(0x202633)
                                },
                            )),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(rgb(0x8b949e))
                            .child("Allow multiple side panels stacked on each edge, matching Tauri multi-open mode."),
                    )
                    .child(
                        div()
                            .mt_3()
                            .child(small_button(
                                "settings-panel-multi-open",
                                if self.panel_multi_open {
                                    "Disable Multi-Open"
                                } else {
                                    "Enable Multi-Open"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_panel_multi_open(cx);
                                }),
                            )),
                    ),
            )
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
                            .child("Clipboard and Mouse"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(4)
                            .gap_3()
                            .child(setting_state(
                                "Copy On Select",
                                if self.settings.interaction_copy_on_select {
                                    "enabled"
                                } else {
                                    "disabled"
                                },
                            ))
                            .child(setting_state(
                                "Right Click",
                                if self.settings.interaction_right_click_paste {
                                    "paste"
                                } else {
                                    "actions"
                                },
                            ))
                            .child(setting_state(
                                "Alt As Meta",
                                if self.settings.interaction_alt_as_meta {
                                    "enabled"
                                } else {
                                    "disabled"
                                },
                            ))
                            .child(setting_state(
                                "Mac IME",
                                if self.settings.interaction_mac_ime_compatibility {
                                    "compatible"
                                } else {
                                    "normal"
                                },
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .flex_wrap()
                            .child(small_button(
                                "interaction-copy-select",
                                if self.settings.interaction_copy_on_select {
                                    "Copy On"
                                } else {
                                    "Copy Off"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_interaction_copy_on_select(cx);
                                }),
                            ))
                            .child(small_button(
                                "interaction-right-paste",
                                if self.settings.interaction_right_click_paste {
                                    "Right Paste"
                                } else {
                                    "Right Menu"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_interaction_right_click_paste(cx);
                                }),
                            ))
                            .child(small_button(
                                "interaction-alt-meta",
                                if self.settings.interaction_alt_as_meta {
                                    "Alt Meta On"
                                } else {
                                    "Alt Meta Off"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_alt_as_meta(cx);
                                }),
                            ))
                            .child(small_button(
                                "interaction-mac-ime",
                                if self.settings.interaction_mac_ime_compatibility {
                                    "IME On"
                                } else {
                                    "IME Off"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_mac_ime_compatibility(cx);
                                }),
                            )),
                    ),
            )
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
                            .child("Command Input"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(4)
                            .gap_3()
                            .child(setting_state(
                                "Suggestions",
                                if self.settings.interaction_command_suggestions_enabled {
                                    "enabled"
                                } else {
                                    "disabled"
                                },
                            ))
                            .child(metric(
                                "Min Chars",
                                self.settings
                                    .interaction_command_suggestion_min_chars
                                    .to_string(),
                            ))
                            .child(metric(
                                "Max Chars",
                                self.settings
                                    .interaction_command_suggestion_max_chars
                                    .to_string(),
                            ))
                            .child(metric(
                                "Dup Delay",
                                format!(
                                    "{} ms",
                                    self.settings.interaction_duplicate_session_command_delay_ms
                                ),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .flex_wrap()
                            .child(small_button(
                                "interaction-suggestions",
                                if self.settings.interaction_command_suggestions_enabled {
                                    "Suggest On"
                                } else {
                                    "Suggest Off"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_command_suggestions(cx);
                                }),
                            ))
                            .child(small_button(
                                "interaction-min-minus",
                                "Min -1",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_min_chars(-1, cx);
                                }),
                            ))
                            .child(small_button(
                                "interaction-min-plus",
                                "Min +1",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_min_chars(1, cx);
                                }),
                            ))
                            .child(small_button(
                                "interaction-max-minus",
                                "Max -8",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_max_chars(-8, cx);
                                }),
                            ))
                            .child(small_button(
                                "interaction-max-plus",
                                "Max +8",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_max_chars(8, cx);
                                }),
                            ))
                            .child(small_button(
                                "interaction-delay-minus",
                                "-100 ms",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_duplicate_session_command_delay(-100, cx);
                                }),
                            ))
                            .child(small_button(
                                "interaction-delay-plus",
                                "+100 ms",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_duplicate_session_command_delay(100, cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(2)
                            .gap_3()
                            .child(metric(
                                "Word Separators",
                                truncate_preview(&self.settings.interaction_word_separators, 32),
                            ))
                            .child(metric(
                                "Encoding",
                                self.settings.interaction_default_encoding.clone(),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(policy_button(
                                "interaction-encoding-utf8",
                                "UTF-8",
                                self.settings.interaction_default_encoding == "UTF-8",
                                cx.listener(|this, _, _, cx| {
                                    this.set_interaction_encoding("UTF-8", cx);
                                }),
                            ))
                            .child(policy_button(
                                "interaction-encoding-gbk",
                                "GBK",
                                self.settings.interaction_default_encoding == "GBK",
                                cx.listener(|this, _, _, cx| {
                                    this.set_interaction_encoding("GBK", cx);
                                }),
                            ))
                            .child(policy_button(
                                "interaction-word-default",
                                "Default Separators",
                                self.settings.interaction_word_separators
                                    == " \t\r\n\"'`~!@#$%^&*()-=+[{]}\\|;:,<.>/?",
                                cx.listener(|this, _, _, cx| {
                                    this.set_interaction_word_separators(
                                        " \t\r\n\"'`~!@#$%^&*()-=+[{]}\\|;:,<.>/?",
                                        cx,
                                    );
                                }),
                            ))
                            .child(policy_button(
                                "interaction-word-minimal",
                                "Minimal Separators",
                                self.settings.interaction_word_separators == " \t\r\n",
                                cx.listener(|this, _, _, cx| {
                                    this.set_interaction_word_separators(" \t\r\n", cx);
                                }),
                            )),
                    ),
            )
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
                            .child("Tab Mouse Actions"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(3)
                            .gap_3()
                            .child(metric(
                                "Double Click",
                                tab_mouse_action_label(
                                    &self.settings.interaction_tab_double_click_action,
                                )
                                .to_string(),
                            ))
                            .child(metric(
                                "Middle Click",
                                tab_mouse_action_label(
                                    &self.settings.interaction_tab_middle_click_action,
                                )
                                .to_string(),
                            ))
                            .child(metric(
                                "Right Click",
                                tab_mouse_action_label(
                                    &self.settings.interaction_tab_right_click_action,
                                )
                                .to_string(),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .flex_wrap()
                            .child(small_button(
                                "interaction-cycle-double",
                                "Cycle Double",
                                cx.listener(|this, _, _, cx| {
                                    this.cycle_tab_mouse_action(TabMouseActionTarget::Double, cx);
                                }),
                            ))
                            .child(small_button(
                                "interaction-cycle-middle",
                                "Cycle Middle",
                                cx.listener(|this, _, _, cx| {
                                    this.cycle_tab_mouse_action(TabMouseActionTarget::Middle, cx);
                                }),
                            ))
                            .child(small_button(
                                "interaction-cycle-right",
                                "Cycle Right",
                                cx.listener(|this, _, _, cx| {
                                    this.cycle_tab_mouse_action(TabMouseActionTarget::Right, cx);
                                }),
                            )),
                    ),
            )
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
