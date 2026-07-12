use super::*;
use gpui::{App, ClickEvent, SharedString, Window};

impl NyaTermApp {
    pub(super) fn terminal_general_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Tauri TerminalTab general: number rows + feature switches (no metric cards).
        let scrollback = self.settings.terminal_scrollback_lines.to_string();
        let keep_alive = self.settings.terminal_keep_alive_interval.to_string();
        let remote_stats_interval = self.settings.ui_remote_stats_interval.to_string();
        let process_interval = self.settings.ui_process_manager_interval.to_string();
        let docker_interval = self.settings.ui_docker_manager_interval.to_string();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(palette, 
                Some("Session"),
                Some("Scrollback depth and SSH keep-alive cadence."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Scrollback lines",
                        Some(SharedString::from("How many terminal lines to retain for scrollback.")),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .min_w(px(72.))
                                    .font_family("JetBrains Mono")
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(600.))
                                    .text_color(rgb(palette.text))
                                    .child(scrollback),
                            )
                            .child(small_button(palette, 
                                "terminal-scrollback-minus",
                                "−100",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_terminal_scrollback_lines(-100, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "terminal-scrollback-plus",
                                "+100",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_terminal_scrollback_lines(100, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Keep-alive interval",
                        Some(SharedString::from("Seconds between keep-alive packets (0 disables).")),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .min_w(px(56.))
                                    .font_family("JetBrains Mono")
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(600.))
                                    .text_color(rgb(palette.text))
                                    .child(format!("{keep_alive}s")),
                            )
                            .child(small_button(palette, 
                                "terminal-keepalive-minus",
                                "−5",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_terminal_keep_alive_interval(-5, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "terminal-keepalive-plus",
                                "+5",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_terminal_keep_alive_interval(5, cx);
                                }),
                            )),
                    )),
            ))
            .child(settings_form_section(palette, 
                Some("Display"),
                Some("Terminal chrome toggles matching Tauri TerminalTab."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Hardware acceleration",
                        Some(SharedString::from("Prefer GPU-accelerated terminal rendering when available.")),
                        settings_switch(palette, 
                            "terminal-hw-accel",
                            self.settings.terminal_hardware_acceleration,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_terminal_hardware_acceleration(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Workspace padding",
                        Some(SharedString::from("Add breathing room around the terminal surface.")),
                        settings_switch(palette, 
                            "terminal-workspace-padding",
                            self.settings.terminal_show_workspace_padding,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_terminal_workspace_padding(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Line numbers",
                        Some(SharedString::from("Prefix rendered terminal rows with line numbers.")),
                        settings_switch(palette, 
                            "terminal-line-numbers",
                            self.settings.terminal_show_line_numbers,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_terminal_line_numbers(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Timestamps",
                        Some(SharedString::from("Show per-line timestamps when metadata is available.")),
                        settings_switch(palette, 
                            "terminal-timestamps",
                            self.settings.terminal_show_timestamps,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_terminal_timestamps(cx);
                            }),
                        ),
                    ))
                    .when(self.settings.terminal_show_timestamps, |this| {
                        this.child(
                            div()
                                .pl_3()
                                .ml_1()
                                .border_l_1()
                                .border_color(rgb(palette.border))
                                .child(settings_form_row(
                                    palette,
                                    "Timestamp milliseconds",
                                    Some(SharedString::from(
                                        "Include millisecond precision on timestamps.",
                                    )),
                                    settings_switch(
                                        palette,
                                        "terminal-timestamp-ms",
                                        self.settings.terminal_show_timestamp_milliseconds,
                                        cx.listener(|this, _, _, cx| {
                                            this.toggle_terminal_timestamp_milliseconds(cx);
                                        }),
                                    ),
                                )),
                        )
                    })
                    .child(settings_form_row(palette, 
                        "Multi-line paste dialog",
                        Some(SharedString::from("Confirm multi-line pastes before sending them to the session.")),
                        settings_switch(palette, 
                            "terminal-multi-line-paste",
                            self.settings.terminal_show_multi_line_paste_dialog,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_multi_line_paste_dialog(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Paste image as path",
                        Some(SharedString::from("When pasting an image, insert a temp file path instead of binary data.")),
                        settings_switch(palette, 
                            "terminal-paste-image-path",
                            self.settings.terminal_paste_image_as_path,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_paste_image_as_path(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Remote stats",
                        Some(SharedString::from("Show remote host resource stats in the activity bar.")),
                        settings_switch(palette, 
                            "terminal-remote-stats",
                            self.settings.ui_show_remote_stats,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_remote_stats_panel(cx);
                            }),
                        ),
                    ))
                    .when(self.settings.ui_show_remote_stats, |this| {
                        this.child(
                            div()
                                .pl_3()
                                .ml_1()
                                .border_l_1()
                                .border_color(rgb(palette.border))
                                .child(settings_form_row(
                                    palette,
                                    "Remote stats interval",
                                    Some(SharedString::from("Seconds between remote stats refreshes.")),
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            div()
                                                .min_w(px(40.))
                                                .font_family("JetBrains Mono")
                                                .text_size(px(11.))
                                                .text_color(rgb(palette.text))
                                                .child(format!("{remote_stats_interval}s")),
                                        )
                                        .child(small_button(
                                            palette,
                                            "terminal-remote-stats-interval-minus",
                                            "−",
                                            cx.listener(|this, _, _, cx| {
                                                this.adjust_remote_stats_interval(-1, cx);
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            "terminal-remote-stats-interval-plus",
                                            "+",
                                            cx.listener(|this, _, _, cx| {
                                                this.adjust_remote_stats_interval(1, cx);
                                            }),
                                        )),
                                )),
                        )
                    })
                    .child(settings_form_row(palette, 
                        "Process manager",
                        Some(SharedString::from("Show remote process manager panel.")),
                        settings_switch(palette, 
                            "terminal-process-manager",
                            self.settings.ui_show_process_manager,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_process_manager_panel(cx);
                            }),
                        ),
                    ))
                    .when(self.settings.ui_show_process_manager, |this| {
                        this.child(
                            div()
                                .pl_3()
                                .ml_1()
                                .border_l_1()
                                .border_color(rgb(palette.border))
                                .child(settings_form_row(
                                    palette,
                                    "Process manager interval",
                                    Some(SharedString::from("Seconds between process list refreshes.")),
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            div()
                                                .min_w(px(40.))
                                                .font_family("JetBrains Mono")
                                                .text_size(px(11.))
                                                .text_color(rgb(palette.text))
                                                .child(format!("{process_interval}s")),
                                        )
                                        .child(small_button(
                                            palette,
                                            "terminal-process-interval-minus",
                                            "−",
                                            cx.listener(|this, _, _, cx| {
                                                this.adjust_process_manager_interval(-1, cx);
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            "terminal-process-interval-plus",
                                            "+",
                                            cx.listener(|this, _, _, cx| {
                                                this.adjust_process_manager_interval(1, cx);
                                            }),
                                        )),
                                )),
                        )
                    })
                    .child(settings_form_row(palette, 
                        "Docker manager",
                        Some(SharedString::from("Show remote Docker manager panel.")),
                        settings_switch(palette, 
                            "terminal-docker-manager",
                            self.settings.ui_show_docker_manager,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_docker_manager_panel(cx);
                            }),
                        ),
                    ))
                    .when(self.settings.ui_show_docker_manager, |this| {
                        this.child(
                            div()
                                .pl_3()
                                .ml_1()
                                .border_l_1()
                                .border_color(rgb(palette.border))
                                .child(settings_form_row(
                                    palette,
                                    "Docker manager interval",
                                    Some(SharedString::from("Seconds between Docker refreshes.")),
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            div()
                                                .min_w(px(40.))
                                                .font_family("JetBrains Mono")
                                                .text_size(px(11.))
                                                .text_color(rgb(palette.text))
                                                .child(format!("{docker_interval}s")),
                                        )
                                        .child(small_button(
                                            palette,
                                            "terminal-docker-interval-minus",
                                            "−",
                                            cx.listener(|this, _, _, cx| {
                                                this.adjust_docker_manager_interval(-1, cx);
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            "terminal-docker-interval-plus",
                                            "+",
                                            cx.listener(|this, _, _, cx| {
                                                this.adjust_docker_manager_interval(1, cx);
                                            }),
                                        )),
                                )),
                        )
                    }),
            ))
            .child(settings_form_section(palette,
                Some("Action links"),
                Some("Detect IP / host:port / archives in terminal text for quick commands."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette,
                        "Enabled",
                        Some(SharedString::from("Ctrl/Cmd-click runs the default action; context menu lists all.")),
                        settings_switch(palette,
                            "terminal-action-links",
                            self.settings.terminal_action_links_enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_terminal_action_links(cx);
                            }),
                        ),
                    ))
                    .when(self.settings.terminal_action_links_enabled, |this| {
                        this.child(
                            div()
                                .pl_3()
                                .ml_1()
                                .border_l_1()
                                .border_color(rgb(palette.border))
                                .flex()
                                .flex_col()
                                .gap_3()
                                .child(settings_form_row(palette,
                                    "IPv4 matcher",
                                    None,
                                    settings_switch(palette,
                                        "terminal-action-links-ipv4",
                                        self.settings.terminal_action_links_matchers.ipv4,
                                        cx.listener(|this, _, _, cx| {
                                            this.toggle_terminal_action_links_matcher("ipv4", cx);
                                        }),
                                    ),
                                ))
                                .child(settings_form_row(palette,
                                    "Host:Port matcher",
                                    None,
                                    settings_switch(palette,
                                        "terminal-action-links-host-port",
                                        self.settings.terminal_action_links_matchers.host_port,
                                        cx.listener(|this, _, _, cx| {
                                            this.toggle_terminal_action_links_matcher("host_port", cx);
                                        }),
                                    ),
                                ))
                                .child(settings_form_row(palette,
                                    "Archive matcher",
                                    None,
                                    settings_switch(palette,
                                        "terminal-action-links-archive",
                                        self.settings.terminal_action_links_matchers.archive,
                                        cx.listener(|this, _, _, cx| {
                                            this.toggle_terminal_action_links_matcher("archive", cx);
                                        }),
                                    ),
                                )),
                        )
                    }),
            ))
    }

    pub(super) fn terminal_search_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                palette,
                Some("Terminal search"),
                Some("Default flags for in-buffer find."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "Search mode",
                        Some(SharedString::from(
                            "Buffer searches the live terminal; History searches session logs.",
                        )),
                        div()
                            .flex()
                            .gap_1()
                            .child(settings_choice_chip(
                                palette,
                                "settings-search-mode-buffer",
                                "Buffer",
                                self.terminal_search_mode == TerminalSearchMode::Buffer,
                                cx.listener(|this, _, _, cx| {
                                    this.terminal_search_mode = TerminalSearchMode::Buffer;
                                    this.terminal_search_active_index = 0;
                                    cx.notify();
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "settings-search-mode-history",
                                "History",
                                self.terminal_search_mode == TerminalSearchMode::History,
                                cx.listener(|this, _, _, cx| {
                                    this.terminal_search_mode = TerminalSearchMode::History;
                                    this.terminal_search_active_index = 0;
                                    cx.notify();
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Case sensitive",
                        None,
                        settings_switch(
                            palette,
                            "settings-search-case",
                            self.terminal_search_case_sensitive,
                            cx.listener(|this, _, _, cx| {
                                this.terminal_search_case_sensitive =
                                    !this.terminal_search_case_sensitive;
                                this.terminal_search_active_index = 0;
                                cx.notify();
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Regular expression",
                        None,
                        settings_switch(
                            palette,
                            "settings-search-regex",
                            self.terminal_search_regex,
                            cx.listener(|this, _, _, cx| {
                                this.terminal_search_regex = !this.terminal_search_regex;
                                this.terminal_search_active_index = 0;
                                cx.notify();
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Whole word",
                        None,
                        settings_switch(
                            palette,
                            "settings-search-word",
                            self.terminal_search_whole_word,
                            cx.listener(|this, _, _, cx| {
                                this.terminal_search_whole_word = !this.terminal_search_whole_word;
                                this.terminal_search_active_index = 0;
                                cx.notify();
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Open search",
                        Some(SharedString::from(
                            "Focus the terminal search bar in the workspace.",
                        )),
                        small_button(
                            palette,
                            "settings-search-open",
                            "Open",
                            cx.listener(|this, _, window, cx| {
                                this.open_terminal_search(window, cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("Command search"),
                Some("Shared matcher sources for history and quick commands."),
                settings_form_row(
                    palette,
                    "Catalog",
                    Some(SharedString::from(format!(
                        "{} history · {} quick commands",
                        self.command_history.len(),
                        self.quick_commands.len()
                    ))),
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette.text_muted))
                        .child("Native fuzzy"),
                ),
            ))
            .child(self.online_search_engines_settings_section(cx))
            .child(self.keyword_highlights_settings_section(cx))
    }

    fn online_search_engines_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let engines = self.settings.search_custom_engines.clone();
        let menu_count = engines.iter().filter(|engine| engine.show_in_menu).count();
        let edit_index = self.search_engine_edit_index;
        let expanded_index = self.search_engine_expanded_index;
        let edit_field = self.search_engine_edit_field;

        settings_form_section(
            palette,
            Some("Online search engines"),
            Some("Engines shown on the terminal selection context menu (Tauri Search tab)."),
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(settings_form_row(
                    palette,
                    "Catalog",
                    Some(SharedString::from(format!(
                        "{} engines · {} in menu",
                        engines.len(),
                        menu_count
                    ))),
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(small_button(
                            palette,
                            "settings-search-engine-add",
                            "Add",
                            cx.listener(|this, _, window, cx| {
                                this.add_search_engine(cx);
                                window.focus(&this.search_engine_focus);
                            }),
                        ))
                        .child(small_button(
                            palette,
                            "settings-search-engine-reset",
                            "Reset",
                            cx.listener(|this, _, _, cx| {
                                this.reset_search_engines(cx);
                            }),
                        )),
                ))
                .when(engines.is_empty(), |this| {
                    this.child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.input))
                            .px_4()
                            .py_6()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(palette.text_dimmed))
                                    .child(
                                        "No custom engines — Add one for selection online search.",
                                    ),
                            ),
                    )
                })
                .children(engines.into_iter().enumerate().map(|(index, engine)| {
                    let is_open = expanded_index == Some(index);
                    let name_active =
                        edit_index == Some(index) && edit_field == SearchEngineEditorField::Name;
                    let url_active =
                        edit_index == Some(index) && edit_field == SearchEngineEditorField::Url;
                    let name_value = if engine.name.is_empty() {
                        " ".to_string()
                    } else {
                        engine.name.clone()
                    };
                    let url_value = if engine.url_template.is_empty() {
                        " ".to_string()
                    } else {
                        engine.url_template.clone()
                    };
                    let has_placeholder = engine.url_template.contains("%s");
                    let icon_label = search_engine_icon_label(engine.icon.as_deref());
                    let icon_color = search_engine_icon_color(engine.icon.as_deref());
                    div()
                        .id(SharedString::from(format!(
                            "settings-search-engine-{index}"
                        )))
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .bg(rgb(palette.input))
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .id(SharedString::from(format!(
                                    "settings-search-engine-header-{index}"
                                )))
                                .px_3()
                                .py_2()
                                .flex()
                                .items_center()
                                .gap_2()
                                .cursor_pointer()
                                .hover(|this| this.bg(rgb(palette.hover)))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.expand_search_engine(index, cx);
                                }))
                                .child(
                                    div()
                                        .id(SharedString::from(format!(
                                            "settings-search-engine-icon-{index}"
                                        )))
                                        .size(px(22.))
                                        .rounded_md()
                                        .bg(rgb(palette.bg))
                                        .border_1()
                                        .border_color(rgb(palette.border))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_size(px(10.))
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(icon_color))
                                        .child(icon_label)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.cycle_search_engine_icon(index, cx);
                                        })),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .font_weight(FontWeight(700.))
                                                .text_color(rgb(palette.text))
                                                .overflow_hidden()
                                                .child(if engine.name.trim().is_empty() {
                                                    "Unnamed engine".to_string()
                                                } else {
                                                    engine.name.clone()
                                                }),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(rgb(palette.text_muted))
                                                .overflow_hidden()
                                                .child(if engine.url_template.trim().is_empty() {
                                                    "No URL template".to_string()
                                                } else {
                                                    truncate_preview(&engine.url_template, 48)
                                                }),
                                        ),
                                )
                                .child(settings_switch(
                                    palette,
                                    format!("settings-search-engine-menu-{index}"),
                                    engine.show_in_menu,
                                    cx.listener(move |this, _, _, cx| {
                                        this.toggle_search_engine_in_menu(index, cx);
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    format!("settings-search-engine-test-{index}"),
                                    "Test",
                                    cx.listener(move |this, _, _, cx| {
                                        this.test_search_engine(index, cx);
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    format!("settings-search-engine-del-{index}"),
                                    "Delete",
                                    cx.listener(move |this, _, _, cx| {
                                        this.remove_search_engine(index, cx);
                                    }),
                                ))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .child(if is_open { "▾" } else { "▸" }),
                                ),
                        )
                        .when(is_open, |this| {
                            this.child(
                                div()
                                    .border_t_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.bg))
                                    .px_3()
                                    .py_3()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(
                                        transfer_input(
                                            format!("settings-search-engine-name-{index}"),
                                            "Name",
                                            name_value,
                                            name_active,
                                            palette,
                                        )
                                        .track_focus(&self.search_engine_focus)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.focus_search_engine_field(
                                                index,
                                                SearchEngineEditorField::Name,
                                                window,
                                                cx,
                                            );
                                        }))
                                        .on_key_down(
                                            cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                                cx.stop_propagation();
                                                this.handle_search_engine_key_down(event, cx);
                                            }),
                                        ),
                                    )
                                    .child(
                                        transfer_input(
                                            format!("settings-search-engine-url-{index}"),
                                            "URL template (%s = query)",
                                            url_value,
                                            url_active,
                                            palette,
                                        )
                                        .track_focus(&self.search_engine_focus)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.focus_search_engine_field(
                                                index,
                                                SearchEngineEditorField::Url,
                                                window,
                                                cx,
                                            );
                                        }))
                                        .on_key_down(
                                            cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                                cx.stop_propagation();
                                                this.handle_search_engine_key_down(event, cx);
                                            }),
                                        ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(rgb(if has_placeholder {
                                                palette.text_muted
                                            } else {
                                                palette.danger
                                            }))
                                            .child(if has_placeholder {
                                                "Ready — Test opens the URL with query \"nyaterm\"."
                                                    .to_string()
                                            } else {
                                                "URL should include %s for the selected text"
                                                    .to_string()
                                            }),
                                    ),
                            )
                        })
                })),
        )
    }

    fn keyword_highlights_settings_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let is_dark = self.terminal_theme_is_dark();
        let rules = self.keyword_highlights.rules.clone();
        let active = rules.iter().filter(|rule| rule.enabled).count();
        let expanded = self.keyword_highlight_expanded_id.clone();
        let edit_id = self.keyword_highlight_edit_id.clone();
        let edit_field = self.keyword_highlight_edit_field;
        let prompt = match self.keyword_highlight_path_prompt {
            Some(KeywordHighlightPathPromptKind::Import) => "selecting import file",
            None => "legacy JSON import",
        };
        let builtin_ids = nyaterm_domain::builtin_keyword_rule_ids();

        settings_form_section(
            palette,
            Some("Keyword highlights"),
            Some("Match terminal output keywords with colored highlights (Tauri TerminalTab)."),
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(settings_form_row(
                    palette,
                    "Enabled",
                    Some(SharedString::from(format!(
                        "{active}/{} custom rules · {prompt}",
                        rules.len()
                    ))),
                    settings_switch(
                        palette,
                        "settings-keyword-highlights-enabled",
                        self.keyword_highlights.enabled,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_keyword_highlights(cx);
                        }),
                    ),
                ))
                .when(self.keyword_highlights.enabled, |this| {
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
                                "Across wrapped lines",
                                Some(SharedString::from(
                                    "Continue matches across soft-wrapped terminal lines.",
                                )),
                                settings_switch(
                                    palette,
                                    "settings-keyword-highlights-wrap",
                                    self.keyword_highlights.across_wrapped_lines,
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_keyword_highlights_wrapped(cx);
                                    }),
                                ),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight(600.))
                                            .text_color(rgb(palette.text))
                                            .child("Built-in rules"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(rgb(palette.text_muted))
                                            .child(
                                                "Toggle catalog rules used when no custom rule matches.",
                                            ),
                                    )
                                    .child(
                                        div()
                                            .grid()
                                            .grid_cols(2)
                                            .gap_2()
                                            .children(builtin_ids.iter().map(|id| {
                                                let id = (*id).to_string();
                                                let label =
                                                    nyaterm_domain::builtin_keyword_rule_label(&id);
                                                let swatch =
                                                    nyaterm_domain::builtin_keyword_rule_swatch(
                                                        &id, is_dark,
                                                    );
                                                let enabled = self
                                                    .keyword_highlights
                                                    .builtin_rules
                                                    .get(&id)
                                                    .copied()
                                                    .unwrap_or(true);
                                                let color =
                                                    parse_keyword_swatch(swatch).unwrap_or(0x79c0ff);
                                                let rid = id.clone();
                                                div()
                                                    .rounded_md()
                                                    .border_1()
                                                    .border_color(rgb(palette.border))
                                                    .bg(rgb(palette.bg))
                                                    .px_3()
                                                    .py_2()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .gap_2()
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .gap_2()
                                                            .min_w_0()
                                                            .child(
                                                                div()
                                                                    .size(px(10.))
                                                                    .rounded_full()
                                                                    .bg(rgb(color))
                                                                    .border_1()
                                                                    .border_color(rgb(
                                                                        palette.border,
                                                                    )),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_size(px(12.))
                                                                    .text_color(rgb(
                                                                        palette.text_muted,
                                                                    ))
                                                                    .overflow_hidden()
                                                                    .child(label),
                                                            ),
                                                    )
                                                    .child(settings_switch(
                                                        palette,
                                                        format!(
                                                            "settings-keyword-builtin-{id}"
                                                        ),
                                                        enabled,
                                                        cx.listener(move |this, _, _, cx| {
                                                            this.toggle_keyword_highlight_builtin(
                                                                rid.clone(),
                                                                cx,
                                                            );
                                                        }),
                                                    ))
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight(600.))
                                            .text_color(rgb(palette.text))
                                            .child("Custom rules"),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .child(small_button(
                                                palette,
                                                "settings-keyword-highlights-import",
                                                "Import",
                                                cx.listener(|this, _, _, cx| {
                                                    this.prompt_keyword_highlight_import(cx);
                                                }),
                                            ))
                                            .child(small_button(
                                                palette,
                                                "settings-keyword-highlights-add",
                                                "Add",
                                                cx.listener(|this, _, window, cx| {
                                                    this.add_keyword_highlight_rule(window, cx);
                                                }),
                                            )),
                                    ),
                            )
                            .when(rules.is_empty(), |this| {
                                this.child(
                                    div()
                                        .rounded_md()
                                        .border_1()
                                        .border_color(rgb(palette.border))
                                        .bg(rgb(palette.input))
                                        .px_4()
                                        .py_6()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(rgb(palette.text_dimmed))
                                                .child(
                                                    "No custom rules — Add one or Import legacy JSON.",
                                                ),
                                        ),
                                )
                            })
                            .children(rules.into_iter().map(|rule| {
                                let is_open = expanded.as_deref() == Some(rule.id.as_str());
                                let pattern_count =
                                    rule.patterns.iter().filter(|p| !p.trim().is_empty()).count();
                                let swatch = if is_dark {
                                    rule.color_dark.as_str()
                                } else {
                                    rule.color_light.as_str()
                                };
                                let color = parse_keyword_swatch(swatch).unwrap_or(0x79c0ff);
                                let rule_id = rule.id.clone();
                                let rule_id_toggle = rule.id.clone();
                                let rule_id_delete = rule.id.clone();
                                let name_active = edit_id.as_deref() == Some(rule.id.as_str())
                                    && edit_field == KeywordHighlightEditorField::Name;
                                let patterns_active = edit_id.as_deref()
                                    == Some(rule.id.as_str())
                                    && edit_field == KeywordHighlightEditorField::Patterns;
                                let dark_active = edit_id.as_deref() == Some(rule.id.as_str())
                                    && edit_field == KeywordHighlightEditorField::ColorDark;
                                let light_active = edit_id.as_deref() == Some(rule.id.as_str())
                                    && edit_field == KeywordHighlightEditorField::ColorLight;
                                let name_value = if rule.name.is_empty() {
                                    " ".to_string()
                                } else {
                                    rule.name.clone()
                                };
                                let patterns_value = if rule.patterns.is_empty() {
                                    " ".to_string()
                                } else {
                                    rule.patterns.join("\n")
                                };
                                let dark_value = if rule.color_dark.is_empty() {
                                    " ".to_string()
                                } else {
                                    rule.color_dark.clone()
                                };
                                let light_value = if rule.color_light.is_empty() {
                                    " ".to_string()
                                } else {
                                    rule.color_light.clone()
                                };
                                let palette_dark = nyaterm_domain::keyword_highlight_color_palette(true);
                                let palette_light =
                                    nyaterm_domain::keyword_highlight_color_palette(false);

                                div()
                                    .id(SharedString::from(format!(
                                        "settings-keyword-rule-{}",
                                        rule.id
                                    )))
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.input))
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .id(SharedString::from(format!(
                                                "settings-keyword-rule-header-{}",
                                                rule.id
                                            )))
                                            .px_3()
                                            .py_2()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .cursor_pointer()
                                            .hover(|this| this.bg(rgb(palette.hover)))
                                            .on_click(cx.listener({
                                                let rule_id = rule_id.clone();
                                                move |this, _, _, cx| {
                                                    this.expand_keyword_highlight_rule(
                                                        rule_id.clone(),
                                                        cx,
                                                    );
                                                }
                                            }))
                                            .child(
                                                div()
                                                    .size(px(10.))
                                                    .rounded_full()
                                                    .bg(rgb(color))
                                                    .border_1()
                                                    .border_color(rgb(palette.border))
                                                    .flex_none(),
                                            )
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .flex_1()
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight(600.))
                                                    .text_color(rgb(palette.text))
                                                    .overflow_hidden()
                                                    .child(if rule.name.trim().is_empty() {
                                                        "Untitled rule".to_string()
                                                    } else {
                                                        rule.name.clone()
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .text_color(rgb(palette.text_muted))
                                                    .child(format!("{pattern_count} patterns")),
                                            )
                                            .child(settings_switch(
                                                palette,
                                                format!("settings-keyword-rule-enabled-{}", rule.id),
                                                rule.enabled,
                                                cx.listener(move |this, _, _, cx| {
                                                    this.toggle_keyword_highlight_rule(
                                                        rule_id_toggle.clone(),
                                                        cx,
                                                    );
                                                }),
                                            ))
                                            .child(small_button(
                                                palette,
                                                format!("settings-keyword-rule-delete-{}", rule.id),
                                                "Delete",
                                                cx.listener(move |this, _, _, cx| {
                                                    this.remove_keyword_highlight_rule(
                                                        rule_id_delete.clone(),
                                                        cx,
                                                    );
                                                }),
                                            ))
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(rgb(palette.text_dimmed))
                                                    .child(if is_open { "▾" } else { "▸" }),
                                            ),
                                    )
                                    .when(is_open, |this| {
                                        this.child(
                                            div()
                                                .border_t_1()
                                                .border_color(rgb(palette.border))
                                                .bg(rgb(palette.bg))
                                                .px_3()
                                                .py_3()
                                                .flex()
                                                .flex_col()
                                                .gap_3()
                                                .track_focus(&self.keyword_highlight_focus)
                                                .on_key_down(cx.listener(
                                                    |this, event: &KeyDownEvent, _, cx| {
                                                        this.handle_keyword_highlight_key_down(
                                                            event, cx,
                                                        );
                                                    },
                                                ))
                                                .child(settings_form_row(
                                                    palette,
                                                    "Name",
                                                    None,
                                                    div()
                                                        .id(SharedString::from(format!(
                                                            "settings-keyword-rule-name-{}",
                                                            rule.id
                                                        )))
                                                        .min_w(px(160.))
                                                        .h(px(28.))
                                                        .px_2()
                                                        .rounded_md()
                                                        .border_1()
                                                        .border_color(if name_active {
                                                            rgb(palette.accent)
                                                        } else {
                                                            rgb(palette.border)
                                                        })
                                                        .bg(rgb(palette.input))
                                                        .flex()
                                                        .items_center()
                                                        .text_size(px(12.))
                                                        .text_color(rgb(palette.text))
                                                        .cursor_pointer()
                                                        .child(name_value)
                                                        .on_click(cx.listener({
                                                            let rule_id = rule_id.clone();
                                                            move |this, _, window, cx| {
                                                                this.focus_keyword_highlight_field(
                                                                    rule_id.clone(),
                                                                    KeywordHighlightEditorField::Name,
                                                                    window,
                                                                    cx,
                                                                );
                                                            }
                                                        })),
                                                ))
                                                .child(
                                                    div()
                                                        .flex()
                                                        .flex_col()
                                                        .gap_2()
                                                        .child(
                                                            div()
                                                                .text_size(px(11.))
                                                                .text_color(rgb(
                                                                    palette.text_muted,
                                                                ))
                                                                .child(
                                                                    "Patterns (one regex per line)",
                                                                ),
                                                        )
                                                        .child(
                                                            div()
                                                                .id(SharedString::from(format!(
                                                                    "settings-keyword-rule-patterns-{}",
                                                                    rule.id
                                                                )))
                                                                .min_h(px(72.))
                                                                .px_2()
                                                                .py_2()
                                                                .rounded_md()
                                                                .border_1()
                                                                .border_color(if patterns_active {
                                                                    rgb(palette.accent)
                                                                } else {
                                                                    rgb(palette.border)
                                                                })
                                                                .bg(rgb(palette.input))
                                                                .font_family("JetBrains Mono")
                                                                .text_size(px(11.))
                                                                .text_color(rgb(palette.text))
                                                                .line_height(px(16.))
                                                                .cursor_pointer()
                                                                .child(patterns_value)
                                                                .on_click(cx.listener({
                                                                    let rule_id = rule_id.clone();
                                                                    move |this, _, window, cx| {
                                                                        this.focus_keyword_highlight_field(
                                                                            rule_id.clone(),
                                                                            KeywordHighlightEditorField::Patterns,
                                                                            window,
                                                                            cx,
                                                                        );
                                                                    }
                                                                })),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .flex()
                                                        .flex_col()
                                                        .gap_2()
                                                        .child(settings_form_row(
                                                            palette,
                                                            "Dark color",
                                                            None,
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap_2()
                                                                .child(
                                                                    div()
                                                                        .size(px(20.))
                                                                        .rounded_md()
                                                                        .bg(rgb(
                                                                            parse_keyword_swatch(
                                                                                &rule.color_dark,
                                                                            )
                                                                            .unwrap_or(0x79c0ff),
                                                                        ))
                                                                        .border_1()
                                                                        .border_color(rgb(
                                                                            palette.border,
                                                                        )),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .id(SharedString::from(
                                                                            format!(
                                                                                "settings-keyword-rule-dark-{}",
                                                                                rule.id
                                                                            ),
                                                                        ))
                                                                        .min_w(px(88.))
                                                                        .h(px(28.))
                                                                        .px_2()
                                                                        .rounded_md()
                                                                        .border_1()
                                                                        .border_color(
                                                                            if dark_active {
                                                                                rgb(palette.accent)
                                                                            } else {
                                                                                rgb(palette.border)
                                                                            },
                                                                        )
                                                                        .bg(rgb(palette.input))
                                                                        .font_family(
                                                                            "JetBrains Mono",
                                                                        )
                                                                        .text_size(px(11.))
                                                                        .text_color(rgb(
                                                                            palette.text,
                                                                        ))
                                                                        .flex()
                                                                        .items_center()
                                                                        .cursor_pointer()
                                                                        .child(dark_value)
                                                                        .on_click(cx.listener({
                                                                            let rule_id =
                                                                                rule_id.clone();
                                                                            move |this,
                                                                                  _,
                                                                                  window,
                                                                                  cx| {
                                                                                this.focus_keyword_highlight_field(
                                                                                    rule_id.clone(),
                                                                                    KeywordHighlightEditorField::ColorDark,
                                                                                    window,
                                                                                    cx,
                                                                                );
                                                                            }
                                                                        })),
                                                                ),
                                                        ))
                                                        .child(
                                                            div()
                                                                .flex()
                                                                .flex_wrap()
                                                                .gap_1()
                                                                .children(palette_dark.iter().map(
                                                                    |swatch| {
                                                                        let color = parse_keyword_swatch(swatch)
                                                                            .unwrap_or(0x79c0ff);
                                                                        let rid = rule_id.clone();
                                                                        let value = (*swatch).to_string();
                                                                        div()
                                                                            .id(SharedString::from(
                                                                                format!(
                                                                                    "settings-keyword-swatch-dark-{}-{swatch}",
                                                                                    rule.id
                                                                                ),
                                                                            ))
                                                                            .size(px(16.))
                                                                            .rounded_sm()
                                                                            .bg(rgb(color))
                                                                            .border_1()
                                                                            .border_color(rgb(
                                                                                palette.border,
                                                                            ))
                                                                            .cursor_pointer()
                                                                            .on_click(cx.listener(
                                                                                move |this, _, _, cx| {
                                                                                    this.set_keyword_highlight_rule_color(
                                                                                        rid.clone(),
                                                                                        true,
                                                                                        &value,
                                                                                        cx,
                                                                                    );
                                                                                },
                                                                            ))
                                                                    },
                                                                )),
                                                        )
                                                        .child(settings_form_row(
                                                            palette,
                                                            "Light color",
                                                            None,
                                                            div()
                                                                .flex()
                                                                .items_center()
                                                                .gap_2()
                                                                .child(
                                                                    div()
                                                                        .size(px(20.))
                                                                        .rounded_md()
                                                                        .bg(rgb(
                                                                            parse_keyword_swatch(
                                                                                &rule.color_light,
                                                                            )
                                                                            .unwrap_or(0x0969da),
                                                                        ))
                                                                        .border_1()
                                                                        .border_color(rgb(
                                                                            palette.border,
                                                                        )),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .id(SharedString::from(
                                                                            format!(
                                                                                "settings-keyword-rule-light-{}",
                                                                                rule.id
                                                                            ),
                                                                        ))
                                                                        .min_w(px(88.))
                                                                        .h(px(28.))
                                                                        .px_2()
                                                                        .rounded_md()
                                                                        .border_1()
                                                                        .border_color(
                                                                            if light_active {
                                                                                rgb(palette.accent)
                                                                            } else {
                                                                                rgb(palette.border)
                                                                            },
                                                                        )
                                                                        .bg(rgb(palette.input))
                                                                        .font_family(
                                                                            "JetBrains Mono",
                                                                        )
                                                                        .text_size(px(11.))
                                                                        .text_color(rgb(
                                                                            palette.text,
                                                                        ))
                                                                        .flex()
                                                                        .items_center()
                                                                        .cursor_pointer()
                                                                        .child(light_value)
                                                                        .on_click(cx.listener({
                                                                            let rule_id =
                                                                                rule_id.clone();
                                                                            move |this,
                                                                                  _,
                                                                                  window,
                                                                                  cx| {
                                                                                this.focus_keyword_highlight_field(
                                                                                    rule_id.clone(),
                                                                                    KeywordHighlightEditorField::ColorLight,
                                                                                    window,
                                                                                    cx,
                                                                                );
                                                                            }
                                                                        })),
                                                                ),
                                                        ))
                                                        .child(
                                                            div()
                                                                .flex()
                                                                .flex_wrap()
                                                                .gap_1()
                                                                .children(
                                                                    palette_light.iter().map(
                                                                        |swatch| {
                                                                            let color =
                                                                                parse_keyword_swatch(
                                                                                    swatch,
                                                                                )
                                                                                .unwrap_or(
                                                                                    0x0969da,
                                                                                );
                                                                            let rid =
                                                                                rule_id.clone();
                                                                            let value =
                                                                                (*swatch)
                                                                                    .to_string();
                                                                            div()
                                                                                .id(SharedString::from(
                                                                                    format!(
                                                                                        "settings-keyword-swatch-light-{}-{swatch}",
                                                                                        rule.id
                                                                                    ),
                                                                                ))
                                                                                .size(px(16.))
                                                                                .rounded_sm()
                                                                                .bg(rgb(color))
                                                                                .border_1()
                                                                                .border_color(rgb(
                                                                                    palette.border,
                                                                                ))
                                                                                .cursor_pointer()
                                                                                .on_click(
                                                                                    cx.listener(
                                                                                        move |this,
                                                                                              _,
                                                                                              _,
                                                                                              cx| {
                                                                                            this.set_keyword_highlight_rule_color(
                                                                                                rid.clone(),
                                                                                                false,
                                                                                                &value,
                                                                                                cx,
                                                                                            );
                                                                                        },
                                                                                    ),
                                                                                )
                                                                        },
                                                                    ),
                                                                ),
                                                        ),
                                                ),
                                        )
                                    })
                            })),
                    )
                }),
        )
    }
}

