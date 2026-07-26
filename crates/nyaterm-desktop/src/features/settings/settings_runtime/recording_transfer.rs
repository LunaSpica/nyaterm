use super::*;

impl NyaTermApp {
    pub(in crate::features) fn update_host_key_policy(
        &mut self,
        policy: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.host_key_policy = policy.to_string();
        if self.defer_settings_persistence(cx) {
            self.terminal_status = format!("host key policy staged as {policy}");
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_host_key_policy(policy))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.terminal_status = format!("host key policy set to {policy}");
                self.store_status.message = "settings saved".to_string();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.terminal_status = format!("failed to save host key policy: {error}");
                self.store_status.message = format!("settings save failed: {error}");
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn toggle_recording_auto_start(&mut self, cx: &mut Context<Self>) {
        self.settings.recording_auto_start = !self.settings.recording_auto_start;
        self.save_recording_settings(cx);
    }

    pub(in crate::features) fn toggle_recording_io_labels(&mut self, cx: &mut Context<Self>) {
        self.settings.recording_include_io_labels = !self.settings.recording_include_io_labels;
        self.save_recording_settings(cx);
    }

    pub(in crate::features) fn toggle_recording_timestamps(&mut self, cx: &mut Context<Self>) {
        self.settings.recording_include_timestamps = !self.settings.recording_include_timestamps;
        self.save_recording_settings(cx);
    }

    pub(in crate::features) fn adjust_recording_memory_limit(
        &mut self,
        delta_mib: i64,
        cx: &mut Context<Self>,
    ) {
        let current_mib = (self.settings.recording_memory_limit_bytes / (1024 * 1024)).max(1);
        let next_mib = if delta_mib.is_negative() {
            current_mib.saturating_sub(delta_mib.unsigned_abs()).max(1)
        } else {
            current_mib.saturating_add(delta_mib as u64).min(512)
        };
        self.settings.recording_memory_limit_bytes = next_mib * 1024 * 1024;
        self.save_recording_settings(cx);
    }

    pub(in crate::features) fn save_recording_settings(&mut self, cx: &mut Context<Self>) {
        self.recording_manager
            .set_memory_limit(self.settings.recording_memory_limit_bytes as usize);
        if self.defer_settings_persistence(cx) {
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_recording_settings(&self.settings))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.recording_manager
                    .set_memory_limit(self.settings.recording_memory_limit_bytes as usize);
                self.store_status.message = "recording settings saved".to_string();
                self.store_status.ready = true;
                self.terminal_status = "recording settings saved".to_string();
            }
            Err(error) => {
                self.store_status.message = format!("recording settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn update_transfer_duplicate_policy(
        &mut self,
        policy: SftpDuplicatePolicy,
        cx: &mut Context<Self>,
    ) {
        self.transfer.paths.duplicate_policy = policy;
        self.settings.transfer_duplicate_strategy = duplicate_policy_label(policy).to_string();
        self.save_transfer_settings("transfer duplicate policy saved", cx);
    }

    pub(in crate::features) fn update_transfer_editor_type(
        &mut self,
        editor_type: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.transfer_editor_type = editor_type.to_string();
        self.save_transfer_settings("transfer editor preference saved", cx);
    }

    pub(in crate::features) fn toggle_transfer_ask_save_location(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.transfer_ask_save_location = !self.settings.transfer_ask_save_location;
        self.save_transfer_settings("transfer save-location preference saved", cx);
    }

    pub(in crate::features) fn toggle_transfer_preserve_timestamps(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.transfer_preserve_timestamps = !self.settings.transfer_preserve_timestamps;
        self.save_transfer_settings("transfer timestamp preference saved", cx);
    }

    pub(in crate::features) fn toggle_transfer_resume_broken(&mut self, cx: &mut Context<Self>) {
        self.settings.transfer_resume_broken_transfer =
            !self.settings.transfer_resume_broken_transfer;
        self.save_transfer_settings("transfer resume preference saved", cx);
    }

    pub(in crate::features) fn adjust_transfer_download_threads(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        self.settings.transfer_download_threads =
            adjust_u32_setting(self.settings.transfer_download_threads, delta, 1, 10);
        self.save_transfer_settings("transfer download concurrency saved", cx);
    }

    pub(in crate::features) fn adjust_transfer_upload_threads(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        self.settings.transfer_upload_threads =
            adjust_u32_setting(self.settings.transfer_upload_threads, delta, 1, 10);
        self.save_transfer_settings("transfer upload concurrency saved", cx);
    }

    pub(in crate::features) fn adjust_transfer_max_retries(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        self.settings.transfer_max_retries =
            adjust_u32_setting(self.settings.transfer_max_retries, delta, 0, 10);
        self.save_transfer_settings("transfer retry setting saved", cx);
    }

    pub(in crate::features) fn adjust_transfer_buffer_size(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let step_delta = delta.saturating_mul(8);
        self.settings.transfer_buffer_size =
            adjust_u32_setting(self.settings.transfer_buffer_size, step_delta, 8, 256);
        self.save_transfer_settings("transfer buffer setting saved", cx);
    }

    pub(in crate::features) fn update_transfer_file_permissions(
        &mut self,
        permissions: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.transfer_default_file_permissions = permissions.to_string();
        self.save_transfer_settings("transfer default permissions saved", cx);
    }

    pub(in crate::features) fn save_transfer_settings(
        &mut self,
        success_status: &'static str,
        cx: &mut Context<Self>,
    ) {
        if self.defer_settings_persistence(cx) {
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_transfer_settings(&self.settings))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.transfer.paths.duplicate_policy = SftpDuplicatePolicy::from_legacy_value(
                    &self.settings.transfer_duplicate_strategy,
                );
                self.store_status.message = "transfer settings saved".to_string();
                self.store_status.ready = true;
                self.terminal_status = success_status.to_string();
            }
            Err(error) => {
                self.store_status.message = format!("transfer settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn handle_transfer_default_editor_key_down(
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
                self.settings.transfer_default_editor.pop();
                self.terminal_status = "transfer editor command edited".to_string();
                cx.notify();
            }
            "enter" => {
                self.save_transfer_settings("transfer editor command saved", cx);
            }
            "escape" => {
                self.terminal_status = "transfer editor command input blurred".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.settings.transfer_default_editor.push_str(input);
                    self.terminal_status = "transfer editor command edited".to_string();
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn handle_transfer_download_path_key_down(
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
                self.settings.transfer_download_path.pop();
                cx.notify();
            }
            "enter" => {
                self.save_transfer_settings("transfer download path saved", cx);
            }
            "escape" => cx.notify(),
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.settings.transfer_download_path.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn handle_recording_path_key_down(
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
                self.settings.recording_path.pop();
                cx.notify();
            }
            "enter" => self.save_recording_settings(cx),
            "escape" => cx.notify(),
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.settings.recording_path.push_str(input);
                    cx.notify();
                }
            }
        }
    }
}
