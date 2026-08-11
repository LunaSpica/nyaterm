use std::path::PathBuf;

use gpui::{AppContext, Context};
use nyaterm_core::{
    AppSettingsSummary, ExistingFileBehavior as CoreExistingFileBehavior,
    RecordingMode as CoreRecordingMode, RecordingRotationPolicy as CoreRecordingRotationPolicy,
};
use nyaterm_transport::{
    ExistingFileBehavior as TransportExistingFileBehavior, RecordingContext, RecordingMode,
    RecordingProfile, RecordingRotationPolicy as TransportRecordingRotationPolicy,
};
use time::OffsetDateTime;

use crate::features::NyaTermApp;
use crate::features::formatting::recording_file_path;
use crate::models::{RecordingPathPromptKind, RecordingPathPromptResult, SessionLaunchConfig};

impl NyaTermApp {
    pub(in crate::features) fn prompt_recording_path_for_session(
        &mut self,
        kind: RecordingPathPromptKind,
        session_id: String,
        session_name: String,
        cx: &mut Context<Self>,
    ) {
        if self.remote_desktop.is_session(&session_id) {
            self.shell
                .set_status("recording is not supported for RDP sessions".to_string());
            cx.notify();
            return;
        }
        if !self.recording.begin_path_prompt(kind) {
            self.shell
                .set_status("recording path picker is already open".to_string());
            cx.notify();
            return;
        }
        let exists = self
            .session
            .metadata(&session_id)
            .is_some_and(|metadata| !metadata.disconnected);
        if !exists {
            self.recording.finish_path_prompt();
            self.shell
                .set_status("session no longer exists".to_string());
            self.remove_session_state(&session_id);
            cx.notify();
            return;
        }
        let target = recording_file_path(
            self.settings.summary(),
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
        self.shell.set_status(match kind {
            RecordingPathPromptKind::Start => "selecting recording path".to_string(),
            RecordingPathPromptKind::SaveTranscript => "selecting transcript path".to_string(),
        });
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
                self.shell.set_status(match kind {
                    RecordingPathPromptKind::Start => "recording start cancelled".to_string(),
                    RecordingPathPromptKind::SaveTranscript => {
                        "transcript save cancelled".to_string()
                    }
                });
            }
            RecordingPathPromptResult::Failed(error) => {
                self.shell
                    .set_status(format!("recording path picker failed: {error}"));
            }
            RecordingPathPromptResult::Closed => {
                self.shell
                    .set_status("recording path picker closed before returning".to_string());
            }
        }
    }

    fn start_recording_to_path(&mut self, session_id: &str, path: String, cx: &mut Context<Self>) {
        self.start_recording_with_profile(session_id, Some(PathBuf::from(path)), cx);
    }

    fn start_recording_with_profile(
        &mut self,
        session_id: &str,
        explicit_path: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        if !self.recording.begin_action(session_id, "record") {
            self.shell
                .set_status("recording operation already in progress".to_string());
            cx.notify();
            return;
        }
        let Some((context, profile)) = self.recording_profile_for_session(session_id) else {
            self.recording.finish_action(session_id);
            self.shell
                .set_status("recording start failed: session no longer exists".to_string());
            cx.notify();
            return;
        };
        self.shell.set_status("starting recording".to_string());
        let manager = self.recording.manager_for_job();
        let writer = self.recording.writer();
        let job_session_id = session_id.to_string();
        let memory_limit = self.settings.summary().recording_memory_limit_bytes as usize;
        let task = cx.background_spawn(async move {
            writer.flush();
            manager.set_memory_limit(memory_limit);
            manager
                .start_with_profile(&job_session_id, context, profile, explicit_path)
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
                        this.shell.set_status(format!("recording started: {path}"));
                        this.append_terminal_log(format!("\n# recording started: {path}\n"));
                    }
                    Ok(_) => {
                        this.recording.cleanup_writer_session(&result_session_id);
                        this.shell.set_status(
                            "recording start cancelled because session closed".to_string(),
                        );
                    }
                    Err(error) => {
                        this.shell
                            .set_status(format!("recording start failed: {error}"));
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
            self.shell
                .set_status("recording operation already in progress".to_string());
            cx.notify();
            return;
        }
        self.shell.set_status("stopping recording".to_string());
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
                        this.shell.set_status(format!("recording saved: {path}"));
                        this.append_terminal_log(format!("\n# recording saved: {path}\n"));
                    }
                    Err(error) => {
                        this.shell
                            .set_status(format!("recording stop failed: {error}"));
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
            self.shell
                .set_status("recording operation already in progress".to_string());
            cx.notify();
            return;
        }
        self.shell.set_status("saving transcript".to_string());
        let manager = self.recording.manager_for_job();
        let writer = self.recording.writer();
        let job_session_id = session_id.to_string();
        let memory_limit = self.settings.summary().recording_memory_limit_bytes as usize;
        let include_io_labels = self.settings.summary().recording_include_io_labels;
        let include_timestamps = self.settings.summary().recording_include_timestamps;
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
                        this.shell.set_status(format!("transcript saved: {path}"));
                        this.append_terminal_log(format!("\n# transcript saved: {path}\n"));
                    }
                    Err(error) => {
                        this.shell
                            .set_status(format!("transcript save failed: {error}"));
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
        if !self.effective_recording_auto_start(session_id) {
            return;
        }
        let _ = session_name;
        self.start_recording_with_profile(session_id, None, cx);
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

impl NyaTermApp {
    fn recording_profile_for_session(
        &self,
        session_id: &str,
    ) -> Option<(RecordingContext, RecordingProfile)> {
        let metadata = self.session.metadata(session_id)?;
        if metadata.disconnected {
            return None;
        }
        let summary = self.settings.summary();
        let (session_name, protocol, host, port, username) =
            recording_launch_context(&metadata.launch_config);
        let context = RecordingContext {
            session_id: session_id.to_string(),
            session_name,
            connection_id: metadata.source_connection_id.clone(),
            connection_name: metadata.source_connection_id.clone(),
            group_path: None,
            protocol,
            host,
            port,
            username,
            started_at: OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc()),
        };
        let connection_recording = metadata
            .source_connection_id
            .as_deref()
            .and_then(|connection_id| {
                self.connection_state
                    .connections()
                    .iter()
                    .find(|connection| connection.id == connection_id)
            })
            .and_then(|connection| connection.recording.as_ref());
        let profile = RecordingProfile {
            mode: connection_recording
                .and_then(|settings| settings.mode)
                .map(map_recording_mode)
                .unwrap_or_else(|| map_recording_mode(summary.recording_default_mode)),
            base_path: recording_base_path(summary, self.runtime.config_dir()),
            path_template: connection_recording
                .and_then(|settings| settings.path_template.as_ref())
                .filter(|value| !value.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| summary.recording_path_template.clone()),
            include_timestamps: connection_recording
                .and_then(|settings| settings.include_timestamps)
                .unwrap_or(summary.recording_include_timestamps),
            include_io_labels: summary.recording_include_io_labels,
            include_session_metadata: summary.recording_include_session_metadata,
            rotation: connection_recording
                .and_then(|settings| settings.rotation.as_ref())
                .map(map_recording_rotation)
                .unwrap_or_else(|| map_recording_rotation(&summary.recording_rotation)),
            existing_file_behavior: map_existing_file_behavior(
                summary.recording_existing_file_behavior,
            ),
            include_binary_transfer_payloads: summary.recording_include_binary_transfer_payloads,
        };
        Some((context, profile))
    }

    fn effective_recording_auto_start(&self, session_id: &str) -> bool {
        let Some(metadata) = self.session.metadata(session_id) else {
            return false;
        };
        metadata
            .source_connection_id
            .as_deref()
            .and_then(|connection_id| {
                self.connection_state
                    .connections()
                    .iter()
                    .find(|connection| connection.id == connection_id)
            })
            .and_then(|connection| connection.recording.as_ref())
            .and_then(|settings| settings.auto_start)
            .unwrap_or(self.settings.summary().recording_auto_start)
    }
}

