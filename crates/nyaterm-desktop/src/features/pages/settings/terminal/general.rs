use gpui::{
    App, ClickEvent, Context, FontWeight, IntoElement, SharedString, Window, div, prelude::*, px,
    rgb,
};

use crate::features::{NyaTermApp, TextInputSetup};
use crate::theme::ThemePalette;
use crate::widgets::small_button;

use super::super::{
    settings_form_row, settings_form_section, settings_switch, settings_switch_with_enabled,
};

impl NyaTermApp {
    pub(in crate::features) fn terminal_general_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let x11_display_input = self
            .text_input_box(
                "settings.terminal.x11-display",
                &self.settings.x11_display.clone(),
                TextInputSetup::placeholder(self.tr("settings.x11DisplayPlaceholder")),
                cx,
            )
            .into_any_element();
        let action_links_enabled = self.settings.terminal_action_links_enabled;

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                palette,
                None,
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.scrollbackLines"),
                        Some(SharedString::from(self.tr("settings.scrollbackLinesDesc"))),
                        terminal_number_stepper(
                            palette,
                            "terminal-scrollback-minus",
                            "terminal-scrollback-plus",
                            self.settings.terminal_scrollback_lines.to_string(),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_terminal_scrollback_lines(-100, cx);
                            }),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_terminal_scrollback_lines(100, cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.keepAliveInterval"),
                        Some(SharedString::from(
                            self.tr("settings.keepAliveIntervalDesc"),
                        )),
                        terminal_number_stepper(
                            palette,
                            "terminal-keepalive-minus",
                            "terminal-keepalive-plus",
                            self.settings.terminal_keep_alive_interval.to_string(),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_terminal_keep_alive_interval(-5, cx);
                            }),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_terminal_keep_alive_interval(5, cx);
                            }),
                        ),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(terminal_settings_field_meta(
                                palette,
                                self.tr("settings.x11Display"),
                                self.tr("settings.x11DisplayDesc"),
                            ))
                            .child(div().w_full().max_w(px(520.)).child(x11_display_input)),
                    )
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.hardwareAcceleration"),
                        Some(SharedString::from(
                            self.tr("settings.hardwareAccelerationDesc"),
                        )),
                        settings_switch(
                            palette,
                            "terminal-hardware-acceleration",
                            self.settings.terminal_hardware_acceleration,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_terminal_hardware_acceleration(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.lowLatencyMode"),
                        Some(SharedString::from(self.tr("settings.lowLatencyModeDesc"))),
                        settings_switch(
                            palette,
                            "terminal-low-latency-mode",
                            self.settings.terminal_low_latency_mode,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_terminal_low_latency_mode(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.showWorkspacePadding"),
                        Some(SharedString::from(
                            self.tr("settings.showWorkspacePaddingDesc"),
                        )),
                        settings_switch(
                            palette,
                            "terminal-workspace-padding",
                            self.settings.terminal_show_workspace_padding,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_terminal_workspace_padding(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.showLineNumbers"),
                        Some(SharedString::from(self.tr("settings.showLineNumbersDesc"))),
                        settings_switch(
                            palette,
                            "terminal-line-numbers",
                            self.settings.terminal_show_line_numbers,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_terminal_line_numbers(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.showTimestamps"),
                        Some(SharedString::from(self.tr("settings.showTimestampsDesc"))),
                        settings_switch(
                            palette,
                            "terminal-timestamps",
                            self.settings.terminal_show_timestamps,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_terminal_timestamps(cx);
                            }),
                        ),
                    ))
                    .when(self.settings.terminal_show_timestamps, |this| {
                        this.child(settings_form_row(
                            palette,
                            self.tr("settings.showTimestampMilliseconds"),
                            Some(SharedString::from(
                                self.tr("settings.showTimestampMillisecondsDesc"),
                            )),
                            settings_switch(
                                palette,
                                "terminal-timestamp-ms",
                                self.settings.terminal_show_timestamp_milliseconds,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_terminal_timestamp_milliseconds(cx);
                                }),
                            ),
                        ))
                    })
                    .child(settings_form_row(
                        palette,
                        self.tr("terminal.showMultiLinePasteDialog"),
                        Some(SharedString::from(
                            self.tr("terminal.showMultiLinePasteDialogDesc"),
                        )),
                        settings_switch(
                            palette,
                            "terminal-multi-line-paste",
                            self.settings.terminal_show_multi_line_paste_dialog,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_multi_line_paste_dialog(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("terminal.pasteImageAsPath"),
                        Some(SharedString::from(self.tr("terminal.pasteImageAsPathDesc"))),
                        settings_switch(
                            palette,
                            "terminal-paste-image-path",
                            self.settings.terminal_paste_image_as_path,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_paste_image_as_path(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.showRemoteStats"),
                        Some(SharedString::from(self.tr("settings.showRemoteStatsDesc"))),
                        settings_switch(
                            palette,
                            "terminal-remote-stats",
                            self.settings.ui_show_remote_stats,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_remote_stats_panel(cx);
                            }),
                        ),
                    ))
                    .when(self.settings.ui_show_remote_stats, |this| {
                        this.child(settings_form_row(
                            palette,
                            self.tr("settings.remoteStatsInterval"),
                            Some(SharedString::from(
                                self.tr("settings.remoteStatsIntervalDesc"),
                            )),
                            terminal_number_stepper(
                                palette,
                                "terminal-remote-stats-interval-minus",
                                "terminal-remote-stats-interval-plus",
                                self.settings.ui_remote_stats_interval.to_string(),
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_remote_stats_interval(-1, cx);
                                }),
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_remote_stats_interval(1, cx);
                                }),
                            ),
                        ))
                    })
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.showProcessManager"),
                        Some(SharedString::from(
                            self.tr("settings.showProcessManagerDesc"),
                        )),
                        settings_switch(
                            palette,
                            "terminal-process-manager",
                            self.settings.ui_show_process_manager,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_process_manager_panel(cx);
                            }),
                        ),
                    ))
                    .when(self.settings.ui_show_process_manager, |this| {
                        this.child(settings_form_row(
                            palette,
                            self.tr("settings.processManagerInterval"),
                            Some(SharedString::from(
                                self.tr("settings.processManagerIntervalDesc"),
                            )),
                            terminal_number_stepper(
                                palette,
                                "terminal-process-interval-minus",
                                "terminal-process-interval-plus",
                                self.settings.ui_process_manager_interval.to_string(),
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_process_manager_interval(-1, cx);
                                }),
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_process_manager_interval(1, cx);
                                }),
                            ),
                        ))
                    })
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.showDockerManager"),
                        Some(SharedString::from(
                            self.tr("settings.showDockerManagerDesc"),
                        )),
                        settings_switch(
                            palette,
                            "terminal-docker-manager",
                            self.settings.ui_show_docker_manager,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_docker_manager_panel(cx);
                            }),
                        ),
                    ))
                    .when(self.settings.ui_show_docker_manager, |this| {
                        this.child(settings_form_row(
                            palette,
                            self.tr("settings.dockerManagerInterval"),
                            Some(SharedString::from(
                                self.tr("settings.dockerManagerIntervalDesc"),
                            )),
                            terminal_number_stepper(
                                palette,
                                "terminal-docker-interval-minus",
                                "terminal-docker-interval-plus",
                                self.settings.ui_docker_manager_interval.to_string(),
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_docker_manager_interval(-1, cx);
                                }),
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_docker_manager_interval(1, cx);
                                }),
                            ),
                        ))
                    }),
            ))
            .child(settings_form_section(
                palette,
                None,
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.actionLinks"),
                        Some(SharedString::from(self.tr("settings.actionLinksDesc"))),
                        settings_switch(
                            palette,
                            "terminal-action-links",
                            action_links_enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_terminal_action_links(cx);
                            }),
                        ),
                    ))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight(500.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("settings.actionLinksMatchers")),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(terminal_action_matcher_row(
                                palette,
                                "terminal-action-links-ipv4",
                                self.tr("settings.actionLinksMatcherIpv4"),
                                "192.168.1.1",
                                self.tr("settings.actionLinksMatcherIpv4Desc"),
                                self.settings.terminal_action_links_matchers.ipv4,
                                action_links_enabled,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_terminal_action_links_matcher("ipv4", cx);
                                }),
                            ))
                            .child(terminal_action_matcher_row(
                                palette,
                                "terminal-action-links-host-port",
                                self.tr("settings.actionLinksMatcherHostPort"),
                                "localhost:8080",
                                self.tr("settings.actionLinksMatcherHostPortDesc"),
                                self.settings.terminal_action_links_matchers.host_port,
                                action_links_enabled,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_terminal_action_links_matcher("host_port", cx);
                                }),
                            ))
                            .child(terminal_action_matcher_row(
                                palette,
                                "terminal-action-links-archive",
                                self.tr("settings.actionLinksMatcherArchive"),
                                "backup.tar.gz",
                                self.tr("settings.actionLinksMatcherArchiveDesc"),
                                self.settings.terminal_action_links_matchers.archive,
                                action_links_enabled,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_terminal_action_links_matcher("archive", cx);
                                }),
                            )),
                    ),
            ))
            .child(self.keyword_highlights_settings_section(cx))
    }
}

