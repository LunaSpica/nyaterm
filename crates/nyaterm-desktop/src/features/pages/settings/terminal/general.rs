use super::*;

impl NyaTermApp {
    pub(in crate::features) fn terminal_general_settings_section(
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
                Some("Terminal display toggles for the GPUI surface."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
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
}
