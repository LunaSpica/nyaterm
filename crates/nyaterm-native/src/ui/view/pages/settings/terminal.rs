use super::*;
use gpui::{App, ClickEvent, SharedString, Window};

impl NyaTermApp {
    pub(super) fn terminal_general_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let multi_line_paste = self.settings.terminal_show_multi_line_paste_dialog;
        let remote_stats = self.settings.ui_show_remote_stats;
        let process_manager = self.settings.ui_show_process_manager;
        let docker_manager = self.settings.ui_show_docker_manager;

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
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Terminal General"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(5)
                            .gap_3()
                            .child(metric(
                                "Font",
                                format!(
                                    "{} {}",
                                    self.settings.terminal_font_family,
                                    self.settings.terminal_font_size
                                ),
                            ))
                            .child(metric(
                                "Scrollback",
                                format!("{} lines", self.settings.terminal_scrollback_lines),
                            ))
                            .child(metric(
                                "Keep Alive",
                                format!("{} sec", self.settings.terminal_keep_alive_interval),
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
                                "Paste Guard",
                                if multi_line_paste {
                                    "confirm".to_string()
                                } else {
                                    "direct".to_string()
                                },
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(4)
                            .gap_2()
                            .child(terminal_feature_card(
                                "Multi-line Paste",
                                "dialog + direct + line-by-line",
                                multi_line_paste,
                            ))
                            .child(terminal_feature_card(
                                "Line Numbers",
                                "prefix rendered terminal rows",
                                self.settings.terminal_show_line_numbers,
                            ))
                            .child(terminal_feature_card(
                                "Timestamps",
                                "setting persisted; per-line metadata pending",
                                self.settings.terminal_show_timestamps,
                            ))
                            .child(terminal_feature_card(
                                "Remote Stats",
                                "dedicated GPUI page",
                                remote_stats,
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(3)
                            .gap_2()
                            .child(terminal_feature_card(
                                "Process Manager",
                                "SSH-backed process inspector",
                                process_manager,
                            ))
                            .child(terminal_feature_card(
                                "Docker Manager",
                                "SSH-backed container inspector",
                                docker_manager,
                            ))
                            .child(terminal_feature_card(
                                "Hardware Acceleration",
                                "stored native rendering preference",
                                self.settings.terminal_hardware_acceleration,
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
                                "terminal-scrollback-minus",
                                "-100 lines",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_terminal_scrollback_lines(-100, cx);
                                }),
                            ))
                            .child(small_button(
                                "terminal-scrollback-plus",
                                "+100 lines",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_terminal_scrollback_lines(100, cx);
                                }),
                            ))
                            .child(small_button(
                                "terminal-keepalive-minus",
                                "-5 sec",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_terminal_keep_alive_interval(-5, cx);
                                }),
                            ))
                            .child(small_button(
                                "terminal-keepalive-plus",
                                "+5 sec",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_terminal_keep_alive_interval(5, cx);
                                }),
                            ))
                            .child(small_button(
                                "terminal-font-size-minus",
                                "-1 px",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_terminal_font_size(-1, cx);
                                }),
                            ))
                            .child(small_button(
                                "terminal-font-size-plus",
                                "+1 px",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_terminal_font_size(1, cx);
                                }),
                            ))
                            .child(small_button(
                                "terminal-open-search",
                                "Open Search",
                                cx.listener(|this, _, window, cx| {
                                    this.open_terminal_search(window, cx);
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
                            .child("Terminal Display and Panels"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(3)
                            .gap_2()
                            .child(settings_toggle_button(
                                "terminal-toggle-padding",
                                "Workspace Padding",
                                self.settings.terminal_show_workspace_padding,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_terminal_workspace_padding(cx);
                                }),
                            ))
                            .child(settings_toggle_button(
                                "terminal-toggle-line-numbers",
                                "Line Numbers",
                                self.settings.terminal_show_line_numbers,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_terminal_line_numbers(cx);
                                }),
                            ))
                            .child(settings_toggle_button(
                                "terminal-toggle-timestamps",
                                "Timestamps",
                                self.settings.terminal_show_timestamps,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_terminal_timestamps(cx);
                                }),
                            ))
                            .child(settings_toggle_button(
                                "terminal-toggle-ms",
                                "Milliseconds",
                                self.settings.terminal_show_timestamp_milliseconds,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_terminal_timestamp_milliseconds(cx);
                                }),
                            ))
                            .child(settings_toggle_button(
                                "terminal-toggle-paste-dialog",
                                "Paste Dialog",
                                self.settings.terminal_show_multi_line_paste_dialog,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_multi_line_paste_dialog(cx);
                                }),
                            ))
                            .child(settings_toggle_button(
                                "terminal-toggle-image-path",
                                "Image As Path",
                                self.settings.terminal_paste_image_as_path,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_paste_image_as_path(cx);
                                }),
                            ))
                            .child(settings_toggle_button(
                                "terminal-toggle-hardware",
                                "Hardware Accel",
                                self.settings.terminal_hardware_acceleration,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_terminal_hardware_acceleration(cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(3)
                            .gap_2()
                            .child(settings_toggle_button(
                                "terminal-toggle-remote-stats",
                                "Remote Stats",
                                self.settings.ui_show_remote_stats,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_remote_stats_panel(cx);
                                }),
                            ))
                            .child(settings_toggle_button(
                                "terminal-toggle-process-manager",
                                "Process Manager",
                                self.settings.ui_show_process_manager,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_process_manager_panel(cx);
                                }),
                            ))
                            .child(settings_toggle_button(
                                "terminal-toggle-docker-manager",
                                "Docker Manager",
                                self.settings.ui_show_docker_manager,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_docker_manager_panel(cx);
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
                            .when(self.settings.ui_show_remote_stats, |this| {
                                this.child(metric(
                                    "Stats Interval",
                                    format!("{} sec", self.settings.ui_remote_stats_interval),
                                ))
                                .child(small_button(
                                    "terminal-stats-interval-minus",
                                    "-1",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_remote_stats_interval(-1, cx);
                                    }),
                                ))
                                .child(small_button(
                                    "terminal-stats-interval-plus",
                                    "+1",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_remote_stats_interval(1, cx);
                                    }),
                                ))
                            })
                            .when(self.settings.ui_show_process_manager, |this| {
                                this.child(metric(
                                    "Process Interval",
                                    format!("{} sec", self.settings.ui_process_manager_interval),
                                ))
                                .child(small_button(
                                    "terminal-process-interval-minus",
                                    "-1",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_process_manager_interval(-1, cx);
                                    }),
                                ))
                                .child(small_button(
                                    "terminal-process-interval-plus",
                                    "+1",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_process_manager_interval(1, cx);
                                    }),
                                ))
                            })
                            .when(self.settings.ui_show_docker_manager, |this| {
                                this.child(metric(
                                    "Docker Interval",
                                    format!("{} sec", self.settings.ui_docker_manager_interval),
                                ))
                                .child(small_button(
                                    "terminal-docker-interval-minus",
                                    "-1",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_docker_manager_interval(-1, cx);
                                    }),
                                ))
                                .child(small_button(
                                    "terminal-docker-interval-plus",
                                    "+1",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_docker_manager_interval(1, cx);
                                    }),
                                ))
                            }),
                    ),
            )
            .child(self.recording_settings_section(cx))
    }

    pub(super) fn terminal_search_settings_section(
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
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Terminal Search"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(5)
                            .gap_3()
                            .child(metric("Mode", self.terminal_search_mode.label().to_string()))
                            .child(metric(
                                "Case",
                                if self.terminal_search_case_sensitive {
                                    "sensitive".to_string()
                                } else {
                                    "ignore".to_string()
                                },
                            ))
                            .child(metric(
                                "Regex",
                                if self.terminal_search_regex {
                                    "enabled".to_string()
                                } else {
                                    "disabled".to_string()
                                },
                            ))
                            .child(metric(
                                "Whole Word",
                                if self.terminal_search_whole_word {
                                    "enabled".to_string()
                                } else {
                                    "disabled".to_string()
                                },
                            ))
                            .child(metric(
                                "Query",
                                if self.terminal_search_query.trim().is_empty() {
                                    "empty".to_string()
                                } else {
                                    truncate_preview(&self.terminal_search_query, 24)
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
                            .child(policy_button(
                                "settings-search-mode-buffer",
                                "Buffer",
                                self.terminal_search_mode == TerminalSearchMode::Buffer,
                                cx.listener(|this, _, _, cx| {
                                    this.terminal_search_mode = TerminalSearchMode::Buffer;
                                    this.terminal_search_active_index = 0;
                                    cx.notify();
                                }),
                            ))
                            .child(policy_button(
                                "settings-search-mode-history",
                                "History",
                                self.terminal_search_mode == TerminalSearchMode::History,
                                cx.listener(|this, _, _, cx| {
                                    this.terminal_search_mode = TerminalSearchMode::History;
                                    this.terminal_search_active_index = 0;
                                    cx.notify();
                                }),
                            ))
                            .child(small_button(
                                "settings-search-case",
                                if self.terminal_search_case_sensitive {
                                    "Case On"
                                } else {
                                    "Case Off"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.terminal_search_case_sensitive =
                                        !this.terminal_search_case_sensitive;
                                    this.terminal_search_active_index = 0;
                                    cx.notify();
                                }),
                            ))
                            .child(small_button(
                                "settings-search-regex",
                                if self.terminal_search_regex {
                                    "Regex On"
                                } else {
                                    "Regex Off"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.terminal_search_regex = !this.terminal_search_regex;
                                    this.terminal_search_active_index = 0;
                                    cx.notify();
                                }),
                            ))
                            .child(small_button(
                                "settings-search-word",
                                if self.terminal_search_whole_word {
                                    "Word On"
                                } else {
                                    "Word Off"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.terminal_search_whole_word =
                                        !this.terminal_search_whole_word;
                                    this.terminal_search_active_index = 0;
                                    cx.notify();
                                }),
                            ))
                            .child(small_button(
                                "settings-search-open",
                                "Open Search",
                                cx.listener(|this, _, window, cx| {
                                    this.open_terminal_search(window, cx);
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
                            .child("Command Search"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(3)
                            .gap_3()
                            .child(metric(
                                "History",
                                self.command_history.len().to_string(),
                            ))
                            .child(metric(
                                "Quick Commands",
                                self.quick_commands.len().to_string(),
                            ))
                            .child(metric(
                                "Draft",
                                if self.command_search_draft.trim().is_empty() {
                                    "empty".to_string()
                                } else {
                                    truncate_preview(&self.command_search_draft, 24)
                                },
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(3)
                            .gap_2()
                            .child(search_engine_hint(
                                "Command History",
                                "Fuzzy search over persisted command history.",
                            ))
                            .child(search_engine_hint(
                                "Quick Commands",
                                "Fuzzy search over saved quick commands.",
                            ))
                            .child(search_engine_hint(
                                "Terminal History",
                                "Buffer and recording history search share the same native matcher flags.",
                            )),
                    ),
            )
            .child(self.keyword_highlights_settings_section(cx))
    }

    fn keyword_highlights_settings_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child("Keyword Highlights"),
            )
            .child(
                div()
                    .mt_3()
                    .grid()
                    .grid_cols(3)
                    .gap_3()
                    .child(metric(
                        "State",
                        if self.keyword_highlights.enabled {
                            "enabled".to_string()
                        } else {
                            "disabled".to_string()
                        },
                    ))
                    .child(metric(
                        "Rules",
                        self.keyword_highlights.rules.len().to_string(),
                    ))
                    .child(metric(
                        "Active",
                        self.keyword_highlights
                            .rules
                            .iter()
                            .filter(|rule| rule.enabled)
                            .count()
                            .to_string(),
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(small_button(
                        "settings-keyword-highlights-enabled",
                        if self.keyword_highlights.enabled {
                            "Enabled"
                        } else {
                            "Disabled"
                        },
                        cx.listener(|this, _, _, cx| {
                            this.toggle_keyword_highlights(cx);
                        }),
                    ))
                    .child(small_button(
                        "settings-keyword-highlights-wrap",
                        if self.keyword_highlights.across_wrapped_lines {
                            "Wrap On"
                        } else {
                            "Wrap Off"
                        },
                        cx.listener(|this, _, _, cx| {
                            this.toggle_keyword_highlights_wrapped(cx);
                        }),
                    ))
                    .child(small_button(
                        "settings-keyword-highlights-import",
                        "Import",
                        cx.listener(|this, _, _, cx| {
                            this.prompt_keyword_highlight_import(cx);
                        }),
                    ))
                    .child(div().text_xs().text_color(rgb(0x98a3b8)).child(
                        match self.keyword_highlight_path_prompt {
                            Some(KeywordHighlightPathPromptKind::Import) => "selecting import file",
                            None => "legacy JSON import",
                        },
                    )),
            )
    }
}

fn terminal_feature_card(
    title: &'static str,
    detail: &'static str,
    enabled: bool,
) -> impl IntoElement {
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x263142))
        .bg(rgb(0x0d1320))
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
                        .text_color(rgb(0xe5edf7))
                        .child(title),
                )
                .child(status_pill(
                    if enabled { "on" } else { "off" },
                    if enabled {
                        rgb(0x6ee7b7)
                    } else {
                        rgb(0x98a3b8)
                    },
                    if enabled {
                        rgb(0x12342a)
                    } else {
                        rgb(0x202633)
                    },
                )),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(10.))
                .text_color(rgb(0x8f98aa))
                .line_height(px(14.))
                .child(detail),
        )
}

fn search_engine_hint(title: &'static str, detail: &'static str) -> impl IntoElement {
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x263142))
        .bg(rgb(0x0d1320))
        .p_3()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(800.))
                .text_color(rgb(0xe5edf7))
                .child(title),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(10.))
                .text_color(rgb(0x8f98aa))
                .line_height(px(14.))
                .child(detail),
        )
}

fn settings_toggle_button(
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
            rgb(0x2f8f5b)
        } else {
            rgb(0x303848)
        })
        .bg(if enabled {
            rgb(0x12342a)
        } else {
            rgb(0x151b27)
        })
        .text_color(if enabled {
            rgb(0xbbf7d0)
        } else {
            rgb(0xdbeafe)
        })
        .text_xs()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x223047)))
        .child(label)
        .child(status_pill(
            if enabled { "on" } else { "off" },
            if enabled {
                rgb(0x6ee7b7)
            } else {
                rgb(0x98a3b8)
            },
            if enabled {
                rgb(0x0d241c)
            } else {
                rgb(0x202633)
            },
        ))
        .on_click(on_click)
}
