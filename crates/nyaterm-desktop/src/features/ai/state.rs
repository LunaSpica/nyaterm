//! Grouped AI feature state.
//!
//! The AI panel spans several independent concerns: provider settings, the
//! chat composer and transcript, session history, model discovery, and the
//! agent loop. They were seventy `ai_*` fields on `NyaTermApp`, which made it
//! impossible to see which ones move together.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use gpui::FocusHandle;
use nyaterm_core::{
    AgentCaptureProcessResult, AgentCommandExecutionMode, AgentOutputCaptureProcessor,
    AiCommandCard, AiCustomActionConfig, AiMessage, AiMessageRole, AiMode, AiModelConfigItem,
    AiModelDiscovery, AiModelSource, AiProviderCredential, AiProviderKind, AiSession, AiSettings,
    RiskLevel, ai_model_id_for_credential, ai_model_id_for_provider, merge_model_discoveries,
    now_rfc3339, truncate_preview, uuid,
};

use crate::features::{
    AiAgentLoopState, AiAgentStepStatus, AiAgentStepView, AiChatJobOutput, AiChatWorkerEvent,
    AiDiscoveryJobResult,
};
use crate::models::{
    AiActionEditorField, AiActionListKind, AiCredentialEditorField, AiDetectedErrorState,
    AiInputField, AiMessageMenuState, AiPreparedRequest,
};