fn terminal_feature_card(
    palette: crate::ui::theme::ThemePalette,
    title: &'static str,
    detail: &'static str,
    enabled: bool,
) -> impl IntoElement {
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .text_xs()
                        .font_weight(FontWeight(800.))
                        .text_color(rgb(palette.text))
                        .child(title),
                )
                .child(status_pill(
                    if enabled { "on" } else { "off" },
                    if enabled {
                        rgb(palette.success)
                    } else {
                        rgb(palette.text_muted)
                    },
                    if enabled {
                        rgb(palette.hover)
                    } else {
                        rgb(palette.border)
                    },
                )),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
                .line_height(px(14.))
                .child(detail),
        )
}

fn search_engine_hint(
    palette: crate::ui::theme::ThemePalette,
    title: &'static str,
    detail: &'static str,
) -> impl IntoElement {
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(800.))
                .text_color(rgb(palette.text))
                .child(title),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
                .line_height(px(14.))
                .child(detail),
        )
}

fn settings_toggle_button(
    palette: crate::ui::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(32.))
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .rounded_sm()
        .border_1()
        .border_color(if enabled {
            rgb(palette.success)
        } else {
            rgb(palette.border)
        })
        .bg(if enabled {
            rgb(palette.hover)
        } else {
            rgb(palette.surface)
        })
        .text_color(if enabled {
            rgb(palette.success)
        } else {
            rgb(palette.text)
        })
        .text_xs()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x223047)))
        .child(label)
        .child(status_pill(
            if enabled { "on" } else { "off" },
            if enabled {
                rgb(palette.success)
            } else {
                rgb(palette.text_muted)
            },
            if enabled {
                rgb(0x0d241c)
            } else {
                rgb(palette.border)
            },
        ))
        .on_click(on_click)
}

