use gpui::{Context, KeyDownEvent, Window};
use nyaterm_transport::{SftpAttributeUpdate, SftpFileEntry, SftpFileType, SftpService};

use crate::features::{NyaTermApp, TextInputSetup};
use crate::models::{
    TransferJobEvent, TransferJobKind, TransferJobOutput, TransferJobResult, TransferJobState,
    TransferJobStatus, TransferPropertiesField,
};

use super::{
    format_permissions_octal, normalized_transfer_browser_path, parse_transfer_mode,
    remote_file_name, remote_parent_path, transfer_properties_state_from_entry,
};

impl NyaTermApp {
    pub(super) fn open_current_transfer_browser_properties(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = normalized_transfer_browser_path(&self.transfer.browser.path);
        if path.trim().is_empty() {
            self.terminal.view.status = "open a remote directory first".to_string();
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
            self.terminal.view.status = "select a remote item first".to_string();
            cx.notify();
            return;
        };
        self.forget_text_inputs("transfer.properties.");
        self.transfer.file_ops.properties = Some(transfer_properties_state_from_entry(
            entry.clone(),
            self.session.active_id.clone(),
        ));
        self.terminal.view.status = "remote properties opened".to_string();
        window.focus(&self.transfer.file_ops.properties_focus);
        self.start_sftp_properties_load_job(entry.path, window, cx);
        cx.notify();
    }

    pub(super) fn open_transfer_properties(
        &mut self,
        entry: SftpFileEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.transfer.browser.selected_remote_path = Some(entry.path.clone());
        self.transfer.paths.remote = entry.path.clone();
        self.forget_text_inputs("transfer.properties.");
        self.transfer.file_ops.properties = Some(transfer_properties_state_from_entry(
            entry.clone(),
            self.session.active_id.clone(),
        ));
        self.terminal.view.status = "remote properties opened".to_string();
        window.focus(&self.transfer.file_ops.properties_focus);
        self.start_sftp_properties_load_job(entry.path, window, cx);
        cx.notify();
    }

