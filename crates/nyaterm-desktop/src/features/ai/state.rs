//! Grouped AI feature state.
//!
//! The AI panel spans several independent concerns: provider settings, the
//! chat composer and transcript, session history, model discovery, and the
//! agent loop. They were seventy `ai_*` fields on `NyaTermApp`, which made it
//! impossible to see which ones move together.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Instant;

use gpui::FocusHandle;
use nyaterm_core::{
    AgentCaptureProcessResult, AgentCommandExecutionMode, AgentOutputCaptureProcessor,
    AiCommandCard, AiMessage, AiMessageRole, AiMode, AiSession, AiSettings, truncate_preview, uuid,
};

use crate::features::{AiAgentLoopState, AiAgentStepView, AiChatWorkerEvent, AiDiscoveryJobResult};
use crate::models::{
    AiActionEditorField, AiActionListKind, AiCredentialEditorField, AiDetectedErrorState,
    AiInputField, AiMessageMenuState, AiPreparedRequest,
};

pub(in crate::features) struct AiFeatureState {
    pub(super) settings: AiSettingsState,
    pub(super) chat: AiChatState,
    history: AiHistoryState,
    discovery: AiDiscoveryState,
    pub(super) agent: AiAgentState,
    panel: AiPanelState,
}

/// Focus handles the AI feature needs at construction time.
pub(in crate::features) struct AiFeatureFocus {
    pub chat: FocusHandle,
    pub clear_history_confirm: FocusHandle,
    pub auto_execution_confirm: FocusHandle,
    pub action: FocusHandle,
    pub manual_model: FocusHandle,
    pub credential: FocusHandle,
}

/// Provider settings, model catalog editing and credential drafts.
pub(super) struct AiSettingsState {
    pub(super) config: AiSettings,
    pub(super) model_draft: String,
    pub(super) base_url_draft: String,
    pub(super) secret_draft: String,
    pub(super) model_collapsed_groups: HashSet<String>,
    pub(super) model_query: String,
    pub(super) manual_model_drafts: HashMap<String, String>,
    pub(super) manual_model_focus: FocusHandle,
    pub(super) manual_model_edit_group: Option<String>,
    /// Per-credential API-key drafts; empty means keep the stored secret.
    pub(super) credential_secret_drafts: HashMap<String, String>,
    pub(super) credential_edit: Option<(String, AiCredentialEditorField)>,
    pub(super) credential_focus: FocusHandle,
    pub(super) action_edit: Option<(AiActionListKind, String, AiActionEditorField)>,
    pub(super) action_focus: FocusHandle,
}

/// Composer, in-flight request and the visible transcript.
pub(super) struct AiChatState {
    pub(super) tx: mpsc::Sender<AiChatWorkerEvent>,
    pub(super) rx: mpsc::Receiver<AiChatWorkerEvent>,
    pub(super) pending: bool,
    pub(super) job_id: u64,
    pub(super) cancel: Option<Arc<AtomicBool>>,
    pub(super) session_id: String,
    pub(super) prompt_draft: String,
    pub(super) target_session_ids: Vec<String>,
    pub(super) mention_open: bool,
    pub(super) mention_query: String,
    pub(super) mention_index: usize,
    pub(super) prepared_request: Option<AiPreparedRequest>,
    pub(super) response_preview: String,
    pub(super) messages: Vec<AiMessage>,
    pub(super) streaming_assistant_id: Option<String>,
    pub(super) message_menu: Option<AiMessageMenuState>,
    pub(super) quoted_text: Option<String>,
    pub(super) command_cards: Vec<AiCommandCard>,
    pub(super) focus: FocusHandle,
    pub(super) focus_pending: bool,
}

/// Stored sessions, the history browser and the counters shown beside it.
struct AiHistoryState {
    open: bool,
    query: String,
    job_id: u64,
    pending: bool,
    sessions: Vec<AiSession>,
    session_count: usize,
    message_count: usize,
    audit_count: usize,
    usage_count_job_id: u64,
    audit_write_lock: Arc<Mutex<()>>,
    clear_confirm_open: bool,
    clear_confirm_focus: FocusHandle,
}

/// Model discovery job and the model picker it feeds.
struct AiDiscoveryState {
    tx: mpsc::Sender<AiDiscoveryJobResult>,
    rx: mpsc::Receiver<AiDiscoveryJobResult>,
    pending: bool,
    menu_open: bool,
    query: String,
    index: usize,
}

/// Agent loop: the running task, its steps and their disclosure state.
pub(super) struct AiAgentState {
    pub(super) task_prompt: Option<String>,
    pub(super) step_index: u16,
    pub(super) loop_state: Option<AiAgentLoopState>,
    pub(super) capture: AgentOutputCaptureProcessor,
    pub(super) steps: Vec<AiAgentStepView>,
    pub(super) thought_expanded: HashSet<u16>,
    pub(super) output_expanded: HashSet<u16>,
    pub(super) auto_execution_confirm_open: bool,
    pub(super) auto_execution_confirm_focus: FocusHandle,
}

/// Panel chrome: status line, focus routing and the detected-error banner.
struct AiPanelState {
    execution_menu_open: bool,
    status: String,
    focused_field: AiInputField,
    detected_error: Option<AiDetectedErrorState>,
    error_notice_at: HashMap<String, Instant>,
}

