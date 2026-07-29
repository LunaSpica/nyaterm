use gpui::{Context, Window};
use nyaterm_transport::{SftpService, SshSessionConfig};

use crate::features::{
    NyaTermApp, TransferEditorCloseAfterSave, TransferEditorCloseOutcome,
    TransferEditorDiscardOutcome,
};
use crate::models::{
    TransferEditorState, TransferJobEvent, TransferJobKind, TransferJobOutput, TransferJobResult,
    TransferJobState, TransferJobStatus,
};

use super::super::NATIVE_EDITOR_MAX_BYTES;

impl NyaTermApp {
    pub(in crate::features) fn active_transfer_editor_tab(&self) -> Option<&TransferEditorState> {
        self.transfer.active_editor_tab()
    }

    pub(in crate::features) fn active_transfer_editor_tab_mut(
        &mut self,
    ) -> Option<&mut TransferEditorState> {
        self.transfer.active_editor_tab_mut()
    }

    pub(in crate::features) fn transfer_editor_ssh_config(
        &self,
        session_id: Option<&str>,
    ) -> Option<SshSessionConfig> {
        session_id
            .and_then(|session_id| self.session.metadata(session_id))
            .and_then(|metadata| metadata.ssh_config.clone())
            .or_else(|| {
                (session_id == self.session.active_id())
                    .then(|| self.session.active_ssh_config_owned())
                    .flatten()
            })
    }

    pub(in crate::features) fn activate_transfer_editor_tab(
        &mut self,
        tab_id: &str,
        cx: &mut Context<Self>,
    ) {
        if self.transfer.activate_editor_tab(tab_id) {
            cx.notify();
        }
    }

