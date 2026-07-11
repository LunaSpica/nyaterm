use super::*;
use crate::ui::theme::{APPEARANCE_THEME_IDS, appearance_theme_label};
use crate::ui::shortcuts::{
    SHORTCUT_CATEGORIES, SHORTCUT_REGISTRY, ShortcutCategory, ShortcutDefinition,
    ShortcutNativeStatus, format_hotkey_for_display, shortcut_keys_for,
};

impl NyaTermApp {
    pub(super) fn general_settings_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
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
            .child(settings_form_section(palette, 
                None,
                None,
                settings_form_row(palette, 
                    "Language",
                    Some(SharedString::from("UI language preference for labels and dialogs.")),
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(settings_choice_chip(palette, 
                            "general-lang-en",
                            "English",
                            matches!(language.as_str(), "en" | "en-US"),
                            cx.listener(|this, _, _, cx| {
                                this.update_ui_language("en", cx);
                            }),
                        ))
                        .child(settings_choice_chip(palette, 
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
                                .text_color(rgb(palette.text_dimmed))
                                .child(language_label.to_string()),
                        ),
                ),
            ))
            .child(settings_form_section(palette, 
                Some("Startup & window"),
                Some("Restore sessions and confirm before closing the app."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Restore sessions on startup",
                        Some(SharedString::from("Reopen the previous workspace tabs when NyaTerm starts.")),
                        settings_switch(palette, 
                            "general-startup-restore",
                            self.settings.startup_restore,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_startup_restore(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Restore window layout",
                        Some(SharedString::from(
                            "Restore multi-leaf tab windows and global pane splits with the workspace.",
                        )),
                        settings_switch(palette, 
                            "general-startup-restore-window-layout",
                            self.settings.startup_restore_window_layout,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_startup_restore_window_layout(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Confirm on close",
                        Some(SharedString::from("Ask before quitting when sessions are still open.")),
                        settings_switch(palette, 
                            "general-confirm-close",
                            self.settings.confirm_on_close,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_confirm_on_close(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(palette, 
                Some("Status"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(settings_form_row(palette, 
                        "Connection store",
                        Some(SharedString::from("Native redb store readiness.")),
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight(600.))
                            .text_color(if self.store_status.ready {
                                rgb(palette.success)
                            } else {
                                rgb(palette.danger)
                            })
                            .child(if self.store_status.ready {
                                "Ready"
                            } else {
                                "Offline"
                            }),
                    ))
                    .child(settings_form_row(palette, 
                        "Theme / font",
                        Some(SharedString::from("Current appearance snapshot.")),
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
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
        let palette = self.theme_palette();
        let font_size_label = format!("{} px", self.settings.terminal_font_size);
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                palette,
                Some("Theme"),
                Some("Color scheme used by the native shell chrome (Tauri theme list)."),
                {
                    let current = self.settings.theme.clone();
                    let chips = APPEARANCE_THEME_IDS.iter().fold(
                        div().flex().flex_wrap().gap_1(),
                        |row, theme_id| {
                            let theme_id_owned = (*theme_id).to_string();
                            let selected = current == *theme_id
                                || (current == "catppuccin" && *theme_id == "catppuccin-mocha");
                            row.child(settings_choice_chip(
                                palette,
                                format!("appearance-theme-{theme_id}"),
                                appearance_theme_label(theme_id),
                                selected,
                                cx.listener(move |this, _, _, cx| {
                                    this.update_appearance_theme(&theme_id_owned, cx);
                                }),
                            ))
                        },
                    );
                    chips
                },
            ))
                        .child(settings_form_section(
                palette,
                Some("Background image"),
                Some("Optional shell wallpaper (Tauri Appearance background image)."),
                {
                    let path_label = self
                        .settings
                        .background_image_path
                        .as_deref()
                        .map(|p| {
                            if p.chars().count() > 56 {
                                format!("…{}", p.chars().rev().take(52).collect::<String>().chars().rev().collect::<String>())
                            } else {
                                p.to_string()
                            }
                        })
                        .unwrap_or_else(|| "No image selected".to_string());
                    let has_image = self.settings.background_image_path.is_some();
                    let image_opacity = format!("{}%", self.settings.background_image_opacity);
                    let content_opacity = format!("{}%", self.settings.background_content_opacity);
                    let fit = self.settings.background_image_fit.clone();
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(settings_form_row(
                            palette,
                            "Image",
                            Some(SharedString::from(path_label)),
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(small_button(
                                    palette,
                                    "appearance-wallpaper-browse",
                                    "Browse",
                                    cx.listener(|this, _, _, cx| {
                                        this.prompt_background_image(cx);
                                    }),
                                ))
                                .when(has_image, |this| {
                                    this.child(small_button(
                                        palette,
                                        "appearance-wallpaper-clear",
                                        "Clear",
                                        cx.listener(|this, _, _, cx| {
                                            this.clear_background_image(cx);
                                        }),
                                    ))
                                }),
                        ))
                        .child(settings_form_row(
                            palette,
                            "Fit",
                            Some(SharedString::from("How the wallpaper is scaled in the shell.")),
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_1()
                                .child(settings_choice_chip(
                                    palette,
                                    "appearance-wallpaper-fit-cover",
                                    "Cover",
                                    fit == "cover",
                                    cx.listener(|this, _, _, cx| {
                                        this.set_background_image_fit("cover", cx);
                                    }),
                                ))
                                .child(settings_choice_chip(
                                    palette,
                                    "appearance-wallpaper-fit-contain",
                                    "Contain",
                                    fit == "contain",
                                    cx.listener(|this, _, _, cx| {
                                        this.set_background_image_fit("contain", cx);
                                    }),
                                ))
                                .child(settings_choice_chip(
                                    palette,
                                    "appearance-wallpaper-fit-stretch",
                                    "Stretch",
                                    fit == "stretch" || fit == "fill",
                                    cx.listener(|this, _, _, cx| {
                                        this.set_background_image_fit("stretch", cx);
                                    }),
                                ))
                                .child(settings_choice_chip(
                                    palette,
                                    "appearance-wallpaper-fit-tile",
                                    "Tile",
                                    fit == "tile",
                                    cx.listener(|this, _, _, cx| {
                                        this.set_background_image_fit("tile", cx);
                                    }),
                                )),
                        ))
                        .child(settings_form_row(
                            palette,
                            "Image opacity",
                            Some(SharedString::from("Wallpaper strength over the theme background.")),
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(small_button(
                                    palette,
                                    "appearance-wallpaper-opacity-dec",
                                    "−",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_background_image_opacity(-5, cx);
                                    }),
                                ))
                                .child(
                                    div()
                                        .min_w(px(48.))
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text))
                                        .child(image_opacity),
                                )
                                .child(small_button(
                                    palette,
                                    "appearance-wallpaper-opacity-inc",
                                    "+",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_background_image_opacity(5, cx);
                                    }),
                                )),
                        ))
                        .child(settings_form_row(
                            palette,
                            "Content opacity",
                            Some(SharedString::from("How solid chrome stays when a wallpaper is active.")),
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(small_button(
                                    palette,
                                    "appearance-content-opacity-dec",
                                    "−",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_background_content_opacity(-5, cx);
                                    }),
                                ))
                                .child(
                                    div()
                                        .min_w(px(48.))
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text))
                                        .child(content_opacity),
                                )
                                .child(small_button(
                                    palette,
                                    "appearance-content-opacity-inc",
                                    "+",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_background_content_opacity(5, cx);
                                    }),
                                )),
                        ))
                },
            ))
            .child(settings_form_section(palette, 
                Some("Terminal font"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Font family",
                        Some(SharedString::from("Monospace face used by the GPUI terminal surface.")),
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(palette, 
                                "appearance-font-jetbrains",
                                "JetBrains Mono",
                                self.settings.terminal_font_family == "JetBrains Mono",
                                cx.listener(|this, _, _, cx| {
                                    this.update_terminal_font_family("JetBrains Mono", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "appearance-font-iosevka",
                                "Iosevka",
                                self.settings.terminal_font_family == "Iosevka",
                                cx.listener(|this, _, _, cx| {
                                    this.update_terminal_font_family("Iosevka", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "appearance-font-monospace",
                                "monospace",
                                self.settings.terminal_font_family == "monospace",
                                cx.listener(|this, _, _, cx| {
                                    this.update_terminal_font_family("monospace", cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Font size",
                        Some(SharedString::from("Zoom the terminal text without leaving Settings.")),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(palette, 
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
                                    .text_color(rgb(palette.text))
                                    .child(font_size_label),
                            )
                            .child(small_button(palette, 
                                "appearance-font-plus",
                                "+",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_terminal_font_size(1, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "appearance-font-reset",
                                "Reset",
                                cx.listener(|this, _, _, cx| {
                                    this.reset_terminal_font_size(cx);
                                }),
                            )),
                    )),
            ))

            .child(settings_form_section(palette, 
                Some("Cursor"),
                Some("Terminal caret style (Tauri Appearance cursor settings)."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Cursor style",
                        Some(SharedString::from("Block, underline, or vertical bar caret.")),
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(palette, 
                                "appearance-cursor-block",
                                "Block",
                                self.settings.cursor_style == "block",
                                cx.listener(|this, _, _, cx| {
                                    this.set_cursor_style("block", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "appearance-cursor-underline",
                                "Underline",
                                self.settings.cursor_style == "underline",
                                cx.listener(|this, _, _, cx| {
                                    this.set_cursor_style("underline", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "appearance-cursor-bar",
                                "Bar",
                                self.settings.cursor_style == "bar",
                                cx.listener(|this, _, _, cx| {
                                    this.set_cursor_style("bar", cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Cursor blink",
                        Some(SharedString::from("Blink the caret when the terminal is focused.")),
                        settings_switch(palette, 
                            "appearance-cursor-blink",
                            self.settings.cursor_blink,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_cursor_blink(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(palette, 
                Some("X11 display"),
                Some("Used when launching remote X11-forwarded tools."),
                settings_form_row(palette, 
                    "Display",
                    Some(SharedString::from("Forwarded X11 DISPLAY preference.")),
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_1()
                        .child(settings_choice_chip(palette, 
                            "appearance-x11-auto",
                            "Auto",
                            self.settings.x11_display.trim().is_empty(),
                            cx.listener(|this, _, _, cx| {
                                this.update_x11_display("", cx);
                            }),
                        ))
                        .child(settings_choice_chip(palette, 
                            "appearance-x11-localhost0",
                            "localhost:0",
                            self.settings.x11_display == "localhost:0",
                            cx.listener(|this, _, _, cx| {
                                this.update_x11_display("localhost:0", cx);
                            }),
                        ))
                        .child(settings_choice_chip(palette, 
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
        let palette = self.theme_palette();
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
            .child(settings_form_section(palette, 
                Some("Side panels"),
                Some("Allow multiple side panels stacked on each edge."),
                settings_form_row(palette, 
                    "Multi-open panels",
                    Some(SharedString::from("Stack several left/right panels instead of replacing the active one.")),
                    settings_switch(palette, 
                        "settings-panel-multi-open",
                        self.panel_multi_open,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_panel_multi_open(cx);
                        }),
                    ),
                ),
            ))
            .child(settings_form_section(palette, 
                Some("Clipboard and mouse"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Copy on select",
                        Some(SharedString::from("Copy selected terminal text to the clipboard automatically.")),
                        settings_switch(palette, 
                            "interaction-copy-select",
                            self.settings.interaction_copy_on_select,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_interaction_copy_on_select(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Right-click paste",
                        Some(SharedString::from("Paste from clipboard on right-click instead of opening a menu.")),
                        settings_switch(palette, 
                            "interaction-right-paste",
                            self.settings.interaction_right_click_paste,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_interaction_right_click_paste(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Alt as Meta",
                        Some(SharedString::from("Treat Alt as Meta for terminal key bindings.")),
                        settings_switch(palette, 
                            "interaction-alt-meta",
                            self.settings.interaction_alt_as_meta,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_alt_as_meta(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Mac IME compatibility",
                        Some(SharedString::from("Improve input method editor handling on macOS.")),
                        settings_switch(palette, 
                            "interaction-mac-ime",
                            self.settings.interaction_mac_ime_compatibility,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_mac_ime_compatibility(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(palette, 
                Some("Command input"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Command suggestions",
                        Some(SharedString::from("Offer history-based suggestions while typing commands.")),
                        settings_switch(palette, 
                            "interaction-cmd-suggestions",
                            self.settings.interaction_command_suggestions_enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_command_suggestions(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
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
                                    .text_color(rgb(palette.text))
                                    .child(format!(
                                        "{}–{}",
                                        self.settings.interaction_command_suggestion_min_chars,
                                        self.settings.interaction_command_suggestion_max_chars
                                    )),
                            )
                            .child(small_button(palette, 
                                "interaction-suggest-min-minus",
                                "Min −",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_min_chars(-1, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "interaction-suggest-min-plus",
                                "Min +",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_min_chars(1, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "interaction-suggest-max-minus",
                                "Max −",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_max_chars(-1, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "interaction-suggest-max-plus",
                                "Max +",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_max_chars(1, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Default encoding",
                        Some(SharedString::from("Fallback character encoding for session I/O.")),
                        div()
                            .flex()
                            .gap_1()
                            .child(settings_choice_chip(palette, 
                                "interaction-encoding-utf8",
                                "UTF-8",
                                encoding == "UTF-8",
                                cx.listener(|this, _, _, cx| {
                                    this.set_interaction_encoding("UTF-8", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "interaction-encoding-gbk",
                                "GBK",
                                encoding == "GBK",
                                cx.listener(|this, _, _, cx| {
                                    this.set_interaction_encoding("GBK", cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
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
                                    .text_color(rgb(palette.text_muted))
                                    .child(truncate_preview(&word_sep, 24)),
                            )
                            .child(settings_choice_chip(palette, 
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
                            .child(settings_choice_chip(palette, 
                                "interaction-word-sep-basic",
                                "Basic",
                                word_sep == " \t\r\n",
                                cx.listener(|this, _, _, cx| {
                                    this.set_interaction_word_separators(" \t\r\n", cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
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
                                    .text_color(rgb(palette.text))
                                    .child(format!("{delay_ms} ms")),
                            )
                            .child(small_button(palette, 
                                "interaction-dup-delay-minus",
                                "−100",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_duplicate_session_command_delay(-100, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "interaction-dup-delay-plus",
                                "+100",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_duplicate_session_command_delay(100, cx);
                                }),
                            )),
                    )),
            ))
            .child(settings_form_section(palette, 
                Some("Tab mouse actions"),
                Some("What happens when clicking session tabs."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Double-click",
                        Some(SharedString::from(double_action)),
                        small_button(palette, 
                            "interaction-cycle-double",
                            "Cycle",
                            cx.listener(|this, _, _, cx| {
                                this.cycle_tab_mouse_action(TabMouseActionTarget::Double, cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Middle-click",
                        Some(SharedString::from(middle_action)),
                        small_button(palette, 
                            "interaction-cycle-middle",
                            "Cycle",
                            cx.listener(|this, _, _, cx| {
                                this.cycle_tab_mouse_action(TabMouseActionTarget::Middle, cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Right-click",
                        Some(SharedString::from(right_action)),
                        small_button(palette, 
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
        let palette = self.theme_palette();
        // Tauri KeyboardShortcutsTab: section per category + dense shortcut rows.
        let supported = SHORTCUT_REGISTRY
            .iter()
            .filter(|shortcut| shortcut.native_status == ShortcutNativeStatus::Supported)
            .count();
        let pending = SHORTCUT_REGISTRY
            .iter()
            .filter(|shortcut| shortcut.native_status == ShortcutNativeStatus::Pending)
            .count();
        let overrides = self.settings.keybindings.len();
        let mut groups = div().flex().flex_col().gap_3();
        for category in SHORTCUT_CATEGORIES {
            groups = groups.child(self.shortcut_category_group(category, cx));
        }

        div()
            .id("settings-keybindings-panel")
            .flex()
            .flex_col()
            .gap_3()
            .track_focus(&self.keybindings_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.keybindings_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_keybinding_key_down(event, cx);
            }))
            .child(settings_form_section(palette, 
                Some("Keyboard shortcuts"),
                Some("Record overrides stored in the same keybindings map as the Tauri app."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Registry",
                        Some(SharedString::from(format!(
                            "{} total · {supported} native · {pending} pending · {overrides} overrides",
                            SHORTCUT_REGISTRY.len()
                        ))),
                        if overrides > 0 {
                            small_button(palette, 
                                "keybindings-reset-all",
                                "Reset All",
                                cx.listener(|this, _, _, cx| {
                                    this.reset_all_keybindings(cx);
                                }),
                            )
                            .into_any_element()
                        } else {
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette.text_muted))
                                .child("Defaults")
                                .into_any_element()
                        },
                    ))
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.surface_elevated))
                            .bg(rgb(palette.bg))
                            .px_3()
                            .py_2()
                            .text_size(px(11.))
                            .line_height(px(16.))
                            .text_color(rgb(palette.text_muted))
                            .child(
                                "Press Record, type a shortcut, then Save or Enter. Esc cancels recording.",
                            ),
                    ),
            ))
            .child(groups)
    }

    fn shortcut_category_group(
        &mut self,
        category: ShortcutCategory,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let count = SHORTCUT_REGISTRY
            .iter()
            .filter(|shortcut| shortcut.category == category)
            .count();
        let mut rows = div().flex().flex_col().gap_1();
        for shortcut in SHORTCUT_REGISTRY
            .iter()
            .filter(|shortcut| shortcut.category == category)
        {
            rows = rows.child(self.shortcut_registry_row(shortcut, cx));
        }

        settings_form_section(palette, 
            Some(category.label()),
            None,
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(settings_form_row(palette, 
                    "Shortcuts",
                    Some(SharedString::from(format!("{count} in category"))),
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette.text_muted))
                        .child("Native"),
                ))
                .child(rows),
        )
    }

    fn shortcut_registry_row(
        &mut self,
        shortcut: &'static ShortcutDefinition,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let (badge_fg, badge_bg) = match shortcut.native_status {
            ShortcutNativeStatus::Supported => (rgb(palette.success), rgb(0x12261c)),
            ShortcutNativeStatus::Partial => (rgb(palette.accent), rgb(palette.hover)),
            ShortcutNativeStatus::Pending => (rgb(palette.danger), rgb(0x2d1215)),
            ShortcutNativeStatus::Contextual => (rgb(palette.warning), rgb(0x2a2111)),
        };
        let is_custom = self.settings.keybindings.contains_key(shortcut.id);
        let is_recording = self.keybinding_recording_id.as_deref() == Some(shortcut.id);
        let effective_keys = shortcut_keys_for(shortcut.id, &self.settings.keybindings)
            .unwrap_or_else(|| shortcut.default_keys.to_string());
        let key_display = if is_recording {
            self.keybinding_pending_keys
                .as_deref()
                .map(format_hotkey_for_display)
                .unwrap_or_else(|| "Press keys...".to_string())
        } else {
            format_hotkey_for_display(&effective_keys)
        };
        let shortcut_id = shortcut.id.to_string();
        let reset_shortcut_id = shortcut.id.to_string();

        div()
            .rounded_md()
            .px_2()
            .py_1()
            .border_1()
            .border_color(if is_recording {
                rgb(0x1f6feb)
            } else {
                rgb(palette.surface_elevated)
            })
            .bg(if is_recording {
                rgb(palette.hover)
            } else {
                rgb(palette.bg)
            })
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(600.))
                                    .text_color(rgb(palette.text))
                                    .overflow_hidden()
                                    .child(shortcut.label),
                            )
                            .when(is_custom, |this| {
                                this.child(
                                    div()
                                        .text_size(px(10.))
                                        .font_weight(FontWeight(600.))
                                        .text_color(rgb(0xbc8cff))
                                        .child("custom"),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_dimmed))
                            .overflow_hidden()
                            .child(format!("{} · {}", shortcut.id, shortcut.note)),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .rounded_md()
                    .border_1()
                    .border_color(if is_recording {
                        rgb(0x388bfd)
                    } else {
                        rgb(palette.border)
                    })
                    .bg(rgb(palette.surface))
                    .px_2()
                    .py_0()
                    .h(px(24.))
                    .flex()
                    .items_center()
                    .font_family("JetBrains Mono")
                    .text_size(px(10.))
                    .font_weight(FontWeight(700.))
                    .text_color(if is_recording {
                        rgb(palette.accent)
                    } else {
                        rgb(palette.text)
                    })
                    .child(key_display),
            )
            .child(status_pill(
                shortcut.native_status.label(),
                badge_fg,
                badge_bg,
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .when(is_recording, |this| {
                        this.child(small_button(palette, 
                            format!("keybinding-save-{}", shortcut.id),
                            "Save",
                            cx.listener(|this, _, _, cx| {
                                this.confirm_keybinding_recording(cx);
                            }),
                        ))
                        .child(small_button(palette, 
                            format!("keybinding-cancel-{}", shortcut.id),
                            "Cancel",
                            cx.listener(|this, _, _, cx| {
                                this.cancel_keybinding_recording(cx);
                            }),
                        ))
                    })
                    .when(!is_recording, |this| {
                        this.child(small_button(palette, 
                            format!("keybinding-record-{}", shortcut.id),
                            "Record",
                            cx.listener(move |this, _, window, cx| {
                                this.start_keybinding_recording(shortcut_id.clone(), window, cx);
                            }),
                        ))
                    })
                    .when(is_custom && !is_recording, |this| {
                        this.child(small_button(palette, 
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