impl AiFeatureState {
    pub(in crate::features) fn new(
        settings: AiSettings,
        model_draft: String,
        base_url_draft: String,
        chat_session_id: String,
        session_count: usize,
        message_count: usize,
        audit_count: usize,
        focus: AiFeatureFocus,
    ) -> Self {
        let (chat_tx, chat_rx) = mpsc::channel();
        let (discovery_tx, discovery_rx) = mpsc::channel();
        Self {
            settings: AiSettingsState {
                config: settings,
                model_draft,
                base_url_draft,
                secret_draft: String::new(),
                model_collapsed_groups: HashSet::new(),
                model_query: String::new(),
                manual_model_drafts: HashMap::new(),
                manual_model_focus: focus.manual_model,
                manual_model_edit_group: None,
                credential_secret_drafts: HashMap::new(),
                credential_edit: None,
                credential_focus: focus.credential,
                action_edit: None,
                action_focus: focus.action,
            },
            chat: AiChatState {
                tx: chat_tx,
                rx: chat_rx,
                pending: false,
                job_id: 0,
                cancel: None,
                session_id: chat_session_id,
                prompt_draft: String::new(),
                target_session_ids: Vec::new(),
                mention_open: false,
                mention_query: String::new(),
                mention_index: 0,
                prepared_request: None,
                response_preview: "Ask mode ready".to_string(),
                messages: Vec::new(),
                streaming_assistant_id: None,
                message_menu: None,
                quoted_text: None,
                command_cards: Vec::new(),
                focus: focus.chat,
                focus_pending: false,
            },
            history: AiHistoryState {
                open: false,
                query: String::new(),
                job_id: 0,
                pending: false,
                sessions: Vec::new(),
                session_count,
                message_count,
                audit_count,
                usage_count_job_id: 0,
                audit_write_lock: Arc::new(Mutex::new(())),
                clear_confirm_open: false,
                clear_confirm_focus: focus.clear_history_confirm,
            },
            discovery: AiDiscoveryState {
                tx: discovery_tx,
                rx: discovery_rx,
                pending: false,
                menu_open: false,
                query: String::new(),
                index: 0,
            },
            agent: AiAgentState {
                task_prompt: None,
                step_index: 0,
                loop_state: None,
                capture: AgentOutputCaptureProcessor::new(),
                steps: Vec::new(),
                thought_expanded: HashSet::new(),
                output_expanded: HashSet::new(),
                auto_execution_confirm_open: false,
                auto_execution_confirm_focus: focus.auto_execution_confirm,
            },
            panel: AiPanelState {
                execution_menu_open: false,
                status: "AI settings ready".to_string(),
                focused_field: AiInputField::Model,
                detected_error: None,
                error_notice_at: HashMap::new(),
            },
        }
    }

    pub(in crate::features) fn settings_config(&self) -> &AiSettings {
        &self.settings.config
    }

    pub(in crate::features) fn settings_draft_snapshot(
        &self,
    ) -> (AiSettings, String, String, String) {
        (
            self.settings.config.clone(),
            self.settings.model_draft.clone(),
            self.settings.base_url_draft.clone(),
            self.settings.secret_draft.clone(),
        )
    }

    pub(in crate::features) fn settings_draft_matches(
        &self,
        config: &AiSettings,
        model: &str,
        base_url: &str,
        secret: &str,
    ) -> bool {
        &self.settings.config == config
            && self.settings.model_draft == model
            && self.settings.base_url_draft == base_url
            && self.settings.secret_draft == secret
    }

    pub(in crate::features) fn replace_settings_config(
        &mut self,
        config: AiSettings,
        clear_secret_draft: bool,
    ) {
        self.settings.config = config;
        if clear_secret_draft {
            self.settings.secret_draft.clear();
        }
    }

    pub(in crate::features) fn restore_settings_draft(
        &mut self,
        config: AiSettings,
        model: String,
        base_url: String,
        secret: String,
    ) {
        self.settings.config = config;
        self.settings.model_draft = model;
        self.settings.base_url_draft = base_url;
        self.settings.secret_draft = secret;
    }

    pub(in crate::features) fn close_settings_editors(&mut self) {
        self.settings.action_edit = None;
        self.settings.manual_model_edit_group = None;
    }

    pub(in crate::features) fn settings_model_draft(&self) -> &str {
        &self.settings.model_draft
    }

    pub(in crate::features) fn settings_model_query(&self) -> &str {
        &self.settings.model_query
    }

    pub(in crate::features) fn clear_settings_model_query(&mut self) {
        self.settings.model_query.clear();
    }

    pub(in crate::features) fn settings_model_collapsed_groups(&self) -> &HashSet<String> {
        &self.settings.model_collapsed_groups
    }

    pub(in crate::features) fn settings_manual_model_drafts(&self) -> &HashMap<String, String> {
        &self.settings.manual_model_drafts
    }

    pub(in crate::features) fn settings_credential_secret_drafts(
        &self,
    ) -> &HashMap<String, String> {
        &self.settings.credential_secret_drafts
    }

    pub(in crate::features) fn settings_credential_edit(
        &self,
    ) -> Option<&(String, AiCredentialEditorField)> {
        self.settings.credential_edit.as_ref()
    }

    pub(in crate::features) fn settings_action_focus(&self) -> &FocusHandle {
        &self.settings.action_focus
    }

    pub(in crate::features) fn chat_or_agent_is_running(&self) -> bool {
        self.chat.pending || self.agent.loop_state.is_some()
    }

    pub(in crate::features) fn has_background_work(&self) -> bool {
        self.chat.pending || self.agent.loop_state.is_some() || self.discovery.pending
    }

