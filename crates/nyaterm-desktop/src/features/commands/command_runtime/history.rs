use std::sync::Arc;

use gpui::Context;
use nyaterm_core::{AiCommandCard, truncate_preview};

use crate::features::{
    AiAgentStepStatus, CommandPersistencePoll, CommandPersistenceRequest, CommandPersistenceResult,
    NyaTermApp, SESSION_COMMAND_HISTORY_LIMIT, is_agent_command_card,
};

const COMMAND_PERSISTENCE_EVENT_DRAIN_LIMIT: usize = 32;

impl NyaTermApp {
    pub(in crate::features) fn insert_ai_command_card(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_ai_command_card(index, false, cx);
    }

    pub(in crate::features) fn run_ai_command_card(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_ai_command_card(index, true, cx);
    }

    pub(in crate::features) fn insert_ai_command_card_by_id(
        &mut self,
        card_id: String,
        cx: &mut Context<Self>,
    ) {
        self.apply_ai_command_card_by_id(card_id, false, cx);
    }

    pub(in crate::features) fn run_ai_command_card_by_id(
        &mut self,
        card_id: String,
        cx: &mut Context<Self>,
    ) {
        self.apply_ai_command_card_by_id(card_id, true, cx);
    }

    pub(in crate::features) fn find_ai_command_card(&self, card_id: &str) -> Option<AiCommandCard> {
        self.ai
            .chat
            .command_cards
            .iter()
            .find(|card| card.id == card_id)
            .cloned()
            .or_else(|| {
                self.ai
                    .chat
                    .messages
                    .iter()
                    .flat_map(|message| message.command_cards.iter())
                    .find(|card| card.id == card_id)
                    .cloned()
            })
    }

    pub(in crate::features) fn active_session_history_commands(&self) -> Vec<String> {
        self.session
            .active_id
            .as_deref()
            .and_then(|session_id| self.session.command_history.get(session_id))
            .cloned()
            .unwrap_or_default()
    }

    pub(in crate::features) fn active_session_history_command(
        &self,
        index: usize,
    ) -> Option<String> {
        let session_id = self.session.active_id.as_deref()?;
        self.session
            .command_history
            .get(session_id)?
            .get(index)
            .cloned()
    }

    pub(in crate::features) fn run_history_command(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_history_command(index, true, cx);
    }

    pub(in crate::features) fn apply_history_command(
        &mut self,
        index: usize,
        execute: bool,
        cx: &mut Context<Self>,
    ) {
        if self.session.active_id.is_none() {
            self.terminal.view.status = "start a terminal session before using history".to_string();
            cx.notify();
            return;
        }
        let Some(command_text) = self.active_session_history_command(index) else {
            self.terminal.view.status = "history command is no longer available".to_string();
            cx.notify();
            return;
        };
        let mut command = command_text.trim().to_string();
        if command.is_empty() {
            self.terminal.view.status = "history command is empty".to_string();
            cx.notify();
            return;
        }
        if execute && !command.ends_with('\r') && !command.ends_with('\n') {
            command.push('\r');
        }
        self.send_terminal_input(command.into_bytes(), cx);
        self.terminal.view.status = if execute {
            format!("ran history command '{command_text}'")
        } else {
            format!("inserted history command '{command_text}'")
        };
        cx.notify();
    }

    pub(in crate::features) fn apply_ai_command_card(
        &mut self,
        index: usize,
        execute: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(card) = self.ai.chat.command_cards.get(index).cloned() else {
            self.ai.panel.status = "AI command card is no longer available".to_string();
            cx.notify();
            return;
        };
        self.apply_ai_command_card_value(card, execute, cx);
    }

    pub(in crate::features) fn apply_ai_command_card_by_id(
        &mut self,
        card_id: String,
        execute: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(card) = self.find_ai_command_card(&card_id) else {
            self.ai.panel.status = "AI command card is no longer available".to_string();
            cx.notify();
            return;
        };
        self.apply_ai_command_card_value(card, execute, cx);
    }

