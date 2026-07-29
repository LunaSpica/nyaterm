use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use gpui::FocusHandle;
use nyaterm_core::{AiExecutionProfile, SavedConnection};
use nyaterm_transport::{
    SessionEvent, SessionKind, SessionManager, SshMultiplexHandle, SshSessionConfig,
};

use crate::features::runtime_jobs::SessionStartResult;
use crate::features::{DEFAULT_DUPLICATE_STARTUP_DELAY_MS, SESSION_COMMAND_HISTORY_LIMIT};
use crate::models::{
    ActiveSessionMenuState, SessionEventBridge, SessionEventBridgeDrain, SessionEventBridgeStats,
    SessionLaunchConfig, SessionRuntimeMetadata, StartupCommandAction, StartupCommandRequest,
    TabActionsSubmenu, WorkspaceSplitDirection,
};

use super::auth_runtime::{
    CredentialPromptBroker, CredentialPromptRequest, CredentialPromptState, HostKeyPromptBroker,
    HostKeyPromptRequest, KeyboardInteractivePromptState, NativeOtpCodePreview, NativeOtpProvider,
    SftpDuplicatePromptBroker, SftpDuplicatePromptState,
};
use super::trzsz_runtime::TrzszSessionState;
use super::zmodem_runtime::ZmodemSessionState;

pub(in crate::features) struct SessionFeatureState {
    manager: Arc<SessionManager>,
    event_bridge: SessionEventBridge,
    pub start: SessionStartFeatureState,
    restore: SessionRestoreState,
    events: SessionEventQueueState,
    pub prompts: SessionPromptState,
    pub dialogs: SessionDialogState,
    command_history: HashMap<String, Vec<String>>,
    active_search_draft: String,
    active_menu: Option<ActiveSessionMenuState>,
    /// Per-session reconnect/disconnect busy state ("reconnect" | "disconnect").
    busy_actions: HashMap<String, String>,
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
struct SessionRestoreState {
    complete: bool,
}

#[derive(Default)]
struct SessionEventQueueState {
    pending: VecDeque<SessionEvent>,
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
    duplicate_prompts: Arc<SftpDuplicatePromptBroker>,
    active_duplicate_prompt: Option<SftpDuplicatePromptState>,
    host_key_prompts: Arc<HostKeyPromptBroker>,
    active_host_key_prompt: Option<HostKeyPromptRequest>,
    credential_prompts: Arc<CredentialPromptBroker>,
    active_credential_prompt: Option<CredentialPromptState>,
    active_keyboard_interactive_prompt: Option<KeyboardInteractivePromptState>,
    credential_prompt_focus_pending: bool,
    credential_focus: FocusHandle,
    otp_provider: Arc<NativeOtpProvider>,
}

pub(in crate::features) enum PromptResolution<T> {
    Inactive,
    Changed,
    Ready(T),
}

pub(in crate::features) struct PromptInputTarget {
    pub id: String,
    pub seed: String,
    pub echo: bool,
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

    pub(in crate::features) fn manager(&self) -> &SessionManager {
        &self.manager
    }

    pub(in crate::features) fn manager_handle(&self) -> Arc<SessionManager> {
        Arc::clone(&self.manager)
    }

    pub(in crate::features) fn restore_is_complete(&self) -> bool {
        self.restore.is_complete()
    }

    pub(in crate::features) fn mark_restore_complete(&mut self) -> bool {
        self.restore.mark_complete()
    }

    pub(in crate::features) fn configure_event_bridge(
        &self,
        encoding: String,
        scrollback_limit: usize,
    ) {
        self.event_bridge.configure(encoding, scrollback_limit);
    }

    pub(in crate::features) fn route_session_events_to_ui(&self, session_id: &str) {
        self.event_bridge.route_session_to_ui(session_id);
    }

    pub(in crate::features) fn resume_session_direct_output(&self, session_id: &str) {
        self.event_bridge.resume_session_direct_output(session_id);
    }

    pub(in crate::features) fn clear_event_bridge_session(&self, session_id: &str) {
        self.event_bridge.clear_session(session_id);
    }

    pub(in crate::features) fn drain_event_bridge(
        &self,
        max_events: usize,
        max_output_bytes: usize,
    ) -> SessionEventBridgeDrain {
        self.event_bridge
            .drain_events_with_output_budget(max_events, max_output_bytes)
    }

    pub(in crate::features) fn harvest_event_bridge_stats(&self) -> SessionEventBridgeStats {
        self.event_bridge.harvest_direct_stats()
    }

    pub(in crate::features) fn event_bridge_has_pending_ui_work(&self) -> bool {
        self.event_bridge.has_pending_ui_work()
    }

    pub(in crate::features) fn event_bridge_queued_event_count(&self) -> usize {
        self.event_bridge.queued_event_count()
    }

