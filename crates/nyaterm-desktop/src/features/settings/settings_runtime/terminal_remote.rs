use gpui::Context;
use nyaterm_core::ConnectionStore;

use crate::features::NyaTermApp;

impl NyaTermApp {
    /// Apply an edit from the X11 display box.
    pub(in crate::features) fn apply_terminal_x11_display(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.settings.set_terminal_x11_display(text);
        cx.notify();
    }

    pub(in crate::features) fn toggle_terminal_hardware_acceleration(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.toggle_terminal_hardware_acceleration();
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_terminal_low_latency_mode(&mut self, cx: &mut Context<Self>) {
        let low_latency_mode = self.settings.toggle_terminal_low_latency_mode();
        self.terminal.invalidate_command_suggestion_search();
        if low_latency_mode {
            self.terminal.clear_command_tracking();
        }
        self.invalidate_paint_theme_caches();
        self.save_terminal_settings(cx);
        let session_ids = self
            .visible_terminal_session_ids()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        for session_id in session_ids {
            self.notify_terminal_surface_only(Some(session_id.as_str()), cx);
        }
    }

    pub(in crate::features) fn set_terminal_scrollback_lines(
        &mut self,
        value: u32,
        cx: &mut Context<Self>,
    ) {
        self.settings.set_terminal_scrollback_lines(value);
        if !self.shell.has_settings_draft() {
            self.enforce_terminal_scrollback_limit();
        }
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn set_terminal_keep_alive_interval(
        &mut self,
        value: u32,
        cx: &mut Context<Self>,
    ) {
        self.settings.set_terminal_keep_alive_interval(value);
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_terminal_workspace_padding(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.toggle_terminal_workspace_padding();
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_terminal_line_numbers(&mut self, cx: &mut Context<Self>) {
        self.settings.toggle_terminal_line_numbers();
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_terminal_timestamps(&mut self, cx: &mut Context<Self>) {
        self.settings.toggle_terminal_timestamps();
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_terminal_timestamp_milliseconds(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.toggle_terminal_timestamp_milliseconds();
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_multi_line_paste_dialog(&mut self, cx: &mut Context<Self>) {
        self.settings.toggle_multi_line_paste_dialog();
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_paste_image_as_path(&mut self, cx: &mut Context<Self>) {
        self.settings.toggle_paste_image_as_path();
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_remote_stats_panel(&mut self, cx: &mut Context<Self>) {
        self.settings.toggle_remote_stats_panel();
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn set_remote_stats_interval(
        &mut self,
        value: u32,
        cx: &mut Context<Self>,
    ) {
        self.settings.set_remote_stats_interval(value);
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_process_manager_panel(&mut self, cx: &mut Context<Self>) {
        self.settings.toggle_process_manager_panel();
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn set_process_manager_interval(
        &mut self,
        value: u32,
        cx: &mut Context<Self>,
    ) {
        self.settings.set_process_manager_interval(value);
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_docker_manager_panel(&mut self, cx: &mut Context<Self>) {
        self.settings.toggle_docker_manager_panel();
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn set_docker_manager_interval(
        &mut self,
        value: u32,
        cx: &mut Context<Self>,
    ) {
        self.settings.set_docker_manager_interval(value);
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn save_terminal_settings(&mut self, cx: &mut Context<Self>) {
        if self.defer_settings_persistence(cx) {
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_terminal_settings(self.settings.summary()))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.enforce_terminal_scrollback_limit();
                self.settings
                    .update_store_status("terminal settings saved", true);
                self.shell.set_status("terminal settings saved".to_string());
            }
            Err(error) => {
                let message = format!("terminal settings save failed: {error}");
                self.settings.update_store_status(message.clone(), false);
                self.shell.set_status(message);
            }
        }
        cx.notify();
    }
}
