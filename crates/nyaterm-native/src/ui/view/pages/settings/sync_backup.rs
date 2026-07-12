use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn config_backup_settings_section(
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

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(palette, 
                Some("Config backup"),
                Some("Export or import the native redb configuration store."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
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
                    .child(settings_form_row(palette, 
                        "JSON backup",
                        Some(SharedString::from(
                            "Portable JSON export/import of connections and settings.",
                        )),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(palette, 
                                "settings-config-export",
                                "Export",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_config_export(cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "settings-config-import",
                                "Import",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_config_import(cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Portable .nya",
                        Some(SharedString::from(
                            "Legacy portable snapshot package used by NyaTerm migration.",
                        )),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(palette, 
                                "settings-portable-export",
                                "Export .nya",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_portable_snapshot_export(cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "settings-portable-import",
                                "Import .nya",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_portable_snapshot_import(cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Encrypted .nya",
                        Some(SharedString::from(
                            "AES-GCM package sealed with the master password.",
                        )),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(palette, 
                                "settings-encrypted-portable-export",
                                "Encrypt .nya",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_encrypted_portable_snapshot_export(cx);
                                }),
                            ))
                            .child(small_button(palette, 
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

    pub(in crate::ui::view) fn diagnostics_settings_section(
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
            .child(settings_form_section(palette, 
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
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .children([3_u32, 7, 14, 30].into_iter().map(|days| {
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
                            })),
                    ))
                    .child(settings_form_row(palette, 
                        "Support bundle",
                        Some(SharedString::from(prompt_label)),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(palette, 
                                "settings-diagnostics-export",
                                "Export",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_diagnostics_export(cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "settings-diagnostics-logs",
                                "Logs",
                                cx.listener(|this, _, _, cx| {
                                    this.reveal_log_dir(cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Log directory",
                        Some(SharedString::from(truncate_preview(&log_dir, 64))),
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .child("On disk"),
                    )),
            ))
            .child(settings_form_section(palette, 
                Some("Updates"),
                Some("Check for native application updates."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Native update",
                        Some(SharedString::from(truncate_preview(&self.update_status, 96))),
                        small_button(palette, 
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
                        this.child(settings_form_row(palette, 
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

    pub(in crate::ui::view) fn cloud_sync_input(
        &mut self,
        id: &'static str,
        label: &'static str,
        value: String,
        field: CloudSyncInputField,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        transfer_input(
            id,
            label,
            if value.is_empty() {
                " ".to_string()
            } else {
                value
            },
            self.cloud_sync_focused_field == field,
            self.theme_palette(),
        )
        .track_focus(&self.cloud_sync_focus)
        .on_click(cx.listener(move |this, _, window, cx| {
            this.cloud_sync_focused_field = field;
            window.focus(&this.cloud_sync_focus);
            cx.notify();
        }))
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
            cx.stop_propagation();
            this.handle_cloud_sync_key_down(event, cx);
        }))
    }

    pub(in crate::ui::view) fn cloud_sync_conflict_banner(
        &mut self,
        conflict: CloudSyncConflictState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let provider_action = conflict.provider_action;
        let local_hash = self
            .cloud_sync_state
            .last_synced_payload_hash
            .as_deref()
            .map(compact_id)
            .unwrap_or_else(|| "unsynced".to_string());
        let remote_revision = self
            .cloud_sync_state
            .last_applied_remote_revision
            .as_deref()
            .map(compact_id)
            .unwrap_or_else(|| "unknown".to_string());

        div()
            .mt_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x8a5f1c))
            .bg(rgb(0x1f1a10))
            .p_3()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(0xfacc15))
                                    .child("Cloud Sync Conflict"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0xe2e8f0))
                                    .child(conflict.message),
                            )
                            .child(div().text_xs().text_color(rgb(palette.text_muted)).child(format!(
                                "{} · local {} · remote {}",
                                conflict.provider, local_hash, remote_revision
                            ))),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(palette, 
                                "cloud-conflict-force-push",
                                "Force Push",
                                cx.listener(move |this, _, _, cx| {
                                    this.prompt_cloud_sync_force_push(provider_action, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "cloud-conflict-force-pull",
                                "Force Pull",
                                cx.listener(move |this, _, _, cx| {
                                    this.prompt_cloud_sync_force_pull(provider_action, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "cloud-conflict-dismiss",
                                "Dismiss",
                                cx.listener(|this, _, _, cx| {
                                    this.dismiss_cloud_sync_conflict(cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::ui::view) fn cloud_sync_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let cloud_snapshot_prompt = self
            .active_snapshot_password_prompt
            .clone()
            .filter(|prompt| {
                matches!(
                    prompt.kind,
                    SnapshotPasswordPromptKind::CloudPush
                        | SnapshotPasswordPromptKind::CloudPull
                        | SnapshotPasswordPromptKind::CloudForcePush
                        | SnapshotPasswordPromptKind::CloudForcePull
                        | SnapshotPasswordPromptKind::CloudProviderPush
                        | SnapshotPasswordPromptKind::CloudProviderPull
                        | SnapshotPasswordPromptKind::CloudProviderForcePush
                        | SnapshotPasswordPromptKind::CloudProviderForcePull
                )
            });
        let cloud_remote_path = self
            .runtime
            .config_dir()
            .join("cloud-sync-local")
            .join(&self.cloud_sync_settings.remote_root)
            .display()
            .to_string();
        let cloud_provider_label = format!("local / {}", self.cloud_sync_settings.provider);
        let cloud_last_revision = self
            .cloud_sync_state
            .last_applied_remote_revision
            .as_deref()
            .map(compact_id)
            .unwrap_or_else(|| "none".to_string());
        let cloud_last_hash = self
            .cloud_sync_state
            .last_synced_payload_hash
            .as_deref()
            .map(compact_id)
            .unwrap_or_else(|| "none".to_string());
        let cloud_history_empty = self.cloud_sync_history.is_empty();
        let cloud_history_0 = self.cloud_sync_history.first().cloned();
        let cloud_history_1 = self.cloud_sync_history.get(1).cloned();
        let cloud_history_2 = self.cloud_sync_history.get(2).cloned();
        let cloud_history_0_expanded = cloud_history_0
            .as_ref()
            .map(|entry| self.cloud_sync_history_expanded.contains(&entry.id))
            .unwrap_or(false);
        let cloud_history_1_expanded = cloud_history_1
            .as_ref()
            .map(|entry| self.cloud_sync_history_expanded.contains(&entry.id))
            .unwrap_or(false);
        let cloud_history_2_expanded = cloud_history_2
            .as_ref()
            .map(|entry| self.cloud_sync_history_expanded.contains(&entry.id))
            .unwrap_or(false);
        let cloud_conflict = self.cloud_sync_conflict.clone();
        let active_cloud_provider = configured_cloud_sync_provider(&self.cloud_sync_settings);
        let webdav_password_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.webdav_password,
            &self.cloud_sync_settings.webdav.password,
        );
        let s3_access_key_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.s3_access_key_id,
            &self.cloud_sync_settings.s3.access_key_id,
        );
        let s3_secret_key_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.s3_secret_access_key,
            &self.cloud_sync_settings.s3.secret_access_key,
        );
        let s3_session_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.s3_session_token,
            &self.cloud_sync_settings.s3.session_token,
        );
        let google_drive_access_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.google_drive_access_token,
            &self.cloud_sync_settings.google_drive.access_token,
        );
        let google_drive_refresh_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.google_drive_refresh_token,
            &self.cloud_sync_settings.google_drive.refresh_token,
        );
        let google_drive_client_secret_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.google_drive_client_secret,
            &self.cloud_sync_settings.google_drive.client_secret,
        );
        let onedrive_access_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.onedrive_access_token,
            &self.cloud_sync_settings.onedrive.access_token,
        );
        let onedrive_refresh_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.onedrive_refresh_token,
            &self.cloud_sync_settings.onedrive.refresh_token,
        );
        let onedrive_client_secret_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.onedrive_client_secret,
            &self.cloud_sync_settings.onedrive.client_secret,
        );
        let aliyun_drive_access_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.aliyun_drive_access_token,
            &self.cloud_sync_settings.aliyun_drive.access_token,
        );
        let aliyun_drive_refresh_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.aliyun_drive_refresh_token,
            &self.cloud_sync_settings.aliyun_drive.refresh_token,
        );
        let aliyun_drive_client_secret_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.aliyun_drive_client_secret,
            &self.cloud_sync_settings.aliyun_drive.client_secret,
        );
        let gitee_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.gitee_token,
            &self.cloud_sync_settings.gitee_snippet.access_token,
        );
        let github_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.github_token,
            &self.cloud_sync_settings.github_gist.access_token,
        );

        // Tauri SyncBackupTab density: section/switch rows + provider chips.
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(palette, 
                Some("Cloud sync"),
                Some("Mirror encrypted configuration snapshots to a remote provider."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Enable cloud sync",
                        Some(SharedString::from(self.cloud_sync_status.clone())),
                        settings_switch(palette, 
                            "cloud-sync-enabled",
                            self.cloud_sync_settings.enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_cloud_sync_enabled(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Snapshot state",
                        Some(SharedString::from(format!(
                            "provider {cloud_provider_label} · rev {cloud_last_revision} · hash {cloud_last_hash}"
                        ))),
                        small_button(palette, 
                            "cloud-sync-save",
                            "Save",
                            cx.listener(|this, _, _, cx| {
                                this.save_cloud_sync_settings(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Provider",
                        Some(SharedString::from(
                            "Local mirror, WebDAV/S3, cloud drives, or snippet backends.",
                        )),
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(palette, 
                                "cloud-provider-local",
                                "Local",
                                active_cloud_provider == "local_directory",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("local_directory", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "cloud-provider-webdav",
                                "WebDAV",
                                active_cloud_provider == "webdav",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("webdav", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "cloud-provider-s3",
                                "S3",
                                active_cloud_provider == "s3",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("s3", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "cloud-provider-google-drive",
                                "Drive",
                                active_cloud_provider == "google_drive",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("google_drive", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "cloud-provider-onedrive",
                                "OneDrive",
                                active_cloud_provider == "onedrive",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("onedrive", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "cloud-provider-aliyun-drive",
                                "Aliyun",
                                active_cloud_provider == "aliyun_drive",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("aliyun_drive", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "cloud-provider-gitee",
                                "Gitee",
                                active_cloud_provider == "gitee_snippet",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("gitee_snippet", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "cloud-provider-github",
                                "GitHub",
                                active_cloud_provider == "github_gist",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("github_gist", cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Remote root",
                        Some(SharedString::from(truncate_preview(&cloud_remote_path, 48))),
                        self.cloud_sync_input(
                            "cloud-sync-remote-root",
                            "Remote Root",
                            self.cloud_sync_settings.remote_root.clone(),
                            CloudSyncInputField::RemoteRoot,
                            cx,
                        ),
                    )),
            ))
            .when(active_cloud_provider == "webdav", |this| {
                this.child(settings_form_section(palette, 
                    Some("WebDAV"),
                    Some("Endpoint and credentials for the selected WebDAV target."),
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .grid()
                                .grid_cols(2)
                                .gap_2()
                                .child(self.cloud_sync_input(
                                    "cloud-webdav-endpoint",
                                    "Endpoint",
                                    self.cloud_sync_settings.webdav.endpoint.clone(),
                                    CloudSyncInputField::WebdavEndpoint,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-webdav-root",
                                    "Root",
                                    self.cloud_sync_settings.webdav.root.clone(),
                                    CloudSyncInputField::WebdavRoot,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-webdav-username",
                                    "Username",
                                    self.cloud_sync_settings.webdav.username.clone(),
                                    CloudSyncInputField::WebdavUsername,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-webdav-password",
                                    "Password",
                                    webdav_password_value,
                                    CloudSyncInputField::WebdavPassword,
                                    cx,
                                )),
                        ),
                ))
            })
            .when(active_cloud_provider == "s3", |this| {
                this.child(settings_form_section(palette, 
                    Some("S3 Compatible"),
                    Some("Bucket, region, and access keys for S3-compatible storage."),
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .grid()
                                .grid_cols(3)
                                .gap_2()
                                .child(self.cloud_sync_input(
                                    "cloud-s3-endpoint",
                                    "Endpoint",
                                    self.cloud_sync_settings.s3.endpoint.clone(),
                                    CloudSyncInputField::S3Endpoint,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-s3-bucket",
                                    "Bucket",
                                    self.cloud_sync_settings.s3.bucket.clone(),
                                    CloudSyncInputField::S3Bucket,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-s3-region",
                                    "Region",
                                    self.cloud_sync_settings.s3.region.clone(),
                                    CloudSyncInputField::S3Region,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-s3-root",
                                    "S3 Root",
                                    self.cloud_sync_settings.s3.root.clone(),
                                    CloudSyncInputField::S3Root,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-s3-access-key",
                                    "Access Key",
                                    s3_access_key_value,
                                    CloudSyncInputField::S3AccessKeyId,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-s3-secret-key",
                                    "Secret Key",
                                    s3_secret_key_value,
                                    CloudSyncInputField::S3SecretAccessKey,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-s3-session-token",
                                    "Session Token",
                                    s3_session_token_value,
                                    CloudSyncInputField::S3SessionToken,
                                    cx,
                                )),
                        )
                        .child(settings_form_row(palette, 
                            "Virtual host style",
                            Some(SharedString::from(
                                "Use virtual-hosted-style URLs instead of path style.",
                            )),
                            settings_switch(palette, 
                                "cloud-s3-url-style",
                                self.cloud_sync_settings.s3.virtual_host_style,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_s3_virtual_host_style(cx);
                                }),
                            ),
                        )),
                ))
            })
            .when(active_cloud_provider == "google_drive", |this| {
                this.child(settings_form_section(palette, 
                    Some("Google Drive"),
                    Some("OAuth client credentials and tokens for Drive sync."),
                    div()
                        .grid()
                        .grid_cols(2)
                        .gap_2()
                        .child(self.cloud_sync_input(
                            "cloud-google-drive-root",
                            "Drive Root",
                            self.cloud_sync_settings.google_drive.root.clone(),
                            CloudSyncInputField::GoogleDriveRoot,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-google-drive-client-id",
                            "Client ID",
                            self.cloud_sync_settings
                                .google_drive
                                .client_id
                                .clone()
                                .unwrap_or_default(),
                            CloudSyncInputField::GoogleDriveClientId,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-google-drive-client-secret",
                            "Client Secret",
                            google_drive_client_secret_value,
                            CloudSyncInputField::GoogleDriveClientSecret,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-google-drive-access-token",
                            "Access Token",
                            google_drive_access_token_value,
                            CloudSyncInputField::GoogleDriveAccessToken,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-google-drive-refresh-token",
                            "Refresh Token",
                            google_drive_refresh_token_value,
                            CloudSyncInputField::GoogleDriveRefreshToken,
                            cx,
                        )),
                ))
            })
            .when(active_cloud_provider == "onedrive", |this| {
                this.child(settings_form_section(palette, 
                    Some("OneDrive"),
                    Some("Microsoft Graph credentials for OneDrive sync."),
                    div()
                        .grid()
                        .grid_cols(2)
                        .gap_2()
                        .child(self.cloud_sync_input(
                            "cloud-onedrive-root",
                            "OneDrive Root",
                            self.cloud_sync_settings.onedrive.root.clone(),
                            CloudSyncInputField::OneDriveRoot,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-onedrive-client-id",
                            "Client ID",
                            self.cloud_sync_settings
                                .onedrive
                                .client_id
                                .clone()
                                .unwrap_or_default(),
                            CloudSyncInputField::OneDriveClientId,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-onedrive-client-secret",
                            "Client Secret",
                            onedrive_client_secret_value,
                            CloudSyncInputField::OneDriveClientSecret,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-onedrive-access-token",
                            "Access Token",
                            onedrive_access_token_value,
                            CloudSyncInputField::OneDriveAccessToken,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-onedrive-refresh-token",
                            "Refresh Token",
                            onedrive_refresh_token_value,
                            CloudSyncInputField::OneDriveRefreshToken,
                            cx,
                        )),
                ))
            })
            .when(active_cloud_provider == "aliyun_drive", |this| {
                this.child(settings_form_section(palette, 
                    Some("Aliyun Drive"),
                    Some("AliyunDrive OAuth credentials and tokens."),
                    div()
                        .grid()
                        .grid_cols(2)
                        .gap_2()
                        .child(self.cloud_sync_input(
                            "cloud-aliyun-drive-root",
                            "Drive Root",
                            self.cloud_sync_settings.aliyun_drive.root.clone(),
                            CloudSyncInputField::AliyunDriveRoot,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-aliyun-drive-client-id",
                            "Client ID",
                            self.cloud_sync_settings
                                .aliyun_drive
                                .client_id
                                .clone()
                                .unwrap_or_default(),
                            CloudSyncInputField::AliyunDriveClientId,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-aliyun-drive-client-secret",
                            "Client Secret",
                            aliyun_drive_client_secret_value,
                            CloudSyncInputField::AliyunDriveClientSecret,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-aliyun-drive-access-token",
                            "Access Token",
                            aliyun_drive_access_token_value,
                            CloudSyncInputField::AliyunDriveAccessToken,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-aliyun-drive-refresh-token",
                            "Refresh Token",
                            aliyun_drive_refresh_token_value,
                            CloudSyncInputField::AliyunDriveRefreshToken,
                            cx,
                        )),
                ))
            })
            .when(active_cloud_provider == "gitee_snippet", |this| {
                this.child(settings_form_section(palette, 
                    Some("Gitee Snippet"),
                    Some("API endpoint, snippet id, and personal access token."),
                    div()
                        .grid()
                        .grid_cols(3)
                        .gap_2()
                        .child(self.cloud_sync_input(
                            "cloud-gitee-endpoint",
                            "API Endpoint",
                            self.cloud_sync_settings.gitee_snippet.api_endpoint.clone(),
                            CloudSyncInputField::GiteeEndpoint,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-gitee-gist",
                            "Snippet ID",
                            self.cloud_sync_settings.gitee_snippet.gist_id.clone(),
                            CloudSyncInputField::GiteeGistId,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-gitee-token",
                            "Token",
                            gitee_token_value,
                            CloudSyncInputField::GiteeToken,
                            cx,
                        )),
                ))
            })
            .when(active_cloud_provider == "github_gist", |this| {
                this.child(settings_form_section(palette, 
                    Some("GitHub Gist"),
                    Some("Gist id and token for encrypted snapshot sync."),
                    div()
                        .grid()
                        .grid_cols(2)
                        .gap_2()
                        .child(self.cloud_sync_input(
                            "cloud-github-gist",
                            "Gist ID",
                            self.cloud_sync_settings.github_gist.gist_id.clone(),
                            CloudSyncInputField::GithubGistId,
                            cx,
                        ))
                        .child(self.cloud_sync_input(
                            "cloud-github-token",
                            "Token",
                            github_token_value,
                            CloudSyncInputField::GithubToken,
                            cx,
                        )),
                ))
            })
            .child(settings_form_section(palette, 
                Some("Sync actions"),
                Some("Push or pull encrypted snapshots locally or via the selected provider."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Local mirror",
                        Some(SharedString::from(format!(
                            "device {} · revision {} · hash {}",
                            compact_id(&self.cloud_sync_state.device_id),
                            cloud_last_revision,
                            cloud_last_hash
                        ))),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(palette, 
                                "settings-cloud-sync-push",
                                "Push Local",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_local_cloud_sync_push(cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "settings-cloud-sync-pull",
                                "Pull Local",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_local_cloud_sync_pull(cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Provider",
                        Some(SharedString::from(format!(
                            "{} · {}",
                            cloud_provider_label,
                            truncate_preview(&cloud_remote_path, 40)
                        ))),
                        div()
                            .flex()
                            .gap_1()
                            .child(small_button(palette, 
                                "settings-provider-cloud-sync-push",
                                "Push Provider",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_provider_cloud_sync_push(cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "settings-provider-cloud-sync-pull",
                                "Pull Provider",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_provider_cloud_sync_pull(cx);
                                }),
                            )),
                    )),
            ))
            .when_some(cloud_conflict, |this, conflict| {
                this.child(self.cloud_sync_conflict_banner(conflict, cx))
            })
            .when_some(cloud_snapshot_prompt, |this, prompt| {
                this.child(self.snapshot_password_prompt_banner(prompt, cx))
            })
            .child(settings_form_section(palette, 
                Some("Recent history"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .when(cloud_history_empty, |this| {
                        this.child(
                            div()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.surface_elevated))
                                .bg(rgb(palette.bg))
                                .px_3()
                                .py_2()
                                .text_size(px(11.))
                                .text_color(rgb(palette.text_muted))
                                .child("No sync runs recorded"),
                        )
                    })
                    .when_some(cloud_history_0, |this, entry| {
                        let entry_id = entry.id.clone();
                        this.child(cloud_sync_history_row(
                            palette,
                            entry.clone(),
                            cloud_history_0_expanded,
                            cx.listener(move |this, _, _, cx| {
                                this.toggle_cloud_sync_history_details(&entry_id, cx);
                            }),
                            cx.listener({
                                let message = entry.message.clone();
                                move |this, _, _, cx| {
                                    if message.trim().is_empty() {
                                        this.terminal_status =
                                            "history entry has no message".to_string();
                                    } else {
                                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                            message.clone(),
                                        ));
                                        this.terminal_status =
                                            "sync history message copied".to_string();
                                    }
                                    cx.notify();
                                }
                            }),
                        ))
                    })
                    .when_some(cloud_history_1, |this, entry| {
                        let entry_id = entry.id.clone();
                        this.child(cloud_sync_history_row(
                            palette,
                            entry.clone(),
                            cloud_history_1_expanded,
                            cx.listener(move |this, _, _, cx| {
                                this.toggle_cloud_sync_history_details(&entry_id, cx);
                            }),
                            cx.listener({
                                let message = entry.message.clone();
                                move |this, _, _, cx| {
                                    if message.trim().is_empty() {
                                        this.terminal_status =
                                            "history entry has no message".to_string();
                                    } else {
                                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                            message.clone(),
                                        ));
                                        this.terminal_status =
                                            "sync history message copied".to_string();
                                    }
                                    cx.notify();
                                }
                            }),
                        ))
                    })
                    .when_some(cloud_history_2, |this, entry| {
                        let entry_id = entry.id.clone();
                        this.child(cloud_sync_history_row(
                            palette,
                            entry.clone(),
                            cloud_history_2_expanded,
                            cx.listener(move |this, _, _, cx| {
                                this.toggle_cloud_sync_history_details(&entry_id, cx);
                            }),
                            cx.listener({
                                let message = entry.message.clone();
                                move |this, _, _, cx| {
                                    if message.trim().is_empty() {
                                        this.terminal_status =
                                            "history entry has no message".to_string();
                                    } else {
                                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                            message.clone(),
                                        ));
                                        this.terminal_status =
                                            "sync history message copied".to_string();
                                    }
                                    cx.notify();
                                }
                            }),
                        ))
                    }),
            ))
    }
}


fn sync_provider_hint(palette: crate::ui::theme::ThemePalette, title: &'static str, detail: &'static str) -> impl IntoElement {    div()
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
