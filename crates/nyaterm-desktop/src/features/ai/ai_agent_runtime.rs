use super::*;

impl NyaTermApp {
    pub(in crate::features) fn upsert_ai_agent_step(
        &mut self,
        step_index: u16,
        status: AiAgentStepStatus,
        title: impl Into<String>,
        detail: impl Into<String>,
    ) {
        let title = title.into();
        let detail = detail.into();
        // Infer Tauri AgentStepView sections from existing call-site titles.
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
            .ai_agent_steps
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
            let command = if looks_like_command && !detail.trim().is_empty() {
                Some(detail.clone())
            } else {
                None
            };
            let observation = if looks_like_observation && !detail.trim().is_empty() {
                Some(detail.clone())
            } else {
                None
            };
            let thought = if looks_like_thought && !detail.trim().is_empty() {
                Some(detail.clone())
            } else {
                None
            };
            self.ai_agent_steps.push(AiAgentStepView {
                step_index,
                status,
                title,
                detail,
                thought,
                command,
                observation,
            });
        }
        let overflow = self.ai_agent_steps.len().saturating_sub(16);
        if overflow > 0 {
            let removed: Vec<u16> = self
                .ai_agent_steps
                .iter()
                .take(overflow)
                .map(|step| step.step_index)
                .collect();
            self.ai_agent_steps.drain(..overflow);
            for idx in removed {
                self.ai_agent_thought_expanded.remove(&idx);
                self.ai_agent_output_expanded.remove(&idx);
            }
        }
    }

    pub(in crate::features) fn toggle_ai_agent_thought_expanded(
        &mut self,
        step_index: u16,
        cx: &mut Context<Self>,
    ) {
        if self.ai_agent_thought_expanded.contains(&step_index) {
            self.ai_agent_thought_expanded.remove(&step_index);
        } else {
            self.ai_agent_thought_expanded.insert(step_index);
        }
        cx.notify();
    }

    pub(in crate::features) fn toggle_ai_agent_output_expanded(
        &mut self,
        step_index: u16,
        cx: &mut Context<Self>,
    ) {
        if self.ai_agent_output_expanded.contains(&step_index) {
            self.ai_agent_output_expanded.remove(&step_index);
        } else {
            self.ai_agent_output_expanded.insert(step_index);
        }
        cx.notify();
    }

    pub(in crate::features) fn record_ai_command_card_audit(
        &mut self,
        card: &AiCommandCard,
        execute: bool,
        inserted_to_terminal: bool,
    ) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| {
            store
                .append_ai_audit(AppendAiAuditRequest {
                    connection_id: self.active_session_id.clone(),
                    action: if execute {
                        "ai.command_card_run".to_string()
                    } else {
                        "ai.command_card_insert".to_string()
                    },
                    user_input: Some(self.ai_response_preview.clone()),
                    generated_command: Some(card.command.clone()),
                    risk_level: card.risk_level.clone(),
                    inserted_to_terminal,
                    executed: execute,
                    blocked: false,
                })
                .map(|_| ())
        }) {
            Ok(()) => {
                self.refresh_ai_usage_counts();
            }
            Err(error) => {
                self.store_status.message = format!("AI audit save failed: {error}");
                self.store_status.ready = false;
            }
        }
    }

    pub(in crate::features) fn begin_ai_agent_observation(
        &mut self,
        command: &str,
    ) -> Result<Option<String>, String> {
        let Some(terminal_session_id) = self.active_session_id.clone() else {
            return Ok(None);
        };
        let task_prompt = self
            .ai_agent_task_prompt
            .clone()
            .unwrap_or_else(|| self.ai_response_preview.clone());
        let max_steps = self.ai_settings.max_agent_steps.unwrap_or(10).max(1);
        let step_index = self.ai_agent_step_index;
        if step_index.saturating_add(1) >= max_steps {
            self.ai_agent_loop = None;
            self.ai_status =
                format!("AI Agent reached max step limit ({max_steps}); review terminal output");
            return Ok(None);
        }
        self.ai_agent_step_index = self.ai_agent_step_index.saturating_add(1);
        let now = Instant::now();
        let timeout = self
            .ai_settings
            .agent_step_timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(AI_AGENT_DEFAULT_STEP_TIMEOUT);
        let profile = self.active_ai_execution_profile();
        if profile == AiExecutionProfile::Disabled {
            return Err("AI Agent command execution is disabled for this session".to_string());
        }
        let marker_id = format!("agent-{}", uuid());
        let (marker_id, wrapped_command) =
            match build_agent_capture_command(profile, &marker_id, command.trim()) {
                Some(wrapped) => {
                    self.ai_agent_capture.register(marker_id.clone());
                    (Some(marker_id), Some(wrapped))
                }
                None => (None, None),
            };
        let output_start_len = self
            .terminal_buffer_text_for_session(&terminal_session_id)
            .len();
        self.ai_agent_loop = Some(AiAgentLoopState {
            ai_session_id: self.ai_chat_session_id.clone(),
            terminal_session_id,
            task_prompt,
            command: command.trim().to_string(),
            marker_id,
            background_job_id: None,
            step_index,
            max_steps,
            output_start_len,
            started_at: now,
            min_wait_until: now + AI_AGENT_OBSERVATION_MIN_WAIT,
            timeout_at: now + timeout,
            last_seen_len: output_start_len,
            stable_since: now,
        });
        self.sync_session_event_bridge_policy();
        self.ai_status = format!(
            "AI Agent observing command output for step {}/{}",
            step_index + 1,
            max_steps
        );
        self.upsert_ai_agent_step(
            step_index,
            AiAgentStepStatus::Running,
            "Running",
            truncate_preview(command.trim(), 140),
        );
        Ok(wrapped_command)
    }

    pub(in crate::features) fn begin_ai_agent_background_execution(
        &mut self,
        command: &str,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let Some(terminal_session_id) = self.active_session_id.clone() else {
            return Err(
                "Start a terminal session before using AI Agent background execution".to_string(),
            );
        };
        let session = self
            .session_manager
            .list_sessions()
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|session| session.id == terminal_session_id)
            .ok_or_else(|| "Active terminal session was not found".to_string())?;
        let (target, target_label) = match session.kind {
            SessionKind::Ssh => {
                let config = self
                    .active_ssh_config
                    .clone()
                    .ok_or_else(|| "Active SSH session is missing its exec config".to_string())?;
                (AiAgentBackgroundTarget::Ssh(config), "SSH")
            }
            SessionKind::LocalPty => (
                AiAgentBackgroundTarget::Local {
                    working_dir: session.working_dir.clone(),
                },
                "local",
            ),
            SessionKind::Telnet | SessionKind::RawTcp | SessionKind::Serial => {
                return Err(format!(
                    "AI Agent background execution is not supported for {:?} sessions",
                    session.kind
                ));
            }
        };
        let task_prompt = self
            .ai_agent_task_prompt
            .clone()
            .unwrap_or_else(|| self.ai_response_preview.clone());
        let max_steps = self.ai_settings.max_agent_steps.unwrap_or(10).max(1);
        let step_index = self.ai_agent_step_index;
        if step_index.saturating_add(1) >= max_steps {
            self.ai_agent_loop = None;
            return Err(format!(
                "AI Agent reached max step limit ({max_steps}); review terminal output"
            ));
        }
        self.ai_agent_step_index = self.ai_agent_step_index.saturating_add(1);
        let now = Instant::now();
        let timeout = self
            .ai_settings
            .agent_step_timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(AI_AGENT_DEFAULT_STEP_TIMEOUT);
        let (job_id, cancel) = self.begin_ai_chat_job();
        let output_start_len = self
            .terminal_buffer_text_for_session(&terminal_session_id)
            .len();
        let state = AiAgentLoopState {
            ai_session_id: self.ai_chat_session_id.clone(),
            terminal_session_id,
            task_prompt,
            command: command.trim().to_string(),
            marker_id: None,
            background_job_id: Some(job_id),
            step_index,
            max_steps,
            output_start_len,
            started_at: now,
            min_wait_until: now,
            timeout_at: now + timeout,
            last_seen_len: output_start_len,
            stable_since: now,
        };
        self.ai_agent_loop = Some(state.clone());
        self.ai_status = format!(
            "AI Agent running {target_label} background command for step {}/{}",
            step_index + 1,
            max_steps
        );
        self.upsert_ai_agent_step(
            step_index,
            AiAgentStepStatus::Running,
            format!("{target_label} background"),
            truncate_preview(command.trim(), 140),
        );
        let tx = self.ai_chat_tx.clone();
        let command = state.command.clone();
        std::thread::spawn(move || {
            let started = Instant::now();
            let result = if ai_job_cancelled(&cancel) {
                Err("AI Agent background command cancelled".to_string())
            } else {
                match target {
                    AiAgentBackgroundTarget::Ssh(config) => SshProcessService::new(config)
                        .run_command(&command, timeout)
                        .map(|output| remote_command_observation(output, started))
                        .map_err(|error| error.to_string()),
                    AiAgentBackgroundTarget::Local { working_dir } => {
                        run_local_command(&command, working_dir, timeout)
                            .map(|output| remote_command_observation(output, started))
                            .map_err(|error| error.to_string())
                    }
                }
            };
            if !ai_job_cancelled(&cancel) {
                let _ = tx.send(AiChatWorkerEvent::AgentBackgroundFinished {
                    job_id,
                    state,
                    result,
                });
            }
        });
        cx.notify();
        Ok(())
    }

    pub(in crate::features) fn drive_ai_agent_loop(&mut self, cx: &mut Context<Self>) -> bool {
        if self.ai_chat_pending {
            return false;
        }
        let Some(state) = self.ai_agent_loop.as_mut() else {
            return false;
        };
        if state.background_job_id.is_some() {
            return false;
        }
        if self.active_session_id.as_deref() != Some(state.terminal_session_id.as_str()) {
            let step_index = state.step_index;
            self.ai_agent_loop = None;
            self.ai_status =
                "AI Agent loop stopped because the terminal session changed".to_string();
            self.upsert_ai_agent_step(
                step_index,
                AiAgentStepStatus::Failed,
                "Stopped",
                "Terminal session changed",
            );
            let _ = cx;
            return true;
        }

        let now = Instant::now();
        let current_len = self
            .terminal_views
            .get(&state.terminal_session_id)
            .map(|view| view.output.len())
            .unwrap_or_else(|| self.terminal_output.len());
        if current_len != state.last_seen_len {
            state.last_seen_len = current_len;
            state.stable_since = now;
            return false;
        }
        if now < state.min_wait_until {
            return false;
        }
        let has_observed_output = current_len > state.output_start_len;
        let output_is_quiet = now.duration_since(state.stable_since) >= AI_AGENT_OBSERVATION_QUIET;
        let timed_out = now >= state.timeout_at;
        if timed_out && let Some(marker_id) = state.marker_id.clone() {
            let timeout_ms = now
                .duration_since(state.started_at)
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX);
            let command = state.command.clone();
            self.ai_agent_capture.cancel(&marker_id);
            self.sync_session_event_bridge_policy();
            let Some(state) = self.ai_agent_loop.take() else {
                return false;
            };
            let observation = CommandObservation {
                output: "(command timed out; capture markers were not detected in terminal output)"
                    .to_string(),
                exit_code: None,
                duration_ms: timeout_ms,
            };
            self.ai_status = format!("AI Agent command capture timed out: {command}");
            self.upsert_ai_agent_step(
                state.step_index,
                AiAgentStepStatus::Failed,
                "Timed out",
                observation_summary(&observation),
            );
            self.start_ai_agent_continuation(state, observation, cx);
            return true;
        }
        if !timed_out && (!has_observed_output || !output_is_quiet) {
            return false;
        }
        if state.marker_id.is_some() {
            return false;
        }

        let Some(state) = self.ai_agent_loop.take() else {
            return false;
        };
        let terminal_output = self.terminal_buffer_text_for_session(&state.terminal_session_id);
        let output = terminal_output
            .get(state.output_start_len..)
            .unwrap_or_default()
            .to_string();
        let duration_ms = now
            .duration_since(state.started_at)
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let observation = CommandObservation {
            output,
            exit_code: None,
            duration_ms,
        };
        self.upsert_ai_agent_step(
            state.step_index,
            AiAgentStepStatus::Completed,
            "Observed",
            observation_summary(&observation),
        );
        self.start_ai_agent_continuation(state, observation, cx);
        true
    }

    pub(in crate::features) fn handle_ai_agent_captured_output(
        &mut self,
        captured: AgentCapturedOutput,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.ai_agent_loop.take() else {
            return;
        };
        if state.marker_id.as_deref() != Some(captured.marker_id.as_str()) {
            self.ai_agent_loop = Some(state);
            return;
        }
        let observation = CommandObservation {
            output: captured.output,
            exit_code: captured.exit_code,
            duration_ms: captured.duration_ms,
        };
        self.ai_status = match observation.exit_code {
            Some(code) => format!("AI Agent captured command output with exit code {code}"),
            None => "AI Agent captured command output".to_string(),
        };
        self.upsert_ai_agent_step(
            state.step_index,
            AiAgentStepStatus::Completed,
            "Observed",
            observation_summary(&observation),
        );
        self.start_ai_agent_continuation(state, observation, cx);
    }

    pub(in crate::features) fn note_ai_agent_output_discontinuity(
        &mut self,
        session_id: &str,
        dropped_bytes: usize,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self
            .ai_agent_loop
            .as_ref()
            .is_some_and(|state| state.terminal_session_id == session_id)
        {
            return false;
        }
        let Some(state) = self.ai_agent_loop.take() else {
            return false;
        };
        if let Some(marker_id) = state.marker_id.as_deref() {
            self.ai_agent_capture.cancel(marker_id);
            self.sync_session_event_bridge_policy();
        }
        let duration_ms = state
            .started_at
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let observation = CommandObservation {
            output: format!(
                "(terminal output dropped {dropped_bytes} byte(s); command output is incomplete)"
            ),
            exit_code: None,
            duration_ms,
        };
        self.ai_status =
            "AI Agent command observation stopped because terminal output was dropped".to_string();
        self.upsert_ai_agent_step(
            state.step_index,
            AiAgentStepStatus::Failed,
            "Output dropped",
            observation_summary(&observation),
        );
        self.start_ai_agent_continuation(state, observation, cx);
        true
    }

    fn active_ai_execution_profile(&self) -> AiExecutionProfile {
        if self.active_ai_execution_profile != AiExecutionProfile::Auto {
            return self.active_ai_execution_profile;
        }
        let Some(session_id) = self.active_session_id.as_deref() else {
            return AiExecutionProfile::SendOnly;
        };
        self.session_manager
            .list_sessions()
            .ok()
            .and_then(|sessions| {
                sessions
                    .into_iter()
                    .find(|session| session.id == session_id)
            })
            .map(|session| match session.kind {
                SessionKind::LocalPty
                | SessionKind::Ssh
                | SessionKind::Telnet
                | SessionKind::RawTcp => AiExecutionProfile::Posix,
                SessionKind::Serial => AiExecutionProfile::SendOnly,
            })
            .unwrap_or(AiExecutionProfile::SendOnly)
    }

    pub(in crate::features) fn start_ai_agent_continuation(
        &mut self,
        state: AiAgentLoopState,
        observation: CommandObservation,
        cx: &mut Context<Self>,
    ) {
        if self.ai_chat_pending {
            self.ai_agent_loop = Some(state);
            return;
        }
        let observation_message =
            build_observation_message(&observation, &state.command, &self.settings.language);
        let settings = self.ai_settings.clone();
        let terminal_session_id = state.terminal_session_id.clone();
        let request = AiChatRequest {
            stream_id: None,
            session_id: Some(state.ai_session_id.clone()),
            connection_id: Some(terminal_session_id.clone()),
            terminal_session_id: Some(terminal_session_id.clone()),
            mode: AiMode::Agent,
            model_id: settings.default_model_id.clone(),
            model_name: None,
            action: AiAction::GenerateCommand,
            user_input: format!(
                "Continue the same Agent task.\n\nOriginal task:\n{}\n\n{}",
                state.task_prompt, observation_message
            ),
            context: self.ai_terminal_context_for_session(Some(&terminal_session_id)),
            options: Default::default(),
        };
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        let tx = self.ai_chat_tx.clone();
        let session_id = state.ai_session_id;
        let (job_id, cancel) = self.begin_ai_chat_job();

        self.ai_chat_pending = true;
        self.ai_response_preview = format!(
            "Running AI Agent continuation step {}/{}...",
            state.step_index + 2,
            state.max_steps
        );
        self.ai_command_cards.clear();
        self.ai_status = self.ai_response_preview.clone();
        self.upsert_ai_agent_step(
            state.step_index.saturating_add(1),
            AiAgentStepStatus::Planning,
            "Planning",
            "Continuing from the latest command observation",
        );
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
}
