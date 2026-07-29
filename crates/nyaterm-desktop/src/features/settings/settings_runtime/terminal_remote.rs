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
        self.settings.summary.x11_display = text;
        cx.notify();
    }

    pub(in crate::features) fn toggle_terminal_hardware_acceleration(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.terminal_hardware_acceleration =
            !self.settings.summary.terminal_hardware_acceleration;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_terminal_low_latency_mode(&mut self, cx: &mut Context<Self>) {
        self.settings.summary.terminal_low_latency_mode =
            !self.settings.summary.terminal_low_latency_mode;
        self.terminal.assist.invalidate_command_suggestion_search();
        if self.settings.summary.terminal_low_latency_mode {
            self.terminal.assist.clear_command_tracking();
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

    pub(in crate::features) fn adjust_terminal_scrollback_lines(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let next =
            (self.settings.summary.terminal_scrollback_lines as i32 + delta).clamp(100, 100_000);
        self.settings.summary.terminal_scrollback_lines = next as u32;
        if !self.shell.has_settings_draft() {
            self.enforce_terminal_scrollback_limit();
        }
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn adjust_terminal_keep_alive_interval(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let next =
            (self.settings.summary.terminal_keep_alive_interval as i32 + delta).clamp(0, 600);
        self.settings.summary.terminal_keep_alive_interval = next as u32;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_terminal_workspace_padding(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.terminal_show_workspace_padding =
            !self.settings.summary.terminal_show_workspace_padding;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_terminal_line_numbers(&mut self, cx: &mut Context<Self>) {
        self.settings.summary.terminal_show_line_numbers =
            !self.settings.summary.terminal_show_line_numbers;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_terminal_timestamps(&mut self, cx: &mut Context<Self>) {
        self.settings.summary.terminal_show_timestamps =
            !self.settings.summary.terminal_show_timestamps;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_terminal_timestamp_milliseconds(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.terminal_show_timestamp_milliseconds =
            !self.settings.summary.terminal_show_timestamp_milliseconds;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_multi_line_paste_dialog(&mut self, cx: &mut Context<Self>) {
        self.settings.summary.terminal_show_multi_line_paste_dialog =
            !self.settings.summary.terminal_show_multi_line_paste_dialog;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_paste_image_as_path(&mut self, cx: &mut Context<Self>) {
        self.settings.summary.terminal_paste_image_as_path =
            !self.settings.summary.terminal_paste_image_as_path;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_remote_stats_panel(&mut self, cx: &mut Context<Self>) {
        self.settings.summary.ui_show_remote_stats = !self.settings.summary.ui_show_remote_stats;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn adjust_remote_stats_interval(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.summary.ui_remote_stats_interval as i32 + delta).clamp(1, 60);
        self.settings.summary.ui_remote_stats_interval = next as u32;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_process_manager_panel(&mut self, cx: &mut Context<Self>) {
        self.settings.summary.ui_show_process_manager =
            !self.settings.summary.ui_show_process_manager;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn adjust_process_manager_interval(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.summary.ui_process_manager_interval as i32 + delta).clamp(3, 120);
        self.settings.summary.ui_process_manager_interval = next as u32;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_docker_manager_panel(&mut self, cx: &mut Context<Self>) {
        self.settings.summary.ui_show_docker_manager =
            !self.settings.summary.ui_show_docker_manager;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn adjust_docker_manager_interval(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.summary.ui_docker_manager_interval as i32 + delta).clamp(3, 120);
        self.settings.summary.ui_docker_manager_interval = next as u32;
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
        .and_then(|store| store.save_terminal_settings(&self.settings.summary))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.enforce_terminal_scrollback_limit();
                self.settings.store_status.message = "terminal settings saved".to_string();
                self.settings.store_status.ready = true;
                self.terminal.view.status = "terminal settings saved".to_string();
            }
            Err(error) => {
                self.settings.store_status.message =
                    format!("terminal settings save failed: {error}");
                self.settings.store_status.ready = false;
                self.terminal.view.status = self.settings.store_status.message.clone();
            }
        }
        cx.notify();
    }
}
