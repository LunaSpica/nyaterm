use gpui::{Context, Window};

use crate::features::NyaTermApp;
use crate::models::SnapshotPasswordPromptKind;

impl NyaTermApp {
    pub(in crate::features) fn prompt_provider_cloud_sync_push(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.block_cloud_sync_for_settings_draft(cx) {
            return;
        }
        self.start_snapshot_password_prompt(
            SnapshotPasswordPromptKind::CloudProviderPush,
            window,
            cx,
        );
    }

    pub(in crate::features) fn prompt_provider_cloud_sync_pull(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.block_cloud_sync_for_settings_draft(cx) {
            return;
        }
        if self.session.active_id().is_some() || self.session.start_has_pending() {
            self.shell
                .set_status("close active session before pulling provider cloud sync".to_string());
            cx.notify();
            return;
        }
        self.start_snapshot_password_prompt(
            SnapshotPasswordPromptKind::CloudProviderPull,
            window,
            cx,
        );
    }

    pub(in crate::features) fn prompt_cloud_sync_force_push(
        &mut self,
        provider_action: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.block_cloud_sync_for_settings_draft(cx) {
            return;
        }
        let kind = if provider_action {
            SnapshotPasswordPromptKind::CloudProviderForcePush
        } else {
            SnapshotPasswordPromptKind::CloudForcePush
        };
        self.start_snapshot_password_prompt(kind, window, cx);
    }

    pub(in crate::features) fn prompt_cloud_sync_force_pull(
        &mut self,
        provider_action: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.block_cloud_sync_for_settings_draft(cx) {
            return;
        }
        if self.session.active_id().is_some() || self.session.start_has_pending() {
            self.shell.set_status(
                if provider_action {
                    "close active session before force pulling provider cloud sync"
                } else {
                    "close active session before force pulling cloud sync"
                }
                .to_string(),
            );
            cx.notify();
            return;
        }
        let kind = if provider_action {
            SnapshotPasswordPromptKind::CloudProviderForcePull
        } else {
            SnapshotPasswordPromptKind::CloudForcePull
        };
        self.start_snapshot_password_prompt(kind, window, cx);
    }
}
