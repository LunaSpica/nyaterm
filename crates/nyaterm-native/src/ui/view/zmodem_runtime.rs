use super::*;
use nyaterm_session::{
    ZmodemAction, ZmodemDetectResult, ZmodemDetector, ZmodemDirection, ZmodemEvent, ZmodemTransfer,
    start_zmodem_transfer,
};
use std::path::PathBuf;

pub(in crate::ui::view) struct ZmodemSessionState {
    pub(in crate::ui::view) detector: ZmodemDetector,
    pub(in crate::ui::view) transfer: Option<ZmodemTransfer>,
    pub(in crate::ui::view) pending_upload: Option<Vec<PathBuf>>,
    /// Download waiting for user to pick a save directory.
    pub(in crate::ui::view) pending_download: bool,
}

impl Default for ZmodemSessionState {
    fn default() -> Self {
        Self {
            detector: ZmodemDetector::new(),
            transfer: None,
            pending_upload: None,
            pending_download: false,
        }
    }
}

impl NyaTermApp {
    fn zmodem_state_mut(&mut self, session_id: &str) -> &mut ZmodemSessionState {
        self.zmodem_sessions
            .entry(session_id.to_string())
            .or_default()
    }

    pub(in crate::ui::view) fn clear_zmodem_session(&mut self, session_id: &str) {
        self.zmodem_sessions.remove(session_id);
    }

