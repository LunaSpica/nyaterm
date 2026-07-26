use super::*;

use crate::models::{QuickCommandSortMode, QuickCommandViewMode};

impl NyaTermApp {
    pub(in crate::features) fn close_quick_command_toolbar_popovers(&mut self) {
        self.quick_command_state.list.sort_menu_open = false;
        self.quick_command_state.list.view_menu_open = false;
        self.quick_command_state.ai.popover_open = false;
        self.quick_command_state.list.category_menu = None;
    }

    pub(in crate::features) fn refresh_quick_commands(&mut self) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.load_quick_commands())
        {
            Ok(config) => {
                self.quick_commands = Arc::from(config.commands);
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
        self.quick_command_state.list.row_menu = None;
        self.close_quick_command_toolbar_popovers();
        self.quick_command_state.list.view_mode = mode;
        self.settings.ui_quick_cmd_view_mode = quick_command_view_mode_setting(mode).to_string();
        self.save_quick_command_ui_settings(cx);
    }

    pub(in crate::features) fn set_quick_command_sort_mode(
        &mut self,
        mode: QuickCommandSortMode,
        cx: &mut Context<Self>,
    ) {
        self.quick_command_state.list.row_menu = None;
        self.close_quick_command_toolbar_popovers();
        self.quick_command_state.list.sort_mode = mode;
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
                self.apply_gpui_settings(settings);
                self.store_status.message = "quick command UI settings saved".to_string();
                self.store_status.ready = true;
                self.terminal.view.status = "quick command UI settings saved".to_string();
            }
            Err(error) => {
                self.store_status.message =
                    format!("quick command UI settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal.view.status = self.store_status.message.clone();
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
                self.quick_command_state.list.search_draft.pop();
                cx.notify();
            }
            "escape" => {
                self.quick_command_state.list.search_draft.clear();
                self.quick_command_state.list.selected_category = "all".to_string();
                self.terminal.view.status = "quick command filters cleared".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.quick_command_state.list.search_draft.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn toggle_quick_command_ai_popover(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let next = !self.quick_command_state.ai.popover_open;
        self.close_quick_command_toolbar_popovers();
        self.quick_command_state.list.row_menu = None;
        self.quick_command_state.ai.popover_open = next;
        if next {
            window.focus(&self.quick_command_state.ai.focus);
        }
        cx.notify();
    }

    pub(in crate::features) fn submit_quick_command_ai_prompt(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let prompt = self.quick_command_state.ai.prompt_draft.trim().to_string();
        if prompt.is_empty() {
            self.terminal.view.status = "describe a command to generate".to_string();
            cx.notify();
            return;
        }

        self.quick_command_state.ai.prompt_draft.clear();
        self.close_quick_command_toolbar_popovers();
        self.ai.chat.prompt_draft = format!("Generate a shell command for: {prompt}");
        self.ai.chat.response_preview = "Quick command generation ready".to_string();
        self.ai.panel.status = "quick command AI assist".to_string();
        self.ensure_panel_open(NavItem::AiAssistant);
        window.focus(&self.ai.chat.focus);
        cx.notify();
    }

    pub(in crate::features) fn handle_quick_command_ai_prompt_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "enter" => {
                self.submit_quick_command_ai_prompt(window, cx);
            }
            "backspace" if !keystroke.modifiers.platform => {
                self.quick_command_state.ai.prompt_draft.pop();
                cx.notify();
            }
            "escape" => {
                self.quick_command_state.ai.popover_open = false;
                cx.notify();
            }
            _ if !keystroke.modifiers.platform => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.quick_command_state.ai.prompt_draft.push_str(input);
                    cx.notify();
                }
            }
            _ => {}
        }
    }
}
