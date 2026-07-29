use gpui::{AppContext, Context, PathPromptOptions, SharedString, Window};
use nyaterm_core::ConnectionStore;

use crate::features::NyaTermApp;
use crate::models::ConnectionImportSource;

enum ConnectionImportResult {
    Imported(usize),
    Cancelled,
    Failed(String),
    Closed,
}

impl ConnectionImportSource {
    fn prompt_label(self) -> &'static str {
        match self {
            Self::NyatermBackup => "Import NyaTerm backup",
            Self::Xshell => "Import Xshell .xts sessions",
            Self::MobaXterm => "Import MobaXterm .mxtsessions sessions",
            Self::WindTerm => "Import WindTerm .sessions file",
            Self::NyatermJson => "Import NyaTerm sessions JSON",
        }
    }

    fn selecting_status(self) -> &'static str {
        match self {
            Self::NyatermBackup => "selecting NyaTerm backup",
            Self::Xshell => "selecting Xshell session import file",
            Self::MobaXterm => "selecting MobaXterm session import file",
            Self::WindTerm => "selecting WindTerm session import file",
            Self::NyatermJson => "selecting NyaTerm session JSON",
        }
    }
}

impl NyaTermApp {
    pub(in crate::features) fn open_connection_import_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.connection_state.import_path_prompt_active()
            || self.settings.config_path_prompt_active()
        {
            self.shell.status = "connection import picker is already open".to_string();
            cx.notify();
            return;
        }

        self.connection_state.open_import_dialog();
        self.connection_state.close_list_more_menu();
        self.shell.close_title_menus();
        self.shell.status = "select a connection import source".to_string();
        let import_focus = self.connection_state.import_focus_handle();
        window.focus(&import_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_connection_import_dialog(&mut self, cx: &mut Context<Self>) {
        self.connection_state.close_import_dialog();
        cx.notify();
    }

    pub(in crate::features) fn select_connection_import_source(
        &mut self,
        source: ConnectionImportSource,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.close_import_dialog();
        if source == ConnectionImportSource::NyatermBackup {
            self.prompt_portable_snapshot_import(cx);
            return;
        }
        self.prompt_connection_session_import(source, cx);
    }

    fn prompt_connection_session_import(
        &mut self,
        source: ConnectionImportSource,
        cx: &mut Context<Self>,
    ) {
        if self.connection_state.import_path_prompt_active() {
            self.shell.status = "connection import picker is already open".to_string();
            cx.notify();
            return;
        }

        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from(source.prompt_label())),
        });
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.connection_state.begin_import_path_prompt(source);
        self.shell.status = source.selecting_status().to_string();

        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => match paths.into_iter().next() {
                    Some(path) => {
                        cx.background_spawn(async move {
                            match ConnectionStore::open_with_portable_key_path(
                                &config_dir,
                                portable_key_path,
                            ) {
                                Ok(store) => match nyaterm_core::import_sessions(&store, &path) {
                                    Ok(count) => ConnectionImportResult::Imported(count),
                                    Err(error) => ConnectionImportResult::Failed(error.to_string()),
                                },
                                Err(error) => ConnectionImportResult::Failed(error.to_string()),
                            }
                        })
                        .await
                    }
                    None => ConnectionImportResult::Cancelled,
                },
                Ok(Ok(None)) => ConnectionImportResult::Cancelled,
                Ok(Err(error)) => ConnectionImportResult::Failed(error.to_string()),
                Err(_) => ConnectionImportResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_connection_import_result(result, cx);
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_connection_import_result(
        &mut self,
        result: ConnectionImportResult,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.finish_import_path_prompt();
        match result {
            ConnectionImportResult::Imported(count) => {
                self.refresh_store_from_runtime();
                self.connection_state.expand_list_groups(
                    self.connection_catalog
                        .groups()
                        .iter()
                        .map(|group| group.id.clone()),
                );
                let message = self
                    .tr("savedConnections.importSuccess")
                    .replace("{{count}}", &count.to_string());
                self.shell.status = message.clone();
                self.settings.set_store_message(message);
                self.settings.set_store_ready(true);
            }
            ConnectionImportResult::Cancelled => {
                self.shell.status = "connection import cancelled".to_string();
            }
            ConnectionImportResult::Failed(error) => {
                let message = self
                    .tr("savedConnections.importFailed")
                    .replace("{{error}}", &error);
                self.shell.status = message.clone();
                self.settings.set_store_message(message);
                self.settings.set_store_ready(false);
            }
            ConnectionImportResult::Closed => {
                self.shell.status = "connection import picker closed".to_string();
            }
        }
        cx.notify();
    }
}
