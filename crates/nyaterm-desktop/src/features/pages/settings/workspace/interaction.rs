use super::*;

impl NyaTermApp {
    pub(in crate::features) fn interaction_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Tauri InteractionTab: clipboard/mouse, command input, keyboard, tab mouse, encoding.
        let encoding = self.settings.interaction_default_encoding.clone();
        let word_sep = self.settings.interaction_word_separators.clone();
        let double_action = self.settings.interaction_tab_double_click_action.clone();
        let middle_action = self.settings.interaction_tab_middle_click_action.clone();
        let right_action = self.settings.interaction_tab_right_click_action.clone();
        let delay_ms = self.settings.interaction_duplicate_session_command_delay_ms;
        let min_chars = self.settings.interaction_command_suggestion_min_chars;
        let max_chars = self.settings.interaction_command_suggestion_max_chars;
        let suggestions_enabled = self.settings.interaction_command_suggestions_enabled;

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                palette,
                Some("Clipboard and mouse"),
                Some("Selection copy and right-click paste behavior."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "Copy on select",
                        Some(SharedString::from(
                            "Copy selected terminal text to the clipboard automatically.",
                        )),
                        settings_switch(
                            palette,
                            "interaction-copy-select",
                            self.settings.interaction_copy_on_select,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_interaction_copy_on_select(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Right-click paste",
                        Some(SharedString::from(
                            "Paste from clipboard on right-click instead of opening a menu.",
                        )),
                        settings_switch(
                            palette,
                            "interaction-right-paste",
                            self.settings.interaction_right_click_paste,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_interaction_right_click_paste(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("Command input"),
                Some("Suggestions, word separators, and duplicate-session command delay."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "Command suggestions",
                        Some(SharedString::from(
                            "Offer history-based suggestions while typing commands.",
                        )),
                        settings_switch(
                            palette,
                            "interaction-cmd-suggestions",
                            suggestions_enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_command_suggestions(cx);
                            }),
                        ),
                    ))
                    .when(suggestions_enabled, |this| {
                        this.child(
                            div()
                                .pl_3()
                                .ml_1()
                                .border_l_1()
                                .border_color(rgb(palette.border))
                                .flex()
                                .flex_col()
                                .gap_3()
                                .child(settings_form_row(
                                    palette,
                                    "Min characters",
                                    Some(SharedString::from(
                                        "Start offering suggestions after this many typed characters.",
                                    )),
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            div()
                                                .min_w(px(28.))
                                                .font_family("JetBrains Mono")
                                                .text_size(px(11.))
                                                .text_color(rgb(palette.text))
                                                .child(min_chars.to_string()),
                                        )
                                        .child(small_button(
                                            palette,
                                            "interaction-suggest-min-minus",
                                            "−",
                                            cx.listener(|this, _, _, cx| {
                                                this.adjust_command_suggestion_min_chars(-1, cx);
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            "interaction-suggest-min-plus",
                                            "+",
                                            cx.listener(|this, _, _, cx| {
                                                this.adjust_command_suggestion_min_chars(1, cx);
                                            }),
                                        )),
                                ))
                                .child(settings_form_row(
                                    palette,
                                    "Max characters",
                                    Some(SharedString::from(
                                        "Ignore suggestion matching once the line exceeds this length.",
                                    )),
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            div()
                                                .min_w(px(28.))
                                                .font_family("JetBrains Mono")
                                                .text_size(px(11.))
                                                .text_color(rgb(palette.text))
                                                .child(max_chars.to_string()),
                                        )
                                        .child(small_button(
                                            palette,
                                            "interaction-suggest-max-minus",
                                            "−",
                                            cx.listener(|this, _, _, cx| {
                                                this.adjust_command_suggestion_max_chars(-1, cx);
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            "interaction-suggest-max-plus",
                                            "+",
                                            cx.listener(|this, _, _, cx| {
                                                this.adjust_command_suggestion_max_chars(1, cx);
                                            }),
                                        )),
                                )),
                        )
                    })
                    .child(settings_form_row(
                        palette,
                        "Word separators",
                        Some(SharedString::from(
                            "Characters that split double-click word selection.",
                        )),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(truncate_preview(&word_sep, 28)),
                            )
                            .child(settings_choice_chip(
                                palette,
                                "interaction-word-sep-shell",
                                "Shell",
                                word_sep.contains('/') && word_sep.contains('|'),
                                cx.listener(|this, _, _, cx| {
                                    this.set_interaction_word_separators(
                                        " `\"'()[]{}<>|&;/",
                                        cx,
                                    );
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "interaction-word-sep-basic",
                                "Basic",
                                word_sep == " \t\r\n",
                                cx.listener(|this, _, _, cx| {
                                    this.set_interaction_word_separators(" \t\r\n", cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Duplicate session delay",
                        Some(SharedString::from(
                            "Delay before replaying the startup command on a duplicated tab.",
                        )),
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
                            .child(small_button(
                                palette,
                                "interaction-dup-delay-minus",
                                "−100",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_duplicate_session_command_delay(-100, cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "interaction-dup-delay-plus",
                                "+100",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_duplicate_session_command_delay(100, cx);
                                }),
                            )),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("Keyboard"),
                Some("Terminal meta key and macOS IME compatibility."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "Alt as Meta",
                        Some(SharedString::from(
                            "Treat Alt as Meta for terminal key bindings.",
                        )),
                        settings_switch(
                            palette,
                            "interaction-alt-meta",
                            self.settings.interaction_alt_as_meta,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_alt_as_meta(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Mac IME compatibility",
                        Some(SharedString::from(
                            "Improve input method editor handling on macOS.",
                        )),
                        settings_switch(
                            palette,
                            "interaction-mac-ime",
                            self.settings.interaction_mac_ime_compatibility,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_mac_ime_compatibility(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("Tab mouse actions"),
                Some("What happens when clicking session tabs (Tauri Interaction selects)."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(self.tab_mouse_action_settings_row(
                        palette,
                        "Double-click",
                        "interaction-tab-double",
                        TabMouseActionTarget::Double,
                        &double_action,
                        cx,
                    ))
                    .child(self.tab_mouse_action_settings_row(
                        palette,
                        "Middle-click",
                        "interaction-tab-middle",
                        TabMouseActionTarget::Middle,
                        &middle_action,
                        cx,
                    ))
                    .child(self.tab_mouse_action_settings_row(
                        palette,
                        "Right-click",
                        "interaction-tab-right",
                        TabMouseActionTarget::Right,
                        &right_action,
                        cx,
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("Encoding"),
                Some("Fallback character encoding for session I/O."),
                settings_form_row(
                    palette,
                    "Default encoding",
                    Some(SharedString::from(
                        "Used when a session does not specify an encoding.",
                    )),
                    div()
                        .flex()
                        .gap_1()
                        .child(settings_choice_chip(
                            palette,
                            "interaction-encoding-utf8",
                            "UTF-8",
                            encoding == "UTF-8",
                            cx.listener(|this, _, _, cx| {
                                this.set_interaction_encoding("UTF-8", cx);
                            }),
                        ))
                        .child(settings_choice_chip(
                            palette,
                            "interaction-encoding-gbk",
                            "GBK",
                            encoding == "GBK",
                            cx.listener(|this, _, _, cx| {
                                this.set_interaction_encoding("GBK", cx);
                            }),
                        )),
                ),
            ))
    }

    pub(in crate::features) fn tab_mouse_action_settings_row(
        &mut self,
        palette: ThemePalette,
        label: &'static str,
        id_prefix: &'static str,
        target: TabMouseActionTarget,
        current: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let chips = TAB_MOUSE_ACTIONS.iter().fold(
            div().flex().flex_wrap().gap_1().max_w(px(420.)),
            |row, action| {
                let action_id = (*action).to_string();
                let selected = current == *action;
                let chip_id = format!("{id_prefix}-{action}");
                let short_static: &'static str = match *action {
                    "none" => "None",
                    "rename_tab" => "Rename",
                    "copy_tab_name" => "Copy name",
                    "copy_server_ip" => "Copy IP",
                    "duplicate_session" => "Duplicate",
                    "multiplex_ssh" => "Mux SSH",
                    "reconnect_session" => "Reconnect",
                    "disconnect_session" => "Disconnect",
                    "close_tab" => "Close",
                    _ => "None",
                };
                row.child(settings_choice_chip(
                    palette,
                    chip_id,
                    short_static,
                    selected,
                    cx.listener(move |this, _, _, cx| {
                        this.set_tab_mouse_action(target, &action_id, cx);
                    }),
                ))
            },
        );
        settings_form_row(
            palette,
            label,
            Some(SharedString::from(tab_mouse_action_label(current))),
            chips,
        )
    }
}
