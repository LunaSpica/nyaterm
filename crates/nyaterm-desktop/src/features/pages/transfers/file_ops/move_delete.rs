use super::*;

impl NyaTermApp {
    pub(in crate::features) fn open_transfer_move_dialog(
        &mut self,
        old_path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = remote_file_name(&old_path);
        if old_path.trim().is_empty() || old_path == "/" || name == "." || name == ".." {
            self.terminal.view.status = format!("cannot move {old_path}");
            cx.notify();
            return;
        }
        self.transfer.file_ops.move_to = Some(TransferMoveState {
            old_path: old_path.clone(),
            name,
            value: old_path,
        });
        self.terminal.view.status = "SFTP move opened".to_string();
        window.focus(&self.transfer.file_ops.move_focus);
        cx.notify();
    }

    pub(in crate::features) fn open_selected_transfer_move_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(old_path) = self.transfer.browser.selected_remote_path.clone() else {
            self.terminal.view.status = "select a remote item before moving".to_string();
            cx.notify();
            return;
        };
        self.open_transfer_move_dialog(old_path, window, cx);
    }

    pub(in crate::features) fn close_transfer_move_dialog(&mut self, cx: &mut Context<Self>) {
        self.transfer.file_ops.move_to = None;
        self.terminal.view.status = "SFTP move cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn submit_transfer_move(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer.file_ops.move_to.clone() else {
            self.terminal.view.status = "no SFTP move is active".to_string();
            cx.notify();
            return;
        };
        let new_path = state.value.trim().to_string();
        if new_path.is_empty() {
            self.terminal.view.status = "target path cannot be empty".to_string();
            cx.notify();
            return;
        }
        if new_path == state.old_path {
            self.transfer.file_ops.move_to = None;
            self.terminal.view.status = "SFTP move unchanged".to_string();
            cx.notify();
            return;
        }
        self.transfer.file_ops.move_to = None;
        self.start_sftp_move_job(state.old_path, new_path, window, cx);
    }

    pub(in crate::features) fn handle_transfer_move_key_down(
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
            "escape" => self.close_transfer_move_dialog(cx),
            "enter" => self.submit_transfer_move(window, cx),
            "backspace" => {
                if let Some(state) = self.transfer.file_ops.move_to.as_mut() {
                    state.value.pop();
                    cx.notify();
                }
            }
            _ => {
                let Some(state) = self.transfer.file_ops.move_to.as_mut() else {
                    return;
                };
                if state.value.chars().count() >= 1024 {
                    return;
                }
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    let remaining = 1024usize.saturating_sub(state.value.chars().count());
                    state.value.extend(input.chars().take(remaining));
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn start_sftp_move_job(
        &mut self,
        old_path: String,
        new_path: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal.view.status = "start an SSH session first".to_string();
            self.ensure_panel_open(crate::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        let parent_path = remote_parent_path(&old_path);
        let id = self.next_transfer_id("sftp-move");
        self.transfer.queue.jobs.push(TransferJobState {
            id: id.clone(),
            session_id: self.active_session_id.clone(),
            kind: TransferJobKind::Move {
                old_path: old_path.clone(),
                new_path: new_path.clone(),
                parent_path: parent_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Moving {old_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.terminal.view.status = format!("SFTP move started: {old_path} -> {new_path}");
        let transfer_tx = self.transfer.queue.tx.clone();
        std::thread::spawn(move || {
            let service = SftpService::new(config);
            let result = service
                .rename_path(&old_path, &new_path)
                .and_then(|_| service.list_dir(&parent_path))
                .map(|entries| TransferJobOutput::Moved {
                    old_path,
                    new_path,
                    parent_path,
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

    pub(in crate::features) fn open_selected_transfer_delete_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let paths = self
            .selected_transfer_entries()
            .into_iter()
            .map(|entry| entry.path)
            .filter(|path| {
                let name = remote_file_name(path);
                !path.trim().is_empty() && path != "/" && name != "." && name != ".."
            })
            .collect::<Vec<_>>();
        if paths.is_empty() {
            self.terminal.view.status = "mark remote items before deleting".to_string();
            cx.notify();
            return;
        }
        let remote_path = paths.first().cloned().unwrap_or_default();
        let name = if paths.len() == 1 {
            remote_file_name(&remote_path)
        } else {
            format!("{} remote items", paths.len())
        };
        self.transfer.file_ops.delete = Some(TransferDeleteState {
            remote_path,
            name,
            paths,
        });
        self.terminal.view.status = "SFTP delete confirmation opened".to_string();
        window.focus(&self.transfer.file_ops.delete_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_transfer_delete_dialog(&mut self, cx: &mut Context<Self>) {
        self.transfer.file_ops.delete = None;
        self.terminal.view.status = "SFTP delete cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn submit_transfer_delete(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer.file_ops.delete.take() else {
            self.terminal.view.status = "no SFTP delete is active".to_string();
            cx.notify();
            return;
        };
        let total = state.paths.len();
        for remote_path in state.paths {
            self.start_sftp_delete_job(remote_path, window, cx);
        }
        self.terminal.view.status = format!("{total} SFTP delete job(s) started");
        cx.notify();
    }

    pub(in crate::features) fn handle_transfer_delete_key_down(
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
            "escape" => self.close_transfer_delete_dialog(cx),
            "enter" => self.submit_transfer_delete(window, cx),
            _ => {}
        }
    }

    pub(in crate::features) fn start_sftp_delete_job(
        &mut self,
        remote_path: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal.view.status = "start an SSH session first".to_string();
            self.ensure_panel_open(crate::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        let parent_path = remote_parent_path(&remote_path);
        let id = self.next_transfer_id("sftp-delete");
        self.transfer.queue.jobs.push(TransferJobState {
            id: id.clone(),
            session_id: self.active_session_id.clone(),
            kind: TransferJobKind::Delete {
                remote_path: remote_path.clone(),
                parent_path: parent_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Deleting {remote_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.terminal.view.status = format!("SFTP delete started: {remote_path}");
        let transfer_tx = self.transfer.queue.tx.clone();
        std::thread::spawn(move || {
            let service = SftpService::new(config);
            let result = service
                .delete_path(&remote_path)
                .and_then(|_| service.list_dir(&parent_path))
                .map(|entries| TransferJobOutput::Deleted {
                    remote_path,
                    parent_path,
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
}