pub(in crate::features) struct AiFeatureState {
    settings: AiSettingsState,
    chat: AiChatState,
    history: AiHistoryState,
    discovery: AiDiscoveryState,
    agent: AiAgentState,
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
struct AiSettingsState {
    config: AiSettings,
    model_draft: String,
    base_url_draft: String,
    secret_draft: String,
    model_collapsed_groups: HashSet<String>,
    model_query: String,
    manual_model_drafts: HashMap<String, String>,
    manual_model_focus: FocusHandle,
    manual_model_edit_group: Option<String>,
    /// Per-credential API-key drafts; empty means keep the stored secret.
    credential_secret_drafts: HashMap<String, String>,
    credential_edit: Option<(String, AiCredentialEditorField)>,
    credential_focus: FocusHandle,
    action_edit: Option<(AiActionListKind, String, AiActionEditorField)>,
    action_focus: FocusHandle,
}

/// Composer, in-flight request and the visible transcript.
struct AiChatState {
    tx: mpsc::Sender<AiChatWorkerEvent>,
    rx: mpsc::Receiver<AiChatWorkerEvent>,
    pending: bool,
    job_id: u64,
    cancel: Option<Arc<AtomicBool>>,
    session_id: String,
    prompt_draft: String,
    target_session_ids: Vec<String>,
    mention_open: bool,
    mention_query: String,
    mention_index: usize,
    prepared_request: Option<AiPreparedRequest>,
    response_preview: String,
    messages: Vec<AiMessage>,
    streaming_assistant_id: Option<String>,
    message_menu: Option<AiMessageMenuState>,
    quoted_text: Option<String>,
    command_cards: Vec<AiCommandCard>,
    focus: FocusHandle,
    focus_pending: bool,
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
struct AiAgentState {
    task_prompt: Option<String>,
    step_index: u16,
    loop_state: Option<AiAgentLoopState>,
    capture: AgentOutputCaptureProcessor,
    steps: Vec<AiAgentStepView>,
    thought_expanded: HashSet<u16>,
    output_expanded: HashSet<u16>,
    auto_execution_confirm_open: bool,
    auto_execution_confirm_focus: FocusHandle,
}

pub(in crate::features) struct AiChatLaunch {
    pub(in crate::features) job_id: u64,
    pub(in crate::features) cancel: Arc<AtomicBool>,
    pub(in crate::features) tx: mpsc::Sender<AiChatWorkerEvent>,
    pub(in crate::features) session_id: String,
}

pub(in crate::features) struct AiChatFinishEffect {
    pub(in crate::features) session_id: String,
    pub(in crate::features) succeeded: bool,
    pub(in crate::features) clear_prompt_input: bool,
    pub(in crate::features) refresh_usage_counts: bool,
    pub(in crate::features) auto_execute_first: bool,
}

pub(in crate::features) enum AiAgentBackgroundEffect {
    Ignored,
    MatchedStale,
    Continue(Box<AiAgentLoopState>, nyaterm_core::CommandObservation),
    Failed,
}

pub(in crate::features) enum AiAgentObservationPoll {
    Waiting,
    Target(AiAgentLoopState),
    TimedOut(AiAgentLoopState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::features) enum AiSettingsMutation {
    Ignored,
    Notify,
    Persist,
}

/// Panel chrome: status line, focus routing and the detected-error banner.
struct AiPanelState {
    execution_menu_open: bool,
    status: String,
    focused_field: AiInputField,
    detected_error: Option<AiDetectedErrorState>,
    error_notice_at: HashMap<String, Instant>,
}

fn is_builtin_ai_provider_id(id: &str) -> bool {
    matches!(
        id,
        "openai"
            | "anthropic"
            | "gemini"
            | "deepseek"
            | "ollama"
            | "xai"
            | "cohere"
            | "mimo"
            | "zai"
            | "groq"
    )
}

fn seed_builtin_ai_models_for_provider(settings: &mut AiSettings, provider_kind: &AiProviderKind) {
    let names: &[&str] = match provider_kind {
        AiProviderKind::Openai => &[
            "gpt-4o-mini",
            "gpt-4o",
            "gpt-4.1",
            "gpt-4.1-mini",
            "o3-mini",
            "o4-mini",
        ],
        AiProviderKind::Anthropic => &[
            "claude-3-haiku-20240307",
            "claude-3-5-sonnet-20241022",
            "claude-sonnet-4-20250514",
        ],
        AiProviderKind::Gemini => &["gemini-2.0-flash", "gemini-1.5-pro"],
        AiProviderKind::Deepseek => &["deepseek-chat", "deepseek-reasoner"],
        AiProviderKind::Ollama => &["llama3", "llama3.1", "qwen2.5"],
        AiProviderKind::Xai => &["grok-3", "grok-2"],
        AiProviderKind::Cohere => &["command-a-03-2025", "command-r-plus"],
        AiProviderKind::Mimo => &["mimo-v2.5-pro"],
        AiProviderKind::Zai => &["glm-4", "glm-4-flash"],
        AiProviderKind::Groq => &["llama-3.3-70b-versatile"],
        AiProviderKind::OpenaiCompatible => &[],
    };
    let existing: HashSet<String> = settings
        .models
        .iter()
        .map(|model| model.id.clone())
        .collect();
    for name in names {
        let model_id = ai_model_id_for_provider(provider_kind, name);
        if existing.contains(&model_id) {
            continue;
        }
        settings.models.push(AiModelConfigItem {
            id: model_id,
            name: (*name).to_string(),
            provider_kind: Some(provider_kind.clone()),
            credential_id: None,
            enabled: false,
            source: AiModelSource::RustGenai,
            last_seen_at: None,
        });
    }
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

    pub(in crate::features) fn settings_config_cloned(&self) -> AiSettings {
        self.settings.config.clone()
    }

    pub(in crate::features) fn settings_enabled(&self) -> bool {
        self.settings.config.enabled
    }

    pub(in crate::features) fn settings_max_agent_steps(&self) -> u16 {
        self.settings.config.max_agent_steps.unwrap_or(10).max(1)
    }

    pub(in crate::features) fn settings_context_line_limit(&self) -> usize {
        self.settings.config.context_line_limit as usize
    }

    pub(in crate::features) fn sync_settings_active_profile_drafts(
        &mut self,
        model: String,
        base_url: String,
    ) {
        self.settings.model_draft = model;
        self.settings.base_url_draft = base_url;
        self.settings.secret_draft.clear();
    }

    pub(in crate::features) fn pending_settings(&self) -> AiSettings {
        let mut next = self.settings.config.clone();
        let active_id = next.active_profile_id.clone();
        let mut active_kind = None;
        let mut active_name = active_id.clone();
        let mut active_base_url = (!self.settings.base_url_draft.trim().is_empty())
            .then(|| self.settings.base_url_draft.trim().to_string());
        let active_model = self.settings.model_draft.trim().to_string();

        if let Some(profile) = next
            .provider_profiles
            .iter_mut()
            .find(|profile| profile.id == active_id)
        {
            profile.enabled = true;
            if !active_model.is_empty() {
                profile.model = active_model.clone();
            }
            profile.base_url = active_base_url.clone();
            if !self.settings.secret_draft.is_empty() {
                profile.api_key = Some(self.settings.secret_draft.clone());
            }
            active_kind = Some(profile.provider_kind.clone());
            active_name = profile.name.clone();
            active_base_url = profile.base_url.clone();
        }

        if let Some(kind) = active_kind.clone() {
            let credential = AiProviderCredential {
                id: active_id.clone(),
                name: active_name,
                provider_kind: kind.clone(),
                base_url: active_base_url.clone(),
                api_key: if self.settings.secret_draft.is_empty() {
                    next.provider_credentials
                        .iter()
                        .find(|credential| credential.id == active_id)
                        .and_then(|credential| credential.api_key.clone())
                } else {
                    Some(self.settings.secret_draft.clone())
                },
                enabled: true,
            };
            if let Some(existing) = next
                .provider_credentials
                .iter_mut()
                .find(|credential| credential.id == active_id)
            {
                *existing = credential;
            } else {
                next.provider_credentials.push(credential);
            }

            if !active_model.is_empty() {
                let model_id = if active_base_url
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                    || kind == AiProviderKind::OpenaiCompatible
                {
                    ai_model_id_for_credential(&active_id, &active_model)
                } else {
                    ai_model_id_for_provider(&kind, &active_model)
                };
                let model_index = next
                    .models
                    .iter()
                    .position(|model| model.credential_id.as_deref() == Some(active_id.as_str()))
                    .or_else(|| next.models.iter().position(|model| model.id == model_id));
                if let Some(model_index) = model_index {
                    let model = &mut next.models[model_index];
                    model.id = model_id.clone();
                    model.name = active_model.clone();
                    model.provider_kind = Some(kind);
                    model.credential_id = active_base_url
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty())
                        .then(|| active_id.clone());
                    model.enabled = true;
                } else {
                    next.models.push(AiModelConfigItem {
                        id: model_id.clone(),
                        name: active_model,
                        provider_kind: Some(kind),
                        credential_id: active_base_url
                            .as_deref()
                            .is_some_and(|value| !value.trim().is_empty())
                            .then(|| active_id.clone()),
                        enabled: true,
                        source: AiModelSource::Manual,
                        last_seen_at: None,
                    });
                }
                next.default_model_id = Some(model_id);
            }
        }
        next
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

    pub(in crate::features) fn accept_saved_settings(&mut self, saved: AiSettings) {
        self.settings.config = saved;
    }

    pub(in crate::features) fn toggle_settings_enabled(&mut self) {
        self.settings.config.enabled = !self.settings.config.enabled;
        self.panel.status = if self.settings.config.enabled {
            "AI enabled"
        } else {
            "AI disabled"
        }
        .to_string();
    }

    pub(in crate::features) fn set_settings_mode(&mut self, mode: AiMode) {
        self.settings.config.default_mode = mode;
        self.panel.status = "AI mode updated".to_string();
    }

    pub(in crate::features) fn set_settings_command_mode(
        &mut self,
        mode: AgentCommandExecutionMode,
    ) {
        self.settings.config.agent_command_execution_mode = mode;
        self.panel.status = "Agent command policy updated".to_string();
    }

    pub(in crate::features) fn toggle_settings_background_execution(&mut self) {
        self.settings.config.agent_background_execution_enabled =
            !self.settings.config.agent_background_execution_enabled;
        self.panel.status = if self.settings.config.agent_background_execution_enabled {
            "Agent background execution enabled"
        } else {
            "Agent background execution disabled"
        }
        .to_string();
    }

    pub(in crate::features) fn toggle_settings_redaction(&mut self) {
        self.settings.config.redaction_enabled = !self.settings.config.redaction_enabled;
        self.panel.status = "AI redaction updated".to_string();
    }

    pub(in crate::features) fn toggle_settings_allow_save_command(&mut self) {
        self.settings.config.allow_save_command = !self.settings.config.allow_save_command;
        self.panel.status = "AI command saving updated".to_string();
    }

    pub(in crate::features) fn toggle_settings_record_history(&mut self) {
        self.settings.config.record_history = !self.settings.config.record_history;
        self.panel.status = "AI history recording updated".to_string();
    }

    pub(in crate::features) fn adjust_settings_context_line_limit(&mut self, delta: i32) {
        let current = self.settings.config.context_line_limit as i32;
        self.settings.config.context_line_limit = (current + delta).clamp(50, 500) as u32;
        self.panel.status = "AI context line limit updated".to_string();
    }

    pub(in crate::features) fn adjust_settings_timeout_ms(&mut self, delta: i64) {
        let current = self.settings.config.timeout_ms as i64;
        self.settings.config.timeout_ms = (current + delta).clamp(5_000, 300_000) as u64;
        self.panel.status = "AI timeout updated".to_string();
    }

    pub(in crate::features) fn adjust_settings_agent_steps(&mut self, delta: i16) {
        let current = self.settings.config.max_agent_steps.unwrap_or(10) as i16;
        self.settings.config.max_agent_steps = Some((current + delta).clamp(1, 50) as u16);
        self.panel.status = "AI Agent max steps updated".to_string();
    }

    pub(in crate::features) fn adjust_settings_agent_step_timeout_ms(&mut self, delta: i64) {
        let current = self.settings.config.agent_step_timeout_ms.unwrap_or(30_000) as i64;
        self.settings.config.agent_step_timeout_ms =
            Some((current + delta).clamp(5_000, 120_000) as u64);
        self.panel.status = "AI Agent step timeout updated".to_string();
    }

    pub(in crate::features) fn adjust_settings_terminal_output_lines(&mut self, delta: i16) {
        let current = self.settings.config.terminal_output_lines as i16;
        self.settings.config.terminal_output_lines = (current + delta).clamp(0, 100) as u16;
        self.panel.status = "AI terminal output lines updated".to_string();
    }

    pub(in crate::features) fn adjust_settings_file_size_mb(&mut self, delta: i64) {
        let mb = 1024 * 1024;
        let current = (self.settings.config.max_ai_file_size_bytes / mb).max(1) as i64;
        self.settings.config.max_ai_file_size_bytes = (current + delta).clamp(1, 256) as u64 * mb;
        self.panel.status = "AI file size limit updated".to_string();
    }

    pub(in crate::features) fn set_settings_smart_auto_execute_max_risk(
        &mut self,
        risk: RiskLevel,
    ) {
        self.settings.config.agent_smart_auto_execute_max_risk = risk;
        self.panel.status = "AI smart auto-execute risk updated".to_string();
    }

    fn select_first_enabled_model(&mut self) {
        self.settings.config.default_model_id = self
            .settings
            .config
            .models
            .iter()
            .find(|model| model.enabled)
            .map(|model| model.id.clone());
    }

    fn default_model_is_enabled(&self) -> bool {
        self.settings
            .config
            .default_model_id
            .as_ref()
            .is_some_and(|id| {
                self.settings
                    .config
                    .models
                    .iter()
                    .any(|model| model.enabled && model.id == *id)
            })
    }

    fn configured_default_model_is_disabled(&self) -> bool {
        self.settings.config.default_model_id.is_some() && !self.default_model_is_enabled()
    }

    pub(in crate::features) fn toggle_settings_model_enabled(&mut self, model_id: &str) {
        if let Some(model) = self
            .settings
            .config
            .models
            .iter_mut()
            .find(|model| model.id == model_id)
        {
            model.enabled = !model.enabled;
            self.panel.status = "AI model list updated".to_string();
        }
        if !self.default_model_is_enabled() {
            self.select_first_enabled_model();
        }
    }

    pub(in crate::features) fn set_settings_model_query(&mut self, text: String) {
        self.settings.model_query = text;
    }

    pub(in crate::features) fn set_settings_default_model(&mut self, model_id: &str) {
        if let Some(model) = self
            .settings
            .config
            .models
            .iter_mut()
            .find(|model| model.id == model_id)
        {
            model.enabled = true;
            self.settings.config.default_model_id = Some(model.id.clone());
            self.panel.status = "AI default model updated".to_string();
        }
    }

    pub(in crate::features) fn remove_settings_manual_model(
        &mut self,
        model_id: &str,
    ) -> AiSettingsMutation {
        let Some(model) = self
            .settings
            .config
            .models
            .iter()
            .find(|model| model.id == model_id)
            .cloned()
        else {
            return AiSettingsMutation::Ignored;
        };
        if model.source != AiModelSource::Manual {
            self.panel.status = "Only manual models can be deleted".to_string();
            return AiSettingsMutation::Notify;
        }
        self.settings
            .config
            .models
            .retain(|item| item.id != model_id);
        if self.settings.config.default_model_id.as_deref() == Some(model_id) {
            self.select_first_enabled_model();
        }
        self.panel.status = format!("Deleted manual model {}", model.name);
        AiSettingsMutation::Persist
    }

    pub(in crate::features) fn add_settings_manual_model(
        &mut self,
        credential_id: &str,
        name: &str,
    ) -> AiSettingsMutation {
        let name = name.trim().to_string();
        if name.is_empty() {
            self.panel.status = "Manual model name is required".to_string();
            return AiSettingsMutation::Notify;
        }
        let Some(credential) = self
            .settings
            .config
            .provider_credentials
            .iter()
            .find(|credential| credential.id == credential_id)
            .cloned()
        else {
            self.panel.status = "Credential not found".to_string();
            return AiSettingsMutation::Notify;
        };
        let builtin = is_builtin_ai_provider_id(&credential.id);
        let model_id = if builtin {
            ai_model_id_for_provider(&credential.provider_kind, &name)
        } else {
            ai_model_id_for_credential(&credential.id, &name)
        };
        if let Some(existing) = self
            .settings
            .config
            .models
            .iter_mut()
            .find(|model| model.id == model_id)
        {
            existing.enabled = true;
            existing.name = name.clone();
            existing.provider_kind = Some(credential.provider_kind.clone());
            existing.credential_id = (!builtin).then(|| credential.id.clone());
            self.settings.config.default_model_id = Some(model_id);
            self.panel.status = format!("Enabled model {name}");
            return AiSettingsMutation::Persist;
        }
        self.settings.config.models.insert(
            0,
            AiModelConfigItem {
                id: model_id.clone(),
                name: name.clone(),
                provider_kind: Some(credential.provider_kind),
                credential_id: (!builtin).then_some(credential.id),
                enabled: true,
                source: AiModelSource::Manual,
                last_seen_at: None,
            },
        );
        if !self.default_model_is_enabled() {
            self.settings.config.default_model_id = Some(model_id);
        }
        self.panel.status = format!("Added manual model {name}");
        AiSettingsMutation::Persist
    }

    pub(in crate::features) fn toggle_settings_model_group(&mut self, group_key: String) {
        if !self.settings.model_collapsed_groups.remove(&group_key) {
            self.settings.model_collapsed_groups.insert(group_key);
        }
    }

    pub(in crate::features) fn begin_settings_manual_model_edit(&mut self, group_key: &str) {
        self.settings.manual_model_edit_group = Some(group_key.to_string());
    }

    pub(in crate::features) fn cancel_settings_manual_model_edit(&mut self) -> FocusHandle {
        self.settings.manual_model_edit_group = None;
        self.settings.manual_model_focus.clone()
    }

    pub(in crate::features) fn settings_manual_model_submission(
        &self,
        group_key: &str,
    ) -> Option<(String, String)> {
        let credential_id = self
            .settings
            .config
            .provider_credentials
            .iter()
            .find(|credential| credential.id == group_key)?
            .id
            .clone();
        let draft = self
            .settings
            .manual_model_drafts
            .get(group_key)
            .cloned()
            .unwrap_or_default();
        Some((credential_id, draft))
    }

    pub(in crate::features) fn apply_settings_manual_model_input(
        &mut self,
        group_key: &str,
        text: String,
    ) -> bool {
        if !self
            .settings
            .config
            .provider_credentials
            .iter()
            .any(|credential| credential.id == group_key)
        {
            return false;
        }
        self.settings
            .manual_model_drafts
            .insert(group_key.to_string(), text);
        self.settings.manual_model_edit_group = Some(group_key.to_string());
        true
    }

    pub(in crate::features) fn settings_manual_model_draft(&self, group_key: &str) -> String {
        self.settings
            .manual_model_drafts
            .get(group_key)
            .cloned()
            .unwrap_or_default()
    }

    pub(in crate::features) fn focus_settings_manual_model_edit(&mut self, group_key: String) {
        self.settings.manual_model_edit_group = Some(group_key);
    }

    pub(in crate::features) fn clear_settings_manual_model_draft(&mut self, group_key: &str) {
        self.settings
            .manual_model_drafts
            .insert(group_key.to_string(), String::new());
    }

    pub(in crate::features) fn toggle_settings_credential_enabled(
        &mut self,
        credential_id: &str,
    ) -> AiSettingsMutation {
        let Some(index) = self
            .settings
            .config
            .provider_credentials
            .iter()
            .position(|credential| credential.id == credential_id)
        else {
            return AiSettingsMutation::Ignored;
        };
        let enabled = !self.settings.config.provider_credentials[index].enabled;
        let name = self.settings.config.provider_credentials[index]
            .name
            .clone();
        let provider_kind = self.settings.config.provider_credentials[index]
            .provider_kind
            .clone();
        self.settings.config.provider_credentials[index].enabled = enabled;
        if let Some(profile) = self
            .settings
            .config
            .provider_profiles
            .iter_mut()
            .find(|profile| profile.id == credential_id)
        {
            profile.enabled = enabled;
        }
        if is_builtin_ai_provider_id(credential_id) {
            if enabled {
                seed_builtin_ai_models_for_provider(&mut self.settings.config, &provider_kind);
            } else {
                self.settings.config.models.retain(|model| {
                    model.provider_kind.as_ref() != Some(&provider_kind)
                        || model.credential_id.is_some()
                });
                if self.configured_default_model_is_disabled() {
                    self.select_first_enabled_model();
                }
            }
        }
        self.panel.status = format!(
            "AI credential {name} {}",
            if enabled { "enabled" } else { "disabled" }
        );
        AiSettingsMutation::Persist
    }

    pub(in crate::features) fn apply_settings_credential_input(
        &mut self,
        rest: &str,
        text: String,
    ) -> bool {
        let Some((credential_id, field)) = rest.rsplit_once('.') else {
            return false;
        };
        match field {
            "api-key" => {
                self.settings
                    .credential_secret_drafts
                    .insert(credential_id.to_string(), text);
            }
            "name" | "base-url" => {
                let Some(credential) = self
                    .settings
                    .config
                    .provider_credentials
                    .iter_mut()
                    .find(|credential| credential.id == credential_id)
                else {
                    return false;
                };
                if field == "name" {
                    credential.name = text;
                } else {
                    credential.base_url = (!text.trim().is_empty()).then_some(text);
                }
            }
            _ => return false,
        }
        self.settings.credential_edit = None;
        self.panel.status = "AI credential edited".to_string();
        true
    }

    pub(in crate::features) fn commit_settings_credential_edits(&mut self, credential_id: &str) {
        let secret_draft = self
            .settings
            .credential_secret_drafts
            .get(credential_id)
            .cloned()
            .unwrap_or_default();
        if let Some(credential) = self
            .settings
            .config
            .provider_credentials
            .iter_mut()
            .find(|credential| credential.id == credential_id)
        {
            if !secret_draft.is_empty() {
                credential.api_key = Some(secret_draft.clone());
            }
            let name = credential.name.clone();
            let base_url = credential.base_url.clone();
            let api_key = credential.api_key.clone();
            let enabled = credential.enabled;
            if let Some(profile) = self
                .settings
                .config
                .provider_profiles
                .iter_mut()
                .find(|profile| profile.id == credential_id)
            {
                profile.name = name;
                profile.base_url = base_url;
                if !secret_draft.is_empty() {
                    profile.api_key = Some(secret_draft);
                } else if api_key.is_some() {
                    // Preserve the stored masked/encrypted key through merge_masked.
                }
                profile.enabled = enabled;
            }
        }
        self.settings.credential_secret_drafts.remove(credential_id);
        self.panel.status = "AI credential saved".to_string();
    }

    pub(in crate::features) fn add_settings_credential(&mut self, id: String) -> FocusHandle {
        self.settings.config.provider_credentials.insert(
            0,
            AiProviderCredential {
                id: id.clone(),
                name: String::new(),
                provider_kind: AiProviderKind::OpenaiCompatible,
                base_url: Some(String::new()),
                api_key: None,
                enabled: true,
            },
        );
        self.settings.credential_edit = Some((id, AiCredentialEditorField::Name));
        self.panel.status = "AI credential added".to_string();
        self.settings.credential_focus.clone()
    }

    pub(in crate::features) fn remove_settings_credential(
        &mut self,
        credential_id: &str,
    ) -> AiSettingsMutation {
        if is_builtin_ai_provider_id(credential_id) {
            self.panel.status = "Built-in AI credentials cannot be deleted".to_string();
            return AiSettingsMutation::Notify;
        }
        self.settings
            .config
            .provider_credentials
            .retain(|credential| credential.id != credential_id);
        self.settings
            .config
            .models
            .retain(|model| model.credential_id.as_deref() != Some(credential_id));
        if self.configured_default_model_is_disabled() {
            self.select_first_enabled_model();
        }
        self.settings.credential_edit = None;
        self.settings.credential_secret_drafts.remove(credential_id);
        self.panel.status = "AI credential removed".to_string();
        AiSettingsMutation::Persist
    }

    pub(in crate::features) fn settings_action_value(
        &self,
        kind: AiActionListKind,
        action_id: &str,
        field: AiActionEditorField,
    ) -> String {
        self.settings
            .action(kind, action_id)
            .map(|action| match field {
                AiActionEditorField::Name => action.name.clone(),
                AiActionEditorField::Prompt => action.prompt.clone(),
            })
            .unwrap_or_default()
    }

    pub(in crate::features) fn focus_settings_action(
        &mut self,
        kind: AiActionListKind,
        action_id: String,
        field: AiActionEditorField,
    ) {
        self.settings.action_edit = Some((kind, action_id, field));
    }

    pub(in crate::features) fn toggle_settings_action_enabled(
        &mut self,
        kind: AiActionListKind,
        action_id: &str,
    ) -> bool {
        let Some(action) = self.settings.action_mut(kind, action_id) else {
            return false;
        };
        action.enabled = !action.enabled;
        self.panel.status = "AI action toggled".to_string();
        true
    }

    pub(in crate::features) fn add_settings_action(&mut self, kind: AiActionListKind, id: String) {
        self.settings.actions_mut(kind).push(AiCustomActionConfig {
            id: id.clone(),
            name: "Custom AI action".to_string(),
            prompt: String::new(),
            enabled: true,
        });
        self.settings.action_edit = Some((kind, id, AiActionEditorField::Name));
        self.panel.status = "AI action added".to_string();
    }

    pub(in crate::features) fn remove_settings_action(
        &mut self,
        kind: AiActionListKind,
        action_id: &str,
    ) {
        self.settings
            .actions_mut(kind)
            .retain(|action| action.id != action_id);
        if self
            .settings
            .action_edit
            .as_ref()
            .is_some_and(|(edit_kind, id, _)| *edit_kind == kind && id == action_id)
        {
            self.settings.action_edit = None;
        }
        self.panel.status = "AI action removed".to_string();
    }

    pub(in crate::features) fn settings_action_edit(
        &self,
    ) -> Option<(AiActionListKind, String, AiActionEditorField)> {
        self.settings.action_edit.clone()
    }

    pub(in crate::features) fn cancel_settings_action_edit(&mut self) -> FocusHandle {
        self.settings.action_edit = None;
        self.settings.action_focus.clone()
    }

    pub(in crate::features) fn apply_settings_action_input(
        &mut self,
        kind: AiActionListKind,
        action_id: &str,
        field: AiActionEditorField,
        text: String,
    ) -> bool {
        let Some(action) = self.settings.action_mut(kind, action_id) else {
            return false;
        };
        match field {
            AiActionEditorField::Name => action.name = text,
            AiActionEditorField::Prompt => action.prompt = text,
        }
        self.settings.action_edit = Some((kind, action_id.to_string(), field));
        true
    }

    pub(in crate::features) fn discovery_settings(
        &self,
    ) -> (AiSettings, Vec<AiProviderCredential>) {
        let credentials = self
            .settings
            .config
            .provider_credentials
            .iter()
            .filter(|credential| {
                credential.enabled
                    && credential.provider_kind == AiProviderKind::OpenaiCompatible
                    && credential
                        .base_url
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty())
            })
            .cloned()
            .collect();
        (self.settings.config.clone(), credentials)
    }

