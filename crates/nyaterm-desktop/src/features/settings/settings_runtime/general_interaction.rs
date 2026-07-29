use gpui::Context;
use nyaterm_core::ConnectionStore;

use crate::features::NyaTermApp;

impl NyaTermApp {
    pub(in crate::features) fn update_ui_language(
        &mut self,
        language: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.set_language(language);
        self.save_general_settings(cx);
    }

    pub(in crate::features) fn toggle_startup_restore(&mut self, cx: &mut Context<Self>) {
        self.settings.toggle_startup_restore();
        self.save_general_settings(cx);
    }

    pub(in crate::features) fn toggle_startup_restore_window_layout(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let restore_window_layout = self.settings.toggle_startup_restore_window_layout();
        self.save_general_settings(cx);
        if !restore_window_layout && !self.shell.has_settings_draft() {
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
        self.settings.toggle_confirm_on_close();
        self.save_general_settings(cx);
    }

    pub(in crate::features) fn toggle_minimize_to_tray(&mut self, cx: &mut Context<Self>) {
        self.settings.toggle_minimize_to_tray();
        self.save_general_settings(cx);
    }

    pub(in crate::features) fn set_diagnostics_level(
        &mut self,
        level: &'static str,
        cx: &mut Context<Self>,
    ) {
        if !self.settings.set_diagnostics_level(level) {
            return;
        }
        self.save_diagnostics_settings(cx);
    }

    pub(in crate::features) fn set_diagnostics_retention_days(
        &mut self,
        days: u32,
        cx: &mut Context<Self>,
    ) {
        if !self.settings.set_diagnostics_retention_days(days) {
            return;
        }
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
        .and_then(|store| store.save_diagnostics_settings(self.settings.summary()))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.settings
                    .update_store_status("diagnostics settings saved", true);
                self.shell.status = "diagnostics settings saved".to_string();
            }
            Err(error) => {
                let message = format!("diagnostics settings save failed: {error}");
                self.settings.update_store_status(message.clone(), false);
                self.shell.status = message;
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
        .and_then(|store| store.save_general_settings(self.settings.summary()))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.settings
                    .update_store_status("general settings saved", true);
                self.shell.status = "general settings saved".to_string();
            }
            Err(error) => {
                let message = format!("general settings save failed: {error}");
                self.settings.update_store_status(message.clone(), false);
                self.shell.status = message;
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn toggle_interaction_copy_on_select(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.toggle_interaction_copy_on_select();
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn toggle_interaction_right_click_paste(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.toggle_interaction_right_click_paste();
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn toggle_command_suggestions(&mut self, cx: &mut Context<Self>) {
        let suggestions_enabled = self.settings.toggle_command_suggestions();
        if !suggestions_enabled && !self.shell.has_settings_draft() {
            self.terminal.clear_command_tracking();
        }
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn adjust_command_suggestion_min_chars(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        self.settings.adjust_command_suggestion_min_chars(delta);
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn adjust_command_suggestion_max_chars(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        self.settings.adjust_command_suggestion_max_chars(delta);
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn adjust_duplicate_session_command_delay(
        &mut self,
        delta_ms: i32,
        cx: &mut Context<Self>,
    ) {
        self.settings
            .adjust_duplicate_session_command_delay(delta_ms);
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn toggle_alt_as_meta(&mut self, cx: &mut Context<Self>) {
        self.settings.toggle_alt_as_meta();
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn toggle_mac_ime_compatibility(&mut self, cx: &mut Context<Self>) {
        self.settings.toggle_mac_ime_compatibility();
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn set_interaction_encoding(
        &mut self,
        encoding: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.set_interaction_encoding(encoding);
        self.sync_terminal_encodings_from_settings();
        self.save_interaction_settings(cx);
    }

    /// Apply an edit from the word separators box.
    pub(in crate::features) fn apply_interaction_word_separators(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.settings.set_interaction_word_separators(text);
        cx.notify();
    }

    pub(in crate::features) fn save_interaction_settings(&mut self, cx: &mut Context<Self>) {
        if self.defer_settings_persistence(cx) {
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_interaction_settings(self.settings.summary()))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.settings
                    .update_store_status("interaction settings saved", true);
                self.shell.status = "interaction settings saved".to_string();
            }
            Err(error) => {
                let message = format!("interaction settings save failed: {error}");
                self.settings.update_store_status(message.clone(), false);
                self.shell.status = message;
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn toggle_screen_lock_enabled(&mut self, cx: &mut Context<Self>) {
        self.settings.toggle_screen_lock_enabled();
        self.security.reset_screen_lock_idle_timer();
        self.save_screen_lock_settings(cx);
    }

    pub(in crate::features) fn adjust_idle_lock_minutes(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        self.settings.adjust_idle_lock_minutes(delta);
        self.security.reset_screen_lock_idle_timer();
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
        .and_then(|store| store.save_screen_lock_settings(self.settings.summary()))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.settings
                    .update_store_status("screen lock settings saved", true);
                self.shell.status = "screen lock settings saved".to_string();
            }
            Err(error) => {
                let message = format!("screen lock settings save failed: {error}");
                self.settings.update_store_status(message.clone(), false);
                self.shell.status = message;
            }
        }
        cx.notify();
    }
}
