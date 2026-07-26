use super::*;

impl NyaTermApp {
    pub(in crate::features) fn sync_ai_drafts_from_active_profile(&mut self) {
        let (model, base_url) = ai_active_profile_drafts(&self.ai.settings.config);
        self.ai.settings.model_draft = model;
        self.ai.settings.base_url_draft = base_url;
        self.ai.settings.secret_draft.clear();
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
                if this.ai.history.job_id != job_id {
                    return;
                }
                this.ai.history.pending = false;
                match result {
                    Ok(sessions) => {
                        this.ai.history.sessions = sessions;
                        this.ai.panel.status = "AI history loaded".to_string();
                    }
                    Err(error) => {
                        this.ai.history.sessions.clear();
                        this.ai.panel.status = format!("failed to load AI history: {error}");
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(in crate::features) fn start_new_ai_chat(&mut self, cx: &mut Context<Self>) {
        self.ai.start_new_chat();
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
        let source_session_id = self.ai.chat.session_id.clone();
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
                if this.ai.history.job_id != job_id {
                    return;
                }
                this.ai.history.pending = false;
                if this.ai.chat.session_id != source_session_id {
                    this.ai.panel.status = "AI session load cancelled".to_string();
                    cx.notify();
                    return;
                }
                match result {
                    Ok(messages) => {
                        this.ai.chat.session_id = session_id;
                        this.ai.chat.messages = messages;
                        this.ai.chat.streaming_assistant_id = None;
                        this.ai.history.open = false;
                        this.ai.chat.message_menu = None;
                        this.ai.chat.quoted_text = None;
                        this.ai.chat.command_cards.clear();
                        if let Some(last) = this
                            .ai
                            .chat
                            .messages
                            .iter()
                            .rev()
                            .find(|message| matches!(message.role, AiMessageRole::Assistant))
                        {
                            this.ai.chat.response_preview = truncate_preview(&last.content, 320);
                            this.ai.chat.command_cards = last.command_cards.clone();
                        } else {
                            this.ai.chat.response_preview.clear();
                        }
                        this.ai.panel.status =
                            format!("loaded AI session {}", compact_id(&this.ai.chat.session_id));
                    }
                    Err(error) => {
                        this.ai.panel.status = format!("failed to load AI session: {error}");
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
                if this.ai.history.job_id != job_id {
                    return;
                }
                this.ai.history.pending = false;
                match result {
                    Ok(()) => {
                        if this.ai.chat.session_id == session_id {
                            this.ai.chat.messages.clear();
                            this.ai.chat.command_cards.clear();
                            this.ai.chat.streaming_assistant_id = None;
                            this.ai.chat.message_menu = None;
                            this.ai.chat.quoted_text = None;
                            this.ai.chat.session_id = format!("ai-session-{}", uuid());
                            this.ai.chat.response_preview = "Ask mode ready".to_string();
                        }
                        this.ai
                            .history
                            .sessions
                            .retain(|session| session.id != session_id);
                        this.ai.panel.status = "AI session deleted".to_string();
                        this.refresh_ai_usage_counts(cx);
                    }
                    Err(error) => {
                        this.ai.panel.status = format!("failed to delete AI session: {error}");
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
        let source_session_id = self.ai.chat.session_id.clone();
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
                if this.ai.history.job_id != job_id {
                    return;
                }
                this.ai.history.pending = false;
                match result {
                    Ok(()) => {
                        this.ai.history.sessions.clear();
                        this.ai.history.query.clear();
                        if this.ai.chat.session_id == source_session_id {
                            this.ai.chat.messages.clear();
                            this.ai.chat.command_cards.clear();
                            this.ai.chat.streaming_assistant_id = None;
                            this.ai.chat.message_menu = None;
                            this.ai.chat.quoted_text = None;
                            this.ai.panel.detected_error = None;
                            this.ai.chat.session_id = format!("ai-session-{}", uuid());
                            this.ai.chat.response_preview =
                                if this.ai.settings.config.default_mode == AiMode::Agent {
                                    "Agent mode ready".to_string()
                                } else {
                                    "Ask mode ready".to_string()
                                };
                        }
                        this.ai.panel.status = "AI history cleared".to_string();
                        this.refresh_ai_usage_counts(cx);
                    }
                    Err(error) => {
                        this.ai.panel.status = format!("failed to clear AI history: {error}");
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
                self.ai.history.open = false;
                self.ai.history.query.clear();
                cx.notify();
            }
            "backspace" => {
                self.ai.history.query.pop();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.ai.history.query.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn refresh_ai_usage_counts(&mut self, cx: &mut Context<Self>) {
        self.ai.history.usage_count_job_id =
            self.ai.history.usage_count_job_id.wrapping_add(1).max(1);
        let job_id = self.ai.history.usage_count_job_id;
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
                if this.ai.history.usage_count_job_id != job_id {
                    return;
                }
                if let Ok((sessions, messages, audits)) = result {
                    this.ai.history.session_count = sessions;
                    this.ai.history.message_count = messages;
                    this.ai.history.audit_count = audits;
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
        if self.ai.history.pending {
            self.ai.panel.status = "AI history operation already in progress".to_string();
            cx.notify();
            return None;
        }
        self.ai.history.job_id = self.ai.history.job_id.wrapping_add(1).max(1);
        self.ai.history.pending = true;
        self.ai.panel.status = status.to_string();
        cx.notify();
        Some(self.ai.history.job_id)
    }
}
