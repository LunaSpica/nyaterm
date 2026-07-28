use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use gpui::FocusHandle;
use nyaterm_core::{AiExecutionProfile, SavedConnection};
use nyaterm_transport::{
    SessionEvent, SessionKind, SessionManager, SshMultiplexHandle, SshSessionConfig,
};

use crate::features::DEFAULT_DUPLICATE_STARTUP_DELAY_MS;
use crate::features::runtime_jobs::SessionStartResult;
use crate::models::{
    ActiveSessionMenuState, SessionEventBridge, SessionLaunchConfig, SessionRuntimeMetadata,
    StartupCommandAction, StartupCommandRequest, TabActionsSubmenu, WorkspaceSplitDirection,
};

use super::auth_runtime::{
    CredentialPromptBroker, CredentialPromptState, HostKeyPromptBroker, HostKeyPromptRequest,
    KeyboardInteractivePromptState, NativeOtpProvider, SftpDuplicatePromptBroker,
    SftpDuplicatePromptState,
};
use super::trzsz_runtime::TrzszSessionState;
use super::zmodem_runtime::ZmodemSessionState;

pub(in crate::features) struct SessionFeatureState {
    pub manager: Arc<SessionManager>,
    pub event_bridge: SessionEventBridge,
    pub start: SessionStartFeatureState,
    pub restore: SessionRestoreState,
    pub events: SessionEventQueueState,
    pub prompts: SessionPromptState,
    pub dialogs: SessionDialogState,
    pub command_history: HashMap<String, Vec<String>>,
    pub active_search_draft: String,
    pub active_menu: Option<ActiveSessionMenuState>,
    /// Per-session reconnect/disconnect busy state ("reconnect" | "disconnect").
    pub busy_actions: HashMap<String, String>,
    pub active_id: Option<String>,
    pub active_ssh_config: Option<SshSessionConfig>,
    pub active_ai_execution_profile: AiExecutionProfile,
    pub order: Vec<String>,
    pub metadata: HashMap<String, SessionRuntimeMetadata>,
    pub custom_names: HashMap<String, String>,
    /// OSC 0/2 titles from the session PTY (fall back when no custom rename).
    pub dynamic_titles: HashMap<String, String>,
    /// Latest OSC 7 working directories per session.
    pub cwds: HashMap<String, String>,
    /// Per-session ZMODEM detector / transfer state (UI-layer interception).
    pub zmodem: HashMap<String, ZmodemSessionState>,
    /// Per-session trzsz trigger detector state (pre-parser protocol slot).
    pub trzsz: HashMap<String, TrzszSessionState>,
    pub tab_colors: HashMap<String, u32>,
    pub multiplex_handles: HashMap<String, SshMultiplexHandle>,
}

#[derive(Default)]
pub(in crate::features) struct SessionRestoreState {
    complete: bool,
}

#[derive(Default)]
pub(in crate::features) struct SessionEventQueueState {
    pub pending: VecDeque<SessionEvent>,
}

pub(in crate::features) struct SessionFeatureFocus {
    pub credential: FocusHandle,
    pub tab_actions: FocusHandle,
    pub close_all: FocusHandle,
    pub rename: FocusHandle,
    pub color_picker: FocusHandle,
    pub info: FocusHandle,
    pub startup_command: FocusHandle,
    pub temporary_ssh_link: FocusHandle,
}

/// Native authentication and transfer prompts tied to the session runtime.
pub(in crate::features) struct SessionPromptState {
    pub duplicate_prompts: Arc<SftpDuplicatePromptBroker>,
    pub active_duplicate_prompt: Option<SftpDuplicatePromptState>,
    pub host_key_prompts: Arc<HostKeyPromptBroker>,
    pub active_host_key_prompt: Option<HostKeyPromptRequest>,
    pub credential_prompts: Arc<CredentialPromptBroker>,
    pub active_credential_prompt: Option<CredentialPromptState>,
    pub active_keyboard_interactive_prompt: Option<KeyboardInteractivePromptState>,
    pub credential_prompt_focus_pending: bool,
    pub credential_focus: FocusHandle,
    pub otp_provider: Arc<NativeOtpProvider>,
}

