use super::*;

impl NyaTermApp {

    pub(in crate::ui::view) fn prompt_transfer_download_path_setting(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.transfer_path_prompt.is_some() {
            self.terminal_status = "native path picker is already open".to_string();
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
        self.terminal_status = "selecting default download directory".to_string();
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
                    this.terminal_status = "download path selection cancelled".to_string();
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::ui::view) fn prompt_recording_path_setting(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let options = PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(SharedString::from("Select recording directory")),
        };
        let receiver = cx.prompt_for_paths(options);
        self.terminal_status = "selecting recording directory".to_string();
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
                    this.terminal_status = "recording path selection cancelled".to_string();
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::ui::view) fn prompt_transfer_default_editor_setting(
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
        self.terminal_status = "selecting default editor".to_string();
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
                    this.terminal_status = "editor path selection cancelled".to_string();
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }


    pub(in crate::ui::view) fn resolved_transfer_download_dir(&self) -> Option<PathBuf> {
        let configured = self.settings.transfer_download_path.trim();
        if configured.is_empty() {
            return default_transfer_download_dir();
        }
        Some(expand_transfer_home_path(configured))
    }

    pub(in crate::ui::view) fn reveal_transfer_download_dir(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.resolved_transfer_download_dir() else {
            self.terminal_status = "cannot determine system download directory".to_string();
            cx.notify();
            return;
        };

        if path.exists() && !path.is_dir() {
            self.terminal_status = format!(
                "configured download path is not a directory: {}",
                path.display()
            );
            cx.notify();
            return;
        }

        match std::fs::create_dir_all(&path) {
            Ok(()) => {
                cx.reveal_path(&path);
                self.terminal_status = format!("opened download directory {}", path.display());
            }
            Err(error) => {
                self.terminal_status = format!("failed to prepare download directory: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn normalized_transfer_remote_path(&self) -> String {
        let value = self.transfer_remote_path.trim();
        if value.is_empty() {
            ".".to_string()
        } else {
            value.to_string()
        }
    }

    pub(in crate::ui::view) fn normalized_transfer_local_path(&self) -> PathBuf {
        let value = self.transfer_local_path.trim();
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

    pub(in crate::ui::view) fn prompt_transfer_path(
        &mut self,
        kind: TransferPathPromptKind,
        cx: &mut Context<Self>,
    ) {
        if self.transfer_path_prompt.is_some() {
            self.terminal_status = "native path picker is already open".to_string();
            cx.notify();
            return;
        }

        let options = match kind {
            TransferPathPromptKind::UploadFile => PathPromptOptions {
                files: true,
                directories: false,
                multiple: false,
                prompt: Some(SharedString::from("Select upload file")),
            },
            TransferPathPromptKind::UploadDirectory => PathPromptOptions {
                files: false,
                directories: true,
                multiple: false,
                prompt: Some(SharedString::from("Select upload directory")),
            },
            TransferPathPromptKind::DownloadDirectory => PathPromptOptions {
                files: false,
                directories: true,
                multiple: false,
                prompt: Some(SharedString::from("Select download directory")),
            },
        };
        let remote_path = self.normalized_transfer_remote_path();
        let receiver = cx.prompt_for_paths(options);
        self.transfer_path_prompt = Some(kind);
        self.terminal_status = match kind {
            TransferPathPromptKind::UploadFile => "selecting upload file".to_string(),
            TransferPathPromptKind::UploadDirectory => "selecting upload directory".to_string(),
            TransferPathPromptKind::DownloadDirectory => "selecting download directory".to_string(),
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
                this.apply_transfer_path_prompt_result(kind, remote_path, result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::ui::view) fn prompt_transfer_download_directory_and_start(
        &mut self,
        remote_paths: Vec<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if remote_paths.is_empty() {
            self.terminal_status = "select remote items before downloading".to_string();
            cx.notify();
            return;
        }
        if self.transfer_path_prompt.is_some() {
            self.terminal_status = "native path picker is already open".to_string();
            cx.notify();
            return;
        }
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

        let options = PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(SharedString::from("Select download directory")),
        };
        let receiver = cx.prompt_for_paths(options);
        self.transfer_path_prompt = Some(TransferPathPromptKind::DownloadDirectory);
        self.terminal_status = "selecting download directory".to_string();
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

    pub(in crate::ui::view) fn prompt_transfer_browser_upload_path(
        &mut self,
        kind: TransferPathPromptKind,
        cx: &mut Context<Self>,
    ) {
        if !matches!(
            kind,
            TransferPathPromptKind::UploadFile | TransferPathPromptKind::UploadDirectory
        ) {
            self.terminal_status = "browser upload requires a file or directory".to_string();
            cx.notify();
            return;
        }
        if self.transfer_path_prompt.is_some() {
            self.terminal_status = "native path picker is already open".to_string();
            cx.notify();
            return;
        }

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
        self.transfer_path_prompt = Some(kind);
        self.terminal_status = match kind {
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
                    result,
                    cx,
                );
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_transfer_path_prompt_result(
        &mut self,
        kind: TransferPathPromptKind,
        remote_path: String,
        result: TransferPathPromptResult,
    ) {
        self.transfer_path_prompt = None;
        match result {
            TransferPathPromptResult::Selected(paths) => {
                let Some(path) = paths.into_iter().next() else {
                    self.terminal_status = "path picker cancelled".to_string();
                    return;
                };
                let selected = match kind {
                    TransferPathPromptKind::UploadFile
                    | TransferPathPromptKind::UploadDirectory => path,
                    TransferPathPromptKind::DownloadDirectory => {
                        path.join(download_file_name_from_remote_path(&remote_path))
                    }
                };
                self.transfer_local_path = selected.display().to_string();
                self.transfer_focused_field = TransferInputField::Local;
                self.terminal_status = match kind {
                    TransferPathPromptKind::UploadFile => "upload file selected".to_string(),
                    TransferPathPromptKind::UploadDirectory => {
                        "upload directory selected".to_string()
                    }
                    TransferPathPromptKind::DownloadDirectory => {
                        "download target selected".to_string()
                    }
                };
            }
            TransferPathPromptResult::Cancelled => {
                self.terminal_status = "path picker cancelled".to_string();
            }
            TransferPathPromptResult::Failed(error) => {
                self.terminal_status = format!("path picker failed: {error}");
            }
            TransferPathPromptResult::Closed => {
                self.terminal_status = "path picker closed before returning".to_string();
            }
        }
    }

    fn apply_transfer_download_start_prompt_result(
        &mut self,
        remote_paths: Vec<String>,
        config: SshSessionConfig,
        duplicate_policy: SftpDuplicatePolicy,
        duplicate_resolver: Option<Arc<dyn SftpDuplicateResolver>>,
        transfer_options: SftpTransferOptions,
        result: TransferPathPromptResult,
        cx: &mut Context<Self>,
    ) {
        self.transfer_path_prompt = None;
        match result {
            TransferPathPromptResult::Selected(paths) => {
                let Some(directory) = paths.into_iter().next() else {
                    self.terminal_status = "path picker cancelled".to_string();
                    return;
                };
                let total = remote_paths.len();
                self.transfer_local_path = directory.display().to_string();
                self.transfer_focused_field = TransferInputField::Local;
                for remote_path in remote_paths {
                    let local_path =
                        directory.join(download_file_name_from_remote_path(&remote_path));
                    self.enqueue_sftp_download_job_for_target(
                        config.clone(),
                        remote_path,
                        local_path,
                        duplicate_policy,
                        duplicate_resolver.clone(),
                        transfer_options.clone(),
                        cx,
                    );
                }
                self.terminal_status = format!("{total} SFTP download job(s) started");
                self.transfer_browser_status =
                    format!("Downloading {total} item(s) to {}", directory.display());
            }
            TransferPathPromptResult::Cancelled => {
                self.terminal_status = "download directory selection cancelled".to_string();
                self.transfer_browser_status = "download selection cancelled".to_string();
            }
            TransferPathPromptResult::Failed(error) => {
                self.terminal_status = format!("path picker failed: {error}");
                self.transfer_browser_status = self.terminal_status.clone();
            }
            TransferPathPromptResult::Closed => {
                self.terminal_status = "path picker closed before returning".to_string();
                self.transfer_browser_status = self.terminal_status.clone();
            }
        }
    }

    fn apply_transfer_browser_upload_path_prompt_result(
        &mut self,
        kind: TransferPathPromptKind,
        remote_path: String,
        result: TransferPathPromptResult,
        cx: &mut Context<Self>,
    ) {
        self.transfer_path_prompt = None;
        match result {
            TransferPathPromptResult::Selected(paths) => {
                if paths.is_empty() {
                    self.terminal_status = "path picker cancelled".to_string();
                    self.transfer_browser_status = "upload selection cancelled".to_string();
                    return;
                }
                let total = paths.len();
                if total == 1 {
                    self.transfer_local_path = paths[0].display().to_string();
                } else {
                    self.transfer_local_path = format!("{total} selected upload files");
                }
                self.transfer_remote_path = remote_path.clone();
                self.transfer_focused_field = TransferInputField::Local;
                self.transfer_browser_status = if total == 1 {
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
                    self.start_sftp_upload_job_for_target(path, target_path, cx);
                }
            }
            TransferPathPromptResult::Cancelled => {
                self.terminal_status = "path picker cancelled".to_string();
                self.transfer_browser_status = "upload selection cancelled".to_string();
            }
            TransferPathPromptResult::Failed(error) => {
                self.terminal_status = format!("path picker failed: {error}");
                self.transfer_browser_status = self.terminal_status.clone();
            }
            TransferPathPromptResult::Closed => {
                self.terminal_status = "path picker closed before returning".to_string();
                self.transfer_browser_status = self.terminal_status.clone();
            }
        }
    }

    fn normalized_transfer_browser_upload_target(&self) -> String {
        let value = self.transfer_browser_path.trim();
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
