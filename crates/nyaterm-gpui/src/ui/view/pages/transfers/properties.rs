use super::*;

impl NyaTermApp {
    pub(super) fn open_current_transfer_browser_properties(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = normalized_transfer_browser_path(&self.transfer_browser_path);
        if path.trim().is_empty() {
            self.terminal_status = "open a remote directory first".to_string();
            cx.notify();
            return;
        }
        let name = remote_file_name(&path);
        let entry = SftpFileEntry {
            name: if name.is_empty() { path.clone() } else { name },
            path,
            file_type: SftpFileType::Directory,
            size: Some(0),
            permissions: None,
            owner: String::new(),
            group: String::new(),
            modified_at: None,
        };
        self.open_transfer_properties(entry, window, cx);
    }

    pub(super) fn open_selected_transfer_properties(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.selected_transfer_entry() else {
            self.terminal_status = "select a remote item first".to_string();
            cx.notify();
            return;
        };
        self.transfer_properties = Some(transfer_properties_state_from_entry(entry.clone()));
        self.terminal_status = "remote properties opened".to_string();
        window.focus(&self.transfer_properties_focus);
        self.start_sftp_properties_load_job(entry.path, window, cx);
        cx.notify();
    }

    pub(super) fn open_transfer_properties(
        &mut self,
        entry: SftpFileEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.transfer_selected_remote_path = Some(entry.path.clone());
        self.transfer_remote_path = entry.path.clone();
        self.transfer_properties = Some(transfer_properties_state_from_entry(entry.clone()));
        self.terminal_status = "remote properties opened".to_string();
        window.focus(&self.transfer_properties_focus);
        self.start_sftp_properties_load_job(entry.path, window, cx);
        cx.notify();
    }

    pub(super) fn close_transfer_properties(&mut self, cx: &mut Context<Self>) {
        self.transfer_properties = None;
        self.terminal_status = "remote properties closed".to_string();
        cx.notify();
    }

    pub(super) fn handle_transfer_properties_key_down(
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
            "escape" => self.close_transfer_properties(cx),
            "enter" => self.submit_transfer_properties(window, cx),
            "tab" => {
                if let Some(state) = self.transfer_properties.as_mut() {
                    state.focused_field = match state.focused_field {
                        TransferPropertiesField::Mode => TransferPropertiesField::Owner,
                        TransferPropertiesField::Owner => TransferPropertiesField::Group,
                        TransferPropertiesField::Group => TransferPropertiesField::Mode,
                    };
                }
                cx.notify();
            }
            "backspace" => {
                if let Some(state) = self.transfer_properties.as_mut() {
                    match state.focused_field {
                        TransferPropertiesField::Mode => {
                            state.mode_value.pop();
                        }
                        TransferPropertiesField::Owner => {
                            state.owner_value.pop();
                        }
                        TransferPropertiesField::Group => {
                            state.group_value.pop();
                        }
                    }
                    state.error = None;
                }
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                    && let Some(state) = self.transfer_properties.as_mut()
                {
                    match state.focused_field {
                        TransferPropertiesField::Mode => {
                            if input.chars().all(|value| ('0'..='7').contains(&value))
                                && state.mode_value.len() < 4
                            {
                                state.mode_value.push_str(input);
                            }
                        }
                        TransferPropertiesField::Owner => state.owner_value.push_str(input),
                        TransferPropertiesField::Group => state.group_value.push_str(input),
                    }
                    state.error = None;
                    cx.notify();
                }
            }
        }
    }

    pub(super) fn start_sftp_properties_load_job(
        &mut self,
        remote_path: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            cx.notify();
            return;
        };
        let id = self.next_transfer_id("sftp-properties");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::LoadProperties {
                remote_path: remote_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Loading properties for {remote_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.transfer_browser_status = format!("Loading properties for {remote_path}");
        let transfer_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let result = SftpService::new(config)
                .file_properties(&remote_path)
                .map(|properties| TransferJobOutput::PropertiesLoaded {
                    remote_path,
                    properties,
                })
                .map_err(|error| error.to_string());
            let _ = transfer_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }

    pub(super) fn submit_transfer_properties(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer_properties.clone() else {
            self.terminal_status = "no remote properties dialog is active".to_string();
            cx.notify();
            return;
        };
        let Some(properties) = state.properties.clone() else {
            self.terminal_status = "remote properties are still loading".to_string();
            cx.notify();
            return;
        };
        let mode = match parse_transfer_mode(&state.mode_value) {
            Some(mode) => mode,
            None => {
                if let Some(state) = self.transfer_properties.as_mut() {
                    state.error = Some("Mode must be a 3 or 4 digit octal value.".to_string());
                }
                cx.notify();
                return;
            }
        };
        let owner = state.owner_value.trim().to_string();
        let group = state.group_value.trim().to_string();
        if owner.is_empty() || group.is_empty() {
            if let Some(state) = self.transfer_properties.as_mut() {
                state.error = Some("Owner and group are required.".to_string());
            }
            cx.notify();
            return;
        }
        let initial_mode = properties
            .permissions
            .map(format_permissions_octal)
            .unwrap_or_else(|| "0644".to_string());
        let owner_changed =
            owner != properties.owner && properties.uid.is_none_or(|uid| owner != uid.to_string());
        let group_changed =
            group != properties.group && properties.gid.is_none_or(|gid| group != gid.to_string());
        let update = SftpAttributeUpdate {
            mode: (state.mode_value != initial_mode).then_some(mode),
            owner: owner_changed.then_some(owner),
            group: group_changed.then_some(group),
            recursive: state.recursive && properties.file_type == SftpFileType::Directory,
        };
        if update.mode.is_none() && update.owner.is_none() && update.group.is_none() {
            self.close_transfer_properties(cx);
            return;
        }
        if let Some(state) = self.transfer_properties.as_mut() {
            state.saving = true;
            state.error = None;
        }
        self.start_sftp_properties_update_job(
            properties.path,
            remote_parent_path(&state.entry.path),
            update,
            window,
            cx,
        );
    }

    pub(super) fn start_sftp_properties_update_job(
        &mut self,
        remote_path: String,
        parent_path: String,
        update: SftpAttributeUpdate,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            cx.notify();
            return;
        };
        let id = self.next_transfer_id("sftp-update-properties");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::UpdateProperties {
                remote_path: remote_path.clone(),
                parent_path: parent_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Updating properties for {remote_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.transfer_browser_status = format!("Updating properties for {remote_path}");
        let transfer_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let service = SftpService::new(config);
                service.update_path_attributes(&remote_path, update)?;
                let properties = service.file_properties(&remote_path)?;
                let entries = service.list_dir(&parent_path)?;
                Ok(TransferJobOutput::PropertiesUpdated {
                    remote_path,
                    parent_path,
                    properties,
                    entries,
                })
            })()
            .map_err(|error: anyhow::Error| error.to_string());
            let _ = transfer_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }
}
