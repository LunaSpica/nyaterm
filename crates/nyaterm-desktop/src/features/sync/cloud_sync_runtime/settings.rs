use super::*;

use crate::models::SettingsTab;

impl NyaTermApp {
    pub(in crate::features) fn update_cloud_sync_provider(
        &mut self,
        provider: &'static str,
        cx: &mut Context<Self>,
    ) {
        if !self.cloud_sync_form_enabled() {
            return;
        }
        if provider != "github_gist" && self.github_gist_auth.pending {
            self.cancel_github_gist_auth(cx);
        }
        self.cloud_sync_settings.provider = provider.to_string();
        self.cloud_sync_provider_menu_open = false;
        self.cloud_sync_status = format!("provider set to {provider}; save to persist");
        cx.notify();
    }

    pub(in crate::features) fn toggle_cloud_sync_enabled(&mut self, cx: &mut Context<Self>) {
        if !self.cloud_sync_settings.enabled
            && (!self.settings_master_password_enabled
                || (!self.settings.has_master_password
                    && self.settings_master_password_draft.is_empty()))
        {
            self.settings_active_tab = SettingsTab::Security;
            self.cloud_sync_status =
                "configure a master password before enabling cloud sync".to_string();
            self.terminal.view.status = self.cloud_sync_status.clone();
            cx.notify();
            return;
        }
        self.cloud_sync_settings.enabled = !self.cloud_sync_settings.enabled;
        self.cloud_sync_status = if self.cloud_sync_settings.enabled {
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
        self.cloud_sync_settings.s3.virtual_host_style =
            !self.cloud_sync_settings.s3.virtual_host_style;
        self.cloud_sync_status = if self.cloud_sync_settings.s3.virtual_host_style {
            "S3 virtual-host style enabled; save to persist"
        } else {
            "S3 path-style URLs enabled; save to persist"
        }
        .to_string();
        cx.notify();
    }

    pub(in crate::features) fn toggle_cloud_sync_auto_check(&mut self, cx: &mut Context<Self>) {
        if !self.cloud_sync_form_enabled() || !self.cloud_sync_settings.enabled {
            return;
        }
        self.cloud_sync_settings.auto_check_on_startup =
            !self.cloud_sync_settings.auto_check_on_startup;
        self.cloud_sync_status = "cloud sync auto-check setting edited".to_string();
        cx.notify();
    }

    pub(in crate::features) fn toggle_cloud_sync_auto_push(&mut self, cx: &mut Context<Self>) {
        if !self.cloud_sync_form_enabled() || !self.cloud_sync_settings.enabled {
            return;
        }
        self.cloud_sync_settings.auto_push_on_change =
            !self.cloud_sync_settings.auto_push_on_change;
        self.cloud_sync_status = "cloud sync auto-push setting edited".to_string();
        cx.notify();
    }

    pub(in crate::features) fn adjust_cloud_sync_debounce(
        &mut self,
        delta: i64,
        cx: &mut Context<Self>,
    ) {
        if !self.cloud_sync_form_enabled()
            || !self.cloud_sync_settings.enabled
            || !self.cloud_sync_settings.auto_push_on_change
        {
            return;
        }
        let current = self.cloud_sync_settings.sync_debounce_seconds as i64;
        self.cloud_sync_settings.sync_debounce_seconds = (current + delta).clamp(1, 3_600) as u64;
        self.cloud_sync_status = "cloud sync debounce setting edited".to_string();
        cx.notify();
    }

    pub(in crate::features) fn handle_cloud_sync_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        if !self.cloud_sync_form_enabled() {
            return;
        }
        if self.github_gist_auth.pending
            && self.cloud_sync_focused_field == CloudSyncInputField::GithubGistId
        {
            return;
        }
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.cloud_sync_input_value_mut().pop();
                self.cloud_sync_status = "cloud sync settings edited".to_string();
                cx.notify();
            }
            "escape" => {
                self.cloud_sync_status = "cloud sync input blurred".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.cloud_sync_input_value_mut().push_str(input);
                    self.cloud_sync_status = "cloud sync settings edited".to_string();
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn cloud_sync_input_value_mut(&mut self) -> &mut String {
        match self.cloud_sync_focused_field {
            CloudSyncInputField::RemoteRoot => &mut self.cloud_sync_settings.remote_root,
            CloudSyncInputField::DeviceName => &mut self.cloud_sync_settings.device_name,
            CloudSyncInputField::WebdavEndpoint => &mut self.cloud_sync_settings.webdav.endpoint,
            CloudSyncInputField::WebdavRoot => &mut self.cloud_sync_settings.webdav.root,
            CloudSyncInputField::WebdavUsername => &mut self.cloud_sync_settings.webdav.username,
            CloudSyncInputField::WebdavPassword => {
                &mut self.cloud_sync_secret_draft.webdav_password
            }
            CloudSyncInputField::S3Endpoint => &mut self.cloud_sync_settings.s3.endpoint,
            CloudSyncInputField::S3Bucket => &mut self.cloud_sync_settings.s3.bucket,
            CloudSyncInputField::S3Region => &mut self.cloud_sync_settings.s3.region,
            CloudSyncInputField::S3Root => &mut self.cloud_sync_settings.s3.root,
            CloudSyncInputField::S3AccessKeyId => {
                &mut self.cloud_sync_secret_draft.s3_access_key_id
            }
            CloudSyncInputField::S3SecretAccessKey => {
                &mut self.cloud_sync_secret_draft.s3_secret_access_key
            }
            CloudSyncInputField::S3SessionToken => {
                &mut self.cloud_sync_secret_draft.s3_session_token
            }
            CloudSyncInputField::GoogleDriveRoot => &mut self.cloud_sync_settings.google_drive.root,
            CloudSyncInputField::GoogleDriveAccessToken => {
                &mut self.cloud_sync_secret_draft.google_drive_access_token
            }
            CloudSyncInputField::GoogleDriveRefreshToken => {
                &mut self.cloud_sync_secret_draft.google_drive_refresh_token
            }
            CloudSyncInputField::GoogleDriveClientId => self
                .cloud_sync_settings
                .google_drive
                .client_id
                .get_or_insert_with(String::new),
            CloudSyncInputField::GoogleDriveClientSecret => {
                &mut self.cloud_sync_secret_draft.google_drive_client_secret
            }
            CloudSyncInputField::OneDriveRoot => &mut self.cloud_sync_settings.onedrive.root,
            CloudSyncInputField::OneDriveAccessToken => {
                &mut self.cloud_sync_secret_draft.onedrive_access_token
            }
            CloudSyncInputField::OneDriveRefreshToken => {
                &mut self.cloud_sync_secret_draft.onedrive_refresh_token
            }
            CloudSyncInputField::OneDriveClientId => self
                .cloud_sync_settings
                .onedrive
                .client_id
                .get_or_insert_with(String::new),
            CloudSyncInputField::OneDriveClientSecret => {
                &mut self.cloud_sync_secret_draft.onedrive_client_secret
            }
            CloudSyncInputField::AliyunDriveRoot => &mut self.cloud_sync_settings.aliyun_drive.root,
            CloudSyncInputField::AliyunDriveType => {
                &mut self.cloud_sync_settings.aliyun_drive.drive_type
            }
            CloudSyncInputField::AliyunDriveAccessToken => {
                &mut self.cloud_sync_secret_draft.aliyun_drive_access_token
            }
            CloudSyncInputField::AliyunDriveRefreshToken => {
                &mut self.cloud_sync_secret_draft.aliyun_drive_refresh_token
            }
            CloudSyncInputField::AliyunDriveClientId => self
                .cloud_sync_settings
                .aliyun_drive
                .client_id
                .get_or_insert_with(String::new),
            CloudSyncInputField::AliyunDriveClientSecret => {
                &mut self.cloud_sync_secret_draft.aliyun_drive_client_secret
            }
            CloudSyncInputField::GiteeEndpoint => {
                &mut self.cloud_sync_settings.gitee_snippet.api_endpoint
            }
            CloudSyncInputField::GiteeGistId => &mut self.cloud_sync_settings.gitee_snippet.gist_id,
            CloudSyncInputField::GiteeToken => &mut self.cloud_sync_secret_draft.gitee_token,
            CloudSyncInputField::GithubGistId => &mut self.cloud_sync_settings.github_gist.gist_id,
        }
    }

    pub(in crate::features) fn cloud_sync_form_enabled(&self) -> bool {
        self.settings_master_password_enabled
            && (self.settings.has_master_password
                || !self.settings_master_password_draft.is_empty())
    }
}
