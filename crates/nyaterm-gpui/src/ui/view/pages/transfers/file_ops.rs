use super::*;

impl NyaTermApp {
    pub(super) fn open_transfer_new_folder_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let parent_path = if self.transfer_browser_path.trim().is_empty() {
            self.normalized_transfer_remote_path()
        } else {
            self.transfer_browser_path.clone()
        };
        self.transfer_new_folder = Some(TransferNewFolderState {
            parent_path,
            value: "New Folder".to_string(),
            mode: 0o755,
            open_after_create: false,
        });
        self.terminal_status = "SFTP new folder opened".to_string();
        window.focus(&self.transfer_new_folder_focus);
        cx.notify();
    }

    pub(super) fn close_transfer_new_folder_dialog(&mut self, cx: &mut Context<Self>) {
        self.transfer_new_folder = None;
        self.terminal_status = "SFTP new folder cancelled".to_string();
        cx.notify();
    }

    pub(super) fn submit_transfer_new_folder(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer_new_folder.clone() else {
            self.terminal_status = "no SFTP new folder is active".to_string();
            cx.notify();
            return;
        };
        let name = state.value.trim().to_string();
        if !valid_remote_child_name(&name) {
            self.terminal_status = "folder name must be a single non-empty name".to_string();
            cx.notify();
            return;
        }
        self.transfer_new_folder = None;
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

    pub(super) fn handle_transfer_new_folder_key_down(
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
                if let Some(state) = self.transfer_new_folder.as_mut() {
                    state.value.pop();
                    cx.notify();
                }
            }
            _ => {
                let Some(state) = self.transfer_new_folder.as_mut() else {
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

    pub(super) fn start_sftp_mkdir_job(
        &mut self,
        remote_path: String,
        parent_path: String,
        mode: u32,
        open_after_create: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.ensure_panel_open(crate::ui::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        self.ensure_event_pump(window, cx);
        let id = self.next_transfer_id("sftp-mkdir");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
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
        self.terminal_status = format!("SFTP create folder started: {remote_path}");
        let transfer_tx = self.transfer_tx.clone();
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

    pub(super) fn open_transfer_new_file_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let parent_path = if self.transfer_browser_path.trim().is_empty() {
            self.normalized_transfer_remote_path()
        } else {
            self.transfer_browser_path.clone()
        };
        self.transfer_new_file = Some(TransferNewFileState {
            parent_path,
            value: "new-file.txt".to_string(),
            mode: 0o644,
        });
        self.terminal_status = "SFTP new file opened".to_string();
        window.focus(&self.transfer_new_file_focus);
        cx.notify();
    }

    pub(super) fn close_transfer_new_file_dialog(&mut self, cx: &mut Context<Self>) {
        self.transfer_new_file = None;
        self.terminal_status = "SFTP new file cancelled".to_string();
        cx.notify();
    }

    pub(super) fn submit_transfer_new_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.transfer_new_file.clone() else {
            self.terminal_status = "no SFTP new file is active".to_string();
            cx.notify();
            return;
        };
        let name = state.value.trim().to_string();
        if !valid_remote_child_name(&name) {
            self.terminal_status = "file name must be a single non-empty name".to_string();
            cx.notify();
            return;
        }
        self.transfer_new_file = None;
        let remote_path = remote_child_path(&state.parent_path, &name);
        self.start_sftp_create_file_job(remote_path, state.parent_path, state.mode, window, cx);
    }

    pub(super) fn handle_transfer_new_file_key_down(
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
                if let Some(state) = self.transfer_new_file.as_mut() {
                    state.value.pop();
                    cx.notify();
                }
            }
            _ => {
                let Some(state) = self.transfer_new_file.as_mut() else {
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

    pub(super) fn start_sftp_create_file_job(
        &mut self,
        remote_path: String,
        parent_path: String,
        mode: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.ensure_panel_open(crate::ui::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        self.ensure_event_pump(window, cx);
        let id = self.next_transfer_id("sftp-create-file");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
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
        self.terminal_status = format!("SFTP create file started: {remote_path}");
        let transfer_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let service = SftpService::new(config);
            let result = service
                .create_file_path(&remote_path, Some(mode))
                .and_then(|_| service.list_dir(&parent_path))
                .map(|entries| TransferJobOutput::CreatedFile {
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

    pub(super) fn open_transfer_new_symlink_dialog(
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

    pub(super) fn close_transfer_new_symlink_dialog(&mut self, cx: &mut Context<Self>) {
        self.transfer_new_symlink = None;
        self.terminal_status = "SFTP new symlink cancelled".to_string();
        cx.notify();
    }

    pub(super) fn submit_transfer_new_symlink(
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

    pub(super) fn handle_transfer_new_symlink_key_down(
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

    pub(super) fn start_sftp_symlink_job(
        &mut self,
        link_path: String,
        target_path: String,
        parent_path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.ensure_panel_open(crate::ui::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        self.ensure_event_pump(window, cx);
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

    pub(in crate::ui::view) fn open_transfer_rename_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.ensure_panel_open(crate::ui::models::NavItem::Transfers);
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

    pub(super) fn open_transfer_rename_for_path_after_delay(
        &mut self,
        old_path: String,
        cx: &mut Context<Self>,
    ) {
        if self.open_transfer_rename_for_path(old_path, cx) {
            self.transfer_rename_focus_pending = true;
            cx.notify();
        }
    }

    fn open_transfer_rename_for_path(&mut self, old_path: String, cx: &mut Context<Self>) -> bool {
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

    pub(super) fn close_transfer_rename_dialog(&mut self, cx: &mut Context<Self>) {
        self.transfer_rename = None;
        self.transfer_rename_focus_pending = false;
        self.terminal_status = "SFTP rename cancelled".to_string();
        cx.notify();
    }

    pub(super) fn submit_transfer_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn handle_transfer_rename_key_down(
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

    pub(super) fn start_sftp_rename_job(
        &mut self,
        old_path: String,
        new_path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.ensure_panel_open(crate::ui::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        let parent_path = remote_parent_path(&old_path);
        self.ensure_event_pump(window, cx);
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

    pub(super) fn open_transfer_move_dialog(
        &mut self,
        old_path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = remote_file_name(&old_path);
        if old_path.trim().is_empty() || old_path == "/" || name == "." || name == ".." {
            self.terminal_status = format!("cannot move {old_path}");
            cx.notify();
            return;
        }
        self.transfer_move = Some(TransferMoveState {
            old_path: old_path.clone(),
            name,
            value: old_path,
        });
        self.terminal_status = "SFTP move opened".to_string();
        window.focus(&self.transfer_move_focus);
        cx.notify();
    }

    pub(super) fn open_selected_transfer_move_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(old_path) = self.transfer_selected_remote_path.clone() else {
            self.terminal_status = "select a remote item before moving".to_string();
            cx.notify();
            return;
        };
        self.open_transfer_move_dialog(old_path, window, cx);
    }

    pub(super) fn close_transfer_move_dialog(&mut self, cx: &mut Context<Self>) {
        self.transfer_move = None;
        self.terminal_status = "SFTP move cancelled".to_string();
        cx.notify();
    }

    pub(super) fn submit_transfer_move(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.transfer_move.clone() else {
            self.terminal_status = "no SFTP move is active".to_string();
            cx.notify();
            return;
        };
        let new_path = state.value.trim().to_string();
        if new_path.is_empty() {
            self.terminal_status = "target path cannot be empty".to_string();
            cx.notify();
            return;
        }
        if new_path == state.old_path {
            self.transfer_move = None;
            self.terminal_status = "SFTP move unchanged".to_string();
            cx.notify();
            return;
        }
        self.transfer_move = None;
        self.start_sftp_move_job(state.old_path, new_path, window, cx);
    }

    pub(super) fn handle_transfer_move_key_down(
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
                if let Some(state) = self.transfer_move.as_mut() {
                    state.value.pop();
                    cx.notify();
                }
            }
            _ => {
                let Some(state) = self.transfer_move.as_mut() else {
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

    pub(super) fn start_sftp_move_job(
        &mut self,
        old_path: String,
        new_path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.ensure_panel_open(crate::ui::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        let parent_path = remote_parent_path(&old_path);
        self.ensure_event_pump(window, cx);
        let id = self.next_transfer_id("sftp-move");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
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
        self.terminal_status = format!("SFTP move started: {old_path} -> {new_path}");
        let transfer_tx = self.transfer_tx.clone();
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

    pub(super) fn open_selected_transfer_delete_dialog(
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
            self.terminal_status = "mark remote items before deleting".to_string();
            cx.notify();
            return;
        }
        let remote_path = paths.first().cloned().unwrap_or_default();
        let name = if paths.len() == 1 {
            remote_file_name(&remote_path)
        } else {
            format!("{} remote items", paths.len())
        };
        self.transfer_delete = Some(TransferDeleteState {
            remote_path,
            name,
            paths,
        });
        self.terminal_status = "SFTP delete confirmation opened".to_string();
        window.focus(&self.transfer_delete_focus);
        cx.notify();
    }

    pub(super) fn close_transfer_delete_dialog(&mut self, cx: &mut Context<Self>) {
        self.transfer_delete = None;
        self.terminal_status = "SFTP delete cancelled".to_string();
        cx.notify();
    }

    pub(super) fn submit_transfer_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.transfer_delete.take() else {
            self.terminal_status = "no SFTP delete is active".to_string();
            cx.notify();
            return;
        };
        let total = state.paths.len();
        for remote_path in state.paths {
            self.start_sftp_delete_job(remote_path, window, cx);
        }
        self.terminal_status = format!("{total} SFTP delete job(s) started");
        cx.notify();
    }

    pub(super) fn handle_transfer_delete_key_down(
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

    pub(super) fn start_sftp_delete_job(
        &mut self,
        remote_path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.ensure_panel_open(crate::ui::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        let parent_path = remote_parent_path(&remote_path);
        self.ensure_event_pump(window, cx);
        let id = self.next_transfer_id("sftp-delete");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
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
        self.terminal_status = format!("SFTP delete started: {remote_path}");
        let transfer_tx = self.transfer_tx.clone();
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
