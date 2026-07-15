use super::*;

const AI_CHAT_EVENT_DRAIN_LIMIT: usize = 256;

impl NyaTermApp {
    pub(in crate::features) fn begin_ai_chat_job(&mut self) -> (u64, Arc<AtomicBool>) {
        self.ai_chat_job_id = self.ai_chat_job_id.wrapping_add(1).max(1);
        let cancel = Arc::new(AtomicBool::new(false));
        self.ai_chat_cancel = Some(cancel.clone());
        (self.ai_chat_job_id, cancel)
    }

    pub(in crate::features) fn cancel_ai_chat(&mut self, cx: &mut Context<Self>) {
        if let Some(cancel) = self.ai_chat_cancel.as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.ai_chat_job_id = self.ai_chat_job_id.wrapping_add(1).max(1);
        self.ai_chat_pending = false;
        self.ai_chat_cancel = None;
        let cancelled_step = self
            .ai_agent_loop
            .as_ref()
            .map(|state| state.step_index)
            .or_else(|| self.ai_agent_steps.last().map(|step| step.step_index));
        if let Some(state) = self.ai_agent_loop.take()
            && let Some(marker_id) = state.marker_id.as_deref()
        {
            self.ai_agent_capture.cancel(marker_id);
        }
        self.ai_agent_capture = AgentOutputCaptureProcessor::new();
        self.sync_session_event_bridge_policy();
        self.ai_agent_task_prompt = None;
        self.ai_command_cards.clear();
        self.ai_response_preview = "AI request cancelled".to_string();
        if let Some(assistant_id) = self.ai_streaming_assistant_id.take() {
            if let Some(message) = self
                .ai_chat_messages
                .iter_mut()
                .rev()
                .find(|message| message.id == assistant_id)
            {
                if message.content.trim().is_empty() {
                    message.content = "AI request cancelled".to_string();
                }
            }
        }
        self.ai_status = "AI request cancelled".to_string();
        if let Some(step_index) = cancelled_step {
            self.upsert_ai_agent_step(
                step_index,
                AiAgentStepStatus::Cancelled,
                "Cancelled",
                "AI Agent request was cancelled",
            );
        }
        self.store_status.message = self.ai_status.clone();
        cx.notify();
    }

    pub(in crate::features) fn start_ai_ask(&mut self, cx: &mut Context<Self>) {
        if self.ai_chat_pending {
            self.ai_response_preview = "AI request already running".to_string();
            cx.notify();
            return;
        }
        if self.ai_agent_loop.is_some() {
            self.ai_response_preview = "AI Agent step already running".to_string();
            self.ai_status = self.ai_response_preview.clone();
            cx.notify();
            return;
        }
        let prompt = self.ai_prompt_draft.trim().to_string();
        if prompt.is_empty() {
            self.ai_response_preview = "Enter a prompt first".to_string();
            cx.notify();
            return;
        }
        if !self.ai_settings.enabled {
            self.ai_response_preview = "AI assistant is disabled".to_string();
            cx.notify();
            return;
        }

        let settings = self.ai_settings.clone();
        let mode = settings.default_mode.clone();
        if mode == AiMode::Agent && self.active_session_id.is_none() {
            self.ai_response_preview =
                "Start a terminal session before running Agent mode".to_string();
            self.ai_status = self.ai_response_preview.clone();
            cx.notify();
            return;
        }
        let prepared_request = self.ai_prepared_request.clone();
        let action = prepared_request
            .as_ref()
            .map(|request| request.action.clone())
            .unwrap_or(AiAction::GenerateCommand);
        let context = prepared_request
            .as_ref()
            .map(|request| request.context.clone())
            .unwrap_or_else(|| self.ai_terminal_context());
        let source_label = prepared_request
            .as_ref()
            .map(|request| request.source_label.clone());
        let session_id = self.ai_chat_session_id.clone();
        let request = AiChatRequest {
            stream_id: None,
            session_id: Some(session_id.clone()),
            connection_id: self.active_session_id.clone(),
            terminal_session_id: self.active_session_id.clone(),
            mode: mode.clone(),
            model_id: settings.default_model_id.clone(),
            model_name: None,
            action,
            user_input: prompt.clone(),
            context,
            options: Default::default(),
        };
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        let tx = self.ai_chat_tx.clone();
        let (job_id, cancel) = self.begin_ai_chat_job();

        if mode == AiMode::Agent {
            self.ai_agent_task_prompt = Some(request.user_input.clone());
            self.ai_agent_step_index = 0;
            self.ai_agent_steps.clear();
            self.ai_agent_thought_expanded.clear();
            self.ai_agent_output_expanded.clear();
            self.upsert_ai_agent_step(
                0,
                AiAgentStepStatus::Planning,
                "Planning",
                truncate_preview(&request.user_input, 120),
            );
        } else {
            self.ai_agent_task_prompt = None;
            self.ai_agent_step_index = 0;
            self.ai_agent_loop = None;
            self.ai_agent_steps.clear();
            self.ai_agent_thought_expanded.clear();
            self.ai_agent_output_expanded.clear();
        }
        self.ai_chat_pending = true;
        self.ai_response_preview = if mode == AiMode::Agent {
            "Running AI Agent step...".to_string()
        } else {
            "Running AI request...".to_string()
        };
        self.ai_command_cards.clear();
        let now = now_rfc3339();
        let assistant_id = format!("assistant-{}", uuid());
        self.ai_chat_messages.push(AiMessage {
            id: format!("user-{}", uuid()),
            session_id: self.ai_chat_session_id.clone(),
            role: AiMessageRole::User,
            content: prompt.clone(),
            created_at: now.clone(),
            reasoning_content: None,
            command_cards: Vec::new(),
        });
        self.ai_chat_messages.push(AiMessage {
            id: assistant_id.clone(),
            session_id: self.ai_chat_session_id.clone(),
            role: AiMessageRole::Assistant,
            content: String::new(),
            created_at: now,
            reasoning_content: None,
            command_cards: Vec::new(),
        });
        self.ai_streaming_assistant_id = Some(assistant_id);
        self.ai_status = if mode == AiMode::Agent {
            "AI Agent step started".to_string()
        } else if let Some(source_label) = source_label.as_ref() {
            format!("AI file action started: {source_label}")
        } else {
            "AI Ask request started".to_string()
        };
        self.ai_prepared_request = None;
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
        self.ai_terminal_context_for_session(self.active_session_id.as_deref())
    }

