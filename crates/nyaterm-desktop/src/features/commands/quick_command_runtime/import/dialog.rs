use gpui::{AppContext, Context, PathPromptOptions, SharedString, Window};

use crate::features::NyaTermApp;
use crate::models::{QuickCommandImportPathPromptKind, QuickCommandImportPathPromptResult};

use super::sources::import_quick_commands_from_path;

impl NyaTermApp {
    pub(in crate::features) fn open_quick_command_import_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.commands.open_quick_import_dialog() {
            self.shell.status = "quick command import picker is already open".to_string();
            cx.notify();
            return;
        }

        self.shell.status = "select a quick command import source".to_string();
        window.focus(self.commands.quick_import_focus());
        cx.notify();
    }

    pub(in crate::features) fn close_quick_command_import_dialog(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.commands.close_quick_import_dialog();
        cx.notify();
    }

    pub(in crate::features) fn select_quick_command_import_source(
        &mut self,
        kind: QuickCommandImportPathPromptKind,
        cx: &mut Context<Self>,
    ) {
        self.prompt_quick_command_import(kind, cx);
    }

    fn prompt_quick_command_import(
        &mut self,
        kind: QuickCommandImportPathPromptKind,
        cx: &mut Context<Self>,
    ) {
        if !self.commands.request_quick_import_path(kind) {
            self.shell.status = "quick command import picker is already open".to_string();
            cx.notify();
            return;
        }

        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from(kind.prompt_label())),
        };
        let receiver = cx.prompt_for_paths(options);
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.shell.status = kind.selecting_status().to_string();

        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => match paths.into_iter().next() {
                    Some(path) => {
                        cx.background_spawn(async move {
                            match import_quick_commands_from_path(
                                &config_dir,
                                portable_key_path,
                                kind,
                                &path,
                            ) {
                                Ok(summary) => QuickCommandImportPathPromptResult::Imported {
                                    imported_commands: summary.imported_commands,
                                    imported_categories: summary.imported_categories,
                                    updated_commands: summary.updated_commands,
                                    total_commands: summary.total_commands,
                                    total_categories: summary.total_categories,
                                },
                                Err(error) => QuickCommandImportPathPromptResult::Failed(error),
                            }
                        })
                        .await
                    }
                    None => QuickCommandImportPathPromptResult::Cancelled,
                },
                Ok(Ok(None)) => QuickCommandImportPathPromptResult::Cancelled,
                Ok(Err(error)) => QuickCommandImportPathPromptResult::Failed(error.to_string()),
                Err(_) => QuickCommandImportPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_quick_command_import_result(result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_quick_command_import_result(&mut self, result: QuickCommandImportPathPromptResult) {
        self.commands.finish_quick_import_path();
        match result {
            QuickCommandImportPathPromptResult::Imported {
                imported_commands,
                imported_categories,
                updated_commands,
                total_commands,
                total_categories,
            } => {
                self.refresh_quick_commands();
                self.shell.status = format!(
                    "imported {imported_commands} quick command(s), updated {updated_commands}, categories +{imported_categories}, total {total_commands}/{total_categories}"
                );
                self.settings.set_store_message(self.shell.status.clone());
                self.settings.set_store_ready(true);
            }
            QuickCommandImportPathPromptResult::Cancelled => {
                self.shell.status = "quick command import cancelled".to_string();
            }
            QuickCommandImportPathPromptResult::Failed(error) => {
                self.shell.status = format!("quick command import failed: {error}");
                self.settings.set_store_message(self.shell.status.clone());
                self.settings.set_store_ready(false);
            }
            QuickCommandImportPathPromptResult::Closed => {
                self.shell.status =
                    "quick command import picker closed before returning".to_string();
            }
        }
    }
}
