use super::*;

use crate::features::{dialog_action_button, format_cloud_provider};

#[path = "cloud_sync/providers.rs"]
mod providers;
impl NyaTermApp {
    fn cloud_sync_provider_select(
        &mut self,
        active_provider: &str,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.theme_palette();
        let menu_open = self.cloud_sync_provider_menu_open && enabled;
        let providers = [
            ("webdav", "WebDAV"),
            ("s3", "S3 Compatible"),
            ("gitee_snippet", "Gitee Snippet"),
            ("github_gist", "GitHub Gist"),
            ("google_drive", "Google Drive"),
            ("onedrive", "OneDrive"),
            ("aliyun_drive", "AliyunDrive"),
        ];
        let active_label = cloud_sync_provider_label(active_provider);
        let mut menu = div()
            .mt_1()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface_elevated))
            .py_1()
            .flex()
            .flex_col();

        for (provider, label) in providers {
            let selected = provider == active_provider;
            menu = menu.child(
                div()
                    .id(SharedString::from(format!("cloud-provider-{provider}")))
                    .h(px(30.))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .bg(if selected {
                        rgb(palette.hover)
                    } else {
                        rgb(palette.surface_elevated)
                    })
                    .hover(move |this| this.bg(rgb(palette.hover)))
                    .text_size(px(11.))
                    .font_weight(if selected {
                        FontWeight(600.)
                    } else {
                        FontWeight(500.)
                    })
                    .text_color(if selected {
                        rgb(palette.primary)
                    } else {
                        rgb(palette.text)
                    })
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.update_cloud_sync_provider(provider, cx);
                    })),
            );
        }

        div()
            .w_full()
            .max_w(px(220.))
            .flex()
            .flex_col()
            .opacity(if enabled { 1.0 } else { 0.45 })
            .child(
                div()
                    .id("cloud-provider-select")
                    .h(px(32.))
                    .px_3()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text))
                    .child(active_label)
                    .child(svg().size(px(13.)).path("icons/chevron-down.svg"))
                    .when(enabled, |this| {
                        this.cursor_pointer()
                            .hover(move |this| this.bg(rgb(palette.hover)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cloud_sync_provider_menu_open =
                                    !this.cloud_sync_provider_menu_open;
                                cx.notify();
                            }))
                    }),
            )
            .when(menu_open, |this| this.child(menu))
            .into_any_element()
    }

    pub(in crate::features) fn cloud_sync_input(
        &mut self,
        id: &'static str,
        label: &'static str,
        value: String,
        field: CloudSyncInputField,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let enabled = self.cloud_sync_form_enabled();
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
        .opacity(if enabled { 1.0 } else { 0.45 })
        .track_focus(&self.cloud_sync_focus)
        .when(enabled, |this| {
            this.on_click(cx.listener(move |this, _, window, cx| {
                this.cloud_sync_focused_field = field;
                window.focus(&this.cloud_sync_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                this.handle_cloud_sync_key_down(event, cx);
            }))
        })
    }

    pub(in crate::features) fn cloud_sync_conflict_banner(
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
            .rounded_md()
            .border_1()
            .border_color(rgba((palette.warning << 8) | 0x4d))
            .bg(rgba((palette.warning << 8) | 0x1a))
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("settings.syncConflictTitle")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .line_height(px(18.))
                            .text_color(rgb(palette.text_muted))
                            .child(conflict.message),
                    ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .child(cloud_sync_conflict_stat(
                        palette,
                        self.tr("settings.localSnapshot"),
                        local_hash,
                    ))
                    .child(cloud_sync_conflict_stat(
                        palette,
                        self.tr("settings.remoteSnapshot"),
                        remote_revision,
                    )),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_muted))
                    .child(format_cloud_provider(&conflict.provider)),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(small_button(
                        palette,
                        "cloud-conflict-force-pull",
                        self.tr("settings.downloadRemoteVersion"),
                        cx.listener(move |this, _, _, cx| {
                            this.prompt_cloud_sync_force_pull(provider_action, cx);
                        }),
                    ))
                    .child(dialog_action_button(
                        palette,
                        "cloud-conflict-force-push",
                        self.tr("settings.uploadLocalVersion"),
                        false,
                        cx.listener(move |this, _, _, cx| {
                            this.prompt_cloud_sync_force_push(provider_action, cx);
                        }),
                    )),
            )
    }

    pub(in crate::features) fn cloud_sync_settings_section(
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
        let cloud_conflict = self.cloud_sync_conflict.clone();
        let active_cloud_provider = configured_cloud_sync_provider(&self.cloud_sync_settings);
        let form_enabled = self.cloud_sync_form_enabled();
        let auto_sync_enabled = form_enabled && self.cloud_sync_settings.enabled;
        let debounce_enabled = auto_sync_enabled && self.cloud_sync_settings.auto_push_on_change;
        let validation_key = cloud_sync_validation_key(&self.pending_cloud_sync_settings());
        let validation_message = (form_enabled && self.cloud_sync_settings.enabled)
            .then(|| validation_key.map(|key| self.tr(key)))
            .flatten();
        let settings_dirty = self.settings_draft_dirty();
        let action_block_message = if !form_enabled {
            Some(self.tr("settings.masterPasswordRequiredDesc"))
        } else if settings_dirty {
            Some(self.tr("settings.applySettingsFirst"))
        } else {
            validation_key.map(|key| self.tr(key))
        };
        let actions_busy = cloud_snapshot_prompt.is_some() || self.cloud_sync_job_running;
        let can_run_actions = action_block_message.is_none() && !actions_busy;
        let can_run_enabled_actions = can_run_actions && self.cloud_sync_settings.enabled;
        let sync_state_key = cloud_sync_state_i18n_key(
            self.cloud_sync_settings.enabled,
            cloud_conflict.is_some(),
            &self.cloud_sync_status,
        );
        let sync_running = sync_state_key == "settings.syncState.running";
        let current_operation = if sync_running {
            self.cloud_sync_status.clone()
        } else {
            self.tr("settings.none").to_string()
        };
        let provider_label = cloud_sync_provider_label(&active_cloud_provider).to_string();
        let last_checked = self
            .cloud_sync_state
            .last_checked_at_ms
            .map(format_history_timestamp_ms)
            .unwrap_or_else(|| self.tr("settings.never").to_string());
        let last_synced = self
            .cloud_sync_state
            .last_synced_at_ms
            .map(format_history_timestamp_ms)
            .unwrap_or_else(|| self.tr("settings.never").to_string());
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
        let provider_fields = match active_cloud_provider.as_str() {
            "webdav" => self.cloud_sync_webdav_provider_fields(webdav_password_value, cx),
            "s3" => self.cloud_sync_s3_provider_fields(
                s3_access_key_value,
                s3_secret_key_value,
                s3_session_token_value,
                cx,
            ),
            "google_drive" => self.cloud_sync_oauth_provider_fields(
                "google_drive",
                google_drive_access_token_value,
                google_drive_refresh_token_value,
                google_drive_client_secret_value,
                cx,
            ),
            "onedrive" => self.cloud_sync_oauth_provider_fields(
                "onedrive",
                onedrive_access_token_value,
                onedrive_refresh_token_value,
                onedrive_client_secret_value,
                cx,
            ),
            "aliyun_drive" => self.cloud_sync_aliyun_provider_fields(
                aliyun_drive_access_token_value,
                aliyun_drive_refresh_token_value,
                aliyun_drive_client_secret_value,
                cx,
            ),
            "gitee_snippet" => self.cloud_sync_gitee_provider_fields(gitee_token_value, cx),
            "github_gist" => self.cloud_sync_github_provider_fields(cx),
            _ => div().into_any_element(),
        };

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(settings_form_section(
                palette,
                Some(self.tr("settings.syncProviderConfig")),
                Some(self.tr("settings.syncProviderConfigDesc")),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.enableCloudSync"),
                        Some(SharedString::from(self.tr("settings.enableCloudSyncDesc"))),
                        settings_switch_with_enabled(
                            palette,
                            "cloud-sync-enabled",
                            self.cloud_sync_settings.enabled,
                            form_enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_cloud_sync_enabled(cx);
                            }),
                        ),
                    ))
                    .when(!form_enabled, |this| {
                        this.child(
                            div()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.warning))
                                .bg(rgba((palette.warning << 8) | 0x14))
                                .p_3()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_3()
                                .child(
                                    div()
                                        .min_w_0()
                                        .text_size(px(12.))
                                        .text_color(rgb(palette.text_muted))
                                        .child(self.tr("settings.syncMasterPasswordMissingDesc")),
                                )
                                .child(small_button(
                                    palette,
                                    "cloud-open-security",
                                    self.tr("settings.openSecuritySettings"),
                                    cx.listener(|this, _, _, cx| {
                                        this.settings_active_tab = SettingsTab::Security;
                                        cx.notify();
                                    }),
                                )),
                        )
                    })
                    .when_some(validation_message, |this, message| {
                        this.child(
                            div()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.warning))
                                .bg(rgba((palette.warning << 8) | 0x14))
                                .px_3()
                                .py_2()
                                .text_size(px(12.))
                                .text_color(rgb(palette.text_muted))
                                .child(message),
                        )
                    })
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.syncProvider"),
                        Some(SharedString::from(self.tr("settings.syncProviderDesc"))),
                        self.cloud_sync_provider_select(&active_cloud_provider, form_enabled, cx),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.deviceName"),
                        Some(SharedString::from(self.tr("settings.deviceNameDesc"))),
                        self.cloud_sync_input(
                            "cloud-sync-device-name",
                            self.tr("settings.deviceName"),
                            self.cloud_sync_settings.device_name.clone(),
                            CloudSyncInputField::DeviceName,
                            cx,
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.remoteNamespace"),
                        Some(SharedString::from(self.tr("settings.remoteNamespaceDesc"))),
                        self.cloud_sync_input(
                            "cloud-sync-remote-root",
                            self.tr("settings.remoteNamespace"),
                            self.cloud_sync_settings.remote_root.clone(),
                            CloudSyncInputField::RemoteRoot,
                            cx,
                        ),
                    ))
                    .child(provider_fields),
            ))
            .child(settings_form_section(
                palette,
                Some(self.tr("settings.autoSyncStrategy")),
                Some(self.tr("settings.autoSyncStrategyDesc")),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .when_some(action_block_message, |this, message| {
                        this.child(
                            div()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.warning))
                                .bg(rgba((palette.warning << 8) | 0x14))
                                .px_3()
                                .py_2()
                                .text_size(px(12.))
                                .text_color(rgb(palette.text_muted))
                                .child(message),
                        )
                    })
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.autoCheckOnStartup"),
                        Some(SharedString::from(
                            self.tr("settings.autoCheckOnStartupDesc"),
                        )),
                        settings_switch_with_enabled(
                            palette,
                            "cloud-auto-check",
                            self.cloud_sync_settings.auto_check_on_startup,
                            auto_sync_enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_cloud_sync_auto_check(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.autoPushOnChange"),
                        Some(SharedString::from(self.tr("settings.autoPushOnChangeDesc"))),
                        settings_switch_with_enabled(
                            palette,
                            "cloud-auto-push",
                            self.cloud_sync_settings.auto_push_on_change,
                            auto_sync_enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_cloud_sync_auto_push(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.syncDebounceSeconds"),
                        Some(SharedString::from(
                            self.tr("settings.syncDebounceSecondsDesc"),
                        )),
                        cloud_sync_number_stepper(
                            palette,
                            self.cloud_sync_settings.sync_debounce_seconds,
                            debounce_enabled,
                            cx.listener(|this, _, _, cx| {
                                this.adjust_cloud_sync_debounce(-1, cx);
                            }),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_cloud_sync_debounce(1, cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some(self.tr("settings.manualSyncActions")),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap_2()
                            .child(cloud_sync_status_item(
                                palette,
                                self.tr("settings.syncStatus"),
                                self.tr(sync_state_key).to_string(),
                            ))
                            .child(cloud_sync_status_item(
                                palette,
                                self.tr("settings.syncProvider"),
                                provider_label,
                            ))
                            .child(cloud_sync_status_item(
                                palette,
                                self.tr("settings.lastSyncCheck"),
                                last_checked,
                            ))
                            .child(cloud_sync_status_item(
                                palette,
                                self.tr("settings.lastSyncAt"),
                                last_synced,
                            ))
                            .child(cloud_sync_status_item(
                                palette,
                                self.tr("settings.currentOperation"),
                                current_operation,
                            )),
                    )
                    .when(!self.cloud_sync_status.is_empty(), |this| {
                        this.child(
                            div()
                                .min_w_0()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.surface_elevated))
                                .px_3()
                                .py_2()
                                .text_size(px(12.))
                                .text_color(rgb(palette.text_muted))
                                .child(self.cloud_sync_status.clone()),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(cloud_sync_action_button(
                                palette,
                                "settings-provider-cloud-sync-test",
                                self.tr("settings.testConnection"),
                                can_run_actions,
                                cx.listener(|this, _, _, cx| {
                                    this.run_provider_cloud_sync_test(cx);
                                }),
                            ))
                            .child(cloud_sync_action_button(
                                palette,
                                "settings-provider-cloud-sync-push",
                                self.tr("settings.syncPushNow"),
                                can_run_enabled_actions,
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_provider_cloud_sync_push(cx);
                                }),
                            ))
                            .child(cloud_sync_action_button(
                                palette,
                                "settings-provider-cloud-sync-pull",
                                self.tr("settings.syncPullNow"),
                                can_run_enabled_actions,
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_provider_cloud_sync_pull(cx);
                                }),
                            )),
                    ),
            ))
            .when_some(cloud_snapshot_prompt, |this, prompt| {
                this.child(self.snapshot_password_prompt_banner(prompt, cx))
            })
            .child(settings_form_section(
                palette,
                Some(self.tr("settings.syncConflictSection")),
                Some(self.tr("settings.syncConflictSectionDesc")),
                if let Some(conflict) = cloud_conflict {
                    self.cloud_sync_conflict_banner(conflict, cx)
                        .into_any_element()
                } else {
                    div()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .px_4()
                        .py_5()
                        .text_center()
                        .text_size(px(12.))
                        .text_color(rgb(palette.text_muted))
                        .child(self.tr("settings.noSyncConflict"))
                        .into_any_element()
                },
            ))
    }
}

