use gpui::{Context, Window};
use nyaterm_transport::{SftpService, SshSessionConfig};

use crate::features::NyaTermApp;
use crate::models::{
    TransferEditorState, TransferEditorWorkspaceState, TransferJobEvent, TransferJobKind,
    TransferJobOutput, TransferJobResult, TransferJobState, TransferJobStatus,
};

use super::super::NATIVE_EDITOR_MAX_BYTES;

impl NyaTermApp {
    pub(in crate::features) fn active_transfer_editor_tab(&self) -> Option<&TransferEditorState> {
        self.transfer
            .editor
            .workspace
            .as_ref()
            .and_then(TransferEditorWorkspaceState::active_tab)
    }

    pub(in crate::features) fn active_transfer_editor_tab_mut(
        &mut self,
    ) -> Option<&mut TransferEditorState> {
        self.transfer
            .editor
            .workspace
            .as_mut()
            .and_then(TransferEditorWorkspaceState::active_tab_mut)
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
        let Some(workspace) = self.transfer.editor.workspace.as_mut() else {
            return;
        };
        self.transfer.editor.tabs_menu_open = false;
        if workspace.tabs.iter().any(|tab| tab.id == tab_id) && workspace.active_tab_id != tab_id {
            workspace.active_tab_id = tab_id.to_string();
            workspace.close_confirm = false;
            workspace.pending_close_tab_id = None;
            workspace.close_after_save_all = false;
            cx.notify();
        }
    }