    pub(super) fn close_transfer_properties(&mut self, cx: &mut Context<Self>) {
        self.transfer.file_ops.properties = None;
        self.forget_text_inputs("transfer.properties.");
        self.terminal.view.status = "remote properties closed".to_string();
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
                if let Some(field) = self.transfer.file_ops.properties.as_ref().map(|state| {
                    match state.focused_field {
                        TransferPropertiesField::Mode => TransferPropertiesField::Owner,
                        TransferPropertiesField::Owner => TransferPropertiesField::Group,
                        TransferPropertiesField::Group => TransferPropertiesField::Mode,
                    }
                }) {
                    self.focus_transfer_properties_field(field, window, cx);
                }
            }
            _ => {}
        }
    }

    pub(super) fn focus_transfer_properties_field(
        &mut self,
        field: TransferPropertiesField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer.file_ops.properties.as_mut() else {
            return;
        };
        state.focused_field = field;
        let (id, value, placeholder) = match field {
            TransferPropertiesField::Mode => {
                ("transfer.properties.mode", state.mode_value.clone(), "0644")
            }
            TransferPropertiesField::Owner => (
                "transfer.properties.owner",
                state.owner_value.clone(),
                self.tr("fileExplorer.owner"),
            ),
            TransferPropertiesField::Group => (
                "transfer.properties.group",
                state.group_value.clone(),
                self.tr("fileExplorer.group"),
            ),
        };
        let input = self.text_input(id, &value, TextInputSetup::placeholder(placeholder), cx);
        window.focus(&input.read(cx).focus_handle());
        cx.notify();
    }

    pub(in crate::features) fn apply_transfer_properties_input(
        &mut self,
        field_id: &str,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let Some(field) = (match field_id {
            "mode" => Some(TransferPropertiesField::Mode),
            "owner" => Some(TransferPropertiesField::Owner),
            "group" => Some(TransferPropertiesField::Group),
            _ => None,
        }) else {
            return;
        };
        let filtered = normalize_transfer_properties_input(field, &text);
        let Some(state) = self.transfer.file_ops.properties.as_mut() else {
            return;
        };
        match field {
            TransferPropertiesField::Mode => state.mode_value = filtered.clone(),
            TransferPropertiesField::Owner => state.owner_value = filtered.clone(),
            TransferPropertiesField::Group => state.group_value = filtered.clone(),
        }
        state.focused_field = field;
        state.error = None;
        if filtered != text {
            self.reset_text_input(&format!("transfer.properties.{field_id}"), &filtered, cx);
        }
        cx.notify();
    }

    pub(in crate::features) fn sync_transfer_properties_inputs(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.transfer.file_ops.properties.as_ref() else {
            return;
        };
        let mode = state.mode_value.clone();
        let owner = state.owner_value.clone();
        let group = state.group_value.clone();
        self.reset_text_input("transfer.properties.mode", &mode, cx);
        self.reset_text_input("transfer.properties.owner", &owner, cx);
        self.reset_text_input("transfer.properties.group", &group, cx);
    }

    pub(super) fn start_sftp_properties_load_job(
        &mut self,
        remote_path: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config.clone() else {
            self.terminal.view.status = "start an SSH session first".to_string();
            cx.notify();
            return;
        };
        let id = self.next_transfer_id("sftp-properties");
        self.transfer.queue.jobs.push(TransferJobState {
            id: id.clone(),
            session_id: self.session.active_id.clone(),
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
        self.transfer.browser.status = format!("Loading properties for {remote_path}");
        let transfer_tx = self.transfer.queue.tx.clone();
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
        let Some(state) = self.transfer.file_ops.properties.clone() else {
            self.terminal.view.status = "no remote properties dialog is active".to_string();
            cx.notify();
            return;
        };
        let Some(properties) = state.properties.clone() else {
            self.terminal.view.status = "remote properties are still loading".to_string();
            cx.notify();
            return;
        };
        let mode = match parse_transfer_mode(&state.mode_value) {
            Some(mode) => mode,
            None => {
                if let Some(state) = self.transfer.file_ops.properties.as_mut() {
                    state.error = Some("Mode must be a 3 or 4 digit octal value.".to_string());
                }
                cx.notify();
                return;
            }
        };
        let owner = state.owner_value.trim().to_string();
        let group = state.group_value.trim().to_string();
        if owner.is_empty() || group.is_empty() {
            if let Some(state) = self.transfer.file_ops.properties.as_mut() {
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
        if let Some(state) = self.transfer.file_ops.properties.as_mut() {
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
        let Some(config) = self.session.active_ssh_config.clone() else {
            self.terminal.view.status = "start an SSH session first".to_string();
            cx.notify();
            return;
        };
        let id = self.next_transfer_id("sftp-update-properties");
        self.transfer.queue.jobs.push(TransferJobState {
            id: id.clone(),
            session_id: self.session.active_id.clone(),
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
        self.transfer.browser.status = format!("Updating properties for {remote_path}");
        let transfer_tx = self.transfer.queue.tx.clone();
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

fn normalize_transfer_properties_input(field: TransferPropertiesField, text: &str) -> String {
    if field == TransferPropertiesField::Mode {
        text.chars()
            .filter(|value| ('0'..='7').contains(value))
            .take(4)
            .collect()
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::models::TransferPropertiesField;

    use super::normalize_transfer_properties_input;

    #[test]
    fn properties_mode_input_keeps_four_octal_digits() {
        assert_eq!(
            normalize_transfer_properties_input(TransferPropertiesField::Mode, "09a7555"),
            "0755"
        );
    }

    #[test]
    fn properties_owner_and_group_input_are_not_filtered() {
        assert_eq!(
            normalize_transfer_properties_input(TransferPropertiesField::Owner, "dev team"),
            "dev team"
        );
        assert_eq!(
            normalize_transfer_properties_input(TransferPropertiesField::Group, "release-team"),
            "release-team"
        );
    }
}
