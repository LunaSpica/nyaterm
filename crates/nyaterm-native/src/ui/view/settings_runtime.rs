use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn update_host_key_policy(
        &mut self,
        policy: &'static str,
        cx: &mut Context<Self>,
    ) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_host_key_policy(policy))
        {
            Ok(settings) => {
                self.settings = settings;
                self.terminal_status = format!("host key policy set to {policy}");
                self.store_status.message = "settings saved".to_string();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.terminal_status = format!("failed to save host key policy: {error}");
                self.store_status.message = format!("settings save failed: {error}");
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_recording_auto_start(&mut self, cx: &mut Context<Self>) {
        self.settings.recording_auto_start = !self.settings.recording_auto_start;
        self.save_recording_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_recording_io_labels(&mut self, cx: &mut Context<Self>) {
        self.settings.recording_include_io_labels = !self.settings.recording_include_io_labels;
        self.save_recording_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_recording_timestamps(&mut self, cx: &mut Context<Self>) {
        self.settings.recording_include_timestamps = !self.settings.recording_include_timestamps;
        self.save_recording_settings(cx);
    }

    pub(in crate::ui::view) fn adjust_recording_memory_limit(
        &mut self,
        delta_mib: i64,
        cx: &mut Context<Self>,
    ) {
        let current_mib = (self.settings.recording_memory_limit_bytes / (1024 * 1024)).max(1);
        let next_mib = if delta_mib.is_negative() {
            current_mib.saturating_sub(delta_mib.unsigned_abs()).max(1)
        } else {
            current_mib.saturating_add(delta_mib as u64).min(512)
        };
        self.settings.recording_memory_limit_bytes = next_mib * 1024 * 1024;
        self.save_recording_settings(cx);
    }

    pub(in crate::ui::view) fn save_recording_settings(&mut self, cx: &mut Context<Self>) {
        self.recording_manager
            .set_memory_limit(self.settings.recording_memory_limit_bytes as usize);
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_recording_settings(&self.settings))
        {
            Ok(settings) => {
                self.settings = settings;
                self.recording_manager
                    .set_memory_limit(self.settings.recording_memory_limit_bytes as usize);
                self.store_status.message = "recording settings saved".to_string();
                self.store_status.ready = true;
                self.terminal_status = "recording settings saved".to_string();
            }
            Err(error) => {
                self.store_status.message = format!("recording settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn update_transfer_duplicate_policy(
        &mut self,
        policy: SftpDuplicatePolicy,
        cx: &mut Context<Self>,
    ) {
        self.transfer_duplicate_policy = policy;
        self.settings.transfer_duplicate_strategy = duplicate_policy_label(policy).to_string();
        self.save_transfer_settings("transfer duplicate policy saved", cx);
    }

    pub(in crate::ui::view) fn update_transfer_editor_type(
        &mut self,
        editor_type: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.transfer_editor_type = editor_type.to_string();
        self.save_transfer_settings("transfer editor preference saved", cx);
    }

    pub(in crate::ui::view) fn toggle_transfer_ask_save_location(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.transfer_ask_save_location = !self.settings.transfer_ask_save_location;
        self.save_transfer_settings("transfer save-location preference saved", cx);
    }

    pub(in crate::ui::view) fn toggle_transfer_preserve_timestamps(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.transfer_preserve_timestamps = !self.settings.transfer_preserve_timestamps;
        self.save_transfer_settings("transfer timestamp preference saved", cx);
    }

    pub(in crate::ui::view) fn toggle_transfer_resume_broken(&mut self, cx: &mut Context<Self>) {
        self.settings.transfer_resume_broken_transfer =
            !self.settings.transfer_resume_broken_transfer;
        self.save_transfer_settings("transfer resume preference saved", cx);
    }

    pub(in crate::ui::view) fn adjust_transfer_download_threads(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        self.settings.transfer_download_threads =
            adjust_u32_setting(self.settings.transfer_download_threads, delta, 1, 10);
        self.save_transfer_settings("transfer download concurrency saved", cx);
    }

    pub(in crate::ui::view) fn adjust_transfer_upload_threads(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        self.settings.transfer_upload_threads =
            adjust_u32_setting(self.settings.transfer_upload_threads, delta, 1, 10);
        self.save_transfer_settings("transfer upload concurrency saved", cx);
    }

    pub(in crate::ui::view) fn adjust_transfer_max_retries(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        self.settings.transfer_max_retries =
            adjust_u32_setting(self.settings.transfer_max_retries, delta, 0, 10);
        self.save_transfer_settings("transfer retry setting saved", cx);
    }

    pub(in crate::ui::view) fn adjust_transfer_buffer_size(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let step_delta = delta.saturating_mul(8);
        self.settings.transfer_buffer_size =
            adjust_u32_setting(self.settings.transfer_buffer_size, step_delta, 8, 256);
        self.save_transfer_settings("transfer buffer setting saved", cx);
    }

    pub(in crate::ui::view) fn update_transfer_file_permissions(
        &mut self,
        permissions: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.transfer_default_file_permissions = permissions.to_string();
        self.save_transfer_settings("transfer default permissions saved", cx);
    }

    pub(in crate::ui::view) fn save_transfer_settings(
        &mut self,
        success_status: &'static str,
        cx: &mut Context<Self>,
    ) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_transfer_settings(&self.settings))
        {
            Ok(settings) => {
                self.settings = settings;
                self.transfer_duplicate_policy = SftpDuplicatePolicy::from_legacy_value(
                    &self.settings.transfer_duplicate_strategy,
                );
                self.store_status.message = "transfer settings saved".to_string();
                self.store_status.ready = true;
                self.terminal_status = success_status.to_string();
            }
            Err(error) => {
                self.store_status.message = format!("transfer settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_transfer_default_editor_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.settings.transfer_default_editor.pop();
                self.terminal_status = "transfer editor command edited".to_string();
                cx.notify();
            }
            "enter" => {
                self.save_transfer_settings("transfer editor command saved", cx);
            }
            "escape" => {
                self.terminal_status = "transfer editor command input blurred".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.settings.transfer_default_editor.push_str(input);
                    self.terminal_status = "transfer editor command edited".to_string();
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::ui::view) fn update_x11_display(
        &mut self,
        value: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.x11_display = value.to_string();
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn adjust_terminal_scrollback_lines(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.terminal_scrollback_lines as i32 + delta).clamp(100, 100_000);
        self.settings.terminal_scrollback_lines = next as u32;
        self.enforce_terminal_scrollback_limit();
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn adjust_terminal_keep_alive_interval(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.terminal_keep_alive_interval as i32 + delta).clamp(0, 600);
        self.settings.terminal_keep_alive_interval = next as u32;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_terminal_hardware_acceleration(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.terminal_hardware_acceleration =
            !self.settings.terminal_hardware_acceleration;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_terminal_workspace_padding(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.terminal_show_workspace_padding =
            !self.settings.terminal_show_workspace_padding;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_terminal_line_numbers(&mut self, cx: &mut Context<Self>) {
        self.settings.terminal_show_line_numbers = !self.settings.terminal_show_line_numbers;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_terminal_timestamps(&mut self, cx: &mut Context<Self>) {
        self.settings.terminal_show_timestamps = !self.settings.terminal_show_timestamps;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_terminal_timestamp_milliseconds(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.terminal_show_timestamp_milliseconds =
            !self.settings.terminal_show_timestamp_milliseconds;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_multi_line_paste_dialog(&mut self, cx: &mut Context<Self>) {
        self.settings.terminal_show_multi_line_paste_dialog =
            !self.settings.terminal_show_multi_line_paste_dialog;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_paste_image_as_path(&mut self, cx: &mut Context<Self>) {
        self.settings.terminal_paste_image_as_path = !self.settings.terminal_paste_image_as_path;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_remote_stats_panel(&mut self, cx: &mut Context<Self>) {
        self.settings.ui_show_remote_stats = !self.settings.ui_show_remote_stats;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn adjust_remote_stats_interval(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.ui_remote_stats_interval as i32 + delta).clamp(1, 60);
        self.settings.ui_remote_stats_interval = next as u32;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_process_manager_panel(&mut self, cx: &mut Context<Self>) {
        self.settings.ui_show_process_manager = !self.settings.ui_show_process_manager;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn adjust_process_manager_interval(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.ui_process_manager_interval as i32 + delta).clamp(3, 120);
        self.settings.ui_process_manager_interval = next as u32;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_docker_manager_panel(&mut self, cx: &mut Context<Self>) {
        self.settings.ui_show_docker_manager = !self.settings.ui_show_docker_manager;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn adjust_docker_manager_interval(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.ui_docker_manager_interval as i32 + delta).clamp(3, 120);
        self.settings.ui_docker_manager_interval = next as u32;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn save_terminal_settings(&mut self, cx: &mut Context<Self>) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_terminal_settings(&self.settings))
        {
            Ok(settings) => {
                self.settings = settings;
                self.enforce_terminal_scrollback_limit();
                self.store_status.message = "terminal settings saved".to_string();
                self.store_status.ready = true;
                self.terminal_status = "terminal settings saved".to_string();
            }
            Err(error) => {
                self.store_status.message = format!("terminal settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn update_ui_language(
        &mut self,
        language: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.language = language.to_string();
        self.save_general_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_startup_restore(&mut self, cx: &mut Context<Self>) {
        self.settings.startup_restore = !self.settings.startup_restore;
        self.save_general_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_startup_restore_window_layout(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.startup_restore_window_layout =
            !self.settings.startup_restore_window_layout;
        self.save_general_settings(cx);
        if !self.settings.startup_restore_window_layout {
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

    pub(in crate::ui::view) fn toggle_confirm_on_close(&mut self, cx: &mut Context<Self>) {
        self.settings.confirm_on_close = !self.settings.confirm_on_close;
        self.save_general_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_minimize_to_tray(&mut self, cx: &mut Context<Self>) {
        self.settings.minimize_to_tray = !self.settings.minimize_to_tray;
        self.save_general_settings(cx);
    }

    pub(in crate::ui::view) fn set_diagnostics_level(
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

    pub(in crate::ui::view) fn set_diagnostics_retention_days(
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

    pub(in crate::ui::view) fn save_diagnostics_settings(&mut self, cx: &mut Context<Self>) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_diagnostics_settings(&self.settings))
        {
            Ok(settings) => {
                self.settings = settings;
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

    pub(in crate::ui::view) fn save_general_settings(&mut self, cx: &mut Context<Self>) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_general_settings(&self.settings))
        {
            Ok(settings) => {
                self.settings = settings;
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

    pub(in crate::ui::view) fn toggle_interaction_copy_on_select(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.interaction_copy_on_select = !self.settings.interaction_copy_on_select;
        self.save_interaction_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_interaction_right_click_paste(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.interaction_right_click_paste = !self.settings.interaction_right_click_paste;
        self.save_interaction_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_command_suggestions(&mut self, cx: &mut Context<Self>) {
        self.settings.interaction_command_suggestions_enabled =
            !self.settings.interaction_command_suggestions_enabled;
        if !self.settings.interaction_command_suggestions_enabled {
            self.command_suggestions = None;
            self.command_input_tracker = TerminalInputState::new();
            self.command_suggestions_suppressed = false;
            self.pending_command_history_entry = None;
        }
        self.save_interaction_settings(cx);
    }

    pub(in crate::ui::view) fn adjust_command_suggestion_min_chars(
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

    pub(in crate::ui::view) fn adjust_command_suggestion_max_chars(
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

    pub(in crate::ui::view) fn adjust_duplicate_session_command_delay(
        &mut self,
        delta_ms: i32,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.interaction_duplicate_session_command_delay_ms as i32 + delta_ms)
            .clamp(0, 60_000) as u32;
        self.settings.interaction_duplicate_session_command_delay_ms = next;
        self.save_interaction_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_alt_as_meta(&mut self, cx: &mut Context<Self>) {
        self.settings.interaction_alt_as_meta = !self.settings.interaction_alt_as_meta;
        self.save_interaction_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_mac_ime_compatibility(&mut self, cx: &mut Context<Self>) {
        self.settings.interaction_mac_ime_compatibility =
            !self.settings.interaction_mac_ime_compatibility;
        self.save_interaction_settings(cx);
    }

    pub(in crate::ui::view) fn set_interaction_encoding(
        &mut self,
        encoding: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.interaction_default_encoding = encoding.to_string();
        self.save_interaction_settings(cx);
    }

    pub(in crate::ui::view) fn set_interaction_word_separators(
        &mut self,
        value: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.interaction_word_separators = value.to_string();
        self.save_interaction_settings(cx);
    }

    pub(in crate::ui::view) fn save_interaction_settings(&mut self, cx: &mut Context<Self>) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_interaction_settings(&self.settings))
        {
            Ok(settings) => {
                self.settings = settings;
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

    pub(in crate::ui::view) fn toggle_screen_lock_enabled(&mut self, cx: &mut Context<Self>) {
        self.settings.enable_screen_lock = !self.settings.enable_screen_lock;
        self.last_user_activity_at = Instant::now();
        self.save_screen_lock_settings(cx);
    }

    pub(in crate::ui::view) fn adjust_idle_lock_minutes(
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

    pub(in crate::ui::view) fn save_screen_lock_settings(&mut self, cx: &mut Context<Self>) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_screen_lock_settings(&self.settings))
        {
            Ok(settings) => {
                self.settings = settings;
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

    pub(in crate::ui::view) fn add_search_engine(&mut self, cx: &mut Context<Self>) {
        self.settings.search_custom_engines.insert(
            0,
            SearchEngineConfig {
                name: "New Engine".to_string(),
                url_template: "https://example.com/search?q=%s".to_string(),
                icon: Some("default".to_string()),
                show_in_menu: true,
            },
        );
        self.search_engine_expanded_index = Some(0);
        self.search_engine_edit_index = Some(0);
        self.search_engine_edit_field = SearchEngineEditorField::Name;
        self.save_terminal_settings(cx);
        self.terminal_status = "search engine added".to_string();
    }

    pub(in crate::ui::view) fn remove_search_engine(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        if index >= self.settings.search_custom_engines.len() {
            return;
        }
        self.settings.search_custom_engines.remove(index);
        if self.settings.search_custom_engines.is_empty() {
            self.settings.search_custom_engines = default_search_engines();
        }
        if self.search_engine_expanded_index == Some(index) {
            self.search_engine_expanded_index = None;
        } else if let Some(edit) = self.search_engine_expanded_index {
            if edit > index {
                self.search_engine_expanded_index = Some(edit - 1);
            }
        }
        if let Some(edit) = self.search_engine_edit_index {
            if edit == index {
                self.search_engine_edit_index = None;
            } else if edit > index {
                self.search_engine_edit_index = Some(edit - 1);
            }
        }
        self.save_terminal_settings(cx);
        self.terminal_status = "search engine removed".to_string();
    }


    pub(in crate::ui::view) fn cycle_search_engine_icon(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(engine) = self.settings.search_custom_engines.get_mut(index) else {
            return;
        };
        const ICONS: &[&str] = &[
            "google",
            "bing",
            "duckduckgo",
            "github",
            "gitlab",
            "baidu",
            "yahoo",
            "youtube",
            "bilibili",
            "zhihu",
            "openai",
            "claude",
            "gemini",
            "default",
        ];
        let current = engine.icon.as_deref().unwrap_or("default");
        let next = ICONS
            .iter()
            .position(|icon| *icon == current)
            .map(|i| ICONS[(i + 1) % ICONS.len()])
            .unwrap_or("google");
        engine.icon = Some(next.to_string());
        self.save_terminal_settings(cx);
        self.terminal_status = format!("search engine icon: {next}");
    }

    pub(in crate::ui::view) fn toggle_search_engine_in_menu(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(engine) = self.settings.search_custom_engines.get_mut(index) else {
            return;
        };
        engine.show_in_menu = !engine.show_in_menu;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn focus_search_engine_field(
        &mut self,
        index: usize,
        field: SearchEngineEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if index >= self.settings.search_custom_engines.len() {
            return;
        }
        self.search_engine_edit_index = Some(index);
        self.search_engine_edit_field = field;
        window.focus(&self.search_engine_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_search_engine_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let Some(index) = self.search_engine_edit_index else {
            return;
        };
        if index >= self.settings.search_custom_engines.len() {
            return;
        }
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }
        match keystroke.key.as_str() {
            "backspace" => {
                let engine = &mut self.settings.search_custom_engines[index];
                match self.search_engine_edit_field {
                    SearchEngineEditorField::Name => {
                        engine.name.pop();
                    }
                    SearchEngineEditorField::Url => {
                        engine.url_template.pop();
                    }
                }
                self.terminal_status = "search engine edited".to_string();
                cx.notify();
            }
            "tab" => {
                self.search_engine_edit_field = self.search_engine_edit_field.next();
                cx.notify();
            }
            "enter" => {
                self.normalize_search_engines();
                self.save_terminal_settings(cx);
                self.terminal_status = "search engines saved".to_string();
            }
            "escape" => {
                self.search_engine_edit_index = None;
                self.terminal_status = "search engine input blurred".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    let engine = &mut self.settings.search_custom_engines[index];
                    match self.search_engine_edit_field {
                        SearchEngineEditorField::Name => engine.name.push_str(input),
                        SearchEngineEditorField::Url => engine.url_template.push_str(input),
                    }
                    self.terminal_status = "search engine edited".to_string();
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::ui::view) fn reset_search_engines(&mut self, cx: &mut Context<Self>) {
        self.settings.search_custom_engines = default_search_engines();
        self.search_engine_edit_index = None;
        self.save_terminal_settings(cx);
        self.terminal_status = "search engines reset to defaults".to_string();
    }

    pub(in crate::ui::view) fn expand_search_engine(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        if self.search_engine_expanded_index == Some(index) {
            self.search_engine_expanded_index = None;
            self.search_engine_edit_index = None;
        } else {
            self.search_engine_expanded_index = Some(index);
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn test_search_engine(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(engine) = self.settings.search_custom_engines.get(index) else {
            self.terminal_status = "search engine not found".to_string();
            cx.notify();
            return;
        };
        if !engine.url_template.contains("%s") {
            self.terminal_status = "search engine URL must include %s".to_string();
            cx.notify();
            return;
        }
        let url = engine
            .url_template
            .replace("%s", &urlencoding_query("nyaterm"));
        match open_external_url_simple(&url) {
            Ok(()) => {
                self.terminal_status = format!("tested search engine: {}", engine.name);
            }
            Err(error) => {
                self.terminal_status = format!("test search engine failed: {error}");
            }
        }
        cx.notify();
    }


    pub(in crate::ui::view) fn toggle_terminal_action_links(&mut self, cx: &mut Context<Self>) {
        self.settings.terminal_action_links_enabled = !self.settings.terminal_action_links_enabled;
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn toggle_terminal_action_links_matcher(
        &mut self,
        which: &'static str,
        cx: &mut Context<Self>,
    ) {
        match which {
            "ipv4" => {
                self.settings.terminal_action_links_matchers.ipv4 =
                    !self.settings.terminal_action_links_matchers.ipv4;
            }
            "archive" => {
                self.settings.terminal_action_links_matchers.archive =
                    !self.settings.terminal_action_links_matchers.archive;
            }
            "host_port" => {
                self.settings.terminal_action_links_matchers.host_port =
                    !self.settings.terminal_action_links_matchers.host_port;
            }
            _ => return,
        }
        self.save_terminal_settings(cx);
    }

    pub(in crate::ui::view) fn execute_action_link_command(
        &mut self,
        command: String,
        cx: &mut Context<Self>,
    ) {
        let mut bytes = command.into_bytes();
        if !bytes.ends_with(b"\n") {
            bytes.push(b'\n');
        }
        self.send_terminal_input(bytes, cx);
        self.terminal_status = "action link command sent".to_string();
    }

    fn normalize_search_engines(&mut self) {
        self.settings.search_custom_engines.retain(|engine| {
            !engine.name.trim().is_empty() && !engine.url_template.trim().is_empty()
        });
        if self.settings.search_custom_engines.is_empty() {
            self.settings.search_custom_engines = default_search_engines();
        }
        for engine in &mut self.settings.search_custom_engines {
            engine.name = engine.name.trim().to_string();
            engine.url_template = engine.url_template.trim().to_string();
        }
    }
}

fn adjust_u32_setting(current: u32, delta: i32, min: u32, max: u32) -> u32 {
    let next = if delta.is_negative() {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current.saturating_add(delta as u32)
    };
    next.clamp(min, max)
}



fn urlencoding_query(query: &str) -> String {
    let mut out = String::new();
    for ch in query.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(ch),
            ' ' => out.push_str("%20"),
            _ => {
                for byte in ch.to_string().into_bytes() {
                    out.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    out
}

fn open_external_url_simple(url: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = url;
        Err("open URL is not supported on this platform".to_string())
    }
}
