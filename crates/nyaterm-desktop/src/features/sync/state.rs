//! Authoritative state for cloud-sync configuration, history and background jobs.

use std::collections::HashSet;
use std::sync::{Arc, atomic::AtomicBool, mpsc};

use nyaterm_core::{CloudSyncError, CloudSyncHistoryEntry, CloudSyncSettings, CloudSyncState};

use crate::models::{
    CloudSyncConflictState, CloudSyncInputField, CloudSyncSecretDraft, GithubGistAuthJobEvent,
    GithubGistAuthState,
};

pub(in crate::features) struct CloudSyncFeatureState {
    pub settings: CloudSyncSettings,
    pub state: CloudSyncState,
    pub history: Vec<CloudSyncHistoryEntry>,
    pub history_expanded: HashSet<String>,
    pub conflict: Option<CloudSyncConflictState>,
    pub secret_draft: CloudSyncSecretDraft,
    pub status: String,
    /// Prevent overlapping network jobs from applying cloud state out of order.
    pub job_running: bool,
    pub focused_field: CloudSyncInputField,
    pub provider_menu_open: bool,
    pub github: GithubGistAuthFeatureState,
}

pub(in crate::features) struct GithubGistAuthFeatureState {
    pub auth: GithubGistAuthState,
    pub(super) tx: mpsc::Sender<GithubGistAuthJobEvent>,
    pub(super) rx: mpsc::Receiver<GithubGistAuthJobEvent>,
    pub(super) job_id: u64,
    pub(super) cancel: Option<Arc<AtomicBool>>,
}