    pub(in crate::features) fn apply_ai_command_card_value(
        &mut self,
        card: AiCommandCard,
        execute: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(target_session_id) = self.ai_effective_target_session_id() else {
            self.ai.panel.status =
                "Start a terminal session before using an AI command".to_string();
            cx.notify();
            return;
        };
        let mut command = card.command.trim().to_string();
        if command.is_empty() {
            self.ai.panel.status = "AI command card has no command".to_string();
            cx.notify();
            return;
        }
        let should_continue_agent = execute && is_agent_command_card(&card);
        if should_continue_agent && self.ai.settings.config.agent_background_execution_enabled {
            match self.begin_ai_agent_background_execution(&card.command, cx) {
                Ok(()) => {
                    self.record_ai_command_card_audit(&card, true, false, cx);
                    cx.notify();
                }
                Err(error) => {
                    self.ai.panel.status = error;
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
                        self.ai.panel.status.clone(),
                    );
                    cx.notify();
                }
            }
            return;
        }
        if execute && !command.ends_with('\r') && !command.ends_with('\n') {
            command.push('\r');
        }
        let input_bytes = if should_continue_agent {
            match self.begin_ai_agent_observation(&card.command) {
                Ok(Some(wrapped_command)) => wrapped_command.into_bytes(),
                Ok(None) => command.clone().into_bytes(),
                Err(error) => {
                    self.ai.panel.status = error;
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
                        self.ai.panel.status.clone(),
                    );
                    cx.notify();
                    return;
                }
            }
        } else {
            command.clone().into_bytes()
        };

        self.record_ai_command_card_audit(&card, execute, true, cx);

        self.send_terminal_input_to_session(target_session_id, input_bytes, cx);
        self.ai.panel.status = if should_continue_agent {
            if let Some(state) = self.ai.agent.loop_state.as_ref().cloned() {
                self.upsert_ai_agent_step(
                    state.step_index,
                    AiAgentStepStatus::Running,
                    "Running",
                    truncate_preview(&state.command, 140),
                );
                format!(
                    "AI Agent observing command output for step {}/{}",
                    state.step_index + 1,
                    state.max_steps
                )
            } else {
                format!("Ran AI command card '{}'", card.title)
            }
        } else if execute {
            format!("Ran AI command card '{}'", card.title)
        } else {
            format!("Inserted AI command card '{}'", card.title)
        };
        cx.notify();
    }

    pub(in crate::features) fn record_command_history_from_bytes(
        &mut self,
        session_id: Option<&str>,
        bytes: &[u8],
    ) {
        let sessions: Vec<&str> = session_id.into_iter().collect();
        self.record_command_history_for_sessions(&sessions, bytes);
    }

    /// Resolve a submitted command once and attach it to every successful session.
    /// Global command history is appended only once per submission.
    pub(in crate::features) fn record_command_history_for_sessions(
        &mut self,
        session_ids: &[&str],
        bytes: &[u8],
    ) {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return;
        };
        if !text.contains('\n') && !text.contains('\r') {
            return;
        }
        let submitted: Vec<String> =
            if let Some(command) = self.terminal.assist.pending_command_history_entry.take() {
                vec![command]
            } else {
                text.split(['\r', '\n'])
                    .map(str::trim)
                    .filter(|command| !command.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            };
        if submitted.is_empty() {
            return;
        }
        for session_id in session_ids {
            for command in &submitted {
                self.record_session_command_history(session_id, command);
            }
        }
        if !self
            .command_runtime
            .queue(CommandPersistenceRequest::AppendHistory(submitted))
        {
            self.store_status.message = "command history worker is unavailable".to_string();
            self.store_status.ready = false;
        }
    }

    pub(in crate::features) fn queue_quick_command_use_count(&mut self, command_id: String) {
        if self
            .command_runtime
            .queue(CommandPersistenceRequest::IncrementQuickCommand(
                command_id.clone(),
            ))
        {
            if let Some(command) = Arc::make_mut(&mut self.quick_commands)
                .iter_mut()
                .find(|command| command.id == command_id)
            {
                command.use_count = Some(command.use_count.unwrap_or_default().saturating_add(1));
            }
        } else {
            self.store_status.message = "command persistence worker is unavailable".to_string();
            self.store_status.ready = false;
        }
    }

    pub(in crate::features) fn drain_command_persistence_events(&mut self) -> bool {
        let mut dirty = false;
        for _ in 0..COMMAND_PERSISTENCE_EVENT_DRAIN_LIMIT {
            let event = match self.command_runtime.poll() {
                CommandPersistencePoll::Event(event) => event,
                CommandPersistencePoll::Empty => break,
                CommandPersistencePoll::Disconnected { had_pending } => {
                    if had_pending {
                        self.store_status.message =
                            "command persistence worker disconnected".to_string();
                        self.store_status.ready = false;
                        dirty = true;
                    }
                    break;
                }
            };
            dirty = true;
            match event {
                CommandPersistenceResult::History(Ok(history)) => {
                    self.command_history = Arc::from(history);
                }
                CommandPersistenceResult::History(Err(error)) => {
                    self.store_status.message = format!("command history save failed: {error}");
                    self.store_status.ready = false;
                }
                CommandPersistenceResult::QuickCommandUseCount { command_id, result } => {
                    if let Err(error) = result {
                        if let Some(command) = Arc::make_mut(&mut self.quick_commands)
                            .iter_mut()
                            .find(|command| command.id == command_id)
                        {
                            command.use_count =
                                Some(command.use_count.unwrap_or_default().saturating_sub(1));
                        }
                        self.store_status.message =
                            format!("quick command use count update failed: {error}");
                        self.store_status.ready = false;
                    }
                }
            }
        }
        dirty
    }

    pub(in crate::features) fn record_session_command_history(
        &mut self,
        session_id: &str,
        command: &str,
    ) {
        let normalized_command = command.trim();
        if normalized_command.is_empty() {
            return;
        }
        let history = self
            .session
            .command_history
            .entry(session_id.to_string())
            .or_default();
        history.insert(0, normalized_command.to_string());
        history.truncate(SESSION_COMMAND_HISTORY_LIMIT);
    }
}