fn terminal_number_stepper(
    palette: ThemePalette,
    minus_id: &'static str,
    plus_id: &'static str,
    value: String,
    on_minus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_plus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(small_button(palette, minus_id, "-", on_minus))
        .child(
            div()
                .min_w(px(56.))
                .text_center()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(11.))
                .text_color(rgb(palette.text))
                .child(value),
        )
        .child(small_button(palette, plus_id, "+", on_plus))
}

fn terminal_settings_field_meta(
    palette: ThemePalette,
    label: &'static str,
    desc: &'static str,
) -> impl IntoElement {
    div()
        .min_w_0()
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight(500.))
                .text_color(rgb(palette.text))
                .child(label),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(11.))
                .text_color(rgb(palette.text_dimmed))
                .child(desc),
        )
}

fn terminal_action_matcher_row(
    palette: ThemePalette,
    id: &'static str,
    label: &'static str,
    example: &'static str,
    desc: &'static str,
    checked: bool,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .px_3()
        .py_2()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            div()
                .min_w_0()
                .flex_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight(500.))
                                .text_color(rgb(palette.text))
                                .child(label),
                        )
                        .child(
                            div()
                                .px_2()
                                .py(px(1.))
                                .rounded_sm()
                                .bg(rgb(palette.surface_elevated))
                                .font_family(crate::features::gpui_code_font_family())
                                .text_size(px(10.))
                                .text_color(rgb(palette.text_muted))
                                .child(example),
                        ),
                )
                .child(
                    div()
                        .mt_1()
                        .text_size(px(11.))
                        .text_color(rgb(palette.text_dimmed))
                        .child(desc),
                ),
        )
        .child(settings_switch_with_enabled(
            palette, id, checked, enabled, on_click,
        ))
}
