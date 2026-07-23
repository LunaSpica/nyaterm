use super::*;

impl NyaTermApp {
    pub(in crate::features) fn prompt_config_export(&mut self, cx: &mut Context<Self>) {
        if self.config_path_prompt.is_some() {
            self.terminal_status = "config path picker is already open".to_string();
            cx.notify();
            return;
        }

        let directory = self.runtime.config_dir().to_path_buf();
        let receiver = cx.prompt_for_new_path(&directory, Some("nyaterm-backup.redb"));
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.config_path_prompt = Some(ConfigPathPromptKind::Export);
        self.terminal_status = "selecting config backup destination".to_string();
        self.store_status.message = "selecting backup destination".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(path))) => {
                    cx.background_spawn(async move {
                        match ConnectionStore::export_config_database(
                            &config_dir,
                            portable_key_path,
                            &path,
                        ) {
                            Ok(info) => ConfigPathPromptResult::Exported(info),
                            Err(error) => ConfigPathPromptResult::Failed(error.to_string()),
                        }
                    })
                    .await
                }
                Ok(Ok(None)) => ConfigPathPromptResult::Cancelled,
                Ok(Err(error)) => ConfigPathPromptResult::Failed(error.to_string()),
                Err(_) => ConfigPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_config_path_prompt_result(ConfigPathPromptKind::Export, result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features) fn prompt_portable_snapshot_export(&mut self, cx: &mut Context<Self>) {
        if self.config_path_prompt.is_some() {
            self.terminal_status = "config path picker is already open".to_string();
            cx.notify();
            return;
        }

        let directory = self.runtime.config_dir().to_path_buf();
        let receiver = cx.prompt_for_new_path(&directory, Some("nyaterm-backup.nya"));
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.config_path_prompt = Some(ConfigPathPromptKind::PortableExport);
        self.terminal_status = "selecting portable snapshot destination".to_string();
        self.store_status.message = "selecting .nya export destination".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(path))) => {
                    cx.background_spawn(async move {
                        match ConnectionStore::export_portable_snapshot(
                            &config_dir,
                            portable_key_path,
                            &path,
                            "native-local",
                            env!("CARGO_PKG_VERSION"),
                        ) {
                            Ok(info) => ConfigPathPromptResult::Exported(info),
                            Err(error) => ConfigPathPromptResult::Failed(error.to_string()),
                        }
                    })
                    .await
                }
                Ok(Ok(None)) => ConfigPathPromptResult::Cancelled,
                Ok(Err(error)) => ConfigPathPromptResult::Failed(error.to_string()),
                Err(_) => ConfigPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_config_path_prompt_result(ConfigPathPromptKind::PortableExport, result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features) fn prompt_encrypted_portable_snapshot_export(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::Export, cx);
    }

    pub(in crate::features) fn prompt_config_import(&mut self, cx: &mut Context<Self>) {
        if self.block_import_for_settings_draft(cx) {
            return;
        }
        if self.config_path_prompt.is_some() {
            self.terminal_status = "config path picker is already open".to_string();
            cx.notify();
            return;
        }
        if self.active_session_id.is_some() || self.has_pending_session_start() {
            self.terminal_status = "close active session before importing config".to_string();
            cx.notify();
            return;
        }

        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from("Select config backup")),
        };
        let receiver = cx.prompt_for_paths(options);
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.config_path_prompt = Some(ConfigPathPromptKind::Import);
        self.terminal_status = "selecting config backup to import".to_string();
        self.store_status.message = "selecting backup source".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => match paths.into_iter().next() {
                    Some(path) => {
                        cx.background_spawn(async move {
                            match ConnectionStore::import_config_database(
                                &config_dir,
                                portable_key_path,
                                &path,
                            ) {
                                Ok(info) => ConfigPathPromptResult::Imported(info),
                                Err(error) => ConfigPathPromptResult::Failed(error.to_string()),
                            }
                        })
                        .await
                    }
                    None => ConfigPathPromptResult::Cancelled,
                },
                Ok(Ok(None)) => ConfigPathPromptResult::Cancelled,
                Ok(Err(error)) => ConfigPathPromptResult::Failed(error.to_string()),
                Err(_) => ConfigPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_config_path_prompt_result(ConfigPathPromptKind::Import, result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features) fn prompt_portable_snapshot_import(&mut self, cx: &mut Context<Self>) {
        if self.block_import_for_settings_draft(cx) {
            return;
        }
        if self.config_path_prompt.is_some() {
            self.terminal_status = "config path picker is already open".to_string();
            cx.notify();
            return;
        }
        if self.active_session_id.is_some() || self.has_pending_session_start() {
            self.terminal_status = "close active session before importing config".to_string();
            cx.notify();
            return;
        }

        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from("Select .nya snapshot")),
        };
        let receiver = cx.prompt_for_paths(options);
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.config_path_prompt = Some(ConfigPathPromptKind::PortableImport);
        self.terminal_status = "selecting portable snapshot to import".to_string();
        self.store_status.message = "selecting .nya snapshot".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => match paths.into_iter().next() {
                    Some(path) => {
                        cx.background_spawn(async move {
                            match ConnectionStore::import_portable_snapshot(
                                &config_dir,
                                portable_key_path,
                                &path,
                            ) {
                                Ok(info) => ConfigPathPromptResult::Imported(info),
                                Err(error) => ConfigPathPromptResult::Failed(error.to_string()),
                            }
                        })
                        .await
                    }
                    None => ConfigPathPromptResult::Cancelled,
                },
                Ok(Ok(None)) => ConfigPathPromptResult::Cancelled,
                Ok(Err(error)) => ConfigPathPromptResult::Failed(error.to_string()),
                Err(_) => ConfigPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_config_path_prompt_result(ConfigPathPromptKind::PortableImport, result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features) fn prompt_encrypted_portable_snapshot_import(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.block_import_for_settings_draft(cx) {
            return;
        }
        if self.active_session_id.is_some() || self.has_pending_session_start() {
            self.terminal_status = "close active session before importing config".to_string();
            cx.notify();
            return;
        }
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::Import, cx);
    }

    pub(in crate::features) fn start_snapshot_password_prompt(
        &mut self,
        kind: SnapshotPasswordPromptKind,
        cx: &mut Context<Self>,
    ) {
        if self.config_path_prompt.is_some() {
            self.terminal_status = "config path picker is already open".to_string();
            cx.notify();
            return;
        }
        self.active_snapshot_password_prompt = Some(SnapshotPasswordPromptState {
            kind,
            value: String::new(),
        });
        self.terminal_status = match kind {
            SnapshotPasswordPromptKind::Export => "enter password for encrypted .nya export",
            SnapshotPasswordPromptKind::Import => "enter password for encrypted .nya import",
            SnapshotPasswordPromptKind::CloudPush => "enter password for cloud sync push",
            SnapshotPasswordPromptKind::CloudPull => "enter password for cloud sync pull",
            SnapshotPasswordPromptKind::CloudForcePush => {
                "enter password for forced cloud sync push"
            }
            SnapshotPasswordPromptKind::CloudForcePull => {
                "enter password for forced cloud sync pull"
            }
            SnapshotPasswordPromptKind::CloudProviderPush => {
                "enter password for provider cloud sync push"
            }
            SnapshotPasswordPromptKind::CloudProviderPull => {
                "enter password for provider cloud sync pull"
            }
            SnapshotPasswordPromptKind::CloudProviderForcePush => {
                "enter password for forced provider cloud sync push"
            }
            SnapshotPasswordPromptKind::CloudProviderForcePull => {
                "enter password for forced provider cloud sync pull"
            }
        }
        .to_string();
        self.store_status.message = match kind {
            SnapshotPasswordPromptKind::CloudPush
            | SnapshotPasswordPromptKind::CloudPull
            | SnapshotPasswordPromptKind::CloudForcePush
            | SnapshotPasswordPromptKind::CloudForcePull
            | SnapshotPasswordPromptKind::CloudProviderPush
            | SnapshotPasswordPromptKind::CloudProviderPull
            | SnapshotPasswordPromptKind::CloudProviderForcePush
            | SnapshotPasswordPromptKind::CloudProviderForcePull => {
                "awaiting cloud sync password".to_string()
            }
            _ => "awaiting .nya master password".to_string(),
        };
        cx.notify();
    }

    pub(in crate::features) fn submit_snapshot_password_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.active_snapshot_password_prompt.take() else {
            return;
        };
        let password = state.value.trim().to_string();
        if password.is_empty() {
            self.active_snapshot_password_prompt = Some(SnapshotPasswordPromptState {
                kind: state.kind,
                value: String::new(),
            });
            self.terminal_status = "master password is required for encrypted .nya".to_string();
            cx.notify();
            return;
        }

        match state.kind {
            SnapshotPasswordPromptKind::Export => {
                self.prompt_encrypted_portable_snapshot_export_path(password, cx);
            }
            SnapshotPasswordPromptKind::Import => {
                self.prompt_encrypted_portable_snapshot_import_path(password, cx);
            }
            SnapshotPasswordPromptKind::CloudPush => {
                self.run_local_cloud_sync_push(password, false, cx);
            }
            SnapshotPasswordPromptKind::CloudPull => {
                self.run_local_cloud_sync_pull(password, false, cx);
            }
            SnapshotPasswordPromptKind::CloudForcePush => {
                self.run_local_cloud_sync_push(password, true, cx);
            }
            SnapshotPasswordPromptKind::CloudForcePull => {
                self.run_local_cloud_sync_pull(password, true, cx);
            }
            SnapshotPasswordPromptKind::CloudProviderPush => {
                self.run_provider_cloud_sync_push(password, false, cx);
            }
            SnapshotPasswordPromptKind::CloudProviderPull => {
                self.run_provider_cloud_sync_pull(password, false, cx);
            }
            SnapshotPasswordPromptKind::CloudProviderForcePush => {
                self.run_provider_cloud_sync_push(password, true, cx);
            }
            SnapshotPasswordPromptKind::CloudProviderForcePull => {
                self.run_provider_cloud_sync_pull(password, true, cx);
            }
        }
    }

    pub(in crate::features) fn cancel_snapshot_password_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.active_snapshot_password_prompt.take() else {
            return;
        };
        self.terminal_status = match state.kind {
            SnapshotPasswordPromptKind::Export => "encrypted .nya export cancelled".to_string(),
            SnapshotPasswordPromptKind::Import => "encrypted .nya import cancelled".to_string(),
            SnapshotPasswordPromptKind::CloudPush => "cloud sync push cancelled".to_string(),
            SnapshotPasswordPromptKind::CloudPull => "cloud sync pull cancelled".to_string(),
            SnapshotPasswordPromptKind::CloudForcePush => {
                "forced cloud sync push cancelled".to_string()
            }
            SnapshotPasswordPromptKind::CloudForcePull => {
                "forced cloud sync pull cancelled".to_string()
            }
            SnapshotPasswordPromptKind::CloudProviderPush => {
                "provider cloud sync push cancelled".to_string()
            }
            SnapshotPasswordPromptKind::CloudProviderPull => {
                "provider cloud sync pull cancelled".to_string()
            }
            SnapshotPasswordPromptKind::CloudProviderForcePush => {
                "forced provider cloud sync push cancelled".to_string()
            }
            SnapshotPasswordPromptKind::CloudProviderForcePull => {
                "forced provider cloud sync pull cancelled".to_string()
            }
        };
        self.store_status.message = "config picker cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn handle_snapshot_password_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let Some(state) = self.active_snapshot_password_prompt.as_mut() else {
            return;
        };
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "enter" => self.submit_snapshot_password_prompt(cx),
            "escape" => self.cancel_snapshot_password_prompt(cx),
            "backspace" => {
                state.value.pop();
                cx.notify();
            }
            _ => {
                if let Some(value) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|value| !value.is_empty())
                {
                    state.value.push_str(value);
                    cx.notify();
                }
            }
        }
    }

    fn prompt_encrypted_portable_snapshot_export_path(
        &mut self,
        master_password: String,
        cx: &mut Context<Self>,
    ) {
        let directory = self.runtime.config_dir().to_path_buf();
        let receiver = cx.prompt_for_new_path(&directory, Some("nyaterm-encrypted.nya"));
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.config_path_prompt = Some(ConfigPathPromptKind::EncryptedPortableExport);
        self.terminal_status = "selecting encrypted portable snapshot destination".to_string();
        self.store_status.message = "selecting encrypted .nya export destination".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(path))) => {
                    cx.background_spawn(async move {
                        match ConnectionStore::export_encrypted_portable_snapshot(
                            &config_dir,
                            portable_key_path,
                            &path,
                            "native-local",
                            env!("CARGO_PKG_VERSION"),
                            &master_password,
                        ) {
                            Ok(info) => ConfigPathPromptResult::Exported(info),
                            Err(error) => ConfigPathPromptResult::Failed(error.to_string()),
                        }
                    })
                    .await
                }
                Ok(Ok(None)) => ConfigPathPromptResult::Cancelled,
                Ok(Err(error)) => ConfigPathPromptResult::Failed(error.to_string()),
                Err(_) => ConfigPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_config_path_prompt_result(
                    ConfigPathPromptKind::EncryptedPortableExport,
                    result,
                );
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn prompt_encrypted_portable_snapshot_import_path(
        &mut self,
        master_password: String,
        cx: &mut Context<Self>,
    ) {
        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from("Select encrypted .nya snapshot")),
        };
        let receiver = cx.prompt_for_paths(options);
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.config_path_prompt = Some(ConfigPathPromptKind::EncryptedPortableImport);
        self.terminal_status = "selecting encrypted portable snapshot to import".to_string();
        self.store_status.message = "selecting encrypted .nya snapshot".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => match paths.into_iter().next() {
                    Some(path) => {
                        cx.background_spawn(async move {
                            match ConnectionStore::import_encrypted_portable_snapshot(
                                &config_dir,
                                portable_key_path,
                                &path,
                                &master_password,
                            ) {
                                Ok(info) => ConfigPathPromptResult::Imported(info),
                                Err(error) => ConfigPathPromptResult::Failed(error.to_string()),
                            }
                        })
                        .await
                    }
                    None => ConfigPathPromptResult::Cancelled,
                },
                Ok(Ok(None)) => ConfigPathPromptResult::Cancelled,
                Ok(Err(error)) => ConfigPathPromptResult::Failed(error.to_string()),
                Err(_) => ConfigPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_config_path_prompt_result(
                    ConfigPathPromptKind::EncryptedPortableImport,
                    result,
                );
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_config_path_prompt_result(
        &mut self,
        kind: ConfigPathPromptKind,
        result: ConfigPathPromptResult,
    ) {
        self.config_path_prompt = None;
        match result {
            ConfigPathPromptResult::Exported(info) => {
                self.store_status.path = info.database_path.display().to_string();
                self.store_status.message = match kind {
                    ConfigPathPromptKind::PortableExport => {
                        format!("exported {} byte .nya snapshot", info.bytes)
                    }
                    ConfigPathPromptKind::EncryptedPortableExport => {
                        format!("exported {} byte encrypted .nya snapshot", info.bytes)
                    }
                    _ => format!("exported {} byte config backup", info.bytes),
                };
                self.store_status.ready = true;
                self.terminal_status = match kind {
                    ConfigPathPromptKind::PortableExport => {
                        format!(
                            "portable snapshot exported to {}",
                            info.backup_path.display()
                        )
                    }
                    ConfigPathPromptKind::EncryptedPortableExport => {
                        format!(
                            "encrypted portable snapshot exported to {}",
                            info.backup_path.display()
                        )
                    }
                    _ => format!("config exported to {}", info.backup_path.display()),
                };
            }
            ConfigPathPromptResult::Imported(info) => {
                self.refresh_store_from_runtime();
                self.rebase_open_settings_draft();
                let safety = info
                    .safety_backup_path
                    .as_ref()
                    .map(|path| format!("; previous db saved to {}", path.display()))
                    .unwrap_or_default();
                self.store_status.message = match kind {
                    ConfigPathPromptKind::PortableImport => {
                        format!("imported {} byte .nya snapshot{safety}", info.bytes)
                    }
                    ConfigPathPromptKind::EncryptedPortableImport => {
                        format!(
                            "imported {} byte encrypted .nya snapshot{safety}",
                            info.bytes
                        )
                    }
                    _ => format!("imported {} byte config backup{safety}", info.bytes),
                };
                self.store_status.ready = true;
                self.terminal_status = match kind {
                    ConfigPathPromptKind::PortableImport => {
                        format!(
                            "portable snapshot imported from {}",
                            info.backup_path.display()
                        )
                    }
                    ConfigPathPromptKind::EncryptedPortableImport => {
                        format!(
                            "encrypted portable snapshot imported from {}",
                            info.backup_path.display()
                        )
                    }
                    _ => format!("config imported from {}", info.backup_path.display()),
                };
            }
            ConfigPathPromptResult::Cancelled => {
                self.terminal_status = match kind {
                    ConfigPathPromptKind::Export => "config export cancelled".to_string(),
                    ConfigPathPromptKind::Import => "config import cancelled".to_string(),
                    ConfigPathPromptKind::PortableExport => {
                        "portable snapshot export cancelled".to_string()
                    }
                    ConfigPathPromptKind::PortableImport => {
                        "portable snapshot import cancelled".to_string()
                    }
                    ConfigPathPromptKind::EncryptedPortableExport => {
                        "encrypted portable snapshot export cancelled".to_string()
                    }
                    ConfigPathPromptKind::EncryptedPortableImport => {
                        "encrypted portable snapshot import cancelled".to_string()
                    }
                };
                self.store_status.message = "config picker cancelled".to_string();
            }
            ConfigPathPromptResult::Failed(error) => {
                self.terminal_status = match kind {
                    ConfigPathPromptKind::Export => format!("config export failed: {error}"),
                    ConfigPathPromptKind::Import => format!("config import failed: {error}"),
                    ConfigPathPromptKind::PortableExport => {
                        format!("portable snapshot export failed: {error}")
                    }
                    ConfigPathPromptKind::PortableImport => {
                        format!("portable snapshot import failed: {error}")
                    }
                    ConfigPathPromptKind::EncryptedPortableExport => {
                        format!("encrypted portable snapshot export failed: {error}")
                    }
                    ConfigPathPromptKind::EncryptedPortableImport => {
                        format!("encrypted portable snapshot import failed: {error}")
                    }
                };
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
            ConfigPathPromptResult::Closed => {
                self.terminal_status = "config path picker closed before returning".to_string();
                self.store_status.message = "config picker closed".to_string();
            }
        }
    }

    pub(in crate::features) fn refresh_store_from_runtime(&mut self) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => {
                let path = store.db_path().display().to_string();
                match store.load_sessions() {
                    Ok(config) => {
                        self.connections = config.connections;
                        self.connection_groups = config.groups;
                        self.connection_ssh_keys = store.list_ssh_keys().unwrap_or_default();
                        self.connection_otp_entries = store.list_otp_entries().unwrap_or_default();
                        self.connection_saved_passwords =
                            store.list_passwords().unwrap_or_default();
                        self.connection_saved_credentials =
                            store.list_credentials().unwrap_or_default();
                        self.tunnels = store.list_tunnels().unwrap_or_default();
                        self.tunnel_groups = store.list_tunnel_groups().unwrap_or_default();
                        self.proxies = store.list_proxies().unwrap_or_default();
                        self.proxy_groups = store.list_proxy_groups().unwrap_or_default();
                        let quick_commands = store.load_quick_commands().unwrap_or_default();
                        self.quick_commands = Arc::from(quick_commands.commands);
                        self.quick_command_categories = quick_commands.categories;
                        self.command_history =
                            Arc::from(store.list_command_history(64).unwrap_or_default());
                        self.keyword_highlights =
                            store.load_keyword_highlights().unwrap_or_default();
                        self.apply_gpui_settings(
                            store.load_app_settings_summary().unwrap_or_default(),
                        );
                        self.apply_ui_layout_from_settings();
                        self.translation_settings = store
                            .load_translation_settings()
                            .unwrap_or_else(|_| TranslationSettings {
                                target_language: self.settings.language.clone(),
                                ..TranslationSettings::default()
                            });
                        self.translation_secret_draft = TranslationSecretDraft::default();
                        self.translate_target_language =
                            self.translation_settings.target_language.clone();
                        self.recording_manager
                            .set_memory_limit(self.settings.recording_memory_limit_bytes as usize);
                        self.cloud_sync_settings = store
                            .load_cloud_sync_settings()
                            .unwrap_or_else(|_| self.cloud_sync_settings.clone());
                        self.cloud_sync_secret_draft = CloudSyncSecretDraft::default();
                        self.ai_settings = store
                            .load_ai_settings()
                            .unwrap_or_else(|_| self.ai_settings.clone());
                        self.ai_secret_draft.clear();
                        self.sync_ai_drafts_from_active_profile();
                        self.settings_master_password_enabled = self.settings.has_master_password;
                        self.settings_master_password_draft.clear();
                        self.cloud_sync_state = store
                            .load_cloud_sync_state()
                            .unwrap_or_else(|_| self.cloud_sync_state.clone());
                        self.transfer_duplicate_policy = SftpDuplicatePolicy::from_legacy_value(
                            &self.settings.transfer_duplicate_strategy,
                        );
                        self.store_status = StoreStatus {
                            path,
                            message: "redb connection store online".to_string(),
                            ready: true,
                        };
                    }
                    Err(error) => {
                        self.connections.clear();
                        self.connection_groups.clear();
                        self.connection_ssh_keys.clear();
                        self.connection_otp_entries.clear();
                        self.connection_saved_passwords.clear();
                        self.connection_saved_credentials.clear();
                        self.tunnels.clear();
                        self.tunnel_groups.clear();
                        self.proxies.clear();
                        self.proxy_groups.clear();
                        self.quick_commands = Arc::default();
                        self.quick_command_categories.clear();
                        self.command_history = Arc::default();
                        self.keyword_highlights = KeywordHighlightConfig::default();
                        self.apply_gpui_settings(AppSettingsSummary::default());
                        self.translation_settings = TranslationSettings::default();
                        self.translation_secret_draft = TranslationSecretDraft::default();
                        self.translate_target_language =
                            self.translation_settings.target_language.clone();
                        self.store_status = StoreStatus {
                            path,
                            message: format!("failed to load sessions: {error}"),
                            ready: false,
                        };
                    }
                }
            }
            Err(error) => {
                self.connections.clear();
                self.tunnels.clear();
                self.tunnel_groups.clear();
                self.proxies.clear();
                self.proxy_groups.clear();
                self.quick_commands = Arc::default();
                self.quick_command_categories.clear();
                self.command_history = Arc::default();
                self.apply_gpui_settings(AppSettingsSummary::default());
                self.translation_settings = TranslationSettings::default();
                self.translation_secret_draft = TranslationSecretDraft::default();
                self.translate_target_language = self.translation_settings.target_language.clone();
                self.store_status = StoreStatus {
                    path: self
                        .runtime
                        .config_dir()
                        .join("nyaterm.redb")
                        .display()
                        .to_string(),
                    message: format!("failed to open store: {error}"),
                    ready: false,
                };
            }
        }
    }
}
