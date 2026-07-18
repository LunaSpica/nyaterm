use super::*;

impl NyaTermApp {
    pub(in crate::features) fn open_new_quick_command_editor(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_menu = None;
        self.quick_command_editor = Some(QuickCommandEditorState::blank());
        self.terminal_status = "quick command editor opened".to_string();
        if !self.open_quick_command_window(cx) {
            window.focus(&self.quick_command_editor_focus);
        }
        cx.notify();
    }

    pub(in crate::features) fn open_edit_quick_command_editor(
        &mut self,
        command_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_menu = None;
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
        if !self.open_quick_command_window(cx) {
            window.focus(&self.quick_command_editor_focus);
        }
        cx.notify();
    }

    pub(in crate::features) fn close_quick_command_editor(&mut self, cx: &mut Context<Self>) {
        self.quick_command_editor = None;
        self.quick_command_window = None;
        self.terminal_status = "quick command editor closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn open_quick_command_details(
        &mut self,
        command_id: String,
        x: gpui::Pixels,
        y: gpui::Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_menu = None;
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
            command,
            x,
            y,
        });
        self.terminal_status = "quick command details opened".to_string();
        window.focus(&self.quick_command_details_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_quick_command_details(&mut self, cx: &mut Context<Self>) {
        self.quick_command_details = None;
        self.terminal_status = "quick command details closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn open_delete_quick_command_confirm(
        &mut self,
        command_id: String,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_menu = None;
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

    pub(in crate::features) fn cancel_delete_quick_command(&mut self, cx: &mut Context<Self>) {
        self.quick_command_delete = None;
        self.terminal_status = "quick command delete cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn confirm_delete_quick_command(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn open_delete_quick_command_category_confirm(
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

    pub(in crate::features) fn cancel_delete_quick_command_category(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_category_delete = None;
        self.terminal_status = "quick command category delete cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn confirm_delete_quick_command_category(
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

    pub(in crate::features) fn open_rename_quick_command_category(
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

    pub(in crate::features) fn cancel_rename_quick_command_category(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_category_rename = None;
        self.terminal_status = "quick command category rename cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn confirm_rename_quick_command_category(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(rename) = self.quick_command_category_rename.clone() else {
            return;
        };
        let name = rename.draft.trim().to_string();
        if name.is_empty() {
            let message = self.tr("quickCommands.categoryNameRequired").to_string();
            if let Some(state) = self.quick_command_category_rename.as_mut() {
                state.error = Some(message);
            }
            cx.notify();
            return;
        }
        if self.quick_command_categories.iter().any(|category| {
            category.id != rename.id && category.name.trim().eq_ignore_ascii_case(name.as_str())
        }) {
            let message = self.tr("quickCommands.categoryNameDuplicated").to_string();
            if let Some(state) = self.quick_command_category_rename.as_mut() {
                state.error = Some(message);
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
                let message = self.tr("quickCommands.categoryNameDuplicated").to_string();
                if let Some(state) = self.quick_command_category_rename.as_mut() {
                    state.error = Some(message);
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

    pub(in crate::features) fn handle_quick_command_category_rename_key_down(
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
}
