use super::*;

impl NyaTermApp {
    pub(in crate::features) fn prompt_recording_path(
        &mut self,
        kind: RecordingPathPromptKind,
        cx: &mut Context<Self>,
    ) {
        if self.recording_path_prompt.is_some() {
            self.terminal_status = "recording path picker is already open".to_string();
            cx.notify();
            return;
        }
        let Some(session_id) = self.active_session_id.clone() else {
            self.terminal_status = "start a session before recording".to_string();
            cx.notify();
            return;
        };
        let session_name = self.session_display_name(&session_id).unwrap_or_else(|| {
            self.active_session_name()
                .unwrap_or_else(|| "session".to_string())
        });
        self.prompt_recording_path_for_session(kind, session_id, session_name, cx);
    }

    pub(in crate::features) fn prompt_recording_path_for_session(
        &mut self,
        kind: RecordingPathPromptKind,
        session_id: String,
        session_name: String,
        cx: &mut Context<Self>,
    ) {
        if self.recording_path_prompt.is_some() {
            self.terminal_status = "recording path picker is already open".to_string();
            cx.notify();
            return;
        }
        let exists = self
            .session_manager
            .list_sessions()
            .unwrap_or_default()
            .into_iter()
            .any(|session| session.id == session_id);
        if !exists {
            self.terminal_status = "session no longer exists".to_string();
            self.remove_session_state(&session_id);
            cx.notify();
            return;
        }
        let target = recording_file_path(&self.settings, self.runtime.config_dir(), &session_name);
        let directory = target
            .parent()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| self.runtime.config_dir().to_path_buf());
        let file_name = target
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("nyaterm-recording.log");
        let receiver = cx.prompt_for_new_path(&directory, Some(file_name));
        self.recording_path_prompt = Some(kind);
        self.terminal_status = match kind {
            RecordingPathPromptKind::Start => "selecting recording path".to_string(),
            RecordingPathPromptKind::SaveTranscript => "selecting transcript path".to_string(),
        };
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(path))) => RecordingPathPromptResult::Selected(path),
                Ok(Ok(None)) => RecordingPathPromptResult::Cancelled,
                Ok(Err(error)) => RecordingPathPromptResult::Failed(error.to_string()),
                Err(_) => RecordingPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_recording_path_prompt_result(kind, session_id, result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_recording_path_prompt_result(
        &mut self,
        kind: RecordingPathPromptKind,
        session_id: String,
        result: RecordingPathPromptResult,
    ) {
        self.recording_path_prompt = None;
        match result {
            RecordingPathPromptResult::Selected(path) => match kind {
                RecordingPathPromptKind::Start => {
                    self.start_recording_to_path(&session_id, path.display().to_string());
                }
                RecordingPathPromptKind::SaveTranscript => {
                    self.save_transcript_to_path(&session_id, path.display().to_string());
                }
            },
            RecordingPathPromptResult::Cancelled => {
                self.terminal_status = match kind {
                    RecordingPathPromptKind::Start => "recording start cancelled".to_string(),
                    RecordingPathPromptKind::SaveTranscript => {
                        "transcript save cancelled".to_string()
                    }
                };
            }
            RecordingPathPromptResult::Failed(error) => {
                self.terminal_status = format!("recording path picker failed: {error}");
            }
            RecordingPathPromptResult::Closed => {
                self.terminal_status = "recording path picker closed before returning".to_string();
            }
        }
    }

    fn start_recording_to_path(&mut self, session_id: &str, path: String) {
        self.recording_busy_actions
            .insert(session_id.to_string(), "record".to_string());
        self.recording_manager
            .set_memory_limit(self.settings.recording_memory_limit_bytes as usize);
        match self.recording_manager.start(
            session_id,
            &path,
            self.settings.recording_include_io_labels,
            self.settings.recording_include_timestamps,
        ) {
            Ok(()) => {
                self.terminal_status = format!("recording started: {path}");
                self.append_terminal_log(format!("\n# recording started: {path}\n"));
            }
            Err(error) => {
                self.terminal_status = format!("recording start failed: {error}");
            }
        }
        self.recording_busy_actions.remove(session_id);
    }

    pub(in crate::features) fn stop_active_recording(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.active_session_id.clone() else {
            self.terminal_status = "no active session to stop recording".to_string();
            cx.notify();
            return;
        };
        self.stop_recording_for_session(&session_id, cx);
    }

    pub(in crate::features) fn stop_recording_for_session(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        self.recording_busy_actions
            .insert(session_id.to_string(), "record".to_string());
        match self.recording_manager.stop(session_id) {
            Ok(path) => {
                self.terminal_status = format!("recording saved: {path}");
                self.append_terminal_log(format!("\n# recording saved: {path}\n"));
            }
            Err(error) => {
                self.terminal_status = format!("recording stop failed: {error}");
            }
        }
        self.recording_busy_actions.remove(session_id);
        cx.notify();
    }

    fn save_transcript_to_path(&mut self, session_id: &str, path: String) {
        self.recording_busy_actions
            .insert(session_id.to_string(), "save".to_string());
        self.recording_manager
            .set_memory_limit(self.settings.recording_memory_limit_bytes as usize);
        match self.recording_manager.save_transcript(
            session_id,
            &path,
            self.settings.recording_include_io_labels,
            self.settings.recording_include_timestamps,
        ) {
            Ok(path) => {
                self.terminal_status = format!("transcript saved: {path}");
                self.append_terminal_log(format!("\n# transcript saved: {path}\n"));
            }
            Err(error) => {
                self.terminal_status = format!("transcript save failed: {error}");
            }
        }
        self.recording_busy_actions.remove(session_id);
    }

    pub(in crate::features) fn maybe_auto_start_recording(
        &mut self,
        session_id: &str,
        session_name: &str,
    ) {
        if !self.settings.recording_auto_start {
            return;
        }
        let path = recording_file_path(&self.settings, self.runtime.config_dir(), session_name);
        self.start_recording_to_path(session_id, path.display().to_string());
    }

    pub(in crate::features) fn handle_recording_search_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.recording_search_draft.pop();
                cx.notify();
            }
            "escape" => {
                self.recording_search_draft.clear();
                self.terminal_status = "recording search cleared".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.recording_search_draft.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn recording_search_results(
        &self,
    ) -> Result<nyaterm_transport::TerminalHistorySearchResponse, String> {
        let Some(session_id) = self.active_session_id.clone() else {
            return Ok(nyaterm_transport::TerminalHistorySearchResponse {
                total: 0,
                elapsed_ms: 0,
                truncated: false,
                results: Vec::new(),
            });
        };
        self.recording_manager
            .search_history(TerminalHistorySearchRequest {
                session_id,
                query: self.recording_search_draft.trim().to_string(),
                case_sensitive: false,
                regex: false,
                whole_word: false,
                limit: Some(8),
                context_before: Some(1),
                context_after: Some(1),
                max_lines: None,
            })
            .map_err(|error| error.to_string())
    }
}
