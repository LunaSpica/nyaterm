use gpui::{AppContext, Context};

use crate::features::NyaTermApp;
use crate::features::formatting::recording_file_path;
use crate::models::{RecordingPathPromptKind, RecordingPathPromptResult};

impl NyaTermApp {
    pub(in crate::features) fn prompt_recording_path_for_session(
        &mut self,
        kind: RecordingPathPromptKind,
        session_id: String,
        session_name: String,
        cx: &mut Context<Self>,
    ) {
        if !self.recording.begin_path_prompt(kind) {
            self.terminal.view.status = "recording path picker is already open".to_string();
            cx.notify();
            return;
        }
        let exists = self
            .session
            .metadata(&session_id)
            .is_some_and(|metadata| !metadata.disconnected);
        if !exists {
            self.recording.finish_path_prompt();
            self.terminal.view.status = "session no longer exists".to_string();
            self.remove_session_state(&session_id);
            cx.notify();
            return;
        }
        let target = recording_file_path(
            &self.settings.summary,
            self.runtime.config_dir(),
            &session_name,
        );
        let directory = target
            .parent()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| self.runtime.config_dir().to_path_buf());
        let file_name = target
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("nyaterm-recording.log");
        let receiver = cx.prompt_for_new_path(&directory, Some(file_name));
        self.terminal.view.status = match kind {
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
                this.apply_recording_path_prompt_result(kind, session_id, result, cx);
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
        cx: &mut Context<Self>,
    ) {
        self.recording.finish_path_prompt();
        match result {
            RecordingPathPromptResult::Selected(path) => match kind {
                RecordingPathPromptKind::Start => {
                    self.start_recording_to_path(&session_id, path.display().to_string(), cx);
                }
                RecordingPathPromptKind::SaveTranscript => {
                    self.save_transcript_to_path(&session_id, path.display().to_string(), cx);
                }
            },
            RecordingPathPromptResult::Cancelled => {
                self.terminal.view.status = match kind {
                    RecordingPathPromptKind::Start => "recording start cancelled".to_string(),
                    RecordingPathPromptKind::SaveTranscript => {
                        "transcript save cancelled".to_string()
                    }
                };
            }
            RecordingPathPromptResult::Failed(error) => {
                self.terminal.view.status = format!("recording path picker failed: {error}");
            }
            RecordingPathPromptResult::Closed => {
                self.terminal.view.status =
                    "recording path picker closed before returning".to_string();
            }
        }
    }

    fn start_recording_to_path(&mut self, session_id: &str, path: String, cx: &mut Context<Self>) {
        if !self.recording.begin_action(session_id, "record") {
            self.terminal.view.status = "recording operation already in progress".to_string();
            cx.notify();
            return;
        }
        self.terminal.view.status = "starting recording".to_string();
        let manager = self.recording.manager_for_job();
        let writer = self.recording.writer();
        let job_session_id = session_id.to_string();
        let memory_limit = self.settings.summary.recording_memory_limit_bytes as usize;
        let include_io_labels = self.settings.summary.recording_include_io_labels;
        let include_timestamps = self.settings.summary.recording_include_timestamps;
        let task = cx.background_spawn(async move {
            writer.flush();
            manager.set_memory_limit(memory_limit);
            manager
                .start(
                    &job_session_id,
                    &path,
                    include_io_labels,
                    include_timestamps,
                )
                .map(|()| path)
                .map_err(|error| error.to_string())
        });
        let result_session_id = session_id.to_string();
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.recording.finish_action(&result_session_id);
                match result {
                    Ok(path)
                        if this
                            .session
                            .metadata(&result_session_id)
                            .is_some_and(|metadata| !metadata.disconnected) =>
                    {
                        this.recording.refresh_active_count();
                        this.terminal.view.status = format!("recording started: {path}");
                        this.append_terminal_log(format!("\n# recording started: {path}\n"));
                    }
                    Ok(_) => {
                        this.recording.cleanup_writer_session(&result_session_id);
                        this.terminal.view.status =
                            "recording start cancelled because session closed".to_string();
                    }
                    Err(error) => {
                        this.terminal.view.status = format!("recording start failed: {error}");
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features) fn stop_recording_for_session(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        if !self.recording.begin_action(session_id, "record") {
            self.terminal.view.status = "recording operation already in progress".to_string();
            cx.notify();
            return;
        }
        self.terminal.view.status = "stopping recording".to_string();
        let manager = self.recording.manager_for_job();
        let writer = self.recording.writer();
        let job_session_id = session_id.to_string();
        let task = cx.background_spawn(async move {
            writer.flush();
            manager
                .stop(&job_session_id)
                .map_err(|error| error.to_string())
        });
        let result_session_id = session_id.to_string();
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.recording.finish_action(&result_session_id);
                this.recording.refresh_active_count();
                match result {
                    Ok(path) => {
                        this.terminal.view.status = format!("recording saved: {path}");
                        this.append_terminal_log(format!("\n# recording saved: {path}\n"));
                    }
                    Err(error) => {
                        this.terminal.view.status = format!("recording stop failed: {error}");
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn save_transcript_to_path(&mut self, session_id: &str, path: String, cx: &mut Context<Self>) {
        if !self.recording.begin_action(session_id, "save") {
            self.terminal.view.status = "recording operation already in progress".to_string();
            cx.notify();
            return;
        }
        self.terminal.view.status = "saving transcript".to_string();
        let manager = self.recording.manager_for_job();
        let writer = self.recording.writer();
        let job_session_id = session_id.to_string();
        let memory_limit = self.settings.summary.recording_memory_limit_bytes as usize;
        let include_io_labels = self.settings.summary.recording_include_io_labels;
        let include_timestamps = self.settings.summary.recording_include_timestamps;
        let task = cx.background_spawn(async move {
            writer.flush();
            manager.set_memory_limit(memory_limit);
            manager
                .save_transcript(
                    &job_session_id,
                    &path,
                    include_io_labels,
                    include_timestamps,
                )
                .map_err(|error| error.to_string())
        });
        let result_session_id = session_id.to_string();
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.recording.finish_action(&result_session_id);
                match result {
                    Ok(path) => {
                        this.terminal.view.status = format!("transcript saved: {path}");
                        this.append_terminal_log(format!("\n# transcript saved: {path}\n"));
                    }
                    Err(error) => {
                        this.terminal.view.status = format!("transcript save failed: {error}");
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features) fn maybe_auto_start_recording(
        &mut self,
        session_id: &str,
        session_name: &str,
        cx: &mut Context<Self>,
    ) {
        if !self.settings.summary.recording_auto_start {
            return;
        }
        let path = recording_file_path(
            &self.settings.summary,
            self.runtime.config_dir(),
            session_name,
        );
        self.start_recording_to_path(session_id, path.display().to_string(), cx);
    }

    pub(in crate::features) fn cleanup_recording_for_session(&mut self, session_id: &str) {
        self.recording.cleanup_session(session_id);
    }

    pub(in crate::features) fn apply_recording_search(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        self.recording.set_search_draft(text);
        cx.notify();
    }
}
