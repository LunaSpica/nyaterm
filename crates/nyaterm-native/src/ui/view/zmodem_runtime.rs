use super::*;
use nyaterm_session::{
    ZmodemAction, ZmodemDetectResult, ZmodemDetector, ZmodemDirection, ZmodemEvent, ZmodemTransfer,
    start_zmodem_transfer,
};
use std::path::PathBuf;

pub(in crate::ui::view) struct ZmodemSessionState {
    pub(in crate::ui::view) detector: ZmodemDetector,
    pub(in crate::ui::view) transfer: Option<ZmodemTransfer>,
    pub(in crate::ui::view) pending_upload: Option<Vec<PathBuf>>,
    /// Download waiting for user to pick a save directory.
    pub(in crate::ui::view) pending_download: bool,
}

impl Default for ZmodemSessionState {
    fn default() -> Self {
        Self {
            detector: ZmodemDetector::new(),
            transfer: None,
            pending_upload: None,
            pending_download: false,
        }
    }
}

impl NyaTermApp {
    fn zmodem_state_mut(&mut self, session_id: &str) -> &mut ZmodemSessionState {
        self.zmodem_sessions
            .entry(session_id.to_string())
            .or_default()
    }

    pub(in crate::ui::view) fn clear_zmodem_session(&mut self, session_id: &str) {
        self.zmodem_sessions.remove(session_id);
    }

