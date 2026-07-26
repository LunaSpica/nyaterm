use super::*;
use crate::models::{TransferInputField, TransferPathPromptKind, TransferPathPromptResult};

impl NyaTermApp {
    pub(in crate::features) fn prompt_transfer_download_path_setting(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.transfer.paths.prompt.is_some() {
            self.terminal.view.status = "native path picker is already open".to_string();
            cx.notify();
            return;
        }
        let options = PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(SharedString::from("Select default download directory")),
        };
        let receiver = cx.prompt_for_paths(options);
        self.terminal.view.status = "selecting default download directory".to_string();
        cx.spawn(async move |this, cx| {
            let path = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| {
                if let Some(path) = path {
                    this.settings.transfer_download_path = path.display().to_string();
                    this.save_transfer_settings("transfer download path saved", cx);
                } else {
                    this.terminal.view.status = "download path selection cancelled".to_string();
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features) fn prompt_recording_path_setting(&mut self, cx: &mut Context<Self>) {
        let options = PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(SharedString::from("Select recording directory")),
        };
        let receiver = cx.prompt_for_paths(options);
        self.terminal.view.status = "selecting recording directory".to_string();
        cx.spawn(async move |this, cx| {
            let path = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| {
                if let Some(path) = path {
                    this.settings.recording_path = path.display().to_string();
                    this.save_recording_settings(cx);
                } else {
                    this.terminal.view.status = "recording path selection cancelled".to_string();
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features) fn prompt_transfer_default_editor_setting(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from("Select default editor executable")),
        };
        let receiver = cx.prompt_for_paths(options);
        self.terminal.view.status = "selecting default editor".to_string();
        cx.spawn(async move |this, cx| {
            let path = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| {
                if let Some(path) = path {
                    this.settings.transfer_default_editor = path.display().to_string();
                    this.save_transfer_settings("transfer editor path saved", cx);
                } else {
                    this.terminal.view.status = "editor path selection cancelled".to_string();
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features) fn resolved_transfer_download_dir(&self) -> Option<PathBuf> {
        let configured = self.settings.transfer_download_path.trim();
        if configured.is_empty() {
            return default_transfer_download_dir();
        }
        Some(expand_transfer_home_path(configured))
    }

    pub(in crate::features) fn reveal_transfer_download_dir(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.resolved_transfer_download_dir() else {
            self.terminal.view.status = "cannot determine system download directory".to_string();
            cx.notify();
            return;
        };

        if path.exists() && !path.is_dir() {
            self.terminal.view.status = format!(
                "configured download path is not a directory: {}",
                path.display()
            );
            cx.notify();
            return;
        }

        match std::fs::create_dir_all(&path) {
            Ok(()) => {
                cx.reveal_path(&path);
                self.terminal.view.status = format!("opened download directory {}", path.display());
            }
            Err(error) => {
                self.terminal.view.status =
                    format!("failed to prepare download directory: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn normalized_transfer_remote_path(&self) -> String {
        let value = self.transfer.paths.remote.trim();
        if value.is_empty() {
            ".".to_string()
        } else {
            value.to_string()
        }
    }

    pub(in crate::features) fn normalized_transfer_local_path(&self) -> PathBuf {
        let value = self.transfer.paths.local.trim();
        if value.is_empty() {
            let file_name =
                download_file_name_from_remote_path(&self.normalized_transfer_remote_path());
            let download_path = self.settings.transfer_download_path.trim();
            if download_path.is_empty() {
                PathBuf::from(file_name)
            } else {
                PathBuf::from(download_path).join(file_name)
            }
        } else {
            PathBuf::from(value)
        }
    }

    pub(in crate::features) fn prompt_transfer_download_directory_and_start(
        &mut self,
        remote_paths: Vec<String>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if remote_paths.is_empty() {
            self.terminal.view.status = "select remote items before downloading".to_string();
            cx.notify();
            return;
        }
        if self.transfer.paths.prompt.is_some() {
            self.terminal.view.status = "native path picker is already open".to_string();
            cx.notify();
            return;
        }
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal.view.status = "start an SSH session first".to_string();
            self.selected_nav = NavItem::Transfers;
            cx.notify();
            return;
        };
        let session_id = self.active_session_id.clone();

        let duplicate_policy = self.transfer.paths.duplicate_policy;
        let duplicate_resolver = (duplicate_policy == SftpDuplicatePolicy::Ask)
            .then(|| self.duplicate_prompts.clone() as Arc<dyn SftpDuplicateResolver>);
        let transfer_options = self.sftp_transfer_options();
        let options = PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(SharedString::from("Select download directory")),
        };
        let receiver = cx.prompt_for_paths(options);
        self.transfer.paths.prompt = Some(TransferPathPromptKind::DownloadDirectory);
        self.terminal.view.status = "selecting download directory".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => {
                    if paths.is_empty() {
                        TransferPathPromptResult::Cancelled
                    } else {
                        TransferPathPromptResult::Selected(paths)
                    }
                }
                Ok(Ok(None)) => TransferPathPromptResult::Cancelled,
                Ok(Err(error)) => TransferPathPromptResult::Failed(error.to_string()),
                Err(_) => TransferPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_transfer_download_start_prompt_result(
                    remote_paths,
                    session_id,
                    config,
                    duplicate_policy,
                    duplicate_resolver,
                    transfer_options,
                    result,
                    cx,
                );
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features) fn prompt_transfer_browser_upload_path(
        &mut self,
        kind: TransferPathPromptKind,
        cx: &mut Context<Self>,
    ) {
        if !matches!(
            kind,
            TransferPathPromptKind::UploadFile | TransferPathPromptKind::UploadDirectory
        ) {
            self.terminal.view.status = "browser upload requires a file or directory".to_string();
            cx.notify();
            return;
        }
        if self.transfer.paths.prompt.is_some() {
            self.terminal.view.status = "native path picker is already open".to_string();
            cx.notify();
            return;
        }
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal.view.status = "start an SSH session first".to_string();
            cx.notify();
            return;
        };
        let session_id = self.active_session_id.clone();
        let duplicate_policy = self.transfer.paths.duplicate_policy;
        let duplicate_resolver = (duplicate_policy == SftpDuplicatePolicy::Ask)
            .then(|| self.duplicate_prompts.clone() as Arc<dyn SftpDuplicateResolver>);
        let transfer_options = self.sftp_transfer_options();

        let options = match kind {
            TransferPathPromptKind::UploadFile => PathPromptOptions {
                files: true,
                directories: false,
                multiple: true,
                prompt: Some(SharedString::from("Select upload files")),
            },
            TransferPathPromptKind::UploadDirectory => PathPromptOptions {
                files: false,
                directories: true,
                multiple: false,
                prompt: Some(SharedString::from("Select upload directory")),
            },
            TransferPathPromptKind::DownloadDirectory => unreachable!(),
        };
        let remote_path = self.normalized_transfer_browser_upload_target();
        let receiver = cx.prompt_for_paths(options);
        self.transfer.paths.prompt = Some(kind);
        self.terminal.view.status = match kind {
            TransferPathPromptKind::UploadFile => "selecting upload file".to_string(),
            TransferPathPromptKind::UploadDirectory => "selecting upload directory".to_string(),
            TransferPathPromptKind::DownloadDirectory => unreachable!(),
        };
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => {
                    if paths.is_empty() {
                        TransferPathPromptResult::Cancelled
                    } else {
                        TransferPathPromptResult::Selected(paths)
                    }
                }
                Ok(Ok(None)) => TransferPathPromptResult::Cancelled,
                Ok(Err(error)) => TransferPathPromptResult::Failed(error.to_string()),
                Err(_) => TransferPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_transfer_browser_upload_path_prompt_result(
                    kind,
                    remote_path,
                    session_id,
                    config,
                    duplicate_policy,
                    duplicate_resolver,
                    transfer_options,
                    result,
                    cx,
                );
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_transfer_download_start_prompt_result(
        &mut self,
        remote_paths: Vec<String>,
        session_id: Option<String>,
        config: SshSessionConfig,
        duplicate_policy: SftpDuplicatePolicy,
        duplicate_resolver: Option<Arc<dyn SftpDuplicateResolver>>,
        transfer_options: SftpTransferOptions,
        result: TransferPathPromptResult,
        cx: &mut Context<Self>,
    ) {
        self.transfer.paths.prompt = None;
        match result {
            TransferPathPromptResult::Selected(paths) => {
                let Some(directory) = paths.into_iter().next() else {
                    self.terminal.view.status = "path picker cancelled".to_string();
                    return;
                };
                let total = remote_paths.len();
                self.transfer.paths.local = directory.display().to_string();
                self.transfer.panel.focused_field = TransferInputField::Local;
                for remote_path in remote_paths {
                    let local_path =
                        directory.join(download_file_name_from_remote_path(&remote_path));
                    self.enqueue_sftp_download_job_for_target(
                        session_id.clone(),
                        config.clone(),
                        remote_path,
                        local_path,
                        duplicate_policy,
                        duplicate_resolver.clone(),
                        transfer_options.clone(),
                        cx,
                    );
                }
                self.terminal.view.status = format!("{total} SFTP download job(s) started");
                self.transfer.browser.status =
                    format!("Downloading {total} item(s) to {}", directory.display());
            }
            TransferPathPromptResult::Cancelled => {
                self.terminal.view.status = "download directory selection cancelled".to_string();
                self.transfer.browser.status = "download selection cancelled".to_string();
            }
            TransferPathPromptResult::Failed(error) => {
                self.terminal.view.status = format!("path picker failed: {error}");
                self.transfer.browser.status = self.terminal.view.status.clone();
            }
            TransferPathPromptResult::Closed => {
                self.terminal.view.status = "path picker closed before returning".to_string();
                self.transfer.browser.status = self.terminal.view.status.clone();
            }
        }
    }

    fn apply_transfer_browser_upload_path_prompt_result(
        &mut self,
        kind: TransferPathPromptKind,
        remote_path: String,
        session_id: Option<String>,
        config: SshSessionConfig,
        duplicate_policy: SftpDuplicatePolicy,
        duplicate_resolver: Option<Arc<dyn SftpDuplicateResolver>>,
        transfer_options: SftpTransferOptions,
        result: TransferPathPromptResult,
        cx: &mut Context<Self>,
    ) {
        self.transfer.paths.prompt = None;
        match result {
            TransferPathPromptResult::Selected(paths) => {
                if paths.is_empty() {
                    self.terminal.view.status = "path picker cancelled".to_string();
                    self.transfer.browser.status = "upload selection cancelled".to_string();
                    return;
                }
                let total = paths.len();
                if total == 1 {
                    self.transfer.paths.local = paths[0].display().to_string();
                } else {
                    self.transfer.paths.local = format!("{total} selected upload files");
                }
                self.transfer.paths.remote = remote_path.clone();
                self.transfer.panel.focused_field = TransferInputField::Local;
                self.transfer.browser.status = if total == 1 {
                    format!(
                        "Uploading {} to {}",
                        paths[0].display(),
                        truncate_preview(&remote_path, 48)
                    )
                } else {
                    format!(
                        "Uploading {total} files to {}",
                        truncate_preview(&remote_path, 48)
                    )
                };
                for path in paths {
                    let fallback = match kind {
                        TransferPathPromptKind::UploadFile => "uploaded_file",
                        TransferPathPromptKind::UploadDirectory => "uploaded_folder",
                        TransferPathPromptKind::DownloadDirectory => unreachable!(),
                    };
                    let upload_name = transfer_upload_local_name(&path, fallback);
                    let target_path = transfer_upload_remote_child_path(&remote_path, &upload_name);
                    self.enqueue_sftp_upload_job_for_target(
                        session_id.clone(),
                        config.clone(),
                        path,
                        target_path,
                        duplicate_policy,
                        duplicate_resolver.clone(),
                        transfer_options.clone(),
                        cx,
                    );
                }
            }
            TransferPathPromptResult::Cancelled => {
                self.terminal.view.status = "path picker cancelled".to_string();
                self.transfer.browser.status = "upload selection cancelled".to_string();
            }
            TransferPathPromptResult::Failed(error) => {
                self.terminal.view.status = format!("path picker failed: {error}");
                self.transfer.browser.status = self.terminal.view.status.clone();
            }
            TransferPathPromptResult::Closed => {
                self.terminal.view.status = "path picker closed before returning".to_string();
                self.transfer.browser.status = self.terminal.view.status.clone();
            }
        }
    }

    fn normalized_transfer_browser_upload_target(&self) -> String {
        let value = self.transfer.browser.path.trim();
        if value.is_empty() {
            self.normalized_transfer_remote_path()
        } else if value == "/" {
            "/".to_string()
        } else {
            value.trim_end_matches('/').to_string()
        }
    }
}

fn default_transfer_download_dir() -> Option<PathBuf> {
    dirs::download_dir().or_else(|| dirs::home_dir().map(|home| home.join("Downloads")))
}

fn expand_transfer_home_path(path: &str) -> PathBuf {
    if path == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from(path));
    }
    path.strip_prefix("~/")
        .and_then(|suffix| dirs::home_dir().map(|home| home.join(suffix)))
        .unwrap_or_else(|| PathBuf::from(path))
}

fn transfer_upload_local_name(path: &std::path::Path, fallback: &str) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn transfer_upload_remote_child_path(remote_dir: &str, name: &str) -> String {
    match remote_dir.trim_end_matches('/') {
        "" | "." => name.to_string(),
        "/" => format!("/{name}"),
        parent => format!("{parent}/{name}"),
    }
}