    pub(in crate::features) fn chat_focus(&self) -> &FocusHandle {
        &self.chat.focus
    }

    pub(in crate::features) fn chat_focus_is_pending(&self) -> bool {
        self.chat.focus_pending
    }

    pub(in crate::features) fn take_chat_focus_request(&mut self) -> bool {
        std::mem::take(&mut self.chat.focus_pending)
    }

    pub(in crate::features) fn chat_session_id(&self) -> &str {
        &self.chat.session_id
    }

    pub(in crate::features) fn chat_prompt_draft(&self) -> &str {
        &self.chat.prompt_draft
    }

    pub(in crate::features) fn chat_target_session_ids(&self) -> &[String] {
        &self.chat.target_session_ids
    }

    pub(in crate::features) fn chat_targets_session(&self, session_id: &str) -> bool {
        self.chat
            .target_session_ids
            .iter()
            .any(|target_id| target_id == session_id)
    }

    pub(in crate::features) fn chat_mention_is_open(&self) -> bool {
        self.chat.mention_open
    }

    pub(in crate::features) fn chat_mention_index(&self) -> usize {
        self.chat.mention_index
    }

    pub(in crate::features) fn clamp_chat_mention_index(&mut self, len: usize) -> usize {
        if len == 0 {
            self.chat.mention_index = 0;
        } else {
            self.chat.mention_index = self.chat.mention_index.min(len - 1);
        }
        self.chat.mention_index
    }

    pub(in crate::features) fn set_chat_mention_index(&mut self, index: usize) {
        self.chat.mention_index = index;
    }

    pub(in crate::features) fn chat_prepared_request(&self) -> Option<&AiPreparedRequest> {
        self.chat.prepared_request.as_ref()
    }

    pub(in crate::features) fn chat_response_preview(&self) -> &str {
        &self.chat.response_preview
    }

    pub(in crate::features) fn set_chat_response_preview(&mut self, preview: impl Into<String>) {
        self.chat.response_preview = preview.into();
    }

    pub(in crate::features) fn chat_messages(&self) -> &[AiMessage] {
        &self.chat.messages
    }

    pub(in crate::features) fn chat_streaming_assistant_id(&self) -> Option<&str> {
        self.chat.streaming_assistant_id.as_deref()
    }

    pub(in crate::features) fn chat_command_cards(&self) -> &[AiCommandCard] {
        &self.chat.command_cards
    }

    pub(in crate::features) fn command_card(&self, index: usize) -> Option<AiCommandCard> {
        self.chat.command_cards.get(index).cloned()
    }

    pub(in crate::features) fn find_command_card(&self, card_id: &str) -> Option<AiCommandCard> {
        self.chat
            .command_cards
            .iter()
            .find(|card| card.id == card_id)
            .cloned()
            .or_else(|| {
                self.chat
                    .messages
                    .iter()
                    .flat_map(|message| message.command_cards.iter())
                    .find(|card| card.id == card_id)
                    .cloned()
            })
    }

    pub(in crate::features) fn chat_message_menu(&self) -> Option<&AiMessageMenuState> {
        self.chat.message_menu.as_ref()
    }

    pub(in crate::features) fn chat_quote(&self) -> Option<&str> {
        self.chat.quoted_text.as_deref()
    }

    pub(in crate::features) fn close_message_menu(&mut self) {
        self.chat.close_message_menu();
    }

    pub(in crate::features) fn open_message_menu(&mut self, menu: AiMessageMenuState) {
        self.chat.message_menu = Some(menu);
        self.history.open = false;
        self.panel.execution_menu_open = false;
        self.discovery.menu_open = false;
    }

    pub(in crate::features) fn quote_message(&mut self, text: String) -> bool {
        let value = text.trim().to_string();
        let quoted = !value.is_empty();
        if quoted {
            self.chat.quoted_text = Some(value);
            self.panel.status = "AI message quoted".to_string();
        } else {
            self.panel.status = "AI message is empty".to_string();
        }
        self.chat.message_menu = None;
        quoted
    }

    pub(in crate::features) fn finish_copy_message(&mut self, copied: bool) {
        self.panel.status = if copied {
            "AI message copied".to_string()
        } else {
            "AI message is empty".to_string()
        };
        self.chat.message_menu = None;
    }

    pub(in crate::features) fn prepare_external_request(
        &mut self,
        request: AiPreparedRequest,
        response_preview: impl Into<String>,
        status: impl Into<String>,
        focus: bool,
    ) {
        self.chat.prepared_request = Some(request);
        self.chat.response_preview = response_preview.into();
        self.panel.status = status.into();
        self.chat.focus_pending = focus;
        self.close_transient_menus();
    }

    pub(in crate::features) fn prepare_detected_error_request(
        &mut self,
        request: AiPreparedRequest,
        session_id: String,
    ) {
        self.chat.prepared_request = Some(request);
        if !self.chat.target_session_ids.contains(&session_id) {
            self.chat.target_session_ids.push(session_id);
        }
        self.close_transient_menus();
    }

    pub(in crate::features) fn history_is_open(&self) -> bool {
        self.history.open
    }

    pub(in crate::features) fn history_query(&self) -> &str {
        &self.history.query
    }

    pub(in crate::features) fn history_sessions(&self) -> &[AiSession] {
        &self.history.sessions
    }

    pub(in crate::features) fn history_is_pending(&self) -> bool {
        self.history.pending
    }

