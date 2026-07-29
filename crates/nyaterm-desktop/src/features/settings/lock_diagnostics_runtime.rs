use gpui::{AppContext, Context, KeyDownEvent, Window};
use nyaterm_core::{
    ConnectionStore, DiagnosticsExportOptions, DiagnosticsRuntimeSnapshot,
    export_diagnostics_archive,
};
use nyaterm_transport::SessionKind;

use crate::features::{NyaTermApp, TextInputSetup};
use crate::models::DiagnosticsPathPromptResult;
use crate::models::TransferJobStatus;

impl NyaTermApp {
    pub(in crate::features) fn lock_app(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let lock_status = if self.settings.summary.has_master_password {
            self.tr("lockScreen.passwordPlaceholder").to_string()
        } else {
            String::new()
        };
        self.security.activate_screen_lock(lock_status);
        self.forget_text_inputs("lock-screen.password");
        self.shell.status = "screen locked".to_string();
        if self.settings.summary.has_master_password {
            let field = self.text_input("lock-screen.password", "", TextInputSetup::masked(), cx);
            window.focus(&field.read(cx).focus_handle());
        } else {
            window.focus(self.security.screen_lock_focus());
        }
        cx.notify();
    }

    pub(in crate::features) fn unlock_app(&mut self, cx: &mut Context<Self>) {
        self.security.deactivate_screen_lock();
        self.forget_text_inputs("lock-screen.password");
        self.shell.status = "screen unlocked".to_string();
        cx.notify();
    }