impl CloudSyncFeatureState {
    pub(in crate::features) fn new(
        settings: CloudSyncSettings,
        state: CloudSyncState,
        history: Vec<CloudSyncHistoryEntry>,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            settings,
            state,
            history,
            history_expanded: HashSet::new(),
            conflict: None,
            secret_draft: CloudSyncSecretDraft::default(),
            status: "local provider ready".to_string(),
            job_running: false,
            focused_field: CloudSyncInputField::RemoteRoot,
            provider_menu_open: false,
            github: GithubGistAuthFeatureState {
                auth: GithubGistAuthState::default(),
                tx,
                rx,
                job_id: 0,
                cancel: None,
            },
        }
    }

    pub(super) fn apply_input(&mut self, field: CloudSyncInputField, text: String) -> bool {
        // A Gist id being fetched is not the user's to edit yet.
        if self.github.auth.pending && field == CloudSyncInputField::GithubGistId {
            return false;
        }
        self.focused_field = field;
        *self.input_value_mut() = text;
        self.status = "cloud sync settings edited".to_string();
        true
    }

    pub(super) fn begin_job(&mut self) -> bool {
        if self.job_running {
            return false;
        }
        self.job_running = true;
        true
    }

    pub(super) fn toggle_history_details(&mut self, entry_id: &str) {
        if self.history_expanded.contains(entry_id) {
            self.history_expanded.remove(entry_id);
        } else {
            self.history_expanded.insert(entry_id.to_string());
        }
    }

    pub(super) fn capture_conflict(
        &mut self,
        error: &CloudSyncError,
        provider: String,
        provider_action: bool,
    ) {
        if let CloudSyncError::Conflict(message) = error {
            self.conflict = Some(CloudSyncConflictState {
                provider,
                message: message.clone(),
                provider_action,
            });
        }
    }

    fn input_value_mut(&mut self) -> &mut String {
        match self.focused_field {
            CloudSyncInputField::RemoteRoot => &mut self.settings.remote_root,
            CloudSyncInputField::DeviceName => &mut self.settings.device_name,
            CloudSyncInputField::WebdavEndpoint => &mut self.settings.webdav.endpoint,
            CloudSyncInputField::WebdavRoot => &mut self.settings.webdav.root,
            CloudSyncInputField::WebdavUsername => &mut self.settings.webdav.username,
            CloudSyncInputField::WebdavPassword => &mut self.secret_draft.webdav_password,
            CloudSyncInputField::S3Endpoint => &mut self.settings.s3.endpoint,
            CloudSyncInputField::S3Bucket => &mut self.settings.s3.bucket,
            CloudSyncInputField::S3Region => &mut self.settings.s3.region,
            CloudSyncInputField::S3Root => &mut self.settings.s3.root,
            CloudSyncInputField::S3AccessKeyId => &mut self.secret_draft.s3_access_key_id,
            CloudSyncInputField::S3SecretAccessKey => &mut self.secret_draft.s3_secret_access_key,
            CloudSyncInputField::S3SessionToken => &mut self.secret_draft.s3_session_token,
            CloudSyncInputField::GoogleDriveRoot => &mut self.settings.google_drive.root,
            CloudSyncInputField::GoogleDriveAccessToken => {
                &mut self.secret_draft.google_drive_access_token
            }
            CloudSyncInputField::GoogleDriveRefreshToken => {
                &mut self.secret_draft.google_drive_refresh_token
            }
            CloudSyncInputField::GoogleDriveClientId => self
                .settings
                .google_drive
                .client_id
                .get_or_insert_with(String::new),
            CloudSyncInputField::GoogleDriveClientSecret => {
                &mut self.secret_draft.google_drive_client_secret
            }
            CloudSyncInputField::OneDriveRoot => &mut self.settings.onedrive.root,
            CloudSyncInputField::OneDriveAccessToken => {
                &mut self.secret_draft.onedrive_access_token
            }
            CloudSyncInputField::OneDriveRefreshToken => {
                &mut self.secret_draft.onedrive_refresh_token
            }
            CloudSyncInputField::OneDriveClientId => self
                .settings
                .onedrive
                .client_id
                .get_or_insert_with(String::new),
            CloudSyncInputField::OneDriveClientSecret => {
                &mut self.secret_draft.onedrive_client_secret
            }
            CloudSyncInputField::AliyunDriveRoot => &mut self.settings.aliyun_drive.root,
            CloudSyncInputField::AliyunDriveType => &mut self.settings.aliyun_drive.drive_type,
            CloudSyncInputField::AliyunDriveAccessToken => {
                &mut self.secret_draft.aliyun_drive_access_token
            }
            CloudSyncInputField::AliyunDriveRefreshToken => {
                &mut self.secret_draft.aliyun_drive_refresh_token
            }
            CloudSyncInputField::AliyunDriveClientId => self
                .settings
                .aliyun_drive
                .client_id
                .get_or_insert_with(String::new),
            CloudSyncInputField::AliyunDriveClientSecret => {
                &mut self.secret_draft.aliyun_drive_client_secret
            }
            CloudSyncInputField::GiteeEndpoint => &mut self.settings.gitee_snippet.api_endpoint,
            CloudSyncInputField::GiteeGistId => &mut self.settings.gitee_snippet.gist_id,
            CloudSyncInputField::GiteeToken => &mut self.secret_draft.gitee_token,
            CloudSyncInputField::GithubGistId => &mut self.settings.github_gist.gist_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use nyaterm_core::{CloudSyncHistoryEntry, CloudSyncSettings, CloudSyncState};

    use crate::models::CloudSyncInputField;

    use super::CloudSyncFeatureState;

    #[test]
    fn cloud_sync_state_owns_loaded_data_and_github_job_channel() {
        let settings = CloudSyncSettings {
            provider: "webdav".to_string(),
            remote_root: "team".to_string(),
            ..CloudSyncSettings::default()
        };
        let state = CloudSyncState::default();
        let history = vec![CloudSyncHistoryEntry::sync(
            "success",
            "manual_push",
            Some("webdav".to_string()),
            None,
            "done".to_string(),
        )];

        let mut cloud_sync = CloudSyncFeatureState::new(settings, state, history);

        assert_eq!(cloud_sync.settings.provider, "webdav");
        assert_eq!(cloud_sync.settings.remote_root, "team");
        assert_eq!(cloud_sync.history.len(), 1);
        assert!(cloud_sync.github.rx.try_recv().is_err());
        assert!(!cloud_sync.job_running);
        assert!(cloud_sync.conflict.is_none());

        cloud_sync.settings.webdav.password = Some("stored".to_string());
        assert!(cloud_sync.apply_input(CloudSyncInputField::WebdavPassword, "draft".to_string(),));
        assert_eq!(
            cloud_sync.settings.webdav.password.as_deref(),
            Some("stored")
        );
        assert_eq!(cloud_sync.secret_draft.webdav_password, "draft");
    }
}