/// Session-scoped overlays, confirmations and editing dialogs.
pub(in crate::features) struct SessionDialogState {
    tab_actions_session_id: Option<String>,
    tab_actions_anchor: Option<(f32, f32)>,
    tab_actions_submenu: Option<TabActionsSubmenu>,
    tab_actions_focus: FocusHandle,
    close_all_sessions_confirm_open: bool,
    pending_quit_after_close_all: bool,
    pending_window_quit: bool,
    close_all_sessions_confirm_focus: FocusHandle,
    rename_session_id: Option<String>,
    rename_draft: String,
    rename_focus: FocusHandle,
    color_picker_open: bool,
    color_picker_focus: FocusHandle,
    session_info_open: bool,
    session_info_focus: FocusHandle,
    startup_command_open: bool,
    startup_command_action: StartupCommandAction,
    startup_command_draft: String,
    startup_command_delay_ms: u64,
    startup_command_focus: FocusHandle,
    temporary_ssh_link_open: bool,
    temporary_ssh_link_draft: String,
    temporary_ssh_link_error: Option<&'static str>,
    temporary_ssh_link_focus: FocusHandle,
}

pub(in crate::features) enum RenameSessionSubmission {
    Inactive,
    Empty,
    Ready { session_id: String, name: String },
}

impl SessionFeatureState {
    pub(in crate::features) fn new(
        manager: Arc<SessionManager>,
        event_bridge: SessionEventBridge,
        otp_provider: Arc<NativeOtpProvider>,
        focus: SessionFeatureFocus,
    ) -> Self {
        Self {
            manager,
            event_bridge,
            start: SessionStartFeatureState::new(),
            restore: SessionRestoreState::default(),
            events: SessionEventQueueState::default(),
            prompts: SessionPromptState {
                duplicate_prompts: Arc::new(SftpDuplicatePromptBroker::default()),
                active_duplicate_prompt: None,
                host_key_prompts: Arc::new(HostKeyPromptBroker::default()),
                active_host_key_prompt: None,
                credential_prompts: Arc::new(CredentialPromptBroker::default()),
                active_credential_prompt: None,
                active_keyboard_interactive_prompt: None,
                credential_prompt_focus_pending: false,
                credential_focus: focus.credential,
                otp_provider,
            },
            dialogs: SessionDialogState {
                tab_actions_session_id: None,
                tab_actions_anchor: None,
                tab_actions_submenu: None,
                tab_actions_focus: focus.tab_actions,
                close_all_sessions_confirm_open: false,
                pending_quit_after_close_all: false,
                pending_window_quit: false,
                close_all_sessions_confirm_focus: focus.close_all,
                rename_session_id: None,
                rename_draft: String::new(),
                rename_focus: focus.rename,
                color_picker_open: false,
                color_picker_focus: focus.color_picker,
                session_info_open: false,
                session_info_focus: focus.info,
                startup_command_open: false,
                startup_command_action: StartupCommandAction::Duplicate,
                startup_command_draft: String::new(),
                startup_command_delay_ms: DEFAULT_DUPLICATE_STARTUP_DELAY_MS,
                startup_command_focus: focus.startup_command,
                temporary_ssh_link_open: false,
                temporary_ssh_link_draft: String::new(),
                temporary_ssh_link_error: None,
                temporary_ssh_link_focus: focus.temporary_ssh_link,
            },
            command_history: HashMap::new(),
            active_search_draft: String::new(),
            active_menu: None,
            busy_actions: HashMap::new(),
            active_id: None,
            active_ssh_config: None,
            active_ai_execution_profile: AiExecutionProfile::SendOnly,
            order: Vec::new(),
            metadata: HashMap::new(),
            custom_names: HashMap::new(),
            dynamic_titles: HashMap::new(),
            cwds: HashMap::new(),
            zmodem: HashMap::new(),
            trzsz: HashMap::new(),
            tab_colors: HashMap::new(),
            multiplex_handles: HashMap::new(),
        }
    }
}

impl SessionRestoreState {
    pub(in crate::features) fn is_complete(&self) -> bool {
        self.complete
    }

    pub(in crate::features) fn mark_complete(&mut self) -> bool {
        if self.complete {
            return false;
        }
        self.complete = true;
        true
    }
}

impl SessionDialogState {
    pub(in crate::features) fn tab_actions_session_id(&self) -> Option<&str> {
        self.tab_actions_session_id.as_deref()
    }

    pub(in crate::features) fn tab_actions_anchor(&self) -> Option<(f32, f32)> {
        self.tab_actions_anchor
    }

    pub(in crate::features) fn tab_actions_submenu(&self) -> Option<TabActionsSubmenu> {
        self.tab_actions_submenu
    }

