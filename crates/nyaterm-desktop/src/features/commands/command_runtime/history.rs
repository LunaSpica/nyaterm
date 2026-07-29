use gpui::Context;
use nyaterm_core::{AiCommandCard, truncate_preview};

use crate::features::{
    AiAgentStepStatus, CommandPersistencePoll, NyaTermApp, is_agent_command_card,
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
        self.ai.find_command_card(card_id)
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
        if self.session.active_id().is_none() {
            self.shell
                .set_status("start a terminal session before using history".to_string());
            cx.notify();
            return;
        }
        let Some(command_text) = self.session.active_command_history_entry(index) else {
            self.shell
                .set_status("history command is no longer available".to_string());
            cx.notify();
            return;
        };
        let mut command = command_text.trim().to_string();
        if command.is_empty() {
            self.shell
                .set_status("history command is empty".to_string());
            cx.notify();
            return;
        }
        if execute && !command.ends_with('\r') && !command.ends_with('\n') {
            command.push('\r');
        }
        self.send_terminal_input(command.into_bytes(), cx);
        self.shell.set_status(if execute {
            format!("ran history command '{command_text}'")
        } else {
            format!("inserted history command '{command_text}'")
        });
        cx.notify();
    }

    pub(in crate::features) fn apply_ai_command_card(
        &mut self,
        index: usize,
        execute: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(card) = self.ai.command_card(index) else {
            self.ai
                .set_panel_status("AI command card is no longer available");
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
            self.ai
                .set_panel_status("AI command card is no longer available");
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
            self.ai
                .set_panel_status("Start a terminal session before using an AI command");
            cx.notify();
            return;
        };
        let mut command = card.command.trim().to_string();
        if command.is_empty() {
            self.ai.set_panel_status("AI command card has no command");
            cx.notify();
            return;
        }
        let should_continue_agent = execute && is_agent_command_card(&card);
        if should_continue_agent && self.ai.settings_config().agent_background_execution_enabled {
            match self.begin_ai_agent_background_execution(&card.command, cx) {
                Ok(()) => {
                    self.record_ai_command_card_audit(&card, true, false, cx);
                    cx.notify();
                }
                Err(error) => {
                    self.ai.set_panel_status(error);
                    let step_index = self.ai.last_agent_step_index();
                    self.upsert_ai_agent_step(
                        step_index,
                        AiAgentStepStatus::Failed,
                        "Failed",
                        self.ai.panel_status().to_string(),
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
                    self.ai.set_panel_status(error);
                    let step_index = self.ai.last_agent_step_index();
                    self.upsert_ai_agent_step(
                        step_index,
                        AiAgentStepStatus::Failed,
                        "Failed",
                        self.ai.panel_status().to_string(),
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
        let status = if should_continue_agent {
            if let Some(state) = self.ai.agent_loop_snapshot() {
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
        self.ai.set_panel_status(status);
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
            if let Some(command) = self.terminal.take_pending_command_history_entry() {
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
                self.session.record_command_history(session_id, command);
            }
        }
        if !self.commands.queue_command_history(submitted) {
            self.settings
                .update_store_status("command history worker is unavailable", false);
        }
    }

    pub(in crate::features) fn queue_quick_command_use_count(&mut self, command_id: String) {
        if !self.commands.queue_quick_command_use_count(command_id) {
            self.settings
                .update_store_status("command persistence worker is unavailable", false);
        }
    }

    pub(in crate::features) fn drain_command_persistence_events(&mut self) -> bool {
        let mut dirty = false;
        for _ in 0..COMMAND_PERSISTENCE_EVENT_DRAIN_LIMIT {
            let event = match self.commands.poll_persistence() {
                CommandPersistencePoll::Event(event) => event,
                CommandPersistencePoll::Empty => break,
                CommandPersistencePoll::Disconnected { had_pending } => {
                    if had_pending {
                        self.settings
                            .update_store_status("command persistence worker disconnected", false);
                        dirty = true;
                    }
                    break;
                }
            };
            dirty = true;
            if let Err(message) = self.commands.apply_persistence_result(event) {
                self.settings.update_store_status(message, false);
            }
        }
        dirty
    }
}
