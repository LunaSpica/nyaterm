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
        if provider != "github_gist" && self.cloud_sync.github.auth.pending {
            self.cancel_github_gist_auth(cx);
        }
        self.cloud_sync.settings.provider = provider.to_string();
        self.cloud_sync.provider_menu_open = false;
        self.cloud_sync.status = format!("provider set to {provider}; save to persist");
        cx.notify();
    }

    pub(in crate::features) fn toggle_cloud_sync_enabled(&mut self, cx: &mut Context<Self>) {
        if !self.cloud_sync.settings.enabled
            && (!self.settings_master_password_enabled
                || (!self.settings.has_master_password
                    && self.settings_master_password_draft.is_empty()))
        {
            self.shell.navigation.settings.active_tab = SettingsTab::Security;
            self.cloud_sync.status =
                "configure a master password before enabling cloud sync".to_string();
            self.terminal.view.status = self.cloud_sync.status.clone();
            cx.notify();
            return;
        }
        self.cloud_sync.settings.enabled = !self.cloud_sync.settings.enabled;
        self.cloud_sync.status = if self.cloud_sync.settings.enabled {
            "cloud sync enabled; save to persist"
        } else {
            "cloud sync disabled; save to persist"
        }
        .to_string();
        cx.notify();
    }

    pub(in crate::features) fn toggle_s3_virtual_host_style(&mut self, cx: &mut Context<Self>) {
        if !self.cloud_sync_form_enabled() {
            return;
        }
        self.cloud_sync.settings.s3.virtual_host_style =
            !self.cloud_sync.settings.s3.virtual_host_style;
        self.cloud_sync.status = if self.cloud_sync.settings.s3.virtual_host_style {
            "S3 virtual-host style enabled; save to persist"
        } else {
            "S3 path-style URLs enabled; save to persist"
        }
        .to_string();
        cx.notify();
    }

    pub(in crate::features) fn toggle_cloud_sync_auto_check(&mut self, cx: &mut Context<Self>) {
        if !self.cloud_sync_form_enabled() || !self.cloud_sync.settings.enabled {
            return;
        }
        self.cloud_sync.settings.auto_check_on_startup =
            !self.cloud_sync.settings.auto_check_on_startup;
        self.cloud_sync.status = "cloud sync auto-check setting edited".to_string();
        cx.notify();
    }

    pub(in crate::features) fn toggle_cloud_sync_auto_push(&mut self, cx: &mut Context<Self>) {
        if !self.cloud_sync_form_enabled() || !self.cloud_sync.settings.enabled {
            return;
        }
        self.cloud_sync.settings.auto_push_on_change =
            !self.cloud_sync.settings.auto_push_on_change;
        self.cloud_sync.status = "cloud sync auto-push setting edited".to_string();
        cx.notify();
    }

    pub(in crate::features) fn adjust_cloud_sync_debounce(
        &mut self,
        delta: i64,
        cx: &mut Context<Self>,
    ) {
        if !self.cloud_sync_form_enabled()
            || !self.cloud_sync.settings.enabled
            || !self.cloud_sync.settings.auto_push_on_change
        {
            return;
        }
        let current = self.cloud_sync.settings.sync_debounce_seconds as i64;
        self.cloud_sync.settings.sync_debounce_seconds = (current + delta).clamp(1, 3_600) as u64;
        self.cloud_sync.status = "cloud sync debounce setting edited".to_string();
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
        self.settings_master_password_enabled
            && (self.settings.has_master_password
                || !self.settings_master_password_draft.is_empty())
    }
}
