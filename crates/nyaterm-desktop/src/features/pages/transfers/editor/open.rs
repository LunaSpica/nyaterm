use super::*;

impl NyaTermApp {
    pub(in crate::features) fn enabled_transfer_file_ai_actions_for_entry(
        &self,
        entry: &SftpFileEntry,
    ) -> Vec<AiCustomActionConfig> {
        if !self.ai_settings.enabled
            || entry.file_type == SftpFileType::Directory
            || entry
                .size
                .is_some_and(|size| size > self.ai_settings.max_ai_file_size_bytes)
        {
            return Vec::new();
        }

        self.ai_settings
            .file_ai_actions
            .iter()
            .filter(|action| action.enabled && !action.name.trim().is_empty())
            .cloned()
            .collect()
    }

    pub(in crate::features) fn start_transfer_file_ai_action(
        &mut self,
        entry: SftpFileEntry,
        action: AiCustomActionConfig,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if entry.file_type == SftpFileType::Directory {
            self.transfer_browser_status = "AI file actions require a file".to_string();
            self.terminal_status = "directories cannot be sent to file AI actions".to_string();
            cx.notify();
            return;
        }
        if entry
            .size
            .is_some_and(|size| size > self.ai_settings.max_ai_file_size_bytes)
        {
            self.transfer_browser_status = "file exceeds AI file size limit".to_string();
            self.terminal_status = format!("{} is too large for AI file actions", entry.path);
            cx.notify();
            return;
        }
        if !self.ai_settings.enabled {
            self.transfer_browser_status = "AI assistant is disabled".to_string();
            self.terminal_status = "AI assistant is disabled".to_string();
            cx.notify();
            return;
        }
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            cx.notify();
            return;
        };

        self.transfer_selected_remote_path = Some(entry.path.clone());
        self.transfer_remote_path = entry.path.clone();

