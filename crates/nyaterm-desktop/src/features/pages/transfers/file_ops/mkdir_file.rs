use crate::features::NyaTermApp;
use crate::features::transfers::session_sftp_service;
use crate::models::{
    TransferJobEvent, TransferJobKind, TransferJobOutput, TransferJobResult, TransferJobState,
    TransferJobStatus, TransferNewFileState, TransferNewFolderState,
};
use gpui::{Context, Window};

use super::super::helpers::{remote_child_path, valid_remote_child_name};

impl NyaTermApp {
    pub(in crate::features) fn open_transfer_new_folder_dialog(
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
            .open_new_folder_dialog(TransferNewFolderState {
                parent_path,
                value: String::new(),
                mode: 0o755,
                open_after_create: false,
            });
        // The box owns its text, so it has to be dropped for the next dialog to
        // open empty.
        self.forget_text_inputs("transfer.new-folder.");
        self.shell.set_status("SFTP new folder opened".to_string());
        self.open_form_dialog(
            (
                self.tr("fileExplorer.newFolder").to_string(),
                500.,
                self.tr("common.confirm").to_string(),
                |app, _, cx| app.transfer_new_folder_dialog_content(cx),
                |app, window, cx| app.submit_transfer_new_folder(window, cx),
                |app, cx| app.close_transfer_new_folder_dialog(cx),
            ),
            window,
            cx,
        );
        cx.notify();
    }

    pub(in crate::features) fn close_transfer_new_folder_dialog(&mut self, cx: &mut Context<Self>) {
        self.transfer.close_new_folder_dialog();
        self.forget_text_inputs("transfer.new-folder.");
        self.shell
            .set_status("SFTP new folder cancelled".to_string());
        cx.notify();
    }

    pub(in crate::features) fn submit_transfer_new_folder(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(state) = self.transfer.new_folder_dialog().cloned() else {
            self.shell
                .set_status("no SFTP new folder is active".to_string());
            cx.notify();
            return true;
        };
        let name = state.value.trim().to_string();
        if !valid_remote_child_name(&name) {
            self.shell
                .set_status("folder name must be a single non-empty name".to_string());
            cx.notify();
            return false;
        }
        self.transfer.close_new_folder_dialog();
        let remote_path = remote_child_path(&state.parent_path, &name);
        self.start_sftp_mkdir_job(
            remote_path,
            state.parent_path,
            state.mode,
            state.open_after_create,
            window,
            cx,
        );
        true
    }

    /// Apply an edit from the new-folder dialog's name box.
    pub(in crate::features) fn apply_transfer_new_folder_name(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        // A remote name has a length limit, and the box will happily take more.
        if self
            .transfer
            .set_new_folder_name(text.chars().take(255).collect())
        {
            cx.notify();
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
        let Some(config) = self.session.active_ssh_config_owned() else {
            self.shell
                .set_status("start an SSH session first".to_string());
            self.ensure_panel_open(crate::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        let multiplex = self.session.active_ssh_multiplex_handle();
        let id = self.transfer.next_transfer_job_id("sftp-mkdir");
        self.transfer.enqueue_transfer_job(TransferJobState {
            id: id.clone(),
            session_id: self.session.active_id_owned(),
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
        self.shell
            .set_status(format!("SFTP create folder started: {remote_path}"));
        let transfer_tx = self.transfer.transfer_event_sender();
        std::thread::spawn(move || {
            let result = session_sftp_service(config, multiplex)
                .and_then(|service| {
                    let list_path = if open_after_create {
                        remote_path.clone()
                    } else {
                        parent_path.clone()
                    };
                    service
                        .create_dir_path(&remote_path, Some(mode))
                        .and_then(|_| service.list_dir(&list_path))
                })
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
        let parent_path = if self.transfer.browser_view().path.trim().is_empty() {
            self.transfer.normalized_remote_path()
        } else {
            self.transfer.browser_view().path.clone()
        };
        self.transfer.open_new_file_dialog(TransferNewFileState {
            parent_path,
            value: String::new(),
            mode: 0o644,
            open_after_create: false,
        });
        self.forget_text_inputs("transfer.new-file.");
        self.shell.set_status("SFTP new file opened".to_string());
        self.open_form_dialog(
            (
                self.tr("fileExplorer.newFile").to_string(),
                500.,
                self.tr("common.confirm").to_string(),
                |app, _, cx| app.transfer_new_file_dialog_content(cx),
                |app, window, cx| app.submit_transfer_new_file(window, cx),
                |app, cx| app.close_transfer_new_file_dialog(cx),
            ),
            window,
            cx,
        );
        cx.notify();
    }

    pub(in crate::features) fn close_transfer_new_file_dialog(&mut self, cx: &mut Context<Self>) {
        self.transfer.close_new_file_dialog();
        self.forget_text_inputs("transfer.new-file.");
        self.shell.set_status("SFTP new file cancelled".to_string());
        cx.notify();
    }

    pub(in crate::features) fn submit_transfer_new_file(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(state) = self.transfer.new_file_dialog().cloned() else {
            self.shell
                .set_status("no SFTP new file is active".to_string());
            cx.notify();
            return true;
        };
        let name = state.value.trim().to_string();
        if !valid_remote_child_name(&name) {
            self.shell
                .set_status("file name must be a single non-empty name".to_string());
            cx.notify();
            return false;
        }
        self.transfer.close_new_file_dialog();
        let remote_path = remote_child_path(&state.parent_path, &name);
        self.start_sftp_create_file_job(
            remote_path,
            state.parent_path,
            state.mode,
            state.open_after_create,
            window,
            cx,
        );
        true
    }

    /// Apply an edit from the new-file dialog's name box.
    pub(in crate::features) fn apply_transfer_new_file_name(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        if self
            .transfer
            .set_new_file_name(text.chars().take(255).collect())
        {
            cx.notify();
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
        let Some(config) = self.session.active_ssh_config_owned() else {
            self.shell
                .set_status("start an SSH session first".to_string());
            self.ensure_panel_open(crate::models::NavItem::Transfers);
            cx.notify();
            return;
        };
        let multiplex = self.session.active_ssh_multiplex_handle();
        let id = self.transfer.next_transfer_job_id("sftp-create-file");
        self.transfer.enqueue_transfer_job(TransferJobState {
            id: id.clone(),
            session_id: self.session.active_id_owned(),
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
        self.shell
            .set_status(format!("SFTP create file started: {remote_path}"));
        let transfer_tx = self.transfer.transfer_event_sender();
        std::thread::spawn(move || {
            let result = session_sftp_service(config, multiplex)
                .and_then(|service| {
                    service
                        .create_file_path(&remote_path, Some(mode))
                        .and_then(|_| service.list_dir(&parent_path))
                })
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