    pub(in crate::features) fn submit_lock_unlock(&mut self, cx: &mut Context<Self>) {
        if !self.settings.summary.has_master_password {
            self.unlock_app(cx);
            return;
        }

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.verify_master_password(self.security.screen_lock_password_draft()))
        {
            Ok(true) => self.unlock_app(cx),
            Ok(false) => {
                let status = self.tr("lockScreen.wrongPassword").to_string();
                self.security.clear_screen_lock_password_with_status(status);
                self.reset_text_input("lock-screen.password", "", cx);
                self.shell.status = "screen unlock rejected".to_string();
                cx.notify();
            }
            Err(error) => {
                let status = format!("{}: {error}", self.tr("lockScreen.unlockFailed"));
                self.security.clear_screen_lock_password_with_status(status);
                self.reset_text_input("lock-screen.password", "", cx);
                self.shell.status = "screen unlock failed".to_string();
                cx.notify();
            }
        }
    }

    pub(in crate::features) fn handle_lock_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "enter" => self.submit_lock_unlock(cx),
            "escape" if !self.settings.summary.has_master_password => self.unlock_app(cx),
            "escape" => {
                let status = self.tr("lockScreen.passwordPlaceholder").to_string();
                self.security.clear_screen_lock_password_with_status(status);
                self.reset_text_input("lock-screen.password", "", cx);
                cx.notify();
            }
            _ => {}
        }
    }

    pub(in crate::features) fn apply_lock_password_input(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let status = self.tr("lockScreen.passwordPlaceholder").to_string();
        self.security.set_screen_lock_password_draft(text, status);
        cx.notify();
    }

    pub(in crate::features) fn reveal_log_dir(&mut self, cx: &mut Context<Self>) {
        match std::fs::create_dir_all(self.runtime.log_dir()) {
            Ok(()) => {
                cx.reveal_path(self.runtime.log_dir());
                self.shell.status =
                    format!("opened log directory {}", self.runtime.log_dir().display());
            }
            Err(error) => {
                self.shell.status = format!("failed to prepare log directory: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn prompt_diagnostics_export(&mut self, cx: &mut Context<Self>) {
        if !self.settings.begin_diagnostics_path_prompt() {
            self.shell.status = "diagnostics path picker is already open".to_string();
            cx.notify();
            return;
        }

        let directory = self.runtime.log_dir().to_path_buf();
        let receiver = cx.prompt_for_new_path(&directory, Some("nyaterm-diagnostics.zip"));
        let runtime = self.runtime.clone();
        let options = self.diagnostics_export_options();
        self.shell.status = "selecting diagnostics export destination".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(path))) => {
                    cx.background_spawn(async move {
                        match export_diagnostics_archive(&runtime, &options, &path) {
                            Ok(info) => DiagnosticsPathPromptResult::Exported(info),
                            Err(error) => DiagnosticsPathPromptResult::Failed(error.to_string()),
                        }
                    })
                    .await
                }
                Ok(Ok(None)) => DiagnosticsPathPromptResult::Cancelled,
                Ok(Err(error)) => DiagnosticsPathPromptResult::Failed(error.to_string()),
                Err(_) => DiagnosticsPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_diagnostics_path_prompt_result(result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features) fn apply_diagnostics_path_prompt_result(
        &mut self,
        result: DiagnosticsPathPromptResult,
    ) {
        if !self.settings.finish_diagnostics_path_prompt() {
            return;
        }
        match result {
            DiagnosticsPathPromptResult::Exported(info) => {
                self.shell.status = format!(
                    "diagnostics exported to {} ({} log file(s), {} bytes)",
                    info.output_path.display(),
                    info.log_files,
                    info.bytes
                );
            }
            DiagnosticsPathPromptResult::Cancelled => {
                self.shell.status = "diagnostics export cancelled".to_string();
            }
            DiagnosticsPathPromptResult::Failed(error) => {
                self.shell.status = format!("diagnostics export failed: {error}");
            }
            DiagnosticsPathPromptResult::Closed => {
                self.shell.status = "diagnostics path picker closed before returning".to_string();
            }
        }
    }

    pub(in crate::features) fn diagnostics_export_options(&self) -> DiagnosticsExportOptions {
        DiagnosticsExportOptions {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            language: self.settings.summary.language.clone(),
            log_level: self.settings.summary.diagnostics_level.clone(),
            retention_days: self.settings.summary.diagnostics_retention_days,
            runtime_snapshot: self.diagnostics_runtime_snapshot(),
        }
    }

    pub(in crate::features) fn diagnostics_runtime_snapshot(&self) -> DiagnosticsRuntimeSnapshot {
        let sessions = self.ordered_sessions();
        let mut local_sessions = 0;
        let mut ssh_sessions = 0;
        let mut telnet_sessions = 0;
        let mut raw_tcp_sessions = 0;
        let mut serial_sessions = 0;
        for session in &sessions {
            match session.kind {
                SessionKind::LocalPty => local_sessions += 1,
                SessionKind::Ssh => ssh_sessions += 1,
                SessionKind::Telnet => telnet_sessions += 1,
                SessionKind::RawTcp => raw_tcp_sessions += 1,
                SessionKind::Serial => serial_sessions += 1,
            }
        }

        let open_tunnels = self.tunnel_state.open_count();
        let mut running_transfers = 0;
        let mut paused_transfers = 0;
        let mut completed_transfers = 0;
        let mut failed_transfers = 0;
        for job in self.transfer.transfer_jobs() {
            match job.status {
                TransferJobStatus::Running | TransferJobStatus::Cancelling => {
                    running_transfers += 1
                }
                TransferJobStatus::Paused => paused_transfers += 1,
                TransferJobStatus::Completed => completed_transfers += 1,
                TransferJobStatus::Failed => failed_transfers += 1,
                TransferJobStatus::Cancelled => {}
            }
        }

        DiagnosticsRuntimeSnapshot {
            active_sessions: sessions.len(),
            local_sessions,
            ssh_sessions,
            telnet_sessions,
            raw_tcp_sessions,
            serial_sessions,
            open_tunnels,
            pending_tunnels: self.tunnel_state.pending_count(),
            saved_connections: self.connection_state.connections().len(),
            saved_tunnels: self.tunnel_state.tunnels().len(),
            running_transfers,
            paused_transfers,
            completed_transfers,
            failed_transfers,
        }
    }
}
