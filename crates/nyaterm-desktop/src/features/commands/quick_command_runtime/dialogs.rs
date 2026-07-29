use gpui::{Context, KeyDownEvent, Window};
use nyaterm_core::ConnectionStore;

use crate::features::NyaTermApp;
use crate::models::{
    QuickCommandCategoryDeleteState, QuickCommandCategoryRenameState, QuickCommandDeleteState,
    QuickCommandDetailsState, QuickCommandEditorState,
};

use super::helpers::quick_command_category_label;

impl NyaTermApp {
    pub(in crate::features) fn open_new_quick_command_editor(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.commands
            .open_quick_editor(QuickCommandEditorState::blank());
        // The boxes own their text, so they have to be dropped for the next
        // command to seed from its own values.
        self.forget_text_inputs("quick-command.editor.");
        self.shell
            .set_status("quick command editor opened".to_string());
        if !self.open_quick_command_window(cx) {
            window.focus(self.commands.quick_editor_focus());
        }
        cx.notify();
    }

    pub(in crate::features) fn open_edit_quick_command_editor(
        &mut self,
        command_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(command) = self
            .commands
            .quick_commands()
            .iter()
            .find(|command| command.id == command_id)
            .cloned()
        else {
            self.shell
                .set_status("quick command is no longer available".to_string());
            cx.notify();
            return;
        };
        self.commands
            .open_quick_editor(QuickCommandEditorState::from_command(command));
        // The boxes own their text, so they have to be dropped for the next
        // command to seed from its own values.
        self.forget_text_inputs("quick-command.editor.");
        self.shell
            .set_status("quick command editor opened".to_string());
        if !self.open_quick_command_window(cx) {
            window.focus(self.commands.quick_editor_focus());
        }
        cx.notify();
    }

    pub(in crate::features) fn close_quick_command_editor(&mut self, cx: &mut Context<Self>) {
        self.commands.close_quick_editor();
        self.forget_text_inputs("quick-command.editor.");
        self.shell
            .set_status("quick command editor closed".to_string());
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
        let Some(command) = self
            .commands
            .quick_commands()
            .iter()
            .find(|command| command.id == command_id)
            .cloned()
        else {
            self.shell
                .set_status("quick command is no longer available".to_string());
            cx.notify();
            return;
        };
        self.commands
            .request_quick_details(QuickCommandDetailsState {
                category: quick_command_category_label(
                    self.commands.quick_command_categories(),
                    &command,
                ),
                command,
                x,
                y,
            });
        self.shell
            .set_status("quick command details opened".to_string());
        window.focus(self.commands.quick_details_focus());
        cx.notify();
    }

    pub(in crate::features) fn close_quick_command_details(&mut self, cx: &mut Context<Self>) {
        self.commands.clear_quick_details();
        self.shell
            .set_status("quick command details closed".to_string());
        cx.notify();
    }

