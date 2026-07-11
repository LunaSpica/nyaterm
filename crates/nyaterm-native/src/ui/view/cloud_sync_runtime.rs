use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn update_cloud_sync_provider(
        &mut self,
        provider: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.cloud_sync_settings.provider = provider.to_string();
        self.cloud_sync_status = format!("provider set to {provider}; save to persist");
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_cloud_sync_enabled(&mut self, cx: &mut Context<Self>) {
        self.cloud_sync_settings.enabled = !self.cloud_sync_settings.enabled;
        self.cloud_sync_status = if self.cloud_sync_settings.enabled {
            "cloud sync enabled; save to persist"
        } else {
            "cloud sync disabled; save to persist"
        }
        .to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_s3_virtual_host_style(&mut self, cx: &mut Context<Self>) {
        self.cloud_sync_settings.s3.virtual_host_style =
            !self.cloud_sync_settings.s3.virtual_host_style;
        self.cloud_sync_status = if self.cloud_sync_settings.s3.virtual_host_style {
            "S3 virtual-host style enabled; save to persist"
        } else {
            "S3 path-style URLs enabled; save to persist"
        }
        .to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn save_cloud_sync_settings(&mut self, cx: &mut Context<Self>) {
        let mut next = self.cloud_sync_settings.clone();
        if !self.cloud_sync_secret_draft.webdav_password.is_empty() {
            next.webdav.password = Some(self.cloud_sync_secret_draft.webdav_password.clone());
        }
        if !self.cloud_sync_secret_draft.s3_access_key_id.is_empty() {
            next.s3.access_key_id = Some(self.cloud_sync_secret_draft.s3_access_key_id.clone());
        }
        if !self.cloud_sync_secret_draft.s3_secret_access_key.is_empty() {
            next.s3.secret_access_key =
                Some(self.cloud_sync_secret_draft.s3_secret_access_key.clone());
        }
        if !self.cloud_sync_secret_draft.s3_session_token.is_empty() {
            next.s3.session_token = Some(self.cloud_sync_secret_draft.s3_session_token.clone());
        }
        if !self
            .cloud_sync_secret_draft
            .google_drive_access_token
            .is_empty()
        {
            next.google_drive.access_token = Some(
                self.cloud_sync_secret_draft
                    .google_drive_access_token
                    .clone(),
            );
        }
        if !self
            .cloud_sync_secret_draft
            .google_drive_refresh_token
            .is_empty()
        {
            next.google_drive.refresh_token = Some(
                self.cloud_sync_secret_draft
                    .google_drive_refresh_token
                    .clone(),
            );
        }
        if !self
            .cloud_sync_secret_draft
            .google_drive_client_secret
            .is_empty()
        {
            next.google_drive.client_secret = Some(
                self.cloud_sync_secret_draft
                    .google_drive_client_secret
                    .clone(),
            );
        }
        if !self
            .cloud_sync_secret_draft
            .onedrive_access_token
            .is_empty()
        {
            next.onedrive.access_token =
                Some(self.cloud_sync_secret_draft.onedrive_access_token.clone());
        }
        if !self
            .cloud_sync_secret_draft
            .onedrive_refresh_token
            .is_empty()
        {
            next.onedrive.refresh_token =
                Some(self.cloud_sync_secret_draft.onedrive_refresh_token.clone());
        }
        if !self
            .cloud_sync_secret_draft
            .onedrive_client_secret
            .is_empty()
        {
            next.onedrive.client_secret =
                Some(self.cloud_sync_secret_draft.onedrive_client_secret.clone());
        }
        if !self
            .cloud_sync_secret_draft
            .aliyun_drive_access_token
            .is_empty()
        {
            next.aliyun_drive.access_token = Some(
                self.cloud_sync_secret_draft
                    .aliyun_drive_access_token
                    .clone(),
            );
        }
        if !self
            .cloud_sync_secret_draft
            .aliyun_drive_refresh_token
            .is_empty()
        {
            next.aliyun_drive.refresh_token = Some(
                self.cloud_sync_secret_draft
                    .aliyun_drive_refresh_token
                    .clone(),
            );
        }
        if !self
            .cloud_sync_secret_draft
            .aliyun_drive_client_secret
            .is_empty()
        {
            next.aliyun_drive.client_secret = Some(
                self.cloud_sync_secret_draft
                    .aliyun_drive_client_secret
                    .clone(),
            );
        }
        if !self.cloud_sync_secret_draft.gitee_token.is_empty() {
            next.gitee_snippet.access_token =
                Some(self.cloud_sync_secret_draft.gitee_token.clone());
        }
        if !self.cloud_sync_secret_draft.github_token.is_empty() {
            next.github_gist.access_token = Some(self.cloud_sync_secret_draft.github_token.clone());
        }

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_cloud_sync_settings(next))
        {
            Ok(saved) => {
                self.cloud_sync_settings = saved;
                self.cloud_sync_secret_draft = CloudSyncSecretDraft::default();
                self.cloud_sync_status = "cloud sync settings saved".to_string();
                self.store_status.message = "cloud sync settings saved".to_string();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.cloud_sync_status = format!("cloud sync settings save failed: {error}");
                self.store_status.message = self.cloud_sync_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_cloud_sync_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.cloud_sync_input_value_mut().pop();
                self.cloud_sync_status = "cloud sync settings edited".to_string();
                cx.notify();
            }
            "escape" => {
                self.cloud_sync_status = "cloud sync input blurred".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.cloud_sync_input_value_mut().push_str(input);
                    self.cloud_sync_status = "cloud sync settings edited".to_string();
                    cx.notify();
                }
            }
        }
    }

    fn cloud_sync_input_value_mut(&mut self) -> &mut String {
        match self.cloud_sync_focused_field {
            CloudSyncInputField::RemoteRoot => &mut self.cloud_sync_settings.remote_root,
            CloudSyncInputField::WebdavEndpoint => &mut self.cloud_sync_settings.webdav.endpoint,
            CloudSyncInputField::WebdavRoot => &mut self.cloud_sync_settings.webdav.root,
            CloudSyncInputField::WebdavUsername => &mut self.cloud_sync_settings.webdav.username,
            CloudSyncInputField::WebdavPassword => {
                &mut self.cloud_sync_secret_draft.webdav_password
            }
            CloudSyncInputField::S3Endpoint => &mut self.cloud_sync_settings.s3.endpoint,
            CloudSyncInputField::S3Bucket => &mut self.cloud_sync_settings.s3.bucket,
            CloudSyncInputField::S3Region => &mut self.cloud_sync_settings.s3.region,
            CloudSyncInputField::S3Root => &mut self.cloud_sync_settings.s3.root,
            CloudSyncInputField::S3AccessKeyId => {
                &mut self.cloud_sync_secret_draft.s3_access_key_id
            }
            CloudSyncInputField::S3SecretAccessKey => {
                &mut self.cloud_sync_secret_draft.s3_secret_access_key
            }
            CloudSyncInputField::S3SessionToken => {
                &mut self.cloud_sync_secret_draft.s3_session_token
            }
            CloudSyncInputField::GoogleDriveRoot => &mut self.cloud_sync_settings.google_drive.root,
            CloudSyncInputField::GoogleDriveAccessToken => {
                &mut self.cloud_sync_secret_draft.google_drive_access_token
            }
            CloudSyncInputField::GoogleDriveRefreshToken => {
                &mut self.cloud_sync_secret_draft.google_drive_refresh_token
            }
            CloudSyncInputField::GoogleDriveClientId => self
                .cloud_sync_settings
                .google_drive
                .client_id
                .get_or_insert_with(String::new),
            CloudSyncInputField::GoogleDriveClientSecret => {
                &mut self.cloud_sync_secret_draft.google_drive_client_secret
            }
            CloudSyncInputField::OneDriveRoot => &mut self.cloud_sync_settings.onedrive.root,
            CloudSyncInputField::OneDriveAccessToken => {
                &mut self.cloud_sync_secret_draft.onedrive_access_token
            }
            CloudSyncInputField::OneDriveRefreshToken => {
                &mut self.cloud_sync_secret_draft.onedrive_refresh_token
            }
            CloudSyncInputField::OneDriveClientId => self
                .cloud_sync_settings
                .onedrive
                .client_id
                .get_or_insert_with(String::new),
            CloudSyncInputField::OneDriveClientSecret => {
                &mut self.cloud_sync_secret_draft.onedrive_client_secret
            }
            CloudSyncInputField::AliyunDriveRoot => &mut self.cloud_sync_settings.aliyun_drive.root,
            CloudSyncInputField::AliyunDriveType => {
                &mut self.cloud_sync_settings.aliyun_drive.drive_type
            }
            CloudSyncInputField::AliyunDriveAccessToken => {
                &mut self.cloud_sync_secret_draft.aliyun_drive_access_token
            }
            CloudSyncInputField::AliyunDriveRefreshToken => {
                &mut self.cloud_sync_secret_draft.aliyun_drive_refresh_token
            }
            CloudSyncInputField::AliyunDriveClientId => self
                .cloud_sync_settings
                .aliyun_drive
                .client_id
                .get_or_insert_with(String::new),
            CloudSyncInputField::AliyunDriveClientSecret => {
                &mut self.cloud_sync_secret_draft.aliyun_drive_client_secret
            }
            CloudSyncInputField::GiteeEndpoint => {
                &mut self.cloud_sync_settings.gitee_snippet.api_endpoint
            }
            CloudSyncInputField::GiteeGistId => &mut self.cloud_sync_settings.gitee_snippet.gist_id,
            CloudSyncInputField::GiteeToken => &mut self.cloud_sync_secret_draft.gitee_token,
            CloudSyncInputField::GithubGistId => &mut self.cloud_sync_settings.github_gist.gist_id,
            CloudSyncInputField::GithubToken => &mut self.cloud_sync_secret_draft.github_token,
        }
    }
}

