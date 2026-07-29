use gpui::Context;
use nyaterm_core::{
    AiCommandCard, AppendAiAuditRequest, ConnectionStore, QuickCommand, QuickCommandCategory, uuid,
};

use crate::features::NyaTermApp;
use crate::models::QuickCommandVariablePromptState;

use super::helpers::{ai_command_card_category_name, unique_quick_command_category_id};
use super::variables::parse_quick_command_variables;

impl NyaTermApp {
    pub(in crate::features) fn save_ai_command_card(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(card) = self.ai.command_card(index) else {
            self.ai
                .set_panel_status("AI command card is no longer available");
            cx.notify();
            return;
        };
        self.save_ai_command_card_value(card, cx);
    }

    pub(in crate::features) fn save_ai_command_card_by_id(
        &mut self,
        card_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(card) = self.find_ai_command_card(&card_id) else {
            self.ai
                .set_panel_status("AI command card is no longer available");
            cx.notify();
            return;
        };
        self.save_ai_command_card_value(card, cx);
    }

    pub(in crate::features) fn save_ai_command_card_value(
        &mut self,
        card: AiCommandCard,
        cx: &mut Context<Self>,
    ) {
        let command_text = card.command.trim();
        if command_text.is_empty() {
            self.ai.set_panel_status("AI command card has no command");
            cx.notify();
            return;
        }

        let result = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| {
            let config = store.load_quick_commands()?;
            let category_name = ai_command_card_category_name(&card);
            let existing_category = config
                .categories
                .iter()
                .find(|category| category.name == category_name)
                .cloned();
            let (category_id, new_category) = match existing_category {
                Some(category) => (category.id, None),
                None => {
                    let id = unique_quick_command_category_id(&config.categories, &category_name);
                    (
                        id.clone(),
                        Some(QuickCommandCategory {
                            id,
                            name: category_name,
                        }),
                    )
                }
            };
            let label = if card.title.trim().is_empty() {
                "AI Command".to_string()
            } else {
                card.title.trim().to_string()
            };
            let description = if card.explanation.trim().is_empty() {
                None
            } else {
                Some(card.explanation.trim().to_string())
            };
            store.upsert_quick_command(
                QuickCommand {
                    id: format!("ai-{}", uuid()),
                    label: label.clone(),
                    command: command_text.to_string(),
                    category_id: Some(category_id),
                    description,
                    color_tag: Some("blue".to_string()),
                    icon_tag: Some("terminal".to_string()),
                    pinned: Some(false),
                    execution_mode: Some("append".to_string()),
                    source: Some("ai".to_string()),
                    risk_level: card.risk_level.clone(),
                    updated_at: None,
                    created_at: None,
                    use_count: None,
                },
                new_category,
            )?;
            store
                .append_ai_audit(AppendAiAuditRequest {
                    connection_id: self.session.active_id_owned(),
                    action: "ai.save_quick_command".to_string(),
                    user_input: Some(self.ai.chat_response_preview().to_string()),
                    generated_command: Some(card.command.clone()),
                    risk_level: card.risk_level.clone(),
                    inserted_to_terminal: false,
                    executed: false,
                    blocked: false,
                })
                .map(|_| label)
        });

