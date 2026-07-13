use super::*;

impl NyaTermApp {
    pub(in crate::features) fn open_transfer_new_symlink_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let parent_path = if self.transfer_browser_path.trim().is_empty() {
            self.normalized_transfer_remote_path()
        } else {
            self.transfer_browser_path.clone()
        };
        self.transfer_new_symlink = Some(TransferNewSymlinkState {
            parent_path,
            name: "new-link".to_string(),
            target: String::new(),
            focused_field: TransferSymlinkField::Name,
        });
        self.terminal_status = "SFTP new symlink opened".to_string();
        window.focus(&self.transfer_new_symlink_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_transfer_new_symlink_dialog(&mut self, cx: &mut Context<Self>) {
        self.transfer_new_symlink = None;
        self.terminal_status = "SFTP new symlink cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn submit_transfer_new_symlink(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer_new_symlink.clone() else {
            self.terminal_status = "no SFTP new symlink is active".to_string();
            cx.notify();
            return;
        };
        let name = state.name.trim().to_string();
        let target_path = state.target.trim().to_string();
        if !valid_remote_child_name(&name) {
            self.terminal_status = "symlink name must be a single non-empty name".to_string();
            cx.notify();
            return;
        }
        if target_path.is_empty() {
            self.terminal_status = "symlink target cannot be empty".to_string();
            cx.notify();
            return;
        }
        self.transfer_new_symlink = None;
        let link_path = remote_child_path(&state.parent_path, &name);
        self.start_sftp_symlink_job(link_path, target_path, state.parent_path, window, cx);
    }

    pub(in crate::features) fn handle_transfer_new_symlink_key_down(
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
            "escape" => self.close_transfer_new_symlink_dialog(cx),
            "enter" => self.submit_transfer_new_symlink(window, cx),
            "tab" => {
                if let Some(state) = self.transfer_new_symlink.as_mut() {
                    state.focused_field = match state.focused_field {
                        TransferSymlinkField::Name => TransferSymlinkField::Target,
                        TransferSymlinkField::Target => TransferSymlinkField::Name,
                    };
                    cx.notify();
                }
            }
            "backspace" => {
                if let Some(state) = self.transfer_new_symlink.as_mut() {
                    match state.focused_field {
                        TransferSymlinkField::Name => {
                            state.name.pop();
                        }
                        TransferSymlinkField::Target => {
                            state.target.pop();
                        }
                    }
                    cx.notify();
                }
            }
            _ => {
                let Some(state) = self.transfer_new_symlink.as_mut() else {
                    return;
                };
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    match state.focused_field {
                        TransferSymlinkField::Name if state.name.chars().count() < 255 => {
                            let remaining = 255usize.saturating_sub(state.name.chars().count());
                            state.name.extend(input.chars().take(remaining));
                        }
                        TransferSymlinkField::Target if state.target.chars().count() < 1024 => {
                            let remaining = 1024usize.saturating_sub(state.target.chars().count());
                            state.target.extend(input.chars().take(remaining));
                        }
                        _ => {}
                    }
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn start_sftp_symlink_job(
        &mut self,
        link_path: String,
        target_path: String,
        parent_path: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.ensure_panel_open(crate::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        let id = self.next_transfer_id("sftp-symlink");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::Symlink {
                link_path: link_path.clone(),
                target_path: target_path.clone(),
                parent_path: parent_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Linking {link_path} -> {target_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.terminal_status = format!("SFTP symlink started: {link_path}");
        let transfer_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let service = SftpService::new(config);
            let result = service
                .create_symlink_path(&link_path, &target_path)
                .and_then(|_| service.list_dir(&parent_path))
                .map(|entries| TransferJobOutput::CreatedSymlink {
                    link_path,
                    target_path,
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

    pub(in crate::features) fn open_transfer_rename_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.ensure_panel_open(crate::models::NavItem::Transfers);
        let Some(old_path) = self.transfer_selected_remote_path.clone() else {
            self.terminal_status = "select an SFTP list entry before renaming".to_string();
            cx.notify();
            return;
        };
        if !self.open_transfer_rename_for_path(old_path, cx) {
            return;
        }
        window.focus(&self.transfer_rename_focus);
        cx.notify();
    }

    pub(in crate::features) fn open_transfer_rename_for_path_after_delay(
        &mut self,
        old_path: String,
        cx: &mut Context<Self>,
    ) {
        if self.open_transfer_rename_for_path(old_path, cx) {
            self.transfer_rename_focus_pending = true;
            cx.notify();
        }
    }

    pub(in crate::features) fn open_transfer_rename_for_path(&mut self, old_path: String, cx: &mut Context<Self>) -> bool {
        let initial_name = remote_file_name(&old_path);
        if initial_name.is_empty() || initial_name == "." || initial_name == ".." {
            self.terminal_status = format!("cannot rename {old_path}");
            cx.notify();
            return false;
        }
        self.transfer_rename = Some(TransferRenameState {
            old_path,
            value: initial_name.clone(),
            initial_name,
        });
        self.terminal_status = "SFTP rename opened".to_string();
        true
    }

    pub(in crate::features) fn close_transfer_rename_dialog(&mut self, cx: &mut Context<Self>) {
        self.transfer_rename = None;
        self.transfer_rename_focus_pending = false;
        self.terminal_status = "SFTP rename cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn submit_transfer_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.transfer_rename.clone() else {
            self.terminal_status = "no SFTP rename is active".to_string();
            cx.notify();
            return;
        };
        let new_name = state.value.trim().to_string();
        if new_name.is_empty() {
            self.terminal_status = "remote name cannot be empty".to_string();
            cx.notify();
            return;
        }
        if new_name.contains('/') || new_name == "." || new_name == ".." {
            self.terminal_status =
                "remote name must be a single file or directory name".to_string();
            cx.notify();
            return;
        }
        if new_name == state.initial_name {
            self.transfer_rename = None;
            self.terminal_status = "SFTP rename unchanged".to_string();
            cx.notify();
            return;
        }
        let new_path = remote_sibling_path(&state.old_path, &new_name);
        self.transfer_rename = None;
        self.start_sftp_rename_job(state.old_path, new_path, window, cx);
    }

    pub(in crate::features) fn handle_transfer_rename_key_down(
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
            "escape" => self.close_transfer_rename_dialog(cx),
            "enter" => self.submit_transfer_rename(window, cx),
            "backspace" => {
                if let Some(state) = self.transfer_rename.as_mut() {
                    state.value.pop();
                    cx.notify();
                }
            }
            _ => {
                let Some(state) = self.transfer_rename.as_mut() else {
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

    pub(in crate::features) fn start_sftp_rename_job(
        &mut self,
        old_path: String,
        new_path: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.ensure_panel_open(crate::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        let parent_path = remote_parent_path(&old_path);
        let id = self.next_transfer_id("sftp-rename");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::Rename {
                old_path: old_path.clone(),
                new_path: new_path.clone(),
                parent_path: parent_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Renaming {old_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.terminal_status = format!("SFTP rename started: {old_path} -> {new_path}");
        let transfer_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let service = SftpService::new(config);
            let result = service
                .rename_path(&old_path, &new_path)
                .and_then(|_| service.list_dir(&parent_path))
                .map(|entries| TransferJobOutput::Renamed {
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

}
