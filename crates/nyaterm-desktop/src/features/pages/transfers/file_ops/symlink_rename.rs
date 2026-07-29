use gpui::{Context, KeyDownEvent, Window};
use nyaterm_transport::SftpService;

use crate::features::NyaTermApp;
use crate::models::{
    TransferJobEvent, TransferJobKind, TransferJobOutput, TransferJobResult, TransferJobState,
    TransferJobStatus, TransferNewSymlinkState, TransferRenameState, TransferSymlinkField,
};

use super::super::helpers::{
    remote_child_path, remote_file_name, remote_parent_path, remote_sibling_path,
    valid_remote_child_name,
};

impl NyaTermApp {
    pub(in crate::features) fn open_transfer_new_symlink_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let parent_path = if self.transfer.browser_view().path.trim().is_empty() {
            self.transfer.normalized_remote_path()
        } else {
            self.transfer.browser_view().path.clone()
        };
        self.transfer
            .open_new_symlink_dialog(TransferNewSymlinkState {
                parent_path,
                name: "new-link".to_string(),
                target: String::new(),
                focused_field: TransferSymlinkField::Name,
            });
        self.forget_text_inputs("transfer.new-symlink.");
        self.shell.set_status("SFTP new symlink opened".to_string());
        window.focus(self.transfer.new_symlink_focus());
        cx.notify();
    }

    pub(in crate::features) fn close_transfer_new_symlink_dialog(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.transfer.close_new_symlink_dialog();
        self.forget_text_inputs("transfer.new-symlink.");
        self.shell
            .set_status("SFTP new symlink cancelled".to_string());
        cx.notify();
    }

    pub(in crate::features) fn submit_transfer_new_symlink(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer.new_symlink_dialog().cloned() else {
            self.shell
                .set_status("no SFTP new symlink is active".to_string());
            cx.notify();
            return;
        };
        let name = state.name.trim().to_string();
        let target_path = state.target.trim().to_string();
        if !valid_remote_child_name(&name) {
            self.shell
                .set_status("symlink name must be a single non-empty name".to_string());
            cx.notify();
            return;
        }
        if target_path.is_empty() {
            self.shell
                .set_status("symlink target cannot be empty".to_string());
            cx.notify();
            return;
        }
        self.transfer.close_new_symlink_dialog();
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

        // The boxes own the text; the dialog owns the keys that close or submit.
        match keystroke.key.as_str() {
            "escape" => self.close_transfer_new_symlink_dialog(cx),
            "enter" => self.submit_transfer_new_symlink(window, cx),
            _ => {}
        }
    }

    /// Apply an edit from one of the symlink dialog's boxes.
    ///
    /// A remote name and a link target have different length limits, and both
    /// are enforced here rather than by the box.
    pub(in crate::features) fn apply_transfer_new_symlink_input(
        &mut self,
        field: TransferSymlinkField,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let value = match field {
            TransferSymlinkField::Name => text.chars().take(255).collect(),
            TransferSymlinkField::Target => text.chars().take(1024).collect(),
        };
        if self.transfer.set_new_symlink_input(field, value) {
            cx.notify();
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
        let Some(config) = self.session.active_ssh_config_owned() else {
            self.shell
                .set_status("start an SSH session first".to_string());
            self.ensure_panel_open(crate::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        let id = self.next_transfer_id("sftp-symlink");
        self.transfer.enqueue_transfer_job(TransferJobState {
            id: id.clone(),
            session_id: self.session.active_id_owned(),
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
        self.shell
            .set_status(format!("SFTP symlink started: {link_path}"));
        let transfer_tx = self.transfer.transfer_event_sender();
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
        self.forget_text_inputs("transfer.rename.");
        self.ensure_panel_open(crate::models::NavItem::Transfers);
        let Some(old_path) = self.transfer.browser_view().selected_remote_path.clone() else {
            self.shell
                .set_status("select an SFTP list entry before renaming".to_string());
            cx.notify();
            return;
        };
        if !self.open_transfer_rename_for_path(old_path, cx) {
            return;
        }
        window.focus(self.transfer.rename_focus());
        cx.notify();
    }

    pub(in crate::features) fn open_transfer_rename_for_path_after_delay(
        &mut self,
        old_path: String,
        cx: &mut Context<Self>,
    ) {
        if self.open_transfer_rename_for_path(old_path, cx) {
            self.transfer.schedule_rename_focus();
            cx.notify();
        }
    }

    pub(in crate::features) fn open_transfer_rename_for_path(
        &mut self,
        old_path: String,
        cx: &mut Context<Self>,
    ) -> bool {
        self.forget_text_inputs("transfer.rename.");
        let initial_name = remote_file_name(&old_path);
        if initial_name.is_empty() || initial_name == "." || initial_name == ".." {
            self.shell.set_status(format!("cannot rename {old_path}"));
            cx.notify();
            return false;
        }
        self.transfer.open_rename_dialog(TransferRenameState {
            old_path,
            value: initial_name.clone(),
            initial_name,
        });
        self.shell.set_status("SFTP rename opened".to_string());
        true
    }

    pub(in crate::features) fn close_transfer_rename_dialog(&mut self, cx: &mut Context<Self>) {
        self.forget_text_inputs("transfer.rename.");
        self.transfer.close_rename_dialog();
        self.shell.set_status("SFTP rename cancelled".to_string());
        cx.notify();
    }

    pub(in crate::features) fn submit_transfer_rename(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer.rename_dialog().cloned() else {
            self.shell
                .set_status("no SFTP rename is active".to_string());
            cx.notify();
            return;
        };
        let new_name = state.value.trim().to_string();
        if new_name.is_empty() {
            self.shell
                .set_status("remote name cannot be empty".to_string());
            cx.notify();
            return;
        }
        if new_name.contains('/') || new_name == "." || new_name == ".." {
            self.shell
                .set_status("remote name must be a single file or directory name".to_string());
            cx.notify();
            return;
        }
        if new_name == state.initial_name {
            self.transfer.close_rename_dialog();
            self.shell.set_status("SFTP rename unchanged".to_string());
            cx.notify();
            return;
        }
        let new_path = remote_sibling_path(&state.old_path, &new_name);
        self.transfer.close_rename_dialog();
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

        // The box owns the text; the row owns the keys that cancel or commit.
        match keystroke.key.as_str() {
            "escape" => {
                cx.stop_propagation();
                self.close_transfer_rename_dialog(cx);
            }
            "enter" => {
                cx.stop_propagation();
                self.submit_transfer_rename(window, cx);
            }
            _ => {}
        }
    }

    /// Apply an edit from the inline rename box.
    pub(in crate::features) fn apply_transfer_rename_input(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        if self
            .transfer
            .set_rename_value(text.chars().take(255).collect())
        {
            cx.notify();
        }
    }

    pub(in crate::features) fn start_sftp_rename_job(
        &mut self,
        old_path: String,
        new_path: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config_owned() else {
            self.shell
                .set_status("start an SSH session first".to_string());
            self.ensure_panel_open(crate::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        let parent_path = remote_parent_path(&old_path);
        let id = self.next_transfer_id("sftp-rename");
        self.transfer.enqueue_transfer_job(TransferJobState {
            id: id.clone(),
            session_id: self.session.active_id_owned(),
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
        self.shell
            .set_status(format!("SFTP rename started: {old_path} -> {new_path}"));
        let transfer_tx = self.transfer.transfer_event_sender();
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