    pub(in crate::features) fn history_clear_confirm_is_open(&self) -> bool {
        self.history.clear_confirm_open
    }

    pub(in crate::features) fn history_clear_confirm_focus(&self) -> &FocusHandle {
        &self.history.clear_confirm_focus
    }

    pub(in crate::features) fn request_history_clear_confirm(&mut self) -> Option<FocusHandle> {
        if self.history.sessions.is_empty() {
            return None;
        }
        self.history.clear_confirm_open = true;
        self.chat.message_menu = None;
        self.discovery.menu_open = false;
        self.panel.execution_menu_open = false;
        Some(self.history.clear_confirm_focus.clone())
    }

    pub(in crate::features) fn cancel_history_clear_confirm(&mut self) {
        self.history.cancel_clear_confirm();
    }

    pub(in crate::features) fn confirm_history_clear(&mut self) -> bool {
        if !self.history.clear_confirm_open {
            return false;
        }
        self.history.clear_confirm_open = false;
        self.history.open = false;
        true
    }

    pub(in crate::features) fn close_history(&mut self) {
        self.history.open = false;
        self.history.query.clear();
    }

    pub(in crate::features) fn clear_history_query(&mut self) {
        self.history.query.clear();
    }

    pub(in crate::features) fn toggle_history(&mut self) -> bool {
        self.panel.execution_menu_open = false;
        self.history.open = !self.history.open;
        if self.history.open {
            self.chat.message_menu = None;
            self.discovery.menu_open = false;
        } else {
            self.history.query.clear();
        }
        self.history.open
    }

    pub(in crate::features) fn history_actions_are_disabled(&self) -> bool {
        self.history.sessions.is_empty() || self.history.pending || self.chat_or_agent_is_running()
    }

    pub(in crate::features) fn history_audit_write_lock(&self) -> Arc<Mutex<()>> {
        Arc::clone(&self.history.audit_write_lock)
    }

    pub(in crate::features) fn begin_history_operation(
        &mut self,
        status: impl Into<String>,
    ) -> Option<u64> {
        if self.history.pending {
            self.panel.status = "AI history operation already in progress".to_string();
            return None;
        }
        self.history.job_id = self.history.job_id.wrapping_add(1).max(1);
        self.history.pending = true;
        self.panel.status = status.into();
        Some(self.history.job_id)
    }

    pub(in crate::features) fn finish_history_session_list(
        &mut self,
        job_id: u64,
        result: Result<Vec<AiSession>, String>,
    ) -> bool {
        if self.history.job_id != job_id {
            return false;
        }
        self.history.pending = false;
        match result {
            Ok(sessions) => {
                self.history.sessions = sessions;
                self.panel.status = "AI history loaded".to_string();
            }
            Err(error) => {
                self.history.sessions.clear();
                self.panel.status = format!("failed to load AI history: {error}");
            }
        }
        true
    }

    pub(in crate::features) fn finish_history_message_load(
        &mut self,
        job_id: u64,
        source_session_id: &str,
        target_session_id: String,
        result: Result<Vec<AiMessage>, String>,
        loaded_status: String,
    ) -> bool {
        if self.history.job_id != job_id {
            return false;
        }
        self.history.pending = false;
        if self.chat.session_id != source_session_id {
            self.panel.status = "AI session load cancelled".to_string();
            return true;
        }
        match result {
            Ok(messages) => {
                self.chat.session_id = target_session_id;
                self.chat.messages = messages;
                self.chat.streaming_assistant_id = None;
                self.history.open = false;
                self.chat.message_menu = None;
                self.chat.quoted_text = None;
                self.chat.command_cards.clear();
                if let Some(last) = self
                    .chat
                    .messages
                    .iter()
                    .rev()
                    .find(|message| matches!(message.role, AiMessageRole::Assistant))
                {
                    self.chat.response_preview = truncate_preview(&last.content, 320);
                    self.chat.command_cards = last.command_cards.clone();
                } else {
                    self.chat.response_preview.clear();
                }
                self.panel.status = loaded_status;
            }
            Err(error) => {
                self.panel.status = format!("failed to load AI session: {error}");
            }
        }
        true
    }

    pub(in crate::features) fn finish_history_session_delete(
        &mut self,
        job_id: u64,
        session_id: &str,
        result: Result<(), String>,
    ) -> Option<bool> {
        if self.history.job_id != job_id {
            return None;
        }
        self.history.pending = false;
        match result {
            Ok(()) => {
                if self.chat.session_id == session_id {
                    self.chat.messages.clear();
                    self.chat.command_cards.clear();
                    self.chat.streaming_assistant_id = None;
                    self.chat.message_menu = None;
                    self.chat.quoted_text = None;
                    self.chat.session_id = format!("ai-session-{}", uuid());
                    self.chat.response_preview = "Ask mode ready".to_string();
                }
                self.history
                    .sessions
                    .retain(|session| session.id != session_id);
                self.panel.status = "AI session deleted".to_string();
                Some(true)
            }
            Err(error) => {
                self.panel.status = format!("failed to delete AI session: {error}");
                Some(false)
            }
        }
    }

