use gpui::{Context, IntoElement, SharedString, div, prelude::*, px, rgb};

use crate::features::NyaTermApp;
use crate::models::HeaderStatusMode;
use crate::widgets::small_button;

use super::super::{
    settings_choice_chip, settings_form_row, settings_form_section, settings_switch,
};

impl NyaTermApp {
    pub(in crate::features) fn general_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Tauri GeneralTab: language, nested startup layout, tray, confirm, diagnostics.
        let language = self.settings.summary().language.clone();
        let language_label = match language.as_str() {
            "zh-CN" | "zh" => "简体中文",
            "zh-TW" => "繁體中文",
            "en" | "en-US" => "English",
            "ja" => "日本語",
            other => other,
        };
        let diagnostics_level = self.settings.summary().diagnostics_level.clone();
        let retention = self.settings.summary().diagnostics_retention_days;
        let days_unit = self.tr("common.days");
        let header_status_mode =
            HeaderStatusMode::from_setting(&self.settings.summary().ui_header_status_mode);
        let header_status_visible = self.settings.summary().ui_header_status_visible;

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
                        self.tr("settings.language"),
                        Some(SharedString::from(self.tr("settings.languageDesc"))),
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
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.headerStatus"),
                        Some(SharedString::from(self.tr("settings.headerStatusDesc"))),
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_1()
                            .child(settings_choice_chip(
                                palette,
                                "general-header-status-hidden",
                                self.tr("headerStatus.hidden"),
                                !header_status_visible,
                                cx.listener(|this, _, _, cx| {
                                    this.set_header_status_visible(false, cx);
                                }),
                            ))
                            .children(HeaderStatusMode::ALL.into_iter().map(|mode| {
                                settings_choice_chip(
                                    palette,
                                    format!("general-header-status-{}", mode.persistence_id()),
                                    self.tr(mode.i18n_key()),
                                    header_status_visible && header_status_mode == mode,
                                    cx.listener(move |this, _, _, cx| {
                                        this.set_header_status_mode(mode, cx);
                                    }),
                                )
                            })),
                    )),
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
                        self.tr("settings.startupRestore"),
                        Some(SharedString::from(self.tr("settings.startupRestoreDesc"))),
                        settings_switch(
                            palette,
                            "general-startup-restore",
                            self.settings.summary().startup_restore,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_startup_restore(cx);
                            }),
                        ),
                    ))
                    .when(self.settings.summary().startup_restore, |this| {
                        this.child(
                            div()
                                .pl_3()
                                .ml_1()
                                .border_l_1()
                                .border_color(rgb(palette.border))
                                .child(settings_form_row(
                                    palette,
                                    self.tr("settings.startupRestoreWindowLayout"),
                                    Some(SharedString::from(
                                        self.tr("settings.startupRestoreWindowLayoutDesc"),
                                    )),
                                    settings_switch(
                                        palette,
                                        "general-startup-restore-window-layout",
                                        self.settings.summary().startup_restore_window_layout,
                                        cx.listener(|this, _, _, cx| {
                                            this.toggle_startup_restore_window_layout(cx);
                                        }),
                                    ),
                                )),
                        )
                    })
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.minimizeToTray"),
                        Some(SharedString::from(self.tr("settings.minimizeToTrayDesc"))),
                        settings_switch(
                            palette,
                            "general-minimize-to-tray",
                            self.settings.summary().minimize_to_tray,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_minimize_to_tray(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.confirmOnClose"),
                        Some(SharedString::from(self.tr("settings.confirmOnCloseDesc"))),
                        settings_switch(
                            palette,
                            "general-confirm-close",
                            self.settings.summary().confirm_on_close,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_confirm_on_close(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some(self.tr("settings.diagnostics")),
                Some(self.tr("settings.diagnosticsDesc")),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.logLevel"),
                        Some(SharedString::from(self.tr("settings.logLevelDesc"))),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(settings_choice_chip(
                                palette,
                                "general-diag-warn",
                                self.tr("settings.logLevelWarn"),
                                diagnostics_level == "warn",
                                cx.listener(|this, _, _, cx| {
                                    this.set_diagnostics_level("warn", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "general-diag-info",
                                self.tr("settings.logLevelInfo"),
                                diagnostics_level == "info",
                                cx.listener(|this, _, _, cx| {
                                    this.set_diagnostics_level("info", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "general-diag-debug",
                                self.tr("settings.logLevelDebug"),
                                diagnostics_level == "debug",
                                cx.listener(|this, _, _, cx| {
                                    this.set_diagnostics_level("debug", cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.logRetention"),
                        Some(SharedString::from(self.tr("settings.logRetentionDesc"))),
                        div().flex().items_center().gap_1().children(
                            [3_u32, 7, 14, 30].into_iter().map(|days| {
                                let selected = retention == days;
                                let id = format!("general-diag-retention-{days}");
                                let label = format!("{days} {days_unit}");
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
                        self.tr("settings.openLogs"),
                        Some(SharedString::from(self.tr("settings.openLogsDesc"))),
                        small_button(
                            palette,
                            "general-open-logs",
                            self.tr("settings.openLogs"),
                            cx.listener(|this, _, _, cx| {
                                this.reveal_log_dir(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.exportDiagnostics"),
                        Some(SharedString::from(
                            self.tr("settings.exportDiagnosticsDesc"),
                        )),
                        small_button(
                            palette,
                            "general-export-diagnostics",
                            self.tr("settings.exportDiagnostics"),
                            cx.listener(|this, _, _, cx| {
                                this.prompt_diagnostics_export(cx);
                            }),
                        ),
                    )),
            ))
    }
}
