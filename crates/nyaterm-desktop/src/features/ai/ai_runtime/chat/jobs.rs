use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use gpui::{Context, KeyDownEvent};
use nyaterm_core::{
    AgentOutputCaptureProcessor, AiAction, AiChatRequest, AiContext, AiMessage, AiMessageRole,
    AiMode, now_rfc3339, truncate_preview, uuid,
};
use nyaterm_transport::SessionInfo;

use crate::features::{
    AiAgentStepStatus, AiChatJobResult, AiChatWorkerEvent, NyaTermApp, compact_id,
    recent_terminal_output, session_kind_label,
};
use crate::models::SessionLaunchConfig;

use super::super::super::ai_jobs::{observation_summary, run_ai_ask_job};

const AI_CHAT_EVENT_DRAIN_LIMIT: usize = 256;

impl NyaTermApp {
    pub(in crate::features) fn begin_ai_chat_job(&mut self) -> (u64, Arc<AtomicBool>) {
        self.ai.chat.job_id = self.ai.chat.job_id.wrapping_add(1).max(1);
        let cancel = Arc::new(AtomicBool::new(false));
        self.ai.chat.cancel = Some(cancel.clone());
        (self.ai.chat.job_id, cancel)
    }

    pub(in crate::features) fn cancel_ai_chat(&mut self, cx: &mut Context<Self>) {
        if let Some(cancel) = self.ai.chat.cancel.as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.ai.chat.job_id = self.ai.chat.job_id.wrapping_add(1).max(1);
        self.ai.chat.pending = false;
        self.ai.chat.cancel = None;
        let cancelled_step = self
            .ai
            .agent
            .loop_state
            .as_ref()
            .map(|state| state.step_index)
            .or_else(|| self.ai.agent.steps.last().map(|step| step.step_index));
        if let Some(state) = self.ai.agent.loop_state.take()
            && let Some(marker_id) = state.marker_id.as_deref()
        {
            self.ai.agent.capture.cancel(marker_id);
        }
        self.ai.agent.capture = AgentOutputCaptureProcessor::new();
        self.sync_session_event_bridge_policy();
        self.ai.agent.task_prompt = None;
        self.ai.chat.command_cards.clear();
        self.ai.chat.response_preview = "AI request cancelled".to_string();
        if let Some(assistant_id) = self.ai.chat.streaming_assistant_id.take() {
            if let Some(message) = self
                .ai
                .chat
                .messages
                .iter_mut()
                .rev()
                .find(|message| message.id == assistant_id)
            {
                if message.content.trim().is_empty() {
                    message.content = "AI request cancelled".to_string();
                }
            }
        }
        self.ai.panel.status = "AI request cancelled".to_string();
        if let Some(step_index) = cancelled_step {
            self.upsert_ai_agent_step(
                step_index,
                AiAgentStepStatus::Cancelled,
                "Cancelled",
                "AI Agent request was cancelled",
            );
        }
        self.settings
            .set_store_message(self.ai.panel.status.clone());
        cx.notify();
    }

