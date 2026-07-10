use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn insert_ai_command_card(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_ai_command_card(index, false, cx);
    }

    pub(in crate::ui::view) fn run_ai_command_card(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_ai_command_card(index, true, cx);
    }

    pub(in crate::ui::view) fn active_session_history_commands(&self) -> Vec<String> {
        self.active_session_id
            .as_deref()
            .and_then(|session_id| self.session_command_history.get(session_id))
            .cloned()
            .unwrap_or_default()
    }

    fn active_session_history_command(&self, index: usize) -> Option<String> {
        let session_id = self.active_session_id.as_deref()?;
        self.session_command_history
            .get(session_id)?
            .get(index)
            .cloned()
    }

    pub(in crate::ui::view) fn insert_history_command(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_history_command(index, false, cx);
    }

    pub(in crate::ui::view) fn run_history_command(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_history_command(index, true, cx);
    }

    pub(in crate::ui::view) fn insert_command_search_result(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_command_search_result(index, false, cx);
    }

    pub(in crate::ui::view) fn run_command_search_result(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_command_search_result(index, true, cx);
    }

    fn apply_command_search_result(&mut self, index: usize, execute: bool, cx: &mut Context<Self>) {
        if self.active_session_id.is_none() {
            self.terminal_status =
                "start a terminal session before using command search".to_string();
            cx.notify();
            return;
        }
        let Some(result) = self.command_search_results().into_iter().nth(index) else {
            self.terminal_status = "command search result is no longer available".to_string();
            cx.notify();
            return;
        };
        let mut command = result.command.trim().to_string();
        if command.is_empty() {
            self.terminal_status = "command search result is empty".to_string();
            cx.notify();
            return;
        }
        if execute && !command.ends_with('\n') {
            command.push('\n');
        }
        self.send_terminal_input(command.into_bytes(), cx);
        self.terminal_status = if execute {
            format!("ran search result '{}'", result.display)
        } else {
            format!("inserted search result '{}'", result.display)
        };
        cx.notify();
    }

    fn apply_history_command(&mut self, index: usize, execute: bool, cx: &mut Context<Self>) {
        if self.active_session_id.is_none() {
            self.terminal_status = "start a terminal session before using history".to_string();
            cx.notify();
            return;
        }
        let Some(command_text) = self.active_session_history_command(index) else {
            self.terminal_status = "history command is no longer available".to_string();
            cx.notify();
            return;
        };
        let mut command = command_text.trim().to_string();
        if command.is_empty() {
            self.terminal_status = "history command is empty".to_string();
            cx.notify();
            return;
        }
        if execute && !command.ends_with('\n') {
            command.push('\n');
        }
        self.send_terminal_input(command.into_bytes(), cx);
        self.terminal_status = if execute {
            format!("ran history command '{command_text}'")
        } else {
            format!("inserted history command '{command_text}'")
        };
        cx.notify();
    }

    fn apply_ai_command_card(&mut self, index: usize, execute: bool, cx: &mut Context<Self>) {
        if self.active_session_id.is_none() {
            self.ai_status = "Start a terminal session before using an AI command".to_string();
            cx.notify();
            return;
        }
        let Some(card) = self.ai_command_cards.get(index).cloned() else {
            self.ai_status = "AI command card is no longer available".to_string();
            cx.notify();
            return;
        };
        let mut command = card.command.trim().to_string();
        if command.is_empty() {
            self.ai_status = "AI command card has no command".to_string();
            cx.notify();
            return;
        }
        let should_continue_agent = execute && is_agent_command_card(&card);
        if should_continue_agent && self.ai_settings.agent_background_execution_enabled {
            match self.begin_ai_agent_background_execution(&card.command, cx) {
                Ok(()) => {
                    self.record_ai_command_card_audit(&card, true, false);
                    cx.notify();
                }
                Err(error) => {
                    self.ai_status = error;
                    let step_index = self
                        .ai_agent_steps
                        .last()
                        .map(|step| step.step_index)
                        .unwrap_or(0);
                    self.upsert_ai_agent_step(
                        step_index,
                        AiAgentStepStatus::Failed,
                        "Failed",
                        self.ai_status.clone(),
                    );
                    cx.notify();
                }
            }
            return;
        }
        if execute && !command.ends_with('\n') {
            command.push('\n');
        }
        let input_bytes = if should_continue_agent {
            match self.begin_ai_agent_observation(&card.command) {
                Ok(Some(wrapped_command)) => wrapped_command.into_bytes(),
                Ok(None) => command.clone().into_bytes(),
                Err(error) => {
                    self.ai_status = error;
                    let step_index = self
                        .ai_agent_steps
                        .last()
                        .map(|step| step.step_index)
                        .unwrap_or(0);
                    self.upsert_ai_agent_step(
                        step_index,
                        AiAgentStepStatus::Failed,
                        "Failed",
                        self.ai_status.clone(),
                    );
                    cx.notify();
                    return;
                }
            }
        } else {
            command.clone().into_bytes()
        };

        self.record_ai_command_card_audit(&card, execute, true);

        self.send_terminal_input(input_bytes, cx);
        self.ai_status = if should_continue_agent {
            if let Some(state) = self.ai_agent_loop.as_ref().cloned() {
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

    pub(in crate::ui::view) fn record_command_history_from_bytes(
        &mut self,
        session_id: Option<&str>,
        bytes: &[u8],
    ) {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return;
        };
        if !text.contains('\n') && !text.contains('\r') {
            return;
        }
        let submitted: Vec<String> = text
            .split(['\r', '\n'])
            .map(str::trim)
            .filter(|command| !command.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        if submitted.is_empty() {
            return;
        }
        if let Some(session_id) = session_id {
            for command in &submitted {
                self.record_session_command_history(session_id, command);
            }
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => {
                for command in submitted {
                    if let Err(error) = store.append_command_history(&command) {
                        self.store_status.message = format!("command history save failed: {error}");
                        self.store_status.ready = false;
                        return;
                    }
                }
                self.command_history = store.list_command_history(64).unwrap_or_default();
            }
            Err(error) => {
                self.store_status.message = format!("command history store failed: {error}");
                self.store_status.ready = false;
            }
        }
    }

    fn record_session_command_history(&mut self, session_id: &str, command: &str) {
        let normalized_command = command.trim();
        if normalized_command.is_empty() {
            return;
        }
        let history = self
            .session_command_history
            .entry(session_id.to_string())
            .or_default();
        history.insert(0, normalized_command.to_string());
        history.truncate(SESSION_COMMAND_HISTORY_LIMIT);
    }

    pub(in crate::ui::view) fn handle_command_search_key_down(
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
            "backspace" => {
                self.command_search_draft.pop();
                cx.notify();
            }
            "escape" => {
                self.command_search_draft.clear();
                self.terminal_status = "command search cleared".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.command_search_draft.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::ui::view) fn command_search_results(&self) -> Vec<nyaterm_domain::FuzzyResult> {
        search_command_sources(
            &self.command_history,
            &self.quick_commands,
            &self.command_search_draft,
            8,
            Some(1),
            Some(512),
        )
    }
}