    pub(in crate::features) fn ai_terminal_context_for_session(
        &self,
        session_id: Option<&str>,
    ) -> AiContext {
        let metadata = session_id.and_then(|session_id| self.session_metadata.get(session_id));
        let ssh = match metadata.map(|metadata| &metadata.launch_config) {
            Some(SessionLaunchConfig::Ssh(config)) => Some(config),
            _ if session_id == self.active_session_id.as_deref() => self.active_ssh_config.as_ref(),
            _ => None,
        };
        let session = session_id.and_then(|session_id| {
            self.session_manager
                .list_sessions()
                .ok()
                .and_then(|sessions| {
                    sessions
                        .into_iter()
                        .find(|session| session.id == session_id)
                })
        });
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
            recent_output: recent_terminal_output(&recent_output, 80),
            selected_text: String::new(),
            input_buffer: String::new(),
        }
    }

    pub(in crate::features) fn handle_ai_prompt_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "enter" => self.start_ai_ask(cx),
            "backspace" => {
                self.ai_prompt_draft.pop();
                cx.notify();
            }
            "escape" => {
                self.ai_response_preview = "AI prompt blurred".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.ai_prompt_draft.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn drain_ai_chat_events(&mut self, cx: &mut Context<Self>) -> bool {
        let mut dirty = false;
        for _ in 0..AI_CHAT_EVENT_DRAIN_LIMIT {
            let Ok(event) = self.ai_chat_rx.try_recv() else {
                break;
            };
            match event {
                AiChatWorkerEvent::Delta {
                    job_id,
                    session_id,
                    text_delta,
                    reasoning_delta,
                } => {
                    if job_id != self.ai_chat_job_id {
                        continue;
                    }
                    dirty = true;
                    if self.ai_response_preview == "Running AI request..." {
                        self.ai_response_preview.clear();
                    }
                    self.ai_response_preview.push_str(&text_delta);
                    self.ai_response_preview = truncate_preview(&self.ai_response_preview, 320);
                    if let Some(assistant_id) = self.ai_streaming_assistant_id.clone() {
                        if let Some(message) = self
                            .ai_chat_messages
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
                    self.ai_status = if reasoning_delta
                        .as_deref()
                        .is_some_and(|delta| !delta.trim().is_empty())
                    {
                        "AI stream receiving; reasoning captured".to_string()
                    } else {
                        "AI stream receiving".to_string()
                    };
                    self.store_status.message = format!("AI session {session_id} streaming");
                    self.store_status.ready = true;
                }
                AiChatWorkerEvent::AgentToolCallDelta {
                    job_id,
                    session_id,
                    tool_name,
                    arguments_delta_len,
                } => {
                    if job_id != self.ai_chat_job_id {
                        continue;
                    }
                    dirty = true;
                    let tool_label = tool_name
                        .as_deref()
                        .filter(|name| !name.trim().is_empty())
                        .unwrap_or("tool");
                    self.ai_status = if arguments_delta_len == 0 {
                        format!("AI Agent selected {tool_label}")
                    } else {
                        format!(
                            "AI Agent streaming {tool_label} arguments (+{arguments_delta_len} chars)"
                        )
                    };
                    let step_index = self
                        .ai_agent_steps
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
                    self.store_status.message =
                        format!("AI session {session_id} streaming Agent tool call");
                    self.store_status.ready = true;
                }
                AiChatWorkerEvent::AgentBackgroundFinished {
                    job_id,
                    state,
                    result,
                } => {
                    if job_id != self.ai_chat_job_id {
                        continue;
                    }
                    dirty = true;
                    self.ai_chat_cancel = None;
                    let Some(active_state) = self.ai_agent_loop.take() else {
                        continue;
                    };
                    if active_state.background_job_id != Some(job_id) {
                        self.ai_agent_loop = Some(active_state);
                        continue;
                    }
                    match result {
                        Ok(observation) => {
                            self.ai_status = match observation.exit_code {
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
                            self.ai_status = format!("AI Agent background command failed: {error}");
                            self.ai_response_preview = self.ai_status.clone();
                            self.upsert_ai_agent_step(
                                state.step_index,
                                AiAgentStepStatus::Failed,
                                "Failed",
                                truncate_preview(&error, 140),
                            );
                            self.store_status.message = self.ai_status.clone();
                            self.store_status.ready = false;
                        }
                    }
                }
                AiChatWorkerEvent::Finished(event) => {
                    if event.job_id != self.ai_chat_job_id {
                        continue;
                    }
                    dirty = true;
                    self.ai_chat_pending = false;
                    self.ai_chat_cancel = None;
                    match event.result {
                        Ok(output) => {
                            let command_count = output.command_cards.len();
                            self.ai_response_preview = if output.text.trim().is_empty() {
                                "AI returned an empty response".to_string()
                            } else {
                                truncate_preview(&output.text, 320)
                            };
                            let mode_label = if output.mode == AiMode::Agent {
                                "AI Agent"
                            } else {
                                "AI Ask"
                            };
                            self.ai_status = format!(
                                "{mode_label} completed; {} command card(s) parsed",
                                command_count
                            );
                            if output.reasoning.is_some() {
                                self.ai_status.push_str("; reasoning captured");
                            }
                            if let Some(note) = output.approval_note.as_deref() {
                                self.ai_status.push_str("; ");
                                self.ai_status.push_str(note);
                            }
                            let auto_execute_first = output.auto_execute_first;
                            let agent_step_index = self
                                .ai_agent_steps
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
                            self.ai_command_cards = output.command_cards.clone();
                            if let Some(assistant_id) = self.ai_streaming_assistant_id.take() {
                                if let Some(message) = self
                                    .ai_chat_messages
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
                            self.store_status.message =
                                format!("AI session {} updated", event.session_id);
                            self.store_status.ready = true;
                            self.ai_prompt_draft.clear();
                            self.refresh_ai_usage_counts();
                            if output.mode == AiMode::Agent {
                                if command_count == 0 {
                                    self.ai_agent_loop = None;
                                    self.ai_agent_task_prompt = None;
                                } else if !auto_execute_first {
                                    self.ai_status.push_str("; awaiting command approval");
                                }
                            }
                            if auto_execute_first && !self.ai_command_cards.is_empty() {
                                self.run_ai_command_card(0, cx);
                            }
                        }
                        Err(error) => {
                            self.ai_response_preview = format!("AI request failed: {error}");
                            self.ai_command_cards.clear();
                            self.ai_status = self.ai_response_preview.clone();
                            if let Some(assistant_id) = self.ai_streaming_assistant_id.take() {
                                if let Some(message) = self
                                    .ai_chat_messages
                                    .iter_mut()
                                    .rev()
                                    .find(|message| message.id == assistant_id)
                                {
                                    message.content = format!("AI request failed: {error}");
                                }
                            }
                            if self.ai_agent_task_prompt.is_some() {
                                let step_index = self
                                    .ai_agent_steps
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
                            self.store_status.message = self.ai_status.clone();
                            self.store_status.ready = false;
                        }
                    }
                }
            }
        }
        let _ = cx;
        dirty
    }
}
