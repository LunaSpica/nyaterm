use super::*;

impl NyaTermApp {
    pub(in crate::features) fn start_sftp_download_job(
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

    pub(in crate::features) fn start_sftp_download_job_for_target(
        &mut self,
        remote_path: String,
        local_path: PathBuf,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.ensure_panel_open(NavItem::Transfers);
            cx.notify();
            return;
        };
        let duplicate_policy = self.transfer_duplicate_policy;
        let duplicate_resolver = (duplicate_policy == SftpDuplicatePolicy::Ask)
            .then(|| self.duplicate_prompts.clone() as Arc<dyn SftpDuplicateResolver>);
        let transfer_options = self.sftp_transfer_options();
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

    pub(in crate::features) fn enqueue_sftp_download_job_for_target(
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
            let mut progress_sender = TransferProgressEventSender::new(id.clone(), progress_tx);
            let result = SftpService::new(config)
                .download_path_with_progress_options_and_resolver_options(
                    &remote_path,
                    local_path,
                    control,
                    duplicate_policy,
                    duplicate_resolver,
                    transfer_options,
                    move |progress| {
                        progress_sender.send(progress);
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

    pub(in crate::features) fn start_sftp_upload_job(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let local_path = self.normalized_transfer_local_path();
        let remote_path = self.normalized_transfer_remote_path();
        self.start_sftp_upload_job_for_target(local_path, remote_path, cx);
    }

    pub(in crate::features) fn start_sftp_upload_job_for_target(
        &mut self,
        local_path: PathBuf,
        remote_path: String,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.ensure_panel_open(NavItem::Transfers);
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
            let mut progress_sender = TransferProgressEventSender::new(id.clone(), progress_tx);
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
                        progress_sender.send(progress);
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

    pub(in crate::features) fn cancel_transfer_job(
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

        // ZMODEM jobs have no SFTP control — cancel via the session ZMODEM state.
        if let TransferJobKind::ZmodemUpload { session_id, .. }
        | TransferJobKind::ZmodemDownload { session_id, .. } = job.kind.clone()
        {
            let id = job.id.clone();
            job.status = TransferJobStatus::Cancelled;
            job.detail = "Cancelled".to_string();
            job.progress = None;
            self.cancel_zmodem_transfer(&session_id, cx);
            self.terminal_status = format!("ZMODEM transfer cancelled: {id}");
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

    pub(in crate::features) fn pause_transfer_job(&mut self, job_id: &str, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn resume_transfer_job(
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

    pub(in crate::features) fn retry_transfer_job(
        &mut self,
        job_id: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.ensure_panel_open(NavItem::Transfers);
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
                    let mut progress_sender =
                        TransferProgressEventSender::new(job_id.clone(), progress_tx);
                    let result = SftpService::new(config)
                        .download_path_with_progress_options_and_resolver_options(
                            &remote_path,
                            local_path,
                            control,
                            duplicate_policy,
                            duplicate_resolver,
                            transfer_options,
                            move |progress| {
                                progress_sender.send(progress);
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
                    let mut progress_sender =
                        TransferProgressEventSender::new(job_id.clone(), progress_tx);
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
                                progress_sender.send(progress);
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

    pub(in crate::features) fn pause_all_transfer_jobs(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn resume_all_transfer_jobs(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn cancel_all_transfer_jobs(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn clear_completed_transfer_jobs(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn clear_stopped_transfer_jobs(&mut self, cx: &mut Context<Self>) {
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
}