    pub(in crate::features) fn apply_settings_model_discoveries(
        &mut self,
        discoveries: Vec<AiModelDiscovery>,
    ) -> usize {
        let discoveries = merge_model_discoveries(discoveries);
        let last_seen_at = Some(now_rfc3339());
        for discovery in &discoveries {
            if let Some(model) = self
                .settings
                .config
                .models
                .iter_mut()
                .find(|model| model.id == discovery.id)
            {
                model.name = discovery.name.clone();
                model.provider_kind = discovery.provider_kind.clone();
                model.credential_id = discovery.credential_id.clone();
                model.source = discovery.source.clone();
                model.last_seen_at = last_seen_at.clone();
            } else {
                self.settings.config.models.push(AiModelConfigItem {
                    id: discovery.id.clone(),
                    name: discovery.name.clone(),
                    provider_kind: discovery.provider_kind.clone(),
                    credential_id: discovery.credential_id.clone(),
                    enabled: false,
                    source: discovery.source.clone(),
                    last_seen_at: last_seen_at.clone(),
                });
            }
        }
        discoveries.len()
    }

    pub(in crate::features) fn chat_or_agent_is_running(&self) -> bool {
        self.chat.pending || self.agent.loop_state.is_some()
    }

    pub(in crate::features) fn chat_is_pending(&self) -> bool {
        self.chat.pending
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

    pub(in crate::features) fn chat_request_prompt(&self) -> Option<String> {
        let prompt = self.chat.prompt_draft.trim();
        if prompt.is_empty() {
            return None;
        }
        Some(
            self.chat
                .quoted_text
                .as_deref()
                .map(str::trim)
                .filter(|quoted| !quoted.is_empty())
                .map(|quoted| format!("> {quoted}\n\n{prompt}"))
                .unwrap_or_else(|| prompt.to_string()),
        )
    }

    pub(in crate::features) fn reject_chat_start(
        &mut self,
        message: impl Into<String>,
        update_panel: bool,
    ) {
        self.chat.response_preview = message.into();
        if update_panel {
            self.panel.status = self.chat.response_preview.clone();
        }
    }

    pub(in crate::features) fn chat_prepared_request_cloned(&self) -> Option<AiPreparedRequest> {
        self.chat.prepared_request.clone()
    }

    pub(in crate::features) fn chat_target_session_ids(&self) -> &[String] {
        &self.chat.target_session_ids
    }

    pub(in crate::features) fn chat_mention_query(&self) -> &str {
        &self.chat.mention_query
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

    pub(in crate::features) fn close_chat_mention(&mut self) {
        self.chat.close_mention();
    }

    pub(in crate::features) fn hide_chat_mention(&mut self) {
        self.chat.mention_open = false;
        self.chat.mention_query.clear();
    }

    pub(in crate::features) fn move_chat_mention_index(
        &mut self,
        candidate_count: usize,
        delta: isize,
    ) {
        if candidate_count == 0 {
            return;
        }
        self.chat.mention_index = if delta < 0 {
            (self.chat.mention_index + candidate_count - 1) % candidate_count
        } else {
            (self.chat.mention_index + 1) % candidate_count
        };
    }

    pub(in crate::features) fn set_chat_prompt_draft(&mut self, text: String) {
        self.chat.prompt_draft = text;
        self.chat.sync_mention_from_prompt();
    }

    pub(in crate::features) fn blur_chat_prompt(&mut self) {
        self.hide_chat_mention();
        self.chat.response_preview = "AI prompt blurred".to_string();
    }

    pub(in crate::features) fn remove_chat_target_session(&mut self, session_id: &str) {
        self.chat
            .target_session_ids
            .retain(|target_id| target_id != session_id);
        self.panel.status = if self.chat.target_session_ids.is_empty() {
            "AI target sessions cleared".to_string()
        } else {
            "AI target session removed".to_string()
        };
    }

    pub(in crate::features) fn select_chat_mention(
        &mut self,
        session_id: String,
        display_name: String,
    ) {
        if self
            .chat
            .target_session_ids
            .iter()
            .any(|target_id| target_id == &session_id)
        {
            self.chat
                .target_session_ids
                .retain(|target_id| target_id != &session_id);
        } else {
            self.chat.target_session_ids.push(session_id);
        }
        if let Some(at_index) = self.chat.prompt_draft.rfind('@') {
            let suffix = &self.chat.prompt_draft[at_index + 1..];
            if !suffix.chars().any(char::is_whitespace) {
                self.chat.prompt_draft.truncate(at_index);
            }
        }
        self.chat.close_mention();
        self.panel.status = format!("AI target session selected: {display_name}");
    }

    pub(in crate::features) fn begin_chat_job(&mut self) -> AiChatLaunch {
        self.chat.job_id = self.chat.job_id.wrapping_add(1).max(1);
        let cancel = Arc::new(AtomicBool::new(false));
        self.chat.cancel = Some(cancel.clone());
        AiChatLaunch {
            job_id: self.chat.job_id,
            cancel,
            tx: self.chat.tx.clone(),
            session_id: self.chat.session_id.clone(),
        }
    }

    pub(in crate::features) fn begin_chat_request(
        &mut self,
        request_prompt: String,
        mode: AiMode,
        source_label: Option<&str>,
    ) -> AiChatLaunch {
        let launch = self.begin_chat_job();
        if mode == AiMode::Agent {
            self.agent.task_prompt = Some(request_prompt.clone());
            self.agent.step_index = 0;
            self.agent.steps.clear();
            self.agent.thought_expanded.clear();
            self.agent.output_expanded.clear();
            self.upsert_agent_step(
                0,
                AiAgentStepStatus::Planning,
                "Planning",
                truncate_preview(&request_prompt, 120),
            );
        } else {
            self.agent.task_prompt = None;
            self.agent.step_index = 0;
            self.agent.loop_state = None;
            self.agent.steps.clear();
            self.agent.thought_expanded.clear();
            self.agent.output_expanded.clear();
        }
        self.chat.pending = true;
        self.chat.response_preview = if mode == AiMode::Agent {
            "Running AI Agent step...".to_string()
        } else {
            "Running AI request...".to_string()
        };
        self.chat.command_cards.clear();
        let now = nyaterm_core::now_rfc3339();
        let assistant_id = format!("assistant-{}", uuid());
        self.chat.messages.push(AiMessage {
            id: format!("user-{}", uuid()),
            session_id: self.chat.session_id.clone(),
            role: AiMessageRole::User,
            content: request_prompt,
            created_at: now.clone(),
            reasoning_content: None,
            command_cards: Vec::new(),
        });
        self.chat.messages.push(AiMessage {
            id: assistant_id.clone(),
            session_id: self.chat.session_id.clone(),
            role: AiMessageRole::Assistant,
            content: String::new(),
            created_at: now,
            reasoning_content: None,
            command_cards: Vec::new(),
        });
        self.chat.prompt_draft.clear();
        self.chat.quoted_text = None;
        self.chat.message_menu = None;
        self.chat.close_mention();
        self.chat.streaming_assistant_id = Some(assistant_id);
        self.panel.status = if mode == AiMode::Agent {
            "AI Agent step started".to_string()
        } else if let Some(source_label) = source_label {
            format!("AI file action started: {source_label}")
        } else {
            "AI Ask request started".to_string()
        };
        self.chat.prepared_request = None;
        launch
    }

    pub(in crate::features) fn cancel_chat_and_agent(&mut self) {
        if let Some(cancel) = self.chat.cancel.as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.chat.job_id = self.chat.job_id.wrapping_add(1).max(1);
        self.chat.pending = false;
        self.chat.cancel = None;
        let cancelled_step = self
            .agent
            .loop_state
            .as_ref()
            .map(|state| state.step_index)
            .or_else(|| self.agent.steps.last().map(|step| step.step_index));
        if let Some(state) = self.agent.loop_state.take()
            && let Some(marker_id) = state.marker_id.as_deref()
        {
            self.agent.capture.cancel(marker_id);
        }
        self.agent.capture = AgentOutputCaptureProcessor::new();
        self.agent.task_prompt = None;
        self.chat.command_cards.clear();
        self.chat.response_preview = "AI request cancelled".to_string();
        if let Some(assistant_id) = self.chat.streaming_assistant_id.take()
            && let Some(message) = self
                .chat
                .messages
                .iter_mut()
                .rev()
                .find(|message| message.id == assistant_id)
            && message.content.trim().is_empty()
        {
            message.content = "AI request cancelled".to_string();
        }
        self.panel.status = "AI request cancelled".to_string();
        if let Some(step_index) = cancelled_step {
            self.upsert_agent_step(
                step_index,
                AiAgentStepStatus::Cancelled,
                "Cancelled",
                "AI Agent request was cancelled",
            );
        }
    }

    pub(in crate::features) fn drain_chat_events(
        &mut self,
        limit: usize,
    ) -> Vec<AiChatWorkerEvent> {
        if !self.chat.pending {
            return Vec::new();
        }
        let mut events = Vec::new();
        for _ in 0..limit {
            let Ok(event) = self.chat.rx.try_recv() else {
                break;
            };
            events.push(event);
        }
        events
    }

    pub(in crate::features) fn apply_chat_delta(
        &mut self,
        job_id: u64,
        text_delta: &str,
        reasoning_delta: Option<&str>,
    ) -> bool {
        if job_id != self.chat.job_id {
            return false;
        }
        if self.chat.response_preview == "Running AI request..." {
            self.chat.response_preview.clear();
        }
        self.chat.response_preview.push_str(text_delta);
        self.chat.response_preview = truncate_preview(&self.chat.response_preview, 320);
        if let Some(assistant_id) = self.chat.streaming_assistant_id.as_deref()
            && let Some(message) = self
                .chat
                .messages
                .iter_mut()
                .rev()
                .find(|message| message.id == assistant_id)
        {
            message.content.push_str(text_delta);
            if let Some(delta) = reasoning_delta.filter(|delta| !delta.trim().is_empty()) {
                let existing = message.reasoning_content.take().unwrap_or_default();
                message.reasoning_content = Some(format!("{existing}{delta}"));
            }
        }
        self.panel.status = if reasoning_delta.is_some_and(|delta| !delta.trim().is_empty()) {
            "AI stream receiving; reasoning captured".to_string()
        } else {
            "AI stream receiving".to_string()
        };
        true
    }

    pub(in crate::features) fn apply_agent_tool_delta(
        &mut self,
        job_id: u64,
        tool_name: Option<&str>,
        arguments_delta_len: usize,
    ) -> bool {
        if job_id != self.chat.job_id {
            return false;
        }
        let tool_label = tool_name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("tool");
        self.panel.status = if arguments_delta_len == 0 {
            format!("AI Agent selected {tool_label}")
        } else {
            format!("AI Agent streaming {tool_label} arguments (+{arguments_delta_len} chars)")
        };
        self.upsert_agent_step(
            self.last_agent_step_index(),
            AiAgentStepStatus::Tool,
            format!("Tool {tool_label}"),
            if arguments_delta_len == 0 {
                "Provider selected an Agent tool".to_string()
            } else {
                format!("Streaming arguments (+{arguments_delta_len} chars)")
            },
        );
        true
    }

    pub(in crate::features) fn finish_agent_background(
        &mut self,
        job_id: u64,
        state: AiAgentLoopState,
        result: Result<nyaterm_core::CommandObservation, String>,
        observation_summary: impl FnOnce(&nyaterm_core::CommandObservation) -> String,
    ) -> AiAgentBackgroundEffect {
        if job_id != self.chat.job_id {
            return AiAgentBackgroundEffect::Ignored;
        }
        self.chat.cancel = None;
        let Some(active_state) = self.agent.loop_state.take() else {
            return AiAgentBackgroundEffect::MatchedStale;
        };
        if active_state.background_job_id != Some(job_id) {
            self.agent.loop_state = Some(active_state);
            return AiAgentBackgroundEffect::MatchedStale;
        }
        match result {
            Ok(observation) => {
                self.panel.status = match observation.exit_code {
                    Some(code) => format!("AI Agent background command exited with {code}"),
                    None => "AI Agent background command completed".to_string(),
                };
                let detail = observation_summary(&observation);
                self.upsert_agent_step(
                    state.step_index,
                    AiAgentStepStatus::Completed,
                    "Observed",
                    detail,
                );
                AiAgentBackgroundEffect::Continue(Box::new(state), observation)
            }
            Err(error) => {
                self.panel.status = format!("AI Agent background command failed: {error}");
                self.chat.response_preview = self.panel.status.clone();
                self.upsert_agent_step(
                    state.step_index,
                    AiAgentStepStatus::Failed,
                    "Failed",
                    truncate_preview(&error, 140),
                );
                AiAgentBackgroundEffect::Failed
            }
        }
    }

    pub(in crate::features) fn finish_chat_job(
        &mut self,
        job_id: u64,
        session_id: String,
        result: Result<AiChatJobOutput, String>,
    ) -> Option<AiChatFinishEffect> {
        if job_id != self.chat.job_id {
            return None;
        }
        self.chat.pending = false;
        self.chat.cancel = None;
        match result {
            Ok(output) => {
                let command_count = output.command_cards.len();
                self.chat.response_preview = if output.text.trim().is_empty() {
                    "AI returned an empty response".to_string()
                } else {
                    truncate_preview(&output.text, 320)
                };
                let mode_label = if output.mode == AiMode::Agent {
                    "AI Agent"
                } else {
                    "AI Ask"
                };
                let mut status =
                    format!("{mode_label} completed; {command_count} command card(s) parsed");
                if output.reasoning.is_some() {
                    status.push_str("; reasoning captured");
                }
                if let Some(note) = output.approval_note.as_deref() {
                    status.push_str("; ");
                    status.push_str(note);
                }
                if output.mode == AiMode::Agent && command_count > 0 && !output.auto_execute_first {
                    status.push_str("; awaiting command approval");
                }
                self.panel.status = status;
                if output.mode == AiMode::Agent {
                    let (step_status, step_title) = if command_count == 0 {
                        (AiAgentStepStatus::Completed, "Final Answer")
                    } else if output.auto_execute_first {
                        (AiAgentStepStatus::Running, "Auto Execute")
                    } else {
                        (AiAgentStepStatus::NeedsApproval, "Needs Approval")
                    };
                    self.upsert_agent_step(
                        self.last_agent_step_index(),
                        step_status,
                        step_title,
                        truncate_preview(&output.text, 140),
                    );
                }
                self.chat.command_cards = output.command_cards.clone();
                if let Some(assistant_id) = self.chat.streaming_assistant_id.take()
                    && let Some(message) = self
                        .chat
                        .messages
                        .iter_mut()
                        .rev()
                        .find(|message| message.id == assistant_id)
                {
                    if !output.text.trim().is_empty() {
                        message.content = output.text.clone();
                    } else if message.content.trim().is_empty() {
                        message.content = "AI returned an empty response".to_string();
                    }
                    message.reasoning_content = output.reasoning;
                    message.command_cards = output.command_cards;
                }
                self.chat.prompt_draft.clear();
                if output.mode == AiMode::Agent && command_count == 0 {
                    self.agent.loop_state = None;
                    self.agent.task_prompt = None;
                }
                Some(AiChatFinishEffect {
                    session_id,
                    succeeded: true,
                    clear_prompt_input: true,
                    refresh_usage_counts: true,
                    auto_execute_first: output.auto_execute_first
                        && !self.chat.command_cards.is_empty(),
                })
            }
            Err(error) => {
                self.chat.response_preview = format!("AI request failed: {error}");
                self.chat.command_cards.clear();
                self.panel.status = self.chat.response_preview.clone();
                if let Some(assistant_id) = self.chat.streaming_assistant_id.take()
                    && let Some(message) = self
                        .chat
                        .messages
                        .iter_mut()
                        .rev()
                        .find(|message| message.id == assistant_id)
                {
                    message.content = format!("AI request failed: {error}");
                }
                if self.agent.task_prompt.is_some() {
                    self.upsert_agent_step(
                        self.last_agent_step_index(),
                        AiAgentStepStatus::Failed,
                        "Failed",
                        truncate_preview(&error, 140),
                    );
                }
                Some(AiChatFinishEffect {
                    session_id,
                    succeeded: false,
                    clear_prompt_input: false,
                    refresh_usage_counts: false,
                    auto_execute_first: false,
                })
            }
        }
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

    pub(in crate::features) fn upsert_agent_step(
        &mut self,
        step_index: u16,
        status: AiAgentStepStatus,
        title: impl Into<String>,
        detail: impl Into<String>,
    ) {
        let title = title.into();
        let detail = detail.into();
        let lower_title = title.to_ascii_lowercase();
        let looks_like_command = matches!(
            status,
            AiAgentStepStatus::Running | AiAgentStepStatus::Tool | AiAgentStepStatus::NeedsApproval
        ) || lower_title.contains("background")
            || lower_title.contains("auto execute")
            || lower_title.contains("needs approval")
            || lower_title.contains("shell")
            || lower_title.contains("running");
        let looks_like_observation = lower_title.contains("observ")
            || lower_title == "done"
            || lower_title == "completed"
            || lower_title == "failed"
            || matches!(
                status,
                AiAgentStepStatus::Completed | AiAgentStepStatus::Failed
            );
        let looks_like_thought = lower_title.contains("plan")
            || lower_title.contains("think")
            || lower_title.contains("final answer")
            || matches!(status, AiAgentStepStatus::Planning);

        if let Some(step) = self
            .agent
            .steps
            .iter_mut()
            .find(|step| step.step_index == step_index)
        {
            step.status = status;
            step.title = title;
            if !detail.trim().is_empty() {
                step.detail = detail.clone();
            }
            if looks_like_command && !detail.trim().is_empty() {
                step.command = Some(detail.clone());
            }
            if looks_like_observation && !detail.trim().is_empty() {
                step.observation = Some(detail.clone());
            }
            if looks_like_thought && !detail.trim().is_empty() {
                step.thought = Some(detail);
            }
        } else {
            self.agent.steps.push(AiAgentStepView {
                step_index,
                status,
                title,
                detail: detail.clone(),
                thought: (looks_like_thought && !detail.trim().is_empty()).then(|| detail.clone()),
                command: (looks_like_command && !detail.trim().is_empty()).then(|| detail.clone()),
                observation: (looks_like_observation && !detail.trim().is_empty())
                    .then_some(detail),
            });
        }
        let overflow = self.agent.steps.len().saturating_sub(16);
        if overflow > 0 {
            let removed: Vec<u16> = self
                .agent
                .steps
                .iter()
                .take(overflow)
                .map(|step| step.step_index)
                .collect();
            self.agent.steps.drain(..overflow);
            for index in removed {
                self.agent.thought_expanded.remove(&index);
                self.agent.output_expanded.remove(&index);
            }
        }
    }

    pub(in crate::features) fn toggle_agent_thought_expanded(&mut self, step_index: u16) {
        if !self.agent.thought_expanded.remove(&step_index) {
            self.agent.thought_expanded.insert(step_index);
        }
    }

    pub(in crate::features) fn toggle_agent_output_expanded(&mut self, step_index: u16) {
        if !self.agent.output_expanded.remove(&step_index) {
            self.agent.output_expanded.insert(step_index);
        }
    }

    pub(in crate::features) fn agent_task_prompt_or_preview(&self) -> String {
        self.agent
            .task_prompt
            .clone()
            .unwrap_or_else(|| self.chat.response_preview.clone())
    }

    pub(in crate::features) fn begin_agent_step(
        &mut self,
        max_steps: u16,
    ) -> Result<(String, u16), String> {
        let step_index = self.agent.step_index;
        if step_index.saturating_add(1) >= max_steps {
            self.agent.loop_state = None;
            return Err(format!(
                "AI Agent reached max step limit ({max_steps}); review terminal output"
            ));
        }
        self.agent.step_index = self.agent.step_index.saturating_add(1);
        Ok((self.agent_task_prompt_or_preview(), step_index))
    }

    pub(in crate::features) fn register_agent_capture(&mut self, marker_id: String) {
        self.agent.capture.register(marker_id);
    }

    pub(in crate::features) fn set_agent_loop(&mut self, state: AiAgentLoopState) {
        self.agent.loop_state = Some(state);
    }

    pub(in crate::features) fn stop_agent_for_closed_target(&mut self) -> Option<u16> {
        let state = self.agent.loop_state.take()?;
        self.panel.status = "AI Agent loop stopped because the target session closed".to_string();
        self.upsert_agent_step(
            state.step_index,
            AiAgentStepStatus::Failed,
            "Stopped",
            "Target session closed",
        );
        Some(state.step_index)
    }

    pub(in crate::features) fn poll_agent_observation(
        &mut self,
        now: Instant,
        current_len: usize,
        quiet: Duration,
    ) -> AiAgentObservationPoll {
        if self.chat.pending {
            return AiAgentObservationPoll::Waiting;
        }
        let Some(state) = self.agent.loop_state.as_mut() else {
            return AiAgentObservationPoll::Waiting;
        };
        if state.background_job_id.is_some() {
            return AiAgentObservationPoll::Waiting;
        }
        if current_len != state.last_seen_len {
            state.last_seen_len = current_len;
            state.stable_since = now;
            return AiAgentObservationPoll::Waiting;
        }
        if now < state.min_wait_until {
            return AiAgentObservationPoll::Waiting;
        }
        let has_observed_output = current_len > state.output_start_len;
        let output_is_quiet = now.duration_since(state.stable_since) >= quiet;
        let timed_out = now >= state.timeout_at;
        if timed_out && state.marker_id.is_some() {
            let state = self.agent.loop_state.take().expect("agent loop is present");
            if let Some(marker_id) = state.marker_id.as_deref() {
                self.agent.capture.cancel(marker_id);
            }
            self.panel.status = format!("AI Agent command capture timed out: {}", state.command);
            return AiAgentObservationPoll::TimedOut(state);
        }
        if !timed_out && (!has_observed_output || !output_is_quiet) {
            return AiAgentObservationPoll::Waiting;
        }
        if state.marker_id.is_some() {
            return AiAgentObservationPoll::Waiting;
        }
        AiAgentObservationPoll::Target(self.agent.loop_state.take().expect("agent loop is present"))
    }

    pub(in crate::features) fn take_agent_loop_for_marker(
        &mut self,
        marker_id: &str,
    ) -> Option<AiAgentLoopState> {
        if !self
            .agent
            .loop_state
            .as_ref()
            .is_some_and(|state| state.marker_id.as_deref() == Some(marker_id))
        {
            return None;
        }
        self.agent.loop_state.take()
    }

    pub(in crate::features) fn take_agent_loop_for_session(
        &mut self,
        session_id: &str,
    ) -> Option<AiAgentLoopState> {
        if !self
            .agent
            .loop_state
            .as_ref()
            .is_some_and(|state| state.terminal_session_id == session_id)
        {
            return None;
        }
        let state = self.agent.loop_state.take()?;
        if let Some(marker_id) = state.marker_id.as_deref() {
            self.agent.capture.cancel(marker_id);
        }
        Some(state)
    }

    pub(in crate::features) fn begin_agent_continuation(
        &mut self,
        state: &AiAgentLoopState,
    ) -> Option<AiChatLaunch> {
        if self.chat.pending {
            self.agent.loop_state = Some(state.clone());
            return None;
        }
        let mut launch = self.begin_chat_job();
        launch.session_id = state.ai_session_id.clone();
        self.chat.pending = true;
        self.chat.response_preview = format!(
            "Running AI Agent continuation step {}/{}...",
            state.step_index + 2,
            state.max_steps
        );
        self.chat.command_cards.clear();
        self.panel.status = self.chat.response_preview.clone();
        self.upsert_agent_step(
            state.step_index.saturating_add(1),
            AiAgentStepStatus::Planning,
            "Planning",
            "Continuing from the latest command observation",
        );
        Some(launch)
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
    fn close_message_menu(&mut self) {
        self.message_menu = None;
    }

    /// Tracks the trailing `@mention` the composer is currently completing.
    ///
    /// Only a trailing run with no whitespace counts, so the picker closes as
    /// soon as the user types past the mention. The rules are unchanged.
    fn sync_mention_from_prompt(&mut self) {
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

impl AiSettingsState {
    fn actions(&self, kind: AiActionListKind) -> &[AiCustomActionConfig] {
        match kind {
            AiActionListKind::Terminal => &self.config.terminal_ai_actions,
            AiActionListKind::File => &self.config.file_ai_actions,
        }
    }

    fn actions_mut(&mut self, kind: AiActionListKind) -> &mut Vec<AiCustomActionConfig> {
        match kind {
            AiActionListKind::Terminal => &mut self.config.terminal_ai_actions,
            AiActionListKind::File => &mut self.config.file_ai_actions,
        }
    }

    fn action(&self, kind: AiActionListKind, action_id: &str) -> Option<&AiCustomActionConfig> {
        self.actions(kind)
            .iter()
            .find(|action| action.id == action_id)
    }

    fn action_mut(
        &mut self,
        kind: AiActionListKind,
        action_id: &str,
    ) -> Option<&mut AiCustomActionConfig> {
        self.actions_mut(kind)
            .iter_mut()
            .find(|action| action.id == action_id)
    }
}

impl AiHistoryState {
    fn cancel_clear_confirm(&mut self) {
        self.clear_confirm_open = false;
    }
}

impl AiAgentState {
    fn cancel_auto_execution_confirm(&mut self) {
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
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};

    use gpui::{TestAppContext, px};
    use nyaterm_core::{
        AiAction, AiContext, AiMessage, AiMessageRole, AiMode, AiModelConfigItem, AiModelSource,
        AiProviderCredential, AiProviderKind, AiSession, AiSettings,
    };

    use crate::features::{
        AiAgentLoopState, AiAgentStepStatus, AiChatJobOutput, AiChatWorkerEvent,
        AiDiscoveryJobResult,
    };
    use crate::models::{AiMessageMenuState, AiPreparedRequest};

    use super::{AiFeatureFocus, AiFeatureState, AiSettingsMutation};

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
    fn pending_settings_preserve_masked_secret_until_a_new_draft_exists() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        state.settings.config.provider_profiles[0].api_key = Some("__SET__".to_string());
        state.settings.config.provider_credentials[0].api_key = Some("__SET__".to_string());

        let pending = state.pending_settings();
        assert_eq!(
            pending.provider_profiles[0].api_key.as_deref(),
            Some("__SET__")
        );
        assert_eq!(
            pending.provider_credentials[0].api_key.as_deref(),
            Some("__SET__")
        );

        state.apply_settings_input(
            crate::models::AiInputField::ApiKey,
            "replacement".to_string(),
        );
        let pending = state.pending_settings();
        assert_eq!(
            pending.provider_profiles[0].api_key.as_deref(),
            Some("replacement")
        );
        assert_eq!(
            pending.provider_credentials[0].api_key.as_deref(),
            Some("replacement")
        );
    }

    #[test]
    fn model_catalog_mutations_keep_default_model_valid() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        let first = "openai:model-a".to_string();
        let fallback = "openai:model-b".to_string();
        state.settings.config.models = vec![
            AiModelConfigItem {
                id: first.clone(),
                name: "model-a".to_string(),
                provider_kind: Some(AiProviderKind::Openai),
                credential_id: None,
                enabled: true,
                source: AiModelSource::RustGenai,
                last_seen_at: None,
            },
            AiModelConfigItem {
                id: fallback.clone(),
                name: "model-b".to_string(),
                provider_kind: Some(AiProviderKind::Openai),
                credential_id: None,
                enabled: true,
                source: AiModelSource::RustGenai,
                last_seen_at: None,
            },
        ];
        state.settings.config.default_model_id = Some(first.clone());

        state.toggle_settings_model_enabled(&first);
        assert_eq!(
            state.settings.config.default_model_id.as_deref(),
            Some(fallback.as_str())
        );
        assert!(state.default_model_is_enabled());

        state
            .settings
            .config
            .provider_credentials
            .push(AiProviderCredential {
                id: "custom".to_string(),
                name: "Custom".to_string(),
                provider_kind: AiProviderKind::OpenaiCompatible,
                base_url: Some("https://example.invalid".to_string()),
                api_key: None,
                enabled: true,
            });
        state.settings.config.default_model_id = None;
        assert_eq!(
            state.add_settings_manual_model("custom", "model-x"),
            AiSettingsMutation::Persist
        );
        let manual_id = state.settings.config.default_model_id.clone().unwrap();
        assert_eq!(
            state.remove_settings_manual_model(&manual_id),
            AiSettingsMutation::Persist
        );
        assert!(state.default_model_is_enabled());
    }

    #[test]
    fn credential_edits_move_secret_drafts_into_both_compatible_records() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        assert!(state.apply_settings_credential_input("openai.api-key", "new-key".to_string()));
        state.commit_settings_credential_edits("openai");

        assert_eq!(
            state.settings.config.provider_credentials[0]
                .api_key
                .as_deref(),
            Some("new-key")
        );
        assert_eq!(
            state.settings.config.provider_profiles[0]
                .api_key
                .as_deref(),
            Some("new-key")
        );
        assert!(
            !state
                .settings
                .credential_secret_drafts
                .contains_key("openai")
        );
    }

    #[test]
    fn credential_catalog_changes_preserve_an_absent_default_model() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        state.settings.config.default_model_id = None;
        state.settings.config.models.push(AiModelConfigItem {
            id: "openai:model-a".to_string(),
            name: "model-a".to_string(),
            provider_kind: Some(AiProviderKind::Openai),
            credential_id: None,
            enabled: true,
            source: AiModelSource::RustGenai,
            last_seen_at: None,
        });

        assert_eq!(
            state.toggle_settings_credential_enabled("openai"),
            AiSettingsMutation::Persist
        );
        assert!(state.settings.config.default_model_id.is_none());

        state
            .settings
            .config
            .provider_credentials
            .push(AiProviderCredential {
                id: "custom".to_string(),
                name: "Custom".to_string(),
                provider_kind: AiProviderKind::OpenaiCompatible,
                base_url: Some("https://example.invalid".to_string()),
                api_key: None,
                enabled: true,
            });
        assert_eq!(
            state.remove_settings_credential("custom"),
            AiSettingsMutation::Persist
        );
        assert!(state.settings.config.default_model_id.is_none());
    }

    #[test]
    fn action_and_discovery_catalog_updates_stay_on_settings_owner() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        state.add_settings_action(
            crate::models::AiActionListKind::Terminal,
            "custom-action".to_string(),
        );
        assert!(state.apply_settings_action_input(
            crate::models::AiActionListKind::Terminal,
            "custom-action",
            crate::models::AiActionEditorField::Prompt,
            "explain".to_string(),
        ));
        assert_eq!(
            state.settings_action_value(
                crate::models::AiActionListKind::Terminal,
                "custom-action",
                crate::models::AiActionEditorField::Prompt,
            ),
            "explain"
        );

        let discovery = nyaterm_core::AiModelDiscovery {
            id: "custom:model-y".to_string(),
            name: "model-y".to_string(),
            provider_kind: Some(AiProviderKind::OpenaiCompatible),
            credential_id: Some("custom".to_string()),
            source: AiModelSource::Manual,
        };
        assert_eq!(state.apply_settings_model_discoveries(vec![discovery]), 1);
        assert!(
            state
                .settings
                .config
                .models
                .iter()
                .any(|model| model.id == "custom:model-y")
        );
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
    fn chat_start_stream_and_finish_are_reduced_by_the_owner() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        state.set_chat_prompt_draft("deploy".to_string());

        let launch = state.begin_chat_request("deploy".to_string(), AiMode::Agent, None);

        assert!(state.chat_is_pending());
        assert_eq!(state.chat_messages().len(), 2);
        assert!(state.chat_prompt_draft().is_empty());
        assert_eq!(state.agent_steps().len(), 1);
        assert_eq!(state.agent_steps()[0].status, AiAgentStepStatus::Planning);
        launch
            .tx
            .send(AiChatWorkerEvent::Delta {
                job_id: launch.job_id,
                session_id: launch.session_id.clone(),
                text_delta: "working".to_string(),
                reasoning_delta: Some("reason".to_string()),
            })
            .unwrap();
        let event = state.drain_chat_events(1).pop().unwrap();
        let AiChatWorkerEvent::Delta {
            job_id,
            text_delta,
            reasoning_delta,
            ..
        } = event
        else {
            panic!("expected stream delta");
        };
        assert!(state.apply_chat_delta(job_id, &text_delta, reasoning_delta.as_deref()));
        assert_eq!(
            state.chat_response_preview(),
            "Running AI Agent step...working"
        );
        assert_eq!(
            state.chat_messages()[1].reasoning_content.as_deref(),
            Some("reason")
        );

        let effect = state
            .finish_chat_job(
                launch.job_id,
                launch.session_id,
                Ok(AiChatJobOutput {
                    mode: AiMode::Agent,
                    text: "done".to_string(),
                    reasoning: Some("final reason".to_string()),
                    command_cards: Vec::new(),
                    auto_execute_first: false,
                    approval_note: None,
                }),
            )
            .unwrap();
        assert!(effect.succeeded);
        assert!(effect.clear_prompt_input);
        assert!(!state.chat_is_pending());
        assert_eq!(state.chat_response_preview(), "done");
        assert_eq!(state.agent_steps()[0].title, "Final Answer");
        assert!(state.agent_loop_snapshot().is_none());
    }

    #[test]
    fn chat_cancel_invalidates_the_job_and_clears_agent_lifecycle() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        let launch = state.begin_chat_request("inspect".to_string(), AiMode::Agent, None);

        state.cancel_chat_and_agent();

        assert!(launch.cancel.load(Ordering::Relaxed));
        assert!(!state.chat_is_pending());
        assert_eq!(state.chat_response_preview(), "AI request cancelled");
        assert_eq!(state.agent_steps()[0].status, AiAgentStepStatus::Cancelled);
        assert!(
            state
                .finish_chat_job(launch.job_id, launch.session_id, Err("late".to_string()),)
                .is_none()
        );
    }