    pub(in crate::features) fn finish_history_clear(
        &mut self,
        job_id: u64,
        source_session_id: &str,
        result: Result<(), String>,
    ) -> Option<bool> {
        if self.history.job_id != job_id {
            return None;
        }
        self.history.pending = false;
        match result {
            Ok(()) => {
                self.history.sessions.clear();
                self.history.query.clear();
                if self.chat.session_id == source_session_id {
                    self.chat.messages.clear();
                    self.chat.command_cards.clear();
                    self.chat.streaming_assistant_id = None;
                    self.chat.message_menu = None;
                    self.chat.quoted_text = None;
                    self.clear_detected_error();
                    self.chat.session_id = format!("ai-session-{}", uuid());
                    self.chat.response_preview =
                        if self.settings.config.default_mode == AiMode::Agent {
                            "Agent mode ready".to_string()
                        } else {
                            "Ask mode ready".to_string()
                        };
                }
                self.panel.status = "AI history cleared".to_string();
                Some(true)
            }
            Err(error) => {
                self.panel.status = format!("failed to clear AI history: {error}");
                Some(false)
            }
        }
    }

    pub(in crate::features) fn set_history_query(&mut self, query: String) {
        self.history.query = query;
    }

    pub(in crate::features) fn begin_history_usage_count_job(&mut self) -> u64 {
        self.history.usage_count_job_id = self.history.usage_count_job_id.wrapping_add(1).max(1);
        self.history.usage_count_job_id
    }

    pub(in crate::features) fn finish_history_usage_counts(
        &mut self,
        job_id: u64,
        result: Result<(usize, usize, usize), String>,
    ) -> bool {
        if self.history.usage_count_job_id != job_id {
            return false;
        }
        let Ok((sessions, messages, audits)) = result else {
            return false;
        };
        self.history.session_count = sessions;
        self.history.message_count = messages;
        self.history.audit_count = audits;
        true
    }

    pub(in crate::features) fn discovery_is_pending(&self) -> bool {
        self.discovery.pending
    }

    pub(in crate::features) fn discovery_menu_is_open(&self) -> bool {
        self.discovery.menu_open
    }

    pub(in crate::features) fn discovery_query(&self) -> &str {
        &self.discovery.query
    }

    pub(in crate::features) fn discovery_index(&self) -> usize {
        self.discovery.index
    }

    pub(in crate::features) fn clamp_discovery_index(&mut self, len: usize) -> usize {
        if len == 0 {
            self.discovery.index = 0;
        } else {
            self.discovery.index = self.discovery.index.min(len - 1);
        }
        self.discovery.index
    }

    pub(in crate::features) fn set_discovery_index(&mut self, index: usize) {
        self.discovery.index = index;
    }

    pub(in crate::features) fn toggle_discovery_menu(&mut self, selected_index: usize) -> bool {
        self.discovery.menu_open = !self.discovery.menu_open;
        if self.discovery.menu_open {
            self.discovery.index = selected_index;
            self.history.open = false;
            self.panel.execution_menu_open = false;
            self.chat.message_menu = None;
        } else {
            self.discovery.query.clear();
            self.discovery.index = 0;
        }
        self.discovery.menu_open
    }

    pub(in crate::features) fn close_discovery_menu(&mut self) {
        self.discovery.menu_open = false;
        self.discovery.query.clear();
        self.discovery.index = 0;
    }

    pub(in crate::features) fn begin_discovery_job(
        &mut self,
    ) -> Option<mpsc::Sender<AiDiscoveryJobResult>> {
        if self.discovery.pending {
            self.panel.status = "AI model discovery already running".to_string();
            return None;
        }
        self.discovery.pending = true;
        self.panel.status = "Discovering AI models...".to_string();
        Some(self.discovery.tx.clone())
    }

    pub(in crate::features) fn drain_discovery_events(
        &mut self,
        limit: usize,
    ) -> Vec<AiDiscoveryJobResult> {
        if !self.discovery.pending {
            return Vec::new();
        }
        let mut events = Vec::new();
        for _ in 0..limit {
            let Ok(event) = self.discovery.rx.try_recv() else {
                break;
            };
            self.discovery.pending = false;
            events.push(event);
        }
        events
    }

    pub(in crate::features) fn set_discovery_query(&mut self, query: String) {
        self.discovery.query = query;
        self.discovery.index = 0;
    }

    /// Returns whether the text field must also be cleared.
    pub(in crate::features) fn escape_discovery_search(&mut self, selected_index: usize) -> bool {
        if self.discovery.query.is_empty() {
            self.discovery.menu_open = false;
            false
        } else {
            self.discovery.query.clear();
            self.discovery.index = selected_index;
            true
        }
    }

    pub(in crate::features) fn move_discovery_index(&mut self, choice_count: usize, delta: isize) {
        if choice_count == 0 {
            return;
        }
        self.discovery.index = if delta < 0 {
            (self.discovery.index + choice_count - 1) % choice_count
        } else {
            (self.discovery.index + 1) % choice_count
        };
    }

    pub(in crate::features) fn agent_steps(&self) -> &[AiAgentStepView] {
        &self.agent.steps
    }

    pub(in crate::features) fn agent_thought_is_expanded(&self, step_index: u16) -> bool {
        self.agent.thought_expanded.contains(&step_index)
    }

    pub(in crate::features) fn agent_output_is_expanded(&self, step_index: u16) -> bool {
        self.agent.output_expanded.contains(&step_index)
    }

    pub(in crate::features) fn agent_auto_confirm_is_open(&self) -> bool {
        self.agent.auto_execution_confirm_open
    }

    pub(in crate::features) fn agent_auto_confirm_focus(&self) -> &FocusHandle {
        &self.agent.auto_execution_confirm_focus
    }

