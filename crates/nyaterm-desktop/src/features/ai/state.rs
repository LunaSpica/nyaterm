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
    AgentOutputCaptureProcessor, AiCommandCard, AiMessage, AiMode, AiSession, AiSettings, uuid,
};

use crate::features::{AiAgentLoopState, AiAgentStepView, AiChatWorkerEvent, AiDiscoveryJobResult};
use crate::models::{
    AiActionEditorField, AiActionListKind, AiCredentialEditorField, AiDetectedErrorState,
    AiInputField, AiMessageMenuState, AiPreparedRequest,
};

pub(in crate::features) struct AiFeatureState {
    pub settings: AiSettingsState,
    pub chat: AiChatState,
    pub history: AiHistoryState,
    pub discovery: AiDiscoveryState,
    pub agent: AiAgentState,
    pub panel: AiPanelState,
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
pub(in crate::features) struct AiSettingsState {
    pub config: AiSettings,
    pub model_draft: String,
    pub base_url_draft: String,
    pub secret_draft: String,
    pub model_collapsed_groups: HashSet<String>,
    pub model_query: String,
    pub manual_model_drafts: HashMap<String, String>,
    pub manual_model_focus: FocusHandle,
    pub manual_model_edit_group: Option<String>,
    /// Per-credential API-key drafts; empty means keep the stored secret.
    pub credential_secret_drafts: HashMap<String, String>,
    pub credential_edit: Option<(String, AiCredentialEditorField)>,
    pub credential_focus: FocusHandle,
    pub action_edit: Option<(AiActionListKind, String, AiActionEditorField)>,
    pub action_focus: FocusHandle,
}

/// Composer, in-flight request and the visible transcript.
pub(in crate::features) struct AiChatState {
    pub tx: mpsc::Sender<AiChatWorkerEvent>,
    pub rx: mpsc::Receiver<AiChatWorkerEvent>,
    pub pending: bool,
    pub job_id: u64,
    pub cancel: Option<Arc<AtomicBool>>,
    pub session_id: String,
    pub prompt_draft: String,
    pub target_session_ids: Vec<String>,
    pub mention_open: bool,
    pub mention_query: String,
    pub mention_index: usize,
    pub prepared_request: Option<AiPreparedRequest>,
    pub response_preview: String,
    pub messages: Vec<AiMessage>,
    pub streaming_assistant_id: Option<String>,
    pub message_menu: Option<AiMessageMenuState>,
    pub quoted_text: Option<String>,
    pub command_cards: Vec<AiCommandCard>,
    pub focus: FocusHandle,
    pub focus_pending: bool,
}

/// Stored sessions, the history browser and the counters shown beside it.
pub(in crate::features) struct AiHistoryState {
    pub open: bool,
    pub query: String,
    pub job_id: u64,
    pub pending: bool,
    pub sessions: Vec<AiSession>,
    pub session_count: usize,
    pub message_count: usize,
    pub audit_count: usize,
    pub usage_count_job_id: u64,
    pub audit_write_lock: Arc<Mutex<()>>,
    pub clear_confirm_open: bool,
    pub clear_confirm_focus: FocusHandle,
}

/// Model discovery job and the model picker it feeds.
pub(in crate::features) struct AiDiscoveryState {
    pub tx: mpsc::Sender<AiDiscoveryJobResult>,
    pub rx: mpsc::Receiver<AiDiscoveryJobResult>,
    pub pending: bool,
    pub menu_open: bool,
    pub query: String,
    pub index: usize,
}

/// Agent loop: the running task, its steps and their disclosure state.
pub(in crate::features) struct AiAgentState {
    pub task_prompt: Option<String>,
    pub step_index: u16,
    pub loop_state: Option<AiAgentLoopState>,
    pub capture: AgentOutputCaptureProcessor,
    pub steps: Vec<AiAgentStepView>,
    pub thought_expanded: HashSet<u16>,
    pub output_expanded: HashSet<u16>,
    pub auto_execution_confirm_open: bool,
    pub auto_execution_confirm_focus: FocusHandle,
}

/// Panel chrome: status line, focus routing and the detected-error banner.
pub(in crate::features) struct AiPanelState {
    pub execution_menu_open: bool,
    pub status: String,
    pub focused_field: AiInputField,
    pub detected_error: Option<AiDetectedErrorState>,
    pub error_notice_at: HashMap<String, Instant>,
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
    pub(in crate::features) fn cancel_clear_confirm(&mut self) {
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
