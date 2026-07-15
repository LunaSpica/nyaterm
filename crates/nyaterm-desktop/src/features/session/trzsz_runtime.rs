use super::*;
use nyaterm_transport::{
    TrzszAction, TrzszDetector, TrzszDownloadEngine, TrzszDownloadEvent, TrzszMode,
    TrzszOutputEvent, TrzszProtocolFrame, TrzszProtocolStream, TrzszTransferEvent,
    TrzszTransferState, TrzszUploadEngine, TrzszUploadEntry, TrzszUploadEvent, TrzszUploadSource,
    build_trzsz_action_frame, build_trzsz_string_frame, trzsz_fail_response,
};
use std::{
    collections::{HashMap, HashSet},
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub(in crate::features) struct TrzszSessionState {
    pub(in crate::features) detector: TrzszDetector,
    pub(in crate::features) transfer: TrzszTransferState,
    pub(in crate::features) protocol: TrzszProtocolStream,
    pub(in crate::features) protocol_active: bool,
    download: Option<TrzszDownloadRuntime>,
    upload: Option<TrzszUploadRuntime>,
}

struct TrzszDownloadRuntime {
    engine: TrzszDownloadEngine,
    directory: PathBuf,
    directory_roots: HashMap<i64, String>,
    pending_path: Option<TrzszDownloadPath>,
    current_file: Option<TrzszDownloadFile>,
}

#[derive(Debug, Clone)]
struct TrzszDownloadPath {
    path_id: i64,
    components: Vec<String>,
}

struct TrzszDownloadFile {
    name: String,
    path: PathBuf,
    file: File,
    size: u64,
}

struct TrzszDownloadProgressUpdate {
    file_name: String,
    local_path: PathBuf,
    bytes_transferred: u64,
    total_bytes: Option<u64>,
    completed: bool,
    fail_reason: Option<String>,
}

struct TrzszUploadRuntime {
    engine: TrzszUploadEngine,
    files: HashMap<String, TrzszUploadFile>,
    remote_names: HashMap<String, String>,
    directory_mode: bool,
}

struct TrzszUploadFile {
    local_path: PathBuf,
    size: u64,
    is_dir: bool,
}

struct TrzszUploadProgressUpdate {
    file_name: String,
    remote_name: String,
    local_path: PathBuf,
    bytes_transferred: u64,
    total_bytes: Option<u64>,
    completed: bool,
    fail_reason: Option<String>,
}

impl Default for TrzszSessionState {
    fn default() -> Self {
        Self {
            detector: TrzszDetector::new(),
            transfer: TrzszTransferState::new(),
            protocol: TrzszProtocolStream::new(),
            protocol_active: false,
            download: None,
            upload: None,
        }
    }
}

impl NyaTermApp {
    fn trzsz_state_mut(&mut self, session_id: &str) -> &mut TrzszSessionState {
        self.trzsz_sessions
            .entry(session_id.to_string())
            .or_default()
    }

    pub(in crate::features) fn clear_trzsz_session(&mut self, session_id: &str) {
        self.trzsz_sessions.remove(session_id);
    }

    pub(in crate::features) fn note_trzsz_output_discontinuity(&mut self, session_id: &str) {
        if let Some(state) = self.trzsz_sessions.get_mut(session_id) {
            state.detector.reset();
            state.transfer = TrzszTransferState::new();
            state.protocol.reset();
            state.protocol_active = false;
            state.download = None;
            state.upload = None;
        }
    }

    /// Consume trzsz marker and protocol bytes before they reach the terminal
    /// parser. Remote `tsz` downloads are handled locally; unsupported upload
    /// modes are rejected with protocol-level failure frames.
    pub(in crate::features) fn process_trzsz_output(
        &mut self,
        session_id: &str,
        data: &[u8],
        cx: &mut Context<Self>,
    ) -> Vec<u8> {
        if data.is_empty() {
            return Vec::new();
        }
        let events = {
            let state = self.trzsz_state_mut(session_id);
            state.detector.scan_terminal_output(data).events
        };

        let mut passthrough = Vec::new();
        let mut protocol_responses = Vec::new();
        let mut latest_trigger_status = None;
        let mut latest_protocol_status = None;
        let mut response_error = false;

        for event in events {
            match event {
                TrzszOutputEvent::Passthrough(bytes) => {
                    let mut protocol_status = None;
                    let protocol_output = {
                        let state = self.trzsz_state_mut(session_id);
                        if !state.protocol_active {
                            passthrough.extend(bytes);
                            continue;
                        }

                        let protocol_output = state.protocol.filter_terminal_output(&bytes);
                        protocol_output
                    };
                    for frame in protocol_output.frames.clone() {
                        self.handle_trzsz_protocol_frame(
                            session_id,
                            frame,
                            &mut protocol_responses,
                            &mut protocol_status,
                            cx,
                        );
                    }
                    passthrough.extend(protocol_output.passthrough);
                    if protocol_status.is_some() {
                        latest_protocol_status = protocol_status;
                    }
                }
                TrzszOutputEvent::Trigger(trigger) => {
                    let action = match trigger.mode {
                        TrzszMode::Send => "send",
                        TrzszMode::Receive => "receive",
                        TrzszMode::Directory => "directory",
                    };
                    let version = trigger.version.as_str();
                    let server = if trigger.remote_is_windows {
                        " Windows server"
                    } else {
                        ""
                    };
                    if trigger.mode == TrzszMode::Send {
                        let Some(directory) = self.prepare_trzsz_download_dir(cx) else {
                            protocol_responses.push(trzsz_fail_response(
                                "trzsz download directory is not available",
                                trigger.remote_is_windows,
                            ));
                            continue;
                        };
                        let action = TrzszAction::local_default(trigger.remote_is_windows);
                        let action_frame =
                            build_trzsz_action_frame(&action, trigger.remote_is_windows);
                        {
                            let state = self.trzsz_state_mut(session_id);
                            state.transfer.observe_trigger(&trigger);
                            state.protocol.reset();
                            state.protocol_active = true;
                            state.download = Some(TrzszDownloadRuntime {
                                engine: TrzszDownloadEngine::new(trigger.remote_is_windows),
                                directory: directory.clone(),
                                directory_roots: HashMap::new(),
                                pending_path: None,
                                current_file: None,
                            });
                            state.upload = None;
                        }
                        protocol_responses.push(action_frame);
                        latest_trigger_status = Some(format!(
                            "trzsz download accepted (v{version}{server}) -> {}",
                            directory.display()
                        ));
                    } else if trigger.mode == TrzszMode::Receive {
                        {
                            let state = self.trzsz_state_mut(session_id);
                            state.transfer.observe_trigger(&trigger);
                            state.protocol.reset();
                            state.protocol_active = true;
                            state.download = None;
                            state.upload = None;
                        }
                        if self.prompt_trzsz_upload_paths(
                            session_id.to_string(),
                            trigger.remote_is_windows,
                            false,
                            cx,
                        ) {
                            latest_trigger_status = Some(format!(
                                "trzsz upload requested (v{version}{server}) - select local files"
                            ));
                        } else {
                            protocol_responses.push(trzsz_fail_response(
                                "trzsz upload file picker is not available",
                                trigger.remote_is_windows,
                            ));
                            if let Some(state) = self.trzsz_sessions.get_mut(session_id) {
                                state.protocol_active = false;
                                state.protocol.reset();
                                state.download = None;
                                state.upload = None;
                            }
                            latest_trigger_status = Some(format!(
                                "trzsz {action} trigger rejected (v{version}{server}) - file picker busy"
                            ));
                        }
                    } else {
                        {
                            let state = self.trzsz_state_mut(session_id);
                            state.transfer.observe_trigger(&trigger);
                            state.protocol.reset();
                            state.protocol_active = true;
                            state.download = None;
                            state.upload = None;
                        }
                        if self.prompt_trzsz_upload_paths(
                            session_id.to_string(),
                            trigger.remote_is_windows,
                            true,
                            cx,
                        ) {
                            latest_trigger_status = Some(format!(
                                "trzsz directory upload requested (v{version}{server}) - select local directories"
                            ));
                        } else {
                            protocol_responses.push(trzsz_fail_response(
                                "trzsz upload file picker is not available",
                                trigger.remote_is_windows,
                            ));
                            if let Some(state) = self.trzsz_sessions.get_mut(session_id) {
                                state.protocol_active = false;
                                state.protocol.reset();
                                state.download = None;
                                state.upload = None;
                            }
                            latest_trigger_status = Some(format!(
                                "trzsz {action} trigger rejected (v{version}{server}) - file picker busy"
                            ));
                        }
                    }
                }
            }
        }

        for response in protocol_responses {
            if let Err(error) = self.write_session_protocol_response(session_id, &response) {
                self.terminal_status = format!("trzsz protocol response failed: {error}");
                response_error = true;
                cx.notify();
            }
        }

        if !response_error && let Some(status) = latest_protocol_status.or(latest_trigger_status) {
            self.terminal_status = status;
            cx.notify();
        }
        passthrough
    }

    fn prepare_trzsz_download_dir(&mut self, cx: &mut Context<Self>) -> Option<PathBuf> {
        let Some(directory) = self.resolved_transfer_download_dir() else {
            self.terminal_status = "cannot determine trzsz download directory".to_string();
            cx.notify();
            return None;
        };
        if directory.exists() && !directory.is_dir() {
            self.terminal_status = format!(
                "trzsz download path is not a directory: {}",
                directory.display()
            );
            cx.notify();
            return None;
        }
        if let Err(error) = std::fs::create_dir_all(&directory) {
            self.terminal_status = format!("failed to prepare trzsz download directory: {error}");
            cx.notify();
            return None;
        }
        Some(directory)
    }

    fn prompt_trzsz_upload_paths(
        &mut self,
        session_id: String,
        remote_is_windows: bool,
        directory_mode: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.transfer_path_prompt.is_some() {
            self.terminal_status = "native path picker is already open".to_string();
            cx.notify();
            return false;
        }

        let options = PathPromptOptions {
            files: !directory_mode,
            directories: directory_mode,
            multiple: true,
            prompt: Some(SharedString::from(if directory_mode {
                "Select trzsz upload directories"
            } else {
                "Select trzsz upload files"
            })),
        };
        let receiver = cx.prompt_for_paths(options);
        self.transfer_path_prompt = Some(if directory_mode {
            TransferPathPromptKind::UploadDirectory
        } else {
            TransferPathPromptKind::UploadFile
        });
        self.terminal_status = if directory_mode {
            "selecting trzsz upload directories".to_string()
        } else {
            "selecting trzsz upload files".to_string()
        };
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => {
                    if paths.is_empty() {
                        TransferPathPromptResult::Cancelled
                    } else {
                        TransferPathPromptResult::Selected(paths)
                    }
                }
                Ok(Ok(None)) => TransferPathPromptResult::Cancelled,
                Ok(Err(error)) => TransferPathPromptResult::Failed(error.to_string()),
                Err(_) => TransferPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_trzsz_upload_path_prompt_result(
                    session_id,
                    remote_is_windows,
                    directory_mode,
                    result,
                    cx,
                );
                cx.notify();
            });
        })
        .detach();
        cx.notify();
        true
    }

    fn apply_trzsz_upload_path_prompt_result(
        &mut self,
        session_id: String,
        remote_is_windows: bool,
        directory_mode: bool,
        result: TransferPathPromptResult,
        cx: &mut Context<Self>,
    ) {
        self.transfer_path_prompt = None;
        match result {
            TransferPathPromptResult::Selected(paths) => {
                self.accept_trzsz_upload_paths(
                    &session_id,
                    remote_is_windows,
                    directory_mode,
                    paths,
                    cx,
                );
            }
            TransferPathPromptResult::Cancelled => {
                self.reject_trzsz_upload_prompt(
                    &session_id,
                    remote_is_windows,
                    "trzsz upload selection cancelled",
                    cx,
                );
            }
            TransferPathPromptResult::Failed(error) => {
                self.reject_trzsz_upload_prompt(
                    &session_id,
                    remote_is_windows,
                    &format!("trzsz upload path picker failed: {error}"),
                    cx,
                );
            }
            TransferPathPromptResult::Closed => {
                self.reject_trzsz_upload_prompt(
                    &session_id,
                    remote_is_windows,
                    "trzsz upload path picker closed before returning",
                    cx,
                );
            }
        }
    }

    fn accept_trzsz_upload_paths(
        &mut self,
        session_id: &str,
        remote_is_windows: bool,
        directory_mode: bool,
        paths: Vec<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        let (entries, files) = match prepare_trzsz_upload_entries(paths, directory_mode) {
            Ok(value) => value,
            Err(error) => {
                self.reject_trzsz_upload_prompt(session_id, remote_is_windows, &error, cx);
                return;
            }
        };
        let file_count = entries.len();
        {
            let state = self.trzsz_state_mut(session_id);
            state.protocol_active = true;
            state.download = None;
            state.upload = Some(TrzszUploadRuntime {
                engine: TrzszUploadEngine::new(remote_is_windows, entries),
                files,
                remote_names: HashMap::new(),
                directory_mode,
            });
        }

        let action = TrzszAction::local_default(remote_is_windows);
        let action_frame = build_trzsz_action_frame(&action, remote_is_windows);
        match self.write_session_protocol_response(session_id, &action_frame) {
            Ok(()) => {
                self.terminal_status =
                    format!("trzsz upload accepted ({file_count} file(s)) [{session_id}]");
            }
            Err(error) => {
                if let Some(state) = self.trzsz_sessions.get_mut(session_id) {
                    state.upload = None;
                    state.protocol_active = false;
                    state.protocol.reset();
                }
                self.terminal_status = format!("trzsz upload ACT failed: {error}");
            }
        }
        cx.notify();
    }

    fn reject_trzsz_upload_prompt(
        &mut self,
        session_id: &str,
        remote_is_windows: bool,
        reason: &str,
        cx: &mut Context<Self>,
    ) {
        let fail = trzsz_fail_response(reason, remote_is_windows);
        if let Err(error) = self.write_session_protocol_response(session_id, &fail) {
            self.terminal_status = format!("trzsz upload reject failed: {error}");
        } else {
            self.terminal_status = reason.to_string();
        }
        if let Some(state) = self.trzsz_sessions.get_mut(session_id) {
            state.upload = None;
            state.protocol_active = false;
            state.protocol.reset();
        }
        cx.notify();
    }

    fn handle_trzsz_protocol_frame(
        &mut self,
        session_id: &str,
        frame: TrzszProtocolFrame,
        responses: &mut Vec<Vec<u8>>,
        status: &mut Option<String>,
        cx: &mut Context<Self>,
    ) {
        let transfer_event = {
            let state = self.trzsz_state_mut(session_id);
            state.transfer.observe_frame(frame.clone())
        };
        match transfer_event {
            TrzszTransferEvent::Config { config } => {
                if let Some(download) = self
                    .trzsz_sessions
                    .get_mut(session_id)
                    .and_then(|state| state.download.as_mut())
                {
                    download.engine.set_directory_mode(config.directory);
                    if config.directory {
                        *status = Some("trzsz directory download accepted".to_string());
                    }
                }
                if self
                    .trzsz_sessions
                    .get(session_id)
                    .is_some_and(|state| state.upload.is_some())
                {
                    let expected_directory = self
                        .trzsz_sessions
                        .get(session_id)
                        .and_then(|state| state.upload.as_ref())
                        .is_some_and(|upload| upload.directory_mode);
                    if config.directory != expected_directory {
                        let reason = if expected_directory {
                            "remote trzsz config did not enable directory upload"
                        } else {
                            "remote trzsz config unexpectedly enabled directory upload"
                        };
                        self.fail_trzsz_upload(session_id, reason, responses, status, cx);
                        return;
                    }
                    self.begin_trzsz_upload(session_id, responses, status, cx);
                    return;
                }
            }
            TrzszTransferEvent::Failure { message } | TrzszTransferEvent::Exit { message } => {
                self.finish_trzsz_download_jobs(session_id, false, Some(&message), cx);
                self.finish_trzsz_upload_jobs(session_id, false, Some(&message), cx);
                if let Some(state) = self.trzsz_sessions.get_mut(session_id) {
                    state.download = None;
                    state.upload = None;
                    state.protocol_active = false;
                    state.protocol.reset();
                }
                *status = Some(format!("trzsz transfer stopped: {message}"));
                return;
            }
            _ => {}
        }

        if self
            .trzsz_sessions
            .get(session_id)
            .is_some_and(|state| state.upload.is_some())
        {
            self.handle_trzsz_upload_frame(session_id, frame, responses, status, cx);
            return;
        }

        let mut progress_updates = Vec::new();
        let mut download_completed = None;
        let mut download_error = None;
        {
            let state = self.trzsz_state_mut(session_id);
            let Some(download) = state.download.as_mut() else {
                return;
            };
            if !is_trzsz_download_frame(&frame) {
                return;
            }
            match download.engine.observe_frame(frame) {
                Ok(step) => {
                    responses.extend(step.responses);
                    for event in step.events {
                        match apply_trzsz_download_event(download, event) {
                            Ok(TrzszDownloadRuntimeUpdate::None) => {}
                            Ok(TrzszDownloadRuntimeUpdate::Progress(update)) => {
                                progress_updates.push(update);
                            }
                            Ok(TrzszDownloadRuntimeUpdate::Completed(names)) => {
                                download_completed = Some(names);
                            }
                            Err(error) => {
                                download_error = Some(error);
                                break;
                            }
                        }
                    }
                }
                Err(error) => {
                    download_error = Some(format!("{error:?}"));
                }
            }
        }

        for update in progress_updates {
            self.update_trzsz_download_job(session_id, update, cx);
        }
        if let Some(error) = download_error {
            self.fail_trzsz_download(session_id, &error, responses, status, cx);
            return;
        }
        if let Some(names) = download_completed {
            let message = if names.is_empty() {
                "trzsz download complete".to_string()
            } else {
                format!("Saved {}", names.join(", "))
            };
            let newline = if self
                .trzsz_sessions
                .get(session_id)
                .is_some_and(|state| state.transfer.remote_is_windows)
            {
                "!\n"
            } else {
                "\n"
            };
            responses.push(build_trzsz_string_frame(
                "EXIT",
                message.as_bytes(),
                newline,
            ));
            self.finish_trzsz_download_jobs(session_id, true, None, cx);
            if let Some(state) = self.trzsz_sessions.get_mut(session_id) {
                state.download = None;
                state.protocol_active = false;
                state.protocol.reset();
            }
            *status = Some(message);
        }
    }

    fn fail_trzsz_download(
        &mut self,
        session_id: &str,
        reason: &str,
        responses: &mut Vec<Vec<u8>>,
        status: &mut Option<String>,
        cx: &mut Context<Self>,
    ) {
        let remote_is_windows = self
            .trzsz_sessions
            .get(session_id)
            .map(|state| state.transfer.remote_is_windows)
            .unwrap_or(false);
        responses.push(trzsz_fail_response(reason, remote_is_windows));
        self.finish_trzsz_download_jobs(session_id, false, Some(reason), cx);
        if let Some(state) = self.trzsz_sessions.get_mut(session_id) {
            state.download = None;
            state.protocol_active = false;
            state.protocol.reset();
        }
        *status = Some(format!("trzsz download failed: {reason}"));
    }

    fn begin_trzsz_upload(
        &mut self,
        session_id: &str,
        responses: &mut Vec<Vec<u8>>,
        status: &mut Option<String>,
        cx: &mut Context<Self>,
    ) {
        let mut upload_events = Vec::new();
        let mut upload_error = None;
        {
            let state = self.trzsz_state_mut(session_id);
            let Some(upload) = state.upload.as_mut() else {
                return;
            };
            match upload.engine.begin() {
                Ok(step) => {
                    responses.extend(step.responses);
                    upload_events.extend(step.events);
                }
                Err(error) => upload_error = Some(format!("{error:?}")),
            }
        }
        if let Some(error) = upload_error {
            self.fail_trzsz_upload(session_id, &error, responses, status, cx);
            return;
        }
        let count = upload_events
            .iter()
            .find_map(|event| match event {
                TrzszUploadEvent::Started { count } => Some(*count),
                _ => None,
            })
            .unwrap_or(0);
        *status = Some(format!("trzsz upload started ({count} file(s))"));
    }

    fn handle_trzsz_upload_frame(
        &mut self,
        session_id: &str,
        frame: TrzszProtocolFrame,
        responses: &mut Vec<Vec<u8>>,
        status: &mut Option<String>,
        cx: &mut Context<Self>,
    ) {
        if !is_trzsz_upload_frame(&frame) {
            return;
        }

        let mut progress_updates = Vec::new();
        let mut upload_completed = None;
        let mut upload_error = None;
        {
            let state = self.trzsz_state_mut(session_id);
            let Some(upload) = state.upload.as_mut() else {
                return;
            };
            match upload.engine.observe_frame(frame) {
                Ok(step) => {
                    responses.extend(step.responses);
                    for event in step.events {
                        match apply_trzsz_upload_event(upload, event) {
                            TrzszUploadRuntimeUpdate::None => {}
                            TrzszUploadRuntimeUpdate::Progress(update) => {
                                progress_updates.push(update);
                            }
                            TrzszUploadRuntimeUpdate::Completed(names) => {
                                upload_completed = Some(names);
                            }
                        }
                    }
                }
                Err(error) => upload_error = Some(format!("{error:?}")),
            }
        }

        for update in progress_updates {
            self.update_trzsz_upload_job(session_id, update, cx);
        }
        if let Some(error) = upload_error {
            self.fail_trzsz_upload(session_id, &error, responses, status, cx);
            return;
        }
        if let Some(names) = upload_completed {
            let message = if names.is_empty() {
                "trzsz upload complete".to_string()
            } else {
                format!("Uploaded {}", names.join(", "))
            };
            let newline = if self
                .trzsz_sessions
                .get(session_id)
                .is_some_and(|state| state.transfer.remote_is_windows)
            {
                "!\n"
            } else {
                "\n"
            };
            responses.push(build_trzsz_string_frame(
                "EXIT",
                message.as_bytes(),
                newline,
            ));
            self.finish_trzsz_upload_jobs(session_id, true, None, cx);
            if let Some(state) = self.trzsz_sessions.get_mut(session_id) {
                state.upload = None;
                state.protocol_active = false;
                state.protocol.reset();
            }
            *status = Some(message);
        }
    }

    fn fail_trzsz_upload(
        &mut self,
        session_id: &str,
        reason: &str,
        responses: &mut Vec<Vec<u8>>,
        status: &mut Option<String>,
        cx: &mut Context<Self>,
    ) {
        let remote_is_windows = self
            .trzsz_sessions
            .get(session_id)
            .map(|state| state.transfer.remote_is_windows)
            .unwrap_or(false);
        responses.push(trzsz_fail_response(reason, remote_is_windows));
        self.finish_trzsz_upload_jobs(session_id, false, Some(reason), cx);
        if let Some(state) = self.trzsz_sessions.get_mut(session_id) {
            state.upload = None;
            state.protocol_active = false;
            state.protocol.reset();
        }
        *status = Some(format!("trzsz upload failed: {reason}"));
    }

    fn update_trzsz_download_job(
        &mut self,
        session_id: &str,
        update: TrzszDownloadProgressUpdate,
        cx: &mut Context<Self>,
    ) {
        let short = short_id(session_id);
        let progress = SftpTransferProgress {
            remote_path: format!("trzsz://{short}/{}", update.file_name),
            local_path: update.local_path.clone(),
            bytes_transferred: update.bytes_transferred,
            total_bytes: update.total_bytes,
        };
        if let Some(job) = self.transfer_jobs.iter_mut().find(|job| {
            matches!(
                &job.kind,
                TransferJobKind::TrzszDownload {
                    session_id: sid,
                    file_name,
                } if sid == session_id && file_name == &update.file_name
            ) && matches!(
                job.status,
                TransferJobStatus::Running | TransferJobStatus::Cancelling
            )
        }) {
            job.progress = Some(progress);
            job.detail = if update.completed {
                "Complete".to_string()
            } else if let Some(reason) = update.fail_reason.as_deref() {
                format!("Failed: {reason}")
            } else if let Some(total) = update.total_bytes.filter(|total| *total > 0) {
                format!(
                    "{:.0}%",
                    (update.bytes_transferred as f64 / total as f64 * 100.).clamp(0., 100.)
                )
            } else {
                format!("{} bytes", update.bytes_transferred)
            };
            if update.completed {
                job.status = TransferJobStatus::Completed;
            } else if update.fail_reason.is_some() {
                job.status = TransferJobStatus::Failed;
            }
            cx.notify();
            return;
        }

        let id = self.next_transfer_id("trzsz-download");
        let status = if update.completed {
            TransferJobStatus::Completed
        } else if update.fail_reason.is_some() {
            TransferJobStatus::Failed
        } else {
            TransferJobStatus::Running
        };
        let detail = update
            .fail_reason
            .as_deref()
            .map(|reason| format!("Failed: {reason}"))
            .unwrap_or_else(|| {
                if update.completed {
                    "Complete".to_string()
                } else {
                    format!("Downloading {}", update.file_name)
                }
            });
        self.transfer_jobs.push(TransferJobState {
            id,
            kind: TransferJobKind::TrzszDownload {
                session_id: session_id.to_string(),
                file_name: update.file_name,
            },
            status,
            detail,
            entries: Vec::new(),
            summary: None,
            progress: Some(progress),
            control: None,
        });
        cx.notify();
    }

    fn update_trzsz_upload_job(
        &mut self,
        session_id: &str,
        update: TrzszUploadProgressUpdate,
        cx: &mut Context<Self>,
    ) {
        let short = short_id(session_id);
        let progress = SftpTransferProgress {
            remote_path: format!("trzsz://{short}/{}", update.remote_name),
            local_path: update.local_path.clone(),
            bytes_transferred: update.bytes_transferred,
            total_bytes: update.total_bytes,
        };
        if let Some(job) = self.transfer_jobs.iter_mut().find(|job| {
            matches!(
                &job.kind,
                TransferJobKind::TrzszUpload {
                    session_id: sid,
                    file_name,
                } if sid == session_id && file_name == &update.file_name
            ) && matches!(
                job.status,
                TransferJobStatus::Running | TransferJobStatus::Cancelling
            )
        }) {
            job.progress = Some(progress);
            job.detail = if update.completed {
                "Complete".to_string()
            } else if let Some(reason) = update.fail_reason.as_deref() {
                format!("Failed: {reason}")
            } else if let Some(total) = update.total_bytes.filter(|total| *total > 0) {
                format!(
                    "{:.0}%",
                    (update.bytes_transferred as f64 / total as f64 * 100.).clamp(0., 100.)
                )
            } else {
                format!("{} bytes", update.bytes_transferred)
            };
            if update.completed {
                job.status = TransferJobStatus::Completed;
            } else if update.fail_reason.is_some() {
                job.status = TransferJobStatus::Failed;
            }
            cx.notify();
            return;
        }

        let id = self.next_transfer_id("trzsz-upload");
        let status = if update.completed {
            TransferJobStatus::Completed
        } else if update.fail_reason.is_some() {
            TransferJobStatus::Failed
        } else {
            TransferJobStatus::Running
        };
        let detail = update
            .fail_reason
            .as_deref()
            .map(|reason| format!("Failed: {reason}"))
            .unwrap_or_else(|| {
                if update.completed {
                    "Complete".to_string()
                } else {
                    format!("Uploading {}", update.file_name)
                }
            });
        self.transfer_jobs.push(TransferJobState {
            id,
            kind: TransferJobKind::TrzszUpload {
                session_id: session_id.to_string(),
                file_name: update.file_name,
            },
            status,
            detail,
            entries: Vec::new(),
            summary: None,
            progress: Some(progress),
            control: None,
        });
        cx.notify();
    }

    fn finish_trzsz_download_jobs(
        &mut self,
        session_id: &str,
        success: bool,
        fail_reason: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        for job in &mut self.transfer_jobs {
            let is_trzsz = matches!(
                &job.kind,
                TransferJobKind::TrzszDownload {
                    session_id: sid,
                    ..
                } if sid == session_id
            );
            if !is_trzsz
                || !matches!(
                    job.status,
                    TransferJobStatus::Running | TransferJobStatus::Cancelling
                )
            {
                continue;
            }
            if success {
                job.status = TransferJobStatus::Completed;
                job.detail = "Complete".to_string();
            } else {
                job.status = TransferJobStatus::Failed;
                job.detail = fail_reason
                    .map(|reason| format!("Failed: {reason}"))
                    .unwrap_or_else(|| "Failed".to_string());
            }
        }
        cx.notify();
    }

    fn finish_trzsz_upload_jobs(
        &mut self,
        session_id: &str,
        success: bool,
        fail_reason: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        for job in &mut self.transfer_jobs {
            let is_trzsz = matches!(
                &job.kind,
                TransferJobKind::TrzszUpload {
                    session_id: sid,
                    ..
                } if sid == session_id
            );
            if !is_trzsz
                || !matches!(
                    job.status,
                    TransferJobStatus::Running | TransferJobStatus::Cancelling
                )
            {
                continue;
            }
            if success {
                job.status = TransferJobStatus::Completed;
                job.detail = "Complete".to_string();
            } else {
                job.status = TransferJobStatus::Failed;
                job.detail = fail_reason
                    .map(|reason| format!("Failed: {reason}"))
                    .unwrap_or_else(|| "Failed".to_string());
            }
        }
        cx.notify();
    }
}