    pub(in crate::features) fn start_ai_ask(&mut self, cx: &mut Context<Self>) {
        if self.ai.chat.pending {
            self.ai.chat.response_preview = "AI request already running".to_string();
            cx.notify();
            return;
        }
        if self.ai.agent.loop_state.is_some() {
            self.ai.chat.response_preview = "AI Agent step already running".to_string();
            self.ai.panel.status = self.ai.chat.response_preview.clone();
            cx.notify();
            return;
        }
        let prompt = self.ai.chat.prompt_draft.trim().to_string();
        if prompt.is_empty() {
            self.ai.chat.response_preview = "Enter a prompt first".to_string();
            cx.notify();
            return;
        }
        let request_prompt = self
            .ai
            .chat
            .quoted_text
            .as_ref()
            .map(|quoted| quoted.trim())
            .filter(|quoted| !quoted.is_empty())
            .map(|quoted| format!("> {quoted}\n\n{prompt}"))
            .unwrap_or_else(|| prompt.clone());
        if !self.ai.settings.config.enabled {
            self.ai.chat.response_preview = "AI assistant is disabled".to_string();
            cx.notify();
            return;
        }
        let Some(model_id) = self.ai_selected_model_id() else {
            self.ai.chat.response_preview = "Enable an AI model before sending".to_string();
            self.ai.panel.status = self.ai.chat.response_preview.clone();
            cx.notify();
            return;
        };

        let settings = self.ai.settings.config.clone();
        let mode = settings.default_mode.clone();
        let target_session_ids = self.ai_effective_target_session_ids();
        let target_session_id = target_session_ids.first().cloned();
        if mode == AiMode::Agent && target_session_id.is_none() {
            self.ai.chat.response_preview =
                "Start a terminal session before running Agent mode".to_string();
            self.ai.panel.status = self.ai.chat.response_preview.clone();
            cx.notify();
            return;
        }
        let prepared_request = self.ai.chat.prepared_request.clone();
        let action = prepared_request
            .as_ref()
            .map(|request| request.action.clone())
            .unwrap_or(AiAction::GenerateCommand);
        let context = prepared_request
            .as_ref()
            .map(|request| request.context.clone())
            .unwrap_or_else(|| self.ai_terminal_context_for_sessions(&target_session_ids));
        let source_label = prepared_request
            .as_ref()
            .map(|request| request.source_label.clone());
        let session_id = self.ai.chat.session_id.clone();
        let request = AiChatRequest {
            stream_id: None,
            session_id: Some(session_id.clone()),
            connection_id: target_session_id.clone(),
            terminal_session_id: target_session_id.clone(),
            mode: mode.clone(),
            model_id: Some(model_id),
            model_name: None,
            action,
            user_input: request_prompt.clone(),
            context,
            options: Default::default(),
        };
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        let tx = self.ai.chat.tx.clone();
        let (job_id, cancel) = self.begin_ai_chat_job();

        if mode == AiMode::Agent {
            self.ai.agent.task_prompt = Some(request.user_input.clone());
            self.ai.agent.step_index = 0;
            self.ai.agent.steps.clear();
            self.ai.agent.thought_expanded.clear();
            self.ai.agent.output_expanded.clear();
            self.upsert_ai_agent_step(
                0,
                AiAgentStepStatus::Planning,
                "Planning",
                truncate_preview(&request.user_input, 120),
            );
        } else {
            self.ai.agent.task_prompt = None;
            self.ai.agent.step_index = 0;
            self.ai.agent.loop_state = None;
            self.ai.agent.steps.clear();
            self.ai.agent.thought_expanded.clear();
            self.ai.agent.output_expanded.clear();
        }
        self.ai.chat.pending = true;
        self.ai.chat.response_preview = if mode == AiMode::Agent {
            "Running AI Agent step...".to_string()
        } else {
            "Running AI request...".to_string()
        };
        self.ai.chat.command_cards.clear();
        let now = now_rfc3339();
        let assistant_id = format!("assistant-{}", uuid());
        self.ai.chat.messages.push(AiMessage {
            id: format!("user-{}", uuid()),
            session_id: self.ai.chat.session_id.clone(),
            role: AiMessageRole::User,
            content: request_prompt.clone(),
            created_at: now.clone(),
            reasoning_content: None,
            command_cards: Vec::new(),
        });
        self.ai.chat.messages.push(AiMessage {
            id: assistant_id.clone(),
            session_id: self.ai.chat.session_id.clone(),
            role: AiMessageRole::Assistant,
            content: String::new(),
            created_at: now,
            reasoning_content: None,
            command_cards: Vec::new(),
        });
        self.reset_text_input("ai.chat.prompt", "", cx);
        self.ai.chat.prompt_draft.clear();
        self.ai.chat.quoted_text = None;
        self.ai.chat.message_menu = None;
        self.ai.chat.mention_open = false;
        self.ai.chat.mention_query.clear();
        self.ai.chat.mention_index = 0;
        self.ai.chat.streaming_assistant_id = Some(assistant_id);
        self.ai.panel.status = if mode == AiMode::Agent {
            "AI Agent step started".to_string()
        } else if let Some(source_label) = source_label.as_ref() {
            format!("AI file action started: {source_label}")
        } else {
            "AI Ask request started".to_string()
        };
        self.ai.chat.prepared_request = None;
        std::thread::spawn(move || {
            let result = run_ai_ask_job(
                config_dir,
                portable_key_path,
                settings,
                request,
                Some(tx.clone()),
                cancel,
                job_id,
            );
            let _ = tx.send(AiChatWorkerEvent::Finished(AiChatJobResult {
                job_id,
                session_id,
                result,
            }));
        });
        cx.notify();
    }