fn cloud_sync_conflict_stat(
    palette: crate::theme::ThemePalette,
    label: &'static str,
    value: String,
) -> impl IntoElement {
    div()
        .min_w_0()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette.text_muted))
                .child(label),
        )
        .child(
            div()
                .mt_2()
                .font_family(crate::features::gpui_code_font_family())
                .text_xs()
                .text_color(rgb(palette.text))
                .child(value),
        )
}

fn cloud_sync_provider_label(provider: &str) -> &'static str {
    match provider {
        "webdav" => "WebDAV",
        "s3" => "S3 Compatible",
        "gitee_snippet" => "Gitee Snippet",
        "github_gist" => "GitHub Gist",
        "google_drive" => "Google Drive",
        "onedrive" => "OneDrive",
        "aliyun_drive" => "AliyunDrive",
        _ => "-",
    }
}

fn cloud_sync_state_i18n_key(enabled: bool, has_conflict: bool, message: &str) -> &'static str {
    if has_conflict {
        return "settings.syncState.conflict";
    }
    if !enabled {
        return "settings.syncState.disabled";
    }
    let message = message.to_ascii_lowercase();
    if message.contains("failed") || message.contains("error") {
        "settings.syncState.failed"
    } else if message.contains("testing")
        || message.contains("pushing")
        || message.contains("pulling")
        || message.contains("started")
        || message.contains("awaiting")
    {
        "settings.syncState.running"
    } else if message.contains("success") || message.contains("up to date") {
        "settings.syncState.success"
    } else {
        "settings.syncState.idle"
    }
}