        let remote_path = entry.path.clone();
        let action_id = action.id.clone();
        let action_name = action.name.clone();
        let prompt = action.prompt.clone();
        let max_bytes = self.ai_settings.max_ai_file_size_bytes;
        let id = self.next_transfer_id("sftp-ai-file");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::AiFileAction {
                remote_path: remote_path.clone(),
                action_id: action_id.clone(),
                action_name: action_name.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Preparing AI file action {action_name} for {remote_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.transfer_browser_status = format!("loading {remote_path} for AI");
        self.terminal_status = format!("SFTP AI file action started: {remote_path}");
        let transfer_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let result = SftpService::new(config)
                .read_text_file(&remote_path, max_bytes)
                .map(|file| TransferJobOutput::AiFileActionLoaded {
                    remote_path,
                    action_id,
                    action_name,
                    prompt,
                    file,
                })
                .map_err(|error| error.to_string());
            let _ = transfer_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn open_selected_transfer_default(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.selected_transfer_entry() else {
            self.terminal_status = "select a remote file first".to_string();
            cx.notify();
            return;
        };
        self.open_transfer_default(entry, window, cx);
    }

    pub(in crate::features) fn open_selected_transfer_editor(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.selected_transfer_entry() else {
            self.terminal_status = "select a remote file first".to_string();
            cx.notify();
            return;
        };
        self.open_transfer_editor(entry, window, cx);
    }

    pub(in crate::features) fn open_selected_transfer_external(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.selected_transfer_entry() else {
            self.terminal_status = "select a remote file first".to_string();
            cx.notify();
            return;
        };
        self.open_transfer_external(entry, window, cx);
    }

    pub(in crate::features) fn open_transfer_default(
        &mut self,
        entry: SftpFileEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.settings.transfer_editor_type == "internal" {
            self.open_transfer_editor(entry, window, cx);
        } else {
            self.open_transfer_external(entry, window, cx);
        }
    }

    pub(in crate::features) fn open_transfer_editor(
        &mut self,
        entry: SftpFileEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match remote_file_text_kind(&entry.name) {
            RemoteFileTextKind::Text => self.open_transfer_editor_direct(entry, window, cx),
            RemoteFileTextKind::Binary => {
                self.transfer_browser_status = "known binary file opened externally".to_string();
                self.open_transfer_external(entry, window, cx);
            }
            RemoteFileTextKind::Unknown => {
                self.transfer_unknown_file = Some(TransferUnknownFileState { entry });
                self.terminal_status = "confirm how to open unknown remote file".to_string();
                window.focus(&self.transfer_unknown_file_focus);
                cx.notify();
            }
        }
    }

    pub(in crate::features) fn open_transfer_editor_direct(
        &mut self,
        entry: SftpFileEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if entry.file_type == SftpFileType::Directory {
            self.terminal_status = "directories cannot be opened in the text editor".to_string();
            cx.notify();
            return;
        }
        self.transfer_selected_remote_path = Some(entry.path.clone());
        self.transfer_remote_path = entry.path.clone();
        self.transfer_editor = Some(TransferEditorState {
            remote_path: entry.path.clone(),
            name: entry.name.clone(),
            content: String::new(),
            search_query: String::new(),
            active_match: 0,
            base_size: entry.size,
            base_modified_at: entry.modified_at.map(u64::from),
            loading: true,
            saving: false,
            dirty: false,
            conflict: false,
            close_confirm: false,
            close_after_save: false,
            reload_confirm: false,
            error: None,
            focused_field: TransferEditorField::Content,
        });
        self.terminal_status = format!("opening remote text file {}", entry.path);
        window.focus(&self.transfer_editor_focus);
        self.start_sftp_editor_load_job(entry.path, window, cx);
        cx.notify();
    }

    pub(in crate::features) fn cancel_transfer_unknown_file(&mut self, cx: &mut Context<Self>) {
        self.transfer_unknown_file = None;
        self.terminal_status = "unknown file open cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn open_unknown_transfer_file_external(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer_unknown_file.take() else {
            cx.notify();
            return;
        };
        self.open_transfer_external(state.entry, window, cx);
    }

    pub(in crate::features) fn open_unknown_transfer_file_internal(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer_unknown_file.take() else {
            cx.notify();
            return;
        };
        self.open_transfer_editor_direct(state.entry, window, cx);
    }

    pub(in crate::features) fn open_transfer_external(
        &mut self,
        entry: SftpFileEntry,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if entry.file_type == SftpFileType::Directory {
            self.terminal_status = "directories cannot be opened in an external editor".to_string();
            cx.notify();
            return;
        }
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.ensure_panel_open(NavItem::Transfers);
            cx.notify();
            return;
        };

        self.transfer_selected_remote_path = Some(entry.path.clone());
        self.transfer_remote_path = entry.path.clone();
        let remote_path = entry.path.clone();
        let local_path = self.transfer_external_open_path(&entry);
        let default_editor = self.settings.transfer_default_editor.clone();
        let transfer_options = self.sftp_transfer_options();
        let id = self.next_transfer_id("sftp-open-external");
        let control = SftpTransferControl::new();
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::OpenExternal {
                remote_path: remote_path.clone(),
                local_path: local_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Opening {remote_path} externally"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: Some(control.clone()),
        });
        self.terminal_status = format!("downloading {remote_path} for external open");

        let progress_tx = self.transfer_tx.clone();
        let finished_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let progress_id = id.clone();
            let result = SftpService::new(config)
                .download_file_with_progress_and_control_options(
                    &remote_path,
                    local_path.clone(),
                    control,
                    transfer_options,
                    move |progress| {
                        let _ = progress_tx.send(TransferJobResult {
                            id: progress_id.clone(),
                            event: TransferJobEvent::Progress(progress),
                        });
                    },
                )
                .map_err(|error| error.to_string())
                .and_then(|_| {
                    open_local_path_with_editor(&local_path, &default_editor).map(|_| {
                        TransferJobOutput::ExternalOpened {
                            remote_path: remote_path.clone(),
                            local_path: local_path.clone(),
                        }
                    })
                });
            let opened = result.is_ok();
            let _ = finished_tx.send(TransferJobResult {
                id: id.clone(),
                event: TransferJobEvent::Finished(result),
            });
            if opened {
                watch_external_editor_file(id, remote_path, local_path, finished_tx);
            }
        });
        cx.notify();
    }

    pub(in crate::features) fn transfer_external_open_path(
        &self,
        entry: &SftpFileEntry,
    ) -> PathBuf {
        let session_id = self
            .active_session_id
            .as_deref()
            .map(sanitize_local_open_segment)
            .unwrap_or_else(|| "session".to_string());
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        let fallback_name;
        let raw_file_name = if entry.name.trim().is_empty() {
            fallback_name = remote_file_name(&entry.path);
            fallback_name.as_str()
        } else {
            entry.name.as_str()
        };
        let file_name = sanitize_local_open_segment(raw_file_name);
        std::env::temp_dir()
            .join("nyaterm")
            .join(session_id)
            .join(timestamp_ms.to_string())
            .join(file_name)
    }
}
