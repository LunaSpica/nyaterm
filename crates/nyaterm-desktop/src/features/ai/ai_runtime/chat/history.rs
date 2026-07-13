use super::*;

impl NyaTermApp {
    pub(in crate::features) fn sync_ai_drafts_from_active_profile(&mut self) {
        let (model, base_url) = ai_active_profile_drafts(&self.ai_settings);
        self.ai_model_draft = model;
        self.ai_base_url_draft = base_url;
        self.ai_secret_draft.clear();
    }

    pub(in crate::features) fn refresh_ai_session_list(&mut self, cx: &mut Context<Self>) {
        let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) else {
            self.ai_sessions.clear();
            cx.notify();
            return;
        };
        self.ai_sessions = store.list_ai_sessions().unwrap_or_default();
        cx.notify();
    }

    pub(in crate::features) fn load_ai_session_messages(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) else {
            self.ai_status = "failed to open store for AI history".to_string();
            cx.notify();
            return;
        };
        match store.list_ai_messages(&session_id) {
            Ok(messages) => {
                self.ai_chat_session_id = session_id;
                self.ai_chat_messages = messages;
                self.ai_streaming_assistant_id = None;
                self.ai_history_open = false;
                self.ai_command_cards.clear();
                if let Some(last) = self
                    .ai_chat_messages
                    .iter()
                    .rev()
                    .find(|message| matches!(message.role, AiMessageRole::Assistant))
                {
                    self.ai_response_preview = truncate_preview(&last.content, 320);
                    self.ai_command_cards = last.command_cards.clone();
                } else {
                    self.ai_response_preview.clear();
                }
                self.ai_status =
                    format!("loaded AI session {}", compact_id(&self.ai_chat_session_id));
            }
            Err(error) => {
                self.ai_status = format!("failed to load AI session: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn delete_ai_session(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) else {
            self.ai_status = "failed to open store for AI history".to_string();
            cx.notify();
            return;
        };
        match store.delete_ai_session(&session_id) {
            Ok(()) => {
                if self.ai_chat_session_id == session_id {
                    self.ai_chat_messages.clear();
                    self.ai_command_cards.clear();
                    self.ai_streaming_assistant_id = None;
                    self.ai_chat_session_id = format!("ai-session-{}", uuid());
                    self.ai_response_preview = "Ask mode ready".to_string();
                }
                self.ai_sessions.retain(|session| session.id != session_id);
                self.ai_status = "AI session deleted".to_string();
            }
            Err(error) => {
                self.ai_status = format!("failed to delete AI session: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn clear_all_ai_history(&mut self, cx: &mut Context<Self>) {
        let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) else {
            self.ai_status = "failed to open store for AI history".to_string();
            cx.notify();
            return;
        };
        match store.clear_ai_history() {
            Ok(()) => {
                self.ai_sessions.clear();
                self.ai_history_query.clear();
                self.ai_chat_messages.clear();
                self.ai_command_cards.clear();
                self.ai_streaming_assistant_id = None;
                self.ai_chat_session_id = format!("ai-session-{}", uuid());
                self.ai_response_preview = if self.ai_settings.default_mode == AiMode::Agent {
                    "Agent mode ready".to_string()
                } else {
                    "Ask mode ready".to_string()
                };
                self.ai_status = "AI history cleared".to_string();
            }
            Err(error) => {
                self.ai_status = format!("failed to clear AI history: {error}");
            }
        }
        cx.notify();
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

    pub(in crate::features) fn refresh_ai_usage_counts(&mut self) {
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