        match result {
            Ok(label) => {
                self.refresh_ai_usage_counts(cx);
                self.refresh_quick_commands();
                self.ai.set_panel_status(format!(
                    "Saved AI command card '{}' to Quick Commands",
                    label
                ));
                self.settings
                    .set_store_message(self.ai.panel_status().to_string());
                self.settings.set_store_ready(true);
            }
            Err(error) => {
                self.ai
                    .set_panel_status(format!("Quick command save failed: {error}"));
                self.settings
                    .set_store_message(self.ai.panel_status().to_string());
                self.settings.set_store_ready(false);
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn run_quick_command_by_id(
        &mut self,
        command_id: String,
        cx: &mut Context<Self>,
    ) {
        self.apply_quick_command_by_id(&command_id, true, false, cx);
    }

    pub(in crate::features) fn send_quick_command_to_all_by_id(
        &mut self,
        command_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(command) = self
            .commands
            .quick_commands()
            .iter()
            .find(|command| command.id == command_id)
            .cloned()
        else {
            self.shell.status = "quick command is no longer available".to_string();
            cx.notify();
            return;
        };
        let execute = command.execution_mode.as_deref() != Some("append");
        self.apply_quick_command_by_id(&command.id, execute, true, cx);
    }

    pub(in crate::features) fn apply_quick_command_by_id(
        &mut self,
        command_id: &str,
        execute: bool,
        send_to_all: bool,
        cx: &mut Context<Self>,
    ) {
        if send_to_all {
            if self.live_session_count() == 0 {
                self.shell.status =
                    "start a terminal session before using a quick command".to_string();
                cx.notify();
                return;
            }
        } else if self.session.active_id().is_none() {
            self.shell.status = "start a terminal session before using a quick command".to_string();
            cx.notify();
            return;
        }
        let Some(command) = self
            .commands
            .quick_commands()
            .iter()
            .find(|command| command.id == command_id)
            .cloned()
        else {
            self.shell.status = "quick command is no longer available".to_string();
            cx.notify();
            return;
        };
        let command_text = command.command.trim().to_string();
        if command_text.is_empty() {
            self.shell.status = "quick command has no command text".to_string();
            cx.notify();
            return;
        }
        let variables = parse_quick_command_variables(&command_text);
        if !variables.is_empty() {
            self.commands
                .request_quick_variable_prompt(QuickCommandVariablePromptState {
                    command_id: command.id,
                    label: command.label,
                    command: command_text,
                    execute,
                    send_to_all,
                    variables,
                    focused_index: 0,
                });
            self.shell.status = "fill quick command variables".to_string();
            cx.notify();
            return;
        }
        self.send_resolved_quick_command(
            command.id,
            command.label,
            command_text,
            execute,
            send_to_all,
            cx,
        );
    }

    pub(in crate::features) fn send_resolved_quick_command(
        &mut self,
        command_id: String,
        label: String,
        mut command_text: String,
        execute: bool,
        send_to_all: bool,
        cx: &mut Context<Self>,
    ) {
        if execute && !command_text.ends_with('\r') && !command_text.ends_with('\n') {
            command_text.push('\r');
        }
        let command_bytes = command_text.into_bytes();
        self.queue_quick_command_use_count(command_id);

        if send_to_all {
            let sessions = self
                .ordered_sessions()
                .into_iter()
                .filter(|session| !self.is_session_disconnected(&session.id))
                .collect::<Vec<_>>();
            if sessions.is_empty() {
                self.shell.status =
                    "start a terminal session before using a quick command".to_string();
                cx.notify();
                return;
            }
            let mut sent = 0usize;
            let mut failed = 0usize;
            let mut ok_sessions = Vec::new();
            for session in sessions {
                match self.write_session_input_recorded(&session.id, &command_bytes) {
                    Ok(()) => {
                        sent += 1;
                        ok_sessions.push(session.id);
                    }
                    Err(_) => failed += 1,
                }
            }
            let session_refs: Vec<&str> = ok_sessions.iter().map(String::as_str).collect();
            self.record_command_history_for_sessions(&session_refs, &command_bytes);
            self.shell.status = if failed == 0 {
                format!("sent quick command '{label}' to {sent} session(s)")
            } else {
                format!("sent quick command '{label}' to {sent} session(s), {failed} failed")
            };
            cx.notify();
            return;
        }

        if self.send_terminal_input(command_bytes, cx) {
            self.shell.status = if execute {
                format!("ran quick command '{label}'")
            } else {
                format!("inserted quick command '{label}'")
            };
            cx.notify();
        }
    }
}
