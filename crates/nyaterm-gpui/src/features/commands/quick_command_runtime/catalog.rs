use super::*;

impl NyaTermApp {
    pub(in crate::features) fn refresh_quick_commands(&mut self) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.load_quick_commands())
        {
            Ok(config) => {
                self.quick_commands = config.commands;
                self.quick_command_categories = config.categories;
            }
            Err(error) => {
                self.store_status.message = format!("quick command refresh failed: {error}");
                self.store_status.ready = false;
            }
        }
    }

    pub(in crate::features) fn set_quick_command_view_mode(
        &mut self,
        mode: QuickCommandViewMode,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_menu_id = None;
        self.quick_command_view_mode = mode;
        self.settings.ui_quick_cmd_view_mode = quick_command_view_mode_setting(mode).to_string();
        self.save_quick_command_ui_settings(cx);
    }

    pub(in crate::features) fn set_quick_command_sort_mode(
        &mut self,
        mode: QuickCommandSortMode,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_menu_id = None;
        self.quick_command_sort_mode = mode;
        self.settings.ui_quick_cmd_sort_mode = quick_command_sort_mode_setting(mode).to_string();
        self.save_quick_command_ui_settings(cx);
    }

    pub(in crate::features) fn save_quick_command_ui_settings(&mut self, cx: &mut Context<Self>) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_quick_command_ui_settings(&self.settings))
        {
            Ok(settings) => {
                self.settings = settings;
                self.store_status.message = "quick command UI settings saved".to_string();
                self.store_status.ready = true;
                self.terminal_status = "quick command UI settings saved".to_string();
            }
            Err(error) => {
                self.store_status.message =
                    format!("quick command UI settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn handle_quick_command_search_key_down(
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
                self.quick_command_search_draft.pop();
                cx.notify();
            }
            "escape" => {
                self.quick_command_search_draft.clear();
                self.quick_command_selected_category = "all".to_string();
                self.terminal_status = "quick command filters cleared".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.quick_command_search_draft.push_str(input);
                    cx.notify();
                }
            }
        }
    }

}
