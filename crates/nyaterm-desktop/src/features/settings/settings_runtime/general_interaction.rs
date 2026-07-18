use super::*;

impl NyaTermApp {
    pub(in crate::features) fn update_ui_language(
        &mut self,
        language: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.language = language.to_string();
        self.save_general_settings(cx);
    }

    pub(in crate::features) fn toggle_startup_restore(&mut self, cx: &mut Context<Self>) {
        self.settings.startup_restore = !self.settings.startup_restore;
        self.save_general_settings(cx);
    }

    pub(in crate::features) fn toggle_startup_restore_window_layout(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.startup_restore_window_layout = !self.settings.startup_restore_window_layout;
        self.save_general_settings(cx);
        if !self.settings.startup_restore_window_layout && self.settings_draft_snapshot.is_none() {
            // Clear stored layouts when the user disables restore.
            let _ = ConnectionStore::open_with_portable_key_path(
                self.runtime.config_dir(),
                self.runtime.portable_key_path().map(ToOwned::to_owned),
            )
            .and_then(|store| {
                store.save_terminal_window_layout(None)?;
                store.save_workspace_pane_layout(None)
            });
        }
    }

    pub(in crate::features) fn toggle_confirm_on_close(&mut self, cx: &mut Context<Self>) {
        self.settings.confirm_on_close = !self.settings.confirm_on_close;
        self.save_general_settings(cx);
    }

    pub(in crate::features) fn toggle_minimize_to_tray(&mut self, cx: &mut Context<Self>) {
        self.settings.minimize_to_tray = !self.settings.minimize_to_tray;
        self.save_general_settings(cx);
    }

    pub(in crate::features) fn set_diagnostics_level(
        &mut self,
        level: &'static str,
        cx: &mut Context<Self>,
    ) {
        let level = match level {
            "warn" | "debug" => level,
            _ => "info",
        };
        if self.settings.diagnostics_level == level {
            return;
        }
        self.settings.diagnostics_level = level.to_string();
        self.save_diagnostics_settings(cx);
    }

    pub(in crate::features) fn set_diagnostics_retention_days(
        &mut self,
        days: u32,
        cx: &mut Context<Self>,
    ) {
        let days = match days {
            3 | 7 | 14 | 30 => days,
            _ => 7,
        };
        if self.settings.diagnostics_retention_days == days {
            return;
        }
        self.settings.diagnostics_retention_days = days;
        self.save_diagnostics_settings(cx);
    }

    pub(in crate::features) fn save_diagnostics_settings(&mut self, cx: &mut Context<Self>) {
        if self.defer_settings_persistence(cx) {
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_diagnostics_settings(&self.settings))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.store_status.message = "diagnostics settings saved".to_string();
                self.store_status.ready = true;
                self.terminal_status = "diagnostics settings saved".to_string();
            }
            Err(error) => {
                self.store_status.message = format!("diagnostics settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn save_general_settings(&mut self, cx: &mut Context<Self>) {
        if self.defer_settings_persistence(cx) {
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_general_settings(&self.settings))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.store_status.message = "general settings saved".to_string();
                self.store_status.ready = true;
                self.terminal_status = "general settings saved".to_string();
            }
            Err(error) => {
                self.store_status.message = format!("general settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn toggle_interaction_copy_on_select(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.interaction_copy_on_select = !self.settings.interaction_copy_on_select;
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn toggle_interaction_right_click_paste(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.interaction_right_click_paste = !self.settings.interaction_right_click_paste;
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn toggle_command_suggestions(&mut self, cx: &mut Context<Self>) {
        self.settings.interaction_command_suggestions_enabled =
            !self.settings.interaction_command_suggestions_enabled;
        if !self.settings.interaction_command_suggestions_enabled
            && self.settings_draft_snapshot.is_none()
        {
            self.command_suggestions = None;
            self.command_input_tracker = TerminalInputState::new();
            self.command_suggestions_suppressed = false;
            self.pending_command_history_entry = None;
        }
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn adjust_command_suggestion_min_chars(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let max_chars = self.settings.interaction_command_suggestion_max_chars;
        let next = (self.settings.interaction_command_suggestion_min_chars as i32 + delta)
            .clamp(1, max_chars as i32) as u32;
        self.settings.interaction_command_suggestion_min_chars = next;
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn adjust_command_suggestion_max_chars(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let min_chars = self.settings.interaction_command_suggestion_min_chars;
        let next = (self.settings.interaction_command_suggestion_max_chars as i32 + delta)
            .clamp(min_chars as i32, 500) as u32;
        self.settings.interaction_command_suggestion_max_chars = next;
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn adjust_duplicate_session_command_delay(
        &mut self,
        delta_ms: i32,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.interaction_duplicate_session_command_delay_ms as i32 + delta_ms)
            .clamp(0, 60_000) as u32;
        self.settings.interaction_duplicate_session_command_delay_ms = next;
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn toggle_alt_as_meta(&mut self, cx: &mut Context<Self>) {
        self.settings.interaction_alt_as_meta = !self.settings.interaction_alt_as_meta;
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn toggle_mac_ime_compatibility(&mut self, cx: &mut Context<Self>) {
        self.settings.interaction_mac_ime_compatibility =
            !self.settings.interaction_mac_ime_compatibility;
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn set_interaction_encoding(
        &mut self,
        encoding: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.interaction_default_encoding = encoding.to_string();
        self.sync_terminal_encodings_from_settings();
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn set_interaction_word_separators(
        &mut self,
        value: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.interaction_word_separators = value.to_string();
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn save_interaction_settings(&mut self, cx: &mut Context<Self>) {
        if self.defer_settings_persistence(cx) {
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_interaction_settings(&self.settings))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.store_status.message = "interaction settings saved".to_string();
                self.store_status.ready = true;
                self.terminal_status = "interaction settings saved".to_string();
            }
            Err(error) => {
                self.store_status.message = format!("interaction settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn toggle_screen_lock_enabled(&mut self, cx: &mut Context<Self>) {
        self.settings.enable_screen_lock = !self.settings.enable_screen_lock;
        self.last_user_activity_at = Instant::now();
        self.save_screen_lock_settings(cx);
    }

    pub(in crate::features) fn adjust_idle_lock_minutes(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let current = self.settings.idle_lock_minutes as i32;
        let next = (current + delta).clamp(0, 1440);
        self.settings.idle_lock_minutes = next as u32;
        self.last_user_activity_at = Instant::now();
        self.save_screen_lock_settings(cx);
    }

    pub(in crate::features) fn save_screen_lock_settings(&mut self, cx: &mut Context<Self>) {
        if self.defer_settings_persistence(cx) {
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_screen_lock_settings(&self.settings))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.store_status.message = "screen lock settings saved".to_string();
                self.store_status.ready = true;
                self.terminal_status = "screen lock settings saved".to_string();
            }
            Err(error) => {
                self.store_status.message = format!("screen lock settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }
}
