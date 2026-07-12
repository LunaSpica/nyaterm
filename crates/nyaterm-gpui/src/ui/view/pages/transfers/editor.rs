use super::*;
use crate::ui::models::NavItem;
use nyaterm_transport::{SftpTransferControl, SftpTransferOptions, SshSessionConfig};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const EXTERNAL_EDITOR_WATCH_INTERVAL: Duration = Duration::from_millis(1000);
const EXTERNAL_EDITOR_UPLOAD_SETTLE: Duration = Duration::from_millis(450);
const EXTERNAL_EDITOR_STARTUP_SUPPRESSION: Duration = Duration::from_secs(2);

impl NyaTermApp {
    pub(super) fn enabled_transfer_file_ai_actions_for_entry(
        &self,
        entry: &SftpFileEntry,
    ) -> Vec<AiCustomActionConfig> {
        if !self.ai_settings.enabled
            || entry.file_type == SftpFileType::Directory
            || entry
                .size
                .is_some_and(|size| size > self.ai_settings.max_ai_file_size_bytes)
        {
            return Vec::new();
        }

        self.ai_settings
            .file_ai_actions
            .iter()
            .filter(|action| action.enabled && !action.name.trim().is_empty())
            .cloned()
            .collect()
    }

    pub(super) fn start_transfer_file_ai_action(
        &mut self,
        entry: SftpFileEntry,
        action: AiCustomActionConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if entry.file_type == SftpFileType::Directory {
            self.transfer_browser_status = "AI file actions require a file".to_string();
            self.terminal_status = "directories cannot be sent to file AI actions".to_string();
            cx.notify();
            return;
        }
        if entry
            .size
            .is_some_and(|size| size > self.ai_settings.max_ai_file_size_bytes)
        {
            self.transfer_browser_status = "file exceeds AI file size limit".to_string();
            self.terminal_status = format!("{} is too large for AI file actions", entry.path);
            cx.notify();
            return;
        }
        if !self.ai_settings.enabled {
            self.transfer_browser_status = "AI assistant is disabled".to_string();
            self.terminal_status = "AI assistant is disabled".to_string();
            cx.notify();
            return;
        }
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            cx.notify();
            return;
        };

        self.ensure_event_pump(window, cx);
        self.transfer_selected_remote_path = Some(entry.path.clone());
        self.transfer_remote_path = entry.path.clone();

