use super::*;

impl NyaTermApp {
    pub(in crate::features) fn sync_ai_drafts_from_active_profile(&mut self) {
        let (model, base_url) = ai_active_profile_drafts(&self.ai_settings);
        self.ai_model_draft = model;
        self.ai_base_url_draft = base_url;
        self.ai_secret_draft.clear();
    }

    pub(in crate::features) fn refresh_ai_session_list(&mut self, cx: &mut Context<Self>) {
        let Some(job_id) = self.begin_ai_history_operation("loading AI history", cx) else {
            return;
        };
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        let task = cx.background_spawn(async move {
            ConnectionStore::open_with_portable_key_path(config_dir, portable_key_path)
                .and_then(|store| store.list_ai_sessions())
                .map_err(|error| error.to_string())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.ai_history_job_id != job_id {
                    return;
                }
                this.ai_history_pending = false;
                match result {
                    Ok(sessions) => {
                        this.ai_sessions = sessions;
                        this.ai_status = "AI history loaded".to_string();
                    }
                    Err(error) => {
                        this.ai_sessions.clear();
                        this.ai_status = format!("failed to load AI history: {error}");
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(in crate::features) fn start_new_ai_chat(&mut self, cx: &mut Context<Self>) {
        self.ai_prompt_draft.clear();
        self.ai_target_session_ids.clear();
        self.ai_message_menu = None;
        self.ai_quoted_text = None;
        self.ai_detected_error = None;
        self.ai_mention_open = false;
        self.ai_mention_query.clear();
        self.ai_mention_index = 0;
        self.ai_response_preview = if self.ai_settings.default_mode == AiMode::Agent {
            "Agent mode ready".to_string()
        } else {
            "Ask mode ready".to_string()
        };
        self.ai_command_cards.clear();
        self.ai_agent_task_prompt = None;
        self.ai_agent_step_index = 0;
        self.ai_agent_loop = None;
        self.ai_agent_capture = AgentOutputCaptureProcessor::new();
        self.ai_agent_steps.clear();
        self.ai_agent_thought_expanded.clear();
        self.ai_agent_output_expanded.clear();
        self.ai_chat_messages.clear();
        self.ai_streaming_assistant_id = None;
        self.ai_prepared_request = None;
        self.ai_chat_session_id = format!("ai-session-{}", uuid());
        self.ai_history_open = false;
        self.ai_history_query.clear();
        self.ai_execution_menu_open = false;
        self.ai_model_menu_open = false;
        self.ai_model_query.clear();
        self.ai_model_index = 0;
        self.ai_status = "new AI chat".to_string();
        cx.notify();
    }

    pub(in crate::features) fn load_ai_session_messages(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(job_id) = self.begin_ai_history_operation("loading AI session", cx) else {
            return;
        };
        let source_session_id = self.ai_chat_session_id.clone();
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        let job_session_id = session_id.clone();
        let task = cx.background_spawn(async move {
            ConnectionStore::open_with_portable_key_path(config_dir, portable_key_path)
                .and_then(|store| store.list_ai_messages(&job_session_id))
                .map_err(|error| error.to_string())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.ai_history_job_id != job_id {
                    return;
                }
                this.ai_history_pending = false;
                if this.ai_chat_session_id != source_session_id {
                    this.ai_status = "AI session load cancelled".to_string();
                    cx.notify();
                    return;
                }
                match result {
                    Ok(messages) => {
                        this.ai_chat_session_id = session_id;
                        this.ai_chat_messages = messages;
                        this.ai_streaming_assistant_id = None;
                        this.ai_history_open = false;
                        this.ai_message_menu = None;
                        this.ai_quoted_text = None;
                        this.ai_command_cards.clear();
                        if let Some(last) = this
                            .ai_chat_messages
                            .iter()
                            .rev()
                            .find(|message| matches!(message.role, AiMessageRole::Assistant))
                        {
                            this.ai_response_preview = truncate_preview(&last.content, 320);
                            this.ai_command_cards = last.command_cards.clone();
                        } else {
                            this.ai_response_preview.clear();
                        }
                        this.ai_status =
                            format!("loaded AI session {}", compact_id(&this.ai_chat_session_id));
                    }
                    Err(error) => {
                        this.ai_status = format!("failed to load AI session: {error}");
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(in crate::features) fn delete_ai_session(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(job_id) = self.begin_ai_history_operation("deleting AI session", cx) else {
            return;
        };
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        let job_session_id = session_id.clone();
        let task = cx.background_spawn(async move {
            ConnectionStore::open_with_portable_key_path(config_dir, portable_key_path)
                .and_then(|store| store.delete_ai_session(&job_session_id))
                .map_err(|error| error.to_string())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.ai_history_job_id != job_id {
                    return;
                }
                this.ai_history_pending = false;
                match result {
                    Ok(()) => {
                        if this.ai_chat_session_id == session_id {
                            this.ai_chat_messages.clear();
                            this.ai_command_cards.clear();
                            this.ai_streaming_assistant_id = None;
                            this.ai_message_menu = None;
                            this.ai_quoted_text = None;
                            this.ai_chat_session_id = format!("ai-session-{}", uuid());
                            this.ai_response_preview = "Ask mode ready".to_string();
                        }
                        this.ai_sessions.retain(|session| session.id != session_id);
                        this.ai_status = "AI session deleted".to_string();
                        this.refresh_ai_usage_counts(cx);
                    }
                    Err(error) => {
                        this.ai_status = format!("failed to delete AI session: {error}");
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(in crate::features) fn clear_all_ai_history(&mut self, cx: &mut Context<Self>) {
        let Some(job_id) = self.begin_ai_history_operation("clearing AI history", cx) else {
            return;
        };
        let source_session_id = self.ai_chat_session_id.clone();
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        let task = cx.background_spawn(async move {
            ConnectionStore::open_with_portable_key_path(config_dir, portable_key_path)
                .and_then(|store| store.clear_ai_history())
                .map_err(|error| error.to_string())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.ai_history_job_id != job_id {
                    return;
                }
                this.ai_history_pending = false;
                match result {
                    Ok(()) => {
                        this.ai_sessions.clear();
                        this.ai_history_query.clear();
                        if this.ai_chat_session_id == source_session_id {
                            this.ai_chat_messages.clear();
                            this.ai_command_cards.clear();
                            this.ai_streaming_assistant_id = None;
                            this.ai_message_menu = None;
                            this.ai_quoted_text = None;
                            this.ai_detected_error = None;
                            this.ai_chat_session_id = format!("ai-session-{}", uuid());
                            this.ai_response_preview =
                                if this.ai_settings.default_mode == AiMode::Agent {
                                    "Agent mode ready".to_string()
                                } else {
                                    "Ask mode ready".to_string()
                                };
                        }
                        this.ai_status = "AI history cleared".to_string();
                        this.refresh_ai_usage_counts(cx);
                    }
                    Err(error) => {
                        this.ai_status = format!("failed to clear AI history: {error}");
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(in crate::features) fn handle_ai_history_search_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }
        match keystroke.key.as_str() {
            "escape" => {
                self.ai_history_open = false;
                self.ai_history_query.clear();
                cx.notify();
            }
            "backspace" => {
                self.ai_history_query.pop();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.ai_history_query.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn refresh_ai_usage_counts(&mut self, cx: &mut Context<Self>) {
        self.ai_usage_count_job_id = self.ai_usage_count_job_id.wrapping_add(1).max(1);
        let job_id = self.ai_usage_count_job_id;
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        let task = cx.background_spawn(async move {
            ConnectionStore::open_with_portable_key_path(config_dir, portable_key_path)
                .map(|store| ai_usage_counts(&store))
                .map_err(|error| error.to_string())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.ai_usage_count_job_id != job_id {
                    return;
                }
                if let Ok((sessions, messages, audits)) = result {
                    this.ai_session_count = sessions;
                    this.ai_message_count = messages;
                    this.ai_audit_count = audits;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn begin_ai_history_operation(
        &mut self,
        status: &'static str,
        cx: &mut Context<Self>,
    ) -> Option<u64> {
        if self.ai_history_pending {
            self.ai_status = "AI history operation already in progress".to_string();
            cx.notify();
            return None;
        }
        self.ai_history_job_id = self.ai_history_job_id.wrapping_add(1).max(1);
        self.ai_history_pending = true;
        self.ai_status = status.to_string();
        cx.notify();
        Some(self.ai_history_job_id)
    }
}