    pub(in crate::features) fn request_agent_auto_confirm(&mut self) -> FocusHandle {
        self.close_transient_menus();
        self.agent.auto_execution_confirm_open = true;
        self.agent.auto_execution_confirm_focus.clone()
    }

    pub(in crate::features) fn cancel_agent_auto_confirm(&mut self) {
        self.agent.cancel_auto_execution_confirm();
    }

    pub(in crate::features) fn confirm_agent_auto_execution(&mut self) -> bool {
        if !self.agent.auto_execution_confirm_open {
            return false;
        }
        self.agent.auto_execution_confirm_open = false;
        self.settings.config.agent_command_execution_mode = AgentCommandExecutionMode::Auto;
        self.panel.status = "Agent execution mode: auto".to_string();
        true
    }

    pub(in crate::features) fn last_agent_step_index(&self) -> u16 {
        self.agent
            .steps
            .last()
            .map(|step| step.step_index)
            .unwrap_or(0)
    }

    pub(in crate::features) fn agent_loop_snapshot(&self) -> Option<AiAgentLoopState> {
        self.agent.loop_state.clone()
    }

    pub(in crate::features) fn process_agent_output(
        &mut self,
        text: &str,
    ) -> AgentCaptureProcessResult {
        self.agent.capture.process(text)
    }

    pub(in crate::features) fn reset_agent_runtime(&mut self) {
        self.agent.loop_state = None;
        self.agent.capture = AgentOutputCaptureProcessor::new();
    }

    pub(in crate::features) fn agent_capture_is_active_for(&self, session_id: &str) -> bool {
        self.agent.capture.has_active()
            && self
                .agent
                .loop_state
                .as_ref()
                .is_some_and(|state| state.terminal_session_id == session_id)
    }

    pub(in crate::features) fn panel_status(&self) -> &str {
        &self.panel.status
    }

    pub(in crate::features) fn set_panel_status(&mut self, status: impl Into<String>) {
        self.panel.status = status.into();
    }

    pub(in crate::features) fn apply_settings_input(&mut self, field: AiInputField, text: String) {
        self.panel.focused_field = field;
        match field {
            AiInputField::Model => self.settings.model_draft = text,
            AiInputField::BaseUrl => self.settings.base_url_draft = text,
            AiInputField::ApiKey => self.settings.secret_draft = text,
            AiInputField::RequestUserAgent => self.settings.config.request_user_agent = text,
        }
        self.panel.status = "AI settings edited".to_string();
    }

    pub(in crate::features) fn panel_execution_menu_is_open(&self) -> bool {
        self.panel.execution_menu_open
    }

    pub(in crate::features) fn toggle_execution_menu(&mut self) -> bool {
        self.history.open = false;
        self.history.query.clear();
        self.panel.execution_menu_open = !self.panel.execution_menu_open;
        if self.panel.execution_menu_open {
            self.chat.message_menu = None;
            self.discovery.menu_open = false;
        }
        self.panel.execution_menu_open
    }

    pub(in crate::features) fn close_execution_menu(&mut self) {
        self.panel.execution_menu_open = false;
    }

    pub(in crate::features) fn panel_detected_error(&self) -> Option<&AiDetectedErrorState> {
        self.panel.detected_error.as_ref()
    }

    pub(in crate::features) fn dismiss_detected_error(&mut self) {
        self.panel.dismiss_detected_error();
    }

    pub(in crate::features) fn clear_detected_error(&mut self) {
        self.panel.detected_error = None;
    }

    pub(in crate::features) fn note_detected_error(
        &mut self,
        session_id: String,
        output: String,
        now: Instant,
    ) -> bool {
        if self
            .panel
            .error_notice_at
            .get(&session_id)
            .is_some_and(|last| now.duration_since(*last) < std::time::Duration::from_secs(30))
        {
            return false;
        }
        self.panel.error_notice_at.insert(session_id.clone(), now);
        self.panel.detected_error = Some(AiDetectedErrorState { session_id, output });
        self.panel.status = "terminal error detected".to_string();
        true
    }

    pub(in crate::features) fn close_transient_menus(&mut self) {
        self.history.open = false;
        self.discovery.menu_open = false;
        self.panel.execution_menu_open = false;
        self.chat.message_menu = None;
    }
}

impl AiChatState {
    pub(in crate::features) fn close_message_menu(&mut self) {
        self.message_menu = None;
    }

    /// Tracks the trailing `@mention` the composer is currently completing.
    ///
    /// Only a trailing run with no whitespace counts, so the picker closes as
    /// soon as the user types past the mention. The rules are unchanged.
    pub(in crate::features) fn sync_mention_from_prompt(&mut self) {
        let Some(at_index) = self.prompt_draft.rfind('@') else {
            self.close_mention();
            return;
        };
        let query = &self.prompt_draft[at_index + 1..];
        if query.chars().any(char::is_whitespace) {
            self.close_mention();
            return;
        }
        if self.mention_query != query {
            self.mention_query = query.to_string();
            self.mention_index = 0;
        }
        self.mention_open = true;
    }

    fn close_mention(&mut self) {
        self.mention_open = false;
        self.mention_query.clear();
        self.mention_index = 0;
    }
}

impl AiHistoryState {
    fn cancel_clear_confirm(&mut self) {
        self.clear_confirm_open = false;
    }
}

impl AiAgentState {
    pub(in crate::features) fn cancel_auto_execution_confirm(&mut self) {
        self.auto_execution_confirm_open = false;
    }
}