        let remote_path = entry.path.clone();
        let action_id = action.id.clone();
        let action_name = action.name.clone();
        let prompt = action.prompt.clone();
        let max_bytes = self.ai_settings.max_ai_file_size_bytes;
        let id = self.next_transfer_id("sftp-ai-file");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::AiFileAction {
                remote_path: remote_path.clone(),
                action_id: action_id.clone(),
                action_name: action_name.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Preparing AI file action {action_name} for {remote_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.transfer_browser_status = format!("loading {remote_path} for AI");
        self.terminal_status = format!("SFTP AI file action started: {remote_path}");
        let transfer_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let result = SftpService::new(config)
                .read_text_file(&remote_path, max_bytes)
                .map(|file| TransferJobOutput::AiFileActionLoaded {
                    remote_path,
                    action_id,
                    action_name,
                    prompt,
                    file,
                })
                .map_err(|error| error.to_string());
            let _ = transfer_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }

    pub(super) fn open_selected_transfer_default(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.selected_transfer_entry() else {
            self.terminal_status = "select a remote file first".to_string();
            cx.notify();
            return;
        };
        self.open_transfer_default(entry, window, cx);
    }

    pub(super) fn open_selected_transfer_editor(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.selected_transfer_entry() else {
            self.terminal_status = "select a remote file first".to_string();
            cx.notify();
            return;
        };
        self.open_transfer_editor(entry, window, cx);
    }

    pub(super) fn open_selected_transfer_external(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.selected_transfer_entry() else {
            self.terminal_status = "select a remote file first".to_string();
            cx.notify();
            return;
        };
        self.open_transfer_external(entry, window, cx);
    }

    pub(super) fn open_transfer_default(
        &mut self,
        entry: SftpFileEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.settings.transfer_editor_type == "internal" {
            self.open_transfer_editor(entry, window, cx);
        } else {
            self.open_transfer_external(entry, window, cx);
        }
    }

    pub(super) fn open_transfer_editor(
        &mut self,
        entry: SftpFileEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match remote_file_text_kind(&entry.name) {
            RemoteFileTextKind::Text => self.open_transfer_editor_direct(entry, window, cx),
            RemoteFileTextKind::Binary => {
                self.transfer_browser_status = "known binary file opened externally".to_string();
                self.open_transfer_external(entry, window, cx);
            }
            RemoteFileTextKind::Unknown => {
                self.transfer_unknown_file = Some(TransferUnknownFileState { entry });
                self.terminal_status = "confirm how to open unknown remote file".to_string();
                window.focus(&self.transfer_unknown_file_focus);
                cx.notify();
            }
        }
    }

    pub(super) fn open_transfer_editor_direct(
        &mut self,
        entry: SftpFileEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if entry.file_type == SftpFileType::Directory {
            self.terminal_status = "directories cannot be opened in the text editor".to_string();
            cx.notify();
            return;
        }
        self.transfer_selected_remote_path = Some(entry.path.clone());
        self.transfer_remote_path = entry.path.clone();
        self.transfer_editor = Some(TransferEditorState {
            remote_path: entry.path.clone(),
            name: entry.name.clone(),
            content: String::new(),
            search_query: String::new(),
            active_match: 0,
            base_size: entry.size,
            base_modified_at: entry.modified_at.map(u64::from),
            loading: true,
            saving: false,
            dirty: false,
            conflict: false,
            close_confirm: false,
            close_after_save: false,
            reload_confirm: false,
            error: None,
            focused_field: TransferEditorField::Content,
        });
        self.terminal_status = format!("opening remote text file {}", entry.path);
        window.focus(&self.transfer_editor_focus);
        self.start_sftp_editor_load_job(entry.path, window, cx);
        cx.notify();
    }

    pub(super) fn cancel_transfer_unknown_file(&mut self, cx: &mut Context<Self>) {
        self.transfer_unknown_file = None;
        self.terminal_status = "unknown file open cancelled".to_string();
        cx.notify();
    }

    pub(super) fn open_unknown_transfer_file_external(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer_unknown_file.take() else {
            cx.notify();
            return;
        };
        self.open_transfer_external(state.entry, window, cx);
    }

    pub(super) fn open_unknown_transfer_file_internal(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer_unknown_file.take() else {
            cx.notify();
            return;
        };
        self.open_transfer_editor_direct(state.entry, window, cx);
    }

    pub(super) fn open_transfer_external(
        &mut self,
        entry: SftpFileEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if entry.file_type == SftpFileType::Directory {
            self.terminal_status = "directories cannot be opened in an external editor".to_string();
            cx.notify();
            return;
        }
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.ensure_panel_open(NavItem::Transfers);
            cx.notify();
            return;
        };

        self.transfer_selected_remote_path = Some(entry.path.clone());
        self.transfer_remote_path = entry.path.clone();
        self.ensure_event_pump(window, cx);

        let remote_path = entry.path.clone();
        let local_path = self.transfer_external_open_path(&entry);
        let default_editor = self.settings.transfer_default_editor.clone();
        let transfer_options = self.sftp_transfer_options();
        let id = self.next_transfer_id("sftp-open-external");
        let control = SftpTransferControl::new();
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::OpenExternal {
                remote_path: remote_path.clone(),
                local_path: local_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Opening {remote_path} externally"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: Some(control.clone()),
        });
        self.terminal_status = format!("downloading {remote_path} for external open");