fn search_engine_icon_label(icon: Option<&str>) -> String {
    match icon.unwrap_or("default") {
        "google" => "G".into(),
        "bing" => "B".into(),
        "duckduckgo" => "D".into(),
        "github" => "GH".into(),
        "gitlab" => "GL".into(),
        "baidu" => "Bd".into(),
        "yahoo" => "Y!".into(),
        "youtube" => "YT".into(),
        "bilibili" => "Bi".into(),
        "zhihu" => "Zh".into(),
        "openai" => "AI".into(),
        "claude" => "Cl".into(),
        "gemini" => "Ge".into(),
        _ => "?".into(),
    }
}

fn search_engine_icon_color(icon: Option<&str>) -> u32 {
    match icon.unwrap_or("default") {
        "google" => 0x4285f4,
        "bing" => 0x008373,
        "duckduckgo" => 0xde5833,
        "github" => 0x8b949e,
        "gitlab" => 0xfc6d26,
        "baidu" => 0x2932e1,
        "yahoo" => 0x410093,
        "youtube" => 0xff0000,
        "bilibili" => 0x00a1d6,
        "zhihu" => 0x0084ff,
        "openai" => 0x10a37f,
        "claude" => 0xd97757,
        "gemini" => 0x4285f4,
        _ => 0x8b949e,
    }
}

fn parse_keyword_swatch(value: &str) -> Option<u32> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}
