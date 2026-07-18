use super::*;

impl NyaTermApp {
    pub(in crate::features) fn start_sftp_list_job(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_sftp_list_job_with_select_after(None, window, cx);
    }

    pub(in crate::features) fn start_sftp_list_job_with_select_after(
        &mut self,
        select_after: Option<String>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.ensure_panel_open(NavItem::Transfers);
            cx.notify();
            return;
        };
        let remote_path = self.normalized_transfer_remote_path();
        self.transfer_browser_path = remote_path.clone();
        self.transfer_browser_status = format!("Listing {remote_path}...");
        self.transfer_browser_loading = true;
        self.transfer_browser_error = None;
        self.transfer_selected_remote_path = None;
        self.transfer_selected_remote_paths.clear();
        let id = self.next_transfer_id("sftp-list");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            session_id: self.active_session_id.clone(),
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

    pub(in crate::features) fn start_transfer_sync_cwd_job(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.transfer_sync_cwd_job_running() {
            self.transfer_browser_status = "remote cwd sync already running".to_string();
            cx.notify();
            return;
        }
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.ensure_panel_open(NavItem::Transfers);
            cx.notify();
            return;
        };
        self.transfer_auto_sync_cwd_last_at = Some(Instant::now());
        let id = self.next_transfer_id("sftp-sync-cwd");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            session_id: self.active_session_id.clone(),
            kind: TransferJobKind::SyncCwd,
            status: TransferJobStatus::Running,
            detail: "Resolving remote cwd".to_string(),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.transfer_browser_status = "Resolving remote cwd...".to_string();
        self.transfer_browser_loading = true;
        self.transfer_browser_error = None;
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

    pub(in crate::features) fn transfer_sync_cwd_job_running(&self) -> bool {
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

    pub(in crate::features) fn start_transfer_browser_home_dir_job(
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
            session_id: self.active_session_id.clone(),
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
}
