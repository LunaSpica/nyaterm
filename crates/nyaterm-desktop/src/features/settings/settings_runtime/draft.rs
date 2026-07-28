use gpui::Context;
use nyaterm_core::ConnectionStore;
use nyaterm_transport::SftpDuplicatePolicy;

use crate::features::NyaTermApp;
use crate::features::app_state::SettingsDraftSnapshot;
use crate::models::{MainMode, NavItem, TranslationSecretDraft};

impl NyaTermApp {
    pub(in crate::features) fn begin_settings_draft(&mut self) {
        if self.shell.navigation.settings.draft_snapshot.is_some() {
            return;
        }
        let (translation_settings, translation_secret_draft) =
            self.translation.settings_draft_snapshot();
        let (cloud_sync_settings, cloud_sync_secret_draft) =
            self.cloud_sync.settings_draft_snapshot();
        self.shell.navigation.settings.draft_snapshot = Some(SettingsDraftSnapshot {
            settings: self.settings.summary.clone(),
            ai_settings: self.ai.settings.config.clone(),
            ai_model_draft: self.ai.settings.model_draft.clone(),
            ai_base_url_draft: self.ai.settings.base_url_draft.clone(),
            ai_secret_draft: self.ai.settings.secret_draft.clone(),
            cloud_sync_settings,
            cloud_sync_secret_draft,
            translation_settings,
            translation_secret_draft,
            keyword_highlights: self.settings.keyword_config.clone(),
            master_password_enabled: self.settings.master_password.enabled,
            master_password_draft: self.settings.master_password.draft.clone(),
        });
    }

    pub(in crate::features) fn settings_draft_dirty(&self) -> bool {
        let Some(snapshot) = self.shell.navigation.settings.draft_snapshot.as_ref() else {
            return false;
        };
        snapshot.settings != self.settings.summary
            || snapshot.ai_settings != self.ai.settings.config
            || snapshot.ai_model_draft != self.ai.settings.model_draft
            || snapshot.ai_base_url_draft != self.ai.settings.base_url_draft
            || snapshot.ai_secret_draft != self.ai.settings.secret_draft
            || !self.cloud_sync.settings_draft_matches(
                &snapshot.cloud_sync_settings,
                &snapshot.cloud_sync_secret_draft,
            )
            || !self.translation.settings_draft_matches(
                &snapshot.translation_settings,
                &snapshot.translation_secret_draft,
            )
            || snapshot.keyword_highlights != self.settings.keyword_config
            || snapshot.master_password_enabled != self.settings.master_password.enabled
            || snapshot.master_password_draft != self.settings.master_password.draft
    }

    /// Returns true when a settings save should stay in the in-memory draft.
    pub(in crate::features) fn defer_settings_persistence(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.shell.navigation.settings.draft_snapshot.is_none() {
            return false;
        }
        self.settings.store_status.message = "settings draft changed".to_string();
        self.settings.store_status.ready = true;
        self.terminal.view.status = "settings draft changed; apply to persist".to_string();
        cx.notify();
        true
    }

