use super::*;

impl NyaTermApp {
    pub(in crate::features) fn open_transfer_new_folder_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let parent_path = if self.transfer.browser.path.trim().is_empty() {
            self.normalized_transfer_remote_path()
        } else {
            self.transfer.browser.path.clone()
        };
        self.transfer.file_ops.new_folder = Some(TransferNewFolderState {
            parent_path,
            value: String::new(),
            mode: 0o755,
            open_after_create: false,
        });
        self.terminal.view.status = "SFTP new folder opened".to_string();
        window.focus(&self.transfer.file_ops.new_folder_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_transfer_new_folder_dialog(&mut self, cx: &mut Context<Self>) {
        self.transfer.file_ops.new_folder = None;
        self.terminal.view.status = "SFTP new folder cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn submit_transfer_new_folder(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer.file_ops.new_folder.clone() else {
            self.terminal.view.status = "no SFTP new folder is active".to_string();
            cx.notify();
            return;
        };
        let name = state.value.trim().to_string();
        if !valid_remote_child_name(&name) {
            self.terminal.view.status = "folder name must be a single non-empty name".to_string();
            cx.notify();
            return;
        }
        self.transfer.file_ops.new_folder = None;
        let remote_path = remote_child_path(&state.parent_path, &name);
        self.start_sftp_mkdir_job(
            remote_path,
            state.parent_path,
            state.mode,
            state.open_after_create,
            window,
            cx,
        );
    }

    pub(in crate::features) fn handle_transfer_new_folder_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "escape" => self.close_transfer_new_folder_dialog(cx),
            "enter" => self.submit_transfer_new_folder(window, cx),
            "backspace" => {
                if let Some(state) = self.transfer.file_ops.new_folder.as_mut() {
                    state.value.pop();
                    cx.notify();
                }
            }
            _ => {
                let Some(state) = self.transfer.file_ops.new_folder.as_mut() else {
                    return;
                };
                if state.value.chars().count() >= 255 {
                    return;
                }
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    let remaining = 255usize.saturating_sub(state.value.chars().count());
                    state.value.extend(input.chars().take(remaining));
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn start_sftp_mkdir_job(
        &mut self,
        remote_path: String,
        parent_path: String,
        mode: u32,
        open_after_create: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal.view.status = "start an SSH session first".to_string();
            self.ensure_panel_open(crate::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        let id = self.next_transfer_id("sftp-mkdir");
        self.transfer.queue.jobs.push(TransferJobState {
            id: id.clone(),
            session_id: self.active_session_id.clone(),
            kind: TransferJobKind::Mkdir {
                remote_path: remote_path.clone(),
                parent_path: parent_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Creating {remote_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.terminal.view.status = format!("SFTP create folder started: {remote_path}");
        let transfer_tx = self.transfer.queue.tx.clone();
        std::thread::spawn(move || {
            let service = SftpService::new(config);
            let list_path = if open_after_create {
                remote_path.clone()
            } else {
                parent_path.clone()
            };
            let result = service
                .create_dir_path(&remote_path, Some(mode))
                .and_then(|_| service.list_dir(&list_path))
                .map(|entries| TransferJobOutput::CreatedDirectory {
                    remote_path,
                    parent_path,
                    entries,
                    open_after_create,
                })
                .map_err(|error| error.to_string());
            let _ = transfer_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn open_transfer_new_file_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let parent_path = if self.transfer.browser.path.trim().is_empty() {
            self.normalized_transfer_remote_path()
        } else {
            self.transfer.browser.path.clone()
        };
        self.transfer.file_ops.new_file = Some(TransferNewFileState {
            parent_path,
            value: String::new(),
            mode: 0o644,
            open_after_create: false,
        });
        self.terminal.view.status = "SFTP new file opened".to_string();
        window.focus(&self.transfer.file_ops.new_file_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_transfer_new_file_dialog(&mut self, cx: &mut Context<Self>) {
        self.transfer.file_ops.new_file = None;
        self.terminal.view.status = "SFTP new file cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn submit_transfer_new_file(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer.file_ops.new_file.clone() else {
            self.terminal.view.status = "no SFTP new file is active".to_string();
            cx.notify();
            return;
        };
        let name = state.value.trim().to_string();
        if !valid_remote_child_name(&name) {
            self.terminal.view.status = "file name must be a single non-empty name".to_string();
            cx.notify();
            return;
        }
        self.transfer.file_ops.new_file = None;
        let remote_path = remote_child_path(&state.parent_path, &name);
        self.start_sftp_create_file_job(
            remote_path,
            state.parent_path,
            state.mode,
            state.open_after_create,
            window,
            cx,
        );
    }

    pub(in crate::features) fn handle_transfer_new_file_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "escape" => self.close_transfer_new_file_dialog(cx),
            "enter" => self.submit_transfer_new_file(window, cx),
            "backspace" => {
                if let Some(state) = self.transfer.file_ops.new_file.as_mut() {
                    state.value.pop();
                    cx.notify();
                }
            }
            _ => {
                let Some(state) = self.transfer.file_ops.new_file.as_mut() else {
                    return;
                };
                if state.value.chars().count() >= 255 {
                    return;
                }
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    let remaining = 255usize.saturating_sub(state.value.chars().count());
                    state.value.extend(input.chars().take(remaining));
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn start_sftp_create_file_job(
        &mut self,
        remote_path: String,
        parent_path: String,
        mode: u32,
        open_after_create: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal.view.status = "start an SSH session first".to_string();
            self.ensure_panel_open(crate::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        let id = self.next_transfer_id("sftp-create-file");
        self.transfer.queue.jobs.push(TransferJobState {
            id: id.clone(),
            session_id: self.active_session_id.clone(),
            kind: TransferJobKind::CreateFile {
                remote_path: remote_path.clone(),
                parent_path: parent_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Creating {remote_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.terminal.view.status = format!("SFTP create file started: {remote_path}");
        let transfer_tx = self.transfer.queue.tx.clone();
        std::thread::spawn(move || {
            let service = SftpService::new(config);
            let result = service
                .create_file_path(&remote_path, Some(mode))
                .and_then(|_| service.list_dir(&parent_path))
                .map(|entries| TransferJobOutput::CreatedFile {
                    remote_path,
                    parent_path,
                    entries,
                    open_after_create,
                })
                .map_err(|error| error.to_string());
            let _ = transfer_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }
}