fn cloud_sync_validation_key(settings: &CloudSyncSettings) -> Option<&'static str> {
    match settings.provider.as_str() {
        "webdav" if settings.webdav.endpoint.trim().is_empty() => {
            Some("settings.webdavEndpointRequired")
        }
        "s3" if settings.s3.endpoint.trim().is_empty() => Some("settings.s3EndpointRequired"),
        "s3" if settings.s3.bucket.trim().is_empty() => Some("settings.s3BucketRequired"),
        "s3" if settings
            .s3
            .access_key_id
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            != settings
                .s3
                .secret_access_key
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty() =>
        {
            Some("settings.s3CredentialsIncomplete")
        }
        "gitee_snippet" if settings.gitee_snippet.api_endpoint.trim().is_empty() => {
            Some("settings.giteeSnippetEndpointRequired")
        }
        "gitee_snippet" if settings.gitee_snippet.gist_id.trim().is_empty() => {
            Some("settings.giteeSnippetIdRequired")
        }
        "gitee_snippet"
            if settings
                .gitee_snippet
                .access_token
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty() =>
        {
            Some("settings.giteeSnippetTokenRequired")
        }
        "google_drive" => cloud_sync_drive_validation_key(
            settings.google_drive.refresh_token.as_deref(),
            settings.google_drive.client_id.as_deref(),
            settings.google_drive.client_secret.as_deref(),
        ),
        "onedrive" => cloud_sync_drive_validation_key(
            settings.onedrive.refresh_token.as_deref(),
            settings.onedrive.client_id.as_deref(),
            settings.onedrive.client_secret.as_deref(),
        ),
        "aliyun_drive" => cloud_sync_drive_validation_key(
            settings.aliyun_drive.refresh_token.as_deref(),
            settings.aliyun_drive.client_id.as_deref(),
            settings.aliyun_drive.client_secret.as_deref(),
        ),
        "github_gist" if settings.github_gist.gist_id.trim().is_empty() => {
            Some("settings.githubGistRequired")
        }
        "github_gist"
            if settings
                .github_gist
                .access_token
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty() =>
        {
            Some("settings.githubGistTokenRequired")
        }
        _ => None,
    }
}