    pub(in crate::features) fn pending_settings_cloud_error(&self) -> Option<String> {
        let settings = self.cloud_sync.pending_settings();
        if !settings.enabled {
            return None;
        }
        if !self.settings.master_password.enabled {
            return Some("Enable a master password before enabling cloud sync".to_string());
        }
        if !self.settings.summary.has_master_password
            && self.settings.master_password.draft.is_empty()
        {
            return Some("Enter a master password before enabling cloud sync".to_string());
        }
        let missing = match settings.provider.as_str() {
            "webdav" if settings.webdav.endpoint.trim().is_empty() => {
                Some("WebDAV endpoint is required")
            }
            "s3" if settings.s3.endpoint.trim().is_empty() => Some("S3 endpoint is required"),
            "s3" if settings.s3.bucket.trim().is_empty() => Some("S3 bucket is required"),
            "s3" if settings
                .s3
                .access_key_id
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
                != settings
                    .s3
                    .secret_access_key
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .is_empty() =>
            {
                Some("S3 access key and secret must be provided together")
            }
            "gitee_snippet" if settings.gitee_snippet.api_endpoint.trim().is_empty() => {
                Some("Gitee Snippet API endpoint is required")
            }
            "gitee_snippet" if settings.gitee_snippet.gist_id.trim().is_empty() => {
                Some("Gitee Snippet ID is required")
            }
            "gitee_snippet"
                if settings
                    .gitee_snippet
                    .access_token
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .is_empty() =>
            {
                Some("Gitee Snippet token is required")
            }
            "google_drive" => drive_validation_error(
                settings.google_drive.refresh_token.as_deref(),
                settings.google_drive.client_id.as_deref(),
                settings.google_drive.client_secret.as_deref(),
            ),
            "onedrive" => drive_validation_error(
                settings.onedrive.refresh_token.as_deref(),
                settings.onedrive.client_id.as_deref(),
                settings.onedrive.client_secret.as_deref(),
            ),
            "aliyun_drive" => drive_validation_error(
                settings.aliyun_drive.refresh_token.as_deref(),
                settings.aliyun_drive.client_id.as_deref(),
                settings.aliyun_drive.client_secret.as_deref(),
            ),
            "github_gist" if settings.github_gist.gist_id.trim().is_empty() => {
                Some("GitHub Gist ID is required")
            }
            "github_gist"
                if settings
                    .github_gist
                    .access_token
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .is_empty() =>
            {
                Some("GitHub Gist token is required")
            }
            _ => None,
        };
        missing.map(str::to_string)
    }

    pub(in crate::features) fn block_cloud_sync_for_settings_draft(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.settings_draft_dirty() {
            return false;
        }
        self.cloud_sync
            .set_status("apply settings before running cloud sync");
        self.terminal.view.status = self.cloud_sync.status().to_string();
        cx.notify();
        true
    }

    pub(in crate::features) fn block_import_for_settings_draft(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.settings_draft_dirty() {
            return false;
        }
        self.terminal.view.status = "apply or cancel settings before importing".to_string();
        self.settings.store_status.message = self.terminal.view.status.clone();
        self.settings.store_status.ready = false;
        cx.notify();
        true
    }

    pub(in crate::features) fn rebase_open_settings_draft(&mut self) {
        if self.shell.navigation.settings.draft_snapshot.is_none() {
            return;
        }
        self.shell.navigation.settings.draft_snapshot = None;
        self.settings.rebase_master_password();
        self.begin_settings_draft();
    }

    pub(in crate::features) fn apply_settings_draft(
        &mut self,
        close_after_apply: bool,
        cx: &mut Context<Self>,
    ) {
        if self.shell.navigation.settings.draft_snapshot.is_none() {
            if close_after_apply {
                self.finish_settings_page(cx);
            }
            return;
        }
        if let Some(error) = self.pending_settings_cloud_error() {
            self.settings.store_status.message = error.clone();
            self.settings.store_status.ready = false;
            self.terminal.view.status = format!("settings apply blocked: {error}");
            cx.notify();
            return;
        }

        let settings = self.settings.summary.clone();
        let ai_settings = self.pending_ai_settings();
        let cloud_sync_settings = self.cloud_sync.pending_settings();
        let translation_settings = self.translation.pending_settings();
        let keyword_highlights = self.settings.keyword_config.clone();
        let master_password_update = if self.settings.master_password.draft.is_empty() {
            (self.settings.summary.has_master_password && !self.settings.master_password.enabled)
                .then_some(None)
        } else {
            Some(Some(self.settings.master_password.draft.clone()))
        };
        let result = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| {
            if let Some(next_password) = master_password_update.as_ref() {
                store.save_master_password(next_password.as_deref())?;
            }
            store.save_appearance_settings(&settings)?;
            store.save_terminal_settings(&settings)?;
            store.save_interaction_settings(&settings)?;
            store.save_general_settings(&settings)?;
            store.save_diagnostics_settings(&settings)?;
            store.save_screen_lock_settings(&settings)?;
            store.save_recording_settings(&settings)?;
            store.save_transfer_settings(&settings)?;
            store.save_host_key_policy(&settings.host_key_policy)?;
            store.save_keybindings(&settings.keybindings)?;
            let saved_keyword_highlights = store.save_keyword_highlights(&keyword_highlights)?;
            let saved_translation_settings =
                store.save_translation_settings(translation_settings.clone())?;
            let saved_cloud_sync_settings =
                store.save_cloud_sync_settings(cloud_sync_settings.clone())?;
            let saved_ai_settings = store.save_ai_settings(ai_settings.clone())?;
            Ok((
                store.load_app_settings_summary()?,
                saved_keyword_highlights,
                saved_translation_settings,
                saved_cloud_sync_settings,
                saved_ai_settings,
            ))
        });

