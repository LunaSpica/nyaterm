use gpui::Context;
use nyaterm_core::ConnectionStore;
use nyaterm_transport::SftpDuplicatePolicy;

use crate::features::{NyaTermApp, duplicate_policy_label};

use super::helpers::adjust_u32_setting;

impl NyaTermApp {
    pub(in crate::features) fn update_host_key_policy(
        &mut self,
        policy: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.host_key_policy = policy.to_string();
        if self.defer_settings_persistence(cx) {
            self.terminal.view.status = format!("host key policy staged as {policy}");
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
                self.terminal.view.status = format!("host key policy set to {policy}");
                self.settings.store_status.message = "settings saved".to_string();
                self.settings.store_status.ready = true;
            }
            Err(error) => {
                self.terminal.view.status = format!("failed to save host key policy: {error}");
                self.settings.store_status.message = format!("settings save failed: {error}");
                self.settings.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn toggle_recording_auto_start(&mut self, cx: &mut Context<Self>) {
        self.settings.summary.recording_auto_start = !self.settings.summary.recording_auto_start;
        self.save_recording_settings(cx);
    }

    pub(in crate::features) fn toggle_recording_io_labels(&mut self, cx: &mut Context<Self>) {
        self.settings.summary.recording_include_io_labels =
            !self.settings.summary.recording_include_io_labels;
        self.save_recording_settings(cx);
    }

    pub(in crate::features) fn toggle_recording_timestamps(&mut self, cx: &mut Context<Self>) {
        self.settings.summary.recording_include_timestamps =
            !self.settings.summary.recording_include_timestamps;
        self.save_recording_settings(cx);
    }

    pub(in crate::features) fn adjust_recording_memory_limit(
        &mut self,
        delta_mib: i64,
        cx: &mut Context<Self>,
    ) {
        let current_mib =
            (self.settings.summary.recording_memory_limit_bytes / (1024 * 1024)).max(1);
        let next_mib = if delta_mib.is_negative() {
            current_mib.saturating_sub(delta_mib.unsigned_abs()).max(1)
        } else {
            current_mib.saturating_add(delta_mib as u64).min(512)
        };
        self.settings.summary.recording_memory_limit_bytes = next_mib * 1024 * 1024;
        self.save_recording_settings(cx);
    }

    pub(in crate::features) fn save_recording_settings(&mut self, cx: &mut Context<Self>) {
        self.recording
            .set_memory_limit(self.settings.summary.recording_memory_limit_bytes as usize);
        if self.defer_settings_persistence(cx) {
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_recording_settings(&self.settings.summary))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.recording
                    .set_memory_limit(self.settings.summary.recording_memory_limit_bytes as usize);
                self.settings.store_status.message = "recording settings saved".to_string();
                self.settings.store_status.ready = true;
                self.terminal.view.status = "recording settings saved".to_string();
            }
            Err(error) => {
                self.settings.store_status.message =
                    format!("recording settings save failed: {error}");
                self.settings.store_status.ready = false;
                self.terminal.view.status = self.settings.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn update_transfer_duplicate_policy(
        &mut self,
        policy: SftpDuplicatePolicy,
        cx: &mut Context<Self>,
    ) {
        self.transfer.set_duplicate_policy(policy);
        self.settings.summary.transfer_duplicate_strategy =
            duplicate_policy_label(policy).to_string();
        self.save_transfer_settings("transfer duplicate policy saved", cx);
    }

    pub(in crate::features) fn update_transfer_editor_type(
        &mut self,
        editor_type: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.transfer_editor_type = editor_type.to_string();
        self.save_transfer_settings("transfer editor preference saved", cx);
    }

    pub(in crate::features) fn toggle_transfer_ask_save_location(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.transfer_ask_save_location =
            !self.settings.summary.transfer_ask_save_location;
        self.save_transfer_settings("transfer save-location preference saved", cx);
    }

    pub(in crate::features) fn toggle_transfer_preserve_timestamps(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.transfer_preserve_timestamps =
            !self.settings.summary.transfer_preserve_timestamps;
        self.save_transfer_settings("transfer timestamp preference saved", cx);
    }

    pub(in crate::features) fn toggle_transfer_resume_broken(&mut self, cx: &mut Context<Self>) {
        self.settings.summary.transfer_resume_broken_transfer =
            !self.settings.summary.transfer_resume_broken_transfer;
        self.save_transfer_settings("transfer resume preference saved", cx);
    }

    pub(in crate::features) fn adjust_transfer_download_threads(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.transfer_download_threads = adjust_u32_setting(
            self.settings.summary.transfer_download_threads,
            delta,
            1,
            10,
        );
        self.save_transfer_settings("transfer download concurrency saved", cx);
    }

    pub(in crate::features) fn adjust_transfer_upload_threads(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.transfer_upload_threads =
            adjust_u32_setting(self.settings.summary.transfer_upload_threads, delta, 1, 10);
        self.save_transfer_settings("transfer upload concurrency saved", cx);
    }

    pub(in crate::features) fn adjust_transfer_max_retries(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.transfer_max_retries =
            adjust_u32_setting(self.settings.summary.transfer_max_retries, delta, 0, 10);
        self.save_transfer_settings("transfer retry setting saved", cx);
    }

    pub(in crate::features) fn adjust_transfer_buffer_size(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let step_delta = delta.saturating_mul(8);
        self.settings.summary.transfer_buffer_size = adjust_u32_setting(
            self.settings.summary.transfer_buffer_size,
            step_delta,
            8,
            256,
        );
        self.save_transfer_settings("transfer buffer setting saved", cx);
    }

    pub(in crate::features) fn update_transfer_file_permissions(
        &mut self,
        permissions: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.transfer_default_file_permissions = permissions.to_string();
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
        .and_then(|store| store.save_transfer_settings(&self.settings.summary))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.transfer
                    .set_duplicate_policy(SftpDuplicatePolicy::from_legacy_value(
                        &self.settings.summary.transfer_duplicate_strategy,
                    ));
                self.settings.store_status.message = "transfer settings saved".to_string();
                self.settings.store_status.ready = true;
                self.terminal.view.status = success_status.to_string();
            }
            Err(error) => {
                self.settings.store_status.message =
                    format!("transfer settings save failed: {error}");
                self.settings.store_status.ready = false;
                self.terminal.view.status = self.settings.store_status.message.clone();
            }
        }
        cx.notify();
    }

    /// Apply an edit from the default editor command box.
    pub(in crate::features) fn apply_transfer_default_editor(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.transfer_default_editor = text;
        self.terminal.view.status = "transfer editor command edited".to_string();
        cx.notify();
    }

    /// Apply an edit from the download path box.
    pub(in crate::features) fn apply_transfer_download_path(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.transfer_download_path = text;
        cx.notify();
    }

    /// Apply an edit from the recording path box.
    pub(in crate::features) fn apply_recording_path(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.recording_path = text;
        cx.notify();
    }
}