    pub(in crate::features) fn tab_actions_focus(&self) -> &FocusHandle {
        &self.tab_actions_focus
    }

    pub(in crate::features) fn open_tab_actions(
        &mut self,
        session_id: String,
        anchor: Option<(f32, f32)>,
    ) {
        self.tab_actions_session_id = Some(session_id);
        self.tab_actions_anchor = anchor;
        self.tab_actions_submenu = None;
    }

    pub(in crate::features) fn close_tab_actions(&mut self) {
        self.tab_actions_session_id = None;
        self.tab_actions_anchor = None;
        self.tab_actions_submenu = None;
    }

    pub(in crate::features) fn select_tab_actions_submenu(
        &mut self,
        submenu: TabActionsSubmenu,
    ) -> bool {
        if self.tab_actions_submenu == Some(submenu) {
            return false;
        }
        self.tab_actions_submenu = Some(submenu);
        true
    }

    pub(in crate::features) fn close_all_sessions_confirm_is_open(&self) -> bool {
        self.close_all_sessions_confirm_open
    }

    pub(in crate::features) fn should_quit_after_close_all(&self) -> bool {
        self.pending_quit_after_close_all
    }

    pub(in crate::features) fn close_all_sessions_confirm_focus(&self) -> &FocusHandle {
        &self.close_all_sessions_confirm_focus
    }

    pub(in crate::features) fn request_quit_after_close_all(&mut self) {
        self.pending_quit_after_close_all = true;
    }

    pub(in crate::features) fn open_close_all_sessions_confirm(&mut self) {
        self.close_tab_actions();
        self.close_all_sessions_confirm_open = true;
    }

    pub(in crate::features) fn cancel_close_all_sessions_confirm(&mut self) {
        self.close_all_sessions_confirm_open = false;
        self.pending_quit_after_close_all = false;
        self.pending_window_quit = false;
    }

    pub(in crate::features) fn take_close_all_sessions_confirm(&mut self) -> bool {
        self.close_all_sessions_confirm_open = false;
        let quit_after = self.pending_quit_after_close_all;
        self.pending_quit_after_close_all = false;
        self.pending_window_quit = false;
        quit_after
    }

    pub(in crate::features) fn rename_is_open(&self) -> bool {
        self.rename_session_id.is_some()
    }

    pub(in crate::features) fn rename_draft(&self) -> &str {
        &self.rename_draft
    }

    pub(in crate::features) fn rename_focus(&self) -> &FocusHandle {
        &self.rename_focus
    }

    pub(in crate::features) fn open_rename(&mut self, session_id: String, current_name: &str) {
        self.rename_session_id = Some(session_id);
        self.rename_draft = current_name.chars().take(64).collect();
    }

    pub(in crate::features) fn cancel_rename(&mut self) {
        self.rename_session_id = None;
        self.rename_draft.clear();
    }

    pub(in crate::features) fn take_rename_submission(&mut self) -> RenameSessionSubmission {
        let Some(session_id) = self.rename_session_id.take() else {
            return RenameSessionSubmission::Inactive;
        };
        let name = self
            .rename_draft
            .trim()
            .chars()
            .take(64)
            .collect::<String>();
        self.rename_draft.clear();
        if name.is_empty() {
            self.rename_session_id = Some(session_id);
            return RenameSessionSubmission::Empty;
        }
        RenameSessionSubmission::Ready { session_id, name }
    }

    pub(in crate::features) fn color_picker_is_open(&self) -> bool {
        self.color_picker_open
    }

    pub(in crate::features) fn color_picker_focus(&self) -> &FocusHandle {
        &self.color_picker_focus
    }

    pub(in crate::features) fn close_color_picker(&mut self) {
        self.color_picker_open = false;
    }

    pub(in crate::features) fn session_info_is_open(&self) -> bool {
        self.session_info_open
    }

    pub(in crate::features) fn session_info_focus(&self) -> &FocusHandle {
        &self.session_info_focus
    }

    pub(in crate::features) fn open_session_info(&mut self) {
        self.session_info_open = true;
    }

    pub(in crate::features) fn close_session_info(&mut self) {
        self.session_info_open = false;
    }

    pub(in crate::features) fn startup_command_is_open(&self) -> bool {
        self.startup_command_open
    }

    pub(in crate::features) fn startup_command_action(&self) -> StartupCommandAction {
        self.startup_command_action
    }