    pub(in crate::features) fn close_transfer_editor_tab(
        &mut self,
        tab_id: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace) = self.transfer.editor.workspace.as_mut() else {
            return;
        };
        self.transfer.editor.tabs_menu_open = false;
        let Some(tab) = workspace.tabs.iter().find(|tab| tab.id == tab_id) else {
            return;
        };
        if tab.dirty || tab.saving {
            workspace.active_tab_id = tab_id.to_string();
            workspace.close_confirm = true;
            workspace.pending_close_tab_id = Some(tab_id.to_string());
            workspace.close_after_save_all = false;
            self.terminal.view.status = "remote editor tab has unsaved changes".to_string();
            cx.notify();
            return;
        }
        workspace.remove_tab(tab_id);
        if workspace.tabs.is_empty() {
            self.transfer.editor.workspace = None;
            self.transfer.editor.window_open_pending = false;
        }
        self.terminal.view.status = "remote editor tab closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn close_transfer_editor(&mut self, cx: &mut Context<Self>) {
        let Some(workspace) = self.transfer.editor.workspace.as_mut() else {
            return;
        };
        if let Some(dirty_tab_id) = workspace
            .tabs
            .iter()
            .find(|tab| tab.dirty || tab.saving)
            .map(|tab| tab.id.clone())
        {
            workspace.active_tab_id = dirty_tab_id;
            workspace.close_confirm = true;
            workspace.pending_close_tab_id = None;
            workspace.close_after_save_all = false;
            self.terminal.view.status = "remote editor has unsaved changes".to_string();
            cx.notify();
            return;
        }
        self.transfer.editor.workspace = None;
        self.transfer.editor.tabs_menu_open = false;
        self.transfer.editor.window_open_pending = false;
        self.terminal.view.status = "remote editor closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn discard_transfer_editor(&mut self, cx: &mut Context<Self>) {
        let pending_tab_id = self
            .transfer
            .editor
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.pending_close_tab_id.clone());
        if let Some(tab_id) = pending_tab_id {
            if let Some(workspace) = self.transfer.editor.workspace.as_mut() {
                workspace.remove_tab(&tab_id);
                workspace.close_confirm = false;
                workspace.pending_close_tab_id = None;
                workspace.close_after_save_all = false;
                if workspace.tabs.is_empty() {
                    self.transfer.editor.workspace = None;
                    self.transfer.editor.window_open_pending = false;
                }
            }
            self.terminal.view.status = "remote editor tab discarded".to_string();
        } else {
            self.transfer.editor.workspace = None;
            self.transfer.editor.window_open_pending = false;
            self.terminal.view.status = "remote editor discarded".to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn cancel_transfer_editor_close_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace) = self.transfer.editor.workspace.as_mut() else {
            cx.notify();
            return;
        };
        workspace.close_confirm = false;
        workspace.pending_close_tab_id = None;
        workspace.close_after_save_all = false;
        for tab in &mut workspace.tabs {
            tab.close_after_save = false;
        }
        self.terminal.view.status = "remote editor close cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn cancel_transfer_editor_reload_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self
            .transfer
            .editor
            .workspace
            .as_mut()
            .and_then(TransferEditorWorkspaceState::active_tab_mut)
        else {
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
        self.terminal.view.status = "remote editor reload cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn cancel_transfer_editor_conflict(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self
            .transfer
            .editor
            .workspace
            .as_mut()
            .and_then(TransferEditorWorkspaceState::active_tab_mut)
        else {
            cx.notify();
            return;
        };
        state.conflict = false;
        self.terminal.view.status = "remote editor conflict dismissed".to_string();
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
            if let Some(state) = self
                .transfer
                .editor
                .workspace
                .as_mut()
                .and_then(|workspace| workspace.tab_mut(session_id.as_deref(), &remote_path))
            {
                state.loading = false;
                state.error = Some(error.clone());
            }
            self.terminal.view.status = error;
            cx.notify();
            return;
        };
        let id = self.next_transfer_id("sftp-open-text");
        self.transfer.queue.jobs.push(TransferJobState {
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
        let transfer_tx = self.transfer.queue.tx.clone();
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
        let tab_id = self
            .transfer
            .editor
            .workspace
            .as_ref()
            .and_then(TransferEditorWorkspaceState::active_tab)
            .map(|tab| tab.id.clone());
        let Some(tab_id) = tab_id else {
            self.terminal.view.status = "no remote editor is active".to_string();
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
        let tab_ids = self
            .transfer
            .editor
            .workspace
            .as_ref()
            .map(|workspace| {
                workspace
                    .tabs
                    .iter()
                    .filter(|tab| tab.dirty && !tab.loading && !tab.saving)
                    .map(|tab| tab.id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
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
        let Some(snapshot) = self
            .transfer
            .editor
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.tabs.iter().find(|tab| tab.id == tab_id).cloned())
        else {
            return;
        };
        if snapshot.loading || snapshot.saving {
            return;
        }
        let Some(config) = self.transfer_editor_ssh_config(snapshot.session_id.as_deref()) else {
            let error = self.tr("fileEditor.sourceSessionUnavailable").to_string();
            if let Some(tab) = self
                .transfer
                .editor
                .workspace
                .as_mut()
                .and_then(|workspace| {
                    workspace.tab_mut(snapshot.session_id.as_deref(), &snapshot.remote_path)
                })
            {
                tab.error = Some(error.clone());
            }
            self.terminal.view.status = error;
            cx.notify();
            return;
        };
        if let Some(tab) = self
            .transfer
            .editor
            .workspace
            .as_mut()
            .and_then(|workspace| {
                workspace.tab_mut(snapshot.session_id.as_deref(), &snapshot.remote_path)
            })
        {
            tab.saving = true;
            tab.error = None;
            tab.conflict = false;
            tab.reload_confirm = false;
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
        let pending_tab_id = self
            .transfer
            .editor
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.pending_close_tab_id.clone());
        if let Some(tab_id) = pending_tab_id {
            if let Some(tab) = self
                .transfer
                .editor
                .workspace
                .as_mut()
                .and_then(|workspace| workspace.tabs.iter_mut().find(|tab| tab.id == tab_id))
            {
                if tab.loading {
                    return;
                }
                tab.close_after_save = true;
                if tab.saving {
                    return;
                }
            }
            self.save_transfer_editor_tab(&tab_id, false, window, cx);
            return;
        }

        let Some(workspace) = self.transfer.editor.workspace.as_mut() else {
            self.terminal.view.status = "no remote editor is active".to_string();
            cx.notify();
            return;
        };
        workspace.close_after_save_all = true;
        workspace.close_confirm = false;
        self.save_all_transfer_editor_tabs(window, cx);
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
        self.transfer.queue.jobs.push(TransferJobState {
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
        let transfer_tx = self.transfer.queue.tx.clone();
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
