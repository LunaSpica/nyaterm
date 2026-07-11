use super::*;
use variables::parse_quick_command_variables;

mod import;
mod variables;

pub(in crate::ui::view) const QUICK_COMMAND_COLOR_OPTIONS: [Option<&str>; 6] = [
    None,
    Some("red"),
    Some("green"),
    Some("blue"),
    Some("yellow"),
    Some("purple"),
];
pub(in crate::ui::view) const QUICK_COMMAND_ICON_OPTIONS: [Option<&str>; 31] = [
    None,
    Some("terminal"),
    Some("code"),
    Some("server"),
    Some("folder"),
    Some("sparkles"),
    Some("bolt"),
    Some("docker"),
    Some("k8s"),
    Some("linux"),
    Some("ubuntu"),
    Some("debian"),
    Some("centos"),
    Some("fedora"),
    Some("apple"),
    Some("github"),
    Some("gitlab"),
    Some("nginx"),
    Some("redis"),
    Some("postgres"),
    Some("mysql"),
    Some("mongodb"),
    Some("python"),
    Some("js"),
    Some("ts"),
    Some("rust"),
    Some("go"),
    Some("node"),
    Some("php"),
    Some("aws"),
    Some("gcp"),
];

impl NyaTermApp {
    pub(in crate::ui::view) fn refresh_quick_commands(&mut self) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.load_quick_commands())
        {
            Ok(config) => {
                self.quick_commands = config.commands;
                self.quick_command_categories = config.categories;
            }
            Err(error) => {
                self.store_status.message = format!("quick command refresh failed: {error}");
                self.store_status.ready = false;
            }
        }
    }

    pub(in crate::ui::view) fn set_quick_command_view_mode(
        &mut self,
        mode: QuickCommandViewMode,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_view_mode = mode;
        self.settings.ui_quick_cmd_view_mode = quick_command_view_mode_setting(mode).to_string();
        self.save_quick_command_ui_settings(cx);
    }

    pub(in crate::ui::view) fn set_quick_command_sort_mode(
        &mut self,
        mode: QuickCommandSortMode,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_sort_mode = mode;
        self.settings.ui_quick_cmd_sort_mode = quick_command_sort_mode_setting(mode).to_string();
        self.save_quick_command_ui_settings(cx);
    }

    fn save_quick_command_ui_settings(&mut self, cx: &mut Context<Self>) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_quick_command_ui_settings(&self.settings))
        {
            Ok(settings) => {
                self.settings = settings;
                self.store_status.message = "quick command UI settings saved".to_string();
                self.store_status.ready = true;
                self.terminal_status = "quick command UI settings saved".to_string();
            }
            Err(error) => {
                self.store_status.message =
                    format!("quick command UI settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_quick_command_search_key_down(
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
                self.quick_command_search_draft.pop();
                cx.notify();
            }
            "escape" => {
                self.quick_command_search_draft.clear();
                self.quick_command_selected_category = "all".to_string();
                self.terminal_status = "quick command filters cleared".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.quick_command_search_draft.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::ui::view) fn save_ai_command_card(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(card) = self.ai_command_cards.get(index).cloned() else {
            self.ai_status = "AI command card is no longer available".to_string();
            cx.notify();
            return;
        };
        self.save_ai_command_card_value(card, cx);
    }

    pub(in crate::ui::view) fn save_ai_command_card_by_id(
        &mut self,
        card_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(card) = self.find_ai_command_card(&card_id) else {
            self.ai_status = "AI command card is no longer available".to_string();
            cx.notify();
            return;
        };
        self.save_ai_command_card_value(card, cx);
    }

    fn save_ai_command_card_value(&mut self, card: AiCommandCard, cx: &mut Context<Self>) {
        let command_text = card.command.trim();
        if command_text.is_empty() {
            self.ai_status = "AI command card has no command".to_string();
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
                    connection_id: self.active_session_id.clone(),
                    action: "ai.save_quick_command".to_string(),
                    user_input: Some(self.ai_response_preview.clone()),
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
                self.refresh_ai_usage_counts();
                self.refresh_quick_commands();
                self.ai_status = format!("Saved AI command card '{}' to Quick Commands", label);
                self.store_status.message = self.ai_status.clone();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.ai_status = format!("Quick command save failed: {error}");
                self.store_status.message = self.ai_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn insert_quick_command(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_quick_command(index, false, cx);
    }

    pub(in crate::ui::view) fn run_quick_command(&mut self, index: usize, cx: &mut Context<Self>) {
        self.apply_quick_command(index, true, cx);
    }

    pub(in crate::ui::view) fn insert_quick_command_by_id(
        &mut self,
        command_id: String,
        cx: &mut Context<Self>,
    ) {
        self.apply_quick_command_by_id(&command_id, false, false, cx);
    }

    pub(in crate::ui::view) fn run_quick_command_by_id(
        &mut self,
        command_id: String,
        cx: &mut Context<Self>,
    ) {
        self.apply_quick_command_by_id(&command_id, true, false, cx);
    }

    pub(in crate::ui::view) fn send_quick_command_to_all_by_id(
        &mut self,
        command_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(command) = self
            .quick_commands
            .iter()
            .find(|command| command.id == command_id)
            .cloned()
        else {
            self.terminal_status = "quick command is no longer available".to_string();
            cx.notify();
            return;
        };
        let execute = command.execution_mode.as_deref() != Some("append");
        self.apply_quick_command_by_id(&command.id, execute, true, cx);
    }

    fn apply_quick_command(&mut self, index: usize, execute: bool, cx: &mut Context<Self>) {
        let Some(command_id) = sorted_quick_commands(&self.quick_commands)
            .into_iter()
            .take(5)
            .nth(index)
            .map(|command| command.id)
        else {
            self.terminal_status = "quick command is no longer available".to_string();
            cx.notify();
            return;
        };
        self.apply_quick_command_by_id(&command_id, execute, false, cx);
    }

    fn apply_quick_command_by_id(
        &mut self,
        command_id: &str,
        execute: bool,
        send_to_all: bool,
        cx: &mut Context<Self>,
    ) {
        if send_to_all {
            if self
                .session_manager
                .list_sessions()
                .unwrap_or_default()
                .is_empty()
            {
                self.terminal_status =
                    "start a terminal session before using a quick command".to_string();
                cx.notify();
                return;
            }
        } else if self.active_session_id.is_none() {
            self.terminal_status =
                "start a terminal session before using a quick command".to_string();
            cx.notify();
            return;
        }
        let Some(command) = self
            .quick_commands
            .iter()
            .find(|command| command.id == command_id)
            .cloned()
        else {
            self.terminal_status = "quick command is no longer available".to_string();
            cx.notify();
            return;
        };
        let command_text = command.command.trim().to_string();
        if command_text.is_empty() {
            self.terminal_status = "quick command has no command text".to_string();
            cx.notify();
            return;
        }
        let variables = parse_quick_command_variables(&command_text);
        if !variables.is_empty() {
            self.quick_command_variable_prompt = Some(QuickCommandVariablePromptState {
                command_id: command.id,
                label: command.label,
                command: command_text,
                execute,
                send_to_all,
                variables,
                focused_index: 0,
            });
            self.terminal_status = "fill quick command variables".to_string();
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

    pub(in crate::ui::view) fn send_resolved_quick_command(
        &mut self,
        command_id: String,
        label: String,
        mut command_text: String,
        execute: bool,
        send_to_all: bool,
        cx: &mut Context<Self>,
    ) {
        if execute && !command_text.ends_with('\n') {
            command_text.push('\n');
        }
        let command_bytes = command_text.into_bytes();
        if let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            if let Err(error) = store.increment_quick_command_use_count(&command_id) {
                self.store_status.message =
                    format!("quick command use count update failed: {error}");
                self.store_status.ready = false;
            } else {
                self.refresh_quick_commands();
            }
        }

        if send_to_all {
            let sessions = self.session_manager.list_sessions().unwrap_or_default();
            if sessions.is_empty() {
                self.terminal_status =
                    "start a terminal session before using a quick command".to_string();
                cx.notify();
                return;
            }
            let mut sent = 0usize;
            let mut failed = 0usize;
            for session in sessions {
                match self.session_manager.write(&session.id, &command_bytes) {
                    Ok(()) => {
                        sent += 1;
                        self.recording_manager
                            .write_input(&session.id, &command_bytes);
                        self.record_command_history_from_bytes(Some(&session.id), &command_bytes);
                    }
                    Err(_) => failed += 1,
                }
            }
            self.terminal_status = if failed == 0 {
                format!("sent quick command '{label}' to {sent} session(s)")
            } else {
                format!("sent quick command '{label}' to {sent} session(s), {failed} failed")
            };
            cx.notify();
            return;
        }

        self.send_terminal_input(command_bytes, cx);
        self.terminal_status = if execute {
            format!("ran quick command '{label}'")
        } else {
            format!("inserted quick command '{label}'")
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn open_new_quick_command_editor(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_editor = Some(QuickCommandEditorState::blank());
        self.terminal_status = "quick command editor opened".to_string();
        window.focus(&self.quick_command_editor_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn open_edit_quick_command_editor(
        &mut self,
        command_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(command) = self
            .quick_commands
            .iter()
            .find(|command| command.id == command_id)
            .cloned()
        else {
            self.terminal_status = "quick command is no longer available".to_string();
            cx.notify();
            return;
        };
        self.quick_command_editor = Some(QuickCommandEditorState::from_command(command));
        self.terminal_status = "quick command editor opened".to_string();
        window.focus(&self.quick_command_editor_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn close_quick_command_editor(&mut self, cx: &mut Context<Self>) {
        self.quick_command_editor = None;
        self.terminal_status = "quick command editor closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn open_quick_command_details(
        &mut self,
        command_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(command) = self
            .quick_commands
            .iter()
            .find(|command| command.id == command_id)
            .cloned()
        else {
            self.terminal_status = "quick command is no longer available".to_string();
            cx.notify();
            return;
        };
        self.quick_command_details = Some(QuickCommandDetailsState {
            category: quick_command_category_label(&self.quick_command_categories, &command),
            risk: risk_label(command.risk_level.as_ref()).to_string(),
            command,
        });
        self.terminal_status = "quick command details opened".to_string();
        window.focus(&self.quick_command_details_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn close_quick_command_details(&mut self, cx: &mut Context<Self>) {
        self.quick_command_details = None;
        self.terminal_status = "quick command details closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn copy_quick_command_details(&mut self, cx: &mut Context<Self>) {
        let Some(details) = self.quick_command_details.as_ref() else {
            return;
        };
        let command = &details.command;
        let text = format!(
            "Label: {}\nCategory: {}\nMode: {}\nRisk: {}\nUse count: {}\nCommand:\n{}\n\nDescription:\n{}",
            command.label,
            details.category,
            if command.execution_mode.as_deref() == Some("append") {
                "append"
            } else {
                "execute"
            },
            details.risk,
            command.use_count.unwrap_or_default(),
            command.command,
            command.description.as_deref().unwrap_or_default()
        );
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.terminal_status = "quick command details copied".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn open_delete_quick_command_confirm(
        &mut self,
        command_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(command) = self
            .quick_commands
            .iter()
            .find(|command| command.id == command_id)
            .cloned()
        else {
            self.terminal_status = "quick command is no longer available".to_string();
            cx.notify();
            return;
        };
        self.quick_command_delete = Some(QuickCommandDeleteState {
            id: command.id,
            label: command.label,
        });
        self.terminal_status = "quick command delete confirmation opened".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn cancel_delete_quick_command(&mut self, cx: &mut Context<Self>) {
        self.quick_command_delete = None;
        self.terminal_status = "quick command delete cancelled".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn confirm_delete_quick_command(&mut self, cx: &mut Context<Self>) {
        let Some(delete) = self.quick_command_delete.clone() else {
            return;
        };
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| {
            let mut config = store.load_quick_commands()?;
            let before = config.commands.len();
            config.commands.retain(|command| command.id != delete.id);
            let deleted = config.commands.len() != before;
            store.save_quick_commands(config.clone())?;
            Ok((config, deleted))
        }) {
            Ok((config, deleted)) => {
                self.quick_commands = config.commands;
                self.quick_command_categories = config.categories;
                self.quick_command_delete = None;
                self.store_status.message = if deleted {
                    format!("quick command '{}' deleted", delete.label)
                } else {
                    format!("quick command '{}' was already deleted", delete.label)
                };
                self.store_status.ready = deleted;
                self.terminal_status = self.store_status.message.clone();
            }
            Err(error) => {
                self.store_status.message = format!("quick command delete failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn open_delete_quick_command_category_confirm(
        &mut self,
        category_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(category) = self
            .quick_command_categories
            .iter()
            .find(|category| category.id == category_id)
            .cloned()
        else {
            self.terminal_status = "quick command category is no longer available".to_string();
            cx.notify();
            return;
        };
        let command_count = self
            .quick_commands
            .iter()
            .filter(|command| command.category_id.as_deref() == Some(category.id.as_str()))
            .count();
        self.quick_command_category_delete = Some(QuickCommandCategoryDeleteState {
            id: category.id,
            name: category.name,
            command_count,
        });
        self.terminal_status = "quick command category delete confirmation opened".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn cancel_delete_quick_command_category(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_category_delete = None;
        self.terminal_status = "quick command category delete cancelled".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn confirm_delete_quick_command_category(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(delete) = self.quick_command_category_delete.clone() else {
            return;
        };
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| {
            let mut config = store.load_quick_commands()?;
            let before_categories = config.categories.len();
            let before_commands = config.commands.len();
            config
                .categories
                .retain(|category| category.id != delete.id);
            config
                .commands
                .retain(|command| command.category_id.as_deref() != Some(delete.id.as_str()));
            let deleted_category = config.categories.len() != before_categories;
            let deleted_commands = before_commands.saturating_sub(config.commands.len());
            store.save_quick_commands(config.clone())?;
            Ok((config, deleted_category, deleted_commands))
        }) {
            Ok((config, deleted_category, deleted_commands)) => {
                self.quick_commands = config.commands;
                self.quick_command_categories = config.categories;
                self.quick_command_category_delete = None;
                if self.quick_command_selected_category == delete.id {
                    self.quick_command_selected_category = "all".to_string();
                }
                if let Some(editor) = self.quick_command_editor.as_mut()
                    && editor.category_id.as_deref() == Some(delete.id.as_str())
                {
                    editor.category_id = None;
                    editor.category_draft.clear();
                }
                self.store_status.message = if deleted_category {
                    format!(
                        "quick command category '{}' deleted with {} command(s)",
                        delete.name, deleted_commands
                    )
                } else {
                    format!(
                        "quick command category '{}' was already deleted",
                        delete.name
                    )
                };
                self.store_status.ready = deleted_category;
                self.terminal_status = self.store_status.message.clone();
            }
            Err(error) => {
                self.store_status.message =
                    format!("quick command category delete failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn open_rename_quick_command_category(
        &mut self,
        category_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(category) = self
            .quick_command_categories
            .iter()
            .find(|category| category.id == category_id)
            .cloned()
        else {
            self.terminal_status = "quick command category is no longer available".to_string();
            cx.notify();
            return;
        };
        self.quick_command_category_rename = Some(QuickCommandCategoryRenameState {
            id: category.id,
            original_name: category.name.clone(),
            draft: category.name,
            error: None,
        });
        self.terminal_status = "quick command category rename opened".to_string();
        window.focus(&self.quick_command_category_rename_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn cancel_rename_quick_command_category(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_category_rename = None;
        self.terminal_status = "quick command category rename cancelled".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn confirm_rename_quick_command_category(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(rename) = self.quick_command_category_rename.clone() else {
            return;
        };
        let name = rename.draft.trim().to_string();
        if name.is_empty() {
            if let Some(state) = self.quick_command_category_rename.as_mut() {
                state.error = Some("Category name is required".to_string());
            }
            cx.notify();
            return;
        }
        if self.quick_command_categories.iter().any(|category| {
            category.id != rename.id && category.name.trim().eq_ignore_ascii_case(name.as_str())
        }) {
            if let Some(state) = self.quick_command_category_rename.as_mut() {
                state.error = Some("A category with this name already exists".to_string());
            }
            cx.notify();
            return;
        }

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| {
            let mut config = store.load_quick_commands()?;
            if config.categories.iter().any(|category| {
                category.id != rename.id && category.name.trim().eq_ignore_ascii_case(name.as_str())
            }) {
                if let Some(state) = self.quick_command_category_rename.as_mut() {
                    state.error = Some("A category with this name already exists".to_string());
                }
                return Ok((config, false));
            }
            let mut renamed = false;
            if let Some(category) = config
                .categories
                .iter_mut()
                .find(|category| category.id == rename.id)
            {
                category.name = name.clone();
                renamed = true;
            }
            store.save_quick_commands(config.clone())?;
            Ok((config, renamed))
        }) {
            Ok((config, renamed)) => {
                self.quick_commands = config.commands;
                self.quick_command_categories = config.categories;
                if renamed {
                    self.quick_command_category_rename = None;
                    self.store_status.message = format!(
                        "quick command category '{}' renamed to '{}'",
                        rename.original_name, name
                    );
                    self.store_status.ready = true;
                } else if let Some(state) = self.quick_command_category_rename.as_mut() {
                    state.error = Some("Category is no longer available".to_string());
                    self.store_status.message =
                        "quick command category rename failed: category missing".to_string();
                    self.store_status.ready = false;
                }
                self.terminal_status = self.store_status.message.clone();
            }
            Err(error) => {
                if let Some(state) = self.quick_command_category_rename.as_mut() {
                    state.error = Some(error.to_string());
                }
                self.store_status.message =
                    format!("quick command category rename failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_quick_command_category_rename_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        let primary = keystroke.modifiers.platform || keystroke.modifiers.control;
        if primary && !keystroke.modifiers.alt && matches!(keystroke.key.as_str(), "v" | "V") {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                if let Some(rename) = self.quick_command_category_rename.as_mut() {
                    rename.draft.push_str(&text);
                    rename.error = None;
                    cx.notify();
                }
            }
            return;
        }
        if primary || keystroke.modifiers.alt || keystroke.modifiers.function {
            return;
        }

        match keystroke.key.as_str() {
            "escape" => self.cancel_rename_quick_command_category(cx),
            "enter" => self.confirm_rename_quick_command_category(cx),
            "backspace" => {
                if let Some(rename) = self.quick_command_category_rename.as_mut() {
                    rename.draft.pop();
                    rename.error = None;
                    cx.notify();
                }
            }
            "space" => {
                if let Some(rename) = self.quick_command_category_rename.as_mut() {
                    rename.draft.push(' ');
                    rename.error = None;
                    cx.notify();
                }
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                    && let Some(rename) = self.quick_command_category_rename.as_mut()
                {
                    rename.draft.push_str(input);
                    rename.error = None;
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::ui::view) fn focus_quick_command_editor_field(
        &mut self,
        field: QuickCommandEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.quick_command_editor.as_mut() {
            editor.focused_field = field;
            editor.error = None;
            window.focus(&self.quick_command_editor_focus);
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn set_quick_command_editor_category(
        &mut self,
        category_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.quick_command_editor.as_mut() {
            editor.category_id = category_id;
            editor.category_draft.clear();
            editor.error = None;
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn confirm_quick_command_editor_category_draft(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.quick_command_editor.as_mut() else {
            return;
        };
        let draft = editor.category_draft.trim().to_string();
        if draft.is_empty() {
            return;
        }
        if let Some(existing) = self
            .quick_command_categories
            .iter()
            .find(|category| category.name.eq_ignore_ascii_case(&draft))
        {
            editor.category_id = Some(existing.id.clone());
            editor.category_draft.clear();
        } else {
            editor.category_id = None;
            editor.category_draft = draft;
        }
        editor.error = None;
        cx.notify();
    }

    pub(in crate::ui::view) fn set_quick_command_editor_color(
        &mut self,
        color_tag: Option<&'static str>,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.quick_command_editor.as_mut() {
            editor.color_tag = color_tag.map(ToOwned::to_owned);
            editor.icon_tag = None;
            editor.error = None;
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn set_quick_command_editor_icon(
        &mut self,
        icon_tag: Option<&'static str>,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.quick_command_editor.as_mut() {
            editor.icon_tag = icon_tag.map(ToOwned::to_owned);
            if icon_tag.is_some() {
                editor.color_tag = None;
            }
            editor.error = None;
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn toggle_quick_command_editor_pinned(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.quick_command_editor.as_mut() {
            editor.pinned = !editor.pinned;
            editor.error = None;
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn set_quick_command_editor_execution_mode(
        &mut self,
        mode: &'static str,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.quick_command_editor.as_mut() {
            editor.execution_mode = if mode == "append" {
                "append".to_string()
            } else {
                "execute".to_string()
            };
            editor.error = None;
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn save_quick_command_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.quick_command_editor.as_mut() else {
            return;
        };
        let label = editor.label.trim().to_string();
        let command_text = editor.command.trim().to_string();
        if label.is_empty() {
            editor.error = Some("Label is required".to_string());
            editor.focused_field = QuickCommandEditorField::Label;
            cx.notify();
            return;
        }
        if command_text.is_empty() {
            editor.error = Some("Command is required".to_string());
            editor.focused_field = QuickCommandEditorField::Command;
            cx.notify();
            return;
        }

        let now = unix_millis_now();
        let original = editor.original.clone();
        let category_draft = editor.category_draft.trim().to_string();
        let (category_id, new_category) = if category_draft.is_empty() {
            (editor.category_id.clone(), None)
        } else if let Some(existing) = self
            .quick_command_categories
            .iter()
            .find(|category| category.name.eq_ignore_ascii_case(&category_draft))
        {
            (Some(existing.id.clone()), None)
        } else {
            let category = QuickCommandCategory {
                id: format!("quick-category-{}", uuid()),
                name: category_draft,
            };
            (Some(category.id.clone()), Some(category))
        };
        let command = QuickCommand {
            id: original
                .as_ref()
                .map(|command| command.id.clone())
                .unwrap_or_else(|| format!("qc-{}", uuid())),
            label,
            command: command_text,
            category_id,
            description: non_empty_string(editor.description.clone()),
            color_tag: editor.color_tag.clone(),
            icon_tag: editor.icon_tag.clone(),
            pinned: editor.pinned.then_some(true),
            execution_mode: Some(editor.execution_mode.clone()),
            source: original.as_ref().and_then(|command| command.source.clone()),
            risk_level: original
                .as_ref()
                .and_then(|command| command.risk_level.clone()),
            updated_at: Some(now),
            created_at: original
                .as_ref()
                .and_then(|command| command.created_at)
                .or(Some(now)),
            use_count: original.as_ref().and_then(|command| command.use_count),
        };

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.upsert_quick_command(command.clone(), new_category))
        {
            Ok(config) => {
                self.quick_commands = config.commands;
                self.quick_command_categories = config.categories;
                self.quick_command_editor = None;
                self.store_status.message = format!("quick command '{}' saved", command.label);
                self.store_status.ready = true;
                self.terminal_status = self.store_status.message.clone();
            }
            Err(error) => {
                if let Some(editor) = self.quick_command_editor.as_mut() {
                    editor.error = Some(error.to_string());
                }
                self.store_status.message = format!("quick command save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_quick_command_editor_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        let primary = keystroke.modifiers.platform || keystroke.modifiers.control;
        if primary && !keystroke.modifiers.alt && matches!(keystroke.key.as_str(), "s" | "S") {
            self.save_quick_command_editor(cx);
            return;
        }
        if primary && !keystroke.modifiers.alt && matches!(keystroke.key.as_str(), "v" | "V") {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                self.quick_command_editor_value_mut().push_str(&text);
                if let Some(editor) = self.quick_command_editor.as_mut() {
                    if editor.focused_field == QuickCommandEditorField::Category {
                        editor.category_id = None;
                    }
                    editor.error = None;
                }
                cx.notify();
            }
            return;
        }
        if primary || keystroke.modifiers.alt || keystroke.modifiers.function {
            return;
        }

        match keystroke.key.as_str() {
            "escape" => self.close_quick_command_editor(cx),
            "enter" => {
                if self
                    .quick_command_editor
                    .as_ref()
                    .is_some_and(|editor| editor.focused_field == QuickCommandEditorField::Command)
                {
                    self.quick_command_editor_value_mut().push('\n');
                    cx.notify();
                } else {
                    self.save_quick_command_editor(cx);
                }
            }
            "tab" => {
                if let Some(editor) = self.quick_command_editor.as_mut() {
                    editor.focused_field = match editor.focused_field {
                        QuickCommandEditorField::Label => QuickCommandEditorField::Command,
                        QuickCommandEditorField::Command => QuickCommandEditorField::Category,
                        QuickCommandEditorField::Category => QuickCommandEditorField::Description,
                        QuickCommandEditorField::Description => QuickCommandEditorField::Label,
                    };
                    editor.error = None;
                    cx.notify();
                }
            }
            "backspace" => {
                self.quick_command_editor_value_mut().pop();
                if let Some(editor) = self.quick_command_editor.as_mut() {
                    if editor.focused_field == QuickCommandEditorField::Category {
                        editor.category_id = None;
                    }
                    editor.error = None;
                }
                cx.notify();
            }
            "space" => {
                self.quick_command_editor_value_mut().push(' ');
                if let Some(editor) = self.quick_command_editor.as_mut() {
                    if editor.focused_field == QuickCommandEditorField::Category {
                        editor.category_id = None;
                    }
                    editor.error = None;
                }
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.quick_command_editor_value_mut().push_str(input);
                    if let Some(editor) = self.quick_command_editor.as_mut() {
                        if editor.focused_field == QuickCommandEditorField::Category {
                            editor.category_id = None;
                        }
                        editor.error = None;
                    }
                    cx.notify();
                }
            }
        }
    }

    fn quick_command_editor_value_mut(&mut self) -> &mut String {
        let editor = self
            .quick_command_editor
            .as_mut()
            .expect("quick command editor should be open while editing");
        match editor.focused_field {
            QuickCommandEditorField::Label => &mut editor.label,
            QuickCommandEditorField::Command => &mut editor.command,
            QuickCommandEditorField::Category => &mut editor.category_draft,
            QuickCommandEditorField::Description => &mut editor.description,
        }
    }
}

fn unix_millis_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().try_into().unwrap_or(u64::MAX))
        .unwrap_or_default()
}

pub(in crate::ui::view) fn sorted_quick_commands(commands: &[QuickCommand]) -> Vec<QuickCommand> {
    let mut commands = commands.to_vec();
    commands.sort_by(|left, right| {
        right
            .pinned
            .unwrap_or_default()
            .cmp(&left.pinned.unwrap_or_default())
            .then_with(|| {
                right
                    .use_count
                    .unwrap_or_default()
                    .cmp(&left.use_count.unwrap_or_default())
            })
            .then_with(|| {
                right
                    .updated_at
                    .unwrap_or_default()
                    .cmp(&left.updated_at.unwrap_or_default())
            })
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });
    commands
}

pub(in crate::ui::view) fn quick_command_view_mode_from_setting(
    value: &str,
) -> QuickCommandViewMode {
    match value.trim() {
        "list" => QuickCommandViewMode::List,
        "compact" => QuickCommandViewMode::Compact,
        _ => QuickCommandViewMode::Tile,
    }
}

pub(in crate::ui::view) fn quick_command_sort_mode_from_setting(
    value: &str,
) -> QuickCommandSortMode {
    match value.trim() {
        "name" => QuickCommandSortMode::Name,
        "useCount" => QuickCommandSortMode::Usage,
        _ => QuickCommandSortMode::Created,
    }
}

fn quick_command_view_mode_setting(mode: QuickCommandViewMode) -> &'static str {
    match mode {
        QuickCommandViewMode::List => "list",
        QuickCommandViewMode::Compact => "compact",
        QuickCommandViewMode::Tile => "tile",
    }
}

fn quick_command_sort_mode_setting(mode: QuickCommandSortMode) -> &'static str {
    match mode {
        QuickCommandSortMode::Created => "created",
        QuickCommandSortMode::Name => "name",
        QuickCommandSortMode::Usage => "useCount",
    }
}

pub(in crate::ui::view) fn quick_command_category_label(
    categories: &[QuickCommandCategory],
    command: &QuickCommand,
) -> String {
    command
        .category_id
        .as_deref()
        .and_then(|id| categories.iter().find(|category| category.id == id))
        .map(|category| category.name.clone())
        .unwrap_or_else(|| "Unsorted".to_string())
}

fn ai_command_card_category_name(card: &AiCommandCard) -> String {
    card.category
        .as_deref()
        .map(str::trim)
        .filter(|category| !category.is_empty())
        .unwrap_or("AI Generated")
        .to_string()
}

fn unique_quick_command_category_id(
    categories: &[QuickCommandCategory],
    category_name: &str,
) -> String {
    let base = format!("ai-{}", quick_command_slug(category_name));
    if !categories.iter().any(|category| category.id == base) {
        return base;
    }
    for suffix in 2.. {
        let candidate = format!("{base}-{suffix}");
        if !categories.iter().any(|category| category.id == candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search always returns")
}

fn quick_command_slug(input: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if matches!(ch, '-' | '_' | ' ' | '\t' | '\n' | '\r') && !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "commands".to_string()
    } else {
        slug.to_string()
    }
}