    /// Queue local files for ZMODEM upload (remote `rz`) and start the remote receiver.
    pub(in crate::ui::view) fn start_zmodem_upload(
        &mut self,
        session_id: String,
        files: Vec<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        if files.is_empty() {
            return;
        }
        if self.is_session_disconnected(&session_id) {
            self.terminal_status =
                "session disconnected — reconnect before ZMODEM upload".to_string();
            cx.notify();
            return;
        }
        let state = self.zmodem_state_mut(&session_id);
        if state.transfer.is_some() {
            self.terminal_status = "ZMODEM transfer already active".to_string();
            cx.notify();
            return;
        }
        state.pending_upload = Some(files.clone());
        state.pending_download = false;
        // Remote side runs `rz` and emits ZMODEM upload (local send) headers.
        let cmd = b"rz\r".to_vec();
        match self.session_manager.write(&session_id, &cmd) {
            Ok(()) => {
                self.terminal_status = format!(
                    "ZMODEM upload prepared ({} file(s)) — waiting for remote rz",
                    files.len()
                );
            }
            Err(error) => {
                self.zmodem_state_mut(&session_id).pending_upload = None;
                self.terminal_status = format!("ZMODEM upload failed to start: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn cancel_zmodem_transfer(
        &mut self,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.zmodem_sessions.get_mut(session_id) else {
            return;
        };
        state.pending_upload = None;
        state.pending_download = false;
        let mut actions = Vec::new();
        if let Some(transfer) = state.transfer.as_mut() {
            actions = transfer.cancel();
        }
        state.transfer = None;
        state.detector = ZmodemDetector::new();
        self.apply_zmodem_actions(session_id, actions, cx);
        self.terminal_status = "ZMODEM transfer cancelled".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn accept_zmodem_download(
        &mut self,
        session_id: String,
        save_dir: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.zmodem_sessions.get_mut(&session_id) else {
            return;
        };
        state.pending_download = false;
        let mut actions = Vec::new();
        if let Some(transfer) = state.transfer.as_mut() {
            actions = transfer.accept_download(save_dir);
            if transfer.is_done() {
                state.transfer = None;
                state.detector = ZmodemDetector::new();
            }
        }
        self.apply_zmodem_actions(&session_id, actions, cx);
        cx.notify();
    }

    /// Process raw session output for ZMODEM interception. Returns bytes that
    /// should still be painted in the terminal (empty while a transfer is active).
    pub(in crate::ui::view) fn process_zmodem_output(
        &mut self,
        session_id: &str,
        data: &[u8],
        cx: &mut Context<Self>,
    ) -> Vec<u8> {
        if data.is_empty() {
            return Vec::new();
        }
        let state = self.zmodem_state_mut(session_id);

        // Active transfer: consume all raw bytes.
        if state.transfer.is_some() {
            let mut actions = Vec::new();
            let done = {
                let state = self.zmodem_state_mut(session_id);
                if let Some(transfer) = state.transfer.as_mut() {
                    actions = transfer.feed_incoming(data);
                    transfer.is_done()
                } else {
                    false
                }
            };
            self.apply_zmodem_actions(session_id, actions, cx);
            if done {
                if let Some(state) = self.zmodem_sessions.get_mut(session_id) {
                    state.transfer = None;
                    state.detector = ZmodemDetector::new();
                }
            }
            return Vec::new();
        }

        // Detection path.
        let feed_result = {
            let state = self.zmodem_state_mut(session_id);
            state.detector.feed(data)
        };
        match feed_result {
            ZmodemDetectResult::NoMatch { passthrough } => passthrough,
            ZmodemDetectResult::Detected {
                direction,
                passthrough,
                initial_bytes,
            } => {
                let prepared_upload = if direction == ZmodemDirection::Upload {
                    self.zmodem_state_mut(session_id)
                        .pending_upload
                        .take()
                } else {
                    None
                };
                let (transfer, bootstrap) =
                    start_zmodem_transfer(direction, &initial_bytes, prepared_upload);
                let mut actions = bootstrap;
                {
                    let state = self.zmodem_state_mut(session_id);
                    state.transfer = Some(transfer);
                    if direction == ZmodemDirection::Download {
                        state.pending_download = true;
                    }
                }
                // If upload auto-started with prepared files, bootstrap may already
                // have driven protocol. For download without a path, wait for dialog.
                if direction == ZmodemDirection::Download {
                    self.prompt_zmodem_download_directory(session_id.to_string(), cx);
                }
                self.apply_zmodem_actions(session_id, actions, cx);
                // Surface detection event status.
                self.terminal_status = match direction {
                    ZmodemDirection::Upload => "ZMODEM upload detected".to_string(),
                    ZmodemDirection::Download => {
                        "ZMODEM download detected — choose save folder".to_string()
                    }
                };
                passthrough
            }
        }
    }

    fn apply_zmodem_actions(
        &mut self,
        session_id: &str,
        actions: Vec<ZmodemAction>,
        cx: &mut Context<Self>,
    ) {
        for action in actions {
            match action {
                ZmodemAction::SendToRemote(bytes) => {
                    if let Err(error) = self.session_manager.write(session_id, &bytes) {
                        self.terminal_status = format!("ZMODEM write failed: {error}");
                    }
                }
                ZmodemAction::EmitEvent(event) => self.handle_zmodem_event(session_id, event, cx),
            }
        }
    }

    fn handle_zmodem_event(
        &mut self,
        session_id: &str,
        event: ZmodemEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            ZmodemEvent::Detected { direction } => {
                self.terminal_status = match direction {
                    ZmodemDirection::Upload => "ZMODEM upload in progress".to_string(),
                    ZmodemDirection::Download => "ZMODEM download in progress".to_string(),
                };
            }
            ZmodemEvent::Progress {
                file_name,
                bytes_transferred,
                total_size,
                direction,
            } => {
                let dir = match direction {
                    ZmodemDirection::Upload => "↑",
                    ZmodemDirection::Download => "↓",
                };
                if total_size > 0 {
                    let pct = (bytes_transferred.saturating_mul(100) / total_size).min(100);
                    self.terminal_status =
                        format!("ZMODEM {dir} {file_name}: {pct}% ({bytes_transferred}/{total_size})");
                } else {
                    self.terminal_status =
                        format!("ZMODEM {dir} {file_name}: {bytes_transferred} bytes");
                }
            }
            ZmodemEvent::Complete {
                direction,
                file_count,
            } => {
                let dir = match direction {
                    ZmodemDirection::Upload => "upload",
                    ZmodemDirection::Download => "download",
                };
                self.terminal_status =
                    format!("ZMODEM {dir} complete ({file_count} file(s)) [{session_id}]");
                if let Some(state) = self.zmodem_sessions.get_mut(session_id) {
                    state.transfer = None;
                    state.detector = ZmodemDetector::new();
                    state.pending_download = false;
                    state.pending_upload = None;
                }
            }
            ZmodemEvent::Failed { reason } => {
                self.terminal_status = format!("ZMODEM failed: {reason}");
                if let Some(state) = self.zmodem_sessions.get_mut(session_id) {
                    state.transfer = None;
                    state.detector = ZmodemDetector::new();
                    state.pending_download = false;
                    state.pending_upload = None;
                }
            }
        }
        let _ = session_id;
        cx.notify();
    }

    fn prompt_zmodem_download_directory(&mut self, session_id: String, cx: &mut Context<Self>) {
        let options = PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(SharedString::from("Select ZMODEM download folder")),
        };
        let receiver = cx.prompt_for_paths(options);
        self.terminal_status = "selecting ZMODEM download folder…".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| {
                if let Some(dir) = result {
                    this.accept_zmodem_download(session_id, dir, cx);
                } else {
                    this.cancel_zmodem_transfer(&session_id, cx);
                }
            });
        })
        .detach();
    }
}
