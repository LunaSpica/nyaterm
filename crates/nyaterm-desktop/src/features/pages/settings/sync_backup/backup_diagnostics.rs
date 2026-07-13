use super::*;

impl NyaTermApp {
    pub(in crate::features) fn config_backup_settings_section(
        &mut self,
        backup_snapshot_prompt: Option<SnapshotPasswordPromptState>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let prompt_label = match self.config_path_prompt {
            Some(ConfigPathPromptKind::Export) => "selecting export path",
            Some(ConfigPathPromptKind::Import) => "selecting import path",
            Some(ConfigPathPromptKind::PortableExport) => "selecting .nya export path",
            Some(ConfigPathPromptKind::PortableImport) => "selecting .nya import path",
            Some(ConfigPathPromptKind::EncryptedPortableExport) => {
                "selecting encrypted .nya export path"
            }
            Some(ConfigPathPromptKind::EncryptedPortableImport) => {
                "selecting encrypted .nya import path"
            }
            None => "native redb backup",
        };

        div().flex().flex_col().gap_3().child(settings_form_section(
            palette,
            Some("Config backup"),
            Some("Export or import the native redb configuration store."),
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(settings_form_row(
                    palette,
                    "Store path",
                    Some(SharedString::from(truncate_preview(
                        &self.store_status.path,
                        64,
                    ))),
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette.text_muted))
                        .child(prompt_label),
                ))
                .child(settings_form_row(
                    palette,
                    "JSON backup",
                    Some(SharedString::from(
                        "Portable JSON export/import of connections and settings.",
                    )),
                    div()
                        .flex()
                        .gap_1()
                        .child(small_button(
                            palette,
                            "settings-config-export",
                            "Export",
                            cx.listener(|this, _, _, cx| {
                                this.prompt_config_export(cx);
                            }),
                        ))
                        .child(small_button(
                            palette,
                            "settings-config-import",
                            "Import",
                            cx.listener(|this, _, _, cx| {
                                this.prompt_config_import(cx);
                            }),
                        )),
                ))
                .child(settings_form_row(
                    palette,
                    "Portable .nya",
                    Some(SharedString::from(
                        "Legacy portable snapshot package used by NyaTerm migration.",
                    )),
                    div()
                        .flex()
                        .gap_1()
                        .child(small_button(
                            palette,
                            "settings-portable-export",
                            "Export .nya",
                            cx.listener(|this, _, _, cx| {
                                this.prompt_portable_snapshot_export(cx);
                            }),
                        ))
                        .child(small_button(
                            palette,
                            "settings-portable-import",
                            "Import .nya",
                            cx.listener(|this, _, _, cx| {
                                this.prompt_portable_snapshot_import(cx);
                            }),
                        )),
                ))
                .child(settings_form_row(
                    palette,
                    "Encrypted .nya",
                    Some(SharedString::from(
                        "AES-GCM package sealed with the master password.",
                    )),
                    div()
                        .flex()
                        .gap_1()
                        .child(small_button(
                            palette,
                            "settings-encrypted-portable-export",
                            "Encrypt .nya",
                            cx.listener(|this, _, _, cx| {
                                this.prompt_encrypted_portable_snapshot_export(cx);
                            }),
                        ))
                        .child(small_button(
                            palette,
                            "settings-encrypted-portable-import",
                            "Decrypt .nya",
                            cx.listener(|this, _, _, cx| {
                                this.prompt_encrypted_portable_snapshot_import(cx);
                            }),
                        )),
                ))
                .when_some(backup_snapshot_prompt, |this, prompt| {
                    this.child(self.snapshot_password_prompt_banner(prompt, cx))
                }),
        ))
    }

    pub(in crate::features) fn diagnostics_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let log_dir = self.runtime.log_dir().display().to_string();
        let prompt_label = match self.diagnostics_path_prompt {
            Some(DiagnosticsPathPromptKind::Export) => "selecting export path",
            None => "native diagnostics",
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                palette,
                Some("Diagnostics"),
                Some("Export support bundles and open the native log directory."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "Log level",
                        Some(SharedString::from(
                            "Same as General · Diagnostics; persists under diagnostics.level.",
                        )),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(settings_choice_chip(
                                palette,
                                "sync-diag-warn",
                                "Warn",
                                self.settings.diagnostics_level == "warn",
                                cx.listener(|this, _, _, cx| {
                                    this.set_diagnostics_level("warn", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "sync-diag-info",
                                "Info",
                                self.settings.diagnostics_level == "info",
                                cx.listener(|this, _, _, cx| {
                                    this.set_diagnostics_level("info", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "sync-diag-debug",
                                "Debug",
                                self.settings.diagnostics_level == "debug",
                                cx.listener(|this, _, _, cx| {
                                    this.set_diagnostics_level("debug", cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Log retention",
                        Some(SharedString::from("Retained diagnostics JSONL days.")),
                        div().flex().items_center().gap_1().children(
                            [3_u32, 7, 14, 30].into_iter().map(|days| {
                                let selected = self.settings.diagnostics_retention_days == days;
                                let id = format!("sync-diag-retention-{days}");
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
                        "Support bundle",
                        Some(SharedString::from(prompt_label)),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(
                                palette,
                                "settings-diagnostics-export",
                                "Export",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_diagnostics_export(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "settings-diagnostics-logs",
                                "Logs",
                                cx.listener(|this, _, _, cx| {
                                    this.reveal_log_dir(cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Log directory",
                        Some(SharedString::from(truncate_preview(&log_dir, 64))),
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .child("On disk"),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("Updates"),
                Some("Check for native application updates."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "Native update",
                        Some(SharedString::from(truncate_preview(
                            &self.update_status,
                            96,
                        ))),
                        small_button(
                            palette,
                            "settings-update-check",
                            if self.update_pending {
                                "Checking"
                            } else {
                                "Check"
                            },
                            cx.listener(|this, _, _, cx| {
                                this.start_update_check(cx);
                            }),
                        ),
                    ))
                    .when_some(self.update_info.clone(), |this, info| {
                        let release_url = info.html_url.clone().unwrap_or_else(|| {
                            "https://github.com/nyakang/nyaterm/releases".to_string()
                        });
                        let notes = info.release_notes.unwrap_or_default();
                        this.child(settings_form_row(
                            palette,
                            "Latest release",
                            Some(SharedString::from(format!(
                                "{}{} · {}",
                                info.latest_version,
                                info.release_date
                                    .as_deref()
                                    .map(|date| format!(" · {date}"))
                                    .unwrap_or_default(),
                                truncate_preview(&release_url, 48)
                            ))),
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette.text_muted))
                                .child(if notes.trim().is_empty() {
                                    "no notes".to_string()
                                } else {
                                    truncate_preview(&notes, 48)
                                }),
                        ))
                    }),
            ))
    }
}