    pub(in crate::features) fn startup_command_draft(&self) -> &str {
        &self.startup_command_draft
    }

    pub(in crate::features) fn startup_command_delay_ms(&self) -> u64 {
        self.startup_command_delay_ms
    }

    pub(in crate::features) fn startup_command_focus(&self) -> &FocusHandle {
        &self.startup_command_focus
    }

    pub(in crate::features) fn open_startup_command(
        &mut self,
        action: StartupCommandAction,
        delay_ms: u64,
    ) {
        self.startup_command_open = true;
        self.startup_command_action = action;
        self.startup_command_draft.clear();
        self.startup_command_delay_ms = delay_ms.min(60_000);
    }

    pub(in crate::features) fn cancel_startup_command(&mut self) -> StartupCommandAction {
        let action = self.startup_command_action;
        self.startup_command_open = false;
        self.startup_command_action = StartupCommandAction::Duplicate;
        self.startup_command_draft.clear();
        self.startup_command_delay_ms = DEFAULT_DUPLICATE_STARTUP_DELAY_MS;
        action
    }

    pub(in crate::features) fn take_startup_command(
        &mut self,
    ) -> Option<(StartupCommandAction, StartupCommandRequest)> {
        let command = self.startup_command_draft.trim().to_string();
        if command.is_empty() {
            return None;
        }
        let request = StartupCommandRequest {
            command,
            delay_ms: self.startup_command_delay_ms.min(60_000),
        };
        let action = self.startup_command_action;
        self.startup_command_open = false;
        self.startup_command_action = StartupCommandAction::Duplicate;
        self.startup_command_draft.clear();
        Some((action, request))
    }

    pub(in crate::features) fn adjust_startup_command_delay(&mut self, delta_ms: i64) {
        let next = (self.startup_command_delay_ms as i64 + delta_ms).clamp(0, 60_000);
        self.startup_command_delay_ms = next as u64;
    }

    pub(in crate::features) fn reset_startup_command_delay(&mut self) {
        self.startup_command_delay_ms = 0;
    }

    pub(in crate::features) fn temporary_ssh_link_is_open(&self) -> bool {
        self.temporary_ssh_link_open
    }

    pub(in crate::features) fn temporary_ssh_link_draft(&self) -> &str {
        &self.temporary_ssh_link_draft
    }

    pub(in crate::features) fn temporary_ssh_link_error(&self) -> Option<&'static str> {
        self.temporary_ssh_link_error
    }

    pub(in crate::features) fn temporary_ssh_link_focus(&self) -> &FocusHandle {
        &self.temporary_ssh_link_focus
    }

    pub(in crate::features) fn open_temporary_ssh_link(&mut self) {
        self.temporary_ssh_link_open = true;
        self.temporary_ssh_link_error = None;
    }

    pub(in crate::features) fn close_temporary_ssh_link(&mut self) {
        self.temporary_ssh_link_open = false;
        self.temporary_ssh_link_draft.clear();
        self.temporary_ssh_link_error = None;
    }

    pub(in crate::features) fn reject_temporary_ssh_link(&mut self, error: &'static str) {
        self.temporary_ssh_link_error = Some(error);
    }

    pub(in crate::features) fn apply_temporary_ssh_link(&mut self, text: String) {
        self.temporary_ssh_link_draft = text;
        self.temporary_ssh_link_error = None;
    }

    pub(in crate::features) fn apply_text_input(&mut self, field: &str, text: String) -> bool {
        match field {
            "rename" => self.rename_draft = text,
            "startup-command" => self.startup_command_draft = text,
            _ => return false,
        }
        true
    }
}

#[derive(Debug, Clone)]
pub(in crate::features) enum SessionPaneState {
    Connecting {
        request_id: String,
        name: String,
        kind: SessionKind,
    },
    Live {
        session_id: String,
    },
    Failed {
        name: String,
        error: String,
    },
    Disconnected {
        session_id: String,
    },
}

pub(in crate::features) struct PendingSessionStart {
    pub connection_name: String,
    pub launch_config: Option<SessionLaunchConfig>,
    pub requested_at: Instant,
    pub kind: SessionKind,
    pub ai_execution_profile: AiExecutionProfile,
    pub custom_name: Option<String>,
    pub tab_color: Option<u32>,
    pub after_session_id: Option<String>,
    pub insert_index: Option<usize>,
    pub seed_output: Option<String>,
    pub startup_command: Option<StartupCommandRequest>,
    pub multiplex_key: Option<String>,
    pub source_connection_id: Option<String>,
    /// Existing pane being replaced by this request, when this is a reconnect.
    pub reconnect_session_id: Option<String>,
}

