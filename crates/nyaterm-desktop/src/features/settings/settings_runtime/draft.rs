use super::*;
use crate::features::app_state::SettingsDraftSnapshot;
use crate::models::{CloudSyncSecretDraft, MainMode, TranslationSecretDraft};
use nyaterm_core::TranslationSettings;

impl NyaTermApp {
    pub(in crate::features) fn begin_settings_draft(&mut self) {
        if self.settings_draft_snapshot.is_some() {
            return;
        }
        self.settings_draft_snapshot = Some(SettingsDraftSnapshot {
            settings: self.settings.clone(),
            ai_settings: self.ai.settings.config.clone(),
            ai_model_draft: self.ai.settings.model_draft.clone(),
            ai_base_url_draft: self.ai.settings.base_url_draft.clone(),
            ai_secret_draft: self.ai.settings.secret_draft.clone(),
            cloud_sync_settings: self.cloud_sync_settings.clone(),
            cloud_sync_secret_draft: self.cloud_sync_secret_draft.clone(),
            translation_settings: self.translation_settings.clone(),
            translation_secret_draft: self.translation_secret_draft.clone(),
            keyword_highlights: self.keyword_highlights.clone(),
            master_password_enabled: self.settings_master_password_enabled,
            master_password_draft: self.settings_master_password_draft.clone(),
        });
    }

    pub(in crate::features) fn settings_draft_dirty(&self) -> bool {
        let Some(snapshot) = self.settings_draft_snapshot.as_ref() else {
            return false;
        };
        snapshot.settings != self.settings
            || snapshot.ai_settings != self.ai.settings.config
            || snapshot.ai_model_draft != self.ai.settings.model_draft
            || snapshot.ai_base_url_draft != self.ai.settings.base_url_draft
            || snapshot.ai_secret_draft != self.ai.settings.secret_draft
            || snapshot.cloud_sync_settings != self.cloud_sync_settings
            || snapshot.cloud_sync_secret_draft != self.cloud_sync_secret_draft
            || snapshot.translation_settings != self.translation_settings
            || snapshot.translation_secret_draft != self.translation_secret_draft
            || snapshot.keyword_highlights != self.keyword_highlights
            || snapshot.master_password_enabled != self.settings_master_password_enabled
            || snapshot.master_password_draft != self.settings_master_password_draft
    }

    /// Returns true when a settings save should stay in the in-memory draft.
    pub(in crate::features) fn defer_settings_persistence(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.settings_draft_snapshot.is_none() {
            return false;
        }
        self.store_status.message = "settings draft changed".to_string();
        self.store_status.ready = true;
        self.terminal_status = "settings draft changed; apply to persist".to_string();
        cx.notify();
        true
    }

    pub(in crate::features) fn pending_translation_settings(&self) -> TranslationSettings {
        let mut next = self.translation_settings.clone();
        if !self.translation_secret_draft.deepl_api_key.is_empty() {
            next.deepl_api_key = self.translation_secret_draft.deepl_api_key.clone();
        }
        if !self.translation_secret_draft.baidu_app_key.is_empty() {
            next.baidu_app_key = self.translation_secret_draft.baidu_app_key.clone();
        }
        if !self.translation_secret_draft.ali_app_key.is_empty() {
            next.ali_app_key = self.translation_secret_draft.ali_app_key.clone();
        }
        if !self.translation_secret_draft.youdao_app_key.is_empty() {
            next.youdao_app_key = self.translation_secret_draft.youdao_app_key.clone();
        }
        next
    }

