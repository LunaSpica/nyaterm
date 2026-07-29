use gpui::{AppContext, Context, KeyDownEvent, PathPromptOptions, SharedString, Window};
use nyaterm_core::{
    AppSettingsSummary, ConnectionStore, KeywordHighlightConfig, TranslationSettings,
};
use nyaterm_transport::SftpDuplicatePolicy;

use crate::features::{NyaTermApp, TextInputSetup};
use crate::models::{
    ConfigPathPromptKind, ConfigPathPromptResult, SnapshotPasswordPromptKind,
    TranslationSecretDraft,
};

impl NyaTermApp {
    pub(in crate::features) fn prompt_config_export(&mut self, cx: &mut Context<Self>) {
        if !self
            .settings
            .begin_config_path_prompt(ConfigPathPromptKind::Export)
        {
            self.shell
                .set_status("config path picker is already open".to_string());
            cx.notify();
            return;
        }

        let directory = self.runtime.config_dir().to_path_buf();
        let receiver = cx.prompt_for_new_path(&directory, Some("nyaterm-backup.redb"));
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.shell
            .set_status("selecting config backup destination".to_string());
        self.settings
            .set_store_message("selecting backup destination");
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

    pub(in crate::features) fn prompt_portable_snapshot_import(&mut self, cx: &mut Context<Self>) {
        if self.block_import_for_settings_draft(cx) {
            return;
        }
        if self.settings.config_path_prompt_active() {
            self.shell
                .set_status("config path picker is already open".to_string());
            cx.notify();
            return;
        }
        if self.session.active_id().is_some() || self.has_pending_session_start() {
            self.shell
                .set_status("close active session before importing config".to_string());
            cx.notify();
            return;
        }
        let prompt_started = self
            .settings
            .begin_config_path_prompt(ConfigPathPromptKind::PortableImport);
        debug_assert!(prompt_started);
        if !prompt_started {
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
        self.shell
            .set_status("selecting portable snapshot to import".to_string());
        self.settings.set_store_message("selecting .nya snapshot");
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

    pub(in crate::features) fn start_snapshot_password_prompt(
        &mut self,
        kind: SnapshotPasswordPromptKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.settings.begin_snapshot_password_prompt(kind) {
            self.shell
                .set_status("config path picker is already open".to_string());
            cx.notify();
            return;
        }
        self.forget_text_inputs("snapshot-password.");
        let field = self.text_input("snapshot-password.value", "", TextInputSetup::masked(), cx);
        window.focus(&field.read(cx).focus_handle());
        self.shell.set_status(
            match kind {
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
            .to_string(),
        );
        let store_message = match kind {
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
        self.settings.set_store_message(store_message);
        cx.notify();
    }

    pub(in crate::features) fn submit_snapshot_password_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.settings.take_snapshot_password_prompt() else {
            return;
        };
        let password = state.value.trim().to_string();
        if password.is_empty() {
            self.settings.restore_snapshot_password_prompt(state.kind);
            self.reset_text_input("snapshot-password.value", "", cx);
            self.shell
                .set_status("master password is required for encrypted .nya".to_string());
            cx.notify();
            return;
        }
        self.forget_text_inputs("snapshot-password.");

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
        let Some(state) = self.settings.take_snapshot_password_prompt() else {
            return;
        };
        self.forget_text_inputs("snapshot-password.");
        self.shell.set_status(match state.kind {
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
        });
        self.settings.set_store_message("config picker cancelled");
        cx.notify();
    }

    pub(in crate::features) fn handle_snapshot_password_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        if !self.settings.snapshot_password_prompt_active() {
            return;
        }
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "enter" => self.submit_snapshot_password_prompt(cx),
            "escape" => self.cancel_snapshot_password_prompt(cx),
            _ => {}
        }
    }

    pub(in crate::features) fn apply_snapshot_password_input(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        if !self.settings.apply_snapshot_password_input(text) {
            return;
        }
        self.mark_user_activity();
        cx.notify();
    }

    fn prompt_encrypted_portable_snapshot_export_path(
        &mut self,
        master_password: String,
        cx: &mut Context<Self>,
    ) {
        if !self
            .settings
            .begin_config_path_prompt(ConfigPathPromptKind::EncryptedPortableExport)
        {
            self.shell
                .set_status("config path picker is already open".to_string());
            cx.notify();
            return;
        }
        let directory = self.runtime.config_dir().to_path_buf();
        let receiver = cx.prompt_for_new_path(&directory, Some("nyaterm-encrypted.nya"));
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.shell
            .set_status("selecting encrypted portable snapshot destination".to_string());
        self.settings
            .set_store_message("selecting encrypted .nya export destination");
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
        if !self
            .settings
            .begin_config_path_prompt(ConfigPathPromptKind::EncryptedPortableImport)
        {
            self.shell
                .set_status("config path picker is already open".to_string());
            cx.notify();
            return;
        }
        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from("Select encrypted .nya snapshot")),
        };
        let receiver = cx.prompt_for_paths(options);
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.shell
            .set_status("selecting encrypted portable snapshot to import".to_string());
        self.settings
            .set_store_message("selecting encrypted .nya snapshot");
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
        if !self.settings.finish_config_path_prompt(kind) {
            return;
        }
        match result {
            ConfigPathPromptResult::Exported(info) => {
                let message = match kind {
                    ConfigPathPromptKind::PortableExport => {
                        format!("exported {} byte .nya snapshot", info.bytes)
                    }
                    ConfigPathPromptKind::EncryptedPortableExport => {
                        format!("exported {} byte encrypted .nya snapshot", info.bytes)
                    }
                    _ => format!("exported {} byte config backup", info.bytes),
                };
                self.settings.replace_store_status(
                    info.database_path.display().to_string(),
                    message,
                    true,
                );
                self.shell.set_status(match kind {
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
                });
            }
            ConfigPathPromptResult::Imported(info) => {
                self.refresh_store_from_runtime();
                self.rebase_open_settings_draft();
                let safety = info
                    .safety_backup_path
                    .as_ref()
                    .map(|path| format!("; previous db saved to {}", path.display()))
                    .unwrap_or_default();
                let message = match kind {
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
                self.settings.update_store_status(message, true);
                self.shell.set_status(match kind {
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
                });
            }
            ConfigPathPromptResult::Cancelled => {
                self.shell.set_status(match kind {
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
                });
                self.settings.set_store_message("config picker cancelled");
            }
            ConfigPathPromptResult::Failed(error) => {
                self.shell.set_status(match kind {
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
                });
                self.settings
                    .update_store_status(self.shell.status().to_string(), false);
            }
            ConfigPathPromptResult::Closed => {
                self.shell
                    .set_status("config path picker closed before returning".to_string());
                self.settings.set_store_message("config picker closed");
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
                        self.connection_state
                            .replace_loaded(config.connections, config.groups);
                        self.security.replace_catalog(
                            store.list_ssh_keys().unwrap_or_default(),
                            store.list_otp_entries().unwrap_or_default(),
                            store.list_passwords().unwrap_or_default(),
                            store.list_credentials().unwrap_or_default(),
                        );
                        self.tunnel_state.replace_loaded_catalog(
                            store.list_tunnels().unwrap_or_default(),
                            store.list_tunnel_groups().unwrap_or_default(),
                            store.list_proxies().unwrap_or_default(),
                            store.list_proxy_groups().unwrap_or_default(),
                        );
                        let quick_commands = store.load_quick_commands().unwrap_or_default();
                        self.commands.replace_loaded(
                            quick_commands.commands,
                            quick_commands.categories,
                            store.list_command_history(64).unwrap_or_default(),
                        );
                        self.settings.replace_keyword_config(
                            store.load_keyword_highlights().unwrap_or_default(),
                        );
                        self.apply_gpui_settings(
                            store.load_app_settings_summary().unwrap_or_default(),
                        );
                        self.apply_ui_layout_from_settings();
                        let translation_settings = store
                            .load_translation_settings()
                            .unwrap_or_else(|_| TranslationSettings {
                                target_language: self.settings.summary().language.clone(),
                                ..TranslationSettings::default()
                            });
                        self.translation.replace_settings(
                            translation_settings,
                            TranslationSecretDraft::default(),
                        );
                        self.recording.set_memory_limit(
                            self.settings.summary().recording_memory_limit_bytes as usize,
                        );
                        let cloud_sync_settings = store
                            .load_cloud_sync_settings()
                            .unwrap_or_else(|_| self.cloud_sync.settings().clone());
                        let ai_settings = store
                            .load_ai_settings()
                            .unwrap_or_else(|_| self.ai.settings_config().clone());
                        self.ai.replace_settings_config(ai_settings, true);
                        self.sync_ai_drafts_from_active_profile();
                        self.settings.rebase_master_password();
                        let cloud_sync_state = store
                            .load_cloud_sync_state()
                            .unwrap_or_else(|_| self.cloud_sync.state().clone());
                        self.cloud_sync
                            .replace_loaded(cloud_sync_settings, cloud_sync_state);
                        self.transfer
                            .set_duplicate_policy(SftpDuplicatePolicy::from_legacy_value(
                                &self.settings.summary().transfer_duplicate_strategy,
                            ));
                        self.settings.replace_store_status(
                            path,
                            "redb connection store online".to_string(),
                            true,
                        );
                    }
                    Err(error) => {
                        self.connection_state.clear_loaded();
                        self.security.clear_catalog();
                        self.tunnel_state.clear_catalog();
                        self.commands.clear_loaded();
                        self.settings
                            .replace_keyword_config(KeywordHighlightConfig::default());
                        self.apply_gpui_settings(AppSettingsSummary::default());
                        self.translation.replace_settings(
                            TranslationSettings::default(),
                            TranslationSecretDraft::default(),
                        );
                        self.settings.replace_store_status(
                            path,
                            format!("failed to load sessions: {error}"),
                            false,
                        );
                    }
                }
            }
            Err(error) => {
                self.connection_state.clear_connections();
                self.tunnel_state.clear_catalog();
                self.commands.clear_loaded();
                self.apply_gpui_settings(AppSettingsSummary::default());
                self.translation.replace_settings(
                    TranslationSettings::default(),
                    TranslationSecretDraft::default(),
                );
                self.settings.replace_store_status(
                    self.runtime
                        .config_dir()
                        .join("nyaterm.redb")
                        .display()
                        .to_string(),
                    format!("failed to open store: {error}"),
                    false,
                );
            }
        }
    }
}