        let progress_tx = self.transfer_tx.clone();
        let finished_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let progress_id = id.clone();
            let result = SftpService::new(config)
                .download_file_with_progress_and_control_options(
                    &remote_path,
                    local_path.clone(),
                    control,
                    transfer_options,
                    move |progress| {
                        let _ = progress_tx.send(TransferJobResult {
                            id: progress_id.clone(),
                            event: TransferJobEvent::Progress(progress),
                        });
                    },
                )
                .map_err(|error| error.to_string())
                .and_then(|_| {
                    open_local_path_with_editor(&local_path, &default_editor).map(|_| {
                        TransferJobOutput::ExternalOpened {
                            remote_path: remote_path.clone(),
                            local_path: local_path.clone(),
                        }
                    })
                });
            let opened = result.is_ok();
            let _ = finished_tx.send(TransferJobResult {
                id: id.clone(),
                event: TransferJobEvent::Finished(result),
            });
            if opened {
                watch_external_editor_file(id, remote_path, local_path, finished_tx);
            }
        });
        cx.notify();
    }

    fn transfer_external_open_path(&self, entry: &SftpFileEntry) -> PathBuf {
        let session_id = self
            .active_session_id
            .as_deref()
            .map(sanitize_local_open_segment)
            .unwrap_or_else(|| "session".to_string());
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        let fallback_name;
        let raw_file_name = if entry.name.trim().is_empty() {
            fallback_name = remote_file_name(&entry.path);
            fallback_name.as_str()
        } else {
            entry.name.as_str()
        };
        let file_name = sanitize_local_open_segment(raw_file_name);
        std::env::temp_dir()
            .join("nyaterm")
            .join(session_id)
            .join(timestamp_ms.to_string())
            .join(file_name)
    }

    pub(super) fn close_transfer_editor(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn discard_transfer_editor(&mut self, cx: &mut Context<Self>) {
        self.transfer_editor = None;
        self.terminal_status = "remote editor discarded".to_string();
        cx.notify();
    }

    pub(super) fn cancel_transfer_editor_close_confirm(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn cancel_transfer_editor_reload_confirm(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn start_sftp_editor_load_job(
        &mut self,
        remote_path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            cx.notify();
            return;
        };
        self.ensure_event_pump(window, cx);
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

    pub(super) fn save_transfer_editor(
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

    pub(super) fn save_transfer_editor_and_close(
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

    pub(super) fn start_sftp_editor_save_job(
        &mut self,
        remote_path: String,
        content: String,
        expected_modified_at: Option<u64>,
        expected_size: Option<u64>,
        force: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            cx.notify();
            return;
        };
        self.ensure_event_pump(window, cx);
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

    pub(super) fn handle_transfer_editor_key_down(
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

    pub(super) fn advance_transfer_editor_search(&mut self, delta: isize, cx: &mut Context<Self>) {
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

    pub(super) fn upload_external_editor_sync(
        &mut self,
        job_id: String,
        remote_path: String,
        local_path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        self.spawn_external_editor_sync_upload(job_id, remote_path, local_path);
        cx.notify();
    }

    pub(in crate::ui::view) fn spawn_external_editor_sync_upload(
        &mut self,
        job_id: String,
        remote_path: String,
        local_path: PathBuf,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
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

    pub(in crate::ui::view) fn upload_pending_external_editor_sync(
        &mut self,
        always: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(prompt) = self.transfer_external_sync_prompt.take() else {
            cx.notify();
            return;
        };
        let watch_key = external_editor_watch_key(&prompt.remote_path, &prompt.local_path);
        if always {
            self.transfer_external_always_uploads.insert(watch_key);
        }
        self.upload_external_editor_sync(prompt.job_id, prompt.remote_path, prompt.local_path, cx);
    }

    pub(in crate::ui::view) fn ignore_pending_external_editor_sync(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.transfer_external_sync_prompt = None;
        self.terminal_status = "external edit sync skipped".to_string();
        cx.notify();
    }
}

fn sanitize_local_open_segment(input: &str) -> String {
    let sanitized: String = input
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect();
    let sanitized = sanitized.trim_matches(['.', ' ']).trim();
    if sanitized.is_empty() {
        "remote-file".to_string()
    } else {
        sanitized.to_string()
    }
}

fn open_local_path_with_editor(path: &Path, editor_command: &str) -> Result<(), String> {
    let command = editor_command.trim();
    if command.is_empty() {
        open_local_path_with_system_default(path)
    } else {
        let mut parts = command.split_whitespace();
        let Some(program) = parts.next() else {
            return open_local_path_with_system_default(path);
        };
        Command::new(program)
            .args(parts)
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open {} with {program}: {error}", path.display()))
    }
}

fn open_local_path_with_system_default(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(path);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", ""]).arg(path);
        command
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open {}: {error}", path.display()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteFileTextKind {
    Text,
    Binary,
    Unknown,
}

fn remote_file_text_kind(name: &str) -> RemoteFileTextKind {
    if is_known_text_file(name) {
        RemoteFileTextKind::Text
    } else if is_known_binary_file(name) {
        RemoteFileTextKind::Binary
    } else {
        RemoteFileTextKind::Unknown
    }
}

fn remote_file_extension(name: &str) -> String {
    let normalized = name.trim().to_ascii_lowercase();
    let base = normalized
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(normalized.as_str());
    let Some(index) = base.rfind('.') else {
        return String::new();
    };
    if index == 0 {
        String::new()
    } else {
        base[index + 1..].to_string()
    }
}

fn remote_file_basename(name: &str) -> String {
    name.rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .to_ascii_lowercase()
}

fn is_known_binary_file(name: &str) -> bool {
    matches!(
        remote_file_extension(name).as_str(),
        "jpg"
            | "jpeg"
            | "png"
            | "gif"
            | "bmp"
            | "webp"
            | "ico"
            | "tiff"
            | "tif"
            | "heic"
            | "heif"
            | "avif"
            | "mp3"
            | "wav"
            | "flac"
            | "aac"
            | "ogg"
            | "wma"
            | "m4a"
            | "mp4"
            | "avi"
            | "mkv"
            | "mov"
            | "wmv"
            | "flv"
            | "webm"
            | "zip"
            | "rar"
            | "7z"
            | "tar"
            | "gz"
            | "bz2"
            | "xz"
            | "zst"
            | "tgz"
            | "tbz2"
            | "txz"
            | "iso"
            | "dmg"
            | "exe"
            | "dll"
            | "so"
            | "dylib"
            | "bin"
            | "msi"
            | "deb"
            | "rpm"
            | "apk"
            | "jar"
            | "war"
            | "ear"
            | "pdf"
            | "doc"
            | "docx"
            | "xls"
            | "xlsx"
            | "ppt"
            | "pptx"
            | "ttf"
            | "otf"
            | "woff"
            | "woff2"
            | "db"
            | "sqlite"
            | "sqlite3"
            | "o"
            | "obj"
            | "pyc"
            | "pyo"
            | "class"
    )
}

fn is_known_text_file(name: &str) -> bool {
    let base_name = remote_file_basename(name);
    let normalized = base_name.trim_start_matches('.');
    let extension = remote_file_extension(name);
    matches!(
        extension.as_str(),
        "asc"
            | "bash"
            | "bat"
            | "c"
            | "cfg"
            | "cc"
            | "cjs"
            | "cmd"
            | "conf"
            | "cpp"
            | "cs"
            | "css"
            | "cxx"
            | "csv"
            | "dart"
            | "diff"
            | "env"
            | "fish"
            | "go"
            | "h"
            | "hpp"
            | "htm"
            | "html"
            | "ini"
            | "java"
            | "js"
            | "json"
            | "json5"
            | "jsonc"
            | "jsx"
            | "log"
            | "lua"
            | "markdown"
            | "md"
            | "mjs"
            | "patch"
            | "pem"
            | "php"
            | "pl"
            | "properties"
            | "proto"
            | "ps1"
            | "py"
            | "r"
            | "rb"
            | "rs"
            | "sass"
            | "scss"
            | "service"
            | "sh"
            | "socket"
            | "sql"
            | "swift"
            | "timer"
            | "toml"
            | "ts"
            | "tsx"
            | "txt"
            | "vue"
            | "xml"
            | "yaml"
            | "yml"
            | "zsh"
    ) || matches!(
        normalized,
        "bash_profile"
            | "bash_login"
            | "bash_logout"
            | "bashrc"
            | "cmakelists.txt"
            | "dockerfile"
            | "editorconfig"
            | "env"
            | "env.local"
            | "gitconfig"
            | "gitignore"
            | "gitmodules"
            | "gitattributes"
            | "makefile"
            | "gnumakefile"
            | "npmrc"
            | "profile"
            | "zprofile"
            | "zshenv"
            | "zshrc"
    ) || base_name.ends_with(".dockerfile")
        || base_name.ends_with(".nginx.conf")
        || base_name == "docker-compose.yml"
        || base_name == "docker-compose.yaml"
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LocalFileFingerprint {
    len: u64,
    modified: Option<SystemTime>,
}

impl LocalFileFingerprint {
    fn from_path(path: &Path) -> std::io::Result<Self> {
        let metadata = fs::metadata(path)?;
        Ok(Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
        })
    }

    fn is_content_change_from(&self, previous: &Self, within_startup_window: bool) -> bool {
        if self.len != previous.len {
            return true;
        }
        self.modified != previous.modified && !within_startup_window
    }
}

fn watch_external_editor_file(
    job_id: String,
    remote_path: String,
    local_path: PathBuf,
    transfer_tx: std::sync::mpsc::Sender<TransferJobResult>,
) {
    let watch_started = Instant::now();
    let mut baseline = match LocalFileFingerprint::from_path(&local_path) {
        Ok(fingerprint) => fingerprint,
        Err(error) => {
            let _ = transfer_tx.send(TransferJobResult {
                id: job_id,
                event: TransferJobEvent::Finished(Err(format!(
                    "external editor watch failed for {}: {error}",
                    local_path.display()
                ))),
            });
            return;
        }
    };

    loop {
        std::thread::sleep(EXTERNAL_EDITOR_WATCH_INTERVAL);
        let current = match LocalFileFingerprint::from_path(&local_path) {
            Ok(fingerprint) => fingerprint,
            Err(_) => break,
        };
        if !current.is_content_change_from(
            &baseline,
            watch_started.elapsed() <= EXTERNAL_EDITOR_STARTUP_SUPPRESSION,
        ) {
            if current != baseline {
                baseline = current;
            }
            continue;
        }

        std::thread::sleep(EXTERNAL_EDITOR_UPLOAD_SETTLE);
        let settled = LocalFileFingerprint::from_path(&local_path).unwrap_or(current);
        baseline = settled;
        let _ = transfer_tx.send(TransferJobResult {
            id: job_id.clone(),
            event: TransferJobEvent::ExternalModified {
                remote_path: remote_path.clone(),
                local_path: local_path.clone(),
            },
        });
        if let Ok(after_upload) = LocalFileFingerprint::from_path(&local_path) {
            baseline = after_upload;
        }
    }
}

fn external_editor_watch_key(remote_path: &str, local_path: &Path) -> String {
    format!("{remote_path}\n{}", local_path.display())
}

fn upload_external_editor_file(
    config: &SshSessionConfig,
    job_id: &str,
    remote_path: &str,
    local_path: &Path,
    transfer_options: SftpTransferOptions,
    transfer_tx: &std::sync::mpsc::Sender<TransferJobResult>,
) {
    let _ = transfer_tx.send(TransferJobResult {
        id: job_id.to_string(),
        event: TransferJobEvent::Started {
            detail: format!("Syncing external edit {remote_path}"),
        },
    });
    let control = SftpTransferControl::new();
    let progress_id = job_id.to_string();
    let progress_tx = transfer_tx.clone();
    let result = SftpService::new(config.clone())
        .upload_file_with_progress_and_control_options(
            local_path.to_path_buf(),
            remote_path,
            control,
            transfer_options,
            move |progress| {
                let _ = progress_tx.send(TransferJobResult {
                    id: progress_id.clone(),
                    event: TransferJobEvent::Progress(progress),
                });
            },
        )
        .map(TransferJobOutput::Summary)
        .map_err(|error| error.to_string());
    let _ = transfer_tx.send(TransferJobResult {
        id: job_id.to_string(),
        event: TransferJobEvent::Finished(result),
    });
}