    pub(in crate::features) fn event_bridge_source_queued_event_count(&self) -> usize {
        self.event_bridge.source_queued_event_count()
    }

    pub(in crate::features) fn event_bridge_queued_output_bytes(&self) -> usize {
        self.event_bridge.queued_output_bytes()
    }

    pub(in crate::features) fn event_bridge_source_queued_output_bytes(&self) -> usize {
        self.event_bridge.source_queued_output_bytes()
    }

    pub(in crate::features) fn pending_event_count(&self) -> usize {
        self.events.pending.len()
    }

    pub(in crate::features) fn pending_events_are_empty(&self) -> bool {
        self.events.pending.is_empty()
    }

    pub(in crate::features) fn extend_pending_events(
        &mut self,
        events: impl IntoIterator<Item = SessionEvent>,
    ) {
        self.events.pending.extend(events);
    }

    pub(in crate::features) fn pop_pending_event(&mut self) -> Option<SessionEvent> {
        self.events.pending.pop_front()
    }

    pub(in crate::features) fn pending_event_output_bytes(&self) -> usize {
        self.events
            .pending
            .iter()
            .map(|event| match event {
                SessionEvent::Output { data, .. } => data.len(),
                _ => 0,
            })
            .sum()
    }

    pub(in crate::features) fn command_history_for(&self, session_id: &str) -> Option<&[String]> {
        self.command_history.get(session_id).map(Vec::as_slice)
    }

    pub(in crate::features) fn record_command_history(&mut self, session_id: &str, command: &str) {
        let command = command.trim();
        if command.is_empty() {
            return;
        }
        let history = self
            .command_history
            .entry(session_id.to_string())
            .or_default();
        history.insert(0, command.to_string());
        history.truncate(SESSION_COMMAND_HISTORY_LIMIT);
    }

    pub(in crate::features) fn remove_command_history(&mut self, session_id: &str) {
        self.command_history.remove(session_id);
    }

    pub(in crate::features) fn migrate_command_history(&mut self, old_id: &str, new_id: &str) {
        if let Some(history) = self.command_history.remove(old_id) {
            self.command_history.insert(new_id.to_string(), history);
        }
    }

    pub(in crate::features) fn remove_command_from_all_history(&mut self, command: &str) {
        for history in self.command_history.values_mut() {
            history.retain(|entry| entry != command);
        }
    }

    pub(in crate::features) fn active_search_draft(&self) -> &str {
        &self.active_search_draft
    }

    pub(in crate::features) fn set_active_search_draft(&mut self, draft: String) {
        self.active_search_draft = draft;
    }

    pub(in crate::features) fn active_menu(&self) -> Option<&ActiveSessionMenuState> {
        self.active_menu.as_ref()
    }

    fn set_active_menu(&mut self, menu: ActiveSessionMenuState) {
        self.active_menu = Some(menu);
    }

    pub(in crate::features) fn toggle_active_menu(&mut self, menu: ActiveSessionMenuState) {
        if self
            .active_menu
            .as_ref()
            .is_some_and(|active| active.session_id == menu.session_id)
        {
            self.close_active_menu();
        } else {
            self.set_active_menu(menu);
        }
    }

    pub(in crate::features) fn close_active_menu(&mut self) {
        self.active_menu = None;
    }

    pub(in crate::features) fn busy_action(&self, session_id: &str) -> Option<&str> {
        self.busy_actions.get(session_id).map(String::as_str)
    }

    pub(in crate::features) fn session_is_busy(&self, session_id: &str) -> bool {
        self.busy_actions.contains_key(session_id)
    }

    fn begin_busy_action(&mut self, session_id: String, action: &'static str) -> bool {
        if self.busy_actions.contains_key(&session_id) {
            return false;
        }
        self.busy_actions.insert(session_id, action.to_string());
        self.close_active_menu();
        true
    }

    pub(in crate::features) fn begin_disconnect_action(&mut self, session_id: String) -> bool {
        self.begin_busy_action(session_id, "disconnect")
    }

    pub(in crate::features) fn begin_reconnect_action(&mut self, session_id: String) -> bool {
        self.begin_busy_action(session_id, "reconnect")
    }

    pub(in crate::features) fn finish_busy_action(&mut self, session_id: &str) {
        self.busy_actions.remove(session_id);
    }

