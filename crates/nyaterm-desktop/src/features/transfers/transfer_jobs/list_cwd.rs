use std::time::{Duration, Instant};

use gpui::{Context, Window};
use nyaterm_transport::{SftpService, SshProcessService};

use crate::features::NyaTermApp;
use crate::models::{
    NavItem, TransferBrowserChildrenMenuStatus, TransferBrowserNavigationSnapshot,
    TransferBrowserPathMenuKind, TransferBrowserPathMenuState, TransferJobEvent, TransferJobKind,
    TransferJobOutput, TransferJobResult, TransferJobState, TransferJobStatus,
};

impl NyaTermApp {
    pub(in crate::features) fn start_transfer_browser_children_job(
        &mut self,
        remote_path: String,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config_owned() else {
            if let Some(TransferBrowserPathMenuState {
                kind: TransferBrowserPathMenuKind::Children { status, .. },
                ..
            }) = self.transfer.browser.path_menu.as_mut()
            {
                *status = TransferBrowserChildrenMenuStatus::Error(
                    "start an SSH session first".to_string(),
                );
            }
            cx.notify();
            return;
        };
        let id = self.next_transfer_id("sftp-children");
        if let Some(TransferBrowserPathMenuState {
            kind:
                TransferBrowserPathMenuKind::Children {
                    path,
                    request_id,
                    status,
                    ..
                },
            ..
        }) = self.transfer.browser.path_menu.as_mut()
        {
            if path != &remote_path {
                return;
            }
            *request_id = Some(id.clone());
            *status = TransferBrowserChildrenMenuStatus::Loading;
        } else {
            return;
        }
        self.transfer.queue.jobs.push(TransferJobState {
            id: id.clone(),
            session_id: self.session.active_id_owned(),
            kind: TransferJobKind::ListChildren {
                remote_path: remote_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Listing child directories in {remote_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        let transfer_tx = self.transfer.queue.tx.clone();
        std::thread::spawn(move || {
            let result = SftpService::new(config)
                .list_dir(&remote_path)
                .map(|entries| TransferJobOutput::ChildEntries {
                    remote_path,
                    entries,
                })
                .map_err(|error| error.to_string());
            let _ = transfer_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn start_sftp_list_job(
        &mut self,
        select_after: Option<String>,
        rollback: TransferBrowserNavigationSnapshot,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config_owned() else {
            self.restore_transfer_browser_navigation(rollback);
            self.terminal.view.status = "start an SSH session first".to_string();
            self.ensure_panel_open(NavItem::Transfers);
            cx.notify();
            return;
        };
        let remote_path = self.transfer.normalized_remote_path();
        self.transfer.browser.path = remote_path.clone();
        self.transfer.browser.status = format!("Listing {remote_path}...");
        self.transfer.browser.loading = true;
        self.transfer.browser.error = None;
        self.transfer.browser.selected_remote_path = None;
        self.transfer.browser.selected_remote_paths.clear();
        let id = self.next_transfer_id("sftp-list");
        let job_session_id = self.session.active_id_owned();
        self.transfer
            .browser
            .navigation_jobs
            .insert(job_session_id.clone().unwrap_or_default(), id.clone());
        self.transfer
            .browser
            .pending_navigations
            .insert(id.clone(), rollback);
        self.transfer.queue.jobs.push(TransferJobState {
            id: id.clone(),
            session_id: job_session_id,
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
        self.terminal.view.status = format!("SFTP list started for {remote_path}");
        let transfer_tx = self.transfer.queue.tx.clone();
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
            self.transfer.browser.status = "remote cwd sync already running".to_string();
            cx.notify();
            return;
        }
        let Some(config) = self.session.active_ssh_config_owned() else {
            self.terminal.view.status = "start an SSH session first".to_string();
            self.ensure_panel_open(NavItem::Transfers);
            cx.notify();
            return;
        };
        self.transfer.browser.auto_sync_cwd_last_at = Some(Instant::now());
        let id = self.next_transfer_id("sftp-sync-cwd");
        let job_session_id = self.session.active_id_owned();
        self.transfer
            .browser
            .navigation_jobs
            .insert(job_session_id.clone().unwrap_or_default(), id.clone());
        self.transfer.queue.jobs.push(TransferJobState {
            id: id.clone(),
            session_id: job_session_id,
            kind: TransferJobKind::SyncCwd,
            status: TransferJobStatus::Running,
            detail: "Resolving remote cwd".to_string(),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.transfer.browser.status = "Resolving remote cwd...".to_string();
        self.transfer.browser.loading = true;
        self.transfer.browser.error = None;
        self.terminal.view.status = "SFTP cwd sync started".to_string();
        let transfer_tx = self.transfer.queue.tx.clone();
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
        self.transfer.queue.jobs.iter().any(|job| {
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
        if self.transfer.browser.home_dir_pending || !self.transfer.browser.home_dir.is_empty() {
            return;
        }
        let Some(config) = self.session.active_ssh_config_owned() else {
            self.transfer.browser.status = "remote home requires an SSH session".to_string();
            cx.notify();
            return;
        };
        self.transfer.browser.home_dir_pending = true;
        let id = self.next_transfer_id("sftp-home");
        self.transfer.queue.jobs.push(TransferJobState {
            id: id.clone(),
            session_id: self.session.active_id_owned(),
            kind: TransferJobKind::ResolveHome,
            status: TransferJobStatus::Running,
            detail: "Resolving remote home".to_string(),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.transfer.browser.status = "Resolving remote home...".to_string();
        let transfer_tx = self.transfer.queue.tx.clone();
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