    pub(in crate::features) fn pending_cloud_sync_settings(&self) -> CloudSyncSettings {
        let mut next = self.cloud_sync_settings.clone();
        let draft = &self.cloud_sync_secret_draft;
        if !draft.webdav_password.is_empty() {
            next.webdav.password = Some(draft.webdav_password.clone());
        }
        if !draft.s3_access_key_id.is_empty() {
            next.s3.access_key_id = Some(draft.s3_access_key_id.clone());
        }
        if !draft.s3_secret_access_key.is_empty() {
            next.s3.secret_access_key = Some(draft.s3_secret_access_key.clone());
        }
        if !draft.s3_session_token.is_empty() {
            next.s3.session_token = Some(draft.s3_session_token.clone());
        }
        if !draft.google_drive_access_token.is_empty() {
            next.google_drive.access_token = Some(draft.google_drive_access_token.clone());
        }
        if !draft.google_drive_refresh_token.is_empty() {
            next.google_drive.refresh_token = Some(draft.google_drive_refresh_token.clone());
        }
        if !draft.google_drive_client_secret.is_empty() {
            next.google_drive.client_secret = Some(draft.google_drive_client_secret.clone());
        }
        if !draft.onedrive_access_token.is_empty() {
            next.onedrive.access_token = Some(draft.onedrive_access_token.clone());
        }
        if !draft.onedrive_refresh_token.is_empty() {
            next.onedrive.refresh_token = Some(draft.onedrive_refresh_token.clone());
        }
        if !draft.onedrive_client_secret.is_empty() {
            next.onedrive.client_secret = Some(draft.onedrive_client_secret.clone());
        }
        if !draft.aliyun_drive_access_token.is_empty() {
            next.aliyun_drive.access_token = Some(draft.aliyun_drive_access_token.clone());
        }
        if !draft.aliyun_drive_refresh_token.is_empty() {
            next.aliyun_drive.refresh_token = Some(draft.aliyun_drive_refresh_token.clone());
        }
        if !draft.aliyun_drive_client_secret.is_empty() {
            next.aliyun_drive.client_secret = Some(draft.aliyun_drive_client_secret.clone());
        }
        if !draft.gitee_token.is_empty() {
            next.gitee_snippet.access_token = Some(draft.gitee_token.clone());
        }
        if !draft.github_token.is_empty() {
            next.github_gist.access_token = Some(draft.github_token.clone());
        }
        next
    }