    #[test]
    fn mention_selection_and_navigation_are_atomic_owner_transitions() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        state.set_chat_prompt_draft("run @server".to_string());
        assert!(state.chat_mention_is_open());
        assert_eq!(state.chat_mention_query(), "server");

        state.move_chat_mention_index(3, -1);
        assert_eq!(state.chat_mention_index(), 2);
        state.hide_chat_mention();
        assert_eq!(state.chat_mention_index(), 2);
        state.set_chat_prompt_draft("run @server".to_string());
        state.select_chat_mention("session-a".to_string(), "Server A".to_string());

        assert_eq!(state.chat_prompt_draft(), "run ");
        assert_eq!(state.chat_target_session_ids(), &["session-a".to_string()]);
        assert!(!state.chat_mention_is_open());
        assert_eq!(state.panel_status(), "AI target session selected: Server A");

        state.remove_chat_target_session("session-a");
        assert!(state.chat_target_session_ids().is_empty());
        assert_eq!(state.panel_status(), "AI target sessions cleared");
    }

    #[test]
    fn background_completion_distinguishes_foreign_and_matched_stale_jobs() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        let launch = state.begin_chat_job();
        let now = Instant::now();
        let loop_state = AiAgentLoopState {
            ai_session_id: "session-a".to_string(),
            terminal_session_id: "terminal-a".to_string(),
            task_prompt: "inspect".to_string(),
            command: "pwd".to_string(),
            marker_id: None,
            background_job_id: Some(launch.job_id),
            step_index: 0,
            max_steps: 3,
            output_start_len: 0,
            started_at: now,
            min_wait_until: now,
            timeout_at: now + Duration::from_secs(1),
            last_seen_len: 0,
            stable_since: now,
        };

