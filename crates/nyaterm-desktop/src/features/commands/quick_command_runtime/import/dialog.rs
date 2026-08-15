use gpui::{AppContext, Context, IntoElement, PathPromptOptions, SharedString, Window};
use nyaterm_core::export_quick_commands_json;
use nyaterm_store::ConnectionStore;
use nyaterm_ui::NyaDialogWindowExt as _;

use crate::features::NyaTermApp;
use crate::models::{QuickCommandImportPathPromptKind, QuickCommandImportPathPromptResult};

use super::sources::import_quick_commands_from_path;

impl NyaTermApp {
    pub(in crate::features) fn open_quick_command_import_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.commands.quick_import_path_prompt().is_some() || window.has_active_nya_dialog(cx) {
            self.shell
                .set_status("quick command import picker is already open".to_string());
            cx.notify();
            return;
        }

        self.shell
            .set_status("select a quick command import source".to_string());
        self.open_content_dialog(
            self.tr("quickCommands.importTitle").to_string(),
            380.,
            |app, _, cx| {
                app.quick_command_import_dialog_content(cx)
                    .into_any_element()
            },
            |_, _| {},
            window,
            cx,
        );
        cx.notify();
    }

    pub(in crate::features) fn select_quick_command_import_source(
        &mut self,
        kind: QuickCommandImportPathPromptKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.close_nya_dialog(cx);
        self.prompt_quick_command_import(kind, cx);
    }

    pub(in crate::features) fn prompt_quick_command_export(&mut self, cx: &mut Context<Self>) {
        let directory = self.runtime.config_dir().to_path_buf();
        let receiver = cx.prompt_for_new_path(&directory, Some("nyaterm-quick-commands.json"));
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.shell
            .set_status("selecting quick command export destination".to_string());
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(path))) => {
                    cx.background_spawn(async move {
                        let store = ConnectionStore::open_with_portable_key_path(
                            &config_dir,
                            portable_key_path,
                        )
                        .map_err(|error| error.to_string())?;
                        let config = store
                            .load_quick_commands()
                            .map_err(|error| error.to_string())?;
                        let raw = export_quick_commands_json(config)
                            .map_err(|error| error.to_string())?;
                        std::fs::write(&path, raw).map_err(|error| error.to_string())?;
                        Ok::<_, String>(path)
                    })
                    .await
                }
                Ok(Ok(None)) => Err("cancelled".to_string()),
                Ok(Err(error)) => Err(error.to_string()),
                Err(_) => Err("closed".to_string()),
            };
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(path) => {
                        this.shell
                            .set_status(format!("quick commands exported to {}", path.display()));
                        this.settings
                            .update_store_status(this.shell.status().to_string(), true);
                    }
                    Err(error) if error == "cancelled" => {
                        this.shell
                            .set_status("quick command export cancelled".to_string());
                    }
                    Err(error) if error == "closed" => {
                        this.shell.set_status(
                            "quick command export picker closed before returning".to_string(),
                        );
                    }
                    Err(error) => {
                        this.shell
                            .set_status(format!("quick command export failed: {error}"));
                        this.settings
                            .update_store_status(this.shell.status().to_string(), false);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn prompt_quick_command_import(
        &mut self,
        kind: QuickCommandImportPathPromptKind,
        cx: &mut Context<Self>,
    ) {
        if !self.commands.request_quick_import_path(kind) {
            self.shell
                .set_status("quick command import picker is already open".to_string());
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
        self.shell.set_status(kind.selecting_status().to_string());

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
                this.apply_quick_command_import_result(result, cx);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_quick_command_import_result(
        &mut self,
        result: QuickCommandImportPathPromptResult,
        cx: &mut Context<Self>,
    ) {
        self.commands.finish_quick_import_path();
        match result {
            QuickCommandImportPathPromptResult::Imported {
                imported_commands,
                imported_categories,
                updated_commands,
                total_commands,
                total_categories,
            } => {
                self.refresh_quick_commands(cx);
                self.shell.set_status(format!(
                    "imported {imported_commands} quick command(s), updated {updated_commands}, categories +{imported_categories}, total {total_commands}/{total_categories}"
                ));
                self.settings
                    .update_store_status(self.shell.status().to_string(), true);
            }
            QuickCommandImportPathPromptResult::Cancelled => {
                self.shell
                    .set_status("quick command import cancelled".to_string());
            }
            QuickCommandImportPathPromptResult::Failed(error) => {
                self.shell
                    .set_status(format!("quick command import failed: {error}"));
                self.settings
                    .update_store_status(self.shell.status().to_string(), false);
            }
            QuickCommandImportPathPromptResult::Closed => {
                self.shell
                    .set_status("quick command import picker closed before returning".to_string());
            }
        }
    }
}