    pub(in crate::features) fn ai_terminal_context(&self) -> AiContext {
        self.ai_terminal_context_for_session(self.session.active_id())
    }

    pub(in crate::features) fn ai_selected_model_id(&self) -> Option<String> {
        self.ai
            .settings
            .config
            .models
            .iter()
            .find(|model| {
                model.enabled
                    && self.ai.settings.config.default_model_id.as_deref()
                        == Some(model.id.as_str())
            })
            .or_else(|| {
                self.ai
                    .settings
                    .config
                    .models
                    .iter()
                    .find(|model| model.enabled)
            })
            .map(|model| model.id.clone())
    }

    pub(in crate::features) fn ai_enabled_models(&self) -> Vec<nyaterm_core::AiModelConfigItem> {
        self.ai
            .settings
            .config
            .models
            .iter()
            .filter(|model| model.enabled)
            .cloned()
            .collect()
    }

    pub(in crate::features) fn ai_model_provider_label(
        &self,
        model: &nyaterm_core::AiModelConfigItem,
    ) -> String {
        model
            .credential_id
            .as_ref()
            .and_then(|credential_id| {
                self.ai
                    .settings
                    .config
                    .provider_credentials
                    .iter()
                    .find(|credential| &credential.id == credential_id)
                    .map(|credential| credential.name.clone())
            })
            .or_else(|| model.provider_kind.as_ref().map(|kind| format!("{kind:?}")))
            .unwrap_or_else(|| "model".to_string())
    }

    pub(in crate::features) fn ai_filtered_model_choices(
        &self,
    ) -> Vec<(nyaterm_core::AiModelConfigItem, String)> {
        let query = self.ai.discovery.query.trim().to_ascii_lowercase();
        self.ai_enabled_models()
            .into_iter()
            .filter_map(|model| {
                let provider_label = self.ai_model_provider_label(&model);
                let search_value =
                    format!("{} {} {}", model.name, provider_label, model.id).to_ascii_lowercase();
                (query.is_empty() || search_value.contains(&query))
                    .then_some((model, provider_label))
            })
            .collect()
    }

    pub(in crate::features) fn ai_selected_model_index(&self) -> usize {
        let Some(selected_model_id) = self.ai_selected_model_id() else {
            return 0;
        };
        self.ai_filtered_model_choices()
            .iter()
            .position(|(model, _)| model.id == selected_model_id)
            .unwrap_or(0)
    }

    pub(in crate::features) fn select_ai_model_choice(&mut self, cx: &mut Context<Self>) {
        let choices = self.ai_filtered_model_choices();
        let Some((model, _)) = choices.get(self.ai.discovery.index).cloned() else {
            cx.notify();
            return;
        };
        self.ai.discovery.menu_open = false;
        self.ai.discovery.query.clear();
        self.ai.discovery.index = 0;
        self.set_ai_default_model(model.id.clone(), cx);
        self.ai.panel.status = format!("AI model selected: {}", model.name);
    }

