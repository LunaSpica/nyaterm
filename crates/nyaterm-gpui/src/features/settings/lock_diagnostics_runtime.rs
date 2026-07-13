use super::*;

impl NyaTermApp {
    pub(in crate::features) fn lock_app(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.is_locked = true;
        self.lock_password_draft.clear();
        self.lock_status = if self.settings.has_master_password {
            "Enter the master password to unlock.".to_string()
        } else {
            "No master password is configured.".to_string()
        };
        self.terminal_status = "screen locked".to_string();
        window.focus(&self.lock_focus);
        cx.notify();
    }

    pub(in crate::features) fn unlock_app(&mut self, cx: &mut Context<Self>) {
        self.is_locked = false;
        self.lock_password_draft.clear();
        self.lock_status.clear();
        self.last_user_activity_at = Instant::now();
        self.terminal_status = "screen unlocked".to_string();
        cx.notify();
    }

    pub(in crate::features) fn submit_lock_unlock(&mut self, cx: &mut Context<Self>) {
        if !self.settings.has_master_password {
            self.unlock_app(cx);
            return;
        }

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.verify_master_password(&self.lock_password_draft))
        {
            Ok(true) => self.unlock_app(cx),
            Ok(false) => {
                self.lock_password_draft.clear();
                self.lock_status = "Wrong master password.".to_string();
                self.terminal_status = "screen unlock rejected".to_string();
                cx.notify();
            }
            Err(error) => {
                self.lock_password_draft.clear();
                self.lock_status = format!("Unlock failed: {error}");
                self.terminal_status = "screen unlock failed".to_string();
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
            "escape" if !self.settings.has_master_password => self.unlock_app(cx),
            "escape" => {
                self.lock_password_draft.clear();
                self.lock_status = "Enter the master password to unlock.".to_string();
                cx.notify();
            }
            "backspace" => {
                self.lock_password_draft.pop();
                cx.notify();
            }
            _ if self.settings.has_master_password => {
                if let Some(value) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|value| !value.is_empty())
                {
                    self.lock_password_draft.push_str(value);
                    self.lock_status = "Enter the master password to unlock.".to_string();
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    pub(in crate::features) fn reveal_log_dir(&mut self, cx: &mut Context<Self>) {
        match std::fs::create_dir_all(self.runtime.log_dir()) {
            Ok(()) => {
                cx.reveal_path(self.runtime.log_dir());
                self.terminal_status =
                    format!("opened log directory {}", self.runtime.log_dir().display());
            }
            Err(error) => {
                self.terminal_status = format!("failed to prepare log directory: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn prompt_diagnostics_export(&mut self, cx: &mut Context<Self>) {
        if self.diagnostics_path_prompt.is_some() {
            self.terminal_status = "diagnostics path picker is already open".to_string();
            cx.notify();
            return;
        }

        let directory = self.runtime.log_dir().to_path_buf();
        let receiver = cx.prompt_for_new_path(&directory, Some("nyaterm-diagnostics.zip"));
        let runtime = self.runtime.clone();
        let options = self.diagnostics_export_options();
        self.diagnostics_path_prompt = Some(DiagnosticsPathPromptKind::Export);
        self.terminal_status = "selecting diagnostics export destination".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(path))) => match export_diagnostics_archive(&runtime, &options, &path) {
                    Ok(info) => DiagnosticsPathPromptResult::Exported(info),
                    Err(error) => DiagnosticsPathPromptResult::Failed(error.to_string()),
                },
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
        self.diagnostics_path_prompt = None;
        match result {
            DiagnosticsPathPromptResult::Exported(info) => {
                self.terminal_status = format!(
                    "diagnostics exported to {} ({} log file(s), {} bytes)",
                    info.output_path.display(),
                    info.log_files,
                    info.bytes
                );
            }
            DiagnosticsPathPromptResult::Cancelled => {
                self.terminal_status = "diagnostics export cancelled".to_string();
            }
            DiagnosticsPathPromptResult::Failed(error) => {
                self.terminal_status = format!("diagnostics export failed: {error}");
            }
            DiagnosticsPathPromptResult::Closed => {
                self.terminal_status =
                    "diagnostics path picker closed before returning".to_string();
            }
        }
    }

    pub(in crate::features) fn diagnostics_export_options(&self) -> DiagnosticsExportOptions {
        DiagnosticsExportOptions {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            language: self.settings.language.clone(),
            log_level: self.settings.diagnostics_level.clone(),
            retention_days: self.settings.diagnostics_retention_days,
            runtime_snapshot: self.diagnostics_runtime_snapshot(),
        }
    }

    pub(in crate::features) fn diagnostics_runtime_snapshot(&self) -> DiagnosticsRuntimeSnapshot {
        let sessions = self.session_manager.list_sessions().unwrap_or_default();
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

        let open_tunnels = self
            .tunnel_manager
            .list()
            .map(|items| items.len())
            .unwrap_or(0);
        let mut running_transfers = 0;
        let mut paused_transfers = 0;
        let mut completed_transfers = 0;
        let mut failed_transfers = 0;
        for job in &self.transfer_jobs {
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
            pending_tunnels: self.pending_tunnels.len(),
            saved_connections: self.connections.len(),
            saved_tunnels: self.tunnels.len(),
            running_transfers,
            paused_transfers,
            completed_transfers,
            failed_transfers,
        }
    }
}