enum TrzszDownloadRuntimeUpdate {
    None,
    Progress(TrzszDownloadProgressUpdate),
    Completed(Vec<String>),
}

enum TrzszUploadRuntimeUpdate {
    None,
    Progress(TrzszUploadProgressUpdate),
    Completed(Vec<String>),
}

fn is_trzsz_download_frame(frame: &TrzszProtocolFrame) -> bool {
    matches!(
        frame.frame_type.to_ascii_uppercase().as_str(),
        "NUM" | "NAME" | "SIZE" | "DATA" | "MD5"
    )
}

fn is_trzsz_upload_frame(frame: &TrzszProtocolFrame) -> bool {
    frame.frame_type.eq_ignore_ascii_case("SUCC")
}

fn prepare_trzsz_upload_entries(
    paths: Vec<PathBuf>,
    directory_mode: bool,
) -> Result<(Vec<TrzszUploadEntry>, HashMap<String, TrzszUploadFile>), String> {
    if paths.is_empty() {
        return Err("trzsz upload selection cancelled".to_string());
    }

    let mut used_names = HashSet::new();
    let mut entries = Vec::new();
    let mut files = HashMap::new();
    for (path_id, path) in paths.into_iter().enumerate() {
        let metadata = path
            .metadata()
            .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
        if directory_mode {
            let root_name = unique_trzsz_upload_name(&path, &mut used_names);
            let mut visited_dirs = HashSet::new();
            append_trzsz_upload_path(
                path_id as i64,
                &path,
                metadata,
                vec![root_name],
                &mut visited_dirs,
                &mut entries,
                &mut files,
            )?;
            continue;
        }

        if metadata.is_dir() {
            return Err(format!(
                "trzsz upload path is a directory: {}",
                path.display()
            ));
        }
        if !metadata.is_file() {
            return Err(format!(
                "trzsz upload path is not a file: {}",
                path.display()
            ));
        }
        let name = unique_trzsz_upload_name(&path, &mut used_names);
        let data = std::fs::read(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let size = data.len() as u64;
        entries.push(TrzszUploadEntry {
            name: name.clone(),
            data,
            source: None,
        });
        files.insert(
            name,
            TrzszUploadFile {
                local_path: path,
                size,
                is_dir: false,
            },
        );
    }

    if entries.is_empty() {
        Err("trzsz upload selection cancelled".to_string())
    } else {
        Ok((entries, files))
    }
}

fn append_trzsz_upload_path(
    path_id: i64,
    path: &Path,
    metadata: std::fs::Metadata,
    components: Vec<String>,
    visited_dirs: &mut HashSet<PathBuf>,
    entries: &mut Vec<TrzszUploadEntry>,
    files: &mut HashMap<String, TrzszUploadFile>,
) -> Result<(), String> {
    let entry_name = components.join("/");
    let perm = trzsz_upload_perm(&metadata);
    if metadata.is_dir() {
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("failed to resolve {}: {error}", path.display()))?;
        if !visited_dirs.insert(canonical) {
            return Err(format!("trzsz upload directory cycle: {}", path.display()));
        }
        entries.push(TrzszUploadEntry {
            name: entry_name.clone(),
            data: Vec::new(),
            source: Some(TrzszUploadSource {
                path_id,
                path_name: components.clone(),
                is_dir: true,
                size: 0,
                perm,
            }),
        });
        files.insert(
            entry_name,
            TrzszUploadFile {
                local_path: path.to_path_buf(),
                size: 0,
                is_dir: true,
            },
        );

        let mut children = std::fs::read_dir(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            let child_path = child.path();
            let child_metadata = child
                .metadata()
                .map_err(|error| format!("failed to inspect {}: {error}", child_path.display()))?;
            let mut child_components = components.clone();
            child_components.push(safe_trzsz_upload_file_name(
                &child.file_name().to_string_lossy(),
            ));
            append_trzsz_upload_path(
                path_id,
                &child_path,
                child_metadata,
                child_components,
                visited_dirs,
                entries,
                files,
            )?;
        }
        return Ok(());
    }

    if !metadata.is_file() {
        return Err(format!(
            "trzsz upload path is not a file: {}",
            path.display()
        ));
    }

    let data = std::fs::read(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let size = data.len() as u64;
    entries.push(TrzszUploadEntry {
        name: entry_name.clone(),
        data,
        source: Some(TrzszUploadSource {
            path_id,
            path_name: components,
            is_dir: false,
            size: size as i64,
            perm,
        }),
    });
    files.insert(
        entry_name,
        TrzszUploadFile {
            local_path: path.to_path_buf(),
            size,
            is_dir: false,
        },
    );
    Ok(())
}

#[cfg(unix)]
fn trzsz_upload_perm(metadata: &std::fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt as _;
    Some(metadata.permissions().mode() & 0o777)
}

#[cfg(not(unix))]
fn trzsz_upload_perm(_metadata: &std::fs::Metadata) -> Option<u32> {
    None
}

fn unique_trzsz_upload_name(path: &Path, used_names: &mut HashSet<String>) -> String {
    let base = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("trzsz-upload");
    let base = safe_trzsz_upload_file_name(base);
    if used_names.insert(base.clone()) {
        return base;
    }

    let path = Path::new(&base);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("trzsz-upload");
    let extension = path.extension().and_then(|value| value.to_str());
    for index in 1..10_000 {
        let candidate = if let Some(extension) = extension {
            format!("{stem} ({index}).{extension}")
        } else {
            format!("{stem} ({index})")
        };
        if used_names.insert(candidate.clone()) {
            return candidate;
        }
    }

    let suffix = used_names.len();
    let candidate = format!("{stem} ({suffix})");
    used_names.insert(candidate.clone());
    candidate
}

fn safe_trzsz_upload_file_name(name: &str) -> String {
    let sanitized = name
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | '\0' => '_',
            _ => ch,
        })
        .collect::<String>();
    if sanitized.trim().is_empty() {
        "trzsz-upload".to_string()
    } else {
        sanitized
    }
}

