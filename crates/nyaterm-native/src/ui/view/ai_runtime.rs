use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn update_ai_profile(
        &mut self,
        profile_id: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.ai_settings.active_profile_id = profile_id.to_string();
        self.sync_ai_drafts_from_active_profile();
        self.ai_status = format!("AI provider set to {profile_id}; save to persist");
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_ai_enabled(&mut self, cx: &mut Context<Self>) {
        self.ai_settings.enabled = !self.ai_settings.enabled;
        self.ai_status = if self.ai_settings.enabled {
            "AI enabled; save to persist"
        } else {
            "AI disabled; save to persist"
        }
        .to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn set_ai_mode(&mut self, mode: AiMode, cx: &mut Context<Self>) {
        self.ai_settings.default_mode = mode;
        self.ai_status = "AI mode edited; save to persist".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn set_ai_command_mode(
        &mut self,
        mode: AgentCommandExecutionMode,
        cx: &mut Context<Self>,
    ) {
        self.ai_settings.agent_command_execution_mode = mode;
        self.ai_status = "Agent command policy edited; save to persist".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_ai_background_execution(&mut self, cx: &mut Context<Self>) {
        self.ai_settings.agent_background_execution_enabled =
            !self.ai_settings.agent_background_execution_enabled;
        self.ai_status = if self.ai_settings.agent_background_execution_enabled {
            "Agent background execution enabled; save to persist"
        } else {
            "Agent background execution disabled; save to persist"
        }
        .to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_ai_redaction(&mut self, cx: &mut Context<Self>) {
        self.ai_settings.redaction_enabled = !self.ai_settings.redaction_enabled;
        self.ai_status = "AI redaction edited; save to persist".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_ai_allow_save_command(&mut self, cx: &mut Context<Self>) {
        self.ai_settings.allow_save_command = !self.ai_settings.allow_save_command;
        self.ai_status = "AI command saving edited; save to persist".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_ai_record_history(&mut self, cx: &mut Context<Self>) {
        self.ai_settings.record_history = !self.ai_settings.record_history;
        self.ai_status = "AI history recording edited; save to persist".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn adjust_ai_context_line_limit(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let current = self.ai_settings.context_line_limit as i32;
        self.ai_settings.context_line_limit = (current + delta).clamp(50, 500) as u32;
        self.ai_status = "AI context line limit edited; save to persist".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn adjust_ai_timeout_ms(&mut self, delta: i64, cx: &mut Context<Self>) {
        let current = self.ai_settings.timeout_ms as i64;
        self.ai_settings.timeout_ms = (current + delta).clamp(5_000, 300_000) as u64;
        self.ai_status = "AI timeout edited; save to persist".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn adjust_ai_agent_steps(
        &mut self,
        delta: i16,
        cx: &mut Context<Self>,
    ) {
        let current = self.ai_settings.max_agent_steps.unwrap_or(10) as i16;
        self.ai_settings.max_agent_steps = Some((current + delta).clamp(1, 50) as u16);
        self.ai_status = "AI Agent max steps edited; save to persist".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn adjust_ai_agent_step_timeout_ms(
        &mut self,
        delta: i64,
        cx: &mut Context<Self>,
    ) {
        let current = self.ai_settings.agent_step_timeout_ms.unwrap_or(30_000) as i64;
        self.ai_settings.agent_step_timeout_ms =
            Some((current + delta).clamp(5_000, 120_000) as u64);
        self.ai_status = "AI Agent step timeout edited; save to persist".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn adjust_ai_terminal_output_lines(
        &mut self,
        delta: i16,
        cx: &mut Context<Self>,
    ) {
        let current = self.ai_settings.terminal_output_lines as i16;
        self.ai_settings.terminal_output_lines = (current + delta).clamp(0, 100) as u16;
        self.ai_status = "AI terminal output lines edited; save to persist".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn adjust_ai_file_size_mb(
        &mut self,
        delta: i64,
        cx: &mut Context<Self>,
    ) {
        let mb = 1024 * 1024;
        let current = (self.ai_settings.max_ai_file_size_bytes / mb).max(1) as i64;
        self.ai_settings.max_ai_file_size_bytes = (current + delta).clamp(1, 256) as u64 * mb;
        self.ai_status = "AI file size limit edited; save to persist".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_ai_model_enabled(
        &mut self,
        model_id: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(model) = self
            .ai_settings
            .models
            .iter_mut()
            .find(|model| model.id == model_id)
        {
            model.enabled = !model.enabled;
            if model.enabled {
                self.ai_settings.default_model_id = Some(model.id.clone());
            } else if self.ai_settings.default_model_id.as_deref() == Some(model.id.as_str()) {
                self.ai_settings.default_model_id = self
                    .ai_settings
                    .models
                    .iter()
                    .find(|candidate| candidate.enabled)
                    .map(|candidate| candidate.id.clone());
            }
            self.ai_status = "AI model list edited; save to persist".to_string();
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn set_ai_default_model(
        &mut self,
        model_id: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(model) = self
            .ai_settings
            .models
            .iter_mut()
            .find(|model| model.id == model_id)
        {
            model.enabled = true;
            self.ai_settings.default_model_id = Some(model.id.clone());
            self.ai_status = "AI default model edited; save to persist".to_string();
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn begin_ai_chat_job(&mut self) -> (u64, Arc<AtomicBool>) {
        self.ai_chat_job_id = self.ai_chat_job_id.wrapping_add(1).max(1);
        let cancel = Arc::new(AtomicBool::new(false));
        self.ai_chat_cancel = Some(cancel.clone());
        (self.ai_chat_job_id, cancel)
    }

    pub(in crate::ui::view) fn cancel_ai_chat(&mut self, cx: &mut Context<Self>) {
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
        self.ai_agent_task_prompt = None;
        self.ai_command_cards.clear();
        self.ai_response_preview = "AI request cancelled".to_string();
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

    pub(in crate::ui::view) fn save_ai_settings(&mut self, cx: &mut Context<Self>) {
        let mut next = self.ai_settings.clone();
        let active_id = next.active_profile_id.clone();
        let mut active_kind = None;
        let mut active_name = active_id.clone();
        let mut active_base_url = none_if_blank(&self.ai_base_url_draft);
        let active_model = self.ai_model_draft.trim().to_string();

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
            if !self.ai_secret_draft.is_empty() {
                profile.api_key = Some(self.ai_secret_draft.clone());
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
                api_key: if self.ai_secret_draft.is_empty() {
                    next.provider_credentials
                        .iter()
                        .find(|credential| credential.id == active_id)
                        .and_then(|credential| credential.api_key.clone())
                } else {
                    Some(self.ai_secret_draft.clone())
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
                    next.models.push(nyaterm_domain::AiModelConfigItem {
                        id: model_id.clone(),
                        name: active_model,
                        provider_kind: Some(kind),
                        credential_id: active_base_url
                            .as_deref()
                            .is_some_and(|value| !value.trim().is_empty())
                            .then(|| active_id.clone()),
                        enabled: true,
                        source: nyaterm_domain::AiModelSource::Manual,
                        last_seen_at: None,
                    });
                }
                next.default_model_id = Some(model_id);
            }
        }

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_ai_settings(next))
        {
            Ok(saved) => {
                self.ai_settings = saved;
                self.ai_secret_draft.clear();
                self.sync_ai_drafts_from_active_profile();
                self.refresh_ai_usage_counts();
                self.ai_status = "AI settings saved".to_string();
                self.store_status.message = "AI settings saved".to_string();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.ai_status = format!("AI settings save failed: {error}");
                self.store_status.message = self.ai_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn discover_ai_models(&mut self, cx: &mut Context<Self>) {
        if self.ai_discovery_pending {
            self.ai_status = "AI model discovery already running".to_string();
            cx.notify();
            return;
        }

        let credential = match self.active_ai_discovery_credential() {
            Ok(credential) => credential,
            Err(error) => {
                self.ai_status = error;
                cx.notify();
                return;
            }
        };
        if credential
            .base_url
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            self.ai_status = "AI model discovery requires a Base URL".to_string();
            cx.notify();
            return;
        }

        let settings = self.ai_settings.clone();
        let profile_id = credential.id.clone();
        let tx = self.ai_discovery_tx.clone();
        self.ai_discovery_pending = true;
        self.ai_status = "Discovering AI models...".to_string();
        std::thread::spawn(move || {
            let result = discover_openai_compatible_models(&settings, &credential);
            let _ = tx.send(AiDiscoveryJobResult { profile_id, result });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn active_ai_discovery_credential(
        &self,
    ) -> Result<AiProviderCredential, String> {
        let active_id = self.ai_settings.active_profile_id.as_str();
        let profile = self
            .ai_settings
            .provider_profiles
            .iter()
            .find(|profile| profile.id == active_id)
            .ok_or_else(|| format!("AI provider profile '{active_id}' is not configured"))?;
        let current_credential = self
            .ai_settings
            .provider_credentials
            .iter()
            .find(|credential| credential.id == active_id);

        Ok(AiProviderCredential {
            id: profile.id.clone(),
            name: profile.name.clone(),
            provider_kind: profile.provider_kind.clone(),
            base_url: none_if_blank(&self.ai_base_url_draft),
            api_key: if self.ai_secret_draft.is_empty() {
                current_credential
                    .and_then(|credential| credential.api_key.clone())
                    .or_else(|| profile.api_key.clone())
            } else {
                Some(self.ai_secret_draft.clone())
            },
            enabled: true,
        })
    }

    pub(in crate::ui::view) fn drain_ai_discovery_events(&mut self) {
        while let Ok(event) = self.ai_discovery_rx.try_recv() {
            self.ai_discovery_pending = false;
            match event.result {
                Ok(discoveries) if discoveries.is_empty() => {
                    self.ai_status = "AI discovery returned no models".to_string();
                }
                Ok(discoveries) => {
                    let count = self.apply_ai_model_discoveries(&event.profile_id, discoveries);
                    self.ai_status = format!("Discovered {count} AI model(s); save to persist");
                    self.store_status.message = self.ai_status.clone();
                    self.store_status.ready = true;
                }
                Err(error) => {
                    self.ai_status = format!("AI model discovery failed: {error}");
                    self.store_status.message = self.ai_status.clone();
                    self.store_status.ready = false;
                }
            }
        }
    }

    pub(in crate::ui::view) fn apply_ai_model_discoveries(
        &mut self,
        profile_id: &str,
        discoveries: Vec<AiModelDiscovery>,
    ) -> usize {
        let discoveries = merge_model_discoveries(discoveries);
        let first_discovery = discoveries.first().cloned();
        let discovered_ids: HashSet<String> =
            discoveries.iter().map(|model| model.id.clone()).collect();
        let last_seen_at = Some(now_rfc3339());

        for discovery in &discoveries {
            if let Some(model) = self
                .ai_settings
                .models
                .iter_mut()
                .find(|model| model.id == discovery.id)
            {
                model.name = discovery.name.clone();
                model.provider_kind = discovery.provider_kind.clone();
                model.credential_id = discovery.credential_id.clone();
                model.enabled = true;
                model.source = discovery.source.clone();
                model.last_seen_at = last_seen_at.clone();
            } else {
                self.ai_settings
                    .models
                    .push(nyaterm_domain::AiModelConfigItem {
                        id: discovery.id.clone(),
                        name: discovery.name.clone(),
                        provider_kind: discovery.provider_kind.clone(),
                        credential_id: discovery.credential_id.clone(),
                        enabled: true,
                        source: discovery.source.clone(),
                        last_seen_at: last_seen_at.clone(),
                    });
            }
        }

        if self.ai_settings.active_profile_id == profile_id {
            let draft_model_id = ai_model_id_for_credential(profile_id, self.ai_model_draft.trim());
            if discovered_ids.contains(&draft_model_id) {
                self.ai_settings.default_model_id = Some(draft_model_id);
            } else {
                let current_default_is_valid = self
                    .ai_settings
                    .default_model_id
                    .as_deref()
                    .is_some_and(|id| {
                        self.ai_settings
                            .models
                            .iter()
                            .any(|model| model.enabled && model.id == id)
                    });
                if !current_default_is_valid && let Some(first_discovery) = first_discovery.as_ref()
                {
                    self.ai_settings.default_model_id = Some(first_discovery.id.clone());
                }
                if self.ai_model_draft.trim().is_empty()
                    && let Some(first_discovery) = first_discovery.as_ref()
                {
                    self.ai_model_draft = first_discovery.name.clone();
                }
            }
        }

        discoveries.len()
    }

    pub(in crate::ui::view) fn start_ai_ask(&mut self, cx: &mut Context<Self>) {
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
            user_input: prompt,
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
        }
        self.ai_chat_pending = true;
        self.ai_response_preview = if mode == AiMode::Agent {
            "Running AI Agent step...".to_string()
        } else {
            "Running AI request...".to_string()
        };
        self.ai_command_cards.clear();
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

    pub(in crate::ui::view) fn ai_terminal_context(&self) -> AiContext {
        let ssh = self.active_ssh_config.as_ref();
        let active_session = self.active_session_id.as_deref().and_then(|session_id| {
            self.session_manager
                .list_sessions()
                .ok()
                .and_then(|sessions| {
                    sessions
                        .into_iter()
                        .find(|session| session.id == session_id)
                })
        });
        AiContext {
            connection_name: ssh.map(|config| config.name.clone()),
            host: ssh.map(|config| config.host.clone()),
            port: ssh.map(|config| config.port),
            username: ssh.map(|config| config.username.clone()),
            cwd: active_session
                .as_ref()
                .and_then(|session| session.working_dir.as_ref())
                .map(|path| path.display().to_string()),
            os: None,
            arch: Some(std::env::consts::ARCH.to_string()),
            recent_output: recent_terminal_output(&self.terminal_output, 80),
            selected_text: String::new(),
            input_buffer: String::new(),
        }
    }

    pub(in crate::ui::view) fn handle_ai_prompt_key_down(
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

    pub(in crate::ui::view) fn drain_ai_chat_events(&mut self, cx: &mut Context<Self>) {
        while let Ok(event) = self.ai_chat_rx.try_recv() {
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
                    if self.ai_response_preview == "Running AI request..." {
                        self.ai_response_preview.clear();
                    }
                    self.ai_response_preview.push_str(&text_delta);
                    self.ai_response_preview = truncate_preview(&self.ai_response_preview, 320);
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
                    cx.notify();
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
                    cx.notify();
                }
                AiChatWorkerEvent::AgentBackgroundFinished {
                    job_id,
                    state,
                    result,
                } => {
                    if job_id != self.ai_chat_job_id {
                        continue;
                    }
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
                            cx.notify();
                        }
                    }
                }
                AiChatWorkerEvent::Finished(event) => {
                    if event.job_id != self.ai_chat_job_id {
                        continue;
                    }
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
                            self.ai_command_cards = output.command_cards;
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
    }

    pub(in crate::ui::view) fn sync_ai_drafts_from_active_profile(&mut self) {
        let (model, base_url) = ai_active_profile_drafts(&self.ai_settings);
        self.ai_model_draft = model;
        self.ai_base_url_draft = base_url;
        self.ai_secret_draft.clear();
    }

    pub(in crate::ui::view) fn refresh_ai_usage_counts(&mut self) {
        if let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            let (sessions, messages, audits) = ai_usage_counts(&store);
            self.ai_session_count = sessions;
            self.ai_message_count = messages;
            self.ai_audit_count = audits;
        }
    }
}