impl AiPanelState {
    pub(in crate::features) fn dismiss_detected_error(&mut self) {
        self.detected_error = None;
        self.status = "terminal error notice dismissed".to_string();
    }
}

/// Transitions that span more than one AI concern.
impl AiFeatureState {
    pub(in crate::features) fn clear_quote(&mut self) {
        self.chat.quoted_text = None;
        self.panel.status = "AI quote cleared".to_string();
    }

    /// Resets every per-conversation concern and mints a new session id.
    ///
    /// Provider settings are deliberately untouched; the response preview is
    /// seeded from the configured default mode exactly as before.
    pub(in crate::features) fn start_new_chat(&mut self) {
        self.chat.prompt_draft.clear();
        self.chat.target_session_ids.clear();
        self.chat.message_menu = None;
        self.chat.quoted_text = None;
        self.chat.close_mention();
        self.chat.response_preview = if self.settings.config.default_mode == AiMode::Agent {
            "Agent mode ready".to_string()
        } else {
            "Ask mode ready".to_string()
        };
        self.chat.command_cards.clear();
        self.chat.messages.clear();
        self.chat.streaming_assistant_id = None;
        self.chat.prepared_request = None;
        self.chat.session_id = format!("ai-session-{}", uuid());

        self.agent.task_prompt = None;
        self.agent.step_index = 0;
        self.agent.loop_state = None;
        self.agent.capture = AgentOutputCaptureProcessor::new();
        self.agent.steps.clear();
        self.agent.thought_expanded.clear();
        self.agent.output_expanded.clear();

        self.history.open = false;
        self.history.query.clear();

        self.discovery.menu_open = false;
        self.discovery.query.clear();
        self.discovery.index = 0;

        self.panel.detected_error = None;
        self.panel.execution_menu_open = false;
        self.panel.status = "new AI chat".to_string();
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use gpui::{TestAppContext, px};
    use nyaterm_core::{
        AiAction, AiContext, AiMessage, AiMessageRole, AiMode, AiSession, AiSettings,
    };

    use crate::features::AiDiscoveryJobResult;
    use crate::models::{AiMessageMenuState, AiPreparedRequest};

    use super::{AiFeatureFocus, AiFeatureState};

    fn state(cx: &TestAppContext) -> AiFeatureState {
        let focus = cx.update(|cx| AiFeatureFocus {
            chat: cx.focus_handle(),
            clear_history_confirm: cx.focus_handle(),
            auto_execution_confirm: cx.focus_handle(),
            action: cx.focus_handle(),
            manual_model: cx.focus_handle(),
            credential: cx.focus_handle(),
        });
        AiFeatureState::new(
            AiSettings::default(),
            "model-a".to_string(),
            "https://example.invalid".to_string(),
            "session-a".to_string(),
            0,
            0,
            0,
            focus,
        )
    }

    #[test]
    fn settings_draft_restore_and_replacement_keep_related_values_together() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        state.apply_settings_input(crate::models::AiInputField::ApiKey, "secret".to_string());
        let snapshot = state.settings_draft_snapshot();

        state.apply_settings_input(crate::models::AiInputField::Model, "changed".to_string());
        assert!(!state.settings_draft_matches(&snapshot.0, &snapshot.1, &snapshot.2, &snapshot.3));

        state.restore_settings_draft(snapshot.0, snapshot.1, snapshot.2, snapshot.3);
        let restored = state.settings_draft_snapshot();
        assert_eq!(restored.1, "model-a");
        assert_eq!(restored.3, "secret");

        state.replace_settings_config(AiSettings::default(), true);
        assert!(state.settings_draft_snapshot().3.is_empty());
    }