fn apply_trzsz_upload_event(
    upload: &mut TrzszUploadRuntime,
    event: TrzszUploadEvent,
) -> TrzszUploadRuntimeUpdate {
    match event {
        TrzszUploadEvent::Started { .. } => TrzszUploadRuntimeUpdate::None,
        TrzszUploadEvent::Directory { name, remote_name } => {
            let Some(file) = upload.files.get(&name) else {
                return TrzszUploadRuntimeUpdate::None;
            };
            debug_assert!(file.is_dir);
            TrzszUploadRuntimeUpdate::Progress(TrzszUploadProgressUpdate {
                file_name: name,
                remote_name,
                local_path: file.local_path.clone(),
                bytes_transferred: 0,
                total_bytes: Some(0),
                completed: true,
                fail_reason: None,
            })
        }
        TrzszUploadEvent::FileStarted {
            name,
            remote_name,
            size,
        } => {
            upload
                .remote_names
                .insert(name.clone(), remote_name.clone());
            let Some(file) = upload.files.get(&name) else {
                return TrzszUploadRuntimeUpdate::None;
            };
            debug_assert!(!file.is_dir);
            TrzszUploadRuntimeUpdate::Progress(TrzszUploadProgressUpdate {
                file_name: name,
                remote_name,
                local_path: file.local_path.clone(),
                bytes_transferred: 0,
                total_bytes: (size > 0).then_some(size as u64),
                completed: false,
                fail_reason: None,
            })
        }
        TrzszUploadEvent::Data { name, sent, size } => {
            let Some(file) = upload.files.get(&name) else {
                return TrzszUploadRuntimeUpdate::None;
            };
            TrzszUploadRuntimeUpdate::Progress(TrzszUploadProgressUpdate {
                remote_name: upload
                    .remote_names
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| name.clone()),
                file_name: name,
                local_path: file.local_path.clone(),
                bytes_transferred: sent.max(0) as u64,
                total_bytes: (size > 0).then_some(size as u64),
                completed: false,
                fail_reason: None,
            })
        }
        TrzszUploadEvent::FileFinished { name, .. } => {
            let Some(file) = upload.files.get(&name) else {
                return TrzszUploadRuntimeUpdate::None;
            };
            TrzszUploadRuntimeUpdate::Progress(TrzszUploadProgressUpdate {
                remote_name: upload
                    .remote_names
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| name.clone()),
                file_name: name,
                local_path: file.local_path.clone(),
                bytes_transferred: file.size,
                total_bytes: Some(file.size),
                completed: true,
                fail_reason: None,
            })
        }
        TrzszUploadEvent::Completed { names } => TrzszUploadRuntimeUpdate::Completed(names),
    }
}

