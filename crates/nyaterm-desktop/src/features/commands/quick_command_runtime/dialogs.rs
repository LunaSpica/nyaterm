use gpui::{Context, ParentElement as _, Window, div};
use nyaterm_core::ConnectionStore;
use nyaterm_ui::{NyaConfirmDialog, NyaDialogFooter, NyaDialogWindowExt};

use crate::features::NyaTermApp;
use crate::models::{
    QuickCommandCategoryDeleteState, QuickCommandCategoryRenameState, QuickCommandDetailsState,
    QuickCommandEditorState,
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
            window.focus(self.commands.quick_editor_focus(), cx);
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
            window.focus(self.commands.quick_editor_focus(), cx);
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
        window.focus(self.commands.quick_details_focus(), cx);
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
        self.shell
            .set_status("quick command delete confirmation opened".to_string());
        let title = self.tr("quickCommands.delete").to_string();
        let message = self
            .tr("quickCommands.deleteConfirm")
            .replace("{{name}}", &command.label);
        let cancel_label = self.tr("common.cancel").to_string();
        let delete_label = self.tr("common.delete").to_string();
        let app = cx.weak_entity();
        let command_id = command.id.clone();
        let command_label = command.label.clone();
        window.open_nya_dialog(cx, move |dialog, _, _| {
            let confirm_app = app.clone();
            let command_id = command_id.clone();
            let command_label = command_label.clone();
            NyaConfirmDialog::new(
                dialog.title(title.clone()).width(384.),
                NyaDialogFooter::new(cancel_label.clone(), delete_label.clone()).danger(),
            )
            .content(div().child(message.clone()))
            .on_confirm(move |_, _, cx| {
                confirm_app
                    .update(cx, |app, cx| {
                        app.confirm_delete_quick_command(
                            command_id.clone(),
                            command_label.clone(),
                            cx,
                        )
                    })
                    .is_ok()
            })
            .on_cancel(|_, _, _| true)
            .into_dialog()
        });
        cx.notify();
    }

    fn confirm_delete_quick_command(
        &mut self,
        command_id: String,
        command_label: String,
        cx: &mut Context<Self>,
    ) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| {
            let mut config = store.load_quick_commands()?;
            let before = config.commands.len();
            config.commands.retain(|command| command.id != command_id);
            let deleted = config.commands.len() != before;
            store.save_quick_commands(config.clone())?;
            Ok((config, deleted))
        }) {
            Ok((config, deleted)) => {
                self.commands
                    .replace_quick_command_catalog(config.commands, config.categories);
                self.settings.update_store_status(
                    if deleted {
                        format!("quick command '{command_label}' deleted")
                    } else {
                        format!("quick command '{command_label}' was already deleted")
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
        let command_count = self
            .commands
            .quick_commands()
            .iter()
            .filter(|command| command.category_id.as_deref() == Some(category.id.as_str()))
            .count();
        let category_name = category.name.clone();
        self.commands
            .request_quick_category_delete(QuickCommandCategoryDeleteState {
                id: category.id,
                name: category_name.clone(),
            });
        self.shell
            .set_status("quick command category delete confirmation opened".to_string());
        let title = self.tr("quickCommands.deleteCategory").to_string();
        let message = self
            .tr("quickCommands.deleteCategoryConfirm")
            .replace("{{name}}", &category_name)
            .replace("{{count}}", &command_count.to_string());
        self.open_confirm_dialog_with_cancel(
            (
                title,
                message,
                self.tr("common.delete").to_string(),
                true,
                |app, _, cx| app.confirm_delete_quick_command_category(cx),
                |app, cx| app.cancel_delete_quick_command_category(cx),
            ),
            window,
            cx,
        );
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
    ) -> bool {
        let Some(delete) = self.commands.quick_category_delete().cloned() else {
            return true;
        };
        let succeeded = match ConnectionStore::open_with_portable_key_path(
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
                true
            }
            Err(error) => {
                self.settings.update_store_status(
                    format!("quick command category delete failed: {error}"),
                    false,
                );
                self.shell
                    .set_status(self.settings.store_status().message.to_string());
                false
            }
        };
        cx.notify();
        succeeded
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
        self.open_form_dialog(
            (
                self.tr("quickCommands.renameCategory").to_string(),
                384.,
                self.tr("common.confirm").to_string(),
                |app, _, cx| app.quick_command_category_rename_dialog_content(cx),
                |app, _, cx| app.confirm_rename_quick_command_category(cx),
                |app, cx| app.cancel_rename_quick_command_category(cx),
            ),
            window,
            cx,
        );
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
    ) -> bool {
        let Some(rename) = self.commands.quick_category_rename().cloned() else {
            return true;
        };
        let name = rename.draft.trim().to_string();
        if name.is_empty() {
            let message = self.tr("quickCommands.categoryNameRequired").to_string();
            self.commands.set_quick_category_rename_error(message);
            cx.notify();
            return false;
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
            return false;
        }

        let renamed = match ConnectionStore::open_with_portable_key_path(
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
                renamed
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
                false
            }
        };
        cx.notify();
        renamed
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