    /// Queue local files for ZMODEM upload (remote `rz`) after optional SFTP conflict probe.
    pub(in crate::ui::view) fn start_zmodem_upload(
        &mut self,
        session_id: String,
        files: Vec<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        if files.is_empty() {
            return;
        }
        if self.is_session_disconnected(&session_id) {
            self.terminal_status =
                "session disconnected — reconnect before ZMODEM upload".to_string();
            cx.notify();
            return;
        }
        let state = self.zmodem_state_mut(&session_id);
        if state.transfer.is_some() {
            self.terminal_status = "ZMODEM transfer already active".to_string();
            cx.notify();
            return;
        }

        // Prefer SSH config bound to this session, fall back to active.
        let ssh_config = self
            .session_metadata
            .get(&session_id)
            .and_then(|meta| meta.ssh_config.clone())
            .or_else(|| self.active_ssh_config.clone());

        let Some(config) = ssh_config else {
            // Non-SSH sessions cannot probe; start rz immediately.
            self.begin_zmodem_upload_after_probe(session_id, files, cx);
            return;
        };

        let remote_dir = self
            .session_cwds
            .get(&session_id)
            .cloned()
            .filter(|cwd| !cwd.trim().is_empty())
            .unwrap_or_else(|| "/".to_string());
        let remote_dir = remote_dir.trim_end_matches('/').to_string();
        let remote_dir = if remote_dir.is_empty() {
            "/".to_string()
        } else {
            remote_dir
        };

        let policy = self.transfer_duplicate_policy;
        let resolver = self.duplicate_prompts.clone();
        let id = self.next_transfer_id("zmodem-probe");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::ZmodemConflictProbe {
                session_id: session_id.clone(),
                remote_dir: remote_dir.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!(
                "Probing remote conflicts in {remote_dir} ({} file(s))",
                files.len()
            ),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.terminal_status = format!(
            "ZMODEM preparing upload ({} file(s)) — probing remote conflicts",
            files.len()
        );
        let transfer_tx = self.transfer_tx.clone();
        let probe_session_id = session_id.clone();
        std::thread::spawn(move || {
            let result =
                probe_zmodem_remote_conflicts(config, remote_dir, files, policy, resolver.as_ref())
                    .map(
                        |(resolved, probe_skipped)| TransferJobOutput::ZmodemProbeReady {
                            session_id: probe_session_id,
                            files: resolved,
                            probe_skipped,
                        },
                    )
                    .map_err(|error| error.to_string());
            let _ = transfer_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }

    /// Start remote `rz` after conflict resolution (or when probing is unavailable).
    pub(in crate::ui::view) fn begin_zmodem_upload_after_probe(
        &mut self,
        session_id: String,
        files: Vec<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        if files.is_empty() {
            self.terminal_status =
                "ZMODEM upload cancelled — no files remaining after conflict resolution"
                    .to_string();
            cx.notify();
            return;
        }
        if self.is_session_disconnected(&session_id) {
            self.terminal_status =
                "session disconnected — reconnect before ZMODEM upload".to_string();
            cx.notify();
            return;
        }
        let state = self.zmodem_state_mut(&session_id);
        if state.transfer.is_some() {
            self.terminal_status = "ZMODEM transfer already active".to_string();
            cx.notify();
            return;
        }
        state.pending_upload = Some(files.clone());
        state.pending_download = false;
        // Remote side runs `rz` and emits ZMODEM upload (local send) headers.
        let cmd = b"rz\r".to_vec();
        match self.session_manager.write(&session_id, &cmd) {
            Ok(()) => {
                self.terminal_status = format!(
                    "ZMODEM upload prepared ({} file(s)) — waiting for remote rz",
                    files.len()
                );
            }
            Err(error) => {
                self.zmodem_state_mut(&session_id).pending_upload = None;
                self.terminal_status = format!("ZMODEM upload failed to start: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn cancel_zmodem_transfer(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.zmodem_sessions.get_mut(session_id) else {
            return;
        };
        state.pending_upload = None;
        state.pending_download = false;
        let mut actions = Vec::new();
        if let Some(transfer) = state.transfer.as_mut() {
            actions = transfer.cancel();
        }
        state.transfer = None;
        state.detector = ZmodemDetector::new();
        self.apply_zmodem_actions(session_id, actions, cx);
        self.terminal_status = "ZMODEM transfer cancelled".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn accept_zmodem_download(
        &mut self,
        session_id: String,
        save_dir: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.zmodem_sessions.get_mut(&session_id) else {
            return;
        };
        state.pending_download = false;
        let mut actions = Vec::new();
        if let Some(transfer) = state.transfer.as_mut() {
            actions = transfer.accept_download(save_dir);
            if transfer.is_done() {
                state.transfer = None;
                state.detector = ZmodemDetector::new();
            }
        }
        self.apply_zmodem_actions(&session_id, actions, cx);
        cx.notify();
    }

    /// Process raw session output for ZMODEM interception. Returns bytes that
    /// should still be painted in the terminal (empty while a transfer is active).
    pub(in crate::ui::view) fn process_zmodem_output(
        &mut self,
        session_id: &str,
        data: &[u8],
        cx: &mut Context<Self>,
    ) -> Vec<u8> {
        if data.is_empty() {
            return Vec::new();
        }
        let state = self.zmodem_state_mut(session_id);

        // Active transfer: consume all raw bytes.
        if state.transfer.is_some() {
            let mut actions = Vec::new();
            let done = {
                let state = self.zmodem_state_mut(session_id);
                if let Some(transfer) = state.transfer.as_mut() {
                    actions = transfer.feed_incoming(data);
                    transfer.is_done()
                } else {
                    false
                }
            };
            self.apply_zmodem_actions(session_id, actions, cx);
            if done {
                if let Some(state) = self.zmodem_sessions.get_mut(session_id) {
                    state.transfer = None;
                    state.detector = ZmodemDetector::new();
                }
            }
            return Vec::new();
        }

        // Detection path.
        let feed_result = {
            let state = self.zmodem_state_mut(session_id);
            state.detector.feed(data)
        };
        match feed_result {
            ZmodemDetectResult::NoMatch { passthrough } => passthrough,
            ZmodemDetectResult::Detected {
                direction,
                passthrough,
                initial_bytes,
            } => {
                let prepared_upload = if direction == ZmodemDirection::Upload {
                    self.zmodem_state_mut(session_id).pending_upload.take()
                } else {
                    None
                };
                let (transfer, bootstrap) =
                    start_zmodem_transfer(direction, &initial_bytes, prepared_upload);
                let actions = bootstrap;
                {
                    let state = self.zmodem_state_mut(session_id);
                    state.transfer = Some(transfer);
                    if direction == ZmodemDirection::Download {
                        state.pending_download = true;
                    }
                }
                // If upload auto-started with prepared files, bootstrap may already
                // have driven protocol. For download without a path, wait for dialog.
                if direction == ZmodemDirection::Download {
                    self.prompt_zmodem_download_directory(session_id.to_string(), cx);
                }
                self.apply_zmodem_actions(session_id, actions, cx);
                // Surface detection event status.
                self.terminal_status = match direction {
                    ZmodemDirection::Upload => "ZMODEM upload detected".to_string(),
                    ZmodemDirection::Download => {
                        "ZMODEM download detected — choose save folder".to_string()
                    }
                };
                passthrough
            }
        }
    }

    fn apply_zmodem_actions(
        &mut self,
        session_id: &str,
        actions: Vec<ZmodemAction>,
        cx: &mut Context<Self>,
    ) {
        for action in actions {
            match action {
                ZmodemAction::SendToRemote(bytes) => {
                    if let Err(error) = self.session_manager.write(session_id, &bytes) {
                        self.terminal_status = format!("ZMODEM write failed: {error}");
                    }
                }
                ZmodemAction::EmitEvent(event) => self.handle_zmodem_event(session_id, event, cx),
            }
        }
    }

    fn handle_zmodem_event(
        &mut self,
        session_id: &str,
        event: ZmodemEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            ZmodemEvent::Detected { direction } => {
                self.terminal_status = match direction {
                    ZmodemDirection::Upload => "ZMODEM upload in progress".to_string(),
                    ZmodemDirection::Download => "ZMODEM download in progress".to_string(),
                };
            }
            ZmodemEvent::Progress {
                file_name,
                bytes_transferred,
                total_size,
                direction,
            } => {
                let dir = match direction {
                    ZmodemDirection::Upload => "↑",
                    ZmodemDirection::Download => "↓",
                };
                if total_size > 0 {
                    let pct = (bytes_transferred.saturating_mul(100) / total_size).min(100);
                    self.terminal_status = format!(
                        "ZMODEM {dir} {file_name}: {pct}% ({bytes_transferred}/{total_size})"
                    );
                } else {
                    self.terminal_status =
                        format!("ZMODEM {dir} {file_name}: {bytes_transferred} bytes");
                }
                self.upsert_zmodem_transfer_job(
                    session_id,
                    direction,
                    &file_name,
                    bytes_transferred,
                    total_size,
                    false,
                    None,
                    cx,
                );
            }
            ZmodemEvent::Complete {
                direction,
                file_count,
            } => {
                let dir = match direction {
                    ZmodemDirection::Upload => "upload",
                    ZmodemDirection::Download => "download",
                };
                self.terminal_status =
                    format!("ZMODEM {dir} complete ({file_count} file(s)) [{session_id}]");
                self.finish_zmodem_transfer_jobs(session_id, true, None, cx);
                if let Some(state) = self.zmodem_sessions.get_mut(session_id) {
                    state.transfer = None;
                    state.detector = ZmodemDetector::new();
                    state.pending_download = false;
                    state.pending_upload = None;
                }
            }
            ZmodemEvent::Failed { reason } => {
                self.terminal_status = format!("ZMODEM failed: {reason}");
                self.finish_zmodem_transfer_jobs(session_id, false, Some(reason.as_str()), cx);
                if let Some(state) = self.zmodem_sessions.get_mut(session_id) {
                    state.transfer = None;
                    state.detector = ZmodemDetector::new();
                    state.pending_download = false;
                    state.pending_upload = None;
                }
            }
        }
        cx.notify();
    }

    fn upsert_zmodem_transfer_job(
        &mut self,
        session_id: &str,
        direction: ZmodemDirection,
        file_name: &str,
        bytes_transferred: u64,
        total_size: u64,
        completed: bool,
        fail_reason: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        let short = short_id(session_id);
        let kind = match direction {
            ZmodemDirection::Upload => TransferJobKind::ZmodemUpload {
                session_id: session_id.to_string(),
                file_name: file_name.to_string(),
            },
            ZmodemDirection::Download => TransferJobKind::ZmodemDownload {
                session_id: session_id.to_string(),
                file_name: file_name.to_string(),
            },
        };
        let progress = SftpTransferProgress {
            remote_path: format!("zmodem://{short}/{file_name}"),
            local_path: PathBuf::from(file_name),
            bytes_transferred,
            total_bytes: (total_size > 0).then_some(total_size),
        };
        if let Some(job) = self.transfer_jobs.iter_mut().find(|job| {
            matches!(
                &job.kind,
                TransferJobKind::ZmodemUpload {
                    session_id: sid,
                    file_name: name
                }
                | TransferJobKind::ZmodemDownload {
                    session_id: sid,
                    file_name: name
                } if sid == session_id && name == file_name
            ) && matches!(
                job.status,
                TransferJobStatus::Running | TransferJobStatus::Cancelling
            )
        }) {
            job.progress = Some(progress);
            job.detail = if completed {
                "Complete".to_string()
            } else if let Some(reason) = fail_reason {
                format!("Failed: {reason}")
            } else if total_size > 0 {
                format!(
                    "{:.0}%",
                    (bytes_transferred as f64 / total_size as f64 * 100.).clamp(0., 100.)
                )
            } else {
                format!("{bytes_transferred} bytes")
            };
            if completed {
                job.status = TransferJobStatus::Completed;
            } else if fail_reason.is_some() {
                job.status = TransferJobStatus::Failed;
            }
            return;
        }

        let id = self.next_transfer_id("zmodem");
        let status = if completed {
            TransferJobStatus::Completed
        } else if fail_reason.is_some() {
            TransferJobStatus::Failed
        } else {
            TransferJobStatus::Running
        };
        let detail = fail_reason
            .map(|reason| format!("Failed: {reason}"))
            .unwrap_or_else(|| {
                if completed {
                    "Complete".to_string()
                } else {
                    format!("Transferring {file_name}")
                }
            });
        self.transfer_jobs.push(TransferJobState {
            id,
            kind,
            status,
            detail,
            entries: Vec::new(),
            summary: None,
            progress: Some(progress),
            control: None,
        });
        let _ = cx;
    }

    fn finish_zmodem_transfer_jobs(
        &mut self,
        session_id: &str,
        success: bool,
        fail_reason: Option<&str>,
        _cx: &mut Context<Self>,
    ) {
        for job in &mut self.transfer_jobs {
            let is_zmodem = matches!(
                &job.kind,
                TransferJobKind::ZmodemUpload {
                    session_id: sid,
                    ..
                }
                | TransferJobKind::ZmodemDownload {
                    session_id: sid,
                    ..
                } if sid == session_id
            );
            if !is_zmodem {
                continue;
            }
            if !matches!(
                job.status,
                TransferJobStatus::Running | TransferJobStatus::Cancelling
            ) {
                continue;
            }
            if success {
                job.status = TransferJobStatus::Completed;
                job.detail = "Complete".to_string();
            } else {
                job.status = TransferJobStatus::Failed;
                job.detail = fail_reason
                    .map(|r| format!("Failed: {r}"))
                    .unwrap_or_else(|| "Failed".to_string());
            }
        }
    }

    fn prompt_zmodem_download_directory(&mut self, session_id: String, cx: &mut Context<Self>) {
        let options = PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(SharedString::from("Select ZMODEM download folder")),
        };
        let receiver = cx.prompt_for_paths(options);
        self.terminal_status = "selecting ZMODEM download folder…".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| {
                if let Some(dir) = result {
                    this.accept_zmodem_download(session_id, dir, cx);
                } else {
                    this.cancel_zmodem_transfer(&session_id, cx);
                }
            });
        })
        .detach();
    }
}

fn probe_zmodem_remote_conflicts(
    config: SshSessionConfig,
    remote_dir: String,
    files: Vec<PathBuf>,
    policy: SftpDuplicatePolicy,
    resolver: &dyn SftpDuplicateResolver,
) -> Result<(Vec<PathBuf>, bool), String> {
    let service = SftpService::new(config);
    let entries = match service.list_dir(&remote_dir) {
        Ok(entries) => entries,
        Err(_) => {
            // Tauri: SFTP probe failure/timeout falls through without blocking upload.
            return Ok((files, true));
        }
    };
    let existing: HashSet<String> = entries.into_iter().map(|entry| entry.name).collect();
    let mut clean = Vec::new();
    let mut conflicts = Vec::new();
    for path in files {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        if existing.contains(&name) {
            conflicts.push((path, name));
        } else {
            clean.push(path);
        }
    }
    if conflicts.is_empty() {
        return Ok((clean, false));
    }

    let join_remote = |name: &str| -> String {
        if remote_dir == "/" {
            format!("/{name}")
        } else {
            format!("{remote_dir}/{name}")
        }
    };

    match policy {
        SftpDuplicatePolicy::Skip => Ok((clean, false)),
        SftpDuplicatePolicy::Overwrite | SftpDuplicatePolicy::Rename => {
            for (_path, name) in &conflicts {
                let remote_path = join_remote(name);
                let _ = service.delete_path(&remote_path);
            }
            let mut all = clean;
            all.extend(conflicts.into_iter().map(|(path, _)| path));
            Ok((all, false))
        }
        SftpDuplicatePolicy::Ask => {
            let mut resolved = clean;
            for (path, name) in conflicts {
                let remote_path = join_remote(&name);
                let request = SftpDuplicateRequest {
                    direction: SftpTransferDirection::Upload,
                    source_path: path.display().to_string(),
                    target_path: remote_path.clone(),
                    is_directory: false,
                };
                let decision = resolver
                    .resolve_duplicate(&request)
                    .unwrap_or(SftpDuplicateDecision::Skip);
                match decision {
                    SftpDuplicateDecision::Overwrite | SftpDuplicateDecision::Rename => {
                        let _ = service.delete_path(&remote_path);
                        resolved.push(path);
                    }
                    SftpDuplicateDecision::Skip => {}
                }
            }
            Ok((resolved, false))
        }
    }
}
