use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn start_sftp_list_job(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_sftp_list_job_with_select_after(None, window, cx);
    }

    pub(in crate::ui::view) fn start_sftp_list_job_with_select_after(
        &mut self,
        select_after: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.selected_nav = NavItem::Transfers;
            cx.notify();
            return;
        };
        self.ensure_event_pump(window, cx);
        let remote_path = self.normalized_transfer_remote_path();
        self.transfer_browser_path = remote_path.clone();
        self.transfer_browser_status = format!("Listing {remote_path}...");
        self.transfer_selected_remote_path = None;
        self.transfer_selected_remote_paths.clear();
        let id = self.next_transfer_id("sftp-list");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::ListDir {
                remote_path: remote_path.clone(),
                select_after,
            },
            status: TransferJobStatus::Running,
            detail: format!("Listing {remote_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.terminal_status = format!("SFTP list started for {remote_path}");
        let transfer_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let result = SftpService::new(config)
                .list_dir(&remote_path)
                .map(TransferJobOutput::Entries)
                .map_err(|error| error.to_string());
            let _ = transfer_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn start_transfer_sync_cwd_job(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.transfer_sync_cwd_job_running() {
            self.transfer_browser_status = "remote cwd sync already running".to_string();
            cx.notify();
            return;
        }
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.selected_nav = NavItem::Transfers;
            cx.notify();
            return;
        };
        self.ensure_event_pump(window, cx);
        self.transfer_auto_sync_cwd_last_at = Some(Instant::now());
        let id = self.next_transfer_id("sftp-sync-cwd");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::SyncCwd,
            status: TransferJobStatus::Running,
            detail: "Resolving remote cwd".to_string(),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.transfer_browser_status = "Resolving remote cwd...".to_string();
        self.terminal_status = "SFTP cwd sync started".to_string();
        let transfer_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let output = SshProcessService::new(config.clone())
                    .run_command("pwd -P", Duration::from_secs(10))
                    .map_err(|error| error.to_string())?;
                if output.exit_status.is_some_and(|status| status != 0) {
                    let detail = output
                        .stderr
                        .trim()
                        .lines()
                        .next()
                        .or_else(|| output.stdout.trim().lines().next())
                        .unwrap_or("remote pwd failed");
                    return Err(detail.to_string());
                }
                let remote_path = output
                    .stdout
                    .lines()
                    .map(str::trim)
                    .find(|line| line.starts_with('/'))
                    .ok_or_else(|| "remote pwd did not return an absolute path".to_string())?
                    .to_string();
                let entries = SftpService::new(config)
                    .list_dir(&remote_path)
                    .map_err(|error| error.to_string())?;
                Ok(TransferJobOutput::CwdSynced {
                    remote_path,
                    entries,
                })
            })();
            let _ = transfer_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn transfer_sync_cwd_job_running(&self) -> bool {
        self.transfer_jobs.iter().any(|job| {
            job.kind == TransferJobKind::SyncCwd
                && matches!(
                    job.status,
                    TransferJobStatus::Running
                        | TransferJobStatus::Paused
                        | TransferJobStatus::Cancelling
                )
        })
    }

    pub(in crate::ui::view) fn start_transfer_browser_home_dir_job(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.transfer_browser_home_dir_pending || !self.transfer_browser_home_dir.is_empty() {
            return;
        }
        let Some(config) = self.active_ssh_config.clone() else {
            self.transfer_browser_status = "remote home requires an SSH session".to_string();
            cx.notify();
            return;
        };
        self.transfer_browser_home_dir_pending = true;
        let id = self.next_transfer_id("sftp-home");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::ResolveHome,
            status: TransferJobStatus::Running,
            detail: "Resolving remote home".to_string(),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.transfer_browser_status = "Resolving remote home...".to_string();
        let transfer_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let output = SshProcessService::new(config)
                    .run_command("printf '%s\\n' \"$HOME\"", Duration::from_secs(10))
                    .map_err(|error| error.to_string())?;
                if output.exit_status.is_some_and(|status| status != 0) {
                    let detail = output
                        .stderr
                        .trim()
                        .lines()
                        .next()
                        .or_else(|| output.stdout.trim().lines().next())
                        .unwrap_or("remote home lookup failed");
                    return Err(detail.to_string());
                }
                let home_dir = output
                    .stdout
                    .lines()
                    .map(str::trim)
                    .find(|line| line.starts_with('/'))
                    .ok_or_else(|| "remote home did not return an absolute path".to_string())?
                    .trim_end_matches('/')
                    .to_string();
                Ok(TransferJobOutput::HomeDir(home_dir))
            })();
            let _ = transfer_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn start_sftp_download_job(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let remote_path = self.normalized_transfer_remote_path();
        if self.settings.transfer_ask_save_location {
            self.prompt_transfer_download_directory_and_start(vec![remote_path], window, cx);
            return;
        }
        let local_path = self.normalized_transfer_local_path();
        self.start_sftp_download_job_for_target(remote_path, local_path, window, cx);
    }

    pub(in crate::ui::view) fn start_sftp_download_job_for_target(
        &mut self,
        remote_path: String,
        local_path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.selected_nav = NavItem::Transfers;
            cx.notify();
            return;
        };
        let duplicate_policy = self.transfer_duplicate_policy;
        let duplicate_resolver = (duplicate_policy == SftpDuplicatePolicy::Ask)
            .then(|| self.duplicate_prompts.clone() as Arc<dyn SftpDuplicateResolver>);
        let transfer_options = self.sftp_transfer_options();
        self.ensure_event_pump(window, cx);
        self.enqueue_sftp_download_job_for_target(
            config,
            remote_path,
            local_path,
            duplicate_policy,
            duplicate_resolver,
            transfer_options,
            cx,
        );
    }

    pub(in crate::ui::view) fn enqueue_sftp_download_job_for_target(
        &mut self,
        config: SshSessionConfig,
        remote_path: String,
        local_path: PathBuf,
        duplicate_policy: SftpDuplicatePolicy,
        duplicate_resolver: Option<Arc<dyn SftpDuplicateResolver>>,
        transfer_options: SftpTransferOptions,
        cx: &mut Context<Self>,
    ) {
        let id = self.next_transfer_id("sftp-download");
        let control = SftpTransferControl::new();
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::Download {
                remote_path: remote_path.clone(),
                local_path: local_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Downloading {remote_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: Some(control.clone()),
        });
        self.terminal_status = format!("SFTP download started for {remote_path}");
        let progress_tx = self.transfer_tx.clone();
        let finished_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let progress_id = id.clone();
            let result = SftpService::new(config)
                .download_path_with_progress_options_and_resolver_options(
                    &remote_path,
                    local_path,
                    control,
                    duplicate_policy,
                    duplicate_resolver,
                    transfer_options,
                    move |progress| {
                        let _ = progress_tx.send(TransferJobResult {
                            id: progress_id.clone(),
                            event: TransferJobEvent::Progress(progress),
                        });
                    },
                )
                .map(TransferJobOutput::Summary)
                .map_err(|error| error.to_string());
            let _ = finished_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn start_sftp_upload_job(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let local_path = self.normalized_transfer_local_path();
        let remote_path = self.normalized_transfer_remote_path();
        self.ensure_event_pump(window, cx);
        self.start_sftp_upload_job_for_target(local_path, remote_path, cx);
    }

    pub(in crate::ui::view) fn start_sftp_upload_job_for_target(
        &mut self,
        local_path: PathBuf,
        remote_path: String,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.selected_nav = NavItem::Transfers;
            cx.notify();
            return;
        };
        let duplicate_policy = self.transfer_duplicate_policy;
        let duplicate_resolver = (duplicate_policy == SftpDuplicatePolicy::Ask)
            .then(|| self.duplicate_prompts.clone() as Arc<dyn SftpDuplicateResolver>);
        let transfer_options = self.sftp_transfer_options();
        let id = self.next_transfer_id("sftp-upload");
        let control = SftpTransferControl::new();
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::Upload {
                local_path: local_path.clone(),
                remote_path: remote_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Uploading {}", local_path.display()),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: Some(control.clone()),
        });
        self.terminal_status = format!("SFTP upload started for {}", local_path.display());
        let progress_tx = self.transfer_tx.clone();
        let finished_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let progress_id = id.clone();
            let service = SftpService::new(config);
            let result = service
                .upload_path_with_progress_options_and_resolver_options(
                    local_path,
                    &remote_path,
                    control,
                    duplicate_policy,
                    duplicate_resolver,
                    transfer_options,
                    move |progress| {
                        let _ = progress_tx.send(TransferJobResult {
                            id: progress_id.clone(),
                            event: TransferJobEvent::Progress(progress),
                        });
                    },
                )
                .map(|summary| {
                    if summary.skipped {
                        return TransferJobOutput::Summary(summary);
                    }
                    let parent_path = transfer_job_remote_parent_path(&summary.remote_path);
                    match service.list_dir(&parent_path) {
                        Ok(entries) => TransferJobOutput::Uploaded {
                            summary,
                            parent_path,
                            entries,
                        },
                        Err(_) => TransferJobOutput::Summary(summary),
                    }
                })
                .map_err(|error| error.to_string());
            let _ = finished_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn cancel_transfer_job(
        &mut self,
        job_id: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(job) = self
            .transfer_jobs
            .iter_mut()
            .find(|candidate| candidate.id == job_id)
        else {
            self.terminal_status = "transfer job not found".to_string();
            cx.notify();
            return;
        };

        if !matches!(
            job.status,
            TransferJobStatus::Running | TransferJobStatus::Paused
        ) {
            self.terminal_status = format!("transfer {} is not running", job.id);
            cx.notify();
            return;
        }

        let Some(control) = job.control.as_ref() else {
            self.terminal_status = format!("transfer {} cannot be cancelled", job.id);
            cx.notify();
            return;
        };

        control.cancel();
        job.status = TransferJobStatus::Cancelling;
        job.detail = "Cancelling".to_string();
        self.terminal_status = format!("SFTP transfer cancelling: {}", job.id);
        cx.notify();
    }

    pub(in crate::ui::view) fn pause_transfer_job(&mut self, job_id: &str, cx: &mut Context<Self>) {
        let Some(job) = self
            .transfer_jobs
            .iter_mut()
            .find(|candidate| candidate.id == job_id)
        else {
            self.terminal_status = "transfer job not found".to_string();
            cx.notify();
            return;
        };

        if job.status != TransferJobStatus::Running {
            self.terminal_status = format!("transfer {} is not running", job.id);
            cx.notify();
            return;
        }

        let Some(control) = job.control.as_ref() else {
            self.terminal_status = format!("transfer {} cannot be paused", job.id);
            cx.notify();
            return;
        };

        control.pause();
        job.status = TransferJobStatus::Paused;
        job.detail = "Paused".to_string();
        self.terminal_status = format!("SFTP transfer paused: {}", job.id);
        cx.notify();
    }

    pub(in crate::ui::view) fn resume_transfer_job(
        &mut self,
        job_id: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(job) = self
            .transfer_jobs
            .iter_mut()
            .find(|candidate| candidate.id == job_id)
        else {
            self.terminal_status = "transfer job not found".to_string();
            cx.notify();
            return;
        };

        if job.status != TransferJobStatus::Paused {
            self.terminal_status = format!("transfer {} is not paused", job.id);
            cx.notify();
            return;
        }

        let Some(control) = job.control.as_ref() else {
            self.terminal_status = format!("transfer {} cannot be resumed", job.id);
            cx.notify();
            return;
        };

        control.resume();
        job.status = TransferJobStatus::Running;
        job.detail = "Resuming".to_string();
        self.terminal_status = format!("SFTP transfer resumed: {}", job.id);
        cx.notify();
    }

    pub(in crate::ui::view) fn retry_transfer_job(
        &mut self,
        job_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.selected_nav = NavItem::Transfers;
            cx.notify();
            return;
        };
        let Some(index) = self
            .transfer_jobs
            .iter()
            .position(|candidate| candidate.id == job_id)
        else {
            self.terminal_status = "transfer job not found".to_string();
            cx.notify();
            return;
        };
        let kind = self.transfer_jobs[index].kind.clone();
        if !matches!(
            self.transfer_jobs[index].status,
            TransferJobStatus::Failed | TransferJobStatus::Cancelled
        ) {
            self.terminal_status = format!("transfer {job_id} is not retryable");
            cx.notify();
            return;
        }

        match kind {
            TransferJobKind::Download {
                remote_path,
                local_path,
            } => {
                self.ensure_event_pump(window, cx);
                let duplicate_policy = self.transfer_duplicate_policy;
                let duplicate_resolver = (duplicate_policy == SftpDuplicatePolicy::Ask)
                    .then(|| self.duplicate_prompts.clone() as Arc<dyn SftpDuplicateResolver>);
                let transfer_options = self.sftp_transfer_options();
                let control = SftpTransferControl::new();
                let job = &mut self.transfer_jobs[index];
                job.status = TransferJobStatus::Running;
                job.detail = format!("Retrying download {remote_path}");
                job.entries.clear();
                job.summary = None;
                job.progress = None;
                job.control = Some(control.clone());
                self.terminal_status = format!("retrying SFTP download for {remote_path}");
                let progress_tx = self.transfer_tx.clone();
                let finished_tx = self.transfer_tx.clone();
                std::thread::spawn(move || {
                    let progress_id = job_id.clone();
                    let result = SftpService::new(config)
                        .download_path_with_progress_options_and_resolver_options(
                            &remote_path,
                            local_path,
                            control,
                            duplicate_policy,
                            duplicate_resolver,
                            transfer_options,
                            move |progress| {
                                let _ = progress_tx.send(TransferJobResult {
                                    id: progress_id.clone(),
                                    event: TransferJobEvent::Progress(progress),
                                });
                            },
                        )
                        .map(TransferJobOutput::Summary)
                        .map_err(|error| error.to_string());
                    let _ = finished_tx.send(TransferJobResult {
                        id: job_id,
                        event: TransferJobEvent::Finished(result),
                    });
                });
            }
            TransferJobKind::Upload {
                local_path,
                remote_path,
            } => {
                self.ensure_event_pump(window, cx);
                let duplicate_policy = self.transfer_duplicate_policy;
                let duplicate_resolver = (duplicate_policy == SftpDuplicatePolicy::Ask)
                    .then(|| self.duplicate_prompts.clone() as Arc<dyn SftpDuplicateResolver>);
                let transfer_options = self.sftp_transfer_options();
                let control = SftpTransferControl::new();
                let job = &mut self.transfer_jobs[index];
                job.status = TransferJobStatus::Running;
                job.detail = format!("Retrying upload {}", local_path.display());
                job.entries.clear();
                job.summary = None;
                job.progress = None;
                job.control = Some(control.clone());
                self.terminal_status = format!("retrying SFTP upload for {}", local_path.display());
                let progress_tx = self.transfer_tx.clone();
                let finished_tx = self.transfer_tx.clone();
                std::thread::spawn(move || {
                    let progress_id = job_id.clone();
                    let service = SftpService::new(config);
                    let result = service
                        .upload_path_with_progress_options_and_resolver_options(
                            local_path,
                            &remote_path,
                            control,
                            duplicate_policy,
                            duplicate_resolver,
                            transfer_options,
                            move |progress| {
                                let _ = progress_tx.send(TransferJobResult {
                                    id: progress_id.clone(),
                                    event: TransferJobEvent::Progress(progress),
                                });
                            },
                        )
                        .map(|summary| {
                            if summary.skipped {
                                return TransferJobOutput::Summary(summary);
                            }
                            let parent_path = transfer_job_remote_parent_path(&summary.remote_path);
                            match service.list_dir(&parent_path) {
                                Ok(entries) => TransferJobOutput::Uploaded {
                                    summary,
                                    parent_path,
                                    entries,
                                },
                                Err(_) => TransferJobOutput::Summary(summary),
                            }
                        })
                        .map_err(|error| error.to_string());
                    let _ = finished_tx.send(TransferJobResult {
                        id: job_id,
                        event: TransferJobEvent::Finished(result),
                    });
                });
            }
            _ => {
                self.terminal_status =
                    format!("transfer {job_id} does not support native retry yet");
                cx.notify();
                return;
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn pause_all_transfer_jobs(&mut self, cx: &mut Context<Self>) {
        let mut changed = 0;
        for job in &mut self.transfer_jobs {
            if job.status == TransferJobStatus::Running
                && let Some(control) = job.control.as_ref()
            {
                control.pause();
                job.status = TransferJobStatus::Paused;
                job.detail = "Paused".to_string();
                changed += 1;
            }
        }
        self.terminal_status = if changed == 0 {
            "no running transfer jobs to pause".to_string()
        } else {
            format!("paused {changed} transfer job(s)")
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn resume_all_transfer_jobs(&mut self, cx: &mut Context<Self>) {
        let mut changed = 0;
        for job in &mut self.transfer_jobs {
            if job.status == TransferJobStatus::Paused
                && let Some(control) = job.control.as_ref()
            {
                control.resume();
                job.status = TransferJobStatus::Running;
                job.detail = "Resuming".to_string();
                changed += 1;
            }
        }
        self.terminal_status = if changed == 0 {
            "no paused transfer jobs to resume".to_string()
        } else {
            format!("resumed {changed} transfer job(s)")
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn cancel_all_transfer_jobs(&mut self, cx: &mut Context<Self>) {
        let mut changed = 0;
        for job in &mut self.transfer_jobs {
            if matches!(
                job.status,
                TransferJobStatus::Running | TransferJobStatus::Paused
            ) && let Some(control) = job.control.as_ref()
            {
                control.cancel();
                job.status = TransferJobStatus::Cancelling;
                job.detail = "Cancelling".to_string();
                changed += 1;
            }
        }
        self.terminal_status = if changed == 0 {
            "no active transfer jobs to cancel".to_string()
        } else {
            format!("cancelling {changed} transfer job(s)")
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn clear_completed_transfer_jobs(&mut self, cx: &mut Context<Self>) {
        let before = self.transfer_jobs.len();
        self.transfer_jobs
            .retain(|job| job.status != TransferJobStatus::Completed);
        let removed = before.saturating_sub(self.transfer_jobs.len());
        self.terminal_status = if removed == 0 {
            "no completed transfer jobs to clear".to_string()
        } else {
            format!("cleared {removed} completed transfer job(s)")
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn clear_stopped_transfer_jobs(&mut self, cx: &mut Context<Self>) {
        let before = self.transfer_jobs.len();
        self.transfer_jobs.retain(|job| {
            matches!(
                job.status,
                TransferJobStatus::Running
                    | TransferJobStatus::Paused
                    | TransferJobStatus::Cancelling
            )
        });
        let removed = before.saturating_sub(self.transfer_jobs.len());
        self.terminal_status = if removed == 0 {
            "no stopped transfer jobs to clear".to_string()
        } else {
            format!("cleared {removed} stopped transfer job(s)")
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn select_transfer_job(
        &mut self,
        job_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.transfer_jobs.iter().any(|job| job.id == job_id) {
            self.transfer_selected_job_id = Some(job_id.clone());
            self.terminal_status = format!("selected transfer {job_id}");
        } else {
            self.terminal_status = "transfer job not found".to_string();
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn request_delete_transfer_job(
        &mut self,
        job_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(job) = self.transfer_jobs.iter().find(|job| job.id == job_id) else {
            self.terminal_status = "transfer job not found".to_string();
            cx.notify();
            return;
        };
        if !self.can_delete_transfer_job(&job_id) {
            self.terminal_status = format!("transfer {} cannot be deleted yet", job.id);
            cx.notify();
            return;
        }
        self.transfer_selected_job_id = Some(job.id.clone());
        self.transfer_job_delete = Some(TransferJobDeleteState {
            job_id: job.id.clone(),
            title: transfer_job_title(&job.kind),
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn request_delete_selected_transfer_job(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let job_id = self
            .transfer_selected_job_id
            .clone()
            .or_else(|| self.transfer_jobs.last().map(|job| job.id.clone()));
        let Some(job_id) = job_id else {
            self.terminal_status = "transfer queue is empty".to_string();
            cx.notify();
            return;
        };
        self.request_delete_transfer_job(job_id, cx);
    }

    pub(in crate::ui::view) fn confirm_delete_transfer_job(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.transfer_job_delete.take() else {
            cx.notify();
            return;
        };
        let before = self.transfer_jobs.len();
        self.transfer_jobs.retain(|job| job.id != state.job_id);
        if self.transfer_selected_job_id.as_deref() == Some(state.job_id.as_str()) {
            self.transfer_selected_job_id = None;
        }
        self.terminal_status = if self.transfer_jobs.len() < before {
            format!("deleted transfer {}", state.job_id)
        } else {
            "transfer job not found".to_string()
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn cancel_delete_transfer_job(&mut self, cx: &mut Context<Self>) {
        self.transfer_job_delete = None;
        self.terminal_status = "transfer delete cancelled".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn reveal_transfer_job_target_directory(
        &mut self,
        job_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(job) = self.transfer_jobs.iter().find(|job| job.id == job_id) else {
            self.terminal_status = "transfer job not found".to_string();
            cx.notify();
            return;
        };
        let Some(target_path) = transfer_job_local_target_path(job) else {
            self.terminal_status = format!("transfer {} has no local target", job.id);
            cx.notify();
            return;
        };
        let target_dir = transfer_job_reveal_dir(target_path);
        cx.reveal_path(&target_dir);
        self.terminal_status = format!("opened transfer directory {}", target_dir.display());
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_transfer_queue_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
        let unmodified = !keystroke.modifiers.alt
            && !keystroke.modifiers.control
            && !keystroke.modifiers.platform
            && !keystroke.modifiers.shift;
        if unmodified && keystroke.key == "delete" && self.transfer_job_delete.is_none() {
            cx.stop_propagation();
            self.request_delete_selected_transfer_job(cx);
        }
    }

    pub(in crate::ui::view) fn can_delete_transfer_job(&self, job_id: &str) -> bool {
        self.transfer_jobs
            .iter()
            .find(|job| job.id == job_id)
            .is_some_and(|job| {
                !matches!(
                    job.status,
                    TransferJobStatus::Running
                        | TransferJobStatus::Paused
                        | TransferJobStatus::Cancelling
                )
            })
    }

    pub(in crate::ui::view) fn next_transfer_id(&self, prefix: &str) -> String {
        format!("{prefix}-{}", self.transfer_jobs.len() + 1)
    }
}

fn transfer_job_remote_parent_path(path: &str) -> String {
    let path = path.trim_end_matches('/');
    match path.rfind('/') {
        Some(0) => "/".to_string(),
        Some(index) => path[..index].to_string(),
        None => ".".to_string(),
    }
}

fn transfer_job_local_target_path(job: &TransferJobState) -> Option<PathBuf> {
    job.summary
        .as_ref()
        .map(|summary| summary.local_path.clone())
        .or_else(|| {
            job.progress
                .as_ref()
                .map(|progress| progress.local_path.clone())
        })
        .or_else(|| match &job.kind {
            TransferJobKind::Download { local_path, .. }
            | TransferJobKind::OpenExternal { local_path, .. } => Some(local_path.clone()),
            _ => None,
        })
}

fn transfer_job_reveal_dir(path: PathBuf) -> PathBuf {
    if path.is_dir() {
        return path;
    }
    path.parent()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.clone())
}