impl NyaTermApp {
    pub(in crate::ui::view) fn prompt_local_cloud_sync_push(&mut self, cx: &mut Context<Self>) {
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::CloudPush, cx);
    }

    pub(in crate::ui::view) fn prompt_local_cloud_sync_pull(&mut self, cx: &mut Context<Self>) {
        if self.active_session_id.is_some() || self.pending_session_name.is_some() {
            self.terminal_status = "close active session before pulling cloud sync".to_string();
            cx.notify();
            return;
        }
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::CloudPull, cx);
    }

    pub(in crate::ui::view) fn prompt_provider_cloud_sync_push(&mut self, cx: &mut Context<Self>) {
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::CloudProviderPush, cx);
    }

    pub(in crate::ui::view) fn prompt_provider_cloud_sync_pull(&mut self, cx: &mut Context<Self>) {
        if self.active_session_id.is_some() || self.pending_session_name.is_some() {
            self.terminal_status =
                "close active session before pulling provider cloud sync".to_string();
            cx.notify();
            return;
        }
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::CloudProviderPull, cx);
    }

    pub(in crate::ui::view) fn prompt_cloud_sync_force_push(
        &mut self,
        provider_action: bool,
        cx: &mut Context<Self>,
    ) {
        let kind = if provider_action {
            SnapshotPasswordPromptKind::CloudProviderForcePush
        } else {
            SnapshotPasswordPromptKind::CloudForcePush
        };
        self.start_snapshot_password_prompt(kind, cx);
    }

    pub(in crate::ui::view) fn prompt_cloud_sync_force_pull(
        &mut self,
        provider_action: bool,
        cx: &mut Context<Self>,
    ) {
        if self.active_session_id.is_some() || self.pending_session_name.is_some() {
            self.terminal_status = if provider_action {
                "close active session before force pulling provider cloud sync"
            } else {
                "close active session before force pulling cloud sync"
            }
            .to_string();
            cx.notify();
            return;
        }
        let kind = if provider_action {
            SnapshotPasswordPromptKind::CloudProviderForcePull
        } else {
            SnapshotPasswordPromptKind::CloudForcePull
        };
        self.start_snapshot_password_prompt(kind, cx);
    }

    pub(in crate::ui::view) fn dismiss_cloud_sync_conflict(&mut self, cx: &mut Context<Self>) {
        self.cloud_sync_conflict = None;
        self.cloud_sync_status = "cloud sync conflict dismissed".to_string();
        cx.notify();
    }

    fn capture_cloud_sync_conflict(
        &mut self,
        error: &CloudSyncError,
        provider: String,
        provider_action: bool,
    ) {
        if let CloudSyncError::Conflict(message) = error {
            self.cloud_sync_conflict = Some(CloudSyncConflictState {
                provider,
                message: message.clone(),
                provider_action,
            });
        }
    }

    pub(in crate::ui::view) fn run_local_cloud_sync_push(
        &mut self,
        master_password: String,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let options = self.local_cloud_sync_options(master_password);
        let state = self.cloud_sync_state.clone();
        let started_at = Instant::now();
        self.cloud_sync_status = if force {
            "force pushing local cloud sync snapshot".to_string()
        } else {
            "pushing local cloud sync snapshot".to_string()
        };
        self.terminal_status = "cloud sync push started".to_string();
        cx.spawn(async move |this, cx| {
            let result = push_local_snapshot(&options, &state, force);
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(result) => {
                        this.cloud_sync_conflict = None;
                        let mut history = CloudSyncHistoryEntry::sync(
                            "success",
                            if force {
                                "manual_force_push"
                            } else {
                                "manual_push"
                            },
                            Some(result.status.provider.clone()),
                            result
                                .pointer
                                .as_ref()
                                .map(|pointer| pointer.revision_id.clone()),
                            result.status.message.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                        this.cloud_sync_state = result.state;
                        this.cloud_sync_status = result.status.message.clone();
                        this.terminal_status = result.status.message;
                    }
                    Err(error) => {
                        let status = cloud_sync_history_status(&error);
                        this.cloud_sync_status = format!("push failed: {error}");
                        this.capture_cloud_sync_conflict(
                            &error,
                            "local_directory".to_string(),
                            false,
                        );
                        this.terminal_status = this.cloud_sync_status.clone();
                        let mut history = CloudSyncHistoryEntry::sync(
                            status,
                            if force {
                                "manual_force_push"
                            } else {
                                "manual_push"
                            },
                            Some("local_directory".to_string()),
                            None,
                            this.cloud_sync_status.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::ui::view) fn run_local_cloud_sync_pull(
        &mut self,
        master_password: String,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let options = self.local_cloud_sync_options(master_password);
        let state = self.cloud_sync_state.clone();
        let started_at = Instant::now();
        self.cloud_sync_status = if force {
            "force pulling local cloud sync snapshot".to_string()
        } else {
            "pulling local cloud sync snapshot".to_string()
        };
        self.terminal_status = "cloud sync pull started".to_string();
        cx.spawn(async move |this, cx| {
            let result = pull_local_snapshot(&options, &state, force);
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(result) => {
                        this.cloud_sync_conflict = None;
                        let mut history = CloudSyncHistoryEntry::sync(
                            "success",
                            if force {
                                "manual_force_pull"
                            } else {
                                "manual_pull"
                            },
                            Some(result.status.provider.clone()),
                            result
                                .pointer
                                .as_ref()
                                .map(|pointer| pointer.revision_id.clone()),
                            result.status.message.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                        this.cloud_sync_state = result.state;
                        this.cloud_sync_status = result.status.message.clone();
                        this.terminal_status = result.status.message;
                        this.refresh_store_from_runtime();
                    }
                    Err(error) => {
                        let status = cloud_sync_history_status(&error);
                        this.cloud_sync_status = format!("pull failed: {error}");
                        this.capture_cloud_sync_conflict(
                            &error,
                            "local_directory".to_string(),
                            false,
                        );
                        this.terminal_status = this.cloud_sync_status.clone();
                        let mut history = CloudSyncHistoryEntry::sync(
                            status,
                            if force {
                                "manual_force_pull"
                            } else {
                                "manual_pull"
                            },
                            Some("local_directory".to_string()),
                            None,
                            this.cloud_sync_status.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::ui::view) fn run_provider_cloud_sync_push(
        &mut self,
        master_password: String,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let options = self.local_cloud_sync_options(master_password);
        let state = self.cloud_sync_state.clone();
        let settings = self.cloud_sync_settings.clone();
        let provider = configured_cloud_sync_provider(&settings);
        let started_at = Instant::now();
        self.cloud_sync_status = if force {
            format!("force pushing provider cloud sync snapshot via {provider}")
        } else {
            format!("pushing provider cloud sync snapshot via {provider}")
        };
        self.terminal_status = "provider cloud sync push started".to_string();
        cx.spawn(async move |this, cx| {
            let result = push_provider_snapshot(&settings, &options, &state, force);
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(result) => {
                        this.cloud_sync_conflict = None;
                        let mut history = CloudSyncHistoryEntry::sync(
                            "success",
                            if force {
                                "manual_provider_force_push"
                            } else {
                                "manual_provider_push"
                            },
                            Some(result.status.provider.clone()),
                            result
                                .pointer
                                .as_ref()
                                .map(|pointer| pointer.revision_id.clone()),
                            result.status.message.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                        this.cloud_sync_state = result.state;
                        this.cloud_sync_status = result.status.message.clone();
                        this.terminal_status = result.status.message;
                    }
                    Err(error) => {
                        let status = cloud_sync_history_status(&error);
                        this.cloud_sync_status = format!("provider push failed: {error}");
                        this.capture_cloud_sync_conflict(
                            &error,
                            configured_cloud_sync_provider(&settings),
                            true,
                        );
                        this.terminal_status = this.cloud_sync_status.clone();
                        let mut history = CloudSyncHistoryEntry::sync(
                            status,
                            if force {
                                "manual_provider_force_push"
                            } else {
                                "manual_provider_push"
                            },
                            Some(configured_cloud_sync_provider(&settings)),
                            None,
                            this.cloud_sync_status.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::ui::view) fn run_provider_cloud_sync_pull(
        &mut self,
        master_password: String,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let options = self.local_cloud_sync_options(master_password);
        let state = self.cloud_sync_state.clone();
        let settings = self.cloud_sync_settings.clone();
        let provider = configured_cloud_sync_provider(&settings);
        let started_at = Instant::now();
        self.cloud_sync_status = if force {
            format!("force pulling provider cloud sync snapshot via {provider}")
        } else {
            format!("pulling provider cloud sync snapshot via {provider}")
        };
        self.terminal_status = "provider cloud sync pull started".to_string();
        cx.spawn(async move |this, cx| {
            let result = pull_provider_snapshot(&settings, &options, &state, force);
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(result) => {
                        this.cloud_sync_conflict = None;
                        let mut history = CloudSyncHistoryEntry::sync(
                            "success",
                            if force {
                                "manual_provider_force_pull"
                            } else {
                                "manual_provider_pull"
                            },
                            Some(result.status.provider.clone()),
                            result
                                .pointer
                                .as_ref()
                                .map(|pointer| pointer.revision_id.clone()),
                            result.status.message.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                        this.cloud_sync_state = result.state;
                        this.cloud_sync_status = result.status.message.clone();
                        this.terminal_status = result.status.message;
                        this.refresh_store_from_runtime();
                    }
                    Err(error) => {
                        let status = cloud_sync_history_status(&error);
                        this.cloud_sync_status = format!("provider pull failed: {error}");
                        this.capture_cloud_sync_conflict(
                            &error,
                            configured_cloud_sync_provider(&settings),
                            true,
                        );
                        this.terminal_status = this.cloud_sync_status.clone();
                        let mut history = CloudSyncHistoryEntry::sync(
                            status,
                            if force {
                                "manual_provider_force_pull"
                            } else {
                                "manual_provider_pull"
                            },
                            Some(configured_cloud_sync_provider(&settings)),
                            None,
                            this.cloud_sync_status.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn local_cloud_sync_options(&self, master_password: String) -> LocalCloudSyncOptions {
        LocalCloudSyncOptions {
            config_dir: self.runtime.config_dir().to_path_buf(),
            portable_key_path: self.runtime.portable_key_path().map(ToOwned::to_owned),
            remote_dir: self.runtime.config_dir().join("cloud-sync-local"),
            remote_root: self.cloud_sync_settings.remote_root.clone(),
            device_id: self.cloud_sync_state.device_id.clone(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            master_password,
            enabled: true,
        }
    }

    fn record_cloud_sync_history(&mut self, entry: &CloudSyncHistoryEntry) {
        if let Err(error) = append_cloud_sync_history(self.runtime.log_dir(), entry) {
            self.cloud_sync_status = format!("{}; history log failed: {error}", entry.message);
        }
    }

    pub(in crate::ui::view) fn refresh_cloud_sync_history(&mut self) {
        self.cloud_sync_history = read_cloud_sync_history(
            self.runtime.log_dir(),
            self.settings.diagnostics_retention_days,
            CLOUD_SYNC_HISTORY_LIMIT,
        )
        .unwrap_or_default();
    }

    pub(in crate::ui::view) fn toggle_cloud_sync_history_details(
        &mut self,
        entry_id: &str,
        cx: &mut Context<Self>,
    ) {
        if self.cloud_sync_history_expanded.contains(entry_id) {
            self.cloud_sync_history_expanded.remove(entry_id);
        } else {
            self.cloud_sync_history_expanded.insert(entry_id.to_string());
        }
        cx.notify();
    }
}
