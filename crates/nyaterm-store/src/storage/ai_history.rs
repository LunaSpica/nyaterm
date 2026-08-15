//! AI chat history and audit-log persistence.
//!
//! Split out of `storage.rs` by domain. Document keys, record shapes and the
//! trimming rules are unchanged; this only moves the code.

use super::{
    ConnectionStore, SETTINGS_AI_AUDIT, SETTINGS_AI_HISTORY, SETTINGS_TABLE, StorageError,
};
use nyaterm_core::{
    AiAuditFile, AiAuditLog, AiHistoryFile, AiMessage, AiMessageRole, AiSession,
    AppendAiAuditRequest, now_rfc3339, trim_ai_audit, trim_ai_history, uuid,
};

impl ConnectionStore {
    pub fn load_ai_history(&self) -> Result<AiHistoryFile, StorageError> {
        self.read_json_table::<AiHistoryFile>(SETTINGS_TABLE, SETTINGS_AI_HISTORY)
            .map(|history| history.unwrap_or_default())
    }
    pub fn save_ai_history(&self, mut history: AiHistoryFile) -> Result<(), StorageError> {
        trim_ai_history(&mut history);
        self.save_settings_doc_value(SETTINGS_AI_HISTORY, &serde_json::to_value(history)?)?;
        Ok(())
    }
    pub fn append_ai_user_message(
        &self,
        session_id: &str,
        connection_id: Option<String>,
        user_input: String,
    ) -> Result<(), StorageError> {
        let now = now_rfc3339();
        let title = ai_session_title(&user_input);
        let session_id = session_id.to_string();
        let mut history = self.load_ai_history()?;
        if let Some(session) = history
            .sessions
            .iter_mut()
            .find(|item| item.id == session_id)
        {
            session.updated_at = now.clone();
        } else {
            history.sessions.push(AiSession {
                id: session_id.clone(),
                connection_id,
                title,
                created_at: now.clone(),
                updated_at: now.clone(),
            });
        }
        history.messages.push(AiMessage {
            id: format!("msg-{}", uuid()),
            session_id,
            role: AiMessageRole::User,
            content: user_input,
            created_at: now,
            reasoning_content: None,
            command_cards: Vec::new(),
        });
        self.save_ai_history(history)
    }
    pub fn append_ai_message(&self, message: AiMessage) -> Result<(), StorageError> {
        let mut history = self.load_ai_history()?;
        if let Some(session) = history
            .sessions
            .iter_mut()
            .find(|item| item.id == message.session_id)
        {
            session.updated_at = message.created_at.clone();
        }
        history.messages.push(message);
        self.save_ai_history(history)
    }
    pub fn list_ai_sessions(&self) -> Result<Vec<AiSession>, StorageError> {
        let mut sessions = self.load_ai_history()?.sessions;
        sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(sessions)
    }
    pub fn list_ai_messages(&self, session_id: &str) -> Result<Vec<AiMessage>, StorageError> {
        Ok(self
            .load_ai_history()?
            .messages
            .into_iter()
            .filter(|message| message.session_id == session_id)
            .collect())
    }
    pub fn clear_ai_history(&self) -> Result<(), StorageError> {
        self.save_settings_doc_value(
            SETTINGS_AI_HISTORY,
            &serde_json::to_value(AiHistoryFile::default())?,
        )?;
        Ok(())
    }
    pub fn delete_ai_session(&self, session_id: &str) -> Result<(), StorageError> {
        let mut history = self.load_ai_history()?;
        history.sessions.retain(|session| session.id != session_id);
        history
            .messages
            .retain(|message| message.session_id != session_id);
        self.save_ai_history(history)
    }
    pub fn append_ai_audit(
        &self,
        request: AppendAiAuditRequest,
    ) -> Result<AiAuditLog, StorageError> {
        let log = AiAuditLog {
            id: format!("audit-{}", uuid()),
            connection_id: request.connection_id,
            action: request.action,
            user_input: request.user_input,
            generated_command: request.generated_command,
            risk_level: request.risk_level,
            inserted_to_terminal: request.inserted_to_terminal,
            executed: request.executed,
            blocked: request.blocked,
            created_at: now_rfc3339(),
        };
        let mut file = self.load_ai_audit_file()?;
        file.logs.push(log.clone());
        trim_ai_audit(&mut file);
        self.save_settings_doc_value(SETTINGS_AI_AUDIT, &serde_json::to_value(file)?)?;
        Ok(log)
    }
    pub fn list_ai_audit_logs(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<AiAuditLog>, StorageError> {
        let mut logs = self.load_ai_audit_file()?.logs;
        logs.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        if let Some(limit) = limit {
            logs.truncate(limit);
        }
        Ok(logs)
    }
    fn load_ai_audit_file(&self) -> Result<AiAuditFile, StorageError> {
        self.read_json_table::<AiAuditFile>(SETTINGS_TABLE, SETTINGS_AI_AUDIT)
            .map(|file| file.unwrap_or_default())
    }
}

fn ai_session_title(user_input: &str) -> String {
    let title = user_input.chars().take(42).collect::<String>();
    let title = title.trim();
    if title.is_empty() {
        "AI Session".to_string()
    } else {
        title.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::unique_temp_dir;

    #[test]
    fn ai_history_round_trips_messages_and_deletes_session() {
        let dir = unique_temp_dir("ai-history");
        let store = ConnectionStore::open(&dir).expect("store");

        store
            .append_ai_user_message("session-1", Some("conn-1".to_string()), "  ".to_string())
            .expect("append user");
        let sessions = store.list_ai_sessions().expect("sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].title, "AI Session");
        assert_eq!(sessions[0].connection_id.as_deref(), Some("conn-1"));

        store
            .append_ai_message(AiMessage {
                id: "assistant-1".to_string(),
                session_id: "session-1".to_string(),
                role: AiMessageRole::Assistant,
                content: "hello".to_string(),
                created_at: "2026-04-28T00:00:01Z".to_string(),
                reasoning_content: Some("reasoning".to_string()),
                command_cards: Vec::new(),
            })
            .expect("append assistant");

        let messages = store.list_ai_messages("session-1").expect("messages");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, AiMessageRole::User);
        assert_eq!(messages[1].reasoning_content.as_deref(), Some("reasoning"));

        store
            .delete_ai_session("session-1")
            .expect("delete session");
        assert!(store.list_ai_sessions().expect("sessions").is_empty());
        assert!(
            store
                .list_ai_messages("session-1")
                .expect("messages")
                .is_empty()
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn ai_audit_round_trips_sorts_and_limits_logs() {
        let dir = unique_temp_dir("ai-audit");
        let store = ConnectionStore::open(&dir).expect("store");

        let first = store
            .append_ai_audit(AppendAiAuditRequest {
                connection_id: Some("conn-1".to_string()),
                action: "generate_command".to_string(),
                user_input: Some("list files".to_string()),
                generated_command: Some("ls".to_string()),
                risk_level: Some(nyaterm_core::RiskLevel::Low),
                inserted_to_terminal: true,
                executed: false,
                blocked: false,
            })
            .expect("append audit");
        assert!(first.id.starts_with("audit-"));

        let mut file = AiAuditFile::default();
        file.logs.push(first.clone());
        file.logs.push(AiAuditLog {
            id: "audit-later".to_string(),
            connection_id: None,
            action: "execute".to_string(),
            user_input: None,
            generated_command: None,
            risk_level: Some(nyaterm_core::RiskLevel::Medium),
            inserted_to_terminal: false,
            executed: true,
            blocked: false,
            created_at: "2999-01-01T00:00:00Z".to_string(),
        });
        store
            .save_settings_doc_value(SETTINGS_AI_AUDIT, &serde_json::to_value(file).unwrap())
            .expect("save audit");

        let limited = store.list_ai_audit_logs(Some(1)).expect("audit logs");
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].id, "audit-later");

        std::fs::remove_dir_all(dir).ok();
    }
}