fn recording_base_path(settings: &AppSettingsSummary, config_dir: &std::path::Path) -> PathBuf {
    if settings.recording_path.trim().is_empty() {
        dirs::download_dir().unwrap_or_else(|| config_dir.join("recordings"))
    } else {
        PathBuf::from(settings.recording_path.trim())
    }
}

fn recording_launch_context(
    launch_config: &SessionLaunchConfig,
) -> (String, String, Option<String>, Option<u16>, Option<String>) {
    match launch_config {
        SessionLaunchConfig::Local(config) => {
            (config.name.clone(), "local".to_string(), None, None, None)
        }
        SessionLaunchConfig::Ssh(config) => (
            config.name.clone(),
            "ssh".to_string(),
            Some(config.host.clone()),
            Some(config.port),
            Some(config.username.clone()),
        ),
        SessionLaunchConfig::Telnet(config) => (
            config.name.clone(),
            "telnet".to_string(),
            Some(config.host.clone()),
            Some(config.port),
            Some(config.username.clone()),
        ),
        SessionLaunchConfig::Serial(config) => (
            config.name.clone(),
            "serial".to_string(),
            Some(config.port_name.clone()),
            None,
            None,
        ),
        SessionLaunchConfig::Rdp(config) => (
            config.name.clone(),
            "rdp".to_string(),
            Some(config.host.clone()),
            Some(config.port),
            Some(config.username.clone()),
        ),
    }
}

fn map_recording_mode(mode: CoreRecordingMode) -> RecordingMode {
    match mode {
        CoreRecordingMode::Raw => RecordingMode::Raw,
        CoreRecordingMode::Transcript => RecordingMode::Transcript,
    }
}

fn map_existing_file_behavior(behavior: CoreExistingFileBehavior) -> TransportExistingFileBehavior {
    match behavior {
        CoreExistingFileBehavior::Append => TransportExistingFileBehavior::Append,
        CoreExistingFileBehavior::Overwrite => TransportExistingFileBehavior::Overwrite,
        CoreExistingFileBehavior::Unique => TransportExistingFileBehavior::Unique,
    }
}

fn map_recording_rotation(
    rotation: &CoreRecordingRotationPolicy,
) -> TransportRecordingRotationPolicy {
    match rotation {
        CoreRecordingRotationPolicy::Daily => TransportRecordingRotationPolicy::Daily,
        CoreRecordingRotationPolicy::Size { max_bytes } => TransportRecordingRotationPolicy::Size {
            max_bytes: *max_bytes,
        },
        CoreRecordingRotationPolicy::Session => TransportRecordingRotationPolicy::Session,
    }
}