    pub(in crate::features) fn handle_ai_model_search_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        // The box owns the text; the menu owns the keys that walk and pick.
        let choice_count = self.ai_filtered_model_choices().len();
        match keystroke.key.as_str() {
            "escape" => {
                if self.ai.discovery.query.is_empty() {
                    self.ai.discovery.menu_open = false;
                } else {
                    self.reset_text_input("ai.model-search", "", cx);
                    self.ai.discovery.query.clear();
                    self.ai.discovery.index = self.ai_selected_model_index();
                }
                cx.notify();
            }
            "up" => {
                if choice_count > 0 {
                    self.ai.discovery.index =
                        (self.ai.discovery.index + choice_count - 1) % choice_count;
                }
                cx.notify();
            }
            "down" => {
                if choice_count > 0 {
                    self.ai.discovery.index = (self.ai.discovery.index + 1) % choice_count;
                }
                cx.notify();
            }
            "enter" => {
                if choice_count > 0 {
                    self.select_ai_model_choice(cx);
                } else {
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    /// Apply an edit from the model search box.
    pub(in crate::features) fn apply_ai_model_search(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.ai.discovery.query = text;
        // A new filter means a different list, so the highlight starts over.
        self.ai.discovery.index = 0;
        cx.notify();
    }

    pub(in crate::features) fn ai_effective_target_session_id(&self) -> Option<String> {
        self.ai_effective_target_session_ids().into_iter().next()
    }

    pub(in crate::features) fn ai_effective_target_session_ids(&self) -> Vec<String> {
        let mut session_ids = Vec::new();
        for session_id in &self.ai.chat.target_session_ids {
            if !session_ids.iter().any(|id| id == session_id)
                && self.session_info(session_id).is_some()
                && !self.is_session_disconnected(session_id)
            {
                session_ids.push(session_id.clone());
            }
        }
        if session_ids.is_empty()
            && let Some(active_session_id) = self.session.active_id()
            && self.session_info(active_session_id).is_some()
            && !self.is_session_disconnected(active_session_id)
        {
            session_ids.push(active_session_id.to_string());
        }
        session_ids
    }

    pub(in crate::features) fn ai_terminal_context_for_sessions(
        &self,
        session_ids: &[String],
    ) -> AiContext {
        if session_ids.len() <= 1 {
            return self.ai_terminal_context_for_session(session_ids.first().map(String::as_str));
        }

        let per_session_line_limit =
            (self.ai.settings.config.context_line_limit as usize / session_ids.len()).max(1);
        let mut contexts = Vec::with_capacity(session_ids.len());
        for session_id in session_ids {
            contexts.push((
                self.session_display_name(session_id)
                    .unwrap_or_else(|| compact_id(session_id)),
                self.ai_terminal_context_for_session_with_line_limit(
                    Some(session_id),
                    per_session_line_limit,
                ),
            ));
        }

        AiContext {
            connection_name: Some(
                contexts
                    .iter()
                    .map(|(_, context)| context.connection_name.as_deref().unwrap_or("-"))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            host: Some(
                contexts
                    .iter()
                    .map(|(_, context)| context.host.as_deref().unwrap_or("-"))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            port: contexts.first().and_then(|(_, context)| context.port),
            username: Some(
                contexts
                    .iter()
                    .map(|(_, context)| context.username.as_deref().unwrap_or("-"))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            cwd: Some(
                contexts
                    .iter()
                    .map(|(_, context)| context.cwd.as_deref().unwrap_or("-"))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            os: contexts.first().and_then(|(_, context)| context.os.clone()),
            arch: contexts
                .first()
                .and_then(|(_, context)| context.arch.clone()),
            recent_output: contexts
                .iter()
                .filter_map(|(label, context)| {
                    (!context.recent_output.trim().is_empty())
                        .then(|| format!("[{label}]\n{}", context.recent_output))
                })
                .collect::<Vec<_>>()
                .join("\n---\n"),
            selected_text: contexts
                .iter()
                .map(|(_, context)| context.selected_text.trim())
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
            input_buffer: contexts
                .iter()
                .map(|(_, context)| context.input_buffer.trim())
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    pub(in crate::features) fn ai_mention_candidates(&self) -> Vec<SessionInfo> {
        let query = self.ai.chat.mention_query.trim().to_ascii_lowercase();
        self.ordered_sessions()
            .into_iter()
            .filter(|session| !self.is_session_disconnected(&session.id))
            .filter(|session| {
                if query.is_empty() {
                    return true;
                }
                let display_name = self.session_display_name_by_info(session);
                display_name.to_ascii_lowercase().contains(&query)
                    || session.id.to_ascii_lowercase().contains(&query)
                    || session_kind_label(session.kind)
                        .to_ascii_lowercase()
                        .contains(&query)
            })
            .collect()
    }

    pub(in crate::features) fn remove_ai_target_session(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        self.ai
            .chat
            .target_session_ids
            .retain(|target_id| target_id != &session_id);
        if self.ai.chat.target_session_ids.is_empty() {
            self.ai.panel.status = "AI target sessions cleared".to_string();
        } else {
            self.ai.panel.status = "AI target session removed".to_string();
        }
        cx.notify();
    }

    fn sync_ai_mention_from_prompt(&mut self) {
        self.ai.chat.sync_mention_from_prompt();
    }

    pub(in crate::features) fn select_ai_mention_candidate(&mut self, cx: &mut Context<Self>) {
        let candidates = self.ai_mention_candidates();
        let Some(session) = candidates.get(self.ai.chat.mention_index).cloned() else {
            self.ai.chat.mention_open = false;
            self.ai.chat.mention_query.clear();
            self.ai.chat.mention_index = 0;
            cx.notify();
            return;
        };

        if self
            .ai
            .chat
            .target_session_ids
            .iter()
            .any(|session_id| session_id == &session.id)
        {
            self.ai
                .chat
                .target_session_ids
                .retain(|session_id| session_id != &session.id);
        } else {
            self.ai.chat.target_session_ids.push(session.id.clone());
        }

        if let Some(at_index) = self.ai.chat.prompt_draft.rfind('@') {
            let suffix = &self.ai.chat.prompt_draft[at_index + 1..];
            if !suffix.chars().any(char::is_whitespace) {
                self.ai.chat.prompt_draft.truncate(at_index);
            }
        }
        self.ai.chat.mention_open = false;
        self.ai.chat.mention_query.clear();
        self.ai.chat.mention_index = 0;
        self.ai.panel.status = format!(
            "AI target session selected: {}",
            self.session_display_name_by_info(&session)
        );
        cx.notify();
    }

    pub(in crate::features) fn ai_terminal_context_for_session(
        &self,
        session_id: Option<&str>,
    ) -> AiContext {
        self.ai_terminal_context_for_session_with_line_limit(
            session_id,
            self.ai.settings.config.context_line_limit as usize,
        )
    }

    pub(in crate::features) fn ai_terminal_context_for_session_with_line_limit(
        &self,
        session_id: Option<&str>,
        line_limit: usize,
    ) -> AiContext {
        let metadata = session_id.and_then(|session_id| self.session.metadata(session_id));
        let ssh = match metadata.map(|metadata| &metadata.launch_config) {
            Some(SessionLaunchConfig::Ssh(config)) => Some(config),
            _ if session_id == self.session.active_id() => self.session.active_ssh_config(),
            _ => None,
        };
        let session = session_id.and_then(|session_id| self.session_info(session_id));
        let cwd = metadata
            .and_then(|metadata| match &metadata.launch_config {
                SessionLaunchConfig::Local(config) => config.working_dir.as_ref(),
                _ => None,
            })
            .or_else(|| {
                session
                    .as_ref()
                    .and_then(|session| session.working_dir.as_ref())
            });
        let recent_output = session_id
            .map(|session_id| self.terminal_buffer_text_for_session(session_id))
            .unwrap_or_else(|| self.active_terminal_buffer_text());
        let selected_text = if session_id.is_none() || session_id == self.session.active_id() {
            self.selected_terminal_text().unwrap_or_default()
        } else {
            String::new()
        };
        AiContext {
            connection_name: ssh
                .map(|config| config.name.clone())
                .or_else(|| session.as_ref().map(|session| session.name.clone())),
            host: ssh.map(|config| config.host.clone()),
            port: ssh.map(|config| config.port),
            username: ssh.map(|config| config.username.clone()),
            cwd: cwd.map(|path| path.display().to_string()),
            os: None,
            arch: Some(std::env::consts::ARCH.to_string()),
            recent_output: recent_terminal_output(&recent_output, line_limit.max(1)),
            selected_text,
            input_buffer: String::new(),
        }
    }

    pub(in crate::features) fn handle_ai_prompt_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        if self.ai.chat.pending
            || self.ai.agent.loop_state.is_some()
            || !self.ai.settings.config.enabled
        {
            self.ai.chat.mention_open = false;
            self.ai.chat.mention_query.clear();
            cx.notify();
            return;
        }
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        // While the @-mention list is open it owns the keys that walk and pick;
        // the box keeps the text either way.
        if self.ai.chat.mention_open {
            let candidate_count = self.ai_mention_candidates().len();
            match keystroke.key.as_str() {
                "escape" => {
                    self.ai.chat.mention_open = false;
                    self.ai.chat.mention_query.clear();
                    self.ai.chat.mention_index = 0;
                    cx.notify();
                    return;
                }
                "up" => {
                    if candidate_count > 0 {
                        self.ai.chat.mention_index =
                            (self.ai.chat.mention_index + candidate_count - 1) % candidate_count;
                    }
                    cx.notify();
                    return;
                }
                "down" => {
                    if candidate_count > 0 {
                        self.ai.chat.mention_index =
                            (self.ai.chat.mention_index + 1) % candidate_count;
                    }
                    cx.notify();
                    return;
                }
                "enter" => {
                    self.select_ai_mention_candidate(cx);
                    return;
                }
                _ => {}
            }
        }

        // Shift+Enter is a newline, which the box takes itself; a bare Enter
        // sends.
        match keystroke.key.as_str() {
            "enter" if !keystroke.modifiers.shift => self.start_ai_ask(cx),
            "escape" => {
                self.ai.chat.mention_open = false;
                self.ai.chat.mention_query.clear();
                self.ai.chat.response_preview = "AI prompt blurred".to_string();
                cx.notify();
            }
            _ => {}
        }
    }

    /// Put text into the prompt, from somewhere other than the box.
    ///
    /// The box owns its own buffer, so a caller that only wrote the draft would
    /// leave the two showing different things.
    pub(in crate::features) fn set_ai_prompt_draft(
        &mut self,
        text: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        let text = text.into();
        self.reset_text_input("ai.chat.prompt", &text, cx);
        self.ai.chat.prompt_draft = text;
        self.sync_ai_mention_from_prompt();
        cx.notify();
    }

    /// Apply an edit from the AI prompt box.
    pub(in crate::features) fn apply_ai_prompt(&mut self, text: String, cx: &mut Context<Self>) {
        if self.ai.chat.pending
            || self.ai.agent.loop_state.is_some()
            || !self.ai.settings.config.enabled
        {
            return;
        }
        self.ai.chat.prompt_draft = text;
        self.sync_ai_mention_from_prompt();
        cx.notify();
    }

    pub(in crate::features) fn drain_ai_chat_events(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.ai.chat.pending {
            return false;
        }
        let mut dirty = false;
        for _ in 0..AI_CHAT_EVENT_DRAIN_LIMIT {
            let Ok(event) = self.ai.chat.rx.try_recv() else {
                break;
            };
            match event {
                AiChatWorkerEvent::Delta {
                    job_id,
                    session_id,
                    text_delta,
                    reasoning_delta,
                } => {
                    if job_id != self.ai.chat.job_id {
                        continue;
                    }
                    dirty = true;
                    if self.ai.chat.response_preview == "Running AI request..." {
                        self.ai.chat.response_preview.clear();
                    }
                    self.ai.chat.response_preview.push_str(&text_delta);
                    self.ai.chat.response_preview =
                        truncate_preview(&self.ai.chat.response_preview, 320);
                    if let Some(assistant_id) = self.ai.chat.streaming_assistant_id.clone() {
                        if let Some(message) = self
                            .ai
                            .chat
                            .messages
                            .iter_mut()
                            .rev()
                            .find(|message| message.id == assistant_id)
                        {
                            message.content.push_str(&text_delta);
                            if let Some(delta) = reasoning_delta.as_ref() {
                                if !delta.trim().is_empty() {
                                    let existing =
                                        message.reasoning_content.take().unwrap_or_default();
                                    message.reasoning_content = Some(format!("{existing}{delta}"));
                                }
                            }
                        }
                    }
                    self.ai.panel.status = if reasoning_delta
                        .as_deref()
                        .is_some_and(|delta| !delta.trim().is_empty())
                    {
                        "AI stream receiving; reasoning captured".to_string()
                    } else {
                        "AI stream receiving".to_string()
                    };
                    self.settings
                        .update_store_status(format!("AI session {session_id} streaming"), true);
                }
                AiChatWorkerEvent::AgentToolCallDelta {
                    job_id,
                    session_id,
                    tool_name,
                    arguments_delta_len,
                } => {
                    if job_id != self.ai.chat.job_id {
                        continue;
                    }
                    dirty = true;
                    let tool_label = tool_name
                        .as_deref()
                        .filter(|name| !name.trim().is_empty())
                        .unwrap_or("tool");
                    self.ai.panel.status = if arguments_delta_len == 0 {
                        format!("AI Agent selected {tool_label}")
                    } else {
                        format!(
                            "AI Agent streaming {tool_label} arguments (+{arguments_delta_len} chars)"
                        )
                    };
                    let step_index = self
                        .ai
                        .agent
                        .steps
                        .last()
                        .map(|step| step.step_index)
                        .unwrap_or(0);
                    self.upsert_ai_agent_step(
                        step_index,
                        AiAgentStepStatus::Tool,
                        format!("Tool {tool_label}"),
                        if arguments_delta_len == 0 {
                            "Provider selected an Agent tool".to_string()
                        } else {
                            format!("Streaming arguments (+{arguments_delta_len} chars)")
                        },
                    );
                    self.settings.update_store_status(
                        format!("AI session {session_id} streaming Agent tool call"),
                        true,
                    );
                }
                AiChatWorkerEvent::AgentBackgroundFinished {
                    job_id,
                    state,
                    result,
                } => {
                    if job_id != self.ai.chat.job_id {
                        continue;
                    }
                    dirty = true;
                    self.ai.chat.cancel = None;
                    let Some(active_state) = self.ai.agent.loop_state.take() else {
                        continue;
                    };
                    if active_state.background_job_id != Some(job_id) {
                        self.ai.agent.loop_state = Some(active_state);
                        continue;
                    }
                    match result {
                        Ok(observation) => {
                            self.ai.panel.status = match observation.exit_code {
                                Some(code) => {
                                    format!("AI Agent background command exited with {code}")
                                }
                                None => "AI Agent background command completed".to_string(),
                            };
                            self.upsert_ai_agent_step(
                                state.step_index,
                                AiAgentStepStatus::Completed,
                                "Observed",
                                observation_summary(&observation),
                            );
                            self.start_ai_agent_continuation(state, observation, cx);
                        }
                        Err(error) => {
                            self.ai.panel.status =
                                format!("AI Agent background command failed: {error}");
                            self.ai.chat.response_preview = self.ai.panel.status.clone();
                            self.upsert_ai_agent_step(
                                state.step_index,
                                AiAgentStepStatus::Failed,
                                "Failed",
                                truncate_preview(&error, 140),
                            );
                            self.settings
                                .update_store_status(self.ai.panel.status.clone(), false);
                        }
                    }
                }
                AiChatWorkerEvent::Finished(event) => {
                    if event.job_id != self.ai.chat.job_id {
                        continue;
                    }
                    dirty = true;
                    self.ai.chat.pending = false;
                    self.ai.chat.cancel = None;
                    match event.result {
                        Ok(output) => {
                            let command_count = output.command_cards.len();
                            self.ai.chat.response_preview = if output.text.trim().is_empty() {
                                "AI returned an empty response".to_string()
                            } else {
                                truncate_preview(&output.text, 320)
                            };
                            let mode_label = if output.mode == AiMode::Agent {
                                "AI Agent"
                            } else {
                                "AI Ask"
                            };
                            self.ai.panel.status = format!(
                                "{mode_label} completed; {} command card(s) parsed",
                                command_count
                            );
                            if output.reasoning.is_some() {
                                self.ai.panel.status.push_str("; reasoning captured");
                            }
                            if let Some(note) = output.approval_note.as_deref() {
                                self.ai.panel.status.push_str("; ");
                                self.ai.panel.status.push_str(note);
                            }
                            let auto_execute_first = output.auto_execute_first;
                            let agent_step_index = self
                                .ai
                                .agent
                                .steps
                                .last()
                                .map(|step| step.step_index)
                                .unwrap_or(0);
                            if output.mode == AiMode::Agent {
                                let (step_status, step_title) = if command_count == 0 {
                                    (AiAgentStepStatus::Completed, "Final Answer")
                                } else if auto_execute_first {
                                    (AiAgentStepStatus::Running, "Auto Execute")
                                } else {
                                    (AiAgentStepStatus::NeedsApproval, "Needs Approval")
                                };
                                self.upsert_ai_agent_step(
                                    agent_step_index,
                                    step_status,
                                    step_title,
                                    truncate_preview(&output.text, 140),
                                );
                            }
                            self.ai.chat.command_cards = output.command_cards.clone();
                            if let Some(assistant_id) = self.ai.chat.streaming_assistant_id.take() {
                                if let Some(message) = self
                                    .ai
                                    .chat
                                    .messages
                                    .iter_mut()
                                    .rev()
                                    .find(|message| message.id == assistant_id)
                                {
                                    if !output.text.trim().is_empty() {
                                        message.content = output.text.clone();
                                    } else if message.content.trim().is_empty() {
                                        message.content =
                                            "AI returned an empty response".to_string();
                                    }
                                    message.reasoning_content = output.reasoning.clone();
                                    message.command_cards = output.command_cards.clone();
                                }
                            }
                            self.settings.update_store_status(
                                format!("AI session {} updated", event.session_id),
                                true,
                            );
                            self.reset_text_input("ai.chat.prompt", "", cx);
                            self.ai.chat.prompt_draft.clear();
                            self.refresh_ai_usage_counts(cx);
                            if output.mode == AiMode::Agent {
                                if command_count == 0 {
                                    self.ai.agent.loop_state = None;
                                    self.ai.agent.task_prompt = None;
                                } else if !auto_execute_first {
                                    self.ai.panel.status.push_str("; awaiting command approval");
                                }
                            }
                            if auto_execute_first && !self.ai.chat.command_cards.is_empty() {
                                self.run_ai_command_card(0, cx);
                            }
                        }
                        Err(error) => {
                            self.ai.chat.response_preview = format!("AI request failed: {error}");
                            self.ai.chat.command_cards.clear();
                            self.ai.panel.status = self.ai.chat.response_preview.clone();
                            if let Some(assistant_id) = self.ai.chat.streaming_assistant_id.take() {
                                if let Some(message) = self
                                    .ai
                                    .chat
                                    .messages
                                    .iter_mut()
                                    .rev()
                                    .find(|message| message.id == assistant_id)
                                {
                                    message.content = format!("AI request failed: {error}");
                                }
                            }
                            if self.ai.agent.task_prompt.is_some() {
                                let step_index = self
                                    .ai
                                    .agent
                                    .steps
                                    .last()
                                    .map(|step| step.step_index)
                                    .unwrap_or(0);
                                self.upsert_ai_agent_step(
                                    step_index,
                                    AiAgentStepStatus::Failed,
                                    "Failed",
                                    truncate_preview(&error, 140),
                                );
                            }
                            self.settings
                                .update_store_status(self.ai.panel.status.clone(), false);
                        }
                    }
                }
            }
        }
        let _ = cx;
        dirty
    }
}
