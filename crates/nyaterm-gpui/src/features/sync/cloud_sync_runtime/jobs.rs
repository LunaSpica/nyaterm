use super::*;

impl NyaTermApp {
    pub(in crate::features) fn run_local_cloud_sync_push(
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

    pub(in crate::features) fn run_local_cloud_sync_pull(
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

    pub(in crate::features) fn run_provider_cloud_sync_push(
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

    pub(in crate::features) fn run_provider_cloud_sync_pull(
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

    pub(in crate::features) fn local_cloud_sync_options(&self, master_password: String) -> LocalCloudSyncOptions {
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

    pub(in crate::features) fn record_cloud_sync_history(&mut self, entry: &CloudSyncHistoryEntry) {
        if let Err(error) = append_cloud_sync_history(self.runtime.log_dir(), entry) {
            self.cloud_sync_status = format!("{}; history log failed: {error}", entry.message);
        }
    }

    pub(in crate::features) fn refresh_cloud_sync_history(&mut self) {
        self.cloud_sync_history = read_cloud_sync_history(
            self.runtime.log_dir(),
            self.settings.diagnostics_retention_days,
            CLOUD_SYNC_HISTORY_LIMIT,
        )
        .unwrap_or_default();
    }

    pub(in crate::features) fn toggle_cloud_sync_history_details(
        &mut self,
        entry_id: &str,
        cx: &mut Context<Self>,
    ) {
        if self.cloud_sync_history_expanded.contains(entry_id) {
            self.cloud_sync_history_expanded.remove(entry_id);
        } else {
            self.cloud_sync_history_expanded
                .insert(entry_id.to_string());
        }
        cx.notify();
    }

}
