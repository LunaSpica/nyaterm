use super::*;

impl NyaTermApp {
    pub(in crate::features) fn handle_transfer_editor_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        let primary = keystroke.modifiers.platform || keystroke.modifiers.control;
        if primary && !keystroke.modifiers.alt && keystroke.key.as_str() == "f" {
            if let Some(state) = self.transfer_editor.as_mut() {
                state.focused_field = TransferEditorField::Search;
                state.close_confirm = false;
                state.error = None;
            }
            cx.notify();
            return;
        }
        if primary && !keystroke.modifiers.alt && keystroke.key.as_str() == "s" {
            self.save_transfer_editor(false, window, cx);
            return;
        }
        if primary && !keystroke.modifiers.alt && keystroke.key.as_str() == "enter" {
            self.save_transfer_editor(true, window, cx);
            return;
        }
        if keystroke.modifiers.alt || keystroke.modifiers.function || primary {
            return;
        }
        let focused_field = self
            .transfer_editor
            .as_ref()
            .map(|state| state.focused_field)
            .unwrap_or(TransferEditorField::Content);
        if keystroke.key.as_str() == "escape"
            && self
                .transfer_editor
                .as_ref()
                .is_some_and(|state| state.reload_confirm)
        {
            self.cancel_transfer_editor_reload_confirm(cx);
            return;
        }
        if keystroke.key.as_str() == "escape"
            && self
                .transfer_editor
                .as_ref()
                .is_some_and(|state| state.close_confirm)
        {
            self.cancel_transfer_editor_close_confirm(cx);
            return;
        }
        if focused_field == TransferEditorField::Search {
            match keystroke.key.as_str() {
                "escape" => {
                    if let Some(state) = self.transfer_editor.as_mut() {
                        state.focused_field = TransferEditorField::Content;
                    }
                    cx.notify();
                }
                "enter" => self.advance_transfer_editor_search(1, cx),
                "backspace" => {
                    if let Some(state) = self.transfer_editor.as_mut() {
                        state.search_query.pop();
                        state.active_match = 0;
                    }
                    cx.notify();
                }
                _ => {
                    if let Some(input) = keystroke
                        .key_char
                        .as_deref()
                        .filter(|input| !input.is_empty())
                        && let Some(state) = self.transfer_editor.as_mut()
                    {
                        state.search_query.push_str(input);
                        state.active_match = 0;
                        cx.notify();
                    }
                }
            }
            return;
        }
        match keystroke.key.as_str() {
            "escape" => self.close_transfer_editor(cx),
            "backspace" => {
                if let Some(state) = self.transfer_editor.as_mut() {
                    state.content.pop();
                    state.dirty = true;
                    state.conflict = false;
                    state.close_confirm = false;
                    state.close_after_save = false;
                    state.reload_confirm = false;
                    state.error = None;
                }
                cx.notify();
            }
            "enter" => {
                if let Some(state) = self.transfer_editor.as_mut() {
                    state.content.push('\n');
                    state.dirty = true;
                    state.conflict = false;
                    state.close_confirm = false;
                    state.close_after_save = false;
                    state.reload_confirm = false;
                    state.error = None;
                }
                cx.notify();
            }
            "tab" => {
                if let Some(state) = self.transfer_editor.as_mut() {
                    state.content.push_str("    ");
                    state.dirty = true;
                    state.conflict = false;
                    state.close_confirm = false;
                    state.close_after_save = false;
                    state.reload_confirm = false;
                    state.error = None;
                }
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                    && let Some(state) = self.transfer_editor.as_mut()
                {
                    state.content.push_str(input);
                    state.dirty = true;
                    state.conflict = false;
                    state.close_confirm = false;
                    state.close_after_save = false;
                    state.reload_confirm = false;
                    state.error = None;
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn advance_transfer_editor_search(
        &mut self,
        delta: isize,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer_editor.as_mut() else {
            return;
        };
        let matches = editor_search_matches(&state.content, &state.search_query);
        if matches.is_empty() {
            state.active_match = 0;
        } else if delta >= 0 {
            state.active_match = (state.active_match + 1) % matches.len();
        } else if state.active_match == 0 {
            state.active_match = matches.len() - 1;
        } else {
            state.active_match -= 1;
        }
        cx.notify();
    }

    pub(in crate::features) fn upload_external_editor_sync(
        &mut self,
        session_id: Option<String>,
        job_id: String,
        remote_path: String,
        local_path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        self.spawn_external_editor_sync_upload(session_id, job_id, remote_path, local_path);
        cx.notify();
    }

    pub(in crate::features) fn spawn_external_editor_sync_upload(
        &mut self,
        session_id: Option<String>,
        job_id: String,
        remote_path: String,
        local_path: PathBuf,
    ) {
        let config = session_id
            .as_deref()
            .and_then(|session_id| self.session_metadata.get(session_id))
            .and_then(|metadata| metadata.ssh_config.clone())
            .or_else(|| {
                (session_id.as_deref() == self.active_session_id.as_deref())
                    .then(|| self.active_ssh_config.clone())
                    .flatten()
            });
        let Some(config) = config else {
            self.terminal_status = "start an SSH session before syncing external edits".to_string();
            return;
        };
        let transfer_tx = self.transfer_tx.clone();
        let transfer_options = self.sftp_transfer_options();
        std::thread::spawn(move || {
            upload_external_editor_file(
                &config,
                &job_id,
                &remote_path,
                &local_path,
                transfer_options,
                &transfer_tx,
            );
        });
    }

    pub(in crate::features) fn active_external_editor_sync_prompt(
        &self,
    ) -> Option<(String, TransferExternalSyncPromptState)> {
        let active_session_id = self.active_session_id.as_deref()?;
        self.transfer_external_sync_prompts
            .iter()
            .find(|(prompt_id, prompt)| {
                prompt.session_id.as_deref() == Some(active_session_id)
                    && !self.transfer_external_sync_windows.contains_key(*prompt_id)
            })
            .map(|(prompt_id, prompt)| (prompt_id.clone(), prompt.clone()))
    }

    pub(in crate::features) fn upload_external_editor_sync_prompt(
        &mut self,
        prompt_id: &str,
        always: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(prompt) = self.transfer_external_sync_prompts.remove(prompt_id) else {
            cx.notify();
            return;
        };
        self.transfer_external_sync_windows.remove(prompt_id);
        let watch_key = external_editor_watch_key(&prompt.remote_path, &prompt.local_path);
        if always {
            self.transfer_external_always_uploads.insert(watch_key);
        }
        self.upload_external_editor_sync(
            prompt.session_id,
            prompt.job_id,
            prompt.remote_path,
            prompt.local_path,
            cx,
        );
    }

    pub(in crate::features) fn ignore_external_editor_sync_prompt(
        &mut self,
        prompt_id: &str,
        cx: &mut Context<Self>,
    ) {
        self.transfer_external_sync_prompts.remove(prompt_id);
        self.transfer_external_sync_windows.remove(prompt_id);
        self.terminal_status = "external edit sync skipped".to_string();
        cx.notify();
    }
}
