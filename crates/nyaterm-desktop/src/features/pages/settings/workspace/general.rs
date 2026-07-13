use super::*;

impl NyaTermApp {
    pub(in crate::features) fn general_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Tauri GeneralTab: language, nested startup layout, tray, confirm, diagnostics.
        let language = self.settings.language.clone();
        let language_label = match language.as_str() {
            "zh-CN" | "zh" => "简体中文",
            "zh-TW" => "繁體中文",
            "en" | "en-US" => "English",
            "ja" => "日本語",
            other => other,
        };
        let diagnostics_level = self.settings.diagnostics_level.clone();
        let retention = self.settings.diagnostics_retention_days;
        let log_dir = self.runtime.log_dir().display().to_string();
        let diagnostics_prompt = match self.diagnostics_path_prompt {
            Some(DiagnosticsPathPromptKind::Export) => "selecting export path",
            None => "ready",
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                palette,
                None,
                None,
                settings_form_row(
                    palette,
                    "Language",
                    Some(SharedString::from("UI language preference for labels and dialogs.")),
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(settings_choice_chip(
                            palette,
                            "general-lang-en",
                            "English",
                            matches!(language.as_str(), "en" | "en-US"),
                            cx.listener(|this, _, _, cx| {
                                this.update_ui_language("en", cx);
                            }),
                        ))
                        .child(settings_choice_chip(
                            palette,
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
            .child(settings_form_section(
                palette,
                Some("Startup & window"),
                Some("Restore sessions, tray minimize, and quit confirmation."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "Restore sessions on startup",
                        Some(SharedString::from(
                            "Reopen the previous workspace tabs when NyaTerm starts.",
                        )),
                        settings_switch(
                            palette,
                            "general-startup-restore",
                            self.settings.startup_restore,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_startup_restore(cx);
                            }),
                        ),
                    ))
                    .when(self.settings.startup_restore, |this| {
                        this.child(
                            div()
                                .pl_3()
                                .ml_1()
                                .border_l_1()
                                .border_color(rgb(palette.border))
                                .child(settings_form_row(
                                    palette,
                                    "Restore window layout",
                                    Some(SharedString::from(
                                        "Restore multi-leaf tab windows and global pane splits with the workspace.",
                                    )),
                                    settings_switch(
                                        palette,
                                        "general-startup-restore-window-layout",
                                        self.settings.startup_restore_window_layout,
                                        cx.listener(|this, _, _, cx| {
                                            this.toggle_startup_restore_window_layout(cx);
                                        }),
                                    ),
                                )),
                        )
                    })
                    .child(settings_form_row(
                        palette,
                        "Minimize to tray",
                        Some(SharedString::from(
                            "Hide the main window to the system tray instead of the taskbar when minimized (platform-dependent).",
                        )),
                        settings_switch(
                            palette,
                            "general-minimize-to-tray",
                            self.settings.minimize_to_tray,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_minimize_to_tray(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Confirm on close",
                        Some(SharedString::from(
                            "Ask before quitting when sessions are still open.",
                        )),
                        settings_switch(
                            palette,
                            "general-confirm-close",
                            self.settings.confirm_on_close,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_confirm_on_close(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("Diagnostics"),
                Some("Log level, retention, support bundle export, and log directory."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "Log level",
                        Some(SharedString::from(
                            "Controls native diagnostics verbosity. Restart may be required for file tracing filter.",
                        )),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(settings_choice_chip(
                                palette,
                                "general-diag-warn",
                                "Warn",
                                diagnostics_level == "warn",
                                cx.listener(|this, _, _, cx| {
                                    this.set_diagnostics_level("warn", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "general-diag-info",
                                "Info",
                                diagnostics_level == "info",
                                cx.listener(|this, _, _, cx| {
                                    this.set_diagnostics_level("info", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "general-diag-debug",
                                "Debug",
                                diagnostics_level == "debug",
                                cx.listener(|this, _, _, cx| {
                                    this.set_diagnostics_level("debug", cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Log retention",
                        Some(SharedString::from(
                            "How long retained diagnostics JSONL logs are kept on disk.",
                        )),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .children(
                                [3_u32, 7, 14, 30].into_iter().map(|days| {
                                    let selected = retention == days;
                                    let id = format!("general-diag-retention-{days}");
                                    let label: &'static str = match days {
                                        3 => "3d",
                                        7 => "7d",
                                        14 => "14d",
                                        _ => "30d",
                                    };
                                    settings_choice_chip(
                                        palette,
                                        id,
                                        label,
                                        selected,
                                        cx.listener(move |this, _, _, cx| {
                                            this.set_diagnostics_retention_days(days, cx);
                                        }),
                                    )
                                }),
                            ),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Open logs",
                        Some(SharedString::from(format!(
                            "Reveal the log directory ({})",
                            truncate_preview(&log_dir, 48)
                        ))),
                        small_button(
                            palette,
                            "general-open-logs",
                            "Open logs",
                            cx.listener(|this, _, _, cx| {
                                this.reveal_log_dir(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Export diagnostics",
                        Some(SharedString::from(format!(
                            "Zip support bundle with logs and runtime snapshot ({diagnostics_prompt})."
                        ))),
                        small_button(
                            palette,
                            "general-export-diagnostics",
                            "Export",
                            cx.listener(|this, _, _, cx| {
                                this.prompt_diagnostics_export(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("Status"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(settings_form_row(
                        palette,
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
                    .child(settings_form_row(
                        palette,
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
                    ))
                    .child(settings_form_row(
                        palette,
                        "Diagnostics",
                        Some(SharedString::from("Active log level and retention.")),
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .child(format!(
                                "{} · {} days",
                                self.settings.diagnostics_level,
                                self.settings.diagnostics_retention_days
                            )),
                    )),
            ))
    }
}
