use super::*;

impl NyaTermApp {
    pub(in crate::features) fn prompt_local_cloud_sync_push(&mut self, cx: &mut Context<Self>) {
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::CloudPush, cx);
    }

    pub(in crate::features) fn prompt_local_cloud_sync_pull(&mut self, cx: &mut Context<Self>) {
        if self.active_session_id.is_some() || self.has_pending_session_start() {
            self.terminal_status = "close active session before pulling cloud sync".to_string();
            cx.notify();
            return;
        }
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::CloudPull, cx);
    }

    pub(in crate::features) fn prompt_provider_cloud_sync_push(&mut self, cx: &mut Context<Self>) {
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::CloudProviderPush, cx);
    }

    pub(in crate::features) fn prompt_provider_cloud_sync_pull(&mut self, cx: &mut Context<Self>) {
        if self.active_session_id.is_some() || self.has_pending_session_start() {
            self.terminal_status =
                "close active session before pulling provider cloud sync".to_string();
            cx.notify();
            return;
        }
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::CloudProviderPull, cx);
    }

    pub(in crate::features) fn prompt_cloud_sync_force_push(
        &mut self,
        provider_action: bool,
        cx: &mut Context<Self>,
    ) {
        let kind = if provider_action {
            SnapshotPasswordPromptKind::CloudProviderForcePush
        } else {
            SnapshotPasswordPromptKind::CloudForcePush
        };
        self.start_snapshot_password_prompt(kind, cx);
    }

    pub(in crate::features) fn prompt_cloud_sync_force_pull(
        &mut self,
        provider_action: bool,
        cx: &mut Context<Self>,
    ) {
        if self.active_session_id.is_some() || self.has_pending_session_start() {
            self.terminal_status = if provider_action {
                "close active session before force pulling provider cloud sync"
            } else {
                "close active session before force pulling cloud sync"
            }
            .to_string();
            cx.notify();
            return;
        }
        let kind = if provider_action {
            SnapshotPasswordPromptKind::CloudProviderForcePull
        } else {
            SnapshotPasswordPromptKind::CloudForcePull
        };
        self.start_snapshot_password_prompt(kind, cx);
    }

    pub(in crate::features) fn dismiss_cloud_sync_conflict(&mut self, cx: &mut Context<Self>) {
        self.cloud_sync_conflict = None;
        self.cloud_sync_status = "cloud sync conflict dismissed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn capture_cloud_sync_conflict(
        &mut self,
        error: &CloudSyncError,
        provider: String,
        provider_action: bool,
    ) {
        if let CloudSyncError::Conflict(message) = error {
            self.cloud_sync_conflict = Some(CloudSyncConflictState {
                provider,
                message: message.clone(),
                provider_action,
            });
        }
    }
}