        match result {
            Ok((
                saved_settings,
                saved_keyword_highlights,
                saved_translation_settings,
                saved_cloud_sync_settings,
                saved_ai_settings,
            )) => {
                self.apply_gpui_settings(saved_settings);
                self.settings.rebase_master_password();
                self.ai.settings.config = saved_ai_settings;
                self.cloud_sync
                    .replace_settings(saved_cloud_sync_settings, Default::default());
                self.translation.replace_settings(
                    saved_translation_settings,
                    TranslationSecretDraft::default(),
                );
                self.settings.keyword_config = saved_keyword_highlights;
                self.ai.settings.secret_draft.clear();
                self.sync_ai_drafts_from_active_profile();
                self.recording
                    .set_memory_limit(self.settings.summary.recording_memory_limit_bytes as usize);
                self.transfer.paths.duplicate_policy = SftpDuplicatePolicy::from_legacy_value(
                    &self.settings.summary.transfer_duplicate_strategy,
                );
                self.sync_terminal_encodings_from_settings();
                self.enforce_terminal_scrollback_limit();
                if !self
                    .settings
                    .summary
                    .interaction_command_suggestions_enabled
                {
                    self.terminal.assist.clear_command_tracking();
                }
                self.invalidate_terminal_cell_metrics(cx);
                self.refresh_visible_terminal_surfaces(cx);
                if !self.settings.summary.startup_restore_window_layout {
                    let _ = ConnectionStore::open_with_portable_key_path(
                        self.runtime.config_dir(),
                        self.runtime.portable_key_path().map(ToOwned::to_owned),
                    )
                    .and_then(|store| {
                        store.save_terminal_window_layout(None)?;
                        store.save_workspace_pane_layout(None)
                    });
                }
                self.shell.navigation.settings.draft_snapshot = None;
                self.settings.store_status.message = "settings applied".to_string();
                self.settings.store_status.ready = true;
                self.terminal.view.status = "settings applied".to_string();
                if close_after_apply {
                    self.finish_settings_page(cx);
                } else {
                    self.begin_settings_draft();
                    cx.notify();
                }
            }
            Err(error) => {
                self.settings.store_status.message = format!("settings apply failed: {error}");
                self.settings.store_status.ready = false;
                self.terminal.view.status = self.settings.store_status.message.clone();
                cx.notify();
            }
        }
    }

    pub(in crate::features) fn cancel_settings(&mut self, cx: &mut Context<Self>) {
        if let Some(snapshot) = self.shell.navigation.settings.draft_snapshot.take() {
            self.apply_gpui_settings(snapshot.settings);
            self.ai.settings.config = snapshot.ai_settings;
            self.ai.settings.model_draft = snapshot.ai_model_draft;
            self.ai.settings.base_url_draft = snapshot.ai_base_url_draft;
            self.ai.settings.secret_draft = snapshot.ai_secret_draft;
            self.cloud_sync.replace_settings(
                snapshot.cloud_sync_settings,
                snapshot.cloud_sync_secret_draft,
            );
            self.translation.replace_settings(
                snapshot.translation_settings,
                snapshot.translation_secret_draft,
            );
            self.settings.keyword_config = snapshot.keyword_highlights;
            self.settings.master_password.enabled = snapshot.master_password_enabled;
            self.settings.master_password.draft = snapshot.master_password_draft;
            self.recording
                .set_memory_limit(self.settings.summary.recording_memory_limit_bytes as usize);
            self.transfer.paths.duplicate_policy = SftpDuplicatePolicy::from_legacy_value(
                &self.settings.summary.transfer_duplicate_strategy,
            );
            self.sync_terminal_encodings_from_settings();
            self.invalidate_terminal_cell_metrics(cx);
            self.invalidate_paint_theme_caches();
            self.sync_ai_drafts_from_active_profile();
            self.refresh_visible_terminal_surfaces(cx);
        }
        self.finish_settings_page(cx);
    }

    pub(in crate::features) fn confirm_settings_draft(&mut self, cx: &mut Context<Self>) {
        if self.settings_draft_dirty() {
            self.apply_settings_draft(true, cx);
        } else {
            self.shell.navigation.settings.draft_snapshot = None;
            self.finish_settings_page(cx);
        }
    }

    pub(in crate::features) fn toggle_settings_master_password(&mut self, cx: &mut Context<Self>) {
        self.terminal.view.status = match self
            .settings
            .master_password
            .toggle(self.cloud_sync.settings().enabled)
        {
            Ok(true) => "master password enabled; enter a password".to_string(),
            Ok(false) => "master password removal staged".to_string(),
            Err(error) => error.to_string(),
        };
        cx.notify();
    }

    /// Apply an edit from the master password box.
    pub(in crate::features) fn apply_settings_master_password(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        if !self.settings.master_password.edit_draft(text) {
            return;
        }
        self.terminal.view.status = "master password edited; apply to persist".to_string();
        cx.notify();
    }

    fn finish_settings_page(&mut self, cx: &mut Context<Self>) {
        self.cancel_github_gist_auth(cx);
        self.ai.settings.action_edit = None;
        self.ai.settings.manual_model_edit_group = None;
        self.settings.keyword_highlights.edit_id = None;
        self.forget_text_inputs("ai.settings.action.");
        self.forget_text_inputs("ai.settings.manual-model.");
        self.forget_text_inputs("keyword.highlight.");
        self.shell.navigation.settings.window = None;
        self.shell.navigation.settings.window_open_pending = false;
        if self.shell.navigation.main_mode == MainMode::Page
            && self.shell.navigation.selected_nav == NavItem::Settings
        {
            self.shell.navigation.main_mode = MainMode::Workspace;
            self.shell.panels.left_collapsed = self
                .shell
                .navigation
                .settings
                .previous_left_collapsed
                .take()
                .unwrap_or_else(|| self.shell.panels.active_left.is_none());
            self.shell.panels.right_collapsed = self
                .shell
                .navigation
                .settings
                .previous_right_collapsed
                .take()
                .unwrap_or_else(|| self.shell.panels.active_right.is_none());
            self.persist_ui_layout();
        } else {
            self.shell.navigation.settings.previous_left_collapsed = None;
            self.shell.navigation.settings.previous_right_collapsed = None;
        }
        self.terminal.view.status = "settings closed".to_string();
        cx.notify();
    }
}

fn drive_validation_error(
    refresh_token: Option<&str>,
    client_id: Option<&str>,
    client_secret: Option<&str>,
) -> Option<&'static str> {
    if refresh_token.unwrap_or("").trim().is_empty() {
        return Some("Drive refresh token is required");
    }
    if client_id.unwrap_or("").trim().is_empty() {
        return Some("Drive client ID is required");
    }
    if client_secret.unwrap_or("").trim().is_empty() {
        return Some("Drive client secret is required");
    }
    None
}