fn apply_trzsz_download_event(
    download: &mut TrzszDownloadRuntime,
    event: TrzszDownloadEvent,
) -> Result<TrzszDownloadRuntimeUpdate, String> {
    match event {
        TrzszDownloadEvent::FileCount { .. } | TrzszDownloadEvent::FileName { .. } => {
            Ok(TrzszDownloadRuntimeUpdate::None)
        }
        TrzszDownloadEvent::FilePath {
            path_id,
            components,
            ..
        } => {
            download.pending_path = Some(TrzszDownloadPath {
                path_id,
                components,
            });
            Ok(TrzszDownloadRuntimeUpdate::None)
        }
        TrzszDownloadEvent::Directory {
            name,
            path_id,
            components,
        } => {
            let path = trzsz_directory_download_path(download, path_id, &components)?;
            std::fs::create_dir_all(&path)
                .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
            Ok(TrzszDownloadRuntimeUpdate::Progress(
                TrzszDownloadProgressUpdate {
                    file_name: safe_trzsz_file_name(&name),
                    local_path: path,
                    bytes_transferred: 0,
                    total_bytes: Some(0),
                    completed: true,
                    fail_reason: None,
                },
            ))
        }
        TrzszDownloadEvent::FileSize { name, size } => {
            let pending_path = download.pending_path.take();
            let safe_name = safe_trzsz_file_name(&name);
            let path = if let Some(path_meta) = pending_path {
                trzsz_file_download_path(download, &path_meta)?
            } else {
                unique_trzsz_download_path(&download.directory, &safe_name)
            };
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            }
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
            download.current_file = Some(TrzszDownloadFile {
                name: safe_name.clone(),
                path: path.clone(),
                file,
                size: size.max(0) as u64,
            });
            Ok(TrzszDownloadRuntimeUpdate::Progress(
                TrzszDownloadProgressUpdate {
                    file_name: safe_name,
                    local_path: path,
                    bytes_transferred: 0,
                    total_bytes: (size > 0).then_some(size as u64),
                    completed: false,
                    fail_reason: None,
                },
            ))
        }
        TrzszDownloadEvent::Data {
            bytes,
            received,
            size,
            ..
        } => {
            let Some(current) = download.current_file.as_mut() else {
                return Err("received trzsz data before opening a local file".to_string());
            };
            current
                .file
                .write_all(&bytes)
                .map_err(|error| format!("failed to write {}: {error}", current.path.display()))?;
            Ok(TrzszDownloadRuntimeUpdate::Progress(
                TrzszDownloadProgressUpdate {
                    file_name: current.name.clone(),
                    local_path: current.path.clone(),
                    bytes_transferred: received.max(0) as u64,
                    total_bytes: (size > 0).then_some(size as u64),
                    completed: false,
                    fail_reason: None,
                },
            ))
        }
        TrzszDownloadEvent::FileFinished { .. } => {
            let Some(mut current) = download.current_file.take() else {
                return Ok(TrzszDownloadRuntimeUpdate::None);
            };
            current
                .file
                .flush()
                .map_err(|error| format!("failed to flush {}: {error}", current.path.display()))?;
            Ok(TrzszDownloadRuntimeUpdate::Progress(
                TrzszDownloadProgressUpdate {
                    file_name: current.name,
                    local_path: current.path,
                    bytes_transferred: current.size,
                    total_bytes: Some(current.size),
                    completed: true,
                    fail_reason: None,
                },
            ))
        }
        TrzszDownloadEvent::Completed { names } => Ok(TrzszDownloadRuntimeUpdate::Completed(names)),
    }
}

