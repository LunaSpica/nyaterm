use gpui::Context;

use crate::features::NyaTermApp;
use crate::models::{CloudSyncInputField, SettingsTab};

impl NyaTermApp {
    pub(in crate::features) fn update_cloud_sync_provider(
        &mut self,
        provider: &'static str,
        cx: &mut Context<Self>,
    ) {
        if !self.cloud_sync_form_enabled() {
            return;
        }
        if provider != "github_gist" && self.cloud_sync.github_auth().pending {
            self.cancel_github_gist_auth(cx);
        }
        self.cloud_sync.select_provider(provider);
        cx.notify();
    }

    pub(in crate::features) fn toggle_cloud_sync_enabled(&mut self, cx: &mut Context<Self>) {
        if !self.cloud_sync.settings().enabled
            && (!self.settings.master_password().enabled
                || (!self.settings.summary().has_master_password
                    && self.settings.master_password().draft.is_empty()))
        {
            self.shell.set_settings_active_tab(SettingsTab::Security);
            self.cloud_sync
                .set_status("configure a master password before enabling cloud sync");
            self.shell.status = self.cloud_sync.status().to_string();
            cx.notify();
            return;
        }
        self.cloud_sync.toggle_enabled();
        cx.notify();
    }

    pub(in crate::features) fn toggle_s3_virtual_host_style(&mut self, cx: &mut Context<Self>) {
        if !self.cloud_sync_form_enabled() {
            return;
        }
        self.cloud_sync.toggle_s3_virtual_host_style();
        cx.notify();
    }

    pub(in crate::features) fn toggle_cloud_sync_auto_check(&mut self, cx: &mut Context<Self>) {
        if !self.cloud_sync_form_enabled() || !self.cloud_sync.settings().enabled {
            return;
        }
        self.cloud_sync.toggle_auto_check();
        cx.notify();
    }

    pub(in crate::features) fn toggle_cloud_sync_auto_push(&mut self, cx: &mut Context<Self>) {
        if !self.cloud_sync_form_enabled() || !self.cloud_sync.settings().enabled {
            return;
        }
        self.cloud_sync.toggle_auto_push();
        cx.notify();
    }

    pub(in crate::features) fn adjust_cloud_sync_debounce(
        &mut self,
        delta: i64,
        cx: &mut Context<Self>,
    ) {
        if !self.cloud_sync_form_enabled()
            || !self.cloud_sync.settings().enabled
            || !self.cloud_sync.settings().auto_push_on_change
        {
            return;
        }
        self.cloud_sync.adjust_debounce(delta);
        cx.notify();
    }

    /// Apply an edit from one of the cloud sync inputs.
    pub(in crate::features) fn apply_cloud_sync_input(
        &mut self,
        field: CloudSyncInputField,
        text: String,
        cx: &mut Context<Self>,
    ) {
        if !self.cloud_sync_form_enabled() {
            return;
        }
        if self.cloud_sync.apply_input(field, text) {
            cx.notify();
        }
    }

    pub(in crate::features) fn cloud_sync_form_enabled(&self) -> bool {
        self.settings.master_password().enabled
            && (self.settings.summary().has_master_password
                || !self.settings.master_password().draft.is_empty())
    }
}