    pub(in crate::features) fn close_transfer_editor_tab(
        &mut self,
        tab_id: &str,
        cx: &mut Context<Self>,
    ) {
        match self.transfer.request_editor_tab_close(tab_id) {
            TransferEditorCloseOutcome::Missing => return,
            TransferEditorCloseOutcome::ConfirmationRequired => {
                self.shell.status = "remote editor tab has unsaved changes".to_string();
            }
            TransferEditorCloseOutcome::Closed => {
                self.shell.status = "remote editor tab closed".to_string();
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn close_transfer_editor(&mut self, cx: &mut Context<Self>) {
        match self.transfer.request_editor_close() {
            TransferEditorCloseOutcome::Missing => return,
            TransferEditorCloseOutcome::ConfirmationRequired => {
                self.shell.status = "remote editor has unsaved changes".to_string();
            }
            TransferEditorCloseOutcome::Closed => {
                self.shell.status = "remote editor closed".to_string();
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn discard_transfer_editor(&mut self, cx: &mut Context<Self>) {
        match self.transfer.discard_editor() {
            TransferEditorDiscardOutcome::Missing => return,
            TransferEditorDiscardOutcome::TabDiscarded => {
                self.shell.status = "remote editor tab discarded".to_string();
            }
            TransferEditorDiscardOutcome::WorkspaceDiscarded => {
                self.shell.status = "remote editor discarded".to_string();
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn cancel_transfer_editor_close_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if !self.transfer.cancel_editor_close() {
            cx.notify();
            return;
        }
        self.shell.status = "remote editor close cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn cancel_transfer_editor_reload_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if !self.transfer.cancel_editor_reload() {
            cx.notify();
            return;
        }
        self.shell.status = "remote editor reload cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn cancel_transfer_editor_conflict(&mut self, cx: &mut Context<Self>) {
        if !self.transfer.cancel_editor_conflict() {
            cx.notify();
            return;
        }
        self.shell.status = "remote editor conflict dismissed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn start_sftp_editor_load_job(
        &mut self,
        session_id: Option<String>,
        remote_path: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.transfer_editor_ssh_config(session_id.as_deref()) else {
            let error = self.tr("fileEditor.sourceSessionUnavailable").to_string();
            self.transfer
                .fail_editor_load(session_id.as_deref(), &remote_path, error.clone());
            self.shell.status = error;
            cx.notify();
            return;
        };
        let id = self.next_transfer_id("sftp-open-text");
        self.transfer.enqueue_transfer_job(TransferJobState {
            id: id.clone(),
            session_id,
            kind: TransferJobKind::LoadEditor {
                remote_path: remote_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Opening {remote_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        let transfer_tx = self.transfer.transfer_event_sender();
        std::thread::spawn(move || {
            let result = SftpService::new(config)
                .read_text_file(&remote_path, NATIVE_EDITOR_MAX_BYTES)
                .map(|file| TransferJobOutput::EditorLoaded { remote_path, file })
                .map_err(|error| error.to_string());
            let _ = transfer_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn save_transfer_editor(
        &mut self,
        force: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self.transfer.active_editor_tab().map(|tab| tab.id.clone());
        let Some(tab_id) = tab_id else {
            self.shell.status = "no remote editor is active".to_string();
            cx.notify();
            return;
        };
        self.save_transfer_editor_tab(&tab_id, force, window, cx);
    }

    pub(in crate::features) fn save_all_transfer_editor_tabs(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab_ids = self.transfer.dirty_editor_tab_ids();
        for tab_id in tab_ids {
            self.save_transfer_editor_tab(&tab_id, false, window, cx);
        }
    }

    fn save_transfer_editor_tab(
        &mut self,
        tab_id: &str,
        force: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(snapshot) = self.transfer.editor_tab_snapshot(tab_id) else {
            return;
        };
        if snapshot.loading || snapshot.saving {
            return;
        }
        let Some(config) = self.transfer_editor_ssh_config(snapshot.session_id.as_deref()) else {
            let error = self.tr("fileEditor.sourceSessionUnavailable").to_string();
            self.transfer.set_editor_tab_error(
                snapshot.session_id.as_deref(),
                &snapshot.remote_path,
                error.clone(),
            );
            self.shell.status = error;
            cx.notify();
            return;
        };
        if !self.transfer.begin_editor_tab_save(tab_id) {
            return;
        }
        self.start_sftp_editor_save_job(
            snapshot.session_id,
            config,
            snapshot.remote_path,
            snapshot.content,
            snapshot.base_modified_at,
            snapshot.base_size,
            force,
            window,
            cx,
        );
    }

    pub(in crate::features) fn save_transfer_editor_and_close(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.transfer.prepare_editor_close_after_save() {
            TransferEditorCloseAfterSave::Ready(tab_id) => {
                self.save_transfer_editor_tab(&tab_id, false, window, cx);
            }
            TransferEditorCloseAfterSave::All => {
                self.save_all_transfer_editor_tabs(window, cx);
            }
            TransferEditorCloseAfterSave::Missing => {
                self.shell.status = "no remote editor is active".to_string();
                cx.notify();
            }
            TransferEditorCloseAfterSave::Loading | TransferEditorCloseAfterSave::Saving => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::features) fn start_sftp_editor_save_job(
        &mut self,
        session_id: Option<String>,
        config: SshSessionConfig,
        remote_path: String,
        content: String,
        expected_modified_at: Option<u64>,
        expected_size: Option<u64>,
        force: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = self.next_transfer_id("sftp-save-text");
        self.transfer.enqueue_transfer_job(TransferJobState {
            id: id.clone(),
            session_id,
            kind: TransferJobKind::SaveEditor {
                remote_path: remote_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Saving {remote_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        let transfer_tx = self.transfer.transfer_event_sender();
        std::thread::spawn(move || {
            let result = SftpService::new(config)
                .write_text_file(
                    &remote_path,
                    &content,
                    expected_modified_at,
                    expected_size,
                    force,
                )
                .map(|result| TransferJobOutput::EditorSaved {
                    remote_path,
                    result,
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