fn safe_trzsz_file_name(name: &str) -> String {
    let base = Path::new(name)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("trzsz-download");
    let sanitized = base
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | '\0' => '_',
            _ => ch,
        })
        .collect::<String>();
    if sanitized.trim().is_empty() {
        "trzsz-download".to_string()
    } else {
        sanitized
    }
}

fn safe_trzsz_path_component(component: &str) -> String {
    let sanitized = component
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | '\0' => '_',
            _ => ch,
        })
        .collect::<String>();
    let trimmed = sanitized.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        "trzsz-download".to_string()
    } else {
        sanitized
    }
}

fn trzsz_directory_download_path(
    download: &mut TrzszDownloadRuntime,
    path_id: i64,
    components: &[String],
) -> Result<PathBuf, String> {
    trzsz_nested_download_path(download, path_id, components)
}

fn trzsz_file_download_path(
    download: &mut TrzszDownloadRuntime,
    path: &TrzszDownloadPath,
) -> Result<PathBuf, String> {
    trzsz_nested_download_path(download, path.path_id, &path.components)
}

fn trzsz_nested_download_path(
    download: &mut TrzszDownloadRuntime,
    path_id: i64,
    components: &[String],
) -> Result<PathBuf, String> {
    let Some(root) = components.first() else {
        return Err("received trzsz directory entry without a path".to_string());
    };
    let root_name = if let Some(root_name) = download.directory_roots.get(&path_id) {
        root_name.clone()
    } else {
        let safe_root = safe_trzsz_path_component(root);
        let root_path = unique_trzsz_download_path(&download.directory, &safe_root);
        let root_name = root_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("trzsz-download")
            .to_string();
        download.directory_roots.insert(path_id, root_name.clone());
        root_name
    };

    let mut path = download.directory.join(root_name);
    for component in components.iter().skip(1) {
        path.push(safe_trzsz_path_component(component));
    }
    Ok(path)
}

fn unique_trzsz_download_path(directory: &Path, file_name: &str) -> PathBuf {
    let initial = directory.join(file_name);
    if !initial.exists() {
        return initial;
    }
    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("trzsz-download");
    let extension = path.extension().and_then(|value| value.to_str());
    for index in 1..10_000 {
        let candidate_name = if let Some(extension) = extension {
            format!("{stem} ({index}).{extension}")
        } else {
            format!("{stem} ({index})")
        };
        let candidate = directory.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    directory.join(format!("{stem} ({suffix})"))
}
