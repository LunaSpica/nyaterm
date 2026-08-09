use gpui::{Context, IntoElement, SharedString, div, prelude::*, rgb};
use nyaterm_ui::NyaSelectOption;

use crate::features::NyaTermApp;
use crate::models::HeaderStatusMode;
use crate::widgets::small_button;

use super::super::{settings_form_row, settings_form_section, settings_switch};

impl NyaTermApp {
    pub(in crate::features) fn general_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Tauri GeneralTab: language, nested startup layout, tray, confirm, diagnostics.
        let language = self.settings.summary().language.clone();
        let language_value = match language.as_str() {
            "zh-CN" | "zh" => "zh-CN",
            _ => "en",
        };
        let diagnostics_level = self.settings.summary().diagnostics_level.clone();
        let retention = self.settings.summary().diagnostics_retention_days;
        let days_unit = self.tr("common.days");
        let header_status_mode =
            HeaderStatusMode::from_setting(&self.settings.summary().ui_header_status_mode);
        let header_status_visible = self.settings.summary().ui_header_status_visible;
        let header_status_value = if header_status_visible {
            header_status_mode.persistence_id()
        } else {
            "hidden"
        };

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
                        self.settings_select_control(
                            "settings.general.language",
                            vec![
                                NyaSelectOption::new("en", "English"),
                                NyaSelectOption::new("zh-CN", "中文 (简体)"),
                            ],
                            language_value,
                            false,
                            cx,
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.headerStatus"),
                        Some(SharedString::from(self.tr("settings.headerStatusDesc"))),
                        self.settings_select_control(
                            "settings.general.header-status",
                            std::iter::once(NyaSelectOption::new(
                                "hidden",
                                self.tr("headerStatus.hidden"),
                            ))
                            .chain(HeaderStatusMode::ALL.into_iter().map(|mode| {
                                NyaSelectOption::new(
                                    mode.persistence_id(),
                                    self.tr(mode.i18n_key()),
                                )
                            }))
                            .collect(),
                            header_status_value,
                            false,
                            cx,
                        ),
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
                        self.settings_select_control(
                            "settings.general.diagnostics-level",
                            vec![
                                NyaSelectOption::new("warn", self.tr("settings.logLevelWarn")),
                                NyaSelectOption::new("info", self.tr("settings.logLevelInfo")),
                                NyaSelectOption::new("debug", self.tr("settings.logLevelDebug")),
                            ],
                            diagnostics_level,
                            false,
                            cx,
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.logRetention"),
                        Some(SharedString::from(self.tr("settings.logRetentionDesc"))),
                        self.settings_select_control(
                            "settings.general.diagnostics-retention",
                            [3_u32, 7, 14, 30]
                                .into_iter()
                                .map(|days| {
                                    NyaSelectOption::new(
                                        days.to_string(),
                                        format!("{days} {days_unit}"),
                                    )
                                })
                                .collect(),
                            retention.to_string(),
                            false,
                            cx,
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