/// A session start that remains visible after its worker failed.
///
/// Tauri keeps the failed pane in its original tab, so the GPUI shell must
/// retain the pending metadata instead of reducing the failure to a global
/// banner.
pub(in crate::features) struct FailedSessionStart {
    pub pending: PendingSessionStart,
    pub error: String,
}

pub(in crate::features) struct SessionStartFeatureState {
    tx: mpsc::Sender<SessionStartResult>,
    rx: mpsc::Receiver<SessionStartResult>,
    pub pending: HashMap<String, PendingSessionStart>,
    pub active_pending: Option<String>,
    pub failed: HashMap<String, FailedSessionStart>,
    pub active_failed: Option<String>,
    pub cancelled: HashSet<String>,
    pub panes: HashMap<String, SessionPaneState>,
    pub reconnect_replace_id: Option<String>,
    pub reconnect_failures: HashMap<String, String>,
    pub pending_workspace_split: Option<(WorkspaceSplitDirection, String)>,
    saved_connection_queue: VecDeque<PendingSavedConnectionStart>,
}

#[derive(Clone, Default)]
pub(in crate::features) struct SavedConnectionStartOptions {
    pub custom_name: Option<String>,
    pub tab_color: Option<u32>,
    pub after_session_id: Option<String>,
    pub insert_index: Option<usize>,
    pub seed_output: Option<String>,
    pub startup_command: Option<StartupCommandRequest>,
}

#[derive(Clone)]
pub(in crate::features) struct PendingSavedConnectionStart {
    pub connection: SavedConnection,
    pub options: SavedConnectionStartOptions,
}