    #[test]
    fn transient_ai_menus_are_mutually_exclusive() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);

        assert!(state.toggle_execution_menu());
        assert!(state.toggle_discovery_menu(3));
        assert!(!state.panel_execution_menu_is_open());
        assert!(state.discovery_menu_is_open());

        assert!(state.toggle_history());
        assert!(!state.discovery_menu_is_open());
        state.open_message_menu(AiMessageMenuState {
            message_id: "message".to_string(),
            text: "text".to_string(),
            x: px(1.),
            y: px(2.),
        });
        assert!(!state.history_is_open());
        assert!(state.chat_message_menu().is_some());
    }

    #[test]
    fn history_and_auto_execution_confirmations_transition_on_the_owner() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        state.history.sessions.push(AiSession {
            id: "history".to_string(),
            connection_id: None,
            title: "History".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
        });

        assert!(state.request_history_clear_confirm().is_some());
        assert!(state.history_clear_confirm_is_open());
        assert!(state.confirm_history_clear());
        assert!(!state.history_clear_confirm_is_open());

        state.request_agent_auto_confirm();
        assert!(state.agent_auto_confirm_is_open());
        assert!(state.confirm_agent_auto_execution());
        assert!(!state.agent_auto_confirm_is_open());
        assert_eq!(state.panel_status(), "Agent execution mode: auto");
    }

    #[test]
    fn history_jobs_reject_overlap_and_ignore_stale_completions() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);

        let first = state.begin_history_operation("first").unwrap();
        assert!(state.begin_history_operation("overlap").is_none());
        assert!(state.history_is_pending());
        assert_eq!(
            state.panel_status(),
            "AI history operation already in progress"
        );
        assert!(state.finish_history_session_list(first, Ok(Vec::new())));

        let second = state.begin_history_operation("second").unwrap();
        assert!(!state.finish_history_session_list(first, Ok(Vec::new())));
        assert!(state.history_is_pending());
        assert!(state.finish_history_session_list(second, Ok(Vec::new())));
        assert!(!state.history_is_pending());
    }

    #[test]
    fn history_usage_counts_ignore_superseded_jobs() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);

        let first = state.begin_history_usage_count_job();
        let second = state.begin_history_usage_count_job();
        assert!(!state.finish_history_usage_counts(first, Ok((1, 2, 3))));
        assert_eq!(
            (
                state.history.session_count,
                state.history.message_count,
                state.history.audit_count,
            ),
            (0, 0, 0)
        );
        assert!(state.finish_history_usage_counts(second, Ok((4, 5, 6))));
        assert_eq!(
            (
                state.history.session_count,
                state.history.message_count,
                state.history.audit_count,
            ),
            (4, 5, 6)
        );
    }

    #[test]
    fn history_completion_updates_history_and_chat_atomically() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        state.history.sessions = vec![AiSession {
            id: "session-a".to_string(),
            connection_id: None,
            title: "Session A".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
        }];
        state.chat.messages.push(AiMessage {
            id: "assistant-a".to_string(),
            session_id: "session-a".to_string(),
            role: AiMessageRole::Assistant,
            content: "answer".to_string(),
            created_at: String::new(),
            reasoning_content: None,
            command_cards: Vec::new(),
        });

        let delete_job = state.begin_history_operation("delete").unwrap();
        assert_eq!(
            state.finish_history_session_delete(delete_job, "session-a", Ok(())),
            Some(true)
        );
        assert!(state.history_sessions().is_empty());
        assert!(state.chat_messages().is_empty());
        assert_ne!(state.chat_session_id(), "session-a");

        state.settings.config.default_mode = AiMode::Agent;
        state.history.sessions.push(AiSession {
            id: state.chat_session_id().to_string(),
            connection_id: None,
            title: "Current".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
        });
        let source_session_id = state.chat_session_id().to_string();
        let clear_job = state.begin_history_operation("clear").unwrap();
        assert_eq!(
            state.finish_history_clear(clear_job, &source_session_id, Ok(())),
            Some(true)
        );
        assert!(state.history_sessions().is_empty());
        assert!(state.history_query().is_empty());
        assert_eq!(state.chat_response_preview(), "Agent mode ready");
        assert_eq!(state.panel_status(), "AI history cleared");
    }

    #[test]
    fn discovery_job_and_picker_lifecycles_stay_on_the_owner() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);

        let tx = state.begin_discovery_job().unwrap();
        assert!(state.begin_discovery_job().is_none());
        tx.send(AiDiscoveryJobResult {
            profile_id: "profile".to_string(),
            result: Ok(Vec::new()),
        })
        .unwrap();
        assert_eq!(state.drain_discovery_events(8).len(), 1);
        assert!(!state.discovery_is_pending());

        state.toggle_discovery_menu(2);
        state.set_discovery_query("server".to_string());
        state.move_discovery_index(3, 1);
        state.move_discovery_index(3, -1);
        assert_eq!(state.discovery_index(), 0);
        assert!(state.escape_discovery_search(2));
        assert_eq!(state.discovery_index(), 2);
        assert!(state.discovery_query().is_empty());
        assert!(!state.escape_discovery_search(1));
        assert!(!state.discovery_menu_is_open());
    }

    #[test]
    fn external_request_preparation_sets_request_status_focus_and_closes_menus() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        state.toggle_history();
        let request = AiPreparedRequest {
            action: AiAction::CustomFileAction,
            context: AiContext::default(),
            source_label: "remote file".to_string(),
        };

        state.prepare_external_request(request.clone(), "ready", "loaded", true);

        assert_eq!(state.chat_prepared_request(), Some(&request));
        assert_eq!(state.chat_response_preview(), "ready");
        assert_eq!(state.panel_status(), "loaded");
        assert!(state.chat_focus_is_pending());
        assert!(!state.history_is_open());
    }

    #[test]
    fn detected_error_throttle_and_picker_indices_are_owned_transitions() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        let now = Instant::now();

        assert!(state.note_detected_error("session".to_string(), "first".to_string(), now,));
        assert!(!state.note_detected_error(
            "session".to_string(),
            "second".to_string(),
            now + Duration::from_secs(29),
        ));
        assert!(state.note_detected_error(
            "session".to_string(),
            "third".to_string(),
            now + Duration::from_secs(30),
        ));

        state.set_discovery_index(9);
        state.set_chat_mention_index(7);
        assert_eq!(state.clamp_discovery_index(3), 2);
        assert_eq!(state.clamp_chat_mention_index(2), 1);
        assert_eq!(state.clamp_discovery_index(0), 0);
        assert_eq!(state.clamp_chat_mention_index(0), 0);
    }

    #[test]
    fn panel_status_and_error_banner_change_only_through_owner_operations() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        let now = Instant::now();

        state.set_panel_status("completed");
        assert_eq!(state.panel_status(), "completed");
        state.set_panel_status("replacement");
        assert_eq!(state.panel_status(), "replacement");

        assert!(state.note_detected_error("session".to_string(), "failure".to_string(), now,));
        assert!(state.panel_detected_error().is_some());
        state.clear_detected_error();
        assert!(state.panel_detected_error().is_none());
        assert_eq!(state.panel_status(), "terminal error detected");
    }
}
