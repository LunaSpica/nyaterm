use super::*;

impl NyaTermApp {
    pub(in crate::features) fn close_transfer_editor(&mut self, cx: &mut Context<Self>) {
        if let Some(state) = self.transfer_editor.as_mut()
            && state.dirty
        {
            state.close_confirm = true;
            state.reload_confirm = false;
            state.error = Some("Unsaved changes. Save or Discard before closing.".to_string());
            self.terminal_status = "remote editor has unsaved changes".to_string();
            cx.notify();
            return;
        }
        self.transfer_editor = None;
        self.terminal_status = "remote editor closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn discard_transfer_editor(&mut self, cx: &mut Context<Self>) {
        self.transfer_editor = None;
        self.terminal_status = "remote editor discarded".to_string();
        cx.notify();
    }

    pub(in crate::features) fn cancel_transfer_editor_close_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer_editor.as_mut() else {
            cx.notify();
            return;
        };
        state.close_confirm = false;
        state.close_after_save = false;
        if state
            .error
            .as_deref()
            .is_some_and(|error| error.contains("Unsaved changes"))
        {
            state.error = None;
        }
        self.terminal_status = "remote editor close cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn cancel_transfer_editor_reload_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer_editor.as_mut() else {
            cx.notify();
            return;
        };
        state.reload_confirm = false;
        if state
            .error
            .as_deref()
            .is_some_and(|error| error.contains("Reload will discard"))
        {
            state.error = None;
        }
        self.terminal_status = "remote editor reload cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn start_sftp_editor_load_job(
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
        let id = self.next_transfer_id("sftp-open-text");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
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
        let transfer_tx = self.transfer_tx.clone();
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
        let Some(state) = self.transfer_editor.as_mut() else {
            self.terminal_status = "no remote editor is active".to_string();
            cx.notify();
            return;
        };
        if state.loading || state.saving {
            cx.notify();
            return;
        }
        state.saving = true;
        state.error = None;
        state.conflict = false;
        state.reload_confirm = false;
        let remote_path = state.remote_path.clone();
        let content = state.content.clone();
        let expected_modified_at = state.base_modified_at;
        let expected_size = state.base_size;
        self.start_sftp_editor_save_job(
            remote_path,
            content,
            expected_modified_at,
            expected_size,
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
        let Some(state) = self.transfer_editor.as_mut() else {
            self.terminal_status = "no remote editor is active".to_string();
            cx.notify();
            return;
        };
        if state.loading || state.saving {
            cx.notify();
            return;
        }
        state.close_after_save = true;
        self.save_transfer_editor(false, window, cx);
    }

    pub(in crate::features) fn start_sftp_editor_save_job(
        &mut self,
        remote_path: String,
        content: String,
        expected_modified_at: Option<u64>,
        expected_size: Option<u64>,
        force: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            cx.notify();
            return;
        };
        let id = self.next_transfer_id("sftp-save-text");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
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
        let transfer_tx = self.transfer_tx.clone();
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