        assert!(matches!(
            state.finish_agent_background(
                launch.job_id.wrapping_add(1),
                loop_state.clone(),
                Err("foreign".to_string()),
                |_| String::new(),
            ),
            super::AiAgentBackgroundEffect::Ignored
        ));
        assert!(matches!(
            state.finish_agent_background(
                launch.job_id,
                loop_state,
                Err("stale".to_string()),
                |_| String::new(),
            ),
            super::AiAgentBackgroundEffect::MatchedStale
        ));
    }

    #[test]
    fn agent_step_limit_and_observation_poll_stay_on_the_owner() {
        let cx = TestAppContext::single();
        let mut state = state(&cx);
        assert!(state.begin_agent_step(1).is_err());

        let now = Instant::now();
        state.set_agent_loop(AiAgentLoopState {
            ai_session_id: "session-a".to_string(),
            terminal_session_id: "terminal-a".to_string(),
            task_prompt: "inspect".to_string(),
            command: "pwd".to_string(),
            marker_id: None,
            background_job_id: None,
            step_index: 0,
            max_steps: 3,
            output_start_len: 4,
            started_at: now - Duration::from_secs(2),
            min_wait_until: now - Duration::from_secs(1),
            timeout_at: now + Duration::from_secs(10),
            last_seen_len: 8,
            stable_since: now - Duration::from_secs(1),
        });

        let poll = state.poll_agent_observation(now, 8, Duration::from_millis(100));
        assert!(matches!(poll, super::AiAgentObservationPoll::Target(_)));
        assert!(state.agent_loop_snapshot().is_none());
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