impl SessionStartFeatureState {
    pub(in crate::features) fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            pending: HashMap::new(),
            active_pending: None,
            failed: HashMap::new(),
            active_failed: None,
            cancelled: HashSet::new(),
            panes: HashMap::new(),
            reconnect_replace_id: None,
            reconnect_failures: HashMap::new(),
            pending_workspace_split: None,
            saved_connection_queue: VecDeque::new(),
        }
    }

    pub(in crate::features) fn sender(&self) -> mpsc::Sender<SessionStartResult> {
        self.tx.clone()
    }

    pub(in crate::features) fn try_recv(&self) -> Result<SessionStartResult, mpsc::TryRecvError> {
        self.rx.try_recv()
    }

    pub(in crate::features) fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub(in crate::features) fn has_failed(&self) -> bool {
        !self.failed.is_empty()
    }

    pub(in crate::features) fn queue_saved_connection(
        &mut self,
        connection: SavedConnection,
        options: SavedConnectionStartOptions,
    ) -> usize {
        self.saved_connection_queue
            .push_back(PendingSavedConnectionStart {
                connection,
                options,
            });
        self.saved_connection_queue.len()
    }

    pub(in crate::features) fn pop_saved_connection(
        &mut self,
    ) -> Option<PendingSavedConnectionStart> {
        self.saved_connection_queue.pop_front()
    }

    pub(in crate::features) fn saved_connection_queue_len(&self) -> usize {
        self.saved_connection_queue.len()
    }

    pub(in crate::features) fn has_queued_saved_connections(&self) -> bool {
        !self.saved_connection_queue.is_empty()
    }

    pub(in crate::features) fn saved_connection_is_queued(&self, connection_id: &str) -> bool {
        self.saved_connection_queue
            .iter()
            .any(|queued| queued.connection.id == connection_id)
    }

    pub(in crate::features) fn pending_display_name(&self) -> Option<String> {
        self.active_pending
            .as_deref()
            .and_then(|request_id| self.pending.get(request_id))
            .or_else(|| {
                self.pending
                    .values()
                    .filter(|pending| pending.reconnect_session_id.is_none())
                    .min_by(|left, right| {
                        left.requested_at
                            .cmp(&right.requested_at)
                            .then_with(|| left.connection_name.cmp(&right.connection_name))
                    })
            })
            .map(pending_session_start_display_name)
    }

    pub(in crate::features) fn active_failed(&self) -> Option<&FailedSessionStart> {
        self.active_failed
            .as_deref()
            .and_then(|request_id| self.failed.get(request_id))
    }

    pub(in crate::features) fn failed_display_name(&self) -> Option<String> {
        self.active_failed()
            .or_else(|| {
                self.failed.values().min_by(|left, right| {
                    left.pending
                        .requested_at
                        .cmp(&right.pending.requested_at)
                        .then_with(|| {
                            left.pending
                                .connection_name
                                .cmp(&right.pending.connection_name)
                        })
                })
            })
            .map(failed_session_start_display_name)
    }

    pub(in crate::features) fn select_pending(&mut self, request_id: &str) -> bool {
        if !self.pending.contains_key(request_id) {
            return false;
        }
        self.active_pending = Some(request_id.to_string());
        self.active_failed = None;
        true
    }

    pub(in crate::features) fn close_pending(
        &mut self,
        request_id: &str,
    ) -> Option<PendingSessionStart> {
        let pending = self.pending.remove(request_id)?;
        self.cancelled.insert(request_id.to_string());
        self.panes.remove(request_id);
        if self.active_pending.as_deref() == Some(request_id) {
            self.active_pending = self.latest_pending_request_id();
            if self.active_pending.is_none() {
                self.active_failed = self.latest_failed_request_id();
            }
        }
        Some(pending)
    }

    pub(in crate::features) fn select_failed(&mut self, request_id: &str) -> bool {
        if !self.failed.contains_key(request_id) {
            return false;
        }
        self.active_failed = Some(request_id.to_string());
        self.active_pending = None;
        true
    }

    pub(in crate::features) fn close_failed(
        &mut self,
        request_id: &str,
    ) -> Option<FailedSessionStart> {
        let failed = self.failed.remove(request_id)?;
        self.panes.remove(request_id);
        if self.active_failed.as_deref() == Some(request_id) {
            self.active_failed = None;
            self.active_pending = self.latest_pending_request_id();
            if self.active_pending.is_none() {
                self.active_failed = self.latest_failed_request_id();
            }
        }
        Some(failed)
    }

    pub(in crate::features) fn pending_status_source(&self) -> Option<(String, Instant)> {
        self.pending
            .values()
            .min_by(|left, right| {
                left.requested_at
                    .cmp(&right.requested_at)
                    .then_with(|| left.connection_name.cmp(&right.connection_name))
            })
            .map(|pending| (pending.connection_name.clone(), pending.requested_at))
    }

    fn latest_pending_request_id(&self) -> Option<String> {
        self.pending
            .iter()
            .filter(|(_, pending)| pending.reconnect_session_id.is_none())
            .max_by(|(left_id, left), (right_id, right)| {
                left.requested_at
                    .cmp(&right.requested_at)
                    .then_with(|| left_id.cmp(right_id))
            })
            .map(|(request_id, _)| request_id.clone())
    }

    fn latest_failed_request_id(&self) -> Option<String> {
        self.failed
            .iter()
            .max_by(|(left_id, left), (right_id, right)| {
                left.pending
                    .requested_at
                    .cmp(&right.pending.requested_at)
                    .then_with(|| left_id.cmp(right_id))
            })
            .map(|(request_id, _)| request_id.clone())
    }
}

pub(super) fn pending_session_start_display_name(pending: &PendingSessionStart) -> String {
    pending
        .custom_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(&pending.connection_name)
        .to_string()
}

