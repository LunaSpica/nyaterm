use super::*;

impl NyaTermApp {
    pub(in crate::features) fn update_x11_display(
        &mut self,
        value: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.settings.x11_display = value.to_string();
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn adjust_terminal_scrollback_lines(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.terminal_scrollback_lines as i32 + delta).clamp(100, 100_000);
        self.settings.terminal_scrollback_lines = next as u32;
        self.enforce_terminal_scrollback_limit();
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn adjust_terminal_keep_alive_interval(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.terminal_keep_alive_interval as i32 + delta).clamp(0, 600);
        self.settings.terminal_keep_alive_interval = next as u32;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_terminal_workspace_padding(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.terminal_show_workspace_padding =
            !self.settings.terminal_show_workspace_padding;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_terminal_line_numbers(&mut self, cx: &mut Context<Self>) {
        self.settings.terminal_show_line_numbers = !self.settings.terminal_show_line_numbers;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_terminal_timestamps(&mut self, cx: &mut Context<Self>) {
        self.settings.terminal_show_timestamps = !self.settings.terminal_show_timestamps;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_terminal_timestamp_milliseconds(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.terminal_show_timestamp_milliseconds =
            !self.settings.terminal_show_timestamp_milliseconds;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_multi_line_paste_dialog(&mut self, cx: &mut Context<Self>) {
        self.settings.terminal_show_multi_line_paste_dialog =
            !self.settings.terminal_show_multi_line_paste_dialog;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_paste_image_as_path(&mut self, cx: &mut Context<Self>) {
        self.settings.terminal_paste_image_as_path = !self.settings.terminal_paste_image_as_path;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_remote_stats_panel(&mut self, cx: &mut Context<Self>) {
        self.settings.ui_show_remote_stats = !self.settings.ui_show_remote_stats;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn adjust_remote_stats_interval(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.ui_remote_stats_interval as i32 + delta).clamp(1, 60);
        self.settings.ui_remote_stats_interval = next as u32;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_process_manager_panel(&mut self, cx: &mut Context<Self>) {
        self.settings.ui_show_process_manager = !self.settings.ui_show_process_manager;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn adjust_process_manager_interval(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.ui_process_manager_interval as i32 + delta).clamp(3, 120);
        self.settings.ui_process_manager_interval = next as u32;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_docker_manager_panel(&mut self, cx: &mut Context<Self>) {
        self.settings.ui_show_docker_manager = !self.settings.ui_show_docker_manager;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn adjust_docker_manager_interval(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let next = (self.settings.ui_docker_manager_interval as i32 + delta).clamp(3, 120);
        self.settings.ui_docker_manager_interval = next as u32;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn save_terminal_settings(&mut self, cx: &mut Context<Self>) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_terminal_settings(&self.settings))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
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
}