fn cloud_sync_drive_validation_key(
    refresh_token: Option<&str>,
    client_id: Option<&str>,
    client_secret: Option<&str>,
) -> Option<&'static str> {
    if refresh_token.unwrap_or_default().trim().is_empty() {
        Some("settings.driveRefreshTokenRequired")
    } else if client_id.unwrap_or_default().trim().is_empty() {
        Some("settings.driveClientIdRequired")
    } else if client_secret.unwrap_or_default().trim().is_empty() {
        Some("settings.driveClientSecretRequired")
    } else {
        None
    }
}

fn cloud_sync_action_button(
    palette: ThemePalette,
    id: &'static str,
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let hover = palette.hover;
    div()
        .id(id)
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface_elevated))
        .text_color(rgb(palette.text))
        .text_xs()
        .opacity(if enabled { 1.0 } else { 0.45 })
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(move |this| this.bg(rgb(hover)))
                .on_click(on_click)
        })
        .child(label)
}

fn cloud_sync_number_stepper(
    palette: ThemePalette,
    value: u64,
    enabled: bool,
    on_minus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_plus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .opacity(if enabled { 1.0 } else { 0.45 })
        .child(cloud_sync_step_button(
            palette,
            "cloud-debounce-minus",
            "-",
            enabled,
            on_minus,
        ))
        .child(
            div()
                .min_w(px(56.))
                .text_center()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(11.))
                .text_color(rgb(palette.text))
                .child(value.to_string()),
        )
        .child(cloud_sync_step_button(
            palette,
            "cloud-debounce-plus",
            "+",
            enabled,
            on_plus,
        ))
}

fn cloud_sync_step_button(
    palette: ThemePalette,
    id: &'static str,
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let hover = palette.hover;
    div()
        .id(id)
        .size(px(28.))
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(12.))
        .text_color(rgb(palette.text_muted))
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(move |this| this.bg(rgb(hover)))
                .on_click(on_click)
        })
        .child(label)
}

fn cloud_sync_status_item(
    palette: ThemePalette,
    label: &'static str,
    value: String,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .px_3()
        .py_2()
        .min_w_0()
        .child(
            div()
                .text_size(px(10.))
                .font_weight(FontWeight(600.))
                .text_color(rgb(palette.text_muted))
                .child(label),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(12.))
                .text_color(rgb(palette.text))
                .overflow_hidden()
                .child(value),
        )
}