pub(super) fn failed_session_start_display_name(failed: &FailedSessionStart) -> String {
    pending_session_start_display_name(&failed.pending)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Instant;

    use gpui::TestAppContext;
    use nyaterm_core::{AiExecutionProfile, ConnectionType, SavedConnection};
    use nyaterm_transport::{SessionKind, SessionManager};

    use crate::features::runtime_jobs::SessionStartResult;
    use crate::models::{
        SessionEventBridge, StartupCommandAction, TabActionsSubmenu, TerminalFramePipeline,
    };

    use super::{
        FailedSessionStart, NativeOtpProvider, PendingSessionStart, RenameSessionSubmission,
        SavedConnectionStartOptions, SessionFeatureFocus, SessionFeatureState, SessionRestoreState,
        SessionStartFeatureState,
    };

    #[test]
    fn startup_restore_completion_is_owned_and_idempotent() {
        let mut restore = SessionRestoreState::default();

        assert!(!restore.is_complete());
        assert!(restore.mark_complete());
        assert!(restore.is_complete());
        assert!(!restore.mark_complete());
    }

    fn pending(name: &str) -> PendingSessionStart {
        PendingSessionStart {
            connection_name: name.to_string(),
            launch_config: None,
            requested_at: Instant::now(),
            kind: SessionKind::LocalPty,
            ai_execution_profile: AiExecutionProfile::default(),
            custom_name: None,
            tab_color: None,
            after_session_id: None,
            insert_index: None,
            seed_output: None,
            startup_command: None,
            multiplex_key: None,
            source_connection_id: None,
            reconnect_session_id: None,
        }
    }

    fn saved_connection(id: &str) -> SavedConnection {
        SavedConnection {
            id: id.to_string(),
            name: id.to_string(),
            config: ConnectionType::LocalTerminal {
                shell_path: String::new(),
                shell_args: String::new(),
                working_dir: None,
                ai_execution_profile: AiExecutionProfile::Auto,
            },
            group_id: None,
            description: None,
            sort_order: 0,
            icon: None,
            icon_auto_detect: None,
            auth: None,
            network: None,
            post_login: None,
            created_at_ms: None,
            updated_at_ms: None,
            last_used_at_ms: None,
        }
    }

    #[test]
    fn session_state_owns_live_runtime_and_initializes_transient_state() {
        let cx = TestAppContext::single();
        let focus = || cx.update(|cx| cx.focus_handle());
        let manager = Arc::new(SessionManager::new());
        let event_bridge = SessionEventBridge::spawn(
            Arc::clone(&manager),
            TerminalFramePipeline::default(),
            "utf-8".to_string(),
            10_000,
        );
        let otp_provider = Arc::new(NativeOtpProvider::new(std::path::PathBuf::new(), None));
        let mut sessions = SessionFeatureState::new(
            Arc::clone(&manager),
            event_bridge,
            Arc::clone(&otp_provider),
            SessionFeatureFocus {
                credential: focus(),
                tab_actions: focus(),
                close_all: focus(),
                rename: focus(),
                color_picker: focus(),
                info: focus(),
                startup_command: focus(),
                temporary_ssh_link: focus(),
            },
        );

        assert!(Arc::ptr_eq(&sessions.manager, &manager));
        assert!(!sessions.start.has_pending());
        assert!(!sessions.start.has_failed());
        assert!(sessions.command_history.is_empty());
        assert!(sessions.active_id.is_none());
        assert!(sessions.active_ssh_config.is_none());
        assert_eq!(
            sessions.active_ai_execution_profile,
            AiExecutionProfile::SendOnly
        );
        assert!(sessions.order.is_empty());
        assert!(sessions.metadata.is_empty());
        assert!(sessions.zmodem.is_empty());
        assert!(sessions.trzsz.is_empty());
        assert!(sessions.multiplex_handles.is_empty());
        assert!(Arc::ptr_eq(&sessions.prompts.otp_provider, &otp_provider));
        assert!(sessions.prompts.active_credential_prompt.is_none());
        assert!(
            sessions
                .prompts
                .active_keyboard_interactive_prompt
                .is_none()
        );
        assert!(!sessions.dialogs.close_all_sessions_confirm_is_open());
        assert!(!sessions.dialogs.rename_is_open());
        assert!(!sessions.dialogs.startup_command_is_open());

        sessions
            .dialogs
            .open_startup_command(StartupCommandAction::Multiplex, 75_000);
        assert_eq!(sessions.dialogs.startup_command_delay_ms(), 60_000);
        assert!(
            sessions
                .dialogs
                .apply_text_input("startup-command", "  uptime  ".to_string())
        );
        let (action, request) = sessions
            .dialogs
            .take_startup_command()
            .expect("non-empty command should be accepted");
        assert_eq!(action, StartupCommandAction::Multiplex);
        assert_eq!(request.command, "uptime");
        assert_eq!(request.delay_ms, 60_000);
        assert!(!sessions.dialogs.startup_command_is_open());
        assert_eq!(
            sessions.dialogs.startup_command_action(),
            StartupCommandAction::Duplicate
        );

        sessions
            .dialogs
            .open_tab_actions("session-a".to_string(), Some((10.0, 20.0)));
        assert!(
            sessions
                .dialogs
                .select_tab_actions_submenu(TabActionsSubmenu::Ai)
        );
        sessions.dialogs.request_quit_after_close_all();
        sessions.dialogs.open_close_all_sessions_confirm();
        assert!(sessions.dialogs.tab_actions_session_id().is_none());
        assert!(sessions.dialogs.take_close_all_sessions_confirm());
        assert!(!sessions.dialogs.should_quit_after_close_all());

        sessions
            .dialogs
            .open_rename("session-a".to_string(), "original");
        assert!(
            sessions
                .dialogs
                .apply_text_input("rename", "   ".to_string())
        );
        assert!(matches!(
            sessions.dialogs.take_rename_submission(),
            RenameSessionSubmission::Empty
        ));
        assert!(sessions.dialogs.rename_is_open());
        assert!(
            sessions
                .dialogs
                .apply_text_input("rename", "renamed".to_string())
        );
        assert!(matches!(
            sessions.dialogs.take_rename_submission(),
            RenameSessionSubmission::Ready { session_id, name }
                if session_id == "session-a" && name == "renamed"
        ));

        sessions.dialogs.open_temporary_ssh_link();
        sessions
            .dialogs
            .apply_temporary_ssh_link("user@example.test".to_string());
        sessions
            .dialogs
            .reject_temporary_ssh_link("temporarySsh.invalid");
        assert_eq!(
            sessions.dialogs.temporary_ssh_link_error(),
            Some("temporarySsh.invalid")
        );
        sessions.dialogs.close_temporary_ssh_link();
        assert!(sessions.dialogs.temporary_ssh_link_draft().is_empty());
    }

    #[test]
    fn session_start_state_owns_channel_selection_and_cancellation() {
        let mut starts = SessionStartFeatureState::new();
        starts
            .pending
            .insert("request-1".to_string(), pending("local shell"));

        assert!(starts.select_pending("request-1"));
        assert_eq!(
            starts.pending_display_name().as_deref(),
            Some("local shell")
        );

        starts
            .sender()
            .send(SessionStartResult {
                request_id: "request-1".to_string(),
                connection_name: "local shell".to_string(),
                kind: SessionKind::LocalPty,
                worker_started_at: Instant::now(),
                worker_finished_at: Instant::now(),
                result: Err("cancelled".to_string()),
            })
            .expect("session start event channel should stay connected");
        assert_eq!(
            starts
                .try_recv()
                .expect("session start result should reach its owner")
                .request_id,
            "request-1"
        );

        let closed = starts
            .close_pending("request-1")
            .expect("selected pending start should close");
        assert_eq!(closed.connection_name, "local shell");
        assert!(starts.cancelled.contains("request-1"));
        assert!(!starts.has_pending());
        assert!(starts.active_pending.is_none());
    }

    #[test]
    fn session_start_state_owns_saved_connection_queue_lifecycle() {
        let mut starts = SessionStartFeatureState::new();

        assert!(!starts.has_queued_saved_connections());
        assert_eq!(
            starts.queue_saved_connection(
                saved_connection("connection-1"),
                SavedConnectionStartOptions::default(),
            ),
            1
        );
        assert!(starts.saved_connection_is_queued("connection-1"));
        assert!(!starts.saved_connection_is_queued("connection-2"));

        let queued = starts
            .pop_saved_connection()
            .expect("queued saved connection should remain owned by session starts");
        assert_eq!(queued.connection.id, "connection-1");
        assert!(!starts.has_queued_saved_connections());
    }

    #[test]
    fn closing_pending_starts_preserves_non_reconnect_and_failed_fallback_order() {
        let mut starts = SessionStartFeatureState::new();
        starts
            .pending
            .insert("request-active".to_string(), pending("active"));
        starts
            .pending
            .insert("request-normal".to_string(), pending("normal"));
        let mut reconnect = pending("reconnect");
        reconnect.reconnect_session_id = Some("old-session".to_string());
        starts
            .pending
            .insert("request-reconnect".to_string(), reconnect);
        starts.failed.insert(
            "request-failed".to_string(),
            FailedSessionStart {
                pending: pending("failed"),
                error: "failed".to_string(),
            },
        );

        assert!(starts.select_pending("request-active"));
        starts
            .close_pending("request-active")
            .expect("active start should close");
        assert_eq!(starts.active_pending.as_deref(), Some("request-normal"));
        assert!(starts.active_failed.is_none());

        starts
            .close_pending("request-normal")
            .expect("normal start should close");
        assert!(starts.active_pending.is_none());
        assert_eq!(starts.active_failed.as_deref(), Some("request-failed"));
        assert!(starts.pending.contains_key("request-reconnect"));
    }
}