    pub(in crate::features) fn retain_busy_actions_for_live_sessions(&mut self) {
        self.busy_actions
            .retain(|id, _| self.metadata.contains_key(id));
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

impl SessionPromptState {
    pub(in crate::features) fn duplicate_broker(&self) -> Arc<SftpDuplicatePromptBroker> {
        Arc::clone(&self.duplicate_prompts)
    }

    pub(in crate::features) fn host_key_broker(&self) -> Arc<HostKeyPromptBroker> {
        Arc::clone(&self.host_key_prompts)
    }

    pub(in crate::features) fn credential_broker(&self) -> Arc<CredentialPromptBroker> {
        Arc::clone(&self.credential_prompts)
    }

    pub(in crate::features) fn otp_provider(&self) -> Arc<NativeOtpProvider> {
        Arc::clone(&self.otp_provider)
    }

    pub(in crate::features) fn credential_focus(&self) -> &FocusHandle {
        &self.credential_focus
    }

    pub(in crate::features) fn credential_focus_is_pending(&self) -> bool {
        self.credential_prompt_focus_pending
    }

    pub(in crate::features) fn finish_credential_focus(&mut self) {
        self.credential_prompt_focus_pending = false;
    }

    pub(in crate::features) fn active_duplicate(&self) -> Option<&SftpDuplicatePromptState> {
        self.active_duplicate_prompt.as_ref()
    }

    pub(in crate::features) fn active_host_key(&self) -> Option<&HostKeyPromptRequest> {
        self.active_host_key_prompt.as_ref()
    }

    pub(in crate::features) fn active_credential(&self) -> Option<&CredentialPromptState> {
        self.active_credential_prompt.as_ref()
    }

    pub(in crate::features) fn active_keyboard_interactive(
        &self,
    ) -> Option<&KeyboardInteractivePromptState> {
        self.active_keyboard_interactive_prompt.as_ref()
    }

    pub(in crate::features) fn has_active_credential(&self) -> bool {
        self.active_credential_prompt.is_some()
    }

    pub(in crate::features) fn has_active_keyboard_interactive(&self) -> bool {
        self.active_keyboard_interactive_prompt.is_some()
    }

    pub(in crate::features) fn has_active_ssh_auth(&self) -> bool {
        self.active_host_key_prompt.is_some()
            || self.active_credential_prompt.is_some()
            || self.active_keyboard_interactive_prompt.is_some()
    }

    pub(in crate::features) fn has_pending_or_active_prompt(&self) -> bool {
        self.has_active_ssh_auth()
            || self.active_duplicate_prompt.is_some()
            || self.host_key_prompts.has_pending()
            || self.credential_prompts.has_pending()
            || self.duplicate_prompts.has_pending()
    }

    pub(in crate::features) fn take_host_key_resolution(
        &mut self,
        request_id: &str,
    ) -> PromptResolution<HostKeyPromptRequest> {
        let Some(request) = self.active_host_key_prompt.take() else {
            return PromptResolution::Inactive;
        };
        if request.id != request_id {
            self.active_host_key_prompt = Some(request);
            return PromptResolution::Changed;
        }
        PromptResolution::Ready(request)
    }

    pub(in crate::features) fn take_duplicate_resolution(
        &mut self,
        request_id: &str,
    ) -> PromptResolution<SftpDuplicatePromptState> {
        let Some(prompt) = self.active_duplicate_prompt.take() else {
            return PromptResolution::Inactive;
        };
        if prompt.id != request_id {
            self.active_duplicate_prompt = Some(prompt);
            return PromptResolution::Changed;
        }
        PromptResolution::Ready(prompt)
    }

    pub(in crate::features) fn take_credential(&mut self) -> Option<CredentialPromptState> {
        let state = self.active_credential_prompt.take()?;
        self.credential_prompt_focus_pending = false;
        Some(state)
    }

    pub(in crate::features) fn take_keyboard_interactive(
        &mut self,
    ) -> Option<KeyboardInteractivePromptState> {
        let state = self.active_keyboard_interactive_prompt.take()?;
        self.credential_prompt_focus_pending = false;
        Some(state)
    }

    pub(in crate::features) fn keyboard_interactive_otp_id(&self) -> Option<String> {
        self.active_keyboard_interactive_prompt
            .as_ref()
            .and_then(|state| state.request.otp_id.clone())
    }

    pub(in crate::features) fn keyboard_interactive_otp_code(&self) -> Option<String> {
        self.active_keyboard_interactive_prompt
            .as_ref()
            .and_then(|state| state.otp_code.clone())
    }

    pub(in crate::features) fn apply_keyboard_interactive_otp_result(
        &mut self,
        result: Result<Option<NativeOtpCodePreview>, String>,
        clear_missing_time_step: bool,
    ) -> bool {
        let Some(state) = self.active_keyboard_interactive_prompt.as_mut() else {
            return false;
        };
        match result {
            Ok(Some(preview)) => {
                state.otp_code = Some(preview.code);
                state.otp_type = Some(preview.otp_type);
                state.otp_period = preview.period;
                state.otp_time_step = preview.time_step;
                state.otp_error = None;
                true
            }
            Ok(None) => {
                state.otp_code = None;
                if clear_missing_time_step {
                    state.otp_time_step = None;
                }
                state.otp_error = Some("OTP entry not found".to_string());
                false
            }
            Err(error) => {
                state.otp_code = None;
                state.otp_time_step = None;
                state.otp_error = Some(error);
                false
            }
        }
    }

    pub(in crate::features) fn send_keyboard_interactive_otp_to_response(
        &mut self,
    ) -> Option<(String, String)> {
        let state = self.active_keyboard_interactive_prompt.as_mut()?;
        let code = state.otp_code.clone()?;
        let response = state.responses.first_mut()?;
        *response = code;
        state.focused_index = 0;
        Some((state.id.clone(), response.clone()))
    }

    pub(in crate::features) fn advance_keyboard_interactive_focus(
        &mut self,
        backwards: bool,
    ) -> Option<PromptInputTarget> {
        let state = self.active_keyboard_interactive_prompt.as_mut()?;
        let prompt_count = state.responses.len();
        if prompt_count == 0 {
            return None;
        }
        state.focused_index = if backwards {
            state
                .focused_index
                .checked_sub(1)
                .unwrap_or(prompt_count - 1)
        } else {
            (state.focused_index + 1) % prompt_count
        };
        let index = state.focused_index;
        Some(PromptInputTarget {
            id: format!("ssh.keyboard-interactive.{}.{index}", state.id),
            seed: state.responses[index].clone(),
            echo: state.request.prompts[index].echo,
        })
    }

    pub(in crate::features) fn apply_credential_input(
        &mut self,
        prompt_id: &str,
        text: String,
    ) -> bool {
        let Some(state) = self.active_credential_prompt.as_mut() else {
            return false;
        };
        if state.id != prompt_id {
            return false;
        }
        state.value = text;
        true
    }

    pub(in crate::features) fn apply_keyboard_interactive_input(
        &mut self,
        prompt_id: &str,
        index: usize,
        text: String,
    ) -> bool {
        let Some(state) = self.active_keyboard_interactive_prompt.as_mut() else {
            return false;
        };
        if state.id != prompt_id {
            return false;
        }
        let Some(response) = state.responses.get_mut(index) else {
            return false;
        };
        *response = text;
        state.focused_index = index;
        true
    }

    pub(in crate::features) fn focus_keyboard_interactive_response(
        &mut self,
        prompt_id: &str,
        index: usize,
    ) -> bool {
        let Some(state) = self.active_keyboard_interactive_prompt.as_mut() else {
            return false;
        };
        if state.id != prompt_id || index >= state.responses.len() {
            return false;
        }
        state.focused_index = index;
        true
    }

    pub(in crate::features) fn active_input_target(&self) -> Option<PromptInputTarget> {
        if let Some(state) = self.active_credential_prompt.as_ref() {
            return Some(PromptInputTarget {
                id: format!("ssh.credential.{}", state.id),
                seed: state.value.clone(),
                echo: state.prompt.echo,
            });
        }
        let state = self.active_keyboard_interactive_prompt.as_ref()?;
        let index = (!state.responses.is_empty())
            .then_some(state.focused_index.min(state.responses.len() - 1))?;
        Some(PromptInputTarget {
            id: format!("ssh.keyboard-interactive.{}.{index}", state.id),
            seed: state.responses[index].clone(),
            echo: state.request.prompts[index].echo,
        })
    }

    pub(in crate::features) fn activate_next_host_key(&mut self) -> Option<String> {
        if self.active_host_key_prompt.is_some() || !self.host_key_prompts.has_pending() {
            return None;
        }
        let request = self.host_key_prompts.pop_pending()?;
        let host = request.host_key.host_identifier.clone();
        self.active_host_key_prompt = Some(request);
        Some(host)
    }

    pub(in crate::features) fn take_next_credential_request(
        &self,
    ) -> Option<CredentialPromptRequest> {
        if self.active_credential_prompt.is_some()
            || self.active_keyboard_interactive_prompt.is_some()
            || !self.credential_prompts.has_pending()
        {
            return None;
        }
        self.credential_prompts.pop_pending()
    }

    pub(in crate::features) fn activate_credential(&mut self, state: CredentialPromptState) {
        self.active_credential_prompt = Some(state);
        self.credential_prompt_focus_pending = true;
    }

    pub(in crate::features) fn activate_keyboard_interactive(
        &mut self,
        state: KeyboardInteractivePromptState,
    ) {
        self.active_keyboard_interactive_prompt = Some(state);
        self.credential_prompt_focus_pending = true;
    }

    pub(in crate::features) fn keyboard_totp_refresh_otp_id(&self, now: u64) -> Option<String> {
        let state = self.active_keyboard_interactive_prompt.as_ref()?;
        if state.otp_type.as_deref() != Some("totp") || state.otp_code.is_none() {
            return None;
        }
        let current_step = now / state.otp_period.max(1);
        if state.otp_time_step == Some(current_step) {
            return None;
        }
        state.request.otp_id.clone()
    }

    pub(in crate::features) fn activate_next_duplicate(&mut self) -> Option<String> {
        if self.active_duplicate_prompt.is_some() || !self.duplicate_prompts.has_pending() {
            return None;
        }
        let request = self.duplicate_prompts.pop_pending()?;
        let target = request.request.target_path.clone();
        self.active_duplicate_prompt = Some(SftpDuplicatePromptState {
            id: request.id,
            request: request.request,
            response_tx: request.response_tx,
        });
        Some(target)
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
    pending: HashMap<String, PendingSessionStart>,
    active_pending: Option<String>,
    failed: HashMap<String, FailedSessionStart>,
    active_failed: Option<String>,
    cancelled: HashSet<String>,
    reconnect_replace_id: Option<String>,
    reconnect_failures: HashMap<String, String>,
    pending_workspace_split: Option<(WorkspaceSplitDirection, String)>,
    saved_connection_queue: VecDeque<PendingSavedConnectionStart>,
}

pub(in crate::features) enum SessionStartEventRequest {
    Cancelled,
    Pending {
        pending: Option<PendingSessionStart>,
        was_active: bool,
    },
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

    pub(in crate::features) fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub(in crate::features) fn has_cancelled_results(&self) -> bool {
        !self.cancelled.is_empty()
    }

    pub(in crate::features) fn has_active_pending(&self) -> bool {
        self.active_pending.is_some()
    }

    pub(in crate::features) fn has_active_failed(&self) -> bool {
        self.active_failed.is_some()
    }

    pub(in crate::features) fn request_is_active(&self, request_id: &str) -> bool {
        self.active_pending.as_deref() == Some(request_id)
            || self.active_failed.as_deref() == Some(request_id)
    }

    pub(in crate::features) fn pending_entries(
        &self,
    ) -> impl Iterator<Item = (&String, &PendingSessionStart)> {
        self.pending.iter()
    }

    pub(in crate::features) fn failed_entries(
        &self,
    ) -> impl Iterator<Item = (&String, &FailedSessionStart)> {
        self.failed.iter()
    }

    pub(in crate::features) fn source_connection_is_pending(&self, connection_id: &str) -> bool {
        self.pending
            .values()
            .any(|pending| pending.source_connection_id.as_deref() == Some(connection_id))
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

    pub(in crate::features) fn register_pending(
        &mut self,
        request_id: String,
        mut pending: PendingSessionStart,
    ) -> bool {
        pending.reconnect_session_id = self.reconnect_replace_id.clone();
        let reconnecting = pending.reconnect_session_id.is_some();
        self.pending.insert(request_id.clone(), pending);
        if !reconnecting {
            self.active_pending = Some(request_id);
            self.active_failed = None;
        }
        reconnecting
    }

    pub(in crate::features) fn take_event_request(
        &mut self,
        request_id: &str,
    ) -> SessionStartEventRequest {
        if self.cancelled.remove(request_id) {
            return SessionStartEventRequest::Cancelled;
        }
        let was_active = self.active_pending.as_deref() == Some(request_id);
        let pending = self.pending.remove(request_id);
        if was_active {
            self.active_pending = None;
        }
        SessionStartEventRequest::Pending {
            pending,
            was_active,
        }
    }

    pub(in crate::features) fn complete_success(
        &mut self,
        reconnecting: bool,
        was_active: bool,
        no_active_session: bool,
    ) -> bool {
        if reconnecting {
            self.reconnect_replace_id = None;
        }
        was_active || (self.active_pending.is_none() && no_active_session)
    }

    pub(in crate::features) fn record_failure(
        &mut self,
        request_id: String,
        pending: Option<PendingSessionStart>,
        error: String,
        was_active: bool,
        reconnect_session_exists: bool,
    ) -> bool {
        let reconnect_session_id = pending
            .as_ref()
            .and_then(|pending| pending.reconnect_session_id.clone());
        if let Some(session_id) = reconnect_session_id {
            self.reconnect_replace_id = None;
            if reconnect_session_exists {
                self.reconnect_failures.insert(session_id, error);
            }
            return true;
        }
        if let Some(pending) = pending {
            self.failed
                .insert(request_id.clone(), FailedSessionStart { pending, error });
            if was_active {
                self.active_failed = Some(request_id);
            }
        }
        false
    }

    pub(in crate::features) fn clear_active_selection(&mut self) {
        self.active_pending = None;
        self.active_failed = None;
    }

    pub(in crate::features) fn reconnect_target(&self) -> Option<&str> {
        self.reconnect_replace_id.as_deref()
    }

    pub(in crate::features) fn set_reconnect_target(&mut self, session_id: String) {
        self.reconnect_replace_id = Some(session_id);
    }

    pub(in crate::features) fn clear_reconnect_target(&mut self) {
        self.reconnect_replace_id = None;
    }

    pub(in crate::features) fn reconnect_is_pending(&self, session_id: &str) -> bool {
        self.pending
            .values()
            .any(|pending| pending.reconnect_session_id.as_deref() == Some(session_id))
    }

    pub(in crate::features) fn reconnect_failure(&self, session_id: &str) -> Option<&str> {
        self.reconnect_failures.get(session_id).map(String::as_str)
    }

    pub(in crate::features) fn clear_reconnect_failure(&mut self, session_id: &str) {
        self.reconnect_failures.remove(session_id);
    }

    pub(in crate::features) fn set_pending_workspace_split(
        &mut self,
        direction: WorkspaceSplitDirection,
        source_session_id: String,
    ) {
        self.pending_workspace_split = Some((direction, source_session_id));
    }

    pub(in crate::features) fn take_pending_workspace_split(
        &mut self,
    ) -> Option<(WorkspaceSplitDirection, String)> {
        self.pending_workspace_split.take()
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
    use std::sync::{Arc, mpsc};
    use std::time::Instant;

    use gpui::{TestAppContext, px};
    use nyaterm_core::{AiExecutionProfile, ConnectionType, SavedConnection};
    use nyaterm_transport::{
        SessionEvent, SessionKind, SessionManager, SshCredentialPrompt, SshCredentialPromptKind,
        SshCredentialPromptReason, SshHostKey, SshKeyboardInteractivePrompt,
        SshKeyboardInteractiveRequest,
    };

    use crate::features::runtime_jobs::SessionStartResult;
    use crate::features::session::HostKeyPromptIssue;
    use crate::models::{
        ActiveSessionMenuState, SessionEventBridge, StartupCommandAction, TabActionsSubmenu,
        TerminalFramePipeline, WorkspaceSplitDirection,
    };

    use super::{
        CredentialPromptBroker, CredentialPromptState, FailedSessionStart, HostKeyPromptBroker,
        HostKeyPromptRequest, KeyboardInteractivePromptState, NativeOtpCodePreview,
        NativeOtpProvider, PendingSessionStart, PromptResolution, RenameSessionSubmission,
        SavedConnectionStartOptions, SessionFeatureFocus, SessionFeatureState, SessionPromptState,
        SessionStartEventRequest, SessionStartFeatureState, SftpDuplicatePromptBroker,
    };

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

    fn prompt_state(cx: &TestAppContext) -> SessionPromptState {
        SessionPromptState {
            duplicate_prompts: Arc::new(SftpDuplicatePromptBroker::default()),
            active_duplicate_prompt: None,
            host_key_prompts: Arc::new(HostKeyPromptBroker::default()),
            active_host_key_prompt: None,
            credential_prompts: Arc::new(CredentialPromptBroker::default()),
            active_credential_prompt: None,
            active_keyboard_interactive_prompt: None,
            credential_prompt_focus_pending: false,
            credential_focus: cx.update(|cx| cx.focus_handle()),
            otp_provider: Arc::new(NativeOtpProvider::new(std::path::PathBuf::new(), None)),
        }
    }

    fn credential_prompt_state(id: &str) -> CredentialPromptState {
        let (response_tx, _response_rx) = mpsc::channel();
        CredentialPromptState {
            id: id.to_string(),
            prompt: SshCredentialPrompt {
                host: "example.test".to_string(),
                port: 22,
                username: "nya".to_string(),
                connection_name: "example".to_string(),
                kind: SshCredentialPromptKind::Password,
                reason: SshCredentialPromptReason::MissingPassword,
                attempt: 1,
                prompt_text: None,
                echo: false,
            },
            response_tx,
            value: String::new(),
        }
    }

    #[test]
    fn credential_prompt_owner_isolates_input_and_clears_focus_on_take() {
        let cx = TestAppContext::single();
        let mut prompts = prompt_state(&cx);

        prompts.activate_credential(credential_prompt_state("credential-1"));
        assert!(prompts.credential_focus_is_pending());
        assert!(!prompts.apply_credential_input("credential-2", "wrong".to_string()));
        assert_eq!(
            prompts
                .active_credential()
                .expect("credential prompt should remain active")
                .value,
            ""
        );

        assert!(prompts.apply_credential_input("credential-1", "secret".to_string()));
        let prompt = prompts
            .take_credential()
            .expect("matching credential prompt should be taken");
        assert_eq!(prompt.id, "credential-1");
        assert_eq!(prompt.value, "secret");
        assert!(!prompts.credential_focus_is_pending());
        assert!(prompts.active_credential().is_none());
    }

    #[test]
    fn mismatched_host_key_resolution_preserves_active_prompt() {
        let cx = TestAppContext::single();
        let mut prompts = prompt_state(&cx);
        let (response_tx, _response_rx) = mpsc::channel();
        prompts.active_host_key_prompt = Some(HostKeyPromptRequest {
            id: "host-key-1".to_string(),
            host_key: SshHostKey {
                host: "example.test".to_string(),
                port: 22,
                host_identifier: "example.test".to_string(),
                key_type: "ssh-ed25519".to_string(),
                key_base64: "test-key".to_string(),
                fingerprint: "SHA256:test".to_string(),
            },
            issue: HostKeyPromptIssue::Unknown,
            response_tx,
        });

        assert!(matches!(
            prompts.take_host_key_resolution("host-key-2"),
            PromptResolution::Changed
        ));
        assert_eq!(
            prompts
                .active_host_key()
                .expect("mismatched resolution must restore the prompt")
                .id,
            "host-key-1"
        );
    }

    #[test]
    fn otp_missing_entry_preserves_manual_timing_but_clears_refresh_timing() {
        let cx = TestAppContext::single();
        let mut prompts = prompt_state(&cx);
        let (response_tx, _response_rx) = mpsc::channel();
        prompts.activate_keyboard_interactive(KeyboardInteractivePromptState {
            id: "keyboard-1".to_string(),
            request: SshKeyboardInteractiveRequest {
                host: "example.test".to_string(),
                port: 22,
                username: "nya".to_string(),
                connection_name: "example".to_string(),
                name: "verification".to_string(),
                instructions: String::new(),
                round: 1,
                prompts: vec![SshKeyboardInteractivePrompt {
                    prompt: "Code".to_string(),
                    echo: false,
                }],
                otp_id: Some("otp-1".to_string()),
            },
            response_tx,
            responses: vec![String::new()],
            focused_index: 0,
            otp_code: Some("test-code".to_string()),
            otp_type: Some("totp".to_string()),
            otp_period: 30,
            otp_time_step: Some(7),
            otp_error: None,
        });

        assert!(!prompts.apply_keyboard_interactive_otp_result(Ok(None), false));
        assert_eq!(
            prompts
                .active_keyboard_interactive()
                .expect("keyboard prompt should remain active")
                .otp_time_step,
            Some(7)
        );

        assert!(!prompts.apply_keyboard_interactive_otp_result(Ok(None), true));
        let active = prompts
            .active_keyboard_interactive()
            .expect("keyboard prompt should remain active");
        assert!(active.otp_time_step.is_none());
        assert_eq!(active.otp_error.as_deref(), Some("OTP entry not found"));

        assert!(prompts.apply_keyboard_interactive_otp_result(
            Ok(Some(NativeOtpCodePreview {
                code: "next-code".to_string(),
                otp_type: "totp".to_string(),
                period: 30,
                time_step: Some(8),
            })),
            true,
        ));
        let active = prompts
            .active_keyboard_interactive()
            .expect("keyboard prompt should remain active");
        assert_eq!(active.otp_time_step, Some(8));
        assert!(active.otp_error.is_none());
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

        assert!(Arc::ptr_eq(&sessions.manager_handle(), &manager));
        assert!(!sessions.start.has_pending());
        assert!(!sessions.start.has_failed());
        assert!(sessions.command_history_for("missing").is_none());
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
        assert!(Arc::ptr_eq(&sessions.prompts.otp_provider(), &otp_provider));
        assert!(sessions.prompts.active_credential().is_none());
        assert!(sessions.prompts.active_keyboard_interactive().is_none());
        assert!(!sessions.dialogs.close_all_sessions_confirm_is_open());
        assert!(!sessions.dialogs.rename_is_open());
        assert!(!sessions.dialogs.startup_command_is_open());

        assert!(!sessions.restore_is_complete());
        assert!(sessions.mark_restore_complete());
        assert!(!sessions.mark_restore_complete());

        sessions.extend_pending_events([
            SessionEvent::Output {
                session_id: "session-a".to_string(),
                data: vec![1, 2, 3],
            },
            SessionEvent::Error {
                session_id: "session-a".to_string(),
                message: "test error".to_string(),
            },
        ]);
        assert_eq!(sessions.pending_event_count(), 2);
        assert_eq!(sessions.pending_event_output_bytes(), 3);
        assert!(matches!(
            sessions.pop_pending_event(),
            Some(SessionEvent::Output { data, .. }) if data == vec![1, 2, 3]
        ));

        sessions.record_command_history("session-a", "  git status  ");
        sessions.record_command_history("session-a", "cargo check");
        assert_eq!(
            sessions.command_history_for("session-a"),
            Some(["cargo check".to_string(), "git status".to_string()].as_slice())
        );
        sessions.migrate_command_history("session-a", "session-b");
        sessions.remove_command_from_all_history("git status");
        assert_eq!(
            sessions.command_history_for("session-b"),
            Some(["cargo check".to_string()].as_slice())
        );

        let menu = ActiveSessionMenuState {
            session_id: "session-b".to_string(),
            x: px(10.),
            y: px(20.),
        };
        sessions.toggle_active_menu(menu.clone());
        assert_eq!(
            sessions.active_menu().map(|menu| menu.session_id.as_str()),
            Some("session-b")
        );
        sessions.toggle_active_menu(menu);
        assert!(sessions.active_menu().is_none());
        sessions.set_active_menu(ActiveSessionMenuState {
            session_id: "session-b".to_string(),
            x: px(10.),
            y: px(20.),
        });
        assert!(sessions.begin_reconnect_action("session-b".to_string()));
        assert!(sessions.active_menu().is_none());
        assert_eq!(sessions.busy_action("session-b"), Some("reconnect"));
        assert!(!sessions.begin_disconnect_action("session-b".to_string()));
        sessions.finish_busy_action("session-b");
        assert!(!sessions.session_is_busy("session-b"));

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
        assert!(starts.has_cancelled_results());
        assert!(matches!(
            starts.take_event_request("request-1"),
            SessionStartEventRequest::Cancelled
        ));
        assert!(!starts.has_cancelled_results());
        assert!(!starts.has_pending());
        assert!(!starts.has_active_pending());
    }

    #[test]
    fn session_start_registration_owns_fresh_and_reconnect_selection() {
        let mut fresh = SessionStartFeatureState::new();
        assert!(!fresh.register_pending("request-fresh".to_string(), pending("fresh")));
        assert!(fresh.request_is_active("request-fresh"));
        assert_eq!(fresh.pending_count(), 1);

        let mut reconnect = SessionStartFeatureState::new();
        reconnect.set_reconnect_target("session-old".to_string());
        assert!(reconnect.register_pending("request-reconnect".to_string(), pending("reconnect")));
        assert!(!reconnect.has_active_pending());
        assert!(reconnect.reconnect_is_pending("session-old"));
        assert_eq!(reconnect.reconnect_target(), Some("session-old"));
    }

    #[test]
    fn session_start_results_route_normal_and_reconnect_failures_atomically() {
        let mut fresh = SessionStartFeatureState::new();
        fresh.register_pending("request-fresh".to_string(), pending("fresh"));
        let SessionStartEventRequest::Pending {
            pending: pending_state,
            was_active,
        } = fresh.take_event_request("request-fresh")
        else {
            panic!("fresh result should retain pending metadata");
        };
        assert!(was_active);
        assert!(!fresh.record_failure(
            "request-fresh".to_string(),
            pending_state,
            "connection failed".to_string(),
            was_active,
            false,
        ));
        assert!(fresh.has_failed());
        assert!(fresh.has_active_failed());
        assert_eq!(
            fresh
                .active_failed()
                .expect("active failure should be retained")
                .error,
            "connection failed"
        );

        let mut reconnect = SessionStartFeatureState::new();
        reconnect.set_reconnect_target("session-old".to_string());
        reconnect.register_pending("request-reconnect".to_string(), pending("reconnect"));
        let SessionStartEventRequest::Pending {
            pending: pending_state,
            was_active,
        } = reconnect.take_event_request("request-reconnect")
        else {
            panic!("reconnect result should retain pending metadata");
        };
        assert!(!was_active);
        assert!(reconnect.record_failure(
            "request-reconnect".to_string(),
            pending_state,
            "reconnect failed".to_string(),
            was_active,
            true,
        ));
        assert!(!reconnect.has_failed());
        assert_eq!(
            reconnect.reconnect_failure("session-old"),
            Some("reconnect failed")
        );
        assert!(reconnect.reconnect_target().is_none());
    }

    #[test]
    fn session_start_success_and_workspace_split_are_single_owner_transitions() {
        let mut starts = SessionStartFeatureState::new();
        starts.set_reconnect_target("session-old".to_string());
        assert!(!starts.complete_success(true, false, false));
        assert!(starts.reconnect_target().is_none());

        starts.set_pending_workspace_split(
            WorkspaceSplitDirection::Horizontal,
            "session-source".to_string(),
        );
        assert!(matches!(
            starts.take_pending_workspace_split(),
            Some((WorkspaceSplitDirection::Horizontal, source)) if source == "session-source"
        ));
        assert!(starts.take_pending_workspace_split().is_none());
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
