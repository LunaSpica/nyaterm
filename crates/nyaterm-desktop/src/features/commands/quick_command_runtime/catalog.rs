use gpui::{Context, KeyDownEvent, Window};
use nyaterm_core::ConnectionStore;

use crate::features::{NyaTermApp, TextInputSetup};
use crate::models::{NavItem, QuickCommandSortMode, QuickCommandViewMode};

use super::helpers::{quick_command_sort_mode_setting, quick_command_view_mode_setting};

impl NyaTermApp {
    pub(in crate::features) fn close_quick_command_toolbar_popovers(&mut self) {
        self.commands.close_quick_toolbar_popovers();
    }

    pub(in crate::features) fn refresh_quick_commands(&mut self) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.load_quick_commands())
        {
            Ok(config) => {
                self.commands
                    .replace_quick_command_catalog(config.commands, config.categories);
            }
            Err(error) => {
                self.settings
                    .set_store_message(format!("quick command refresh failed: {error}"));
                self.settings.set_store_ready(false);
            }
        }
    }

    pub(in crate::features) fn set_quick_command_view_mode(
        &mut self,
        mode: QuickCommandViewMode,
        cx: &mut Context<Self>,
    ) {
        self.commands.set_quick_view_mode(mode);
        self.settings.summary.ui_quick_cmd_view_mode =
            quick_command_view_mode_setting(mode).to_string();
        self.save_quick_command_ui_settings(cx);
    }

    pub(in crate::features) fn set_quick_command_sort_mode(
        &mut self,
        mode: QuickCommandSortMode,
        cx: &mut Context<Self>,
    ) {
        self.commands.set_quick_sort_mode(mode);
        self.settings.summary.ui_quick_cmd_sort_mode =
            quick_command_sort_mode_setting(mode).to_string();
        self.save_quick_command_ui_settings(cx);
    }

    pub(in crate::features) fn save_quick_command_ui_settings(&mut self, cx: &mut Context<Self>) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_quick_command_ui_settings(&self.settings.summary))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.settings
                    .set_store_message("quick command UI settings saved".to_string());
                self.settings.set_store_ready(true);
                self.terminal.view.status = "quick command UI settings saved".to_string();
            }
            Err(error) => {
                self.settings
                    .set_store_message(format!("quick command UI settings save failed: {error}"));
                self.settings.set_store_ready(false);
                self.terminal.view.status = self.settings.store_status().message.to_string();
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn apply_quick_command_search(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        self.commands.set_quick_search_draft(text);
        cx.notify();
    }

    pub(in crate::features) fn toggle_quick_command_ai_popover(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let next = self.commands.toggle_quick_ai_popover();
        if next {
            let prompt = self.commands.quick_ai_prompt_draft().to_string();
            let input = self.text_input(
                "quick-command.ai-prompt",
                &prompt,
                TextInputSetup::placeholder(self.tr("ai.placeholder")),
                cx,
            );
            window.focus(&input.read(cx).focus_handle());
        }
        cx.notify();
    }

    pub(in crate::features) fn apply_quick_command_ai_prompt(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        self.commands.set_quick_ai_prompt_draft(text);
        cx.notify();
    }

    pub(in crate::features) fn submit_quick_command_ai_prompt(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(prompt) = self.commands.take_quick_ai_prompt() else {
            self.terminal.view.status = "describe a command to generate".to_string();
            cx.notify();
            return;
        };
        self.reset_text_input("quick-command.ai-prompt", "", cx);
        self.close_quick_command_toolbar_popovers();
        self.set_ai_prompt_draft(format!("Generate a shell command for: {prompt}"), cx);
        self.ai
            .set_chat_response_preview("Quick command generation ready");
        self.ai.set_panel_status("quick command AI assist");
        self.ensure_panel_open(NavItem::AiAssistant);
        window.focus(self.ai.chat_focus());
        cx.notify();
    }

    pub(in crate::features) fn handle_quick_command_ai_prompt_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event.keystroke.key.as_str() {
            "enter" => {
                self.submit_quick_command_ai_prompt(window, cx);
            }
            "escape" => {
                self.commands.close_quick_ai_popover();
                cx.notify();
            }
            _ => {}
        }
    }
}