    pub(in crate::features) fn pending_settings_cloud_error(&self) -> Option<String> {
        let settings = self.pending_cloud_sync_settings();
        if !settings.enabled {
            return None;
        }
        if !self.settings_master_password_enabled {
            return Some("Enable a master password before enabling cloud sync".to_string());
        }
        if !self.settings.has_master_password && self.settings_master_password_draft.is_empty() {
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
        self.cloud_sync_status = "apply settings before running cloud sync".to_string();
        self.terminal_status = self.cloud_sync_status.clone();
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
        self.terminal_status = "apply or cancel settings before importing".to_string();
        self.store_status.message = self.terminal_status.clone();
        self.store_status.ready = false;
        cx.notify();
        true
    }

    pub(in crate::features) fn rebase_open_settings_draft(&mut self) {
        if self.settings_draft_snapshot.is_none() {
            return;
        }
        self.settings_draft_snapshot = None;
        self.settings_master_password_enabled = self.settings.has_master_password;
        self.settings_master_password_draft.clear();
        self.begin_settings_draft();
    }

    pub(in crate::features) fn apply_settings_draft(
        &mut self,
        close_after_apply: bool,
        cx: &mut Context<Self>,
    ) {
        if self.settings_draft_snapshot.is_none() {
            if close_after_apply {
                self.finish_settings_page(cx);
            }
            return;
        }
        if let Some(error) = self.pending_settings_cloud_error() {
            self.store_status.message = error.clone();
            self.store_status.ready = false;
            self.terminal_status = format!("settings apply blocked: {error}");
            cx.notify();
            return;
        }

        let settings = self.settings.clone();
        let ai_settings = self.pending_ai_settings();
        let cloud_sync_settings = self.pending_cloud_sync_settings();
        let translation_settings = self.pending_translation_settings();
        let keyword_highlights = self.keyword_highlights.clone();
        let master_password_update = if self.settings_master_password_draft.is_empty() {
            (self.settings.has_master_password && !self.settings_master_password_enabled)
                .then_some(None)
        } else {
            Some(Some(self.settings_master_password_draft.clone()))
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
                self.settings_master_password_enabled = self.settings.has_master_password;
                self.settings_master_password_draft.clear();
                self.ai.settings.config = saved_ai_settings;
                self.cloud_sync_settings = saved_cloud_sync_settings;
                self.translation_settings = saved_translation_settings;
                self.keyword_highlights = saved_keyword_highlights;
                self.translation_secret_draft = TranslationSecretDraft::default();
                self.cloud_sync_secret_draft = CloudSyncSecretDraft::default();
                self.ai.settings.secret_draft.clear();
                self.sync_ai_drafts_from_active_profile();
                self.translate_target_language = self.translation_settings.target_language.clone();
                self.recording_manager
                    .set_memory_limit(self.settings.recording_memory_limit_bytes as usize);
                self.transfer.paths.duplicate_policy = SftpDuplicatePolicy::from_legacy_value(
                    &self.settings.transfer_duplicate_strategy,
                );
                self.sync_terminal_encodings_from_settings();
                self.enforce_terminal_scrollback_limit();
                if !self.settings.interaction_command_suggestions_enabled {
                    self.command_suggestions = None;
                    self.command_input_tracker = TerminalInputState::new();
                    self.command_suggestions_suppressed = false;
                    self.pending_command_history_entry = None;
                }
                self.invalidate_terminal_cell_metrics(cx);
                self.refresh_visible_terminal_surfaces(cx);
                if !self.settings.startup_restore_window_layout {
                    let _ = ConnectionStore::open_with_portable_key_path(
                        self.runtime.config_dir(),
                        self.runtime.portable_key_path().map(ToOwned::to_owned),
                    )
                    .and_then(|store| {
                        store.save_terminal_window_layout(None)?;
                        store.save_workspace_pane_layout(None)
                    });
                }
                self.settings_draft_snapshot = None;
                self.store_status.message = "settings applied".to_string();
                self.store_status.ready = true;
                self.terminal_status = "settings applied".to_string();
                if close_after_apply {
                    self.finish_settings_page(cx);
                } else {
                    self.begin_settings_draft();
                    cx.notify();
                }
            }
            Err(error) => {
                self.store_status.message = format!("settings apply failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
                cx.notify();
            }
        }
    }

    pub(in crate::features) fn cancel_settings(&mut self, cx: &mut Context<Self>) {
        if let Some(snapshot) = self.settings_draft_snapshot.take() {
            self.apply_gpui_settings(snapshot.settings);
            self.ai.settings.config = snapshot.ai_settings;
            self.ai.settings.model_draft = snapshot.ai_model_draft;
            self.ai.settings.base_url_draft = snapshot.ai_base_url_draft;
            self.ai.settings.secret_draft = snapshot.ai_secret_draft;
            self.cloud_sync_settings = snapshot.cloud_sync_settings;
            self.cloud_sync_secret_draft = snapshot.cloud_sync_secret_draft;
            self.translation_settings = snapshot.translation_settings;
            self.translation_secret_draft = snapshot.translation_secret_draft;
            self.keyword_highlights = snapshot.keyword_highlights;
            self.settings_master_password_enabled = snapshot.master_password_enabled;
            self.settings_master_password_draft = snapshot.master_password_draft;
            self.recording_manager
                .set_memory_limit(self.settings.recording_memory_limit_bytes as usize);
            self.transfer.paths.duplicate_policy =
                SftpDuplicatePolicy::from_legacy_value(&self.settings.transfer_duplicate_strategy);
            self.sync_terminal_encodings_from_settings();
            self.invalidate_terminal_cell_metrics(cx);
            self.invalidate_paint_theme_caches();
            self.sync_ai_drafts_from_active_profile();
            self.translate_target_language = self.translation_settings.target_language.clone();
            self.refresh_visible_terminal_surfaces(cx);
        }
        self.finish_settings_page(cx);
    }

    pub(in crate::features) fn confirm_settings_draft(&mut self, cx: &mut Context<Self>) {
        if self.settings_draft_dirty() {
            self.apply_settings_draft(true, cx);
        } else {
            self.settings_draft_snapshot = None;
            self.finish_settings_page(cx);
        }
    }

    pub(in crate::features) fn toggle_settings_master_password(&mut self, cx: &mut Context<Self>) {
        if self.cloud_sync_settings.enabled && self.settings_master_password_enabled {
            self.terminal_status =
                "disable cloud sync before removing the master password".to_string();
            cx.notify();
            return;
        }
        self.settings_master_password_enabled = !self.settings_master_password_enabled;
        self.settings_master_password_draft.clear();
        self.terminal_status = if self.settings_master_password_enabled {
            "master password enabled; enter a password".to_string()
        } else {
            "master password removal staged".to_string()
        };
        cx.notify();
    }

    pub(in crate::features) fn handle_settings_master_password_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        if !self.settings_master_password_enabled {
            return;
        }
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }
        match keystroke.key.as_str() {
            "backspace" => {
                self.settings_master_password_draft.pop();
            }
            "escape" => {
                self.settings_master_password_draft.clear();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.settings_master_password_draft.push_str(input);
                }
            }
        }
        self.terminal_status = "master password edited; apply to persist".to_string();
        cx.notify();
    }

    fn finish_settings_page(&mut self, cx: &mut Context<Self>) {
        self.cancel_github_gist_auth(cx);
        self.settings_window = None;
        self.settings_window_open_pending = false;
        if self.main_mode == MainMode::Page && self.selected_nav == NavItem::Settings {
            self.main_mode = MainMode::Workspace;
            self.left_sidebar_collapsed = self
                .settings_previous_left_collapsed
                .take()
                .unwrap_or_else(|| self.active_left_panel.is_none());
            self.right_inspector_collapsed = self
                .settings_previous_right_collapsed
                .take()
                .unwrap_or_else(|| self.active_right_panel.is_none());
            self.persist_ui_layout();
        } else {
            self.settings_previous_left_collapsed = None;
            self.settings_previous_right_collapsed = None;
        }
        self.terminal_status = "settings closed".to_string();
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
