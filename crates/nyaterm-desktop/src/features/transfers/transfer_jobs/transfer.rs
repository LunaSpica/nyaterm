use std::path::PathBuf;
use std::sync::Arc;

use gpui::{Context, Window};
use nyaterm_transport::{
    SftpDuplicatePolicy, SftpDuplicateResolver, SftpService, SftpTransferControl,
    SftpTransferOptions, SshSessionConfig,
};

use crate::features::NyaTermApp;
use crate::models::{
    NavItem, TransferJobEvent, TransferJobKind, TransferJobOutput, TransferJobResult,
    TransferJobState, TransferJobStatus,
};

use super::helpers::{TransferProgressEventSender, transfer_job_remote_parent_path};

impl NyaTermApp {
    pub(in crate::features) fn start_sftp_download_job_for_target(
        &mut self,
        remote_path: String,
        local_path: PathBuf,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config.clone() else {
            self.terminal.view.status = "start an SSH session first".to_string();
            self.ensure_panel_open(NavItem::Transfers);
            cx.notify();
            return;
        };
        let duplicate_policy = self.transfer.paths.duplicate_policy;
        let duplicate_resolver = (duplicate_policy == SftpDuplicatePolicy::Ask).then(|| {
            self.session.prompts.duplicate_prompts.clone() as Arc<dyn SftpDuplicateResolver>
        });
        let transfer_options = self.sftp_transfer_options();
        self.enqueue_sftp_download_job_for_target(
            self.session.active_id.clone(),
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
        session_id: Option<String>,
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
        self.transfer.queue.jobs.push(TransferJobState {
            id: id.clone(),
            session_id,
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
        self.terminal.view.status = format!("SFTP download started for {remote_path}");
        let progress_tx = self.transfer.queue.tx.clone();
        let finished_tx = self.transfer.queue.tx.clone();
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

    pub(in crate::features) fn enqueue_sftp_upload_job_for_target(
        &mut self,
        session_id: Option<String>,
        config: SshSessionConfig,
        local_path: PathBuf,
        remote_path: String,
        duplicate_policy: SftpDuplicatePolicy,
        duplicate_resolver: Option<Arc<dyn SftpDuplicateResolver>>,
        transfer_options: SftpTransferOptions,
        cx: &mut Context<Self>,
    ) {
        let id = self.next_transfer_id("sftp-upload");
        let control = SftpTransferControl::new();
        self.transfer.queue.jobs.push(TransferJobState {
            id: id.clone(),
            session_id,
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
        self.terminal.view.status = format!("SFTP upload started for {}", local_path.display());
        let progress_tx = self.transfer.queue.tx.clone();
        let finished_tx = self.transfer.queue.tx.clone();
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
            .transfer
            .queue
            .jobs
            .iter_mut()
            .find(|candidate| candidate.id == job_id)
        else {
            self.terminal.view.status = "transfer job not found".to_string();
            cx.notify();
            return;
        };

        if !matches!(
            job.status,
            TransferJobStatus::Running | TransferJobStatus::Paused
        ) {
            self.terminal.view.status = format!("transfer {} is not running", job.id);
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
            self.terminal.view.status = format!("ZMODEM transfer cancelled: {id}");
            cx.notify();
            return;
        }

        let Some(control) = job.control.as_ref() else {
            self.terminal.view.status = format!("transfer {} cannot be cancelled", job.id);
            cx.notify();
            return;
        };

        control.cancel();
        job.status = TransferJobStatus::Cancelling;
        job.detail = "Cancelling".to_string();
        self.terminal.view.status = format!("SFTP transfer cancelling: {}", job.id);
        cx.notify();
    }

    pub(in crate::features) fn pause_transfer_job(&mut self, job_id: &str, cx: &mut Context<Self>) {
        let Some(job) = self
            .transfer
            .queue
            .jobs
            .iter_mut()
            .find(|candidate| candidate.id == job_id)
        else {
            self.terminal.view.status = "transfer job not found".to_string();
            cx.notify();
            return;
        };

        if job.status != TransferJobStatus::Running {
            self.terminal.view.status = format!("transfer {} is not running", job.id);
            cx.notify();
            return;
        }

        let Some(control) = job.control.as_ref() else {
            self.terminal.view.status = format!("transfer {} cannot be paused", job.id);
            cx.notify();
            return;
        };

        control.pause();
        job.status = TransferJobStatus::Paused;
        job.detail = "Paused".to_string();
        self.terminal.view.status = format!("SFTP transfer paused: {}", job.id);
        cx.notify();
    }

    pub(in crate::features) fn resume_transfer_job(
        &mut self,
        job_id: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(job) = self
            .transfer
            .queue
            .jobs
            .iter_mut()
            .find(|candidate| candidate.id == job_id)
        else {
            self.terminal.view.status = "transfer job not found".to_string();
            cx.notify();
            return;
        };

        if job.status != TransferJobStatus::Paused {
            self.terminal.view.status = format!("transfer {} is not paused", job.id);
            cx.notify();
            return;
        }

        let Some(control) = job.control.as_ref() else {
            self.terminal.view.status = format!("transfer {} cannot be resumed", job.id);
            cx.notify();
            return;
        };

        control.resume();
        job.status = TransferJobStatus::Running;
        job.detail = "Resuming".to_string();
        self.terminal.view.status = format!("SFTP transfer resumed: {}", job.id);
        cx.notify();
    }

    pub(in crate::features) fn retry_transfer_job(
        &mut self,
        job_id: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config.clone() else {
            self.terminal.view.status = "start an SSH session first".to_string();
            self.ensure_panel_open(NavItem::Transfers);
            cx.notify();
            return;
        };
        let Some(index) = self
            .transfer
            .queue
            .jobs
            .iter()
            .position(|candidate| candidate.id == job_id)
        else {
            self.terminal.view.status = "transfer job not found".to_string();
            cx.notify();
            return;
        };
        let kind = self.transfer.queue.jobs[index].kind.clone();
        if !matches!(
            self.transfer.queue.jobs[index].status,
            TransferJobStatus::Failed | TransferJobStatus::Cancelled
        ) {
            self.terminal.view.status = format!("transfer {job_id} is not retryable");
            cx.notify();
            return;
        }

        match kind {
            TransferJobKind::Download {
                remote_path,
                local_path,
            } => {
                let duplicate_policy = self.transfer.paths.duplicate_policy;
                let duplicate_resolver =
                    (duplicate_policy == SftpDuplicatePolicy::Ask).then(|| {
                        self.session.prompts.duplicate_prompts.clone()
                            as Arc<dyn SftpDuplicateResolver>
                    });
                let transfer_options = self.sftp_transfer_options();
                let control = SftpTransferControl::new();
                let job = &mut self.transfer.queue.jobs[index];
                job.status = TransferJobStatus::Running;
                job.detail = format!("Retrying download {remote_path}");
                job.entries.clear();
                job.summary = None;
                job.progress = None;
                job.control = Some(control.clone());
                self.terminal.view.status = format!("retrying SFTP download for {remote_path}");
                let progress_tx = self.transfer.queue.tx.clone();
                let finished_tx = self.transfer.queue.tx.clone();
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
                let duplicate_policy = self.transfer.paths.duplicate_policy;
                let duplicate_resolver =
                    (duplicate_policy == SftpDuplicatePolicy::Ask).then(|| {
                        self.session.prompts.duplicate_prompts.clone()
                            as Arc<dyn SftpDuplicateResolver>
                    });
                let transfer_options = self.sftp_transfer_options();
                let control = SftpTransferControl::new();
                let job = &mut self.transfer.queue.jobs[index];
                job.status = TransferJobStatus::Running;
                job.detail = format!("Retrying upload {}", local_path.display());
                job.entries.clear();
                job.summary = None;
                job.progress = None;
                job.control = Some(control.clone());
                self.terminal.view.status =
                    format!("retrying SFTP upload for {}", local_path.display());
                let progress_tx = self.transfer.queue.tx.clone();
                let finished_tx = self.transfer.queue.tx.clone();
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
                self.terminal.view.status =
                    format!("transfer {job_id} does not support native retry yet");
                cx.notify();
                return;
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn pause_all_transfer_jobs(&mut self, cx: &mut Context<Self>) {
        let active_session_id = self.session.active_id.clone();
        let mut changed = 0;
        for job in &mut self.transfer.queue.jobs {
            if job.is_visible_for_session(active_session_id.as_deref())
                && job.status == TransferJobStatus::Running
                && let Some(control) = job.control.as_ref()
            {
                control.pause();
                job.status = TransferJobStatus::Paused;
                job.detail = "Paused".to_string();
                changed += 1;
            }
        }
        self.terminal.view.status = if changed == 0 {
            "no running transfer jobs to pause".to_string()
        } else {
            format!("paused {changed} transfer job(s)")
        };
        cx.notify();
    }

    pub(in crate::features) fn resume_all_transfer_jobs(&mut self, cx: &mut Context<Self>) {
        let active_session_id = self.session.active_id.clone();
        let mut changed = 0;
        for job in &mut self.transfer.queue.jobs {
            if job.is_visible_for_session(active_session_id.as_deref())
                && job.status == TransferJobStatus::Paused
                && let Some(control) = job.control.as_ref()
            {
                control.resume();
                job.status = TransferJobStatus::Running;
                job.detail = "Resuming".to_string();
                changed += 1;
            }
        }
        self.terminal.view.status = if changed == 0 {
            "no paused transfer jobs to resume".to_string()
        } else {
            format!("resumed {changed} transfer job(s)")
        };
        cx.notify();
    }

    pub(in crate::features) fn cancel_all_transfer_jobs(&mut self, cx: &mut Context<Self>) {
        let active_session_id = self.session.active_id.clone();
        let mut changed = 0;
        for job in &mut self.transfer.queue.jobs {
            if job.is_visible_for_session(active_session_id.as_deref())
                && matches!(
                    job.status,
                    TransferJobStatus::Running | TransferJobStatus::Paused
                )
                && let Some(control) = job.control.as_ref()
            {
                control.cancel();
                job.status = TransferJobStatus::Cancelling;
                job.detail = "Cancelling".to_string();
                changed += 1;
            }
        }
        self.terminal.view.status = if changed == 0 {
            "no active transfer jobs to cancel".to_string()
        } else {
            format!("cancelling {changed} transfer job(s)")
        };
        cx.notify();
    }

    pub(in crate::features) fn clear_completed_transfer_jobs(&mut self, cx: &mut Context<Self>) {
        let active_session_id = self.session.active_id.clone();
        let before = self.transfer.queue.jobs.len();
        self.transfer.queue.jobs.retain(|job| {
            !job.is_visible_for_session(active_session_id.as_deref())
                || job.status != TransferJobStatus::Completed
        });
        let removed = before.saturating_sub(self.transfer.queue.jobs.len());
        self.terminal.view.status = if removed == 0 {
            "no completed transfer jobs to clear".to_string()
        } else {
            format!("cleared {removed} completed transfer job(s)")
        };
        cx.notify();
    }

    pub(in crate::features) fn clear_stopped_transfer_jobs(&mut self, cx: &mut Context<Self>) {
        let active_session_id = self.session.active_id.clone();
        let before = self.transfer.queue.jobs.len();
        self.transfer.queue.jobs.retain(|job| {
            !job.is_visible_for_session(active_session_id.as_deref())
                || matches!(
                    job.status,
                    TransferJobStatus::Running
                        | TransferJobStatus::Paused
                        | TransferJobStatus::Cancelling
                )
        });
        let removed = before.saturating_sub(self.transfer.queue.jobs.len());
        self.terminal.view.status = if removed == 0 {
            "no stopped transfer jobs to clear".to_string()
        } else {
            format!("cleared {removed} stopped transfer job(s)")
        };
        cx.notify();
    }
}