    pub(in crate::features) fn open_delete_quick_command_confirm(
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
            self.shell
                .set_status("quick command is no longer available".to_string());
            cx.notify();
            return;
        };
        self.commands.request_quick_delete(QuickCommandDeleteState {
            id: command.id,
            label: command.label,
        });
        self.shell
            .set_status("quick command delete confirmation opened".to_string());
        cx.notify();
    }

    pub(in crate::features) fn cancel_delete_quick_command(&mut self, cx: &mut Context<Self>) {
        self.commands.clear_quick_delete();
        self.shell
            .set_status("quick command delete cancelled".to_string());
        cx.notify();
    }

    pub(in crate::features) fn confirm_delete_quick_command(&mut self, cx: &mut Context<Self>) {
        let Some(delete) = self.commands.quick_delete().cloned() else {
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
                self.commands
                    .replace_quick_command_catalog(config.commands, config.categories);
                self.commands.clear_quick_delete();
                self.settings.update_store_status(
                    if deleted {
                        format!("quick command '{}' deleted", delete.label)
                    } else {
                        format!("quick command '{}' was already deleted", delete.label)
                    },
                    deleted,
                );
                self.shell
                    .set_status(self.settings.store_status().message.to_string());
            }
            Err(error) => {
                self.settings
                    .update_store_status(format!("quick command delete failed: {error}"), false);
                self.shell
                    .set_status(self.settings.store_status().message.to_string());
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
            .commands
            .quick_command_categories()
            .iter()
            .find(|category| category.id == category_id)
            .cloned()
        else {
            self.shell
                .set_status("quick command category is no longer available".to_string());
            cx.notify();
            return;
        };
        let command_count = self
            .commands
            .quick_commands()
            .iter()
            .filter(|command| command.category_id.as_deref() == Some(category.id.as_str()))
            .count();
        self.commands
            .request_quick_category_delete(QuickCommandCategoryDeleteState {
                id: category.id,
                name: category.name,
                command_count,
            });
        self.shell
            .set_status("quick command category delete confirmation opened".to_string());
        cx.notify();
    }

    pub(in crate::features) fn cancel_delete_quick_command_category(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.commands.clear_quick_category_delete();
        self.shell
            .set_status("quick command category delete cancelled".to_string());
        cx.notify();
    }

    pub(in crate::features) fn confirm_delete_quick_command_category(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(delete) = self.commands.quick_category_delete().cloned() else {
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
                self.commands
                    .replace_quick_command_catalog(config.commands, config.categories);
                self.commands.finish_quick_category_delete(&delete.id);
                self.settings.update_store_status(
                    if deleted_category {
                        format!(
                            "quick command category '{}' deleted with {} command(s)",
                            delete.name, deleted_commands
                        )
                    } else {
                        format!(
                            "quick command category '{}' was already deleted",
                            delete.name
                        )
                    },
                    deleted_category,
                );
                self.shell
                    .set_status(self.settings.store_status().message.to_string());
            }
            Err(error) => {
                self.settings.update_store_status(
                    format!("quick command category delete failed: {error}"),
                    false,
                );
                self.shell
                    .set_status(self.settings.store_status().message.to_string());
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
            .commands
            .quick_command_categories()
            .iter()
            .find(|category| category.id == category_id)
            .cloned()
        else {
            self.shell
                .set_status("quick command category is no longer available".to_string());
            cx.notify();
            return;
        };
        self.commands
            .request_quick_category_rename(QuickCommandCategoryRenameState {
                id: category.id,
                original_name: category.name.clone(),
                draft: category.name,
                error: None,
            });
        self.shell
            .set_status("quick command category rename opened".to_string());
        window.focus(self.commands.quick_category_rename_focus());
        cx.notify();
    }

    pub(in crate::features) fn cancel_rename_quick_command_category(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.commands.clear_quick_category_rename();
        self.shell
            .set_status("quick command category rename cancelled".to_string());
        cx.notify();
    }

    pub(in crate::features) fn confirm_rename_quick_command_category(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(rename) = self.commands.quick_category_rename().cloned() else {
            return;
        };
        let name = rename.draft.trim().to_string();
        if name.is_empty() {
            let message = self.tr("quickCommands.categoryNameRequired").to_string();
            self.commands.set_quick_category_rename_error(message);
            cx.notify();
            return;
        }
        if self
            .commands
            .quick_command_categories()
            .iter()
            .any(|category| {
                category.id != rename.id && category.name.trim().eq_ignore_ascii_case(name.as_str())
            })
        {
            let message = self.tr("quickCommands.categoryNameDuplicated").to_string();
            self.commands.set_quick_category_rename_error(message);
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
                self.commands.set_quick_category_rename_error(message);
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
                self.commands
                    .replace_quick_command_catalog(config.commands, config.categories);
                if renamed {
                    self.commands.clear_quick_category_rename();
                    self.settings.update_store_status(
                        format!(
                            "quick command category '{}' renamed to '{}'",
                            rename.original_name, name
                        ),
                        true,
                    );
                } else {
                    self.commands.set_quick_category_rename_error(
                        "Category is no longer available".to_string(),
                    );
                    self.settings.update_store_status(
                        "quick command category rename failed: category missing",
                        false,
                    );
                }
                self.shell
                    .set_status(self.settings.store_status().message.to_string());
            }
            Err(error) => {
                self.commands
                    .set_quick_category_rename_error(error.to_string());
                self.settings.update_store_status(
                    format!("quick command category rename failed: {error}"),
                    false,
                );
                self.shell
                    .set_status(self.settings.store_status().message.to_string());
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
        if keystroke.modifiers.platform
            || keystroke.modifiers.control
            || keystroke.modifiers.alt
            || keystroke.modifiers.function
        {
            return;
        }

        // The box owns the text; the dialog owns the keys that close or confirm.
        match keystroke.key.as_str() {
            "escape" => self.cancel_rename_quick_command_category(cx),
            "enter" => self.confirm_rename_quick_command_category(cx),
            _ => {}
        }
    }

    /// Apply an edit from the category rename box.
    pub(in crate::features) fn apply_quick_command_category_rename(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        if self.commands.apply_quick_category_rename(text) {
            cx.notify();
        }
    }
}
